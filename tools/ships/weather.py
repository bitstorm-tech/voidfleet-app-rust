"""Verwitterung der Schiffe: Schmutz, Kantenabrieb, Plattenstruktur und Verdeckung als gebackene Maske `<id>_weather`.

Die Maske selbst ist weich (kontinuierlich). Das Spiel (`toon.wesl`, dreckiger Look) liest Schmutz und Ruß als Verlauf
mit Korn und setzt nur für die Rille der Fugen und den Kantenabrieb harte Stufen; die Vorschau (preview.py, noch Clean
Toon) setzt für jeden Kanal eine harte Tonfläche mit den Schwellen aus look.py. So bleiben die Ränder bei jeder
Vergrößerung scharf, und bei Verkleinerung fällt der Mip-Mittelwert unter die Schwelle, statt zu flimmern. Fugen und
Kantenabrieb blendet der Shader zusätzlich nach der Mip-Stufe aus (look.FINE_LOD): Auf dem Brett bleiben Plattenton,
Schmutz und Ruß.

Ablauf je Schiff (`apply`; build.py und preview.py rufen es nach dem Bau, vor dem Wrack):
  1. Abwicklung: eine UV-Ebene über das ganze Mesh (Smart UV Project, liefert bei gleichem Mesh dieselben UVs), Inseln
     mit MARGIN Texeln Abstand. Die Bildgröße folgt aus Fläche und Packdichte, damit das Schiff überall dieselbe
     Texeldichte bekommt (look.WEATHER_DENSITY Texel je Einheit, für die Schiffe in look.WEATHER_DENSE mehr).
  2. Bake (Cycles, CPU, feste Saat, ohne Denoiser) einer Kopie des Schiffs mit einem Hilfsmaterial, allein in einer
     eigenen Szene: Lage, Normale, Umgebungsverdeckung außen (Ritzen) und Material. Den Rand der Inseln füllen die
     Nachbarflächen. Was sich aus der Geometrie genau messen lässt, misst Python je Texel: den Abstand zum nächsten
     Hindernis Richtung Bug (Schlieren; BVH-Strahl, der Raycast-Knoten von Cycles verfehlt im Bake zu viele Treffer) und
     den Abstand zur nächsten sichtbaren konvexen Kante (Abrieb; KD-Baum über Punkte entlang der Kanten, der Bevel-Knoten
     hielte dünne Teile ganz für Kante).
  3. Daraus in numpy die vier Kanäle (unten), Rauschen in Objektkoordinaten mit fester Saat; geschrieben als PNG mit
     eigenem Schreiber (feste Bytes) in den Cache renders/weather/<id>-<hash>.png. Der Hash deckt Mesh, UVs,
     Materialien, Blender-Version, look.py und dieses Modul ab; ein zweiter Bau oder die Vorschau backen nicht noch einmal.
  4. Die toon-Materialien nennen die Maske im Vertrag (`"mask": <id>`, vf.py); das Bild liegt in Blender als
     `<id>_weather` für die Vorschau bereit. build.py schreibt es mit `compress` als `<id>_weather.ktx2` neben das GLB
     (BC7, mit Mip-Kette; ein Viertel des Grafikspeichers der rohen Texel), das GLB selbst trägt kein Bild. wreck.py
     erbt die UVs: Die alten Flächen der Stücke behalten ihre und teilen Maske und Vertrag, die Bruchflächen bekommen
     in derselben Ebene ihre Glut-UVs.

Kanäle (8 Bit, linear gelesen):
  R  Schmutz: Ritzen (wo Teile aufeinandertreffen, unter Kanten), Schlieren von Aufbauten nach achtern (+Y), Ruß an und
     hinter Düsen und Mündungen. Über DIRT_STEP Schmutz, über SOOT_STEP schwerer Ruß.
  G  Kantenabrieb an konvexen Kanten lackierter Flächen, von Rauschen aufgebrochen; über WEAR_STEP.
  B  Helligkeit: Plattenton (je Platte 1 − PLATE_TONE, 1 oder 1 + PLATE_TONE, ohne Platten 1) mal Verdeckung (Licht,
     das Nachbarteile wegnehmen: 1 frei, 1 − OCCLUSION_DARK in der tiefsten Ritze), geteilt durch 1 + PLATE_TONE. Der
     Shader multipliziert die Farbe damit: So sitzen Kleinteile mit einem Schatten auf ihrer Fläche.
  A  Fugen: Abstand zur nächsten Plattenkante geteilt durch SEAM_RANGE, gekappt bei 1; unter SEAM_STEP liegt die Fuge,
     darüber fällt die Plattenkante als Fase ab (der Shader neigt dort die Normale).
Platten sind Rechtecke an den Schiffsachsen (look.PLATE, Reihen versetzt), projiziert entlang der Hauptachse der Normale,
backbord und steuerbord gespiegelt. Den Grad je Schiff (look.WEATHER) und die Anteile je Material
(look.WEATHER_MATERIALS) trägt die Maske selbst: Ein Anteil w wirkt als Exponent 1 / (Grad · w) auf das weiche Feld, ein
höherer Grad lässt also mehr über die Schwelle; über Grad 1 kommen Schmutzflecken auf offenen Flächen dazu. Schiffe ohne
toon-Material (das Trugbild) oder mit Grad 0 bleiben ohne Maske und ohne UVs.

Umgebung: VF_THREADS wie in preview.py (Standard: alle Kerne); VF_DENSITY setzt für Probeläufe die Texeldichte aller
Schiffe; CTT ist der Pfad zum Encoder `ctt` (`cargo install ctt-cli --locked --version 0.6.1`).
"""

