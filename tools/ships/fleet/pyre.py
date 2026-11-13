"""Pyre (Flaggschiff von Admiralin Reyes, „die Glut“), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

Schweres, breites Schlachtschiff, das heiß wirkt. Ein Keilrumpf mit hohem Rückendeck und goldener Zierlinie, davor der
Plasmawerfer als Bug: ein Gehäuse in Glutorange mit Fensterband, darauf ein dunkler Kühlmantel und ein Emitterkegel,
vor dem drei dunkle Klauen eine leuchtende Glutkugel halten. An beiden Flanken je ein langer, dunkler
Kühlkörper mit vier schrägen Glutschlitzen, auf dem Rückendeck zwei Reihen großer Kühlblöcke mit Glutkante.
Dahinter die Wappenplatte mit Reyes' Flamme, achtern die Brücke aus drei Keilen mit Fensterband und Mast, Panzerschürzen
in Glutorange, Kiel und drei schwere Düsen.
Detailzonen mit Zweck (Brennstoff, Plasma, Hitze), dazwischen ruhige Flächen (Rückendeck vor den Kühlblöcken und
zwischen ihnen, Flanken über den Kühlkörpern, Gehäuse des Werfers, Flächen der Brücke):
- Bug: Hitzeschild als große Scheibe vor dem Kühlmantel, aus der die Klauen ragen; Gelenke an den Klauenwurzeln,
  hitzefeste Klauenspitzen (`armor`) um die Glutkugel
- Brennstoff: zwei Satteltanks neben der Brücke, halb ins Rückendeck gesenkt, mit Spannbändern und Ventil; von jedem
  eine Zuleitung auf der Schulter nach vorn in den Plasmawerfer, mit Rohrschelle und Anschlussflansch
- Flanke: Kühlkörper mit Glutschlitzen, RCS-Blöcke an den Rumpfecken
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte und Querträger
- Heck: Maschinenlüfter hinter der Brücke, Gehäusesegment und Düsenkranz je Düse (`parts.engine_details`)
- Unterseite: zweite Plattenlage unter den Schürzen
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an der Brücke und an der Wand des
Rückendecks, Geländer und Leiter an der Brücke, Flak auf den Schultern und dem Brückendach, Peitschenantennen, Schüssel
und Gerätekasten, Rohre von den Tanks zur Brücke, Tankdeckel und Kisten vor der Brücke, Kühlrippen auf den
Kühlkörpern, am Kühlmantel des Werfers und an den Düsen, Positionslichter.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen, wo das Schiff Hitze abführt, Brennstoff leitet und
gewartet wird): Abwärmegitter und Luke auf dem Werfergehäuse, Kühlmittelleitungen über den Kühlmantel, Schrauben am
Hitzeschild, Stellzylinder, Kühlrippen und Gelenkbolzen an den Klauen; Lamellenpakete zwischen den Glutschlitzen, deren
Schienen, ein Rippenrohr und Konsolen an den Kühlkörpern; Laufsteg aus Gitterrost mit Geländer auf der Schulter,
Steigeisen zum Rückendeck, Sättel und Flansche der Zuleitung, Handrad am Ventil, Tankdom und Spannschlösser; Lamellen,
Wartungsdeckel und Kühlmittelanschluss an jedem Kühlblock; Luken, Fenstersimse, Konsolen unter den Brückenflügeln,
Dachaggregate, Stellstreben an den Düsenglocken, Lüfter an den Flanken des Düsenblocks, Klammern an den Schürzen. Dazu
Beschriftung als flache Geometrie: Wegelinien entlang der Kühlblöcke, Warnbänder am Steigeisen und über den Düsen, die
Kennnummer auf dem Rückendeck und am Kragen des Werfers.
"""

import math

from mathutils import Vector

import vf
from parts import (TURN, axle, badge, beam, bolts, engine, engine_details, flak, hatch, number, rail, rcs, spaced, stripes, sym, trim_line,
                   vent, windows)
from vf import (Ship, box, chamfer, dish, grow, hoop, keel, loft, ngon, octagon, pipe, plate, post, revolve, sec, segment, sphere, sweep,
                tube, wedge)

