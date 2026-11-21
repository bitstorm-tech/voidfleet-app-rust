"""Unbekannter Kontakt v2 (Vergleichsmodell für die Modellschau): weiterentwickelte Fassung des fremden Bosses aus
fleet/unknown_contact.py. Alle Regeln aus docs/stilregeln.md („Das Fremde“) gelten streng: gewachsen statt gebaut,
geschlossene nach außen orientierte Körper, keine Flotten-Details, Budget ≤ 55k Dreiecke, kein Teil unter LOW = −1,05.

Verbesserungen gegenüber unknown_contact.py:
- Flughaut als eine durchgehende, geschlossene, organisch profilierte Membran (`membrane`) statt drei gestapelter
  Stufen-Lofts (`web`/`membrane`/`lumen`): weicher, zellgetragener Verlauf vom tief türkisen, biolumineszenten
  Wurzelfeld bis zum dunklen Sichelrand, mit subtiler Durchschein-Wirkung im Shader (`translucency`).
- Stärkere Sichel-Silhouette: weiter zurückgezogene, höher aufgebogene Flügelspitzen und tieferer konkaver
  Hinterkanten-Ausschnitt, bei voller Lesbarkeit beider Leuchtfelder im Brettwinkel und schlankerem Seitenverhältnis
  (~1 : 0,46).
- Tiefes, gezeichnetes Türkis auf den Leuchtsäcken und im Schlund (`soft_glow`) statt ausgebranntem Cyan, dazu ein
  ruhiger, lebendiger Biopuls (`pulse=1.0`) über Organe, glimmende Adern, Flügelwurzeln und Leuchtpunkte.
- Organisches Knochenschwamm-Netz (`sponge`) an den Wangen: geschwungene, geglättete Knochenbälkchen mit verjüngten
  Taillen statt Kugeln und Stäben; vertiefter, fleischiger Schlund mit inneren Gaumen-Zähnen und tief glimmendem Rachen
  hinter `MAW`.
- Saubere Wirbel-Mittelnaht ohne überlappende Rechteckstufen, profilierte Bauchschilde für die Unterseite im Zoom, und
  sauberer organischer Knochen (`dirt=0.0`) ohne braune Rost-/Schmutzflecken aus der Verwitterungsmaske (Verdeckung
  bleibt voll erhalten).
"""

import math

import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

import vf
from fleet.unknown_contact import (
    BONE,
    FLESH,
    HULL_N,
    LOBE,
    LOBES,
    LOW,
    MAW,
    ROUND,
    SAC_AT,
    SAC_R,
    SCALE,
    SINEW,
    STATIONS,
    TWIST,
    WET,
    curve,
    hull_ring,
    limb,
    sac_point,
    skin,
    spline,
    strand,
    tail,
    top_z,
    tree_of,
)
from vf import Ship, loft, revolve, sphere

# ---------------------------------------------------------------- Farben (satteres, tiefes Türkis für Organe & Membran)

# Membran: deckt den gesamten durchgehenden Flügel ab — die Haut (skin.py) zieht den äußeren Sichelrand über negative
# Zeichnung ins dunkle Blaugrün des Schattentons und lässt die Wurzel tief türkis leuchten.
MEMBRANE_V2 = ("#2c968b", "#1c6769", "#0e2835")
# Organe: tieferes, gesättigteres Türkis, das mit `soft_glow` (ORGAN_GLOW_BOOST) gezeichnet bleibt statt ins Cyan
# auszubrennen.
ORGAN_V2 = "#2ec8b4"
# Ruhiger Biopuls des großen Wesens (Hz-Skala im Shader).
PULSE = 1.0

# ---------------------------------------------------------------- Flügel-Kurven (ausgeprägte Sichel, beide Leuchtfelder lesbar)