import hashlib
import json
import math
import os
import struct
import subprocess
import time
import zlib

import bmesh
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree

import look
import vf

ROOT = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(ROOT, "renders", "weather")
MARGIN = 4  # Texel Rand um jede UV-Insel (Bake-Rand gleich), reicht für Mip 2
SIZE_STEP = 32  # Bildgröße auf ein Vielfaches davon, die ersten Mip-Stufen halbieren genau

# Bake: Samples je Texel im gestrahlten Durchgang, Strahlen je Knoten
SAMPLES = 16
AO_SAMPLES = 16
AO_DISTANCE = 0.4  # Reichweite der Umgebungsverdeckung (Ritzen) in Einheiten
LEE_LENGTH = 1.6  # Reichweite der Schlieren hinter einem Hindernis
LEE_BIAS = 0.06  # so hoch muss es mindestens sein
CHUNK = 1 << 21  # Texel je Portion beim Rechnen der Kanäle (etwa 1 GB Arbeitsspeicher)


# ---------------------------------------------------------------- Ablauf


def apply(obj):
    """UVs und Maske des Schiffsobjekts `obj`; die toon-Materialien nennen sie danach im Vertrag (`mask`). Liefert das
    Bild (mit der Texeldichte als Custom Property `vf_density`, sein Dateipfad ist das PNG im Cache) oder None für
    Schiffe ohne Verwitterung."""
    grade = look.WEATHER.get(obj.name, 1.0)
    toon = [m for m in obj.data.materials if vf.spec(m)["kind"] == "toon"]
    if not toon or grade <= 0:
        return None
    started = time.time()
    size, density = unwrap(obj, float(os.environ.get("VF_DENSITY") or look.WEATHER_DENSE.get(obj.name, look.WEATHER_DENSITY)))
    path = os.path.join(CACHE, f"{obj.name}-{fingerprint(obj, size)}.png")
    if not os.path.exists(path):
        os.makedirs(CACHE, exist_ok=True)
        scratch = f"{path}.{os.getpid()}"
        write_png(scratch, mask(obj, size, grade))
        os.replace(scratch, path)  # mehrere Builds und Vorschauen können gleichzeitig laufen
        print(f"Verwitterung {obj.name}: {size} px, {density:.1f} Texel je Einheit, gebacken in {time.time() - started:.1f} s",
              flush=True)
    image = load(obj.name, path)
    image["vf_density"] = density
    for mat in toon:
        mat["vf"]["mask"] = obj.name
    return image


def unwrap(obj, target):
    """Eine UV-Ebene über das ganze Mesh mit `target` Texeln je Einheit; liefert Bildgröße und genaue Texeldichte."""
    mesh = obj.data
    while mesh.uv_layers:
        mesh.uv_layers.remove(mesh.uv_layers[0])
    mesh.uv_layers.new(name="UVMap")
    area = sum(p.area for p in mesh.polygons)
    size = _round_size(target * math.sqrt(area / 0.5))  # Schätzung für den Rand, dann genau
    for _ in range(2):
        _smart_project(obj, MARGIN / size)
        uv_area = _uv_area(mesh)
        size = _round_size(target * math.sqrt(area / uv_area))
    return size, size * math.sqrt(uv_area / area)


def _round_size(size):
    return int(math.ceil(size / SIZE_STEP)) * SIZE_STEP


def _smart_project(obj, margin):
    for other in bpy.context.scene.objects:
        other.select_set(other == obj)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(66), margin_method="FRACTION", island_margin=margin, area_weight=0.0,
                             correct_aspect=True, scale_to_bounds=False)
    bpy.ops.object.mode_set(mode="OBJECT")


def _uvs(mesh):
    uv = np.empty(len(mesh.loops) * 2, dtype=np.float32)
    mesh.uv_layers.active.data.foreach_get("uv", uv)
    return uv.reshape(-1, 2)


