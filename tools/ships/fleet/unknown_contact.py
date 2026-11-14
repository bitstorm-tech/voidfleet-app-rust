"""Unbekannter Kontakt (letzter Boss in Sektor 3): kein Schiff der Menschen, sondern ein Wesen. Gewachsen statt gebaut,
Knochen und Fleisch statt Stahlplatten, biolumineszentes Türkis statt Positionslichtern. Die Stilregeln der Flotte
(docs/stilregeln.md: Fasen, Platten, Fenster, Geländer, Türme, Düsen, Beschriftung) gelten hier bewusst nicht, die
Technik schon: geschlossene, nach außen orientierte, sich überlappende Körper, Materialien nach dem Vertrag in vf.py.

Aufbau von vorn nach achtern (Nase −Y, Länge 12, Spannweite knapp 6, kein Teil unter z = −1,05):
- Maul: drei Paar Knochenzangen. Die oberen greifen nach vorn und innen, die unteren hängen tiefer und laufen nach
  vorn unten zusammen (von vorn ein Schnabel, dessen Klauen sich unten fast berühren), außen zwei kürzere Haken.
  Zwischen oberem und unterem Paar bleibt der Schlund offen (`MAW`, dort beginnt sein Strahl). An den Wurzeln der
  Zangen liegt ein Netz aus Knochenbälkchen über den Wangen (schwammiger Knochen).
- Rückgrat: eine pfeilförmige Kopfplatte mit Mittelgrat, deren Spitze vor dem Schädel als Schnabel nach unten biegt,
  dahinter 14 Wirbel als Winkelplatten, die nach achtern kleiner werden, jeder mit einem Dorn nach hinten oben.
  Wirbelplatte und Rippe sind ein Zug (`rib`): Die Platte läuft am Ende in die Rippe aus, die sich um die Flanke
  legt. Die vorderen fünf Rippen sind die großen Bögen des Brustkorbs, die hinteren enden über den Eiern.
- Leib: ein Loft aus Fleisch mit Längswülsten (Muskelstränge), darüber je Seite zwei Sehnenstränge an Schulter und
  Bauchkante. Unter den vorderen Rippen liegt je Seite ein Leuchtsack (`SAC_AT`) mit Adern, an den hinteren Flanken
  zwischen den Strängen eine Traube Eier, am Bauch eine Reihe Schuppenschilde.
- Flügel: ein Paar gepfeilter Sicheln, außen hochgebogen (von vorn Hörner, Spitzen in `WING_TIP`). Knochige
  Vorderkante, dünner Saum innen und hinten, dazwischen die Membran in drei Lagen: dunkle Haut (`web`), darauf
  gedämpftes Türkis (`membrane`), an der Wurzel das Leuchtfeld (`lumen`). Sehnige Streben mit Durchbrüchen tragen den
  Flügel am Schulterstrang und laufen als Adern, die sich gabeln, zur Vorderkante weiter; dahinter zwei Dornen.
- Heck: fünf Tentakel in zwei Höhen, spitz auslaufend, mit Schuppenringen auf dem Rücken. Keine Düsen.

Materialien (alle rund, also mit geglätteten Normalen; Schlüssel für look.WEATHER_MATERIALS): `bone`, `flesh`, `sinew`,
`membrane`, `web`, `scale`; Lichter `organ` (Leuchtsäcke) und `lumen` (Leuchtfeld der Flügel), beide glühend.
"""

import math

import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

import vf
from vf import Ship, loft, revolve, sphere

# ---------------------------------------------------------------- Farben (Licht, Halbschatten, Schatten)

