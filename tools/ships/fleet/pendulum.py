"""Pendulum (Flaggschiff von Admiral Kerr, „der Uhrmacher“), Fassung für die Nahaufnahme nach docs/stilregeln.md:
gefaste Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

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

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an den beiden unteren Blöcken der
Brücke, Geländer, Leiter, Nadel-, Peitschen- und Schüsselantenne; Messing-Zuleitungen von jedem Kondensator ins
Railgun-Gehäuse und je zwei Stromschienen von den Kondensatorbänken zum Verschluss; neben dem Verschluss Flak und
Lagerkisten, Flak auch auf dem Düsenblock; Kühlrippen an den Düsen; Positionslichter auf den äußersten Zähnen des
Zahnkranzes (oben, backbord, steuerbord).

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen das Uhrwerk: Skalen, Instrumente, Kabel, Messing,
und wo man das Schiff betritt und verschraubt): an der Railgun Flanschringe an den Spulen, Sammelleitungen aus Messing
an den Schienenflanken, Schienenklammern zwischen den Spulen, Schrauben in der Stirnplatte des Jochs und entlang der
Leuchtschiene, Wartungsklappen am Gehäuse; an den Kondensatorbänken je Kondensator ein Rundinstrument (`gauge`) und ein
Isolatorring; am Gimbal die Minutenskala auf beiden Stirnseiten des Zahnkranzes und eine Schraube je Zahn,
Wicklungsbänder aus Messing um den Spulenring, Schrauben in den Lagerplatten, die Schnecke des Stellmotors mit Gängen,
Stehlager und Spannbändern; auf dem Deck Manschetten, wo der Spulenring durchs Deck tritt, Wartungsbühnen aus Gitterrost
mit Geländer neben dem Gimbal, Deckluken; am Verschluss Messingklemmen der Stromschienen, Wartungsklappen, Instrumente
und Steigeisen, Schrauben im Zahnrad des Wappens; an der Flanke Schellen und verschraubter Anschlusskasten am
Kabelkanal, zwei Messingkabel zum Stellmotor, Dockluke mit Trittfläche, Klammern an der Schürze; an der Brücke Tür,
Fenstersimse, Konsolen unter den Brückenflügeln, Mastfuß mit Abspannung, Dachaggregat und Messingkuppel; am Heck
Stellstreben an den Düsenglocken; am Pendel Schrauben in den Lagerdeckeln und der Linse, Klemmring an der Stange. Dazu
Beschriftung als flache Geometrie: die Skala des Zahnkranzes, Wegelinien von der Brückentür zum Verschluss, Warnbänder
an Joch, Deckdurchführungen, Dockluke und Düsenblock, die Kennnummer am Bug an beiden Flanken und an der Rückwand der
Brücke.
"""

import math

from mathutils import Vector

import palette
import vf
from parts import bolts, deckhouse, engine, engine_details, flak, grating, hatch, number, rail, rcs, stripes, trim_line, vent, windows
from vf import (Ship, box, chamfer, dish, flank, hoop, keel, loft, octagon, pipe, plate, post, rect, revolve, sec, segment, sphere, sweep,
                tube)

ZC = 1.0  # Rumpfmitte; Deck bei ZC + 0,75
DECK = ZC + 0.75
ZR = 2.0  # Achse der Railgun
GY, GZ = -1.2, 1.3  # Mitte des Gimbals
COILS = (-3.45, -3.9, -4.35, -4.8, -5.25)  # Spulen um die Schienen
ROUND = 40  # Segmente der großen runden Teile (Spulen, schwere Düsen, Pendellinse)
MID = 24  # mittlere runde Teile (kleine Düsen, Zapfen, Kondensatoren)
RING = 64  # Spulenring des Gimbals (Radius 1,6)
# Düsen (Steuerbord): Radius, Düsenende, Versatz quer und in der Höhe, Segmente
ENGINES = ((0.44, 5.05, 0.6, ZC + 0.05, ROUND), (0.3, 4.85, 1.25, ZC + 0.5, MID))
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


def half(y):
    """Halbe Breite des Rumpfs bei y (Flanke und Deckkante)."""
    for (y0, w0, _, _), (y1, w1, _, _) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            return (w0 + (y - y0) / (y1 - y0) * (w1 - w0)) / 2
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


def segment_plate(r, r_tip, teeth, n=20):
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
    s.add(chamfer(plate(gear(12, 0.6, 0.8), 0.14, "xy"), 0.02), m["brass"], at=(x, y, z + 0.02))
    s.add(post(ROUND, 0.46, 0.46, 0.0, 0.05), m["signal"], at=(x, y, z + 0.07))
    for length, width, angle in ((0.4, 0.13, 25), (0.28, 0.17, 145)):
        hand = [(-width / 2, -0.06), (width / 2, -0.06), (width / 4, length), (-width / 4, length)]
        a = math.radians(angle)
        hand = [(px * math.cos(a) - py * math.sin(a), px * math.sin(a) + py * math.cos(a)) for px, py in hand]
        s.add(plate(hand, 0.05, "xy"), m["trim"], at=(x, y, z + 0.13))
    s.add(post(MID, 0.08, 0.08, 0.0, 0.09), m["trim"], at=(x, y, z + 0.11))