# Rumpf: Querschnitte (y, Breite, Höhe) des Keilprofils um z = 0
HULL = ((-3.4, 2.0, 1.1), (-2.4, 3.4, 1.5), (2.2, 3.6, 1.6), (3.9, 2.8, 1.3), (4.5, 2.0, 1.0))
DECK = 1.15  # Bezugshöhe des Rückendecks; seine Oberkante liegt bei TOP
TOP = DECK + 0.1
PZ = 0.25  # Achse des Plasmawerfers
SKIRT = [(0.7, -2.3), (1.95, -2.0), (2.0, 3.0), (0.7, 3.6)]
CLAW = [(-4.2, 0.3), (-4.2, 0.82), (-5.2, 0.8), (-5.85, 0.46), (-5.7, 0.3)]
CLAW_TIP = [(-5.15, 0.28), (-5.15, 0.825), (-5.22, 0.822), (-5.88, 0.47), (-5.72, 0.28)]
TANK = (1.32, 1.9, 1.1)  # Mitte des Steuerbord-Tanks; Radius 0,3, von y 1,2 bis 2,6
# Wappen: Platte mit Spitze zum Bug, darauf eine Flamme, die zum Bug lodert
PLATE = sym([(0.0, -0.95), (0.62, -0.45), (0.62, 0.55), (0.0, 0.8)])
FLAME = sym([(0.0, -0.74), (0.13, -0.3), (0.3, -0.48), (0.38, -0.04), (0.34, 0.28), (0.18, 0.48), (0.0, 0.54)])
ROUND = 40  # Segmente der großen runden Teile (Kühlmantel, Hitzeschild, Tanks, Düsen)
BLOCKS = (-1.4, -0.35, 0.7)  # Kühlblöcke auf dem Rückendeck (y der Mitte)
SLOTS = (-1.1, -0.15, 0.8, 1.75)  # Glutschlitze der Kühlkörper (y der Mitte)
ENGINES = ((0.52, 5.35, 0.0, 0.25, False), (0.4, 5.15, 0.95, 0.15, True))  # Radius, Düsenende, dx, dz, gespiegelt