BONE = ("#cdb48e", "#a1845f", "#5f4a3f")  # Elfenbein bis Sandbraun, Schatten ins Violettbraune
FLESH = ("#6e2c42", "#4c1f3a", "#2b142e")  # Weinrot, Schatten ins Dunkelviolette
SINEW = ("#55243c", "#3a1a33", "#211026")  # Sehnen, Streben, Adern: eine Stufe dunkler
MEMBRANE = ("#2a8f84", "#1f6a6c", "#173f4d")  # gedämpftes Türkis der Flughaut
WEB = ("#23565f", "#19404c", "#122a38")  # äußere Flughaut: dunkles Blaugrün
SCALE = ("#86aaa4", "#5d807f", "#364e55")  # Schuppen und Eier: Grautürkis
ORGAN = "#45e0c8"  # Leuchtsäcke
LUMEN = "#45e0c8"  # Leuchtfeld der Flügelwurzel

# ---------------------------------------------------------------- Maße

LOW = -1.05  # tiefster erlaubter Punkt (der Boss wird im Spiel um etwa 1,3 vergrößert)
# Spanten des Leibs: y, halbe Breite, halbe Höhe, Mitte in z
STATIONS = [(-4.55, 0.45, 0.40, -0.06), (-4.1, 0.82, 0.64, -0.03), (-3.5, 1.08, 0.83, 0.0), (-2.6, 1.24, 0.93, 0.0),
            (-1.4, 1.16, 0.90, 0.0), (0.0, 0.92, 0.74, 0.0), (1.4, 0.70, 0.58, 0.02), (2.6, 0.52, 0.45, 0.04),
            (3.5, 0.40, 0.35, 0.05), (4.2, 0.26, 0.24, 0.05)]
HULL_N = 56  # Punkte je Spant
ROUND = 2.3  # Exponent der Superellipse
LOBES, LOBE, TWIST = 5.0, 0.035, 0.22  # Muskelwülste je Seite, ihre Höhe, Drall je Einheit
SAC_AT, SAC_R = (0.6, -2.55, 0.02), (0.8, 1.08, 0.9)  # Leuchtsack Steuerbord: Mitte, Halbachsen
MAW = (0.0, -5.25, -0.5)  # Mitte des Schlunds: unter dem Schnabel, zwischen oberen und unteren Zangen
# Flügel Steuerbord: Vorderkante und Innenkante (ab der Wurzelecke die Hinterkante), beide vom Bugdorn zur Spitze
LEAD = [(1.3, -4.8, 0.15), (1.56, -4.43, 0.24), (1.95, -3.77, 0.45), (2.36, -2.8, 0.70), (2.70, -1.6, 0.92),
        (2.86, -0.63, 1.05), (2.88, 0.33, 1.16), (2.72, 1.3, 1.30), (2.38, 2.14, 1.48), (1.95, 2.92, 1.65)]
INNER = [(1.3, -4.8, 0.15), (1.36, -4.2, 0.22), (1.46, -3.4, 0.30), (1.52, -2.4, 0.34), (1.54, -1.2, 0.36),
         (1.50, -0.2, 0.38), (1.44, 0.45, 0.42), (1.66, 0.95, 0.68), (1.85, 1.5, 1.0), (1.94, 2.2, 1.36), (1.95, 2.92, 1.65)]
WING_TIP = LEAD[-1]
CUP = 2.4  # Wölbung der Flughaut: innen fast flach, zur Vorderkante hochgebogen (die Höhe wächst mit t^CUP)
SKIN = 0.02  # halbe Dicke der Flughaut; jede Lage darüber ist um LAYER dicker
LAYER = 0.012

# ---------------------------------------------------------------- Kurven und Schläuche


def curve(points, s):
    """Punkt bei s (0..1) auf der Catmull-Rom-Kurve durch `points`."""
    p = [Vector(q) for q in points]
    k = len(p) - 1
    u = min(max(s, 0.0), 1.0) * k
    j = min(int(u), k - 1)
    t = u - j
    p0, p1, p2, p3 = p[max(j - 1, 0)], p[j], p[j + 1], p[min(j + 2, k)]
    return 0.5 * (2 * p1 + (p2 - p0) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t * t + (3 * p1 - p0 - 3 * p2 + p3) * t ** 3)


def spline(points, n):
    return [curve(points, i / n) for i in range(n + 1)]