def capacitors(s, m, x, y):
    """Kondensatorbank als Satteltasche an der Flanke des Railgun-Gehäuses (x > 0 steuerbord, x < 0 backbord): Kasten in
    `armor`, darauf drei Kondensatoren."""
    side = 1 if x > 0 else -1
    s.add(chamfer(box(0.36, 0.9, 0.36), 0.03), m["armor"], at=(x, y, DECK + 0.15))
    for dy in (-0.3, 0.0, 0.3):
        s.add(post(MID, 0.11, 0.11, DECK + 0.31, DECK + 0.44), m["dark_round"], at=(x + side * 0.055, y + dy, 0))


def gauge(s, m, at, rot, hand=0.0, mirror=False):
    """Rundinstrument an einer Wand (lokal +z nach außen, `rot` dreht es hin): Messingfassung, helles Zifferblatt,
    dunkler Zeiger im Winkel `hand`. Eckig schattiert: Einen runden Deckel an rundem Material hielte die Verwitterung
    für eine Mündung und schwärzte ihn mit Ruß."""
    g = Ship("gauge")
    g.add(post(16, 0.05, 0.05, -0.006, 0.018), m["brass"])
    g.add(post(16, 0.038, 0.038, 0.0, 0.023), m["paint"])
    a = math.radians(hand)
    g.add(box(0.008, 0.034, 0.006), m["dark"], at=(-0.012 * math.sin(a), 0.012 * math.cos(a), 0.024), rot=(0, 0, hand))
    s.place(g, at=at, rot=rot, mirror=mirror)
    g.bm.free()