# Flügel Steuerbord: schlanker (~5,55 Spannweite bei 12,0 Länge), höher geschwungene Sichelspitzen (z = 2,12) und
# weiter nach achtern gezogene Hörner (y = 3,68), dabei offene, sanft gewölbte Wurzelzone für volle Lesbarkeit im Brettwinkel.
LEAD = [
    (1.26, -4.78, 0.14),
    (1.52, -4.35, 0.24),
    (1.92, -3.62, 0.46),
    (2.30, -2.62, 0.72),
    (2.58, -1.45, 0.98),
    (2.68, -0.35, 1.18),
    (2.64,  0.75, 1.36),
    (2.44,  1.82, 1.58),
    (2.08,  2.80, 1.84),
    (1.62,  3.68, 2.12),
]
INNER = [
    (1.26, -4.78, 0.14),
    (1.32, -4.18, 0.20),
    (1.42, -3.38, 0.28),
    (1.48, -2.38, 0.33),
    (1.48, -1.25, 0.35),
    (1.42, -0.25, 0.36),
    (1.36,  0.42, 0.38),
    (1.60,  1.05, 0.72),
    (1.76,  1.85, 1.18),
    (1.78,  2.78, 1.66),
    (1.62,  3.68, 2.12),
]
WING_TIP = LEAD[-1]
CUP = 2.15  # Innenzone offen und gut lesbar, nach außen steiler in die Sichel hochgebogen
SKIN_ROOT = 0.030  # Dicke nahe der Wurzel und Vorderkante
SKIN_EDGE = 0.015  # dünner, straffer Saum in den Zellfenstern und zur Sichelspitze


# ---------------------------------------------------------------- Rückgrat und Kopfplatte (saubere Mittelnaht)


def rib_v2(tree, y0, chord, arm, reach, sweep, w, h, n=10):
    """Wirbelplatte samt Rippe der Steuerbordseite als ein Zug: beginnt genau auf der Mittellinie (sanft verjüngter
    Ansatz unter dem Rückgratkamm statt eckiger Überlappungsstufe) und schmiegt sich um die Flanke."""
    steps = max(8, round((reach + 2) / 7.5))
    path, ups, width, height = [], [], [], []
    for k in range(steps + 1):
        th = -1.5 + (reach + 1.5) * k / steps
        u = max(th, 0.0) / reach
        p, d = skin(tree, y0 + sweep * (1 - (1 - u / 0.55) ** 2), th)
        if th <= arm:
            f = (max(th, 0.0) / arm) ** 0.7
            wd, ht = chord + (w - chord) * f, 0.072 + (h - 0.072) * f
        else:
            f = (th - arm) / (reach - arm)
            wd, ht = w * (1 - f ** 2.6), h * (1 - 0.52 * f)
        # An der Mittellinie leicht abgerundet einschmiegen, damit der Kamm die Naht sauber deckt
        if k == 0:
            wd *= 0.82
            ht *= 0.78
        path.append(p + d * (0.58 * ht))
        ups.append(d)
        width.append(wd)
        height.append(ht)
    return limb(path, width, height, n, ups=ups)


def spine_v2(s, m, tree):
    """14 Wirbel hinter der Kopfplatte mit durchgehendem organischem Rückenkamm, der die Mittelnaht sauber schließt,
    und nach achtern geschwungenen Dornen."""
    crest_pts = []
    for i in range(14):
        y, f = -3.0 + 0.5 * i, 1.0 - 0.042 * i
        if i < 5:
            part = rib_v2(tree, y, 0.24 * f, 30, 152, 0.56, 0.092, 0.062, 8)
        elif i == 5:
            part = rib_v2(tree, y, 0.24 * f, 30, 122, 0.42, 0.076, 0.052, 7)
        elif i < 10:
            part = rib_v2(tree, y, 0.24 * f, 30, 68, 0.30, 0.072 * f + 0.01, 0.046, 7)
        else:
            part = rib_v2(tree, y, 0.24 * f, 34, 48, 0.16, 0.115 * f, 0.050, 7)
        s.add(part, m["bone"], mirror=True)
        z = top_z(tree, 0.0, y) + 0.055
        crest_pts.append((0.0, y, z + 0.025 * f))
        g = f * (1.28 if i < 4 else 1.02)
        thorn = [(0, y - 0.16 * f, z - 0.01), (0, y + 0.14 * f, z + 0.11 * g), (0, y + 0.58 * g, z + 0.32 * g)]
        s.add(limb(spline(thorn, 5), [0.062 * f, 0.054 * f, 0.0], [0.072 * f, 0.068 * f, 0.0], 7), m["bone"])
    # Durchgehender Wirbelkamm entlang der Mittellinie: bindet die 14 Wirbel organisch zusammen
    crest_path = spline([(0.0, -3.45, top_z(tree, 0.0, -3.45) + 0.04)] + crest_pts + [(0.0, 3.85, top_z(tree, 0.0, 3.85) + 0.01)], 20)
    s.add(limb(crest_path, [0.0, 0.078, 0.068, 0.054, 0.038, 0.0], [0.0, 0.065, 0.058, 0.045, 0.032, 0.0], 6), m["bone"])