def ramp(values, t):
    """Stützwerte, linear über 0..1 verteilt; so viele Werte wie Punkte gelten je Punkt."""
    u = t * (len(values) - 1)
    j = min(int(u), len(values) - 2)
    return values[j] + (values[j + 1] - values[j]) * (u - j)


def limb(path, width, height=None, n=10, up=(0, 0, 1), ups=None):
    """Schlauch entlang `path` mit ovalem Querschnitt: `width` quer, `height` (sonst wie `width`) in Richtung `up`
    bzw. je Punkt `ups`, beides Stützwerte (`ramp`). Breite 0 an einem Ende ergibt eine Spitze, sonst einen Deckel
    (der gehört in ein anderes Teil gesteckt)."""
    pts = [Vector(p) for p in path]
    height = height or width
    last = len(pts) - 1
    normal = Vector(up)
    rings = []
    for i, p in enumerate(pts):
        t = (pts[min(i + 1, last)] - pts[max(i - 1, 0)]).normalized()
        if ups:
            normal = Vector(ups[i])
        normal = (normal - t * normal.dot(t)).normalized()
        side = t.cross(normal)
        w, h = ramp(width, i / last), ramp(height, i / last)
        if w <= 0:
            rings.append(p.copy())
        else:
            rings.append([p + side * (w * math.cos(a)) + normal * (h * math.sin(a)) for a in (2 * math.pi * k / n for k in range(n))])
    return loft(rings)


# ---------------------------------------------------------------- Haut: Leib und Leuchtsack als Fläche zum Aufsetzen


def hull_ring(y, a, b, c):
    """Spant des Leibs: Superellipse mit Längswülsten, backbord und steuerbord gleich."""
    pts = []
    for k in range(HULL_N):
        phi = 2 * math.pi * k / HULL_N
        cs, sn = math.cos(phi), math.sin(phi)
        m = 1.0 + LOBE * math.cos(2 * LOBES * math.acos(max(-1.0, min(1.0, sn))) + TWIST * y)
        pts.append(Vector((a * m * math.copysign(abs(cs) ** (2 / ROUND), cs), y, c + b * m * math.copysign(abs(sn) ** (2 / ROUND), sn))))
    return pts


def tree_of(*parts):
    """BVH-Baum über mehrere Teile (bmesh)."""
    verts, polys = [], []
    for bm in parts:
        bm.verts.index_update()
        polys += [[len(verts) + v.index for v in f.verts] for f in bm.faces]
        verts += [v.co.copy() for v in bm.verts]
    return BVHTree.FromPolygons(verts, polys)


def skin(tree, y, angle, off=0.0):
    """Punkt auf der Haut am Spant y in Richtung `angle` (Grad ab oben, 90 = Steuerbord), um `off` nach außen
    versetzt, und diese Richtung."""
    a = math.radians(angle)
    d = Vector((math.sin(a), 0.0, math.cos(a)))
    hit = tree.ray_cast(Vector((0.0, y, 0.0)) + d * 6.0, -d)[0]
    return (hit if hit else Vector((0.0, y, 0.0))) + d * off, d


def top_z(tree, x, y):
    """Höhe der Haut über (x, y); vor dem Bug die des nächsten Punkts dahinter."""
    for k in range(40):
        hit = tree.ray_cast(Vector((x, y + 0.05 * k, 4.0)), Vector((0.0, 0.0, -1.0)))[0]
        if hit:
            return hit.z
    return 0.0


# ---------------------------------------------------------------- Rückgrat und Rippen


