"""Haut der Wesen: Zeichnung, Leuchten, Nässe und Relief des Unbekannten Kontakts und seiner Brut als gebackene Textur
`<id>_skin`, in derselben Abwicklung wie die Maske der Verwitterung (weather.py).

Die Schiffe der Menschen tragen ihre Geschichte in der Maske: Platten, Fugen, Schmutz, Abrieb. Die Wesen sind
gewachsen, und was sie lebendig aussehen lässt, ist ihre Haut: Adern, die sich über das Fleisch ziehen und nahe den
Leuchtorganen glimmen, verstreute Leuchtpunkte an Flanken und Unterseite wie bei Tiefseefischen, Querfalten an Leib und
Tentakeln, Zellen mit erhabenen Rändern in der Flughaut, Schuppen auf Eiern und Bauchschilden, Poren und Haarrisse im
Knochen, nasser Glanz auf Fleisch und Sehnen. Auch die Lichter tragen sie: Ein Leuchtorgan glüht nicht flach, sondern
mit helleren Kernen und dunklen Gefäßen, das Leuchtfeld der Flügel mit denselben Zellen wie die Flughaut um es herum.

Welches Muster eine Fläche trägt, sagt der Schlüssel ihres Materials (`<id>_<schlüssel>`, PATTERNS); ein Schiff bekommt
eine Haut, sobald eines seiner Materialien einen Schlüssel aus PATTERNS hat. Alle Muster sind Rauschen in
Objektkoordinaten mit fester Saat (die Bausteine aus weather.py, dazu Zellrauschen), Längen in Einheiten des
Unbekannten Kontakts; kleinere Wesen vergrößern sie um FEATURE, damit die Zeichnung auf dem Brett lesbar bleibt.

Ablauf (`apply`, build.py ruft es nach weather.apply mit dessen Bildgröße): Bake von Lage, Normale und Material je
Texel (weather.Bake, ein Sample je Durchgang), dazu je Texel der Abstand zum nächsten Licht (KD-Baum über die
Lichtflächen); daraus die Kanäle in numpy, geschrieben in den Cache der Masken als renders/weather/<id>-skin-<hash>.png
(Hash über alles, was die Maske bestimmt, und dieses Modul). Die Materialien mit Muster nennen die Haut im Vertrag
(`"skin": <id>`, vf.py); build.py schreibt sie mit weather.compress als `<id>_skin.ktx2` neben das GLB. Das Wrack erbt
sie wie die Maske. Die Vorschau (preview.py, Clean Toon) zeigt sie nicht.

Kanäle (8 Bit, linear gelesen; `toon.wesl`, „Haut der Wesen“):
  R  Zeichnung: Helligkeit als Exponent, Faktor 2^(SPAN · (2R − 1)), 0,5 neutral. Dunkle Stellen (Adern, Flecken,
     Falten) nehmen im Shader den Farbton des Schattentons ihres Materials an, Fleisch also Violett.
  G  Leuchten im Türkis des Fremden, 0 bis 1 (auf Lichtern ohne Bedeutung).
  B  Nässe: Anteil des Glanzes des Materials, 0 bis 1.
  A  Höhe: 0,5 + h / (2 · DEPTH), h in Einheiten. Der Shader neigt die Normale nach ihrer Steigung (SKIN_DEPTH).
"""

import hashlib
import os

import numpy as np
from mathutils.kdtree import KDTree

import vf
import weather

SPAN = 1.25  # Zeichnung: R = 0 dunkelt auf 2^−SPAN ab, R = 1 hellt auf 2^SPAN auf (toon.wesl: SKIN_SPAN)
DEPTH = 0.02  # Höhe: A = 0 und A = 1 liegen so viele Einheiten unter bzw. über der Fläche (toon.wesl: SKIN_DEPTH)
FEATURE = {"brood": 1.6}  # Zeichnung je Wesen so viel größer (die Brut steht als Staffel zu 0,42 auf dem Brett)
GLOW_REACH = 0.7  # so weit glimmen Adern und Haut um ein Licht
CHUNK = 1 << 20  # Texel je Portion (das Zellrauschen braucht je Texel etwa 1 KB)


# ---------------------------------------------------------------- Ablauf


def apply(obj, size):
    """Haut des Schiffsobjekts `obj` (nach weather.apply, also mit UVs) in `size` Texeln Kantenlänge; die Materialien
    mit Muster nennen sie danach im Vertrag (`skin`). Liefert den Pfad des PNG oder None für Schiffe ohne Haut."""
    skinned = [m for m in obj.data.materials if _key(m, obj.name) in PATTERNS]
    if not skinned:
        return None
    h = hashlib.sha256(weather.fingerprint(obj, size).encode())
    with open(__file__, "rb") as f:
        h.update(f.read())
    path = os.path.join(weather.CACHE, f"{obj.name}-skin-{h.hexdigest()[:16]}.png")
    if not os.path.exists(path):
        os.makedirs(weather.CACHE, exist_ok=True)
        scratch = f"{path}.{os.getpid()}"
        weather.write_png(scratch, pixels(obj, size))
        os.replace(scratch, path)
    for mat in skinned:
        mat["vf"]["skin"] = obj.name
    return path