def head_plate_v2(tree, half=0.86, tip=-5.32, barb=-2.70, notch=-3.40, rows=17):
    """Pfeilförmige Kopfplatte mit ausgeprägtem Mittelgrat, sanft auslaufenden Seitenkanten und abwärts gebogenem
    Schnabel über dem Schlund."""
    rings = []
    cols = 14
    for j in range(-cols, cols + 1):
        x = half * math.copysign(abs(j / cols) ** 1.4, j)
        f = abs(x) / half
        front, back = tip + (barb - tip) * f ** 1.25, notch + (barb - notch) * f ** 1.6
        if j in (-cols, cols):
            rings.append(Vector((x, barb, top_z(tree, x, barb))))
            continue
        ridge = 0.12 * math.exp(-(x / 0.095) ** 2)
        upper, lower = [], []
        for k in range(rows):
            y = front + (back - front) * k / (rows - 1)
            z = top_z(tree, x, max(y, -4.5)) - 0.72 * max(-4.5 - y, 0.0) ** 2
            arc = math.sin(math.pi * k / (rows - 1))
            upper.append(Vector((x, y, z + (0.052 + 0.072 * (1 - f)) * arc ** 0.5 + ridge * arc - (0.02 if k in (0, rows - 1) else 0.0))))
            lower.append(Vector((x, y, z - 0.085)))
        rings.append(upper + lower[::-1])
    return loft(rings)


# ---------------------------------------------------------------- Maul, Schlund und organischer Knochenschwamm