def details(s, m, z, by):
    """Kleinteile (Modulkommentar); `z` ist das Dach der Brücke, `by` ihre Mitte."""
    roof1, roof2 = DECK + 0.5, DECK + 0.95  # Dächer der beiden unteren Blöcke der Brücke

    # ---------------------------------------------------------------- Fensterreihen an der Brücke
    row = [by - 0.15 + 0.13 * k for k in range(7)]
    windows(s, m, [(0.85, y, roof1 - 0.17) for y in row])
    windows(s, m, [(0.85, y, roof1 - 0.35) for y in row[1::2]])
    windows(s, m, [(-0.6 + 0.2 * k, by + 0.7, roof1 - 0.17) for k in range(7)], out="y")
    windows(s, m, [(0.625, by - 0.08 + 0.13 * k, roof2 - 0.2) for k in range(5)])
    windows(s, m, [(-0.4 + 0.2 * k, by + 0.525, roof2 - 0.2) for k in range(5)], out="y")

    # ---------------------------------------------------------------- Antennen
    s.add(post(6, 0.014, 0.006, z + 0.66, z + 1.2), m["dark_round"], at=(0.4, by + 0.15, 0), mirror=True)
    s.add(segment((0.55, by + 0.42, roof2 - 0.02), (0.62, by + 0.92, roof2 + 0.75), 0.012, 6), m["dark_round"], mirror=True)
    # Schüssel backbord, Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, roof1 - 0.02, roof1 + 0.14), m["dark_round"], at=(-0.705, by + 0.5, 0))
    s.add(dish(0.13, 0.05, n=16, rings=3, thickness=0.02), m["armor_round"], at=(-0.705, by + 0.5, roof1 + 0.13), rot=(-40, 0, 150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(0.705, by + 0.5, roof1 + 0.06))
    s.add(post(6, 0.012, 0.006, roof1 + 0.1, roof1 + 0.5), m["dark_round"], at=(0.705, by + 0.56, 0))

    # ---------------------------------------------------------------- Geländer und Leiter an der Brücke
    # Die Geländer stehen innen neben den Fasen der Dachkanten
    rail(s, m, (0.8, by + 0.22, roof1), (0.8, by + 0.65, roof1), 3)
    rail(s, m, (-0.8, by + 0.65, roof1), (0.8, by + 0.65, roof1), 5, mirror=False)
    rail(s, m, (0.58, by + 0.1, roof2), (0.58, by + 0.48, roof2), 3)
    wing = z - 0.255
    rail(s, m, (0.47, by - 0.21, wing), (0.96, by - 0.21, wing), 3)
    rail(s, m, (0.96, by - 0.21, wing), (0.96, by + 0.05, wing), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.5), m["dark"], at=(0.72 + dx, by + 0.707, DECK + 0.25))
    for k in range(6):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.72, by + 0.707, DECK + 0.07 + 0.08 * k))

    # ---------------------------------------------------------------- Railgun: Zuleitungen der Kondensatoren, Stromschienen
    # Bänke wie in `build`: backbord vor dem Gimbal, steuerbord dahinter
    for x, y in ((-0.68, -2.0), (0.68, -0.4)):
        side = 1 if x > 0 else -1
        for dy in (-0.3, 0.0, 0.3):
            cx = x + side * 0.055
            s.add(pipe([(cx, y + dy, DECK + 0.42), (cx, y + dy, ZR + 0.3), (side * 0.4, y + dy, ZR + 0.3)], 0.022, 6), m["brass_round"])
        for dx in (-0.05, 0.05):
            s.add(segment((x + dx, y + 0.43, DECK + 0.028), (x + dx, 0.56, DECK + 0.028), 0.028, 6), m["steel"])

    # ---------------------------------------------------------------- Deck neben dem Verschluss: Flak, Lagerkisten
    for at, yaw in (((1.42, 0.5, DECK), 60), ((1.42, 2.4, DECK), 130), ((0.85, 4.25, ZC + 0.89), 165)):
        flak(s, m, at, yaw)
    s.add(box(0.22, 0.2, 0.14), m["armor"], at=(1.09, 0.95, DECK + 0.07))
    s.add(box(0.2, 0.22, 0.16), m["dark"], at=(-1.08, 0.93, DECK + 0.08))
    s.add(box(0.14, 0.14, 0.1), m["armor"], at=(-1.08, 0.92, DECK + 0.21))

    # ---------------------------------------------------------------- Kühlrippen an den Düsen (wie in `build`)
    for (r, y1, dx, dz, n), ts in zip(ENGINES, ((0.35, 0.4, 0.45), (0.37, 0.44))):
        for t in ts:
            y = 4.3 + (y1 - 4.3) * t
            s.add(tube(n, r * 1.1, r * 1.1, y, y + 0.02, dx, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter auf dem Zahnkranz
    s.add(sphere(0.035, 1), m["glow"], at=(0, GY, GZ + 2.27))
    s.add(sphere(0.035, 1), m["glow"], at=(2.27, GY, GZ), mirror=True)


def fine(s, m, z, by, pivot, bob):
    """Feinteile und Beschriftung (Modulkommentar); `z` und `by` wie in `details`, `pivot` ist die Pendelachse, `bob` die
    Mitte der Pendellinse."""
    roof1, roof2 = DECK + 0.5, DECK + 0.95

    # ---------------------------------------------------------------- Railgun: Spulen, Sammelleitungen, Klammern, Joch
    # Jede Spule sitzt zwischen zwei schmalen Flanschringen (nur am Rand: Ihre Stirnseiten bleiben in ihrer Farbe); die
    # Sammelleitung läuft je Seite an der Schienenflanke durch die Spulen und zeigt sich nur in den Lücken, damit die
    # Spulen auf dem Brett so klar lesen wie vorher. Dort fassen Klammern die beiden Schienen oben und unten; am
    # Mündungsjoch Schrauben in der Stirnplatte und ein Warnband an jeder Flanke
    for y in COILS:
        for dy in (-0.11, 0.11):
            s.add(hoop(ROUND, 0.585, 0.622, 0.02), m["dark_round"], at=(0, y + dy, ZR), rot=(90, 0, 0))
    s.add(segment((0.35, -5.5, ZR), (0.35, -3.15, ZR), 0.02, 6), m["brass_round"], mirror=True)
    for y in ((a + b) / 2 for a, b in zip(COILS, COILS[1:])):
        for sz in (-1, 1):
            s.add(box(0.74, 0.05, 0.05), m["dark"], at=(0, y, ZR + sz * 0.262))
        bolts(s, m["steel"], [(sx * 0.25, y, ZR + 0.287) for sx in (-1, 1)])
    bolts(s, m["steel"], [(sx * 0.25, -5.84, ZR + sz * 0.25) for sx in (-1, 1) for sz in (-1, 1)] + [(sx * 0.27, -5.84, ZR) for sx in (-1, 1)],
          "-y", r=0.016)
    stripes(s, m, (0.51, -5.6, ZR), 0.06, 0.4, rot=(90, 0, 90), mirror=True)

    # ---------------------------------------------------------------- Railgun-Gehäuse: Schrauben, Wartungsklappen
    # Schraubenpaare entlang der Leuchtschiene; je Flanke eine Klappe, wo Kondensatorbank und Spulenring Platz lassen
    bolts(s, m["steel"], [(sx * 0.12, -2.35 + 0.5 * k, ZR + 0.36) for k in range(6) for sx in (-1, 1)])
    hatch(s, m, (-0.525, -0.55, ZR), 0.26, 0.36, rot=(0, -90, 0))
    hatch(s, m, (0.525, -1.75, ZR), 0.26, 0.36, rot=(0, 90, 0))

    # ---------------------------------------------------------------- Kondensatorbänke: Instrumente, Isolatoren
    # Je Kondensator ein Rundinstrument an der Außenwand der Bank und ein heller Isolatorring, Schrauben in den Ecken
    for x, y in ((-0.68, -2.0), (0.68, -0.4)):
        side = 1 if x > 0 else -1
        for dy, hand in ((-0.3, -50), (0.0, 20), (0.3, 65)):
            gauge(s, m, (x + side * 0.18, y + dy, DECK + 0.2), (0, side * 90, 0), hand)
            s.add(post(MID, 0.122, 0.122, DECK + 0.36, DECK + 0.39), m["armor_round"], at=(x + side * 0.055, y + dy, 0))
        bolts(s, m["steel"], [(x + side * 0.18, y + sy * 0.4, DECK + 0.15 + sz * 0.13) for sy in (-1, 1) for sz in (-1, 1)], (0, side * 90, 0))

    # ---------------------------------------------------------------- Gimbal: Skala, Wicklungen, Lager, Schneckentrieb
    # Der Zahnkranz trägt auf beiden Stirnseiten eine Minutenskala wie ein Zifferblatt (an den Zapfen ausgespart) und
    # in jedem Zahn eine Schraube; um den Spulenring sechzehn Wicklungsbänder aus Messing
    g = Ship("scale")
    for k in range(60):
        if k % 30 == 0:
            continue
        a, length = math.radians(6 * k), 0.1 if k % 5 == 0 else 0.05
        for sz in (-1, 1):
            g.add(box(length, 0.012, 0.008), m["paint"], at=((1.85 + length / 2) * math.cos(a), (1.85 + length / 2) * math.sin(a), sz * 0.152),
                  rot=(0, 0, 6 * k))
    for k in range(16):
        a = math.radians(22.5 * k)
        for out, sz in (("z", 1), ("-z", -1)):
            bolts(g, m["steel"], [(2.13 * math.cos(a), 2.13 * math.sin(a), sz * 0.15)], out, r=0.016)
    s.place(g, at=(0, GY, GZ), rot=(90, 0, 0))
    g.bm.free()
    g = Ship("windings")
    for k in range(16):
        a = 22.5 * k + 11.25
        g.add(box(0.2, 0.035, 0.215), m["brass"], at=(1.51 * math.cos(math.radians(a)), 1.51 * math.sin(math.radians(a)), 0), rot=(0, 0, a))
    s.place(g, at=(0, GY, GZ), rot=(90, 0, -35))
    g.bm.free()
    # Schrauben in der Lagerplatte über dem Zahnsegment; die Schnecke des Stellmotors mit Gängen und Stehlager am
    # Rumpf, zwei Spannbänder um den Motor
    bolts(s, m["steel"], [(1.64, GY + 0.27 * math.cos(math.radians(a)), GZ + 0.27 * math.sin(math.radians(a))) for a in (20, 65, 115, 160)],
          "x", mirror=True)
    wx, wz = 1.68, GZ - 0.2
    s.add(tube(12, 0.055, 0.055, GY - 0.32, GY + 0.25, dx=wx, dz=wz), m["brass_round"], mirror=True)
    for k in range(5):
        s.add(tube(12, 0.074, 0.074, GY - 0.2 + 0.08 * k, GY - 0.17 + 0.08 * k, dx=wx, dz=wz), m["brass_round"], mirror=True)
    s.add(chamfer(box(0.3, 0.1, 0.16), 0.015), m["dark"], at=(wx - 0.07, GY - 0.33, wz), mirror=True)
    for y in (GY + 0.33, GY + 0.6):
        s.add(tube(MID, 0.158, 0.158, y, y + 0.03, dx=wx, dz=wz), m["dark_round"], mirror=True)

    # ---------------------------------------------------------------- Deck: Durchführungen, Luken, Wartungsbühne
    # Wo der Spulenring durchs Deck tritt (steuerbord vorn, backbord achtern), eine Manschette und innen ein Warnband;
    # neben dem Gimbal je Seite eine Wartungsbühne aus Gitterrost mit Geländer an der Deckkante, davor die Deckluken
    for sign in (1, -1):
        x, y = sign * 1.18, GY - sign * 0.826
        s.add(chamfer(box(0.25, 0.26, 0.07), 0.015), m["dark"], at=(x, y, DECK + 0.005), rot=(0, 0, -35))
        stripes(s, m, (x - sign * 0.164, y + sign * 0.115, DECK), 0.05, 0.3, rot=(0, 0, -35))
    grating(s, m, 1.26, 1.38, -1.55, -0.65, DECK)
    rail(s, m, (1.4, -1.5, DECK), (1.52, -0.7, DECK), 4)
    hatch(s, m, (1.12, 0.05, DECK), 0.3, 0.36, mirror=True)

    # ---------------------------------------------------------------- Verschluss: Klemmen, Klappe, Instrumente, Steigeisen
    # Messingklemmen, wo die Stromschienen in den Verschluss laufen; an jeder Flanke eine Wartungsklappe, zwei
    # Instrumente und Steigeisen zum Wappen hinauf, im Zahnrad des Wappens eine Schraube je Zahn
    for x in (-0.68, 0.68):
        s.add(chamfer(box(0.24, 0.1, 0.12), 0.015), m["brass"], at=(x, 0.42, DECK + 0.05))
        bolts(s, m["steel"], [(x + dx, 0.42, DECK + 0.11) for dx in (-0.05, 0.05)])
    hatch(s, m, (0.975, 1.65, ZR), 0.3, 0.4, rot=(0, 90, 0), mirror=True)
    for y, hand in ((1.15, 40), (1.29, -60)):
        gauge(s, m, (0.975, y, ZR + 0.1), (0, 90, 0), hand, mirror=True)
    for k in range(6):
        s.add(box(0.03, 0.07, 0.012), m["steel_flat"], at=(0.983, 2.12, DECK + 0.07 + 0.07 * k), mirror=True)
    bolts(s, m["steel"], [(0.7 * math.cos(math.radians(a)), 1.45 + 0.7 * math.sin(math.radians(a)), ZR + 0.47) for a in range(0, 360, 30)])

    # ---------------------------------------------------------------- Flanke: Schellen, Kabel, Dockluke, Klammern
    # Kabelkanal mit vier Schellen und verschraubtem Anschlusskasten, darunter zwei Messingkabel zum Stellmotor; achtern
    # die Dockluke mit Trittfläche (Warnband) auf der Schürze, außen an der Schürze vier Klammern
    for y in (0.3, 1.0, 1.7, 2.4):
        s.add(box(0.13, 0.05, 0.13), m["armor"], at=(1.635, y, 1.4), mirror=True)
        bolts(s, m["steel"], [(1.7, y, 1.44), (1.7, y, 1.36)], "x", mirror=True)
    bolts(s, m["steel"], [(1.73, 2.9 + sy * 0.11, 1.4 + sz * 0.07) for sy in (-1, 1) for sz in (-1, 1)], "x", mirror=True)
    for h in (1.3, 1.25):
        s.add(pipe([(1.612, 2.8, h), (1.612, -0.34, h), (1.66, -0.44, h - 0.1)], 0.014, 6, joints=False), m["brass_round"], mirror=True)
    hatch(s, m, (half(3.3), 3.3, 1.38), 0.22, 0.26, rot=(0, 90, 14), mirror=True)
    stripes(s, m, (half(3.3) + 0.085, 3.3, ZC + 0.14), 0.07, 0.3, rot=(0, 0, 17), mirror=True)
    for y in (-0.3, 0.75, 1.8, 2.7):
        s.add(box(0.03, 0.07, 0.16), m["dark"], at=(1.755, y, ZC + 0.05), mirror=True)
        bolts(s, m["steel"], [(1.77, y, ZC + 0.05)], "x", mirror=True)

    # ---------------------------------------------------------------- Brücke: Tür, Simse, Konsolen, Mastfuß, Dach
    s.add(box(0.024, 0.16, 0.25), m["dark"], at=(0.856, by + 0.11, DECK + 0.12), mirror=True)
    s.add(box(0.02, 0.12, 0.21), m["armor"], at=(0.866, by + 0.11, DECK + 0.12), mirror=True)
    s.add(box(0.012, 0.012, 0.05), m["steel_flat"], at=(0.878, by + 0.07, DECK + 0.115), mirror=True)
    for h in (0.055, 0.185):
        s.add(box(0.014, 0.02, 0.03), m["steel_flat"], at=(0.876, by + 0.162, DECK + h), mirror=True)
    for x, y0, y1, h in ((0.85, by - 0.15, by + 0.63, roof1 - 0.17), (0.625, by - 0.08, by + 0.44, roof2 - 0.2)):
        s.add(box(0.03, y1 - y0 + 0.12, 0.012), m["dark"], at=(x + 0.01, (y0 + y1) / 2, h - 0.031), mirror=True)
    s.add(box(1.32, 0.03, 0.012), m["dark"], at=(0, by + 0.71, roof1 - 0.201))
    s.add(box(0.92, 0.03, 0.012), m["dark"], at=(0, by + 0.535, roof2 - 0.231))
    for y in (by - 0.11, by + 0.02):
        s.add(plate([(0.62, z - 0.41), (0.95, z - 0.41), (0.62, z - 0.68)], 0.03, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    s.add(post(16, 0.14, 0.14, z - 0.004, z + 0.02), m["hull_round"], at=(0, by + 0.15, 0))
    bolts(s, m["steel"], [(0.115 * math.cos(math.radians(a)), by + 0.15 + 0.115 * math.sin(math.radians(a)), z + 0.02) for a in range(0, 360, 60)])
    s.add(segment((0.04, by + 0.16, z + 0.6), (0.24, by + 0.34, z - 0.005), 0.01, 6), m["dark_round"], mirror=True)
    s.add(chamfer(box(0.2, 0.14, 0.08), 0.012), m["armor"], at=(-0.22, by - 0.02, z + 0.036))
    s.add(box(0.12, 0.08, 0.012), m["dark"], at=(-0.22, by - 0.02, z + 0.078))
    s.add(sphere(0.06, 2), m["brass_round"], at=(0.22, by - 0.02, z + 0.02))

    # ---------------------------------------------------------------- Heck: Stellstreben, Schrauben
    for r, y1, dx, dz, _ in ENGINES:
        count = 8 if r > 0.4 else 6
        ya = 4.3 + 0.6 * (y1 - 4.3)
        for k in range(count):
            c, d = math.cos(2 * math.pi * (k + 0.5) / count), math.sin(2 * math.pi * (k + 0.5) / count)
            s.add(segment((dx + 1.1 * r * c, ya, dz + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, dz + 0.93 * r * d), 0.016, 6), m["steel"],
                  mirror=True)
    bolts(s, m["steel"], [(sx * 1.05, 4.25 + sy * 0.15, ZC + 0.89) for sx in (-1, 1) for sy in (-1, 1)])

    # ---------------------------------------------------------------- Pendel: Lagerschrauben, Klemmring, Linse
    bolts(s, m["steel_flat"], [(0.27, pivot.y + 0.145 * math.cos(math.radians(a)), pivot.z + 0.145 * math.sin(math.radians(a)))
                               for a in range(0, 360, 60)], "x", r=0.01, mirror=True)
    swing = (bob - pivot).normalized()
    s.add(segment(pivot + swing * 0.27, pivot + swing * 0.33, 0.12, 16), m["dark_round"])
    bolts(s, m["steel"], [(0.122, bob.y + 0.32 * math.cos(math.radians(a)), bob.z + 0.32 * math.sin(math.radians(a))) for a in range(0, 360, 45)],
          "x", r=0.014, mirror=True)

    # ---------------------------------------------------------------- Beschriftung: Wegelinie, Warnband, Nummer
    # Linie von der Brückentür zu den Steigeisen am Verschluss, Warnbänder neben dem Lüfter des Düsenblocks; die
    # Kennnummer am Bug an beiden Flanken (die Flanke läuft dort um 10° zu) und an der Rückwand der Brücke, lesbar von
    # hinten oben
    s.add(box(0.02, 1.25, 0.006), m["paint"], at=(1.02, 2.675, DECK + 0.001), mirror=True)
    stripes(s, m, (0.6, 4.25, ZC + 0.89), 0.05, 0.3, mirror=True)
    number(s, m["paint"], "03", (half(-2.05), -2.05, 1.4), 0.2, (90, 0, 80))
    number(s, m["paint"], "03", (-half(-2.05), -2.05, 1.4), 0.2, (90, 0, -80))
    number(s, m["paint"], "03", (0, by + 0.375, z - 0.19), 0.2, (90, 0, 180))


def build():
    s = Ship("pendulum")
    m = vf.materials(s.name, "battleship")
    m["brass"] = vf.toon(f"{s.name}_brass", palette.BRASS)
    m["brass_round"] = vf.toon(f"{s.name}_brass_round", palette.BRASS, round=True, glint=vf.GLINT)

    # ---------------------------------------------------------------- Rumpf, Panzerlagen, Zierlinie
    hull = [sec(y, keel(w, h), dz=ZC + dz) for y, w, h, dz in HULL]
    # Die Zierlinie liegt an der Flanke unter der Deckkante, die deshalb gefast sein darf
    s.add(chamfer(loft(hull), 0.05), m["hull"])
    trim_line(s, m, flank(hull, DECK - 0.12, 0.02))
    # Bugpanzer: Kappe um die Bugspitze
    s.add(chamfer(loft([sec(-3.36, keel(1.42, 1.0), dz=ZC + 0.15), sec(-2.75, keel(2.3, 1.29), dz=ZC + 0.108)]), 0.03), m["armor"])
    # Panzerschürzen vor und hinter dem Gimbal
    s.add(chamfer(plate([(1.15, -2.95), (1.62, -2.2), (1.7, -1.75), (1.3, -1.75)], 0.18, "xy"), 0.04), m["armor"], at=(0, 0, ZC + 0.05),
          mirror=True)
    s.add(chamfer(plate([(1.3, -0.65), (1.75, -0.65), (1.75, 2.9), (1.45, 3.75), (1.2, 3.75)], 0.18, "xy"), 0.04), m["armor"],
          at=(0, 0, ZC + 0.05), mirror=True)

    # ---------------------------------------------------------------- Railgun: Gehäuse, Verschluss, Schienen, Spulen, Joch
    s.add(chamfer(loft([sec(-3.15, octagon(0.9, 0.6, 0.35), dz=ZR - 0.1), sec(-2.5, octagon(1.05, 0.72, 0.35), dz=ZR),
                        sec(0.6, octagon(1.05, 0.72, 0.35), dz=ZR)]), 0.03), m["accent"])
    s.add(box(0.09, 3.0, 0.05), m["glow"], at=(0, -1.1, ZR + 0.36))
    s.add(chamfer(loft([sec(0.45, octagon(1.6, 0.66, 0.4), dz=ZR - 0.02), sec(0.75, octagon(1.95, 0.76, 0.35), dz=ZR),
                        sec(2.3, octagon(1.95, 0.76, 0.35), dz=ZR)]), 0.04), m["dark"])
    for x in (0.25, -0.25):
        s.add(chamfer(box(0.17, 3.9, 0.5), 0.02), m["steel"], at=(x, -3.65, ZR))
    for k, y in enumerate(COILS):
        s.add(hoop(ROUND, 0.34, 0.6, 0.22), m["signal" if k % 2 else "brass_round"], at=(0, y, ZR), rot=(90, 0, 0))
    s.add(chamfer(loft([sec(-5.8, octagon(0.86, 0.86, 0.4)), sec(-5.72, octagon(1.02, 1.02, 0.4)), sec(-5.47, octagon(1.02, 1.02, 0.4))]),
                  0.03), m["dark"], at=(0, 0, ZR))
    # Stirnplatte des Jochs um die Mündung
    s.add(loft([sec(-5.84, octagon(0.7, 0.7, 0.4)), sec(-5.79, octagon(0.7, 0.7, 0.4))]), m["armor"], at=(0, 0, ZR))
    s.add(box(0.3, 0.06, 0.48), m["signal"], at=(0, -5.85, ZR))
    # Kühlmantel um die Gehäusespitze, wo die Schienen austreten, mit hellem Spannband
    s.add(chamfer(loft([sec(-3.2, octagon(1.0, 0.7, 0.35), dz=ZR - 0.1), sec(-2.62, octagon(1.14, 0.82, 0.35), dz=ZR - 0.02)]), 0.03),
          m["dark"])
    s.add(loft([sec(-2.98, octagon(1.15, 0.84, 0.35), dz=ZR - 0.07), sec(-2.86, octagon(1.17, 0.86, 0.35), dz=ZR - 0.053)]), m["armor"])
    # Kondensatorbänke, versetzt wie der Spulenring: backbord vor dem Gimbal, steuerbord dahinter
    capacitors(s, m, -0.68, -2.0)
    capacitors(s, m, 0.68, -0.4)

    # ---------------------------------------------------------------- Gimbal: Zahnkranz auf Zapfen, gedrehter Spulenring
    s.add(chamfer(gear_ring(16, 1.8, 2.02, 2.26, 0.3), 0.03), m["brass"], at=(0, GY, GZ), rot=(90, 0, 0))
    s.add(tube(MID, 0.19, 0.19, 1.4, 1.92), m["brass_round"], at=(0, GY, GZ), rot=(0, 0, -90), mirror=True)
    s.add(hoop(RING, 1.42, 1.6, 0.2), m["accent"], at=(0, GY, GZ), rot=(90, 0, -35))
    s.add(hoop(RING, 1.28, 1.43, 0.14), m["signal"], at=(0, GY, GZ), rot=(90, 0, -35))
    for z0, z1 in ((GZ + 1.5, GZ + 1.88), (GZ - 1.88, GZ - 1.5)):
        s.add(post(16, 0.11, 0.11, z0, z1), m["brass_round"], at=(0, GY, 0))
    # Lagerplatte an jedem Zapfen, unten als Zahnsegment, dahinter der Stellmotor, der es über eine Schnecke dreht
    s.add(plate(segment_plate(0.36, 0.44, 5), 0.06, "yz"), m["dark"], at=(1.61, GY, GZ), mirror=True)
    s.add(tube(MID, 0.15, 0.15, GY + 0.25, GY + 0.72, dx=1.68, dz=GZ - 0.2), m["armor_round"], mirror=True)
    s.add(tube(MID, 0.1, 0.1, GY + 0.72, GY + 0.8, dx=1.68, dz=GZ - 0.2), m["dark_round"], mirror=True)
    # Kabelkanal unter der Zierlinie vom Anschlusskasten achtern bis zum Stellmotor
    s.add(sweep([(1.63, 2.9, 1.4), (1.63, -0.12, 1.4), (1.68, -0.42, 1.12)], rect(0.1, 0.1)), m["dark"], mirror=True)
    s.add(chamfer(box(0.18, 0.3, 0.22), 0.02), m["armor"], at=(1.64, 2.9, 1.4), mirror=True)

    # ---------------------------------------------------------------- Pendel unter dem Kiel, nach achtern ausgeschwungen
    pivot = Vector((0, 1.75, 0.2))
    swing = math.radians(14)
    d = Vector((0, math.sin(swing), -math.cos(swing)))
    # Pendelbock: Hängebock unter dem Kiel, Lagerdeckel um die Achse
    s.add(chamfer(plate([(0.3, 0.62), (0.21, 0.08), (-0.21, 0.08), (-0.3, 0.62)], 0.66, "xz"), 0.03), m["dark"], at=(0, pivot.y, 0))
    s.add(tube(MID, 0.17, 0.17, 0.2, 0.27), m["armor_round"], at=tuple(pivot), rot=(0, 0, -90), mirror=True)
    s.add(tube(MID, 0.12, 0.12, -0.34, 0.34), m["brass_round"], at=tuple(pivot), rot=(0, 0, -90))
    s.add(segment(pivot, pivot + d * 0.75, 0.09, 16), m["brass_round"])
    bob = pivot + d * 0.92
    s.add(revolve([(0.0, -0.14), (0.36, -0.12), (0.52, -0.05), (0.52, 0.05), (0.36, 0.12), (0.0, 0.14)], ROUND), m["brass_round"],
          at=tuple(bob), rot=(0, 0, -90))
    s.add(tube(32, 0.26, 0.26, -0.16, 0.16), m["signal"], at=tuple(bob), rot=(0, 0, -90))

    # ---------------------------------------------------------------- Wappen auf dem Verschluss, Maschinenlüfter daneben
    emblem(s, m, (0, 1.45, ZR + 0.38))
    vent(s, m, m["hull"], (1.28, 1.6, DECK), 0.4, 0.9, slats=3, mirror=True)

    # ---------------------------------------------------------------- Brücke achtern: drei Keile, Brückenflügel, Mast
    z, by = DECK, 3.15
    for w, l, h, width in ((1.7, 1.4, 0.5, 0.04), (1.25, 1.05, 0.45, 0.035), (0.9, 0.75, 0.4, 0.03)):
        s.add(deckhouse(w, l, h, width), m["hull"], at=(0, by, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, by - 0.75 / 2 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.465, by + 0.05, z - 0.17), mirror=True)
    s.add(box(0.62, 0.28, 0.16), m["hull"], at=(0.66, by - 0.08, z - 0.335), mirror=True)
    s.add(post(16, 0.09, 0.06, z, z + 0.95), m["hull_round"], at=(0, by + 0.15, 0))
    s.add(box(0.86, 0.07, 0.07), m["hull"], at=(0, by + 0.15, z + 0.66))
    s.add(box(0.56, 0.05, 0.2), m["armor"], at=(0, by + 0.06, z + 0.36), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, by + 0.15, z + 0.99))

    # ---------------------------------------------------------------- RCS an den Rumpfecken: vorn auf der Bugkappe, achtern
    # neben der Brücke, beide auf dem Deck, damit ihre Konturen nicht mit der Zierlinie verschmelzen
    rcs(s, m, (0.88, -2.75, DECK + 0.06), -1)
    rcs(s, m, (1.09, 3.7, DECK + 0.06), 1)

    # ---------------------------------------------------------------- Unterseite: Kiel vorn, Spanten am V-Boden des Bugs
    # (vor dem Spulenring, der steuerbord bei y ≈ −2,05 aus dem Boden tritt); ihre Stirnflächen fangen in `hero` Licht
    s.add(chamfer(plate([(-3.15, 0.68), (-1.8, 0.42), (-1.8, -0.05), (-2.75, 0.2)], 0.16, "yz"), 0.03), m["dark"])
    for y in (-2.6, -2.3):
        x0, x1 = 0.06, 0.95
        z0, z1 = bottom_z(x0, y), bottom_z(x1, y)
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.12, "xz"), m["hull"], at=(0, y, 0), mirror=True)

    # ---------------------------------------------------------------- Düsenblock mit Lüfter, vier Düsen
    s.add(chamfer(box(2.5, 0.5, 1.2), 0.04), m["dark"], at=(0, 4.25, ZC + 0.25))
    s.add(chamfer(box(2.2, 0.38, 0.06), 0.015), m["armor"], at=(0, 4.25, ZC + 0.86))
    vent(s, m, m["armor"], (0, 4.25, ZC + 0.89), 0.9, 0.3, slats=2)
    for r, y1, dx, dz, n in ENGINES:
        engine(s, m, r, 4.3, y1, dx=dx, dz=dz, n=n, mirror=True)
        engine_details(s, m, r, 4.3, y1, dx=dx, dz=dz, n=n, mirror=True)
    details(s, m, z, by)
    fine(s, m, z, by, pivot, bob)
    return s.finish()