def _key(mat, ship):
    """Schlüssel eines Materials `<ship>_<schlüssel>`."""
    return mat.name.removeprefix(f"{ship}_")


def pixels(obj, size):
    """Die Haut als RGBA-Bytes (Blender-Reihenfolge)."""
    baker = weather.Bake(obj, size)
    rays = baker.render("rays", 1)
    covered = (rays[..., 2] > 0.5).ravel()
    material = np.rint(rays[..., 1]).astype(np.int64).ravel()
    del rays
    position, normal = baker.render("position", 1).reshape(-1, 3), baker.render("normal", 1).reshape(-1, 3)
    baker.free()
    lights = _lights(obj)
    scale = FEATURE.get(obj.name, 1.0)
    out = np.empty((size * size, 4), dtype=np.uint8)
    out[:] = _encode(*(np.zeros(1) for _ in range(4)))
    for index, mat in enumerate(obj.data.materials):
        pattern = PATTERNS.get(_key(mat, obj.name))
        texels = np.flatnonzero(covered & (material == index))
        if pattern is None or len(texels) == 0:
            continue
        for part in np.array_split(texels, len(texels) // CHUNK + 1):
            p = position[part].astype(np.float64)
            n = normal[part].astype(np.float64)
            n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-6)
            out[part] = _encode(*pattern(p / scale, n, _distance(p, lights, GLOW_REACH * scale) / scale))
    return out.reshape(size, size, 4)


def _encode(tone, glow, wet, height):
    """Kanäle als Bytes: Zeichnung (Exponent), Leuchten, Nässe, Höhe (Einheiten)."""
    channels = np.stack([0.5 + tone / (2 * SPAN), glow, wet, 0.5 + height / (2 * DEPTH)], axis=1)
    return np.rint(np.clip(channels, 0.0, 1.0) * 255).astype(np.uint8)


def _lights(obj):
    """Punkte der Lichtflächen (Eckpunkte und Mitten) als KD-Baum samt ihrer Hülle (kleinste und größte Ecke), None
    ohne Lichter."""
    mesh = obj.data
    light = {i for i, m in enumerate(mesh.materials) if vf.spec(m)["kind"] == "light"}
    points = []
    for p in mesh.polygons:
        if p.material_index in light:
            points.append(p.center.copy())
            points += [mesh.vertices[v].co.copy() for v in p.vertices]
    if not points:
        return None
    kd = KDTree(len(points))
    for i, q in enumerate(points):
        kd.insert(q, i)
    kd.balance()
    return kd, np.min(points, axis=0), np.max(points, axis=0)


def _distance(points, lights, reach):
    """Abstand zum nächsten Licht je Punkt, unendlich ab `reach`; gesucht wird nur in der um `reach` erweiterten Hülle
    der Lichter."""
    out = np.full(len(points), np.inf)
    if lights is None:
        return out
    kd, low, high = lights
    for i in np.flatnonzero(np.all((points > low - reach) & (points < high + reach), axis=1)):
        d = kd.find(points[i])[2]
        if d < reach:
            out[i] = d
    return out


# ---------------------------------------------------------------- Bausteine


def _ss(a, b, x):
    return weather._smoothstep(a, b, x)


def _fbm(p, seed, octaves=3):
    return weather._fbm(p, seed, octaves)


def _noise(p, seed):
    return weather._perlin(p, seed)


def _warp(p, amount, frequency, seed):
    """Lage, gegen sich selbst verschoben (Domain Warping): gerade Linien des Rauschens werden zu gewundenen."""
    return p + amount * np.stack([_noise(p * frequency + 31.7 * k, seed + k) for k in range(3)], axis=1)


def _lines(p, seed, width):
    """Netz feiner Linien, wo das Gradientenrauschen durch null geht: 1 auf der Linie, 0 ab `width` (in Rauschwerten,
    bei einer Zelle je Einheit etwa so viele Einheiten)."""
    return 1 - _ss(0.0, width, np.abs(_noise(p, seed)))