def jaws_and_maw(s, m):
    """Drei Paar Knochenzangen mit sauber im Kieferknochen verankerten inneren Gaumen-Zähnen, fleischiger Maulwulst
    und tief im dunklen Schlund zurückgesetztem, gedämpft glimmendem Rachen hinter `MAW`."""
    upper = [(0.62, -4.08, 0.28), (0.70, -4.85, 0.36), (0.54, -5.50, 0.28), (0.15, -6.00, 0.04)]
    s.add(limb(spline(upper, 20), [0.22, 0.20, 0.15, 0.0], [0.17, 0.15, 0.10, 0.0], 12), m["bone"], mirror=True)
    barb = [(0.58, -5.08, 0.30), (0.34, -5.42, 0.20), (0.20, -5.68, 0.10)]
    s.add(limb(spline(barb, 8), [0.09, 0.06, 0.0], [0.07, 0.05, 0.0], 8), m["bone"], mirror=True)
    lower = [(0.55, -3.95, -0.40), (0.60, -4.75, -0.66), (0.44, -5.35, -0.82), (0.10, -5.80, -0.94)]
    s.add(limb(spline(lower, 20), [0.21, 0.19, 0.13, 0.0], [0.16, 0.13, 0.09, 0.0], 12), m["bone"], mirror=True)
    hook = [(0.93, -3.80, 0.12), (1.22, -4.35, 0.18), (1.26, -4.85, 0.16), (1.03, -5.24, 0.08)]
    s.add(limb(spline(hook, 16), [0.15, 0.13, 0.09, 0.0], [0.12, 0.10, 0.07, 0.0], 10), m["bone"], mirror=True)

    # Innere Zähne: Wurzeln sitzen exakt auf der Mittelkurve der oberen bzw. unteren Zangen (kein Ablösen im Zoom)
    for jaw_pts, sv, tip_off in (
        (upper, 0.34, Vector((-0.30, -0.10, -0.15))),
        (upper, 0.54, Vector((-0.26, -0.09, -0.13))),
        (lower, 0.36, Vector((-0.28, -0.09, 0.18))),
        (lower, 0.56, Vector((-0.24, -0.08, 0.16))),
    ):
        a = curve(jaw_pts, sv)
        b = a + tip_off
        mid = a.lerp(b, 0.48) + Vector((0.0, 0.05, 0.0))
        s.add(limb(spline([a, mid, b], 4), [0.075, 0.042, 0.0], [0.062, 0.036, 0.0], 6), m["bone"], mirror=True)

    # Fleischige Schlundlippen und dunkler Rachenring: rahmen den tief liegenden Schlund organisch ein
    gum_upper = [(0.0, -4.64, -0.04), (0.38, -4.52, 0.08), (0.64, -4.25, 0.18), (0.82, -3.95, 0.12)]
    s.add(limb(spline(gum_upper, 10), [0.15, 0.17, 0.15, 0.06], [0.13, 0.15, 0.13, 0.05], 8), m["sinew"], mirror=True)
    gum_lower = [(0.0, -4.62, -0.54), (0.34, -4.52, -0.52), (0.56, -4.26, -0.40), (0.74, -3.96, -0.22)]
    s.add(limb(spline(gum_lower, 10), [0.16, 0.17, 0.14, 0.06], [0.14, 0.15, 0.12, 0.05], 8), m["sinew"], mirror=True)
    # Seitliche Schlundfalten schließen den Rachenraum nach links/rechts ab, damit der Kern tief eingebettet wirkt
    pharynx_side = [(0.22, -4.60, -0.06), (0.30, -4.56, -0.28), (0.22, -4.58, -0.50)]
    s.add(limb(spline(pharynx_side, 6), [0.10, 0.13, 0.10], [0.11, 0.13, 0.11], 7), m["sinew"], mirror=True)

    # Tiefliegender, zurückgesetzter Rachenkern hinter den Lippen (MAW = -5,25 bleibt weit davor frei)
    throat = sphere(1.0, 2)
    s.add(throat, m["organ"], at=(0.0, -4.50, -0.29), scale=(0.24, 0.16, 0.21))