def _uv_area(mesh):
    uv = _uvs(mesh)
    area = 0.0
    for p in mesh.polygons:
        x, y = uv[p.loop_start:p.loop_start + p.loop_total].T
        area += 0.5 * abs(float(np.dot(x, np.roll(y, 1)) - np.dot(y, np.roll(x, 1))))
    return area


def fingerprint(obj, size):
    """Hash über alles, was die Maske bestimmt."""
    mesh = obj.data
    h = hashlib.sha256()
    co = np.empty(len(mesh.vertices) * 3, dtype=np.float32)
    mesh.vertices.foreach_get("co", co)
    corners = np.empty(len(mesh.loops), dtype=np.int32)
    mesh.loops.foreach_get("vertex_index", corners)
    starts = np.empty(len(mesh.polygons), dtype=np.int32)
    mesh.polygons.foreach_get("loop_start", starts)
    index = np.empty(len(mesh.polygons), dtype=np.int32)
    mesh.polygons.foreach_get("material_index", index)
    normals = np.array([n.vector[:] for n in mesh.corner_normals], dtype=np.float32)
    for array in (co, corners, starts, index, normals, _uvs(mesh)):
        h.update(array.tobytes())
    h.update(json.dumps([[m.name, vf.spec(m)] for m in mesh.materials], sort_keys=True).encode())
    h.update(f"{size} {bpy.app.version_string}".encode())
    for module in (__file__, look.__file__):
        with open(module, "rb") as f:
            h.update(f.read())
    return h.hexdigest()[:16]


def load(ship, path):
    """Das PNG als Bild `<ship>_weather` (Daten, kein Farbraum, Kanäle unabhängig)."""
    old = bpy.data.images.get(f"{ship}_weather")
    if old:
        bpy.data.images.remove(old)
    image = bpy.data.images.load(path, check_existing=False)
    image.name = f"{ship}_weather"
    image.colorspace_settings.name = "Non-Color"
    image.alpha_mode = "CHANNEL_PACKED"
    return image


def image_of(mat):
    """Die Maske eines Materials (das Bild des Schiffs, das sein Vertrag nennt) oder None."""
    ship = mat["vf"].get("mask")
    return bpy.data.images.get(f"{ship}_weather") if ship else None


def compress(source, target):
    """Schreibt das PNG `source` als KTX2 `target` fürs Spiel: BC7 (ein Byte je Texel statt vier), als Daten gelesen
    (linear, Alpha ein Kanal wie die anderen), mit Mip-Kette und zstd. Dasselbe Bild ergibt dieselben Bytes."""
    subprocess.run([os.environ.get("CTT", "ctt"), source, "-o", target, "-f", "bc7", "--input-color-space", "linear", "--mipmap",
                    "--zstd"], check=True)


# ---------------------------------------------------------------- Bake


def _bake_scene():
    scene = bpy.data.scenes.new("VF_Bake")
    scene.render.engine = "CYCLES"
    cycles = scene.cycles
    cycles.device = "CPU"
    cycles.use_denoising = False
    cycles.use_adaptive_sampling = False
    cycles.seed = 0
    cycles.use_animated_seed = False
    threads = os.environ.get("VF_THREADS")
    if threads:
        scene.render.threads_mode = "FIXED"
        scene.render.threads = int(threads)
    bake = scene.render.bake
    bake.margin = MARGIN
    bake.margin_type = "ADJACENT_FACES"
    bake.use_clear = False
    bake.target = "IMAGE_TEXTURES"
    return scene


def _bake_material():
    """Hilfsmaterial: Emission aus einem der Durchgänge (`_bake` hängt ihn um), dazu das Zielbild."""
    mat = bpy.data.materials.new("VF_Bake")
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()
    emission = nodes.new("ShaderNodeEmission")
    emission.name = "Emission"
    links.new(emission.outputs["Emission"], nodes.new("ShaderNodeOutputMaterial").inputs["Surface"])
    target = nodes.new("ShaderNodeTexImage")
    target.name = "Target"
    nodes.active = target
    geo = nodes.new("ShaderNodeNewGeometry")

    def combine(name, r, g, b):
        node = nodes.new("ShaderNodeCombineXYZ")
        node.name = name
        for socket, value in zip(node.inputs, (r, g, b)):
            if isinstance(value, (int, float)):
                socket.default_value = value
            else:
                links.new(value, socket)

    ao = nodes.new("ShaderNodeAmbientOcclusion")
    ao.samples, ao.only_local, ao.inside = AO_SAMPLES, True, False
    ao.inputs["Distance"].default_value = AO_DISTANCE
    attribute = nodes.new("ShaderNodeAttribute")
    attribute.attribute_type, attribute.attribute_name = "GEOMETRY", "vf_material"
    # Je Texel gehört zum Bake genau ein Dreieck: Der Index bleibt über alle Samples gleich, die Verdeckung wird gemittelt
    combine("rays", ao.outputs["AO"], attribute.outputs["Fac"], 1.0)
    sources = {"position": geo.outputs["Position"], "normal": geo.outputs["Normal"], "rays": nodes["rays"].outputs["Vector"]}
    return mat, sources