def _cells(p, seed):
    """Zellrauschen (Worley) in 3D, eine Zelle je Einheit mit einem zufälligen Punkt: Abstand zum nächsten Punkt (F1),
    zum zweitnächsten (F2) und ein Zufallswert 0..1 der nächsten Zelle."""
    base = np.floor(p).astype(np.int64)
    f1, f2, value = np.full(len(p), 9.0), np.full(len(p), 9.0), np.zeros(len(p))
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            for dz in (-1, 0, 1):
                cell = base + (dx, dy, dz)
                h = weather._hash(cell[:, 0], cell[:, 1], cell[:, 2], seed)
                bits = [(h >> np.uint64(s)) & np.uint64(0xFFFF) for s in (0, 16, 32)]
                jitter = np.stack([b.astype(np.float64) / 65535 for b in bits], axis=1)
                d = np.linalg.norm(p - cell - jitter, axis=1)
                closer = d < f1
                f2 = np.where(closer, f1, np.minimum(f2, d))
                value = np.where(closer, (h >> np.uint64(48)).astype(np.float64) / 65535, value)
                f1 = np.where(closer, d, f1)
    return f1, f2, value


def _falloff(distance, reach):
    """1 am Licht, 0 ab `reach`, dazwischen quadratisch."""
    return np.clip(1 - distance / reach, 0.0, 1.0) ** 2


def _freckles(p, n, cell=0.22, radius=0.05, share=0.35):
    """Leuchtpunkte wie bei Tiefseefischen, verstreut über Flanken und Unterseite (nicht oben auf dem Rücken): je Zelle
    von `cell` Einheiten einer an zufälliger Stelle, in `share` der Zellen; 1 im Kern, zum Rand weich."""
    f1, _, value = _cells(p / cell, 51)
    dots = (1 - _ss(0.4 * radius, radius, f1 * cell)) * (value < share)
    return dots * _ss(0.92, 0.75, n[:, 2])


# ---------------------------------------------------------------- Muster je Material (Zeichnung, Leuchten, Nässe, Höhe)


def _flesh(p, n, near):
    """Fleisch: Adern und Kapillaren, Querfalten, Flecken auf dem Rücken, ein hellerer Bauch, Leuchtpunkte an Flanken
    und Unterseite; nahe den Lichtern glimmen die Adern, und die Haut scheint durch."""
    w = _warp(p, 0.35, 0.8, 11)
    veins = _lines(w * 1.4, 12, 0.13)
    capillaries = _lines(w * 3.2, 13, 0.1) * _ss(0.4, 0.65, _fbm(p * 1.1, 14))
    folds = _lines(_warp(p, 0.08, 2.0, 15) * (2.2, 8.5, 2.2), 16, 0.16)
    f1, _, spot = _cells(w * 2.2, 17)
    blotch = (1 - _ss(0.2, 0.45, f1)) * (spot < 0.55) * _ss(-0.2, 0.5, n[:, 2])
    mottle = 2 * _fbm(p * 0.7, 18) - 1
    belly = _ss(-0.25, -0.75, n[:, 2])
    lamps = _freckles(p, n)
    near_light = _falloff(near, GLOW_REACH)
    tone = 0.45 * mottle - 0.55 * blotch + 0.4 * belly - 1.0 * veins - 0.4 * capillaries - 0.2 * folds + 0.4 * lamps
    glow = np.maximum(lamps, near_light * (0.8 * veins + 0.3 * capillaries) + 0.2 * _falloff(near, 0.2))
    wet = 0.45 + 0.7 * (_fbm(p * 2.3, 19) - 0.5) + 0.35 * folds
    height = 0.01 * veins + 0.003 * capillaries - 0.008 * folds + 0.004 * mottle + 0.003 * lamps
    return tone, glow, wet, height


def _sinew(p, n, near):
    """Sehnen, Streben, Adern: dunkel und nass, mit feinen Furchen wie Fasern."""
    fibres = _lines(_warp(p, 0.1, 2.5, 21) * (6.0, 9.0, 6.0), 22, 0.18)
    mottle = 2 * _fbm(p * 1.5, 23) - 1
    tone = 0.25 * mottle - 0.4 * fibres
    wet = 0.75 + 0.25 * fibres
    height = -0.004 * fibres + 0.002 * mottle
    return tone, np.zeros(len(p)), wet, height


def _bone(p, n, near):
    """Knochen: Elfenbein mit Flecken und dunkleren Wachstumsringen, Poren und vereinzelten Haarrissen; matt bis
    poliert."""
    mottle = 2 * _fbm(p * 1.2, 31) - 1
    stains = _ss(0.45, 0.85, _fbm(p * 0.9, 32))
    rings = _lines(_warp(p, 0.06, 1.5, 33) * (3.0, 7.0, 3.0), 34, 0.12)
    cracks = _lines(_warp(p, 0.2, 1.0, 35) * 2.6, 36, 0.035) * _ss(0.58, 0.7, _fbm(p * 0.8, 37))
    f1, _, pit = _cells(p * 13.0, 38)
    pores = (1 - _ss(0.12, 0.3, f1)) * (pit < 0.45)
    tone = 0.2 * mottle - 0.3 * stains - 0.2 * rings - 0.7 * cracks - 0.4 * pores
    wet = 0.2 + 0.5 * _ss(0.4, 0.7, _fbm(p * 1.7, 39)) * (1 - stains)
    height = 0.003 * mottle - 0.003 * rings - 0.006 * cracks - 0.006 * pores
    return tone, np.zeros(len(p)), wet, height