def sponge_v2(s, m, tree):
    """Organischer Knochenschwamm (Trabekel-Netz) an den Wangen: unregelmäßig gewachsene, geschwungene Knochenbälkchen
    mit breit verschmolzenen Knotenwurzeln und schlanken Taillen — eng an die Wangenhaut geschmiegt, ohne abstehende
    Kugeln oder Zäune."""
    node = {}
    for i in range(6):
        for j in range(5):
            # Stärker organischer Versatz statt starrem Gitter
            y = -4.42 + 0.205 * i + (0.095 if j % 2 else 0.0) + s.rng.uniform(-0.050, 0.050)
            angle = 53.0 + 19.5 * j + s.rng.uniform(-5.5, 5.5)
            interior = 0 < i < 5 and 0 < j < 4
            off = 0.006 if interior else (-0.052 if i == 0 else -0.036)
            node[i, j] = skin(tree, y, angle, off)

    # Geschwungene Trabekel-Bälkchen: breit gefächerte Wurzeln (verschmelzen organisch an den Knoten ohne extra Kugeln)
    # und verjüngte konkave Taillen dazwischen
    for (i, j), (p, d) in node.items():
        neighbors = ((i + 1, j), (i, j + 1), (i + 1 if j % 2 else i - 1, j + 1))
        for other in neighbors:
            if other not in node or s.rng.random() < 0.18:
                continue
            q, e = node[other]
            primary = other[1] == j
            r = s.rng.uniform(0.038, 0.054) if primary else s.rng.uniform(0.026, 0.040)
            mid_dir = (d + e).normalized()
            chord = q - p
            tangent = chord.cross(mid_dir)
            if tangent.length > 1e-5:
                tangent.normalize()
            bow = s.rng.uniform(-0.034, 0.034)
            lift = s.rng.uniform(0.008, 0.022)
            p1 = p.lerp(q, 0.30) + mid_dir * (0.75 * lift) + tangent * bow
            p2 = p.lerp(q, 0.70) + mid_dir * (0.75 * lift) - tangent * (0.55 * bow)
            path = spline([p - d * 0.015, p1, p2, q - e * 0.015], 3)
            widths = [1.45 * r, 0.78 * r, 0.56 * r, 0.78 * r, 1.45 * r]
            heights = [1.05 * r, 0.65 * r, 0.52 * r, 0.65 * r, 1.05 * r]
            s.add(limb(path, widths, heights, n=6, up=mid_dir), m["bone"], mirror=True)

    # Kräftige Jochbein-Bögen verankern das Netz an den Zangenwurzeln und rahmen die Wange sauber ein
    zyg_up = [skin(tree, -3.52, 55.0, -0.025)[0], skin(tree, -3.96, 65.0, 0.035)[0], Vector((0.68, -4.35, 0.24))]
    s.add(limb(spline(zyg_up, 6), [0.06, 0.078, 0.095], [0.045, 0.058, 0.075], 8), m["bone"], mirror=True)
    zyg_low = [skin(tree, -3.52, 118.0, -0.025)[0], skin(tree, -3.95, 114.0, 0.030)[0], Vector((0.56, -4.22, -0.44))]
    s.add(limb(spline(zyg_low, 6), [0.055, 0.070, 0.088], [0.042, 0.054, 0.070], 8), m["bone"], mirror=True)


# ---------------------------------------------------------------- Organe, Eier und Unterseite


def belly_scute(width, length, thickness):
    """Überlappender, geschwungener Bauchschild mit leichtem Mittelkiel (geschlossener Loft quer über X)."""
    rings = []
    cols = 5
    rows = 5
    for j in range(-cols, cols + 1):
        u = j / cols
        x = width * u
        f = abs(u)
        y0 = -length * (0.48 - 0.14 * f * f)
        y1 = length * (0.56 - 0.34 * f ** 1.4)
        if j in (-cols, cols):
            rings.append(Vector((x, 0.5 * (y0 + y1), 0.03 * width)))
            continue
        arch = 0.12 * width * f * f
        keel = 0.022 * math.exp(-(f / 0.25) ** 2)
        upper, lower = [], []
        for k in range(rows):
            v = k / (rows - 1)
            y = y0 + (y1 - y0) * v
            env = math.sin(math.pi * v) ** 0.55
            lower.append(Vector((x, y, arch - (thickness * env + keel * env))))
            upper.append(Vector((x, y, arch + 0.55 * thickness)))
        rings.append(upper + lower[::-1])
    return loft(rings)


