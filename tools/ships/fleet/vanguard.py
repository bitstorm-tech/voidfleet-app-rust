"""Vanguard (Flaggschiff von Admiralin Vance, „der Hammer“), Fassung für die Nahaufnahme nach docs/stilregeln.md:
gefaste Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

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

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an den beiden unteren Blöcken der
Brücke, Flak an der Deckkante neben dem Rückgrat, Nadel-, Peitschen- und Schüsselantenne, Geländer und Leiter an der
Brücke, auf dem Düsengehäuse Treibstoffleitungen zu den oberen Düsen, Lagerkisten und Heckgeländer, Kühlrippen an den
Bugkanonen und den vier kleinen Düsen, Positionslichter an den Klingenspitzen und unter dem Kiel.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen, wie das Schiff rammt, seine Waffen füttert und wo
man es betritt und verschraubt): am Rammbug der Mittelgrat der Speerspitze, Schrauben in der Karmin-Lage und am
Rammkragen, Halter und Eckschuh um die leuchtenden Schneiden, die Energieleitung vom Rückgrat über den Kragen zur
Schneide, Stopfbuchse, Lagerbock und Hydraulikleitung an den Stoßdämpfern; an den Bugkanonen Rücklaufzylinder mit
Kolbenstange, Verschlussklappe und Warnband; am Munitionskanal Schellen und verschraubte Anschlusskästen, auf den
Klingen die aufgedoppelte Schneidenleiste und Holme mit Lagerbock an der Flanke, Dockluke mit Trittfläche; auf dem
Hauptdeck Laufstege aus Gitterrost, die Munitionsluke über dem Magazin und der Aufgang zur Brückentür; auf dem Rückgrat
Schrauben im Wappen und an den Turmringen, das Warnband vor dem Schwenkbereich der Rohre, Munitionsluken, Steigeisen
an der Barbette, Sims und Sprossen am Fensterband; an der Brücke Fenstersimse, Konsolen unter den Brückenflügeln,
Mastfuß mit Abspannung, Dachaggregate; am Heck Wartungsluke und Schrauben am Düsengehäuse, Rohrschellen, Stellstreben
an den Düsenglocken (an den vier kleinen außen, an der Hauptdüse oben und unten, wo die Nachbarn Platz lassen). Dazu
Beschriftung als flache Geometrie: Wegelinien an den Kanten des Rückgrats, Warnbänder, die Kennnummer an beiden Flanken
und an der Rückwand der Brücke.
"""

import math

from mathutils.bvhtree import BVHTree

import vf
from parts import (badge, beam, bellows, bolts, deckhouse, engine, engine_details, flak, hatch, number, on_surface, rail, rcs, spaced,
                   stairs, stripes, sym, trim_line, turret, turret_details, vent, windows)
from vf import (Ship, box, chamfer, dish, grow, hexagon, keel, loft, octagon, pipe, plate, post, rbox, rect, sec, segment, sphere, sweep, tip,
                tube)

# Abzeichen von Admiralin Vance wie auf ihrer Uniform: Pfeilspitze mit Schaft und Fiederung, zeigt zum Bug
PLATE = sym([(0.0, -0.85), (0.58, -0.3), (0.58, 0.52), (0.0, 0.82)])
ARROW = sym([(0.0, -0.66), (0.4, -0.18), (0.13, -0.18), (0.13, 0.19), (0.36, 0.38), (0.36, 0.56), (0.0, 0.26)])
# Rumpf (y, Breite, Höhe) als Keilprofil; Rammbug (y, Breite, Höhe, dz) als Sechseck hinter der Spitze RAM_TIP
HULL = ((-2.4, 1.8, 1.0), (-1.2, 2.4, 1.3), (2.9, 2.5, 1.35), (3.9, 2.0, 1.1))
RAM = ((-4.7, 0.95, 0.42, -0.05), (-3.5, 2.05, 0.85, 0.0), (-2.6, 1.55, 0.95, 0.05), (-1.6, 1.6, 1.0, 0.08))
RAM_TIP = -5.65
# Karmin-Lage auf dem Rammbug (y, Breite, Höhe, dz) hinter ihrer Spitze LANCE_TIP; leuchtende Schneide (Steuerbord)
LANCE = ((-4.5, 0.7, 0.3, 0.12), (-3.5, 1.55, 0.55, 0.25), (-2.5, 1.15, 0.6, 0.32), (-1.7, 1.15, 0.6, 0.34))
LANCE_TIP = -5.2
EDGE = [(0.0, RAM_TIP - 0.02, -0.1)] + [(w / 2 + 0.01, y, dz) for y, w, h, dz in RAM[:3]]
# Panzerklinge (Steuerbord): Vorderkante von der Spitze bis zur Heckspitze, dann die Hinterkante zurück zum Rumpf
BLADE = [(1.0, -1.7), (1.6, -0.7), (2.2, 2.4), (2.2, 3.9), (1.7, 3.3), (1.0, 2.9)]
LOWER = [(0.7, -0.9), (1.35, -0.1), (1.85, 2.7), (1.8, 3.5), (0.7, 3.1)]
SPINE = 0.8  # Achse des Rückgrats
ROUND = 40  # Segmente der großen runden Teile (Barbette, Hauptdüse)
MID = 32  # Bugkanonen und die vier kleinen Düsen
SHOCK = (1.02, 0.24)  # Achse (x, z) der Stoßdämpfer hinter dem Rammkragen


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


