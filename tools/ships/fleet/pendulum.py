"""Pendulum (Flaggschiff von Admiral Kerr, „der Uhrmacher“), Clean Toon in der detailreichen Fassung (STYLE.md).

Railgun-Schlachtschiff auf einem schmalen Keilrumpf. Die Railgun ist das Rückgrat: ein Gehäuse in der Akzentfarbe auf
dem Deck, vorn ragen ihre Schienen weit über den Bug, umwickelt von fünf groben Spulen im Wechsel Messing und Signalblau,
mit Mündungsjoch. Um den Rumpf das Gimbal: ein senkrechter Messing-Zahnkranz mit sechzehn groben Zähnen auf Zapfen an den
Flanken, darin ein um die Hochachse gedrehter Spulenring, der auf dem Brett als Ellipse zu sehen ist. Unter dem Kiel
schwingt das Pendel nach achtern. Auf dem Verschluss das Wappen Kerrs (Zahnrad, Zifferblatt, Zeiger), achtern die
Brücke aus drei Keilen mit Fensterband und Mast, Panzerschürzen, Zierlinie, Kiel und vier Düsen.
Detailzonen mit Zweck (Präzisionsmechanik), dazwischen ruhige Flächen (Deck hinter dem Gimbal, über den Schürzen
liegende Flanken, Gehäuse zwischen Gimbal und Verschluss, Flächen der Brücke):
- Bug: Bugpanzer-Kappe mit RCS-Blöcken; Kühlmantel mit Spannband, wo die Schienen aus dem Gehäuse treten; Stirnplatte
  am Mündungsjoch
- Railgun: zwei Kondensatorbänke als Satteltaschen am Gehäuse mit je drei Kondensatoren, versetzt wie der Spulenring
  (backbord vor dem Gimbal, steuerbord dahinter), wo er Platz lässt
- Gimbal: an jedem Zapfen eine Lagerplatte, unten als Zahnsegment, mit Stellmotor (Schneckentrieb) dahinter
- Flanke: Kabelkanal unter der Zierlinie vom Anschlusskasten achtern zum Stellmotor
- Verschluss: Maschinenlüfter beiderseits auf dem Deck
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte und Querträger, RCS-Blöcke daneben
- Heck: Lüfter auf dem Düsenblock, Gehäusesegment und Düsenkranz je Düse (`parts.engine_details`)
- Unterseite: Pendelbock mit Lagerdeckeln an der Pendelachse, Spanten am V-Boden des Bugs
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import math

from mathutils import Vector

import palette
import vf
from parts import engine, engine_details, rcs, trim_line, vent
from vf import Ship, box, flank, hoop, keel, loft, octagon, plate, post, rect, revolve, sec, segment, sphere, sweep, tube, wedge

ZC = 1.0  # Rumpfmitte; Deck bei ZC + 0,75
DECK = ZC + 0.75
ZR = 2.0  # Achse der Railgun
GY, GZ = -1.2, 1.3  # Mitte des Gimbals
# Rumpf: (y, Breite, Höhe, Anhebung) des Keilprofils
HULL = ((-3.3, 1.3, 0.9, 0.15), (-2.5, 2.6, 1.3, 0.1), (-0.8, 3.2, 1.5, 0.0), (2.6, 3.2, 1.5, 0.0), (3.8, 2.6, 1.3, 0.1),
        (4.3, 2.0, 1.0, 0.25))


def bottom_z(x, y):
    """Höhe des V-Bodens bei (x, y), x ≥ 0, zwischen zwei Querschnitten von HULL."""
    for (y0, w0, h0, d0), (y1, w1, h1, d1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            w, h, dz = w0 + t * (w1 - w0), h0 + t * (h1 - h0), d0 + t * (d1 - d0)
            keel_z, side_z = ZC + dz - h / 2, ZC + dz + h / 2 - 0.4 * h
            return keel_z + (side_z - keel_z) * x / (w / 2)
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def gear(teeth, r_root, r_tip, width=0.5):
    """Zahnrad-Umriss (x, y) mit `teeth` trapezförmigen Zähnen zwischen r_root und r_tip; `width` ist der Anteil des
    Zahns an der Teilung."""
    pts = []
    for k in range(teeth):
        a = 2 * math.pi * k / teeth
        half = math.pi / teeth * width
        for angle, radius in ((a - half * 1.25, r_root), (a - half * 0.75, r_tip), (a + half * 0.75, r_tip), (a + half * 1.25, r_root)):
            pts.append((radius * math.cos(angle), radius * math.sin(angle)))
    return pts


def gear_ring(teeth, r_in, r_root, r_tip, depth):
    """Zahnkranz um die Z-Achse: innen rund (Radius r_in), außen Zähne, `depth` dick."""
    outer = gear(teeth, r_root, r_tip)
    inner = [(r_in * x / math.hypot(x, y), r_in * y / math.hypot(x, y)) for x, y in outer]
    h = depth / 2
    rings = [[Vector((x, y, z)) for x, y in pts] for pts, z in ((inner, -h), (outer, -h), (outer, h), (inner, h), (inner, -h))]
    return loft(rings, cap_start=False, cap_end=False)


def segment_plate(r, r_tip, teeth, n=12):
    """Umriss (y, z) einer Lagerplatte um den Ursprung: obere Hälfte rund mit Radius r, untere Hälfte ein Zahnsegment mit
    `teeth` trapezförmigen Zähnen bis r_tip (Winkel 0 zeigt nach achtern, 90° nach oben)."""
    pts = [(r * math.cos(math.pi * k / n), r * math.sin(math.pi * k / n)) for k in range(n + 1)]
    pitch = math.pi / teeth
    for k in range(teeth):
        a = math.pi + pitch * (k + 0.5)
        for angle, radius in ((a - pitch * 0.32, r), (a - pitch * 0.2, r_tip), (a + pitch * 0.2, r_tip), (a + pitch * 0.32, r)):
            pts.append((radius * math.cos(angle), radius * math.sin(angle)))
    return pts


def emblem(s, m, at):
    """Wappen Kerrs liegend: Messing-Zahnrad, Zifferblatt in Signalblau, zwei goldene Zeiger."""
    x, y, z = at
    s.add(plate(gear(12, 0.6, 0.8), 0.14, "xy"), m["brass"], at=(x, y, z + 0.02))
    s.add(post(24, 0.46, 0.46, 0.0, 0.05), m["signal"], at=(x, y, z + 0.07))
    for length, width, angle in ((0.4, 0.13, 25), (0.28, 0.17, 145)):
        hand = [(-width / 2, -0.06), (width / 2, -0.06), (width / 4, length), (-width / 4, length)]
        a = math.radians(angle)
        hand = [(px * math.cos(a) - py * math.sin(a), px * math.sin(a) + py * math.cos(a)) for px, py in hand]
        s.add(plate(hand, 0.05, "xy"), m["trim"], at=(x, y, z + 0.13))
    s.add(post(12, 0.08, 0.08, 0.0, 0.09), m["trim"], at=(x, y, z + 0.11))


def capacitors(s, m, x, y):
    """Kondensatorbank als Satteltasche an der Flanke des Railgun-Gehäuses (x > 0 steuerbord, x < 0 backbord): Kasten in
    `armor`, darauf drei Kondensatoren."""
    side = 1 if x > 0 else -1
    s.add(box(0.36, 0.9, 0.36), m["armor"], at=(x, y, DECK + 0.15))
    for dy in (-0.3, 0.0, 0.3):
        s.add(post(16, 0.11, 0.11, DECK + 0.31, DECK + 0.44), m["dark_round"], at=(x + side * 0.055, y + dy, 0))


def build():
    s = Ship("pendulum")
    m = vf.materials(s.name, "battleship")
    m["brass"] = vf.toon(f"{s.name}_brass", palette.BRASS)
    m["brass_round"] = vf.toon(f"{s.name}_brass_round", palette.BRASS, round=True, glint=vf.GLINT)

    # ---------------------------------------------------------------- Rumpf, Panzerlagen, Zierlinie
    hull = [sec(y, keel(w, h), dz=ZC + dz) for y, w, h, dz in HULL]
    s.add(loft(hull), m["hull"])
    trim_line(s, m, flank(hull, DECK - 0.12, 0.02))
    # Bugpanzer: Kappe um die Bugspitze
    s.add(loft([sec(-3.36, keel(1.42, 1.0), dz=ZC + 0.15), sec(-2.75, keel(2.3, 1.29), dz=ZC + 0.108)]), m["armor"])
    # Panzerschürzen vor und hinter dem Gimbal
    s.add(plate([(1.15, -2.95), (1.62, -2.2), (1.7, -1.75), (1.3, -1.75)], 0.18, "xy"), m["armor"], at=(0, 0, ZC + 0.05), mirror=True)
    s.add(plate([(1.3, -0.65), (1.75, -0.65), (1.75, 2.9), (1.45, 3.75), (1.2, 3.75)], 0.18, "xy"), m["armor"], at=(0, 0, ZC + 0.05), mirror=True)

    # ---------------------------------------------------------------- Railgun: Gehäuse, Verschluss, Schienen, Spulen, Joch
    s.add(loft([sec(-3.15, octagon(0.9, 0.6, 0.35), dz=ZR - 0.1), sec(-2.5, octagon(1.05, 0.72, 0.35), dz=ZR),
                sec(0.6, octagon(1.05, 0.72, 0.35), dz=ZR)]), m["accent"])
    s.add(box(0.09, 3.0, 0.05), m["glow"], at=(0, -1.1, ZR + 0.36))
    s.add(loft([sec(0.45, octagon(1.6, 0.66, 0.4), dz=ZR - 0.02), sec(0.75, octagon(1.95, 0.76, 0.35), dz=ZR),
                sec(2.3, octagon(1.95, 0.76, 0.35), dz=ZR)]), m["dark"])
    for x in (0.25, -0.25):
        s.add(box(0.17, 3.9, 0.5), m["steel"], at=(x, -3.65, ZR))
    for k, y in enumerate((-3.45, -3.9, -4.35, -4.8, -5.25)):
        s.add(hoop(24, 0.34, 0.6, 0.22), m["signal" if k % 2 else "brass_round"], at=(0, y, ZR), rot=(90, 0, 0))
    s.add(loft([sec(-5.8, octagon(0.86, 0.86, 0.4)), sec(-5.72, octagon(1.02, 1.02, 0.4)), sec(-5.47, octagon(1.02, 1.02, 0.4))]), m["dark"],
          at=(0, 0, ZR))
    # Stirnplatte des Jochs um die Mündung
    s.add(loft([sec(-5.84, octagon(0.7, 0.7, 0.4)), sec(-5.79, octagon(0.7, 0.7, 0.4))]), m["armor"], at=(0, 0, ZR))
    s.add(box(0.3, 0.06, 0.48), m["signal"], at=(0, -5.85, ZR))
    # Kühlmantel um die Gehäusespitze, wo die Schienen austreten, mit hellem Spannband
    s.add(loft([sec(-3.2, octagon(1.0, 0.7, 0.35), dz=ZR - 0.1), sec(-2.62, octagon(1.14, 0.82, 0.35), dz=ZR - 0.02)]), m["dark"])
    s.add(loft([sec(-2.98, octagon(1.15, 0.84, 0.35), dz=ZR - 0.07), sec(-2.86, octagon(1.17, 0.86, 0.35), dz=ZR - 0.053)]), m["armor"])
    # Kondensatorbänke, versetzt wie der Spulenring: backbord vor dem Gimbal, steuerbord dahinter
    capacitors(s, m, -0.68, -2.0)
    capacitors(s, m, 0.68, -0.4)

    # ---------------------------------------------------------------- Gimbal: Zahnkranz auf Zapfen, gedrehter Spulenring
    s.add(gear_ring(16, 1.8, 2.02, 2.26, 0.3), m["brass"], at=(0, GY, GZ), rot=(90, 0, 0))
    s.add(tube(20, 0.19, 0.19, 1.4, 1.92), m["brass_round"], at=(0, GY, GZ), rot=(0, 0, -90), mirror=True)
    s.add(hoop(32, 1.42, 1.6, 0.2), m["accent"], at=(0, GY, GZ), rot=(90, 0, -35))
    s.add(hoop(32, 1.28, 1.43, 0.14), m["signal"], at=(0, GY, GZ), rot=(90, 0, -35))
    for z0, z1 in ((GZ + 1.5, GZ + 1.88), (GZ - 1.88, GZ - 1.5)):
        s.add(post(12, 0.11, 0.11, z0, z1), m["brass_round"], at=(0, GY, 0))
    # Lagerplatte an jedem Zapfen, unten als Zahnsegment, dahinter der Stellmotor, der es über eine Schnecke dreht
    s.add(plate(segment_plate(0.36, 0.44, 5), 0.06, "yz"), m["dark"], at=(1.61, GY, GZ), mirror=True)
    s.add(tube(16, 0.15, 0.15, GY + 0.25, GY + 0.72, dx=1.68, dz=GZ - 0.2), m["armor_round"], mirror=True)
    s.add(tube(16, 0.1, 0.1, GY + 0.72, GY + 0.8, dx=1.68, dz=GZ - 0.2), m["dark_round"], mirror=True)
    # Kabelkanal unter der Zierlinie vom Anschlusskasten achtern bis zum Stellmotor
    s.add(sweep([(1.63, 2.9, 1.4), (1.63, -0.12, 1.4), (1.68, -0.42, 1.12)], rect(0.1, 0.1)), m["dark"], mirror=True)
    s.add(box(0.18, 0.3, 0.22), m["armor"], at=(1.64, 2.9, 1.4), mirror=True)

    # ---------------------------------------------------------------- Pendel unter dem Kiel, nach achtern ausgeschwungen
    pivot = Vector((0, 1.75, 0.2))
    swing = math.radians(14)
    d = Vector((0, math.sin(swing), -math.cos(swing)))
    # Pendelbock: Hängebock unter dem Kiel, Lagerdeckel um die Achse
    s.add(plate([(0.3, 0.62), (0.21, 0.08), (-0.21, 0.08), (-0.3, 0.62)], 0.66, "xz"), m["dark"], at=(0, pivot.y, 0))
    s.add(tube(16, 0.17, 0.17, 0.2, 0.27), m["armor_round"], at=tuple(pivot), rot=(0, 0, -90), mirror=True)
    s.add(tube(16, 0.12, 0.12, -0.34, 0.34), m["brass_round"], at=tuple(pivot), rot=(0, 0, -90))
    s.add(segment(pivot, pivot + d * 0.75, 0.09, 12), m["brass_round"])
    bob = pivot + d * 0.92
    s.add(revolve([(0.0, -0.14), (0.36, -0.12), (0.52, -0.05), (0.52, 0.05), (0.36, 0.12), (0.0, 0.14)], 28), m["brass_round"],
          at=tuple(bob), rot=(0, 0, -90))
    s.add(tube(24, 0.26, 0.26, -0.16, 0.16), m["signal"], at=tuple(bob), rot=(0, 0, -90))

    # ---------------------------------------------------------------- Wappen auf dem Verschluss, Maschinenlüfter daneben
    emblem(s, m, (0, 1.45, ZR + 0.38))
    vent(s, m, m["hull"], (1.28, 1.6, DECK), 0.4, 0.9, slats=3, mirror=True)

    # ---------------------------------------------------------------- Brücke achtern: drei Keile, Brückenflügel, Mast
    z, by = DECK, 3.15
    for w, l, h in ((1.7, 1.4, 0.5), (1.25, 1.05, 0.45), (0.9, 0.75, 0.4)):
        s.add(wedge(w, l, h), m["hull"], at=(0, by, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, by - 0.75 / 2 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.465, by + 0.05, z - 0.17), mirror=True)
    s.add(box(0.62, 0.28, 0.16), m["hull"], at=(0.66, by - 0.08, z - 0.335), mirror=True)
    s.add(post(10, 0.09, 0.06, z, z + 0.95), m["hull_round"], at=(0, by + 0.15, 0))
    s.add(box(0.86, 0.07, 0.07), m["hull"], at=(0, by + 0.15, z + 0.66))
    s.add(box(0.56, 0.05, 0.2), m["armor"], at=(0, by + 0.06, z + 0.36), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, by + 0.15, z + 0.99))

    # ---------------------------------------------------------------- RCS an den Rumpfecken: vorn auf der Bugkappe, achtern
    # neben der Brücke, beide auf dem Deck, damit ihre Konturen nicht mit der Zierlinie verschmelzen
    rcs(s, m, (0.88, -2.75, DECK + 0.06), -1)
    rcs(s, m, (1.09, 3.7, DECK + 0.06), 1)

    # ---------------------------------------------------------------- Unterseite: Kiel vorn, Spanten am V-Boden des Bugs
    # (vor dem Spulenring, der steuerbord bei y ≈ −2,05 aus dem Boden tritt); ihre Stirnflächen fangen in `hero` Licht
    s.add(plate([(-3.15, 0.68), (-1.8, 0.42), (-1.8, -0.05), (-2.75, 0.2)], 0.16, "yz"), m["dark"])
    for y in (-2.6, -2.3):
        x0, x1 = 0.06, 0.95
        z0, z1 = bottom_z(x0, y), bottom_z(x1, y)
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.12, "xz"), m["hull"], at=(0, y, 0), mirror=True)

    # ---------------------------------------------------------------- Düsenblock mit Lüfter, vier Düsen
    s.add(box(2.5, 0.5, 1.2), m["dark"], at=(0, 4.25, ZC + 0.25))
    s.add(box(2.2, 0.38, 0.06), m["armor"], at=(0, 4.25, ZC + 0.86))
    vent(s, m, m["armor"], (0, 4.25, ZC + 0.89), 0.9, 0.3, slats=2)
    for r, y1, dx, dz in ((0.44, 5.05, 0.6, ZC + 0.05), (0.3, 4.85, 1.25, ZC + 0.5)):
        engine(s, m, r, 4.3, y1, dx=dx, dz=dz, mirror=True)
        engine_details(s, m, r, 4.3, y1, dx=dx, dz=dz, mirror=True)
    return s.finish()