def organs_v2(s, m, tree):
    """Adern auf den Leuchtsäcken, Eiertraube an der hinteren Flanke, profilierte Bauchschilde und ventrale
    Sehnenrippen auf der Unterseite (alle Teile streng ≥ LOW = −1,05)."""
    veins = [
        [(-4, 8), (-30, 24), (-58, 34), (-84, 30)],
        [(6, 10), (30, 30), (58, 38), (84, 30)],
        [(-2, 0), (-26, -18), (-54, -30), (-80, -28)],
        [(4, -4), (26, -26), (54, -36), (80, -30)],
        [(0, 12), (-6, 44), (6, 72)],
        [(0, -8), (6, -44), (-6, -66)],
        [(-30, 24), (-40, 2), (-62, -8)],
        [(30, 30), (44, 6), (66, -6)],
    ]
    for vein_pts in veins:
        pairs = [sac_point(*curve([(az, el, 0.0) for az, el in vein_pts], k / 10)[:2]) for k in range(11)]
        s.add(limb([p for p, _ in pairs], [0.05, 0.04, 0.028, 0.0], n=6, ups=[d for _, d in pairs]), m["sinew"], mirror=True)

    # Eiertraube zwischen Schulter- und Bauchstrang
    egg = [(0.0, -1.0)] + [(math.sin(math.pi * k / 6), -math.cos(math.pi * k / 6)) for k in range(1, 6)] + [(0.0, 1.0)]
    for row in range(4):
        for i in range(8):
            y, angle = 0.3 + 0.27 * i + (0.135 if row % 2 else 0.0), 70.0 + 13.5 * row
            if ((y - 1.3) / 1.15) ** 2 + ((angle - 90.0) / 28.0) ** 2 > 1.0:
                continue
            r = s.rng.uniform(0.10, 0.16) * (1.0 - 0.12 * y)
            at = skin(tree, y + s.rng.uniform(-0.04, 0.04), angle + s.rng.uniform(-3.0, 3.0), 0.22 * r)[0]
            s.add(revolve(egg, 10), m["scale"], at=at, scale=(r, 1.2 * r, r), rot=(s.rng.uniform(-20, 20), 0, s.rng.uniform(-25, 25)), mirror=True)

    # Profilierte, geschuppte Bauchschilde entlang der Unterseite (überlappend, mit feinem Mittelkiel)
    for i in range(12):
        y = -3.6 + 0.60 * i
        f = 1.0 - 0.048 * i
        p, _ = skin(tree, y, 180.0)
        thickness = 0.048 * (0.75 + 0.25 * f)
        z = max(p.z + 0.015, LOW + thickness + 0.03)
        s.add(belly_scute(0.36 * f, 0.68 * f, thickness), m["scale"], at=(0.0, y, z), rot=(3.5, 0, 0))

    # Paar schlanke ventrale Sehnenstränge eng an der Bauchflanke (ohne Überbrücken der vorderen Kerbe)
    s.add(strand(tree, -3.5, 3.0, 152, 148, 0.075, 0.048, steps=20), m["sinew"], mirror=True)


# ---------------------------------------------------------------- Flügel (ein durchgehender, nahtloser Sichel-Loft)


def wing(sv, tv):
    """Punkt der Flughaut (Steuerbord): sv vom Bugdorn zur Sichelspitze (0..1), tv von der Innenkante zur Vorderkante
    (0..1); mit sanfter organischer Wölbung zwischen den Adern."""
    a, b = curve(INNER, sv), curve(LEAD, sv)
    p = a.lerp(b, tv)
    p.z = a.z + (b.z - a.z) * (tv ** CUP)
    billow = 0.020 * math.sin(math.pi * tv) * math.sin(math.pi * sv) * (0.5 + 0.5 * math.sin(sv * 22.0))
    p.z += billow
    return p


def wing_membrane(ns=48, nt=9):
    """Eine einzige geschlossene, durchgehende Flughaut ohne Stufen: dicker an Wurzel und Vorderkante, zu einem
    straffen, dünnen Häutchen in den Zellfenstern und zur Sichelspitze hin verjüngt."""
    rings = []
    for k in range(ns + 1):
        sv = k / ns
        if k in (0, ns):
            rings.append(wing(sv, 0.5))
            continue
        span_taper = math.sin(math.pi * sv) ** 0.35
        upper, lower = [], []
        for j in range(nt + 1):
            tv = j / nt
            p = wing(sv, tv)
            edge_body = 1.0 - 0.42 * math.sin(math.pi * tv)
            half = (SKIN_EDGE + (SKIN_ROOT - SKIN_EDGE) * edge_body) * (0.45 + 0.55 * span_taper)
            p_next = wing(sv, min(tv + 0.02, 1.0))
            p_prev = wing(sv, max(tv - 0.02, 0.0))
            p_aft = wing(min(sv + 0.015, 1.0), tv)
            p_fore = wing(max(sv - 0.015, 0.0), tv)
            n = (p_aft - p_fore).cross(p_next - p_prev)
            if n.length > 1e-6:
                n.normalize()
                if n.z < 0:
                    n = -n
            else:
                n = Vector((0.0, 0.0, 1.0))
            upper.append(p + n * half)
            lower.append(p - n * half)
        rings.append(upper + lower[::-1])
    return loft(rings)