def bake(obj, size):
    """Die gebackenen Größen je Texel (Blender-Reihenfolge, Zeile 0 unten): position, normal und rays (AO außen,
    Materialindex, 1 = belegt; −1 außerhalb der Inseln und ihres Rands)."""
    scene = _bake_scene()
    mesh = obj.data.copy()
    index = np.empty(len(mesh.polygons), dtype=np.int32)
    mesh.polygons.foreach_get("material_index", index)
    mesh.attributes.new("vf_material", "FLOAT", "FACE").data.foreach_set("value", index.astype(np.float32))
    mesh.polygons.foreach_set("material_index", np.zeros_like(index))
    mesh.materials.clear()
    mat, sources = _bake_material()
    mesh.materials.append(mat)
    tmp = bpy.data.objects.new("VF_Bake", mesh)
    scene.collection.objects.link(tmp)
    view_layer = scene.view_layers[0]
    view_layer.objects.active = tmp
    tmp.select_set(True, view_layer=view_layer)
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    out = {}
    for name, samples in (("position", 1), ("normal", 1), ("rays", SAMPLES)):
        image = bpy.data.images.new(f"VF_Bake_{name}", size, size, alpha=True, float_buffer=True)
        image.colorspace_settings.name = "Non-Color"
        image.pixels.foreach_set(np.full(size * size * 4, -1.0, dtype=np.float32))
        nodes["Target"].image = image
        links.new(sources[name], nodes["Emission"].inputs["Color"])
        scene.cycles.samples = samples
        with bpy.context.temp_override(scene=scene, view_layer=view_layer, active_object=tmp, object=tmp,
                                       selected_objects=[tmp], selected_editable_objects=[tmp]):
            bpy.ops.object.bake(type="EMIT")
        pixels = np.empty(size * size * 4, dtype=np.float32)
        image.pixels.foreach_get(pixels)
        out[name] = pixels.reshape(size, size, 4)[..., :3].copy()
        bpy.data.images.remove(image)
    bpy.data.objects.remove(tmp)
    bpy.data.meshes.remove(mesh)
    bpy.data.materials.remove(mat)
    bpy.data.scenes.remove(scene)
    return out


# ---------------------------------------------------------------- Rauschen (numpy, feste Saat)

_GRADIENTS = np.array([[1, 1, 0], [-1, 1, 0], [1, -1, 0], [-1, -1, 0], [1, 0, 1], [-1, 0, 1], [1, 0, -1], [-1, 0, -1],
                       [0, 1, 1], [0, -1, 1], [0, 1, -1], [0, -1, -1]], dtype=np.float64)


def _hash(ix, iy, iz, seed):
    """Ganzzahliger Hash dreier Gitterkoordinaten (splitmix64), gleichverteilt in 0..2^64."""
    with np.errstate(over="ignore"):
        h = (ix.astype(np.uint64) * np.uint64(0x9E3779B97F4A7C15)) ^ (iy.astype(np.uint64) * np.uint64(0xC2B2AE3D27D4EB4F))
        h ^= iz.astype(np.uint64) * np.uint64(0x165667B19E3779F9) ^ np.uint64(seed * 0x27D4EB2F165667C5 % 2**64)
        h ^= h >> np.uint64(31)
        h *= np.uint64(0xBF58476D1CE4E5B9)
        h ^= h >> np.uint64(27)
        h *= np.uint64(0x94D049BB133111EB)
        h ^= h >> np.uint64(31)
    return h


def _random(cells, seed):
    """Zufallswert 0..1 je Zelle (Ganzzahlen, Spalten x, y, z)."""
    return (_hash(cells[:, 0], cells[:, 1], cells[:, 2], seed) >> np.uint64(11)).astype(np.float64) / float(2**53)


def _perlin(p, seed):
    """Gradientenrauschen in 3D, etwa −1..1, eine Zelle je Einheit."""
    cell = np.floor(p)
    f = p - cell
    cell = cell.astype(np.int64)
    u = f * f * f * (f * (f * 6 - 15) + 10)
    total = np.zeros(len(p))
    for dx in (0, 1):
        for dy in (0, 1):
            for dz in (0, 1):
                corner = np.array((dx, dy, dz))
                g = _GRADIENTS[(_hash(cell[:, 0] + dx, cell[:, 1] + dy, cell[:, 2] + dz, seed) % np.uint64(12)).astype(np.int64)]
                w = np.prod(np.where(corner, u, 1 - u), axis=1)
                total += w * np.einsum("ij,ij->i", g, f - corner)
    return total