def rib(tree, y0, chord, arm, reach, sweep, w, h, n=10):
    """Wirbelplatte samt Rippe der Steuerbordseite als ein Zug auf der Haut: am Scheitel die Platte (halbe Tiefe
    `chord`), über `arm` Grad gepfeilt und schmaler, dann die Rippe (halbe Breite `w`, halbe Dicke `h`) bis `reach` Grad,
    in der Flanke um `sweep` nach achtern gebogen, unten spitz. Beginnt knapp jenseits der Mitte: Die gespiegelte
    Hälfte steckt darin."""
    steps = max(8, round((reach + 6) / 7.5))
    path, ups, width, height = [], [], [], []
    for k in range(steps + 1):
        th = -6.0 + (reach + 6.0) * k / steps
        u = max(th, 0.0) / reach
        p, d = skin(tree, y0 + sweep * (1 - (1 - u / 0.55) ** 2), th)
        if th <= arm:
            f = (max(th, 0.0) / arm) ** 0.7
            wd, ht = chord + (w - chord) * f, 0.07 + (h - 0.07) * f
        else:
            f = (th - arm) / (reach - arm)
            wd, ht = w * (1 - f ** 3), h * (1 - 0.5 * f)
        path.append(p + d * (0.6 * ht))
        ups.append(d)
        width.append(wd)
        height.append(ht)
    return limb(path, width, height, n, ups=ups)


def spine(s, m, tree):
    """14 Wirbel hinter der Kopfplatte, nach achtern kleiner: Winkelplatte mit Rippe (`rib`) und ein Dorn nach hinten
    oben. Die vorderen fünf tragen die großen Rippenbögen über den Leuchtsäcken, der sechste reicht noch in die
    Flanke, die folgenden enden am Schulterstrang über den Eiern, die letzten vier sind nur Platte."""
    for i in range(14):
        y, f = -3.0 + 0.5 * i, 1.0 - 0.042 * i
        if i < 5:
            part = rib(tree, y, 0.24 * f, 30, 150, 0.55, 0.09, 0.06, 10)
        elif i == 5:
            part = rib(tree, y, 0.24 * f, 30, 120, 0.4, 0.075, 0.05, 8)
        elif i < 10:
            part = rib(tree, y, 0.24 * f, 30, 66, 0.3, 0.07 * f + 0.01, 0.045, 8)
        else:
            part = rib(tree, y, 0.24 * f, 34, 46, 0.16, 0.12 * f, 0.05, 8)
        s.add(part, m["bone"], mirror=True)
        z = top_z(tree, 0.0, y) + 0.05
        g = f * (1.25 if i < 4 else 1.0)
        thorn = [(0, y - 0.12 * f, z), (0, y + 0.15 * f, z + 0.1 * g), (0, y + 0.56 * g, z + 0.3 * g)]
        s.add(limb(spline(thorn, 6), [0.06 * f, 0.055 * f, 0.0], [0.07 * f, 0.07 * f, 0.0], 8), m["bone"])


def head_plate(tree, half=0.84, tip=-5.3, barb=-2.72, notch=-3.42, rows=17):
    """Pfeilförmige Kopfplatte auf dem Schädel: Spitze zum Bug, Widerhaken nach achtern, Kerbe in der Mitte hinten,
    dazu ein Grat auf der Mittellinie. Vor dem Schädel hängt die Spitze frei über und biegt sich als Schnabel nach unten.
    Ein Loft quer über x; jeder Ring läuft oben von der Vorder- zur Hinterkante und unten zurück."""
    rings = []
    cols = 14
    for j in range(-cols, cols + 1):
        x = half * math.copysign(abs(j / cols) ** 1.4, j)
        f = abs(x) / half
        front, back = tip + (barb - tip) * f ** 1.25, notch + (barb - notch) * f ** 1.6
        if j in (-cols, cols):
            rings.append(Vector((x, barb, top_z(tree, x, barb))))
            continue
        ridge = 0.11 * math.exp(-(x / 0.09) ** 2)
        upper, lower = [], []
        for k in range(rows):
            y = front + (back - front) * k / (rows - 1)
            z = top_z(tree, x, max(y, -4.5)) - 0.75 * max(-4.5 - y, 0.0) ** 2  # vor dem Schädel: Höhe halten, dann abwärts
            arc = math.sin(math.pi * k / (rows - 1))
            upper.append(Vector((x, y, z + (0.05 + 0.07 * (1 - f)) * arc ** 0.5 + ridge * arc - (0.02 if k in (0, rows - 1) else 0.0))))
            lower.append(Vector((x, y, z - 0.08)))
        rings.append(upper + lower[::-1])
    return loft(rings)