def wing_vein(s0, s1, t0=0.0, t1=1.0, r=0.036, steps=12):
    """Geschwungene Flügelader in der Membran von (s0, t0) nach (s1, t1), elegant nach achtern gebogen und verjüngt;
    bleibt auch an der dickeren Vorderkante (SKIN_ROOT) sauber sichtbar bis in den Rand."""
    path = [wing(s0 + (s1 - s0) * (k / steps) ** 1.45, t0 + (t1 - t0) * k / steps) for k in range(steps + 1)]
    tip_h = max(1.35 * r, SKIN_ROOT + 0.008)
    return limb(path, [r, 0.84 * r, 0.68 * r, 0.52 * r], [1.85 * r, 1.65 * r, 1.48 * r, tip_h], 6)


def wings_v2(s, m, tree):
    """Sichelflügel mit einer einzigen nahtlosen, durchscheinenden Membran (`membrane`), kräftiger Vorderkante,
    schlankem Innensaum, organisch verjüngten Schulterstreben, sauber gegabelten Adern und zwei Sicheldornen."""
    s.add(wing_membrane(48, 9), m["membrane"], mirror=True)
    # Vorderkante: kräftiger Sichelbogen, messerscharf an Bugdorn und Sichelspitze
    s.add(limb(spline(LEAD, 48), [0.0, 0.068, 0.10, 0.118, 0.112, 0.095, 0.068, 0.036, 0.0], n=10), m["flesh"], mirror=True)
    # Schlanker Knochenkamm, fest in die obere Schulter der Vorderkante eingebettet
    crest_lead = [curve(LEAD, 0.10 + 0.56 * k / 20) + Vector((-0.02, 0.0, 0.032 * math.sin(math.pi * k / 20))) for k in range(21)]
    s.add(limb(crest_lead, [0.0, 0.044, 0.052, 0.040, 0.022, 0.0], [0.0, 0.040, 0.054, 0.042, 0.022, 0.0], 6), m["bone"], mirror=True)
    # Innensaum & konkave Hinterkante
    s.add(limb(spline(INNER, 44), [0.0, 0.045, 0.065, 0.068, 0.062, 0.052, 0.042, 0.032, 0.0], n=6), m["flesh"], mirror=True)

    # Schulterstreben: organisch tailliert; jede läuft als Hauptader bis in die Vorderkante und gabelt sich exakt auf
    # ihrer eigenen Bahn
    for sv, to in ((0.18, 0.12), (0.27, 0.23), (0.36, 0.36), (0.45, 0.51), (0.54, 0.68)):
        end = curve(INNER, sv)
        root = skin(tree, end.y - 0.12, 58.0, -0.06)[0]
        path = spline([root, root.lerp(end, 0.52) + Vector((0.0, 0.03, 0.03)), end, wing(sv, 0.16)], 8)
        s.add(limb(path, [0.18, 0.092, 0.105, 0.040, 0.0], [0.092, 0.050, 0.056, 0.034, 0.0], 8), m["sinew"], mirror=True)
        s.add(wing_vein(sv, to, 0.0, 1.0, 0.036, 12), m["sinew"], mirror=True)
        # Gabelung exakt auf der Hauptader bei t = 0,42
        mid = sv + (to - sv) * 0.42 ** 1.45
        s.add(wing_vein(mid, min(to + 0.10, 0.96), 0.42, 1.0, 0.026, 8), m["sinew"], mirror=True)
        # Am hintersten Strang ein zweiter sauberer Abzweig in die Sichelspitze (direkt von der Hauptader bei t = 0,24)
        if sv > 0.5:
            mid_aft = sv + (to - sv) * 0.24 ** 1.45
            s.add(wing_vein(mid_aft, 0.84, 0.24, 1.0, 0.024, 9), m["sinew"], mirror=True)

    # Zwei nach achtern geschwungene Dornen hinter der Flügelwurzel, folgen der Sichel-Bewegung
    for y, reach, r in ((0.62, 1.45, 0.095), (1.52, 1.18, 0.072)):
        root = skin(tree, y, 58.0, -0.06)[0]
        thorn = [root, root + Vector((0.36, 0.52, 0.15)) * reach, root + Vector((0.40, 1.08, 0.38)) * reach]
        s.add(limb(spline(thorn, 9), [r, 0.72 * r, 0.0], n=7), m["flesh"], mirror=True)