def _fbm(p, seed, octaves=3):
    """Mehrere Oktaven Gradientenrauschen, auf etwa 0..1 gebracht (Mittel 0,5)."""
    total, amp, norm = np.zeros(len(p)), 1.0, 0.0
    for o in range(octaves):
        total += amp * _perlin(p * (2 ** o) + 17.3 * o, seed + o)
        norm += amp
        amp *= 0.5
    return np.clip(0.5 + 0.75 * total / norm, 0.0, 1.0)


def _blur(image, covered, passes=2):
    """Mittel über 3 × 3 belegte Texel, `passes`-mal: glättet das Rauschen der gestrahlten Größen. Reicht nicht über den
    Rand (MARGIN) einer Insel hinaus zur nächsten."""
    value, weight = np.where(covered, image, 0.0), covered.astype(np.float64)
    h, w = image.shape
    for _ in range(passes):
        pv, pw = np.pad(value, 1), np.pad(weight, 1)
        value = sum(pv[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy in (-1, 0, 1) for dx in (-1, 0, 1))
        weight = sum(pw[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy in (-1, 0, 1) for dx in (-1, 0, 1))
    return np.where(weight > 0, value / np.maximum(weight, 1e-9), image)


def _smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


# ---------------------------------------------------------------- Ruß-Quellen


def _solid_tree(mesh):
    """BVH der toon-Flächen: Lichter und Hologramme halten weder Schmutz noch Ruß auf."""
    solid = [p for p in mesh.polygons if vf.spec(mesh.materials[p.material_index])["kind"] == "toon"]
    return BVHTree.FromPolygons([v.co for v in mesh.vertices], [p.vertices[:] for p in solid])


def _lee(points, normals, tree):
    """Abstand zum nächsten Hindernis Richtung Bug (−Y) je Punkt, LEE_LENGTH ohne Treffer: Dahinter laufen Schlieren
    nach achtern. Gestartet wird um LEE_BIAS über der Fläche; Flächen, die nach achtern schauen, bekommen keine (ihr
    Strahl träfe das eigene Teil)."""
    out = np.full(len(points), LEE_LENGTH)
    ahead = Vector((0.0, -1.0, 0.0))
    for i in np.nonzero(normals[:, 1] < 0.2)[0]:
        hit = tree.ray_cast(Vector(points[i] + normals[i] * LEE_BIAS), ahead, LEE_LENGTH)
        if hit[0] is not None:
            out[i] = hit[3]
    return out


def _edge_tree(obj, paint):
    """KD-Baum über Punkte entlang der sichtbaren konvexen Kanten (Knick ab look.WEAR_ANGLE) an Flächen mit Abrieb
    (`paint`: Materialindizes); Punkte, die in einem anderen Teil stecken, fallen weg. None ohne Kanten."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.normal_update()
    tree = _solid_tree(obj.data)
    step = look.WEAR_REACH / 4
    points = []
    for edge in bm.edges:
        if len(edge.link_faces) != 2:
            continue
        a, b = edge.link_faces
        if (a.material_index not in paint and b.material_index not in paint) or a.normal.angle(b.normal, 0.0) < look.WEAR_ANGLE:
            continue
        if (b.calc_center_median() - a.calc_center_median()).dot(a.normal) > 0:  # konkav
            continue
        out = (a.normal + b.normal).normalized()
        v0, v1 = edge.verts[0].co, edge.verts[1].co
        count = max(1, int(math.ceil((v1 - v0).length / step)))
        for k in range(count + 1):
            q = v0.lerp(v1, k / count)
            hit, normal, _, _ = tree.ray_cast(q + out * 1e-3, out)
            if hit is None or normal.dot(out) < 0:  # eine Rückseite hieße: steckt in einem Teil
                points.append(q)
    bm.free()
    if not points:
        return None
    kd = KDTree(len(points))
    for i, q in enumerate(points):
        kd.insert(q, i)
    kd.balance()
    return kd


def _edge_distance(points, select, kd):
    """Abstand zur nächsten Kante je Punkt (nur `select`, sonst unendlich)."""
    out = np.full(len(points), np.inf)
    if kd is not None:
        for i in np.nonzero(select)[0]:
            out[i] = kd.find(points[i])[2]
    return out


def emitters(obj):
    """Ruß-Quellen in Objektkoordinaten [(Mitte, Richtung, Radius, Düse?)]: die Brennkammern (Inseln der Flächen von
    `<id>_core`, Richtung ist die, in die ein Strahl aus ihrer Mitte frei hinausgeht, am liebsten achtern) und die
    Mündungen (kreisrunde, kleine, waagrechte Deckel runder Materialien, vor denen bis SOOT_REACH nichts liegt und die
    nicht in einem anderen Teil stecken). Lichter blockieren nicht."""
    mesh = obj.data
    tree = _solid_tree(mesh)

    def free(origin, direction, reach):
        # Frei, wenn der Strahl nichts trifft oder erst jenseits von `reach` eine Vorderseite (eine Rückseite hieße:
        # der Punkt steckt in einem Teil; die Hüllen sind geschlossen und nach außen orientiert)
        hit, normal, _, distance = tree.ray_cast(origin, direction)
        return hit is None or (normal.dot(direction) < 0 and distance > reach)

    core = {i for i, m in enumerate(mesh.materials) if m.name == f"{obj.name}_core"}
    out = []
    # Inseln der Brennkammern über gemeinsame Eckpunkte
    faces = [p for p in mesh.polygons if p.material_index in core]
    parent = {}

    def root(v):
        while parent.setdefault(v, v) != v:
            parent[v] = parent[parent[v]]
            v = parent[v]
        return v

    for p in faces:
        for v in p.vertices[1:]:
            parent[root(v)] = root(p.vertices[0])
    islands = {}
    for p in faces:
        islands.setdefault(root(p.vertices[0]), []).append(p)
    for island in sorted(islands.values(), key=lambda faces: faces[0].index):
        area = sum(p.area for p in island)
        center = sum((p.center * p.area for p in island), Vector()) / area
        directions = [Vector(d) for d in ((0, 1, 0), (0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1))]
        direction = next((d for d in directions if tree.ray_cast(center, d, 3.0)[0] is None), directions[0])
        verts = {v for p in island for v in p.vertices}
        radius = max((mesh.vertices[v].co - center - direction * (mesh.vertices[v].co - center).dot(direction)).length for v in verts)
        out.append((center, direction, max(radius, 0.05), True))
    rounds = {i for i, m in enumerate(mesh.materials) if vf.spec(m)["kind"] == "toon" and vf.spec(m)["round"]}
    for p in mesh.polygons:
        if p.material_index not in rounds or p.loop_total < 8 or p.area > 0.3 or abs(p.normal.z) > 0.5:
            continue
        reach = [(mesh.vertices[v].co - p.center).length for v in p.vertices]
        if min(reach) < 0.9 * max(reach) or not free(p.center + p.normal * 1e-3, p.normal, look.SOOT_REACH):
            continue
        out.append((p.center.copy(), p.normal.copy(), sum(reach) / len(reach), False))
    return out


def _soot(points, sources, seed):
    """Ruß 0..1 an und hinter den Quellen, zur Mitte hin voll, außen von Rauschen ausgefranst."""
    soot = np.zeros(len(points))
    for center, direction, radius, nozzle in sources:
        q = points - np.array(center)
        d = np.array(direction)
        along = q @ d
        radial = np.linalg.norm(q - along[:, None] * d, axis=1)
        if nozzle:  # Ring um die Brennkammer, dazu die Fahne nach hinten
            ring = 1 - _smoothstep(0.8 * radius, 3.2 * radius, np.linalg.norm(q, axis=1))
            plume = (1 - _smoothstep(0.0, 6 * radius, along)) * (1 - _smoothstep(1.2 * radius, 2.4 * radius, radial)) * (along > 0)
            value = np.maximum(ring * (along > -2.5 * radius), plume)
        else:  # Mündung: der Lauf dahinter
            behind = -along
            value = (1 - _smoothstep(0.0, 4 * radius, behind)) * (1 - _smoothstep(1.3 * radius, 2.3 * radius, radial)) * (behind > -0.5 * radius)
        soot = np.maximum(soot, value)
    return soot * (0.55 + 0.75 * _fbm(points * 4.0, seed + 40, 2))


# ---------------------------------------------------------------- Maske


def weights(mat, ship):
    """Anteile Schmutz, Ruß, Abrieb, Platten eines Materials `<ship>_<schlüssel>` (look.WEATHER_MATERIALS; Lichter und
    Hologramme 0)."""
    s = vf.spec(mat)
    if s["kind"] != "toon":
        return (0.0, 0.0, 0.0, 0.0)
    default = look.WEATHER_ROUND if s["round"] else look.WEATHER_PAINT
    return look.WEATHER_MATERIALS.get(mat.name.removeprefix(f"{ship}_"), default)


def _shape(field, weight, grade):
    """Weiches Feld 0..1 nach Anteil und Grad: Exponent 1 / (Grad · Anteil), 0 ohne Anteil."""
    power = grade * weight
    return np.where(power > 0, np.clip(field, 0.0, 1.0) ** (1.0 / np.maximum(power, 1e-3)), 0.0)


def _plates(p, n):
    """Plattenton (1 − PLATE_TONE, 1, 1 + PLATE_TONE) und Abstand zur nächsten Plattenkante je Punkt: Rechtecke
    look.PLATE an den Schiffsachsen, projiziert entlang der Hauptachse der Normale, Reihen um eine halbe Platte
    versetzt, backbord und steuerbord gespiegelt."""
    q = p.copy()
    q[:, 0] = np.abs(q[:, 0])
    size = np.array(look.PLATE)
    axis = np.argmax(np.abs(n), axis=1)
    tone = np.empty(len(p))
    distance = np.empty(len(p))
    for k in range(3):
        sel = axis == k
        a, b = [i for i in range(3) if i != k]
        u, v = q[sel, a] / size[a], q[sel, b] / size[b]
        cu = np.floor(u)
        v = v + 0.5 * (cu % 2)
        cv = np.floor(v)
        side = (n[sel, k] > 0).astype(np.int64)
        cells = np.stack([cu.astype(np.int64), cv.astype(np.int64), side * 4 + k], axis=1)
        r = _random(cells, 7)
        tone[sel] = 1.0 + look.PLATE_TONE * np.where(r < 0.3, -1.0, np.where(r < 0.7, 0.0, 1.0))
        # Abstand in der Fläche: entlang der Projektion gemessen, durch die Neigung der Fläche geteilt
        du = np.abs(u - np.round(u)) * size[a] / np.sqrt(np.maximum(1 - n[sel, a] ** 2, 0.05))
        dv = np.abs(v - np.round(v)) * size[b] / np.sqrt(np.maximum(1 - n[sel, b] ** 2, 0.05))
        distance[sel] = np.minimum(du, dv)
    return tone, distance


def mask(obj, size, grade):
    """Die Maske als RGBA-Bytes (Blender-Reihenfolge). Die Kanäle entstehen in Portionen von CHUNK Texeln: So wächst
    der Speicher nicht mit der Bildgröße (die Bastion hat bei 128 Texeln je Einheit über 40 Millionen Texel)."""
    geo = bake(obj, size)
    rays = geo.pop("rays")
    covered = rays[..., 2] > 0.5
    material = rays[..., 1].ravel().copy()
    ao = _blur(rays[..., 0].astype(np.float64), covered).ravel()
    del rays
    position, normal = geo["position"].reshape(-1, 3), geo["normal"].reshape(-1, 3)
    materials = list(obj.data.materials)
    table = np.array([weights(m, obj.name) for m in materials])
    solid, sources = _solid_tree(obj.data), emitters(obj)
    edges = _edge_tree(obj, {i for i in range(len(materials)) if table[i][2] > 0})
    out = np.empty((size * size, 4), dtype=np.uint8)
    out[:] = np.rint(np.array([0.0, 0.0, 1.0 / (1.0 + look.PLATE_TONE), 1.0]) * 255)
    inside = np.flatnonzero(covered)
    for part in np.array_split(inside, len(inside) // CHUNK + 1):
        w = table[np.clip(np.rint(material[part]).astype(np.int64), 0, len(materials) - 1)]
        channels = _channels(position[part].astype(np.float64), normal[part].astype(np.float64), w, ao[part], grade, solid, sources, edges)
        out[part] = np.rint(np.clip(channels, 0.0, 1.0) * 255)
    return out.reshape(size, size, 4)


def _channels(p, n, w, ao, grade, solid, sources, edges):
    """Die vier Kanäle (Spalten) für Texel mit Lage `p`, Normale `n`, Anteilen `w` ihres Materials und
    Umgebungsverdeckung `ao`; `solid` ist der BVH der toon-Flächen, `sources` die Ruß-Quellen, `edges` der KD-Baum der
    Kanten mit Abrieb."""
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-6)
    lee = _lee(p, n, solid)

    # Schmutz: Ritzen, von Flecken aufgebrochen; Schlieren hinter Hindernissen, nach achtern gestreckt
    crevice = _smoothstep(look.CREVICE[0], look.CREVICE[1], 1 - ao)
    dirt = crevice * (0.3 + 1.2 * _fbm(p * look.DIRT_SCALE, 1))
    reach = 1 - lee / LEE_LENGTH
    streaky = _fbm(p * np.array(look.STREAK_SCALE), 2, 2)
    streak = reach * _smoothstep(0.45, 0.62, streaky)
    # Über Grad 1 (Schrott) dazu Flecken auf offenen Flächen, mit dem Grad stärker
    patches = max(grade - 1.0, 0.0) * look.PATCH_MIX * _smoothstep(0.5, 0.75, _fbm(p * look.PATCH_SCALE, 5))
    grime = np.maximum(np.maximum(dirt, look.STREAK_MIX * streak), patches)
    red = np.maximum(_shape(grime, w[:, 0], grade), _shape(_soot(p, sources, 3), w[:, 1], grade))
    # Abrieb: Nähe zur sichtbaren konvexen Kante (1 auf ihr, 0 ab WEAR_REACH), außen frei. Das Rauschen sagt, wie tief
    # ein Splitter von der Kante hineinreicht (die meisten Stellen gar nicht), damit es keine durchgehende Fase wird:
    # über der Schwelle 0,5 liegt, was näher als 1 − Tiefe ist. Fern jeder Kante und ohne Splitter bleibt das Feld 0,
    # auch ein hoher Grad macht daraus also keine Fase, nur tiefere Splitter
    convex = _smoothstep(0.62, 0.92, ao)
    near = np.clip(1 - _edge_distance(p, (w[:, 2] > 0) & (convex > 0), edges) / look.WEAR_REACH, 0.0, 1.0)
    depth = np.clip((_fbm(p * look.CHIP_SCALE, 4, 2) - 0.55) * 4.0, 0.0, 0.9)
    wear = 0.5 * near * convex / (1.0 - depth) * _smoothstep(0.0, 0.15, depth)
    tone, distance = _plates(p, n)
    has_plates = w[:, 3] > 0
    shade = 1.0 - look.OCCLUSION_DARK * _smoothstep(look.OCCLUSION[0], look.OCCLUSION[1], 1 - ao)
    return np.stack([red, _shape(wear, w[:, 2], grade), np.where(has_plates, tone, 1.0) * shade / (1.0 + look.PLATE_TONE),
                     np.where(has_plates, np.clip(distance / look.SEAM_RANGE, 0.0, 1.0), 1.0)], axis=1)


# ---------------------------------------------------------------- Mip-Stufen (für die Vorschau) und PNG


def downsample(pixels):
    """Eine Mip-Stufe tiefer: Mittel über 2 × 2 Texel (gerundet), bei ungerader Größe fällt die letzte Zeile bzw.
    Spalte weg; die Mip-Kette des Spiels rechnet `ctt` beim Komprimieren fast genauso."""
    h, w = max(pixels.shape[0] // 2, 1), max(pixels.shape[1] // 2, 1)
    p = pixels.astype(np.uint32)
    ys = np.minimum(np.arange(h)[:, None] * 2 + np.array([0, 1]), pixels.shape[0] - 1)
    xs = np.minimum(np.arange(w)[:, None] * 2 + np.array([0, 1]), pixels.shape[1] - 1)
    total = p[ys[:, 0]][:, xs[:, 0]] + p[ys[:, 0]][:, xs[:, 1]] + p[ys[:, 1]][:, xs[:, 0]] + p[ys[:, 1]][:, xs[:, 1]]
    return ((total + 2) // 4).astype(np.uint8)


def write_png(path, pixels):
    """RGBA-Bytes (Zeile 0 unten wie in Blender) als PNG: Filter je Zeile nach der kleinsten Summe, zlib Stufe 9, keine
    Metadaten; dieselben Pixel ergeben dieselben Bytes."""
    rows = pixels[::-1].reshape(pixels.shape[0], -1).astype(np.int16)
    h, stride = rows.shape
    up = np.vstack([np.zeros((1, stride), np.int16), rows[:-1]])
    left = np.hstack([np.zeros((h, 4), np.int16), rows[:, :-4]])
    corner = np.hstack([np.zeros((h, 4), np.int16), up[:, :-4]])
    guess = left + up - corner
    da, db, dc = np.abs(guess - left), np.abs(guess - up), np.abs(guess - corner)
    paeth = np.where((da <= db) & (da <= dc), left, np.where(db <= dc, up, corner))
    filtered = [(f % 256).astype(np.uint8) for f in (rows, rows - left, rows - up, rows - (left + up) // 2, rows - paeth)]
    score = np.stack([np.abs(f.view(np.int8).astype(np.int32)).sum(axis=1) for f in filtered])
    best = np.argmin(score, axis=0)
    data = b"".join(bytes([k]) + filtered[k][y].tobytes() for y, k in enumerate(best))

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body))

    header = struct.pack(">IIBBBBB", pixels.shape[1], h, 8, 6, 0, 0, 0)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(data, 9)) + chunk(b"IEND", b""))


def pixels_of(image):
    """Die Bytes eines geladenen Maskenbilds (Blender-Reihenfolge)."""
    w, h = image.size
    values = np.empty(w * h * 4, dtype=np.float32)
    image.pixels.foreach_get(values)
    return np.rint(values.reshape(h, w, 4) * 255).astype(np.uint8)