# ---------------------------------------------------------------- Maul


def jaws(s, m):
    """Drei Paar Knochenzangen (Modulkommentar), jede mit flachem, klingenartigem Querschnitt; die oberen tragen innen
    einen Widerhaken."""
    upper = [(0.62, -4.1, 0.28), (0.7, -4.85, 0.36), (0.54, -5.5, 0.28), (0.15, -6.0, 0.04)]
    s.add(limb(spline(upper, 20), [0.22, 0.2, 0.15, 0.0], [0.17, 0.15, 0.1, 0.0], 12), m["bone"], mirror=True)
    barb = [(0.58, -5.1, 0.3), (0.34, -5.42, 0.2), (0.2, -5.68, 0.1)]
    s.add(limb(spline(barb, 8), [0.09, 0.06, 0.0], [0.07, 0.05, 0.0], 8), m["bone"], mirror=True)
    lower = [(0.55, -3.95, -0.4), (0.6, -4.75, -0.66), (0.44, -5.35, -0.82), (0.1, -5.8, -0.94)]
    s.add(limb(spline(lower, 20), [0.21, 0.19, 0.13, 0.0], [0.16, 0.13, 0.09, 0.0], 12), m["bone"], mirror=True)
    hook = [(0.93, -3.8, 0.12), (1.22, -4.35, 0.18), (1.26, -4.85, 0.16), (1.03, -5.24, 0.08)]
    s.add(limb(spline(hook, 16), [0.15, 0.13, 0.09, 0.0], [0.12, 0.1, 0.07, 0.0], 10), m["bone"], mirror=True)


def sponge(s, m, tree):
    """Schwammiger Knochen an den Wurzeln der Zangen: ein unregelmäßiges Netz aus gebogenen Knochenbälkchen über der
    Wange, mit Lücken, durch dessen Maschen das Fleisch zu sehen ist."""
    node = {}
    for i in range(6):
        for j in range(5):
            y = -4.45 + 0.2 * i + (0.1 if j % 2 else 0.0) + s.rng.uniform(-0.05, 0.05)
            node[i, j] = skin(tree, y, 48.0 + 21.0 * j + s.rng.uniform(-5.0, 5.0), 0.02)
    for (i, j), (p, d) in node.items():
        s.add(sphere(s.rng.uniform(0.055, 0.075), 1), m["bone"], at=p, mirror=True)
        for other in ((i + 1, j), (i, j + 1), (i + 1 if j % 2 else i - 1, j + 1)):
            if other in node and s.rng.random() > 0.15:
                q, e = node[other]
                r = s.rng.uniform(0.028, 0.048)
                arch = p.lerp(q, 0.5) + (d + e) * s.rng.uniform(0.0, 0.03)
                s.add(limb([p, arch, q], [r, 0.8 * r, r], n=5, up=d), m["bone"], mirror=True)


# ---------------------------------------------------------------- Leib


def strand(tree, y0, y1, a0, a1, w, h, steps=40):
    """Sehnenstrang längs über die Haut von (y0, Winkel a0) nach (y1, a1): leicht gewellt, mit Muskelbäuchen, an beiden
    Enden spitz."""
    path, ups, width, height = [], [], [], []
    for k in range(steps + 1):
        t = k / steps
        y = y0 + (y1 - y0) * t
        swell = min(1.0, 6 * t, 6 * (1 - t)) * (1.0 + 0.22 * math.sin(2.6 * y + a0))
        p, d = skin(tree, y, a0 + (a1 - a0) * t + 3.5 * math.sin(1.9 * y), 0.5 * h * swell)
        path.append(p)
        ups.append(d)
        width.append(w * swell * (1 - 0.35 * t))
        height.append(h * swell * (1 - 0.35 * t))
    return limb(path, width, height, 8, ups=ups)