# ---------------------------------------------------------------- Zusammenbau


def build():
    s = Ship("unknown_contact_v2")
    # Sauberer Knochen (`dirt=0.0`), kaum Schmutz/keine Schlieren auf Fleisch/Sehnen/Flughaut/Schuppen (`dirt=0.08`),
    # subtile Durchschein-Wirkung auf der nahtlosen Flughaut (`translucency=0.85`), gedämpftes tief-türkises
    # Organleuchten (`soft_glow=True`) und ruhiger lebendiger Biopuls (`pulse=PULSE`).
    m = {
        "bone": vf.toon(f"{s.name}_bone", BONE, round=True, glint=WET["bone"], dirt=0.0),
        "flesh": vf.toon(f"{s.name}_flesh", FLESH, round=True, glint=WET["flesh"], dirt=0.08, pulse=PULSE),
        "sinew": vf.toon(f"{s.name}_sinew", SINEW, round=True, glint=WET["sinew"], dirt=0.08, pulse=PULSE),
        "membrane": vf.toon(
            f"{s.name}_membrane",
            MEMBRANE_V2,
            round=True,
            glint=WET["membrane"],
            dirt=0.05,
            translucency=0.85,
            pulse=PULSE,
        ),
        "scale": vf.toon(f"{s.name}_scale", SCALE, round=True, glint=WET["scale"], dirt=0.08, pulse=PULSE),
        "organ": vf.light(f"{s.name}_organ", ORGAN_V2, glow=True, soft_glow=True, pulse=PULSE),
    }

    hull = loft(vf.smooth([Vector((0, -4.8, -0.08))] + [hull_ring(*station) for station in STATIONS] + [Vector((0, 4.7, 0.06))], 2))
    sac = revolve([(0.0, -1.0)] + [(math.sin(math.pi * k / 16), -math.cos(math.pi * k / 16)) for k in range(1, 16)] + [(0.0, 1.0)], 28)
    bmesh.ops.transform(sac, matrix=Matrix.Translation(SAC_AT) @ Matrix.Diagonal((*SAC_R, 1.0)), verts=sac.verts)
    tree = tree_of(hull, sac)
    s.add(hull, m["flesh"])
    s.add(sac, m["organ"], mirror=True)

    s.add(head_plate_v2(tree), m["bone"])
    spine_v2(s, m, tree)
    jaws_and_maw(s, m)
    sponge_v2(s, m, tree)
    s.add(strand(tree, -4.0, 3.3, 58, 56, 0.17, 0.11), m["sinew"], mirror=True)
    s.add(strand(tree, -3.9, 3.2, 134, 126, 0.16, 0.10), m["sinew"], mirror=True)
    organs_v2(s, m, tree)
    wings_v2(s, m, tree)
    tail(s, m)

    low = min(v.co.z for v in s.bm.verts)
    assert low >= LOW, f"unknown_contact_v2: tiefster Punkt {low:.3f} liegt unter {LOW}"
    return s.finish()