def half(y):
    """Halbe Breite des Rumpfs bei y (Flanke und Deckkante)."""
    return lerp(HULL, y)[1] / 2


def deck(y):
    """Höhe des Hauptdecks bei y; es steigt nach achtern leicht an."""
    return lerp(HULL, y)[2] / 2


def spine_top(y):
    """Oberkante des Rückgrats bei y zwischen −3,1 und 3,0 (sie steigt nach achtern um 0,05)."""
    return SPINE + 0.25 + 0.05 * (y + 3.1) / 6.1


def duct(y):
    """Achse (x, z) des Munitionskanals an der Flanke bei y, gerade zwischen seinen drei Stützstellen."""
    return lerp([(k, half(k) + 0.03, deck(k) - 0.18) for k in (-1.35, 0.2, 1.4)], y)[1:]


def walkway(s, m, x0, x1, y0, y1):
    """Laufsteg aus Gitterrost auf dem Hauptdeck (Steuerbord und gespiegelt) wie `parts.grating`, nur folgt er dem
    Deck, das nach achtern steigt."""
    w, cx = x1 - x0, (x0 + x1) / 2
    s.add(beam((cx, y0, deck(y0) + 0.002), (cx, y1, deck(y1) + 0.002), w, 0.012), m["dark"], mirror=True)
    n = int((y1 - y0) / 0.04)
    for k in range(n):
        y = y0 + (k + 0.5) * (y1 - y0) / n
        s.add(box(w, 0.012, 0.012), m["steel_flat"], at=(cx, y, deck(y) + 0.01), mirror=True)
    for x in (x0, x1):
        s.add(beam((x, y0 - 0.006, deck(y0) + 0.008), (x, y1 + 0.006, deck(y1) + 0.008), 0.016, 0.022), m["armor"], mirror=True)


def core_details(s, m, r, y0, y1):
    """Wie `parts.engine_details` für die große Mitteldüse, nur mit schmalerem Düsenkranz (1,04 r statt 1,16 r): Die
    Glocken der vier kleinen Düsen liegen so dicht an ihr, dass der breite Kranz in sie hineinragte."""
    length = y1 - y0
    ya = y0 + 0.6 * length
    s.add(tube(ROUND, r * 1.09, r * 1.09, y0 + 0.24 * length, y0 + 0.34 * length), m["armor_round"])
    s.add(tube(ROUND, r * 1.04, r * 1.04, ya - 0.12 * r, ya + 0.1 * r), m["dark_round"])


