"""Vanguard (Flaggschiff von Admiralin Vance, „der Hammer“), im Clean-Toon-Stil in der detailreichen Fassung (STYLE.md).

Schlanker, kantiger Kreuzer, der nach vorn drängt: Keilrumpf mit Rückgrat, goldener Zierlinie und Fensterband, davor
der Rammbug als Speerspitze mit leuchtenden Schneiden und Panzerlage in Karmin, darunter zwei schwere Bugkanonen. An den
Flanken gepfeilte Panzerklingen in Karmin, auf dem Rückgrat die Wappenplatte mit Vances Pfeilspitze und zwei Türme (der
hintere überhöht), achtern eine nach hinten gestaffelte Brücke mit Fensterband und Mast, am Heck ein Düsencluster aus
einer großen und vier kleinen Düsen.
Detailzonen mit Zweck (Rammen und schwere Waffen), dazwischen ruhige Flächen (Karmin-Lage des Rammbugs, Rumpfflanken,
Panzerklingen, Rückgrat um das Wappen, Flächen der Brücke):
- Rammbug: dunkler Rammkragen an der Wurzel der Speerspitze, dahinter je Flanke ein Stoßdämpfer (Balg und Zylinder) zum
  Rumpf; drei Spanten unter dem Rammbug
- Bugkanonen: Blendenbalg vor dem Verschluss, Kühlmantel, Laufschelle, zweiteilige Mündungsbremse mit Stahlkern;
  Munitionszuführung vom Rumpf in den Verschluss, Munitionskanal an der Flanke vom Magazin unter den Türmen
- Türme: Barbettenring, Entfernungsmesser-Ohren, Turmluke, Blendenbälge, Mündungsringe (`parts.turret_details`)
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Querträger und Radarplatte
- Heck: Panzerplatte mit Maschinenlüfter auf dem Düsengehäuse, RCS an dessen Flanken, Düsenkranz je Düse
  (`parts.engine_details`, an der großen Düse schmaler, weil die kleinen dicht an ihr sitzen)
- Unterseite: Spanten am V-Boden, zweite Lage unter den unteren Klingen
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import vf
from parts import badge, bellows, engine, engine_details, rcs, sym, trim_line, turret, turret_details, vent
from vf import Ship, box, grow, hexagon, keel, loft, octagon, plate, post, rbox, sec, sphere, tip, tube, wedge

# Abzeichen von Admiralin Vance wie auf ihrer Uniform: Pfeilspitze mit Schaft und Fiederung, zeigt zum Bug
PLATE = sym([(0.0, -0.85), (0.58, -0.3), (0.58, 0.52), (0.0, 0.82)])
ARROW = sym([(0.0, -0.66), (0.4, -0.18), (0.13, -0.18), (0.13, 0.19), (0.36, 0.38), (0.36, 0.56), (0.0, 0.26)])
# Rumpf (y, Breite, Höhe) als Keilprofil; Rammbug (y, Breite, Höhe, dz) als Sechseck hinter der Spitze RAM_TIP
HULL = ((-2.4, 1.8, 1.0), (-1.2, 2.4, 1.3), (2.9, 2.5, 1.35), (3.9, 2.0, 1.1))
RAM = ((-4.7, 0.95, 0.42, -0.05), (-3.5, 2.05, 0.85, 0.0), (-2.6, 1.55, 0.95, 0.05), (-1.6, 1.6, 1.0, 0.08))
RAM_TIP = -5.65
LOWER = [(0.7, -0.9), (1.35, -0.1), (1.85, 2.7), (1.8, 3.5), (0.7, 3.1)]


def lerp(rows, y):
    """Zeile (y, …) der Querschnittstabelle `rows` bei y, linear zwischen den Stützstellen."""
    for a, b in zip(rows, rows[1:]):
        if a[0] <= y <= b[0]:
            t = (y - a[0]) / (b[0] - a[0])
            return [p + t * (q - p) for p, q in zip(a, b)]
    raise ValueError(f"y = {y} liegt außerhalb der Tabelle")


def bottom_z(x, y):
    """Höhe des V-Bodens des Rumpfs bei (x, y), x ≥ 0 (keel: Kielpunkt bei −h/2, Kimmkante bei 0,1 h)."""
    _, w, h = lerp(HULL, y)
    return -h / 2 + 0.6 * h * x / (w / 2)


def core_details(s, m, r, y0, y1):
    """Wie `parts.engine_details` für die große Mitteldüse, nur mit schmalerem Düsenkranz (1,04 r statt 1,16 r): Die
    Glocken der vier kleinen Düsen liegen so dicht an ihr, dass der breite Kranz in sie hineinragte."""
    length = y1 - y0
    ya = y0 + 0.6 * length
    s.add(tube(20, r * 1.09, r * 1.09, y0 + 0.24 * length, y0 + 0.34 * length), m["armor_round"])
    s.add(tube(20, r * 1.04, r * 1.04, ya - 0.12 * r, ya + 0.1 * r), m["dark_round"])


def build():
    s = Ship("vanguard")
    m = vf.materials(s.name, "cruiser")

    # ---------------------------------------------------------------- Rumpf, Rückgrat, Klingen
    # Kantiger Keilrumpf mit Rückgrat, Zierlinie und Fensterband
    s.add(loft([sec(y, keel(w, h)) for y, w, h in HULL]), m["hull"])
    spine = 0.8
    s.add(loft([sec(-3.5, octagon(1.1, 0.34), dz=-0.14), sec(-3.1, octagon(1.75, 0.5)), sec(3.0, octagon(1.75, 0.6)), sec(3.6, octagon(1.1, 0.4))]),
          m["hull"], at=(0, 0, spine))
    trim_line(s, m, [(w / 2 + 0.01, y, h / 2 + 0.005) for y, w, h in HULL])
    s.add(box(0.04, 3.6, 0.08), m["window"], at=(0.885, 0.3, spine), mirror=True)
    # Gepfeilte Panzerklingen in zwei Lagen, unter der unteren eine zweite Platte
    s.add(plate([(1.0, -1.7), (1.6, -0.7), (2.2, 2.4), (2.2, 3.9), (1.7, 3.3), (1.0, 2.9)], 0.22, "xy"), m["accent"], at=(0, 0, 0.2), mirror=True)
    s.add(plate(LOWER, 0.18, "xy"), m["hull"], at=(0, 0, -0.2), mirror=True)
    s.add(plate(grow(LOWER, -0.14), 0.07, "xy"), m["armor"], at=(0, 0, -0.32), mirror=True)

    # ---------------------------------------------------------------- Rammbug
    # Speerspitze: Klinge mit scharfen Seitenkanten, Panzerlage in Karmin, leuchtende Schneiden
    s.add(loft([tip(RAM_TIP, dz=-0.1)] + [sec(y, hexagon(w, h), dz=dz) for y, w, h, dz in RAM]), m["hull"])
    armor = [(-4.5, 0.7, 0.3, 0.12), (-3.5, 1.55, 0.55, 0.25), (-2.5, 1.15, 0.6, 0.32), (-1.7, 1.15, 0.6, 0.34)]
    s.add(loft([tip(-5.2, dz=0.05)] + [sec(y, hexagon(w, h), dz=dz) for y, w, h, dz in armor]), m["accent"])
    edge = [(0.0, RAM_TIP - 0.02, -0.1)] + [(w / 2 + 0.01, y, dz) for y, w, h, dz in RAM[:3]]
    s.add(vf.sweep(edge, vf.rect(0.06, 0.1)), m["glow"], mirror=True)
    # Rammkragen an der Wurzel, dahinter je Flanke ein Stoßdämpfer zum Rumpf; Spanten unter der Klinge
    s.add(loft([sec(y, octagon(2.1, 1.15, 0.35), dz=0.06) for y in (-2.75, -2.38)]), m["dark"])
    dx, dz = 1.02, 0.24
    s.add(bellows(12, 0.12, -2.42, -2.17), m["armor_round"], at=(dx, 0, dz), mirror=True)
    s.add(tube(16, 0.13, 0.13, -2.17, -1.75), m["dark_round"], at=(dx, 0, dz), mirror=True)
    for y in (-4.35, -3.85, -3.35):
        _, _, h, d = lerp(RAM, y)
        s.add(box(0.6, 0.12, 0.1), m["dark"], at=(0, y, d - h / 2 - 0.03))

    # ---------------------------------------------------------------- Bugkanonen unter dem Rammbug
    # Verschluss mit Munitionszuführung vom Rumpf, Blendenbalg, Lauf, Kühlmantel, Laufschelle, zweiteilige
    # Mündungsbremse mit Stahlkern im Spalt und glühendem Kern
    gx, gz = 0.78, -0.62
    s.add(rbox(0.62, 1.7, 0.6, 0.06, 1), m["dark"], at=(gx, -1.9, gz), mirror=True)
    s.add(box(0.3, 0.34, 0.36), m["armor"], at=(gx, -1.45, -0.2), mirror=True)
    s.add(bellows(18, 0.31, -3.0, -2.74), m["armor_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.21, 0.19, -4.75, -2.6), m["steel"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.27, 0.27, -3.5, -3.0), m["dark_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.25, 0.25, -4.05, -3.93), m["armor_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.18, 0.18, -5.0, -4.62), m["steel"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.29, 0.29, -5.04, -4.9), m["dark_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.29, 0.29, -4.78, -4.62), m["dark_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(18, 0.14, 0.14, -5.08, -5.02), m["glow"], at=(gx, 0, gz), mirror=True)
    # Munitionskanal an der Flanke vom Magazin unter den Türmen zur Zuführung, mit Anschlusskästen an beiden Enden
    duct = []
    for y in (1.4, 0.2, -1.35):
        _, w, h = lerp(HULL, y)
        duct.append((w / 2 + 0.03, y, h / 2 - 0.18))
    s.add(vf.sweep(duct, vf.rect(0.1, 0.1)), m["dark"], mirror=True)
    for x, y, z in (duct[0], duct[-1]):
        s.add(box(0.18, 0.3, 0.22), m["armor"], at=(x + 0.01, y, z), mirror=True)

    # ---------------------------------------------------------------- Türme
    # Zwei Türme auf dem Rückgrat, der hintere überhöht auf einer Barbette, beide mit ihren Kleinteilen
    top = spine + 0.25
    turret(s, m, (0, 0.15, top), r=0.46, barrel=1.15)
    turret_details(s, m, (0, 0.15, top), r=0.46, barrel=1.15)
    s.add(post(24, 0.54, 0.52, top, top + 0.34), m["hull_round"], at=(0, 1.35, 0))
    turret(s, m, (0, 1.35, top + 0.34), r=0.46, barrel=1.35)
    turret_details(s, m, (0, 1.35, top + 0.34), r=0.46, barrel=1.35)

    # Wappenplatte vor den Türmen
    badge(s, m, (0, -2.1, top), PLATE, ARROW)

    # ---------------------------------------------------------------- Brücke
    # Nach achtern gestaffelt mit Fensterband, Brückenflügel unter den Seitenfenstern, Mast mit Querträger,
    # Radarplatte und Licht
    z = top
    for w, l, h, y in ((1.6, 1.5, 0.5, 2.65), (1.25, 1.15, 0.45, 2.93), (0.95, 0.85, 0.4, 3.2)):
        s.add(wedge(w, l, h), m["hull"], at=(0, y, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, 3.2 - 0.425 + 0.11, z - 0.16))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.485, 3.25, z - 0.16), mirror=True)
    s.add(box(0.6, 0.28, 0.16), m["hull"], at=(0.68, 3.12, z - 0.335), mirror=True)
    s.add(post(10, 0.09, 0.06, z, z + 0.6), m["hull_round"], at=(0, 3.35, 0))
    s.add(box(0.7, 0.07, 0.07), m["hull"], at=(0, 3.35, z + 0.48))
    s.add(box(0.46, 0.05, 0.18), m["armor"], at=(0, 3.26, z + 0.22), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, 3.35, z + 0.64))

    # ---------------------------------------------------------------- Heck: Düsencluster
    # Gehäuse mit Panzerplatte und Maschinenlüfter, RCS an den Flanken; eine große und vier kleine Düsen
    s.add(rbox(2.6, 0.8, 1.55, 0.08, 1), m["dark"], at=(0, 3.95, 0.0))
    s.add(plate([(-1.0, 3.55), (1.0, 3.55), (1.0, 4.3), (-1.0, 4.3)], 0.06, "xy"), m["armor"], at=(0, 0, 0.8))
    vent(s, m, m["armor"], (0, 4.1, 0.83), 0.32, 0.8, slats=2, rot=(0, 0, 90))
    s.add(box(0.04, 0.66, 0.12), m["signal"], at=(1.3, 3.95, 0.3), mirror=True)
    rcs(s, m, (1.24, 3.75, -0.42), 1)
    engine(s, m, 0.58, 4.0, 5.15)
    core_details(s, m, 0.58, 4.0, 5.15)
    for dz in (0.45, -0.45):
        engine(s, m, 0.38, 4.0, 4.95, dx=0.85, dz=dz, mirror=True)
        engine_details(s, m, 0.38, 4.0, 4.95, dx=0.85, dz=dz, mirror=True)

    # ---------------------------------------------------------------- Unterseite: Kiel, Spanten am V-Boden
    s.add(plate([(-1.4, -0.5), (3.4, -0.5), (3.6, -1.05), (-0.4, -0.95)], 0.14, "yz"), m["dark"])
    for y in (0.3, 0.9, 1.5, 2.1):
        x0, x1 = 0.02, 0.74
        z0, z1 = bottom_z(x0, y), bottom_z(x1, y)
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.12, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    return s.finish()