def sac_point(az, el):
    """Punkt auf dem Leuchtsack (Steuerbord) und die Richtung nach außen: `az` Grad um die Hochachse (0 = querab,
    negativ zum Bug), `el` Grad nach oben."""
    a, e = math.radians(az), math.radians(el)
    d = Vector((math.cos(e) * math.cos(a), math.cos(e) * math.sin(a), math.sin(e)))
    return Vector(SAC_AT) + Vector((d.x * SAC_R[0], d.y * SAC_R[1], d.z * SAC_R[2])), d


def organs(s, m, tree):
    """Adern auf den Leuchtsäcken, je Seite eine Traube Eier an der hinteren Flanke, Schuppenschilde am Bauch."""
    veins = [[(-4, 8), (-30, 24), (-58, 34), (-84, 30)], [(6, 10), (30, 30), (58, 38), (84, 30)], [(-2, 0), (-26, -18), (-54, -30), (-80, -28)],
             [(4, -4), (26, -26), (54, -36), (80, -30)], [(0, 12), (-6, 44), (6, 72)], [(0, -8), (6, -44), (-6, -66)],
             [(-30, 24), (-40, 2), (-62, -8)], [(30, 30), (44, 6), (66, -6)]]
    for vein in veins:
        pairs = [sac_point(*curve([(az, el, 0.0) for az, el in vein], k / 10)[:2]) for k in range(11)]
        s.add(limb([p for p, _ in pairs], [0.05, 0.04, 0.028, 0.0], n=6, ups=[d for _, d in pairs]), m["sinew"], mirror=True)
    # Eier: dicht gepackt in einem länglichen Feld zwischen Schulter- und Bauchstrang, verschieden groß
    egg = [(0.0, -1.0)] + [(math.sin(math.pi * k / 6), -math.cos(math.pi * k / 6)) for k in range(1, 6)] + [(0.0, 1.0)]
    for row in range(4):
        for i in range(8):
            y, angle = 0.3 + 0.27 * i + (0.135 if row % 2 else 0.0), 70.0 + 13.5 * row
            if ((y - 1.3) / 1.15) ** 2 + ((angle - 90.0) / 28.0) ** 2 > 1.0:
                continue
            r = s.rng.uniform(0.1, 0.16) * (1.0 - 0.12 * y)
            at = skin(tree, y + s.rng.uniform(-0.04, 0.04), angle + s.rng.uniform(-3.0, 3.0), 0.25 * r)[0]
            s.add(revolve(egg, 10), m["scale"], at=at, scale=(r, 1.2 * r, r), rot=(s.rng.uniform(-20, 20), 0, s.rng.uniform(-25, 25)), mirror=True)
    for i in range(11):
        y = -3.4 + 0.62 * i
        f = 1.0 - 0.05 * i
        p, _ = skin(tree, y, 180.0)
        s.add(sphere(1.0, 2), m["scale"], at=(0.0, y, max(p.z, LOW + 0.075)), scale=(0.3 * f, 0.36 * f, 0.045), rot=(4, 0, 0))


# ---------------------------------------------------------------- Flügel


def wing(s, t):
    """Punkt der Flughaut (Steuerbord): s läuft vom Bugdorn zur Spitze, t von der Innenkante zur Vorderkante."""
    a, b = curve(INNER, s), curve(LEAD, s)
    p = a.lerp(b, t)
    p.z = a.z + (b.z - a.z) * t ** CUP
    return p


def bulge(sv, s0, s1, power):
    """Bogen über s0..s1: 0 an den Enden, 1 in der Mitte, mit `power` unter 1 fülliger."""
    return max(math.sin(math.pi * (sv - s0) / (s1 - s0)), 0.0) ** power


def web(s0, s1, t0, t1, half, ns=48, nt=8):
    """Lage der Flughaut zwischen s0 und s1 und je s zwischen t0(s) und t1(s), `half` dick nach oben und unten; an
    den Enden läuft sie in eine Spitze."""
    rings = []
    for k in range(ns + 1):
        sv = s0 + (s1 - s0) * k / ns
        a, b = t0(sv), t1(sv)
        if k in (0, ns):
            rings.append(wing(sv, (a + b) / 2))
            continue
        row = [wing(sv, a + (b - a) * j / nt) for j in range(nt + 1)]
        rings.append([p + Vector((0, 0, half)) for p in row] + [p - Vector((0, 0, half)) for p in reversed(row)])
    return loft(rings)