def hull_at(y):
    """Rumpf bei y: halbe Breite und Höhe der Oberkante (die Schulter neben dem Rückendeck)."""
    for (y0, w0, h0), (y1, w1, h1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            return (w0 + t * (w1 - w0)) / 2, (h0 + t * (h1 - h0)) / 2
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def claw(m):
    """Eine Klaue des Werfers als Baugruppe um die Werferachse (oben liegend): dunkle Platte, Gelenk an der Wurzel vor dem
    Hitzeschild, hitzefeste Spitze."""
    g = Ship("claw")
    g.add(chamfer(plate(CLAW, 0.3, "yz"), 0.03), m["dark"])
    g.add(axle(0.13, 0.42, 24), m["dark_round"], at=(0, -4.6, 0.6))
    g.add(chamfer(plate(CLAW_TIP, 0.34, "yz"), 0.03), m["armor"])
    # Feinteile: Gelenkbolzen, Kühlrippen an den Flanken, Schrauben an der Naht der Spitze, auf dem Rücken ein
    # Stellzylinder vom Hitzeschild zur Spitze
    for sx, out in ((1, "x"), (-1, "-x")):
        g.add(post(6, 0.055, 0.055, -0.01, 0.025), m["steel_flat"], at=(sx * 0.21, -4.6, 0.6), rot=TURN[out])
        for z in (0.4, 0.48, 0.56, 0.64, 0.72):
            g.add(box(0.03, 0.3, 0.016), m["dark"], at=(sx * 0.155, -4.94, z))
        bolts(g, m["steel_flat"], [(sx * 0.17, -5.2, z) for z in (0.42, 0.56, 0.7)], out)
    g.add(box(0.1, 0.06, 0.08), m["armor"], at=(0, -4.54, 0.855))
    g.add(tube(12, 0.04, 0.04, -4.95, -4.5, dz=0.855), m["dark_round"])
    g.add(tube(8, 0.02, 0.02, -5.15, -4.94, dz=0.855), m["steel"])
    g.add(box(0.07, 0.06, 0.07), m["armor"], at=(0, -5.17, 0.85))
    return g


def details(s, m, z, by):
    """Kleinteile (Modulkommentar); `z` ist das Dach der Brücke, `by` ihre Mitte."""
    roof1, roof2, wing = TOP + 0.55, TOP + 1.0, z - 0.255  # Dächer der beiden unteren Keile, Oberseite der Brückenflügel
    tx, ty, tz = TANK

    # ---------------------------------------------------------------- Fensterreihen: Brücke und Wand des Rückendecks
    row = [by - 0.14 + 0.12 * k for k in range(8)]
    windows(s, m, [(0.9, y, roof1 - 0.215) for y in row])
    windows(s, m, [(0.9, y, roof1 - 0.395) for y in row[1::2]])
    windows(s, m, [(-0.6 + 0.2 * k, by + 0.75, roof1 - 0.215) for k in range(7)], out="y")
    windows(s, m, [(0.675, by - 0.09 + 0.12 * k, roof2 - 0.185) for k in range(6)])
    windows(s, m, [(-0.4 + 0.2 * k, by + 0.55, roof2 - 0.185) for k in range(5)], out="y")
    # Wand des Rückendecks über der Zuleitung: vorn und neben der Brücke
    windows(s, m, [(1.1 + 0.05 * (y + 2.3) / 5.9, y, 1.03) for y in [-2.0 + 0.12 * k for k in range(5)] + [2.9 + 0.12 * k for k in range(6)]])

    # ---------------------------------------------------------------- Flak: Schultern vorn und achtern, Dach des unteren Keils
    for (x, y), yaw in (((1.6, -1.6), 30), ((1.36, 3.15), 140)):
        flak(s, m, (x, y, hull_at(y)[1]), yaw)
    flak(s, m, (0.79, by + 0.22, roof1), 90)

    # ---------------------------------------------------------------- Antennen
    s.add(post(6, 0.014, 0.006, z + 0.58, z + 1.1), m["dark_round"], at=(0.4, by + 0.15, 0), mirror=True)
    s.add(segment((0.58, by + 0.5, roof2 - 0.02), (0.66, by + 1.0, roof2 + 0.7), 0.012, 6), m["dark_round"], mirror=True)
    # Schüssel backbord (zeigt nach achtern außen), Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, roof1 - 0.02, roof1 + 0.14), m["dark_round"], at=(-0.79, by + 0.55, 0))
    s.add(dish(0.13, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.79, by + 0.55, roof1 + 0.13), rot=(-40, 0, -150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(0.79, by + 0.58, roof1 + 0.06))
    s.add(post(6, 0.012, 0.006, roof1 + 0.1, roof1 + 0.5), m["dark_round"], at=(0.79, by + 0.64, 0))

    # ---------------------------------------------------------------- Geländer und Leiter an der Brücke
    # (hinter den Fasen der Dachkanten)
    rail(s, m, (0.845, by + 0.4, roof1), (0.845, by + 0.695, roof1), 3)
    rail(s, m, (-0.845, by + 0.695, roof1), (0.845, by + 0.695, roof1), 5, mirror=False)
    rail(s, m, (0.62, by + 0.1, roof2), (0.62, by + 0.495, roof2), 3)
    rail(s, m, (0.45, by - 0.15, wing), (0.99, by - 0.15, wing), 3)
    rail(s, m, (0.99, by - 0.15, wing), (0.99, by + 0.09, wing), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.5), m["dark"], at=(-0.75 + dx, by + 0.757, TOP + 0.25))
    for k in range(6):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(-0.75, by + 0.757, TOP + 0.065 + 0.08 * k))

    # ---------------------------------------------------------------- Brennstoff: Rohre von den Tanks in die Brücke, Tankdeckel
    for x, y in ((0.62, 1.47), (0.7, 1.56)):  # zwischen Tankboden und vorderem Spannband
        s.add(pipe([(1.1, y, TOP + 0.028), (x, y, TOP + 0.028), (x, by - 0.63, TOP + 0.028)], 0.028, 6), m["steel"], mirror=True)
    s.add(post(16, 0.07, 0.07, tz + 0.27, tz + 0.34), m["dark_round"], at=(tx, ty, 0), mirror=True)
    # Lagerkisten vor der Brücke
    s.add(box(0.22, 0.2, 0.14), m["armor"], at=(-0.3, by - 0.86, TOP + 0.07))
    s.add(box(0.2, 0.22, 0.16), m["dark"], at=(0.32, by - 0.87, TOP + 0.08))
    s.add(box(0.14, 0.14, 0.1), m["armor"], at=(0.32, by - 0.87, TOP + 0.21))

    # ---------------------------------------------------------------- Kühlrippen: Kühlkörper, Kühlmantel des Werfers, Düsen
    for y0 in (-0.62, 0.32, 1.28):  # je drei zwischen den Glutschlitzen
        for dy in (-0.1, 0.0, 0.1):
            s.add(box(0.22, 0.03, 0.07), m["armor"], at=(1.9, y0 + dy, 0.48), mirror=True)
    for y in (-4.24, -4.12, -4.0):  # ohne Glanz: die Ringflächen schauen in `hero` zur Kamera
        r = 0.7 - 0.08 * (y + 4.4) / 0.65 + 0.035
        s.add(tube(ROUND, r, r, y, y + 0.04, dz=PZ), m["armor_round"])
    for r, ys, dx, dz, mirror in ((0.52, (4.742, 4.78, 4.818), 0.0, 0.25, False), (0.4, (4.675, 4.705, 4.735), 0.95, 0.15, True)):
        for y in ys:
            s.add(tube(ROUND, r * 1.14, r * 1.14, y, y + 0.02, dx, dz), m["steel"], mirror=mirror)

    # ---------------------------------------------------------------- Positionslichter: Brückenflügel, Kühlkörper achtern, Heck
    s.add(sphere(0.03, 1), m["glow"], at=(1.0, by - 0.03, z - 0.225), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(2.0, 2.24, 0.46), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(0, 4.61, 0.805))