def details(s, m, top, z, gun):
    """Kleinteile (Modulkommentar); `top` ist das Rückgrat unter den Türmen, `z` das Dach der Brücke, `gun` die Achse
    (x, z) der Bugkanonen."""
    roof1, roof2 = top + 0.5, top + 0.95  # Dächer der beiden unteren Blöcke der Brücke
    plate_z = 0.83  # Panzerplatte auf dem Düsengehäuse

    # ---------------------------------------------------------------- Fensterreihen an der Brücke
    row = [2.52 + 0.13 * k for k in range(7)]
    windows(s, m, [(0.8, y, roof1 - 0.17) for y in row])
    windows(s, m, [(0.8, y, roof1 - 0.35) for y in row[1::2]])
    windows(s, m, [(-0.6 + 0.2 * k, 3.4, roof1 - 0.17) for k in range(7)], out="y")
    windows(s, m, [(0.625, 2.85 + 0.13 * k, roof2 - 0.2) for k in range(5)])
    windows(s, m, [(-0.4 + 0.2 * k, 3.505, roof2 - 0.2) for k in range(5)], out="y")

    # ---------------------------------------------------------------- Flak auf dem Deck neben dem Rückgrat
    # Flak nur vor und hinter dem Fensterband des Rückgrats: Davor verdeckten ihre Gehäuse von der Seite das Band
    for y, yaw, inset in ((-1.62, 25, 0.11), (2.25, 140, 0.17)):
        _, w, h = lerp(HULL, y)
        flak(s, m, (w / 2 - inset, y, h / 2), yaw)

    # ---------------------------------------------------------------- Antennen
    s.add(post(6, 0.014, 0.006, z + 0.48, z + 0.98), m["dark_round"], at=(0.3, 3.35, 0), mirror=True)
    s.add(segment((0.57, 3.44, roof2 - 0.02), (0.64, 3.95, roof2 + 0.75), 0.012, 6), m["dark_round"], mirror=True)
    # Schüssel backbord, Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, roof1 - 0.02, roof1 + 0.14), m["dark_round"], at=(-0.71, 2.64, 0))
    s.add(dish(0.11, 0.045, n=16, rings=3, thickness=0.02), m["armor_round"], at=(-0.71, 2.64, roof1 + 0.13), rot=(-40, 0, 150))
    s.add(box(0.12, 0.2, 0.12), m["armor"], at=(0.695, 2.64, roof1 + 0.06))
    s.add(post(6, 0.012, 0.006, roof1 + 0.1, roof1 + 0.42), m["dark_round"], at=(0.695, 2.7, 0))

    # ---------------------------------------------------------------- Geländer und Leiter an der Brücke
    rail(s, m, (0.745, 2.85, roof1), (0.745, 3.345, roof1), 3)  # innen neben der Fase der Dachkante
    wing = z - 0.255
    rail(s, m, (0.49, 2.99, wing), (0.97, 2.99, wing), 3)
    rail(s, m, (0.97, 2.99, wing), (0.97, 3.25, wing), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.56), m["dark"], at=(0.72 + dx, 3.407, top + 0.26))
    for k in range(6):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.72, 3.407, top + 0.05 + 0.08 * k))

    # ---------------------------------------------------------------- Düsengehäuse: Treibstoffleitungen, Kisten, Geländer
    for x in (0.78, 0.9):
        s.add(segment((x, 3.6, plate_z + 0.028), (x, 4.375, plate_z + 0.028), 0.028, 6), m["steel"], mirror=True)
    s.add(box(0.3, 0.16, 0.12), m["armor"], at=(0.84, 3.66, plate_z + 0.06), mirror=True)
    s.add(box(0.22, 0.2, 0.14), m["armor"], at=(0.3, 3.74, plate_z + 0.07))
    s.add(box(0.2, 0.22, 0.16), m["dark"], at=(-0.34, 3.75, plate_z + 0.08))
    s.add(box(0.14, 0.14, 0.1), m["armor"], at=(-0.34, 3.74, plate_z + 0.21))
    rail(s, m, (-0.7, 4.27, plate_z), (0.7, 4.27, plate_z), 5, mirror=False)

    # ---------------------------------------------------------------- Kühlrippen an Bugkanonen und kleinen Düsen
    gx, gz = gun
    for y in (-3.42, -3.29, -3.16):
        s.add(tube(MID, 0.3, 0.3, y, y + 0.03), m["steel"], at=(gx, 0, gz), mirror=True)
    for dz in (0.45, -0.45):
        for y in (4.366, 4.4085, 4.451):
            s.add(tube(MID, 0.433, 0.433, y, y + 0.025, 0.85, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter
    s.add(sphere(0.035, 1), m["glow"], at=(2.15, 3.72, 0.32), mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(0, 3.5, -1.05))


def fine(s, m, top, z, gun, lance):
    """Feinteile und Beschriftung (Modulkommentar); `top`, `z` und `gun` wie in `details`, `lance` ist der BVH-Baum der
    Karmin-Lage des Rammbugs."""
    roof1, roof2 = top + 0.5, top + 0.95
    gx, gz = gun

    # ---------------------------------------------------------------- Rammbug: Mittelgrat, Schrauben, Schneidenhalter
    # Mittelgrat der Speerspitze und Schrauben in der Karmin-Lage (in ihrer Farbe bzw. klein: die Lage bleibt ruhig);
    # dunkle Halter um die leuchtenden Schneiden und ein Eckschuh an der breitesten Stelle
    s.add(sweep([(0, -4.9, 0.144), (0, -4.5, 0.27), (0, -3.5, 0.525)], rect(0.07, 0.05)), m["accent"])
    for y in (-4.5, -4.17, -3.83, -3.5):
        x = 0.35 + 0.425 * (y + 4.5) - 0.09
        hit, turn = on_surface(lance, x, y)
        bolts(s, m["steel"], [tuple(hit)], turn, r=0.016, mirror=True)
    for (a, b), ts in zip(zip(EDGE, EDGE[1:]), ((0.6,), (0.3, 0.7), (0.45,))):
        yaw = math.degrees(math.atan2(a[0] - b[0], b[1] - a[1]))
        for k in ts:
            s.add(box(0.1, 0.06, 0.14), m["dark"], at=tuple(p + k * (q - p) for p, q in zip(a, b)), rot=(0, 0, yaw), mirror=True)
    s.add(chamfer(box(0.13, 0.15, 0.15), 0.02), m["dark"], at=EDGE[2], mirror=True)

    # ---------------------------------------------------------------- Rammkragen: Schrauben, Energieleitung zur Schneide
    # Vom Verteilerkasten am Rückgrat nach vorn, über den Kragen in den Anschlusskasten an seiner Flanke, von dort in
    # die Schneide
    bolts(s, m["steel"], [(1.05, y, h) for y in (-2.69, -2.44) for h in (-0.22, -0.05)] + [(1.05, -2.69, 0.32)], "x", r=0.016, mirror=True)
    bolts(s, m["steel"], [(0.96, y, 0.483) for y in (-2.69, -2.44)], (0, 28.7, 0), r=0.016, mirror=True)
    s.add(chamfer(box(0.07, 0.16, 0.2), 0.015), m["armor"], at=(0.9, -1.92, 0.8), mirror=True)
    s.add(pipe([(0.893, -1.98, 0.84), (0.893, -2.6, 0.84), (0.893, -2.6, 0.535), (1.066, -2.6, 0.444), (1.066, -2.6, 0.2)], 0.018, 6),
          m["steel"], mirror=True)
    for y in (-2.2, -2.42):
        s.add(box(0.03, 0.04, 0.056), m["dark"], at=(0.887, y, 0.84), mirror=True)
    s.add(chamfer(box(0.07, 0.18, 0.16), 0.015), m["armor"], at=(1.075, -2.6, 0.13), mirror=True)
    s.add(segment((1.075, -2.66, 0.1), (0.88, -2.93, 0.035), 0.018, 6), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Stoßdämpfer: Stopfbuchse, Lagerbock, Hydraulik
    dx, dz = SHOCK
    s.add(tube(24, 0.15, 0.15, -2.2, -2.13), m["dark_round"], at=(dx, 0, dz), mirror=True)
    s.add(chamfer(box(0.32, 0.12, 0.32), 0.02), m["armor"], at=(dx + 0.01, -1.76, dz), mirror=True)
    bolts(s, m["steel"], [(dx + 0.01 + sx * 0.115, -1.82, dz + sz * 0.115) for sx in (-1, 1) for sz in (-1, 1)], "-y", mirror=True)
    s.add(segment((dx, -2.38, dz + 0.14), (dx, -1.8, dz + 0.14), 0.016, 6), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Bugkanonen: Rücklaufzylinder, Klappe, Warnband
    # Zwei Rücklaufzylinder außen am Kühlmantel, ihre Kolbenstangen greifen an Ösen der Laufschelle; am Verschluss die
    # Verschlussklappe und ein Warnband vor der Rohrwurzel
    for a in (45, -45):
        c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
        s.add(tube(12, 0.055, 0.055, -3.6, -2.7), m["dark_round"], at=(gx + 0.315 * c, 0, gz + 0.315 * d), mirror=True)
        s.add(tube(6, 0.024, 0.024, -4.0, -3.58), m["steel"], at=(gx + 0.315 * c, 0, gz + 0.315 * d), mirror=True)
        s.add(box(0.11, 0.1, 0.09), m["dark"], at=(gx + 0.29 * c, -3.99, gz + 0.29 * d), rot=(0, -a, 0), mirror=True)
    hatch(s, m, (gx + 0.31, -1.7, gz), 0.3, 0.42, rot=(0, 90, 0), mirror=True)
    stripes(s, m, (gx + 0.31, -2.45, gz), 0.07, 0.4, rot=(90, 0, 90), mirror=True)

    # ---------------------------------------------------------------- Flanke: Schellen, Kästen, Dockluke, Nummer
    # Munitionskanal mit vier Schellen und verschraubten Anschlusskästen; achtern davon die Dockluke mit Trittfläche
    # (Warnband) auf der Klinge und die Kennnummer, auf jeder Seite lesbar
    for y in (-0.8, -0.2, 0.45, 1.05):
        x, h = duct(y)
        s.add(box(0.13, 0.05, 0.13), m["armor"], at=(x, y, h), mirror=True)
        bolts(s, m["steel"], [(x + 0.065, y, h + 0.04), (x + 0.065, y, h - 0.04)], "x", mirror=True)
    for y in (-1.35, 1.4):
        x, h = duct(y)
        bolts(s, m["steel"], [(x + 0.1, y + sy * 0.11, h + sz * 0.07) for sy in (-1, 1) for sz in (-1, 1)], "x", mirror=True)
    hatch(s, m, (half(1.85), 1.85, 0.47), 0.22, 0.26, rot=(0, 90, 0), mirror=True)
    stripes(s, m, (half(1.85) + 0.14, 1.85, 0.31), 0.12, 0.4, mirror=True)
    number(s, m["paint"], "02", (half(2.35), 2.35, 0.47), 0.18, (90, 0, 90))
    number(s, m["paint"], "02", (-half(2.35), 2.35, 0.47), 0.18, (90, 0, -90))

    # ---------------------------------------------------------------- Klingen: Schneidenleiste, Holme, Lagerböcke
    # Aufgedoppelte Leiste entlang der Vorderkante und drei Holme je Klinge, beides in ihrer Farbe (die Fläche bleibt auf
    # dem Brett ruhig) und verschraubt; an der Flanke je ein Lagerbock unter dem Munitionskanal
    outer, mid, inner = (grow(BLADE, -d)[:4] for d in (0.07, 0.17, 0.27))
    for k in range(3):
        s.add(plate([outer[k], outer[k + 1], inner[k + 1], inner[k]], 0.02, "xy"), m["accent"], at=(0, 0, 0.316), mirror=True)
        (ax, ay), (bx, by) = mid[k], mid[k + 1]
        count = round(math.hypot(bx - ax, by - ay) / 0.3)
        row = [(ax + (bx - ax) * (j + 0.5) / count, ay + (by - ay) * (j + 0.5) / count, 0.326) for j in range(count)]
        bolts(s, m["steel"], [q for q in row if q[0] > half(q[1]) + 0.05], mirror=True)
    for y in (-0.2, 1.05, 2.72):
        x0, x1 = half(y) + 0.1, min(2.2, 1.6 + 0.6 * (y + 0.7) / 3.1) - 0.12
        s.add(box(x1 - x0, 0.05, 0.03), m["accent"], at=((x0 + x1) / 2, y, 0.32), mirror=True)
        bolts(s, m["steel"], [(x0 + 0.05 + 0.16 * k, y, 0.335) for k in range(int((x1 - x0 - 0.06) / 0.16) + 1)], mirror=True)
        s.add(chamfer(box(0.12, 0.2, 0.1), 0.015), m["armor"], at=(half(y) + 0.055, y, 0.355), mirror=True)
        bolts(s, m["steel"], [(half(y) + 0.06, y + sy * 0.06, 0.405) for sy in (-1, 1)], mirror=True)

    # ---------------------------------------------------------------- Hauptdeck: Laufstege, Munitionsluke, Aufgang
    # Gitterrost von der vorderen Flak zur Munitionsluke über dem Magazin und weiter zur Treppe, die am Rückgrat zur
    # Tür der Brücke hinaufführt
    walkway(s, m, 0.9, 1.0, -1.18, 0.62)
    walkway(s, m, 0.9, 1.0, 1.18, 2.06)
    hatch(s, m, (half(0.9) - 0.19, 0.9, deck(0.9)), 0.2, 0.36, mirror=True)
    stairs(s, m, 0.875, 2.92, 2.52, deck(2.9) - 0.006, 0.97)
    s.add(box(0.024, 0.16, 0.25), m["dark"], at=(0.806, 2.5, 1.115), mirror=True)
    s.add(box(0.02, 0.12, 0.21), m["armor"], at=(0.816, 2.5, 1.115), mirror=True)
    s.add(box(0.012, 0.012, 0.05), m["steel_flat"], at=(0.828, 2.46, 1.11), mirror=True)
    for h in (1.05, 1.18):
        s.add(box(0.014, 0.02, 0.03), m["steel_flat"], at=(0.826, 2.552, h), mirror=True)

    # ---------------------------------------------------------------- Rückgrat: Wappen, Türme, Fensterband
    # Schrauben im Goldrand des Wappens und auf den Barbettenringen, Warnband vor dem Schwenkbereich der Rohre, dahinter
    # zwei Munitionsluken, Steigeisen an der Barbette des überhöhten Turms; am Fensterband Sims und Sprossen
    bolts(s, m["steel"], [(x, -2.1 + y, top + 0.06) for x, y in spaced(grow(PLATE, 0.04), 0.25)])
    for y, h in ((0.15, top), (1.35, top + 0.34)):
        bolts(s, m["steel"], [(0.52 * math.cos(math.radians(a)), y + 0.52 * math.sin(math.radians(a)), h + 0.05) for a in range(15, 360, 30)])
    stripes(s, m, (0, -1.1, spine_top(-1.1)), 0.07, 1.1, rot=(0, 0, 90))
    hatch(s, m, (0.34, -0.72, spine_top(-0.72)), 0.2, 0.28, mirror=True)
    c, d = math.cos(math.radians(35)), math.sin(math.radians(35))
    for k in range(3):
        s.add(box(0.03, 0.07, 0.012), m["steel_flat"], at=(0.543 * c, 1.35 + 0.543 * d, top + 0.08 + 0.06 * k), rot=(0, 0, 35), mirror=True)
    s.add(box(0.05, 3.7, 0.014), m["dark"], at=(0.89, 0.3, SPINE - 0.05), mirror=True)
    for k in range(1, 8):
        s.add(box(0.05, 0.02, 0.1), m["dark"], at=(0.888, -1.5 + 0.45 * k, SPINE), mirror=True)

    # ---------------------------------------------------------------- Brücke: Simse, Konsolen, Mastfuß, Dachaggregate
    for x, y0, y1, h in ((0.8, 2.52, 3.3, roof1 - 0.17), (0.8, 2.65, 3.17, roof1 - 0.35), (0.625, 2.85, 3.37, roof2 - 0.2)):
        s.add(box(0.03, y1 - y0 + 0.12, 0.012), m["dark"], at=(x + 0.01, (y0 + y1) / 2, h - 0.031), mirror=True)
    s.add(box(1.32, 0.03, 0.012), m["dark"], at=(0, 3.41, roof1 - 0.201))
    s.add(box(0.92, 0.03, 0.012), m["dark"], at=(0, 3.515, roof2 - 0.231))
    for y in (3.04, 3.2):
        s.add(plate([(0.62, z - 0.41), (0.95, z - 0.41), (0.62, z - 0.68)], 0.03, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    s.add(post(16, 0.14, 0.14, z - 0.004, z + 0.02), m["hull_round"], at=(0, 3.35, 0))
    bolts(s, m["steel"], [(0.115 * math.cos(math.radians(a)), 3.35 + 0.115 * math.sin(math.radians(a)), z + 0.02) for a in range(0, 360, 60)])
    s.add(segment((0.04, 3.36, z + 0.42), (0.2, 3.56, z - 0.005), 0.01, 6), m["dark_round"], mirror=True)
    s.add(chamfer(box(0.2, 0.14, 0.08), 0.012), m["armor"], at=(-0.25, 3.18, z + 0.036))
    s.add(box(0.12, 0.08, 0.012), m["dark"], at=(-0.25, 3.18, z + 0.078))
    s.add(sphere(0.06, 2), m["steel"], at=(0.25, 3.18, z + 0.02))

    # ---------------------------------------------------------------- Heck: Wartungsluke, Rohrschellen, Stellstreben
    # Am Düsengehäuse je Flanke eine Wartungsluke zwischen Kennband und RCS und Schrauben in den Ecken, auf seiner
    # Panzerplatte Schellen über den Treibstoffleitungen; Stellstreben außen an den vier kleinen Düsen und oben und
    # unten an der Hauptdüse (dazwischen sitzen die Glocken zu dicht)
    hatch(s, m, (1.3, 3.97, -0.03), 0.24, 0.34, rot=(0, 90, 0), mirror=True)
    bolts(s, m["steel"], [(1.3, y, h) for y in (3.68, 4.22) for h in (-0.62, 0.62)], "x", r=0.016, mirror=True)
    for y in (3.95, 4.2):
        s.add(box(0.2, 0.03, 0.07), m["dark"], at=(0.84, y, 0.862), mirror=True)
    r, y0, y1 = 0.38, 4.0, 4.95
    for sign in (1, -1):
        for a in range(-32, 119, 30):
            c, d = math.cos(math.radians(sign * a)), math.sin(math.radians(sign * a))
            s.add(segment((0.85 + 1.1 * r * c, y0 + 0.6 * (y1 - y0), sign * 0.45 + 1.1 * r * d),
                          (0.85 + 0.93 * r * c, y1 - 0.03, sign * 0.45 + 0.93 * r * d), 0.016, 6), m["steel"], mirror=True)
    r, y1 = 0.58, 5.15
    for a in (70, 90, 110, 250, 270, 290):
        c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
        s.add(segment((1.04 * r * c, y0 + 0.6 * (y1 - y0), 1.04 * r * d), (0.93 * r * c, y1 - 0.03, 0.93 * r * d), 0.016, 6), m["steel"])

    # ---------------------------------------------------------------- Beschriftung: Wegelinien, Warnband, Nummer
    # Linien an beiden Kanten des Rückgrats vom Warnband bis zur Brücke, Warnband vor dem Maschinenlüfter, die Nummer an
    # der Rückwand der Brücke, lesbar von hinten oben
    s.add(beam((0.585, -1.0, spine_top(-1.0) + 0.001), (0.585, 1.87, spine_top(1.87) + 0.001), 0.02, 0.006), m["paint"], mirror=True)
    stripes(s, m, (0, 3.895, 0.83), 0.06, 0.8, rot=(0, 0, 90))
    number(s, m["paint"], "02", (0, 3.625, z - 0.19), 0.2, (90, 0, 180))


def build():
    s = Ship("vanguard")
    m = vf.materials(s.name, "cruiser")

    # ---------------------------------------------------------------- Rumpf, Rückgrat, Klingen
    # Kantiger Keilrumpf mit Rückgrat, Zierlinie und Fensterband
    # Der Rumpf bleibt ungefast: Seine Deckkante liegt unter der Zierlinie, die übrigen Kanten stecken in Klingen und Kiel
    s.add(loft([sec(y, keel(w, h)) for y, w, h in HULL]), m["hull"])
    spine = SPINE
    s.add(chamfer(loft([sec(-3.5, octagon(1.1, 0.34), dz=-0.14), sec(-3.1, octagon(1.75, 0.5)), sec(3.0, octagon(1.75, 0.6)),
                        sec(3.6, octagon(1.1, 0.4))]), 0.04), m["hull"], at=(0, 0, spine))
    trim_line(s, m, [(w / 2 + 0.01, y, h / 2 + 0.005) for y, w, h in HULL])
    s.add(box(0.04, 3.6, 0.08), m["window"], at=(0.885, 0.3, spine), mirror=True)
    # Gepfeilte Panzerklingen in zwei Lagen, unter der unteren eine zweite Platte
    s.add(chamfer(plate(BLADE, 0.22, "xy"), 0.04), m["accent"], at=(0, 0, 0.2), mirror=True)
    s.add(chamfer(plate(LOWER, 0.18, "xy"), 0.03), m["hull"], at=(0, 0, -0.2), mirror=True)
    s.add(chamfer(plate(grow(LOWER, -0.14), 0.07, "xy"), 0.02), m["armor"], at=(0, 0, -0.32), mirror=True)

    # ---------------------------------------------------------------- Rammbug
    # Speerspitze: Klinge mit scharfen Seitenkanten (sie bleibt ungefast, die Schneiden sitzen auf ihren Kanten),
    # Panzerlage in Karmin, leuchtende Schneiden
    s.add(loft([tip(RAM_TIP, dz=-0.1)] + [sec(y, hexagon(w, h), dz=dz) for y, w, h, dz in RAM]), m["hull"])
    lance = chamfer(loft([tip(LANCE_TIP, dz=0.05)] + [sec(y, hexagon(w, h), dz=dz) for y, w, h, dz in LANCE]), 0.03)
    skin = BVHTree.FromBMesh(lance)
    s.add(lance, m["accent"])
    s.add(sweep(EDGE, rect(0.06, 0.1)), m["glow"], mirror=True)
    # Rammkragen an der Wurzel, dahinter je Flanke ein Stoßdämpfer zum Rumpf; Spanten unter der Klinge
    s.add(chamfer(loft([sec(y, octagon(2.1, 1.15, 0.35), dz=0.06) for y in (-2.75, -2.38)]), 0.03), m["dark"])
    dx, dz = SHOCK
    s.add(bellows(24, 0.12, -2.42, -2.17), m["armor_round"], at=(dx, 0, dz), mirror=True)
    s.add(tube(24, 0.13, 0.13, -2.17, -1.75), m["dark_round"], at=(dx, 0, dz), mirror=True)
    for y in (-4.35, -3.85, -3.35):
        _, _, h, d = lerp(RAM, y)
        s.add(box(0.6, 0.12, 0.1), m["dark"], at=(0, y, d - h / 2 - 0.03))

    # ---------------------------------------------------------------- Bugkanonen unter dem Rammbug
    # Verschluss mit Munitionszuführung vom Rumpf, Blendenbalg, Lauf, Kühlmantel, Laufschelle, zweiteilige
    # Mündungsbremse mit Stahlkern im Spalt und glühendem Kern
    gx, gz = 0.78, -0.62
    s.add(rbox(0.62, 1.7, 0.6, 0.06, 1), m["dark"], at=(gx, -1.9, gz), mirror=True)
    s.add(chamfer(box(0.3, 0.34, 0.36), 0.025), m["armor"], at=(gx, -1.45, -0.2), mirror=True)
    s.add(bellows(MID, 0.31, -3.0, -2.74), m["armor_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.21, 0.19, -4.75, -2.6), m["steel"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.27, 0.27, -3.5, -3.0), m["dark_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.25, 0.25, -4.05, -3.93), m["armor_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.18, 0.18, -5.0, -4.62), m["steel"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.29, 0.29, -5.04, -4.9), m["dark_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.29, 0.29, -4.78, -4.62), m["dark_round"], at=(gx, 0, gz), mirror=True)
    s.add(tube(MID, 0.14, 0.14, -5.08, -5.02), m["glow"], at=(gx, 0, gz), mirror=True)
    # Munitionskanal an der Flanke vom Magazin unter den Türmen zur Zuführung, mit Anschlusskästen an beiden Enden
    run = []
    for y in (1.4, 0.2, -1.35):
        _, w, h = lerp(HULL, y)
        run.append((w / 2 + 0.03, y, h / 2 - 0.18))
    s.add(sweep(run, rect(0.1, 0.1)), m["dark"], mirror=True)
    for x, y, z in (run[0], run[-1]):
        s.add(chamfer(box(0.18, 0.3, 0.22), 0.02), m["armor"], at=(x + 0.01, y, z), mirror=True)

    # ---------------------------------------------------------------- Türme
    # Zwei Türme auf dem Rückgrat, der hintere überhöht auf einer Barbette, beide mit ihren Kleinteilen
    top = spine + 0.25
    turret(s, m, (0, 0.15, top), r=0.46, barrel=1.15)
    turret_details(s, m, (0, 0.15, top), r=0.46, barrel=1.15)
    s.add(post(ROUND, 0.54, 0.52, top, top + 0.34), m["hull_round"], at=(0, 1.35, 0))
    turret(s, m, (0, 1.35, top + 0.34), r=0.46, barrel=1.35)
    turret_details(s, m, (0, 1.35, top + 0.34), r=0.46, barrel=1.35)

    # Wappenplatte vor den Türmen
    badge(s, m, (0, -2.1, top), PLATE, ARROW)

    # ---------------------------------------------------------------- Brücke
    # Nach achtern gestaffelt mit Fensterband, Brückenflügel unter den Seitenfenstern, Mast mit Querträger,
    # Radarplatte und Licht
    z = top
    for w, l, h, y, width in ((1.6, 1.5, 0.5, 2.65, 0.04), (1.25, 1.15, 0.45, 2.93, 0.035), (0.95, 0.85, 0.4, 3.2, 0.03)):
        s.add(deckhouse(w, l, h, width), m["hull"], at=(0, y, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, 3.2 - 0.425 + 0.11, z - 0.16))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.485, 3.25, z - 0.16), mirror=True)
    s.add(box(0.6, 0.28, 0.16), m["hull"], at=(0.68, 3.12, z - 0.335), mirror=True)
    s.add(post(16, 0.09, 0.06, z, z + 0.6), m["hull_round"], at=(0, 3.35, 0))
    s.add(box(0.7, 0.07, 0.07), m["hull"], at=(0, 3.35, z + 0.48))
    s.add(box(0.46, 0.05, 0.18), m["armor"], at=(0, 3.26, z + 0.22), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, 3.35, z + 0.64))

    # ---------------------------------------------------------------- Heck: Düsencluster
    # Gehäuse mit Panzerplatte und Maschinenlüfter, RCS an den Flanken; eine große und vier kleine Düsen
    s.add(rbox(2.6, 0.8, 1.55, 0.08, 1), m["dark"], at=(0, 3.95, 0.0))
    s.add(chamfer(plate([(-1.0, 3.55), (1.0, 3.55), (1.0, 4.3), (-1.0, 4.3)], 0.06, "xy"), 0.015), m["armor"], at=(0, 0, 0.8))
    vent(s, m, m["armor"], (0, 4.1, 0.83), 0.32, 0.8, slats=2, rot=(0, 0, 90))
    s.add(box(0.04, 0.66, 0.12), m["signal"], at=(1.3, 3.95, 0.3), mirror=True)
    rcs(s, m, (1.24, 3.75, -0.42), 1)
    engine(s, m, 0.58, 4.0, 5.15, n=ROUND)
    core_details(s, m, 0.58, 4.0, 5.15)
    for dz in (0.45, -0.45):
        engine(s, m, 0.38, 4.0, 4.95, dx=0.85, dz=dz, n=MID, mirror=True)
        engine_details(s, m, 0.38, 4.0, 4.95, dx=0.85, dz=dz, n=MID, mirror=True)

    # ---------------------------------------------------------------- Unterseite: Kiel, Spanten am V-Boden
    s.add(chamfer(plate([(-1.4, -0.5), (3.4, -0.5), (3.6, -1.05), (-0.4, -0.95)], 0.14, "yz"), 0.03), m["dark"])
    for y in (0.3, 0.9, 1.5, 2.1):
        x0, x1 = 0.02, 0.74
        z0, z1 = bottom_z(x0, y), bottom_z(x1, y)
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.12, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    details(s, m, top, z, (gx, gz))
    fine(s, m, top, z, (gx, gz), skin)
    return s.finish()