def vein(s0, s1, t0=0.0, t1=1.0, r=0.04, steps=12):
    """Ader in der Flughaut von (s0, t0) nach (s1, t1), zur Spitze hin nach achtern gebogen und dünner; sie ist höher
    als breit und schaut deshalb oben und unten aus der Haut."""
    path = [wing(s0 + (s1 - s0) * (k / steps) ** 1.5, t0 + (t1 - t0) * k / steps) for k in range(steps + 1)]
    return limb(path, [r, 0.85 * r, 0.7 * r, 0.55 * r], [1.9 * r, 1.6 * r, 1.3 * r, 1.0 * r], 6)


def wings(s, m, tree):
    """Sichelflügel (Modulkommentar): Flughaut in drei Lagen, Vorderkante, Saum, Streben zum Schulterstrang, die als
    Adern in die Haut weiterlaufen und sich gabeln, und zwei Dornen hinter der Wurzel."""
    s.add(web(0.0, 1.0, lambda sv: 0.0, lambda sv: 1.0, SKIN, 48, 8), m["web"], mirror=True)
    s.add(web(0.1, 0.8, lambda sv: 0.0, lambda sv: 0.2 + 0.55 * bulge(sv, 0.1, 0.8, 0.5), SKIN + LAYER, 36, 6), m["membrane"], mirror=True)
    s.add(web(0.22, 0.62, lambda sv: 0.0, lambda sv: 0.1 + 0.36 * bulge(sv, 0.22, 0.62, 0.6), SKIN + 2 * LAYER, 28, 5), m["lumen"], mirror=True)
    s.add(limb(spline(LEAD, 48), [0.0, 0.07, 0.1, 0.115, 0.11, 0.09, 0.06, 0.0], n=10), m["flesh"], mirror=True)
    s.add(limb(spline(INNER, 44), [0.0, 0.05, 0.07, 0.075, 0.07, 0.06, 0.05, 0.04, 0.0], n=6), m["flesh"], mirror=True)
    # Streben: breit an Leib und Saum, tailliert dazwischen (die Lücken sind die Durchbrüche); jede läuft als Ader
    # zur Vorderkante weiter und gabelt sich auf halbem Weg
    for sv, to in ((0.19, 0.1), (0.28, 0.2), (0.37, 0.33), (0.46, 0.48), (0.55, 0.66)):
        end = curve(INNER, sv)
        root = skin(tree, end.y - 0.12, 60.0, -0.06)[0]
        path = spline([root, root.lerp(end, 0.55) + Vector((0.0, 0.03, 0.03)), end, wing(sv, 0.14)], 9)
        s.add(limb(path, [0.24, 0.11, 0.16, 0.0], [0.11, 0.06, 0.07, 0.0], 10), m["sinew"], mirror=True)
        s.add(vein(sv, to), m["sinew"], mirror=True)
        mid = sv + (to - sv) * 0.45 ** 1.5
        s.add(vein(mid, to + 0.09, 0.45, 1.0, 0.03, 8), m["sinew"], mirror=True)
    for y, reach, r in ((0.6, 1.35, 0.1), (1.5, 1.1, 0.075)):
        root = skin(tree, y, 60.0, -0.06)[0]
        thorn = [root, root + Vector((0.34, 0.5, 0.12)) * reach, root + Vector((0.42, 1.0, 0.3)) * reach]
        s.add(limb(spline(thorn, 10), [r, 0.75 * r, 0.0], n=8), m["flesh"], mirror=True)


# ---------------------------------------------------------------- Heck