def catwalk(s, m, x0, x1, y0, y1):
    """Laufsteg aus Gitterrost auf der Schulter (Steuerbord und gespiegelt) wie `parts.grating`; er folgt der Schulter,
    die nach achtern leicht steigt."""
    cx, n = (x0 + x1) / 2, int((y1 - y0) / 0.04)
    z0, z1 = hull_at(y0)[1], hull_at(y1)[1]
    s.add(beam((cx, y0, z0 + 0.002), (cx, y1, z1 + 0.002), x1 - x0, 0.012), m["dark"], mirror=True)
    for k in range(n):
        y = y0 + (k + 0.5) * (y1 - y0) / n
        s.add(box(x1 - x0, 0.012, 0.012), m["steel_flat"], at=(cx, y, hull_at(y)[1] + 0.01), mirror=True)
    for x in (x0, x1):
        s.add(beam((x, y0 - 0.006, z0 + 0.008), (x, y1 + 0.006, z1 + 0.008), 0.016, 0.022), m["armor"], mirror=True)


def fine(s, m, z, by):
    """Feinteile und Beschriftung (Modulkommentar); `z` ist das Dach der Brücke, `by` ihre Mitte."""
    roof1, roof2 = TOP + 0.55, TOP + 1.0
    tx, ty, tz = TANK

    def around(r, step, start=0):
        """Punkte (Sinus, Kosinus) · r im Abstand von `step` Grad."""
        return [(a, r * math.sin(math.radians(a)), r * math.cos(math.radians(a))) for a in range(start, 360, step)]

    # ---------------------------------------------------------------- Werfergehäuse: Abwärmegitter, Luke, Kennnummer
    # Gitter auf der Schräge vor der Wartungsluke (die Schräge fällt um 14° zum Bug); die Kennnummer steht am Kragen,
    # der sich zum Kühlmantel verjüngt (21,8°), auf jeder Seite lesbar
    vent(s, m, m["accent"], (0, -3.65, PZ + 0.6875), 0.7, 0.3, slats=2, rot=(14, 0, 0))
    hatch(s, m, (0, -3.21, PZ + 0.75), 0.36, 0.26)
    number(s, m["dark"], "04", (0.9, -3.65, PZ), 0.26, (90, 0, 68.2))
    number(s, m["dark"], "04", (-0.9, -3.65, PZ), 0.26, (90, 0, -68.2))

    # ---------------------------------------------------------------- Kühlmantel und Hitzeschild
    # Zwei Kühlmittelleitungen je Seite vom Gehäuse über die Rippen des Mantels in den Schild, Schrauben an dessen
    # Rückseite (zwischen den Klauen)
    for a in (52, 68):
        c, d = math.sin(math.radians(a)), math.cos(math.radians(a))
        s.add(segment((0.71 * c, -3.86, PZ + 0.71 * d), (0.76 * c, -4.36, PZ + 0.76 * d), 0.02, 6), m["steel"], mirror=True)
    bolts(s, m["steel_flat"], [(x, -4.33, PZ + dz) for a, x, dz in around(0.76, 20) if a % 120], "y", r=0.012)

    # ---------------------------------------------------------------- Kühlkörper: Schienen, Lamellen, Rippenrohr, Konsolen
    # Jeder Glutschlitz sitzt zwischen zwei verschraubten Schienen, dazwischen Lamellenpakete in derselben Neigung;
    # oben läuft ein Sammelrohr durch die Kühlrippen, Konsolen tragen den Kühlkörper an der Rumpfwand
    for y in SLOTS:
        g = Ship("slot")
        for sy in (-1, 1):
            g.add(box(0.035, 0.025, 0.44), m["dark"], at=(0.0075, sy * 0.105, 0))
            bolts(g, m["steel_flat"], [(0.025, sy * 0.105, sz * 0.16) for sz in (-1, 1)], "x", r=0.01)
        s.place(g, at=(2.05, y, 0.15), rot=(25, 0, 0), mirror=True)
        g.bm.free()
        s.add(plate([(0.0, 0.0), (0.2, 0.0), (0.0, 0.22)], 0.04, "xz"), m["armor"], at=(hull_at(y)[0] - 0.008, y, 0.445), mirror=True)
    for y in (-0.625, 0.325, 1.275):
        g = Ship("fins")
        for k in range(-2, 3):
            g.add(box(0.03, 0.016, 0.4), m["dark"], at=(0.005, 0.05 * k, 0))
        s.place(g, at=(2.05, y, 0.15), rot=(25, 0, 0), mirror=True)
        g.bm.free()
    ends = [(hull_at(y)[0] + 0.035, y, 0.465) for y in (-1.45, 2.25)]
    s.add(segment(*ends, 0.022, 6), m["steel"], mirror=True)
    for x, y, _ in ends:
        s.add(box(0.08, 0.08, 0.07), m["armor"], at=(x, y, 0.48), mirror=True)
    # Wartungsluke in der Rumpfwand vor dem Kühlkörper
    hatch(s, m, (hull_at(-2.15)[0], -2.15, 0.45), 0.26, 0.3, rot=(0, 90, 0), mirror=True)

    # ---------------------------------------------------------------- Schulter: Laufsteg, Steigeisen, Zuleitung
    # Laufsteg aus Gitterrost neben der Zuleitung, Steigeisen an der Wand des Rückendecks in der Lücke zwischen den
    # Kühlblöcken; die Zuleitung liegt in zwei Sätteln und hat zwei Flansche
    catwalk(s, m, 1.52, 1.66, -1.35, 1.0)
    rail(s, m, (1.66, -1.35, hull_at(-1.35)[1] + 0.015), (1.66, 1.0, hull_at(1.0)[1] + 0.015), 9)
    wall = 1.1 + 0.05 * (0.175 + 2.3) / 5.9
    for k in range(5):
        s.add(box(0.03, 0.07, 0.012), m["steel_flat"], at=(wall + 0.008, 0.175, 0.86 + 0.055 * k), mirror=True)
    for y in (-1.25, 0.55):
        top = hull_at(y)[1]
        s.add(box(0.28, 0.06, 0.07), m["armor"], at=(1.36, y, top + 0.03), mirror=True)
        bolts(s, m["steel_flat"], [(1.36 + dx, y, top + 0.065) for dx in (-0.12, 0.12)], r=0.01, mirror=True)
    for y in (-1.7, 0.95):
        s.add(tube(24, 0.125, 0.125, y - 0.025, y + 0.025, dx=1.36, dz=0.835 + 0.035 * (y + 1.95) / 3.3), m["armor_round"], mirror=True)

    # ---------------------------------------------------------------- Tanks: Tankdom, Spannschlösser, Handrad am Ventil
    s.add(post(24, 0.11, 0.11, tz + 0.27, tz + 0.305), m["armor_round"], at=(tx, ty, 0), mirror=True)
    bolts(s, m["steel_flat"], [(tx + x, ty + y, tz + 0.305) for _, x, y in around(0.09, 60)], r=0.01, mirror=True)
    for dy in (-0.25, 0.25):
        s.add(box(0.07, 0.13, 0.03), m["armor"], at=(tx, ty + dy, tz + 0.335), mirror=True)
        bolts(s, m["steel_flat"], [(tx, ty + dy + e, tz + 0.35) for e in (-0.04, 0.04)], r=0.01, mirror=True)
    s.add(post(6, 0.014, 0.014, 1.01, 1.075), m["steel_flat"], at=(1.36, 1.18, 0), mirror=True)
    s.add(hoop(8, 0.04, 0.06, 0.014), m["steel_flat"], at=(1.36, 1.18, 1.07), mirror=True)
    s.add(box(0.1, 0.012, 0.01), m["steel_flat"], at=(1.36, 1.18, 1.07), mirror=True)
    s.add(box(0.012, 0.1, 0.01), m["steel_flat"], at=(1.36, 1.18, 1.074), mirror=True)

    # ---------------------------------------------------------------- Rückendeck: Luken, Schrauben am Wappen, Kühlblöcke
    hatch(s, m, (0, 1.5, TOP), 0.36, 0.3)
    hatch(s, m, (0, -2.62, 1.1586), 0.34, 0.3, rot=(15.95, 0, 0))  # auf der Schräge zum Bug
    bolts(s, m["steel"], [(x, y - 0.35, TOP + 0.01) for x, y in spaced(grow(PLATE, 0.04), 0.25)])
    # Kühlblöcke: Lamellen in der Schräge unter der Glutkante, verschraubter Wartungsdeckel an beiden Flanken, dahinter
    # der Kühlmittelanschluss ins Deck
    for y in BLOCKS:
        for t in (0.15, 0.35, 0.55):
            s.add(box(0.28, 0.02, 0.02), m["armor"], at=(1.0, y - 0.375 + 0.375 * t, TOP + 0.2275 + 0.2475 * t), rot=(33.4, 0, 0), mirror=True)
        for x, sx, out in ((1.21, 1, "x"), (0.79, -1, "-x")):
            s.add(box(0.016, 0.26, 0.2), m["dark"], at=(x + sx * 0.002, y + 0.2, TOP + 0.24), mirror=True)
            bolts(s, m["steel_flat"], [(x + sx * 0.01, y + 0.2 + dy, TOP + 0.24 + dz) for dy in (-0.1, 0.1) for dz in (-0.07, 0.07)], out,
                  r=0.01, mirror=True)
        s.add(pipe([(0.9, y + 0.36, TOP + 0.1), (0.9, y + 0.45, TOP + 0.1), (0.9, y + 0.45, TOP - 0.05)], 0.03, 8), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Brücke: Tür, Fenstersimse, Konsolen, Dach
    hatch(s, m, (0, by - 0.75, TOP + 0.15), 0.26, 0.2, rot=(90, 0, 0))
    s.add(box(0.03, 0.94, 0.014), m["dark"], at=(0.908, by + 0.28, roof1 - 0.251), mirror=True)
    s.add(box(1.34, 0.03, 0.014), m["dark"], at=(0, by + 0.758, roof1 - 0.251))
    s.add(box(0.03, 0.7, 0.014), m["dark"], at=(0.683, by + 0.21, roof2 - 0.221), mirror=True)
    s.add(box(0.94, 0.03, 0.014), m["dark"], at=(0, by + 0.558, roof2 - 0.221))
    for dy in (-0.03, 0.085):  # zwischen den Fenstern
        s.add(plate([(0.0, 0.0), (0.28, 0.0), (0.0, -0.26)], 0.02, "xz"), m["hull"], at=(0.67, by + dy, z - 0.41), mirror=True)
    # Dach: Aggregat mit Deckel, Radom, Mastfuß mit Schrauben
    s.add(chamfer(box(0.2, 0.16, 0.09), 0.015), m["armor"], at=(-0.26, by + 0.27, z + 0.04))
    s.add(box(0.12, 0.09, 0.012), m["dark"], at=(-0.26, by + 0.27, z + 0.088))
    s.add(sphere(0.06, 2), m["steel"], at=(0.26, by + 0.28, z + 0.02))
    s.add(post(16, 0.13, 0.13, z - 0.005, z + 0.03), m["armor_round"], at=(0, by + 0.15, 0))
    bolts(s, m["steel_flat"], [(x, by + 0.15 + y, z + 0.03) for _, x, y in around(0.11, 60, 30)], r=0.01)

    # ---------------------------------------------------------------- Heck: Stellstreben, Luken über den Düsen
    for r, y1, dx, dz, mirror in ENGINES:
        ya = 4.4 + 0.6 * (y1 - 4.4)
        for a, c, d in around(1.0, 45, 22):
            s.add(segment((dx + 1.1 * r * c, ya, dz + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, dz + 0.93 * r * d), 0.016, 6), m["steel"],
                  mirror=mirror)
    hatch(s, m, (0.95, 4.36, 0.8), 0.24, 0.3, mirror=True)
    vent(s, m, m["dark"], (1.3, 4.35, 0.3), 0.5, 0.36, slats=2, rot=(0, 90, 0), mirror=True)

    # ---------------------------------------------------------------- Schürzen: Klammern mit Schrauben
    for y in (-1.4, -0.2, 1.0, 2.2):
        x = 1.95 + 0.05 * (y + 2.0) / 5.0
        s.add(box(0.04, 0.07, 0.22), m["dark"], at=(x, y, -0.35), mirror=True)
        bolts(s, m["steel_flat"], [(x + 0.02, y, -0.35 + dz) for dz in (-0.05, 0.05)], "x", r=0.01, mirror=True)

    # ---------------------------------------------------------------- Beschriftung: Kennnummer, Wegelinien, Warnbänder
    # Die Nummer steht vor dem Wappen, lesbar von hinten oben; Wegelinien führen an den Kühlblöcken entlang, am
    # Steigeisen unterbrochen von einem Warnband; ein zweites Warnband liegt über den Düsen
    number(s, m["paint"], "04", (0, -1.85, TOP), 0.36, (0, 0, 180))
    for y0, y1 in ((-1.9, 0.0), (0.35, 1.25)):
        s.add(box(0.025, y1 - y0, 0.006), m["paint"], at=(0.745, (y0 + y1) / 2, TOP + 0.002), mirror=True)
    stripes(s, m, (0.745, 0.175, TOP), 0.06, 0.3, mirror=True)
    stripes(s, m, (0, 4.5, 0.8), 0.07, 1.2, rot=(0, 0, 90))


def build():
    s = Ship("pyre")
    m = vf.materials(s.name, "battleship")

    # ---------------------------------------------------------------- Rumpf, Rückendeck, Zierlinie
    s.add(loft([sec(y, keel(w, h)) for y, w, h in HULL]), m["hull"])
    trim_line(s, m, [(w / 2 + 0.01, y, h / 2 + 0.005) for y, w, h in HULL])
    # (der Rumpf bleibt scharf: Auf seiner Oberkante sitzt die Zierlinie, die übrigen Kanten liegen in Anbauten)
    s.add(chamfer(loft([sec(-3.0, octagon(1.4, 0.5), dz=DECK - 0.35), sec(-2.3, octagon(2.2, 0.7), dz=DECK - 0.25),
                        sec(3.6, octagon(2.3, 0.7), dz=DECK - 0.25), sec(4.2, octagon(1.6, 0.5), dz=DECK - 0.3)]), 0.04), m["hull"])

    # ---------------------------------------------------------------- Plasmawerfer als Bug
    # Gehäuse in Glutorange mit Fensterband, Kühlmantel, Emitterkegel
    s.add(chamfer(loft([sec(-2.6, octagon(2.0, 1.5, 0.35), dz=PZ), sec(-3.4, octagon(2.0, 1.5, 0.35), dz=PZ),
                        sec(-3.9, octagon(1.6, 1.25, 0.35), dz=PZ)]), 0.05), m["accent"])
    # (das Fensterband liegt auf der oberen Schrägfläche: An der Flanke steckt das Gehäuse im Rumpf)
    s.add(box(0.04, 0.32, 0.09), m["window"], at=(0.802, -3.21, PZ + 0.638), rot=(0, -53.13, 0), mirror=True)
    s.add(tube(ROUND, 0.7, 0.62, -4.4, -3.75, dz=PZ), m["dark_round"])
    s.add(tube(24, 0.32, 0.14, -4.4, -4.95, dz=PZ), m["steel"])
    # Hitzeschild: große Scheibe vor dem Kühlmantel, ohne Glanz (eine glänzende Stirnfläche würde in `hero` weiß)
    s.add(revolve([(0.0, -4.52), (0.88, -4.52), (0.95, -4.46), (0.95, -4.4), (0.8, -4.33), (0.0, -4.33)], ROUND), m["armor_round"], at=(0, 0, PZ))
    # Drei Klauen halten die Glutkugel (von der Seite zwei Klauen mit der Kugel dazwischen, von oben die Kugel um die
    # obere Klaue)
    for angle in (0, 120, 240):
        g = claw(m)
        s.place(g, at=(0, 0, PZ), rot=(0, angle, 0))
        g.bm.free()
    s.add(sphere(0.4, 3), m["glow"], at=(0, -5.3, PZ))

    # ---------------------------------------------------------------- Kühlkörper an den Flanken mit Glutschlitzen (Lichter ohne Glühen)
    s.add(chamfer(plate([(1.55, -1.9), (2.05, -1.5), (2.05, 2.3), (1.55, 2.7)], 0.6, "xy"), 0.04), m["dark"], at=(0, 0, 0.15), mirror=True)
    for y in SLOTS:
        s.add(box(0.06, 0.18, 0.5), m["signal"], at=(2.06, y, 0.15), rot=(25, 0, 0), mirror=True)

    # ---------------------------------------------------------------- Rückendeck: zwei Reihen Kühlblöcke mit Glutkante, Wappen
    for y in BLOCKS:
        s.add(chamfer(wedge(0.42, 0.75, 0.55, front=0.5), 0.03), m["dark"], at=(1.0, y, TOP + 0.2), mirror=True)
        s.add(box(0.3, 0.5, 0.06), m["signal"], at=(1.0, y + 0.1, TOP + 0.49), mirror=True)
    badge(s, m, (0, -0.35, TOP - 0.05), PLATE, FLAME, dy=-0.04)

    # ---------------------------------------------------------------- Brennstoff: Satteltanks und Zuleitungen zum Werfer
    tx, ty, tz = TANK
    s.add(revolve([(0.0, -0.7), (0.17, -0.67), (0.26, -0.6), (0.3, -0.47), (0.3, 0.47), (0.26, 0.6), (0.17, 0.67), (0.0, 0.7)], ROUND),
          m["armor_round"], at=TANK, mirror=True)
    for dy in (-0.25, 0.25):
        s.add(tube(ROUND, 0.33, 0.33, ty + dy - 0.05, ty + dy + 0.05, dx=tx, dz=tz), m["dark_round"], mirror=True)
    # Die Zuleitung liegt auf der Schulter zwischen Rückendeck und Zierlinie und tritt seitlich in das Werfergehäuse
    line = [(1.36, 1.35, 0.87), (1.36, -1.95, 0.835), (1.18, -2.6, 0.79), (0.9, -3.0, 0.74)]
    s.add(sweep(line, ngon(16, 0.1)), m["dark_round"], mirror=True)
    # Ventil vor dem Tank, Rohrschelle am Rückendeck, Anschlussflansch am Werfer
    s.add(tube(24, 0.15, 0.15, 1.1, 1.26, dx=1.36, dz=0.87), m["armor_round"], mirror=True)
    s.add(chamfer(box(0.38, 0.12, 0.24), 0.02), m["armor"], at=(1.31, -0.4, 0.88), mirror=True)
    a, b = Vector(line[2]), Vector(line[3])
    s.add(segment(a.lerp(b, 0.55), a.lerp(b, 0.75), 0.14, 24), m["armor_round"], mirror=True)

    # ---------------------------------------------------------------- Panzerschürzen in Glutorange, darunter eine zweite Lage
    s.add(chamfer(plate(SKIRT, 0.2, "xy"), 0.03), m["accent"], at=(0, 0, -0.35), mirror=True)
    s.add(plate(grow(SKIRT, -0.14), 0.06, "xy"), m["armor"], at=(0, 0, -0.47), mirror=True)

    # ---------------------------------------------------------------- RCS an den Rumpfecken
    for y, along in ((-2.25, -1), (3.65, 1)):
        hw, top = hull_at(y)
        rcs(s, m, (hw - 0.06, y, top + 0.03), along)

    # ---------------------------------------------------------------- Brücke achtern: drei Keile, Brückenflügel, Mast
    z, by = TOP, 2.75
    for w, l, h, edge in ((1.8, 1.5, 0.55, 0.04), (1.35, 1.1, 0.45, 0.04), (0.95, 0.8, 0.4, 0.03)):
        s.add(chamfer(wedge(w, l, h), edge), m["hull"], at=(0, by, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, by - 0.4 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.49, by + 0.05, z - 0.17), mirror=True)
    s.add(chamfer(box(0.62, 0.28, 0.16), 0.015), m["hull"], at=(0.7, by - 0.03, z - 0.335), mirror=True)
    s.add(post(16, 0.09, 0.06, z, z + 0.85), m["hull_round"], at=(0, by + 0.15, 0))
    s.add(box(0.86, 0.07, 0.07), m["hull"], at=(0, by + 0.15, z + 0.58))
    s.add(box(0.56, 0.05, 0.2), m["armor"], at=(0, by + 0.06, z + 0.32), rot=(-15, 0, 0))
    s.add(sphere(0.08, 2), m["glow"], at=(0, by + 0.15, z + 0.89))

    # ---------------------------------------------------------------- Achterdeck: Maschinenlüfter auf der Schräge hinter der Brücke
    # (das Rückendeck fällt ab y 3,6 um 14°, bei y 3,88 liegt es 0,07 unter TOP)
    vent(s, m, m["hull"], (0, 3.88, TOP - 0.07), 0.9, 0.34, slats=2, rot=(-14, 0, 0))

    # ---------------------------------------------------------------- Kiel und drei schwere Düsen
    s.add(chamfer(plate([(-2.6, -0.4), (3.6, -0.5), (3.3, -1.0), (-1.6, -0.9)], 0.24, "yz"), 0.04), m["dark"])
    s.add(chamfer(box(2.6, 0.6, 1.2), 0.05), m["dark"], at=(0, 4.35, 0.2))
    for r, y1, dx, dz, mirror in ENGINES:
        engine(s, m, r, 4.4, y1, dx=dx, dz=dz, n=ROUND, mirror=mirror)
        engine_details(s, m, r, 4.4, y1, dx=dx, dz=dz, n=ROUND, mirror=mirror)
    details(s, m, z, by)
    fine(s, m, z, by)
    return s.finish()