def _wing_cells(p):
    """Zellen der Flughaut, gemeinsam für alle drei Lagen (auch das Leuchtfeld): Rand (1 auf der Zellgrenze) und ein
    Wert je Zelle; dazu größere Adern."""
    f1, f2, value = _cells(_warp(p, 0.12, 1.2, 41) * 3.2, 42)
    rim = 1 - _ss(0.03, 0.13, f2 - f1)
    veins = _lines(_warp(p, 0.3, 0.7, 43) * 0.9, 44, 0.05)
    return rim, value, veins, f1


def _membrane(p, n, near):
    """Flughaut (gedämpftes Türkis): Zellen mit erhabenen, dunklen Rändern, je Zelle ein eigener Ton, zur Mitte
    heller wie durchscheinend; nahe dem Leuchtfeld glimmen die Ränder und die Haut."""
    rim, value, veins, f1 = _wing_cells(p)
    tone = 0.45 * (value - 0.5) + 0.25 * (1 - _ss(0.0, 0.45, f1)) - 0.7 * rim - 0.6 * veins
    glow = 0.6 * _falloff(near, 0.6) * np.maximum(rim, veins) + 0.2 * _falloff(near, 0.15)
    wet = 0.55 + 0.3 * rim
    height = 0.006 * np.maximum(rim, veins) + 0.003 * (1 - _ss(0.0, 0.45, f1))
    return tone, glow, wet, height


def _web(p, n, near):
    """Äußere Flughaut (dunkles Blaugrün): dieselben Zellen wie die Flughaut, gedämpfter und ohne Leuchten."""
    rim, value, veins, f1 = _wing_cells(p)
    tone = 0.35 * (value - 0.5) + 0.15 * (1 - _ss(0.0, 0.45, f1)) - 0.55 * rim - 0.5 * veins
    wet = 0.4 + 0.3 * rim
    height = 0.006 * np.maximum(rim, veins)
    return tone, 0.12 * _falloff(near, 0.6) * rim, wet, height


def _scale(p, n, near):
    """Schuppen, Eier, Bauchschilde: gewölbte Schuppen mit dunklen Fugen, je Schuppe ein eigener Ton, feine Sprenkel;
    vereinzelte Schuppen glimmen wie Leben unter der Schale."""
    f1, f2, value = _cells(p * 10.0, 61)
    groove = 1 - _ss(0.02, 0.12, f2 - f1)
    dome = _ss(0.0, 0.35, f2 - f1)
    g1, _, speck = _cells(p * 28.0, 62)
    specks = (1 - _ss(0.1, 0.25, g1)) * (speck < 0.35)
    tone = 0.5 * (value - 0.5) - 0.6 * groove - 0.35 * specks + 0.15 * dome
    glow = 0.4 * (value > 0.96) * dome
    wet = 0.6 + 0.4 * dome
    height = 0.008 * dome - 0.004 * groove
    return tone, glow, wet, height


def _organ(p, n, near):
    """Leuchtsäcke: hellere Kerne, dunkle Gefäße, ungleichmäßiges Glühen."""
    f1, _, _ = _cells(_warp(p, 0.2, 1.0, 71) * 2.6, 72)
    cores = 1 - _ss(0.1, 0.5, f1)
    vessels = _lines(_warp(p, 0.3, 0.9, 73) * 2.2, 74, 0.08)
    capillaries = _lines(p * 6.0, 75, 0.1)
    tone = 0.5 * cores - 0.9 * vessels - 0.35 * capillaries + 0.3 * (2 * _fbm(p * 1.4, 76) - 1) - 0.15
    zero = np.zeros(len(p))
    return tone, zero, zero, zero


def _lumen(p, n, near):
    """Leuchtfeld der Flügel: die Zellen der Flughaut, von innen erleuchtet, mit dunklen Rändern."""
    rim, value, veins, f1 = _wing_cells(p)
    tone = 0.3 * (value - 0.5) + 0.35 * (1 - _ss(0.0, 0.45, f1)) - 0.8 * np.maximum(rim, veins) - 0.1
    zero = np.zeros(len(p))
    return tone, zero, zero, zero


PATTERNS = {
    "flesh": _flesh,
    "sinew": _sinew,
    "bone": _bone,
    "membrane": _membrane,
    "web": _web,
    "scale": _scale,
    "organ": _organ,
    "lumen": _lumen,
}