def tentacle(s, m, points, r, mirror, steps=30, collars=7):
    """Tentakel entlang der Punkte, spitz auslaufend; auf dem Rücken Schuppenringe wie die Segmente eines Krebsschwanzes
    (vorn dick, hinten unter dem nächsten), die den Bauch frei lassen."""
    path = spline(points, steps)
    radius = [r * (1 - k / steps) ** 0.75 for k in range(steps + 1)]
    s.add(limb(path, radius, n=10), m["flesh"], mirror=mirror)
    for c in range(collars):
        k = 1 + 3 * c
        rr = radius[k + 1]
        ring = [p + Vector((0.0, 0.0, 0.2 * rr)) for p in path[k:k + 5]]
        s.add(limb(ring, [0.7 * rr, 1.1 * rr, 1.06 * rr, 0.98 * rr, 0.7 * rr], n=10), m["scale"], mirror=mirror)


def tail(s, m):
    """Fünf Tentakel: einer in der Mitte, ein oberes Paar, das sich spreizt und an den Spitzen hebt, ein unteres, kürzeres,
    das sich wie Krallen nach innen biegt."""
    tentacle(s, m, [(0, 3.5, 0.0), (0, 4.4, -0.02), (0, 5.3, 0.06), (0, 6.0, 0.18)], 0.25, False)
    tentacle(s, m, [(0.24, 3.3, 0.12), (0.5, 4.2, 0.2), (0.82, 5.0, 0.3), (0.98, 5.82, 0.56)], 0.21, True)
    tentacle(s, m, [(0.36, 3.0, -0.12), (0.76, 3.9, -0.28), (0.98, 4.7, -0.32), (0.74, 5.4, -0.2)], 0.18, True, collars=6)


# ---------------------------------------------------------------- Zusammenbau


def build():
    s = Ship("unknown_contact")
    m = {
        "bone": vf.toon(f"{s.name}_bone", BONE, round=True),
        "flesh": vf.toon(f"{s.name}_flesh", FLESH, round=True),
        "sinew": vf.toon(f"{s.name}_sinew", SINEW, round=True),
        "membrane": vf.toon(f"{s.name}_membrane", MEMBRANE, round=True),
        "web": vf.toon(f"{s.name}_web", WEB, round=True),
        "scale": vf.toon(f"{s.name}_scale", SCALE, round=True, glint=vf.GLINT),
        "organ": vf.light(f"{s.name}_organ", ORGAN),
        "lumen": vf.light(f"{s.name}_lumen", LUMEN),
    }

    # ---------------------------------------------------------------- Leib und Leuchtsäcke; auf ihrer Haut sitzt der Rest
    hull = loft(vf.smooth([Vector((0, -4.8, -0.08))] + [hull_ring(*station) for station in STATIONS] + [Vector((0, 4.7, 0.06))], 3))
    sac = revolve([(0.0, -1.0)] + [(math.sin(math.pi * k / 16), -math.cos(math.pi * k / 16)) for k in range(1, 16)] + [(0.0, 1.0)], 28)
    bmesh.ops.transform(sac, matrix=Matrix.Translation(SAC_AT) @ Matrix.Diagonal((*SAC_R, 1.0)), verts=sac.verts)
    tree = tree_of(hull, sac)
    s.add(hull, m["flesh"])
    s.add(sac, m["organ"], mirror=True)

    s.add(head_plate(tree), m["bone"])
    spine(s, m, tree)
    jaws(s, m)
    sponge(s, m, tree)
    # Sehnenstränge: Schulter (trägt die Flügel) und Bauchkante, sie rahmen Leuchtsack und Eier
    s.add(strand(tree, -4.0, 3.3, 58, 56, 0.17, 0.11), m["sinew"], mirror=True)
    s.add(strand(tree, -3.9, 3.2, 134, 126, 0.16, 0.1), m["sinew"], mirror=True)
    organs(s, m, tree)
    wings(s, m, tree)
    tail(s, m)

    low = min(v.co.z for v in s.bm.verts)
    assert low >= LOW, f"unknown_contact: tiefster Punkt {low:.3f} liegt unter {LOW}"
    return s.finish()
