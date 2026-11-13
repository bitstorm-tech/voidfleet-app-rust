"""Iron Dreadnought (Schlachtschiff), Referenzschiff der Stilregeln (docs/stilregeln.md) für Kleinteile und
Detailzonen, in der Fassung für die Nahaufnahme: gefaste Primär- und Sekundärformen, glatte runde Teile, Kleinteile und
Feinteile.

Primärformen: breiter, kantiger Keilrumpf mit Dorsaldeck und zwei Lagen Panzerschürzen, drei große Zwillingstürme in
Stahlblau (vorn, überhöht, seitlich), zwei schwere Belagerungskanonen am Bug mit magenta Mündung, ein Aufbau aus drei
Blöcken mit Fensterband, Kasematten mit Lichtband, Schildemitter am Deckrand, Kiel und vier Heckdüsen.
Detailzonen mit Zweck, dazwischen ruhige Flächen (Hauptdeck vor den Seitentürmen, Dorsaldeck zwischen den Türmen,
Schürzen, Flächen des Aufbaus):
- Bug: Bugpanzer-Kappe; Kanonen mit Blendenbalg, Kühlmantel, Laufschelle und zweiteiliger Mündungsbremse
- Türme: Barbettenring, Entfernungsmesser-Ohren, Turmluke, Blendenbälge, Mündungsringe (`parts.turret_details`)
- Flanke: zweite Plattenlage auf der Schürze, Kabelkanal mit Anschlusskasten, Kasematte mit Deckel, Luke und Lüfter,
  RCS-Blöcke an den Rumpfecken
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte und Querträger
- Achterdeck und Heck: Maschinenlüfter, Lagerkisten, Gehäusesegment und Düsenkranz je Düse (`parts.engine_details`)
- Unterseite: Kinn mit Hangartor nach vorn, Spanten am V-Boden, Plattenlage unter der unteren Schürze
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`), seit der dreckige Look keine Konturringe mehr zeichnet: Fensterreihen aus Einzelfenstern,
Flak-Türmchen, Nadel- und Schüsselantennen, Geländer, Leiter, Rohrleitungen, Kühlrippen an den Düsen, Positionslichter.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen, wie groß das Schiff ist, und wo man es betritt,
wartet und verschraubt), in den Zonen von oben und auf den ruhigen Flächen dazwischen: Schraubenkranz im Barbettenring,
Handläufe und Periskop auf den Turmdächern, Steigeisen an der hohen Barbette; Niedergang vom Hauptdeck aufs Dorsaldeck
mit Reling; Luken mit Rahmen, Scharnieren und Griff auf Hauptdeck, Schürze und Kasematte; Munitionskästen an der Flak,
Laufsteg aus Gitterrost am Maschinenlüfter, Poller; Fenstersimse, Konsolen unter den Brückenflügeln, Dachaggregat und
Radom, Steigeisen am Mast, Schotten am Aufbau; Klammern an der Schürze, Schellen am Kabelkanal, Halter über dem
Emitterband; Rücklaufzylinder und verschraubte Laufschelle an den Belagerungskanonen, Stellstreben an den Düsenglocken.
Dazu Beschriftung als flache Geometrie: der Schwenkkreis um den überhöhten Turm, Warnbänder am Niedergang und am
Maschinenlüfter, die Kennnummer an der Schürze und an der Rückwand des Aufbaus.
"""

import math

import palette
import vf
from parts import (beam, bellows, bolts, engine, engine_details, flak, grating, hatch, number, rail, rcs, stripes, turret, turret_details, vent,
                   windows)
from vf import (Ship, box, chamfer, dish, grow, hoop, keel, loft, octagon, pipe, plate, post, rbox, rect, sec, segment, sphere, sweep, tube,
                wedge)

# Rumpf: Querschnitte (y, Breite, Höhe) des Keilprofils, eingefügt um HULL_Z angehoben
HULL = ((-4.0, 0.6, 0.5), (-2.8, 1.8, 0.9), (-1.2, 3.6, 1.3), (0.6, 4.4, 1.4), (2.4, 3.8, 1.3), (3.4, 2.6, 1.0))
HULL_Z = 0.1
SKIRT = [(0.6, -3.0), (2.3, -1.0), (2.6, 1.0), (2.2, 2.9), (0.6, 2.9)]
ROUND = 40  # Segmente der großen runden Teile (Barbette, Belagerungskanonen; die Türme rundet `parts.turret`)
JET = 24  # Segmente der vier Heckdüsen (Radius 0,32)


def hull_at(y):
    """Rumpf bei y: halbe Breite, Höhe des Decks, Unterkante der Flanke, Kiel."""
    for (y0, w0, h0), (y1, w1, h1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            w, h = w0 + t * (w1 - w0), h0 + t * (h1 - h0)
            return w / 2, HULL_Z + h / 2, HULL_Z + 0.1 * h, HULL_Z - h / 2
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def slopes(y):
    """Neigungen des Rumpfs bei y in Grad, für `rot`: das Hauptdeck um x (es ist je Abschnitt eben und steigt zur
    Schiffsmitte), die Flanke um z (Richtung ihrer Außenseite, Steuerbord)."""
    for (y0, w0, h0), (y1, w1, h1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            return math.degrees(math.atan((h1 - h0) / 2 / (y1 - y0))), math.degrees(math.atan2((w0 - w1) / 2, y1 - y0))
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def wall_x(y):
    """Wand des Dorsaldecks (Steuerbord) bei y; sie läuft nach achtern um WALL_YAW auseinander."""
    return 0.9 + 0.2 * (y + 2.2) / 4.8


WALL_YAW = -math.degrees(math.atan(0.2 / 4.8))
TOP = 1.265  # Oberseite der Deckplatte auf dem Dorsaldeck
TURRETS = (((0, -1.85, TOP), 0.5, 1.35), ((0, -0.3, 1.6), 0.5, 1.35), ((1.45, 0.95, 0.86), 0.46, 1.2))  # Mitte der Barbette, Radius, Rohrlänge


def bottom_z(x, y):
    """Höhe des V-Bodens bei (x, y), x ≥ 0."""
    hw, _, low, keel_z = hull_at(y)
    return keel_z + (low - keel_z) * x / hw


def details(s, m, z):
    """Kleinteile (Modulkommentar); `z` ist das Dach des Aufbaus."""
    roof1, roof2 = 1.765, 2.215  # Dächer der beiden unteren Blöcke des Aufbaus

    # ---------------------------------------------------------------- Fensterreihen: Aufbau und Wand des Dorsaldecks
    row = [1.86 + 0.12 * k for k in range(8)]
    windows(s, m, [(0.85, y, 1.6) for y in row])
    windows(s, m, [(0.85, y, 1.42) for y in row[1::2]])
    windows(s, m, [(-0.6 + 0.2 * k, 2.75, 1.6) for k in range(7)], out="y")
    windows(s, m, [(0.625, 1.96 + 0.12 * k, 2.03) for k in range(6)])
    windows(s, m, [(-0.4 + 0.2 * k, 2.6, 2.03) for k in range(5)], out="y")
    windows(s, m, [(0.9 + 0.2 * (y + 2.2) / 4.8, y, 0.99) for y in [-0.3 + 0.12 * k for k in range(6)] + [1.6 + 0.12 * k for k in range(8)]])

    # ---------------------------------------------------------------- Flak: Hauptdeck und Dach des Aufbaus
    for (x, y), yaw in (((1.6, -0.85), 30), ((1.85, -0.1), 40), ((1.85, 1.75), 140)):
        flak(s, m, (x, y, hull_at(y)[1]), yaw)
    flak(s, m, (0.74, 2.05, roof1), 90)

    # ---------------------------------------------------------------- Antennen
    s.add(post(6, 0.014, 0.006, z + 0.66, z + 1.2), m["dark_round"], at=(0.4, 2.15, 0), mirror=True)
    s.add(segment((0.55, 2.55, roof2 - 0.02), (0.62, 3.05, 2.95), 0.012, 6), m["dark_round"], mirror=True)
    s.add(segment((0.16, -3.9, 0.3), (0.16, -4.5, 0.34), 0.014, 6), m["steel"], mirror=True)
    # Schüssel backbord, Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, roof1 - 0.02, roof1 + 0.14), m["dark_round"], at=(-0.74, 2.5, 0))
    s.add(dish(0.13, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.74, 2.5, roof1 + 0.13), rot=(-40, 0, 150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(0.74, 2.5, roof1 + 0.06))
    s.add(post(6, 0.012, 0.006, roof1 + 0.1, roof1 + 0.5), m["dark_round"], at=(0.74, 2.56, 0))

    # ---------------------------------------------------------------- Geländer und Leiter am Aufbau
    # Geländer 0,045 von der Dachkante: Die Blöcke sind gefast, an der Kante stünden die Pfosten in der Luft
    rail(s, m, (0.805, 2.27, roof1), (0.805, 2.705, roof1), 3)
    rail(s, m, (-0.805, 2.705, roof1), (0.805, 2.705, roof1), 5, mirror=False)
    rail(s, m, (0.585, 2.1, roof2), (0.585, 2.56, roof2), 3)
    rail(s, m, (0.47, 1.79, z - 0.255), (0.99, 1.79, z - 0.255), 3)
    rail(s, m, (0.99, 1.79, z - 0.255), (0.99, 2.05, z - 0.255), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.5), m["dark"], at=(0.75 + dx, 2.757, 1.515))
    for k in range(6):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.75, 2.757, 1.33 + 0.08 * k))

    # ---------------------------------------------------------------- Rohrleitungen und Kisten auf dem Dorsaldeck
    for x, (ex, ey) in ((0.62, (0.35, 0.05)), (0.7, (0.42, 0.0))):
        s.add(pipe([(x, 1.3, 1.29), (x, 0.5, 1.29), (ex, ey, 1.29)], 0.028, 6), m["steel"], mirror=True)
    s.add(chamfer(box(0.22, 0.2, 0.14), 0.015), m["armor"], at=(0.3, 1.12, 1.335))
    s.add(chamfer(box(0.2, 0.22, 0.16), 0.015), m["dark"], at=(-0.34, 1.11, 1.345))
    s.add(chamfer(box(0.14, 0.14, 0.1), 0.015), m["armor"], at=(-0.34, 1.1, 1.475))

    # ---------------------------------------------------------------- Kühlrippen an den Düsen
    for dz in (0.45, -0.25):
        for t in (0.37, 0.42, 0.47):
            y = 3.3 + 0.7 * t
            s.add(tube(JET, 0.365, 0.365, y, y + 0.025, 0.6, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter
    s.add(sphere(0.03, 1), m["glow"], at=(1.0, 1.92, z - 0.225), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(2.52, 1.0, 0.43), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(0, -4.05, 0.41))


def gangway(s, m, y, x0, x1, z0, z1, width=0.1):
    """Niedergang quer zum Schiff (Steuerbord, gespiegelt) von (x0, z0) auf dem Hauptdeck hinauf nach (x1, z1) an der
    Kante des Dorsaldecks; wie `parts.stairs`, das nur längs läuft: massive Stufen von etwa 0,026 Höhe, zwei Wangen und
    Handläufe auf drei Pfosten."""
    n = max(2, round((z1 - z0) / 0.026))
    rise, run = (z1 - z0) / n, (x1 - x0) / n
    for k in range(n):
        s.add(box(abs(run), width, rise * (k + 1) + 0.01), m["dark"], at=(x0 + run * (k + 0.5), y, z0 + rise * (k + 1) / 2 - 0.005), mirror=True)
    for dy in (-width / 2, width / 2):
        s.add(plate([(x0, z0 - 0.015), (x1, z1 - 0.015), (x1, z1 + 0.035), (x0, z0 + 0.035)], 0.016, "xz"), m["armor"], at=(0, y + dy, 0),
              mirror=True)
        s.add(plate([(x0, z0 + 0.094), (x1, z1 + 0.094), (x1, z1 + 0.106), (x0, z0 + 0.106)], 0.012, "xz"), m["dark"], at=(0, y + dy, 0),
              mirror=True)
        for t in (0.0, 0.5, 1.0):
            s.add(box(0.012, 0.012, 0.1), m["dark"], at=(x0 + t * (x1 - x0), y + dy, z0 + t * (z1 - z0) + 0.055), mirror=True)


def door(s, m, at, rot=(0, 0, 0)):
    """Schott in einer Wand, die nach +y schaut (`rot` dreht es), `at` die Mitte auf der Wandfläche: dunkler Rahmen,
    gefastes Türblatt, Handrad (sechseckig und eckig schattiert, sonst hielte es die Verwitterung für eine Mündung) und
    zwei Scharniere."""
    g = Ship("door")
    g.add(box(0.17, 0.02, 0.25), m["dark"], at=(0, 0.004, 0))
    g.add(chamfer(box(0.13, 0.03, 0.21), 0.008), m["armor"], at=(0, 0.01, 0))
    g.add(tube(6, 0.024, 0.024, 0.0, 0.014), m["steel_flat"], at=(0, 0.022, 0.015))
    for dz in (-0.07, 0.07):
        g.add(box(0.02, 0.012, 0.03), m["steel_flat"], at=(-0.07, 0.022, dz))
    s.place(g, at=at, rot=rot)
    g.bm.free()


def fine(s, m, z):
    """Feinteile und Beschriftung (Modulkommentar); `z` ist das Dach des Aufbaus."""
    cos, sin = (lambda a: math.cos(math.radians(a))), (lambda a: math.sin(math.radians(a)))

    # ---------------------------------------------------------------- Türme: Schraubenkranz, Dach, Steigeisen, Gefahrenkreis
    # Schrauben im Barbettenring jedes Turms, auf dem Dach zwei Handläufe (sie enden vor den Mündungen des Turms
    # darüber) und ein Periskop, die mit dem Turm drehen; an der hohen Barbette des überhöhten Turms Steigeisen nach
    # achtern und auf dem Deck um ihn der Schwenkkreis der Rohre als Linie
    for (x, y, tz), r, _ in TURRETS:
        bolts(s, m["steel"], [(x + 1.125 * r * cos(a), y + 1.125 * r * sin(a), tz + 0.05) for a in (11.25 + 22.5 * k for k in range(16))], mirror=x > 0)

        def roof(dy):
            return tz + 0.14 + 0.86 * r + 0.02 * (dy + 0.5 * r) / 1.1

        with s.turret((x, y, tz), mirror=x > 0):
            for sx in (-0.66 * r, 0.66 * r):
                rail(s, m, (x + sx, y - 0.4 * r, roof(-0.4 * r)), (x + sx, y + 0.25 * r, roof(0.25 * r)), 3, mirror=x > 0, h=0.05)
            s.add(box(0.08, 0.06, 0.05), m["dark"], at=(x + 0.3 * r, y - 0.28 * r, roof(-0.28 * r) + 0.018), mirror=x > 0)
            s.add(box(0.05, 0.012, 0.02), m["steel_flat"], at=(x + 0.3 * r, y - 0.28 * r - 0.03, roof(-0.28 * r) + 0.026), mirror=x > 0)
    for k in range(4):
        h = 0.035 + 0.06 * k
        r = 0.56 - 0.02 * h / 0.335 + 0.008
        s.add(box(0.03, 0.07, 0.012), m["steel_flat"], at=(r * cos(75), -0.3 + r * sin(75), TOP + h), rot=(0, 0, 75), mirror=True)
    s.add(hoop(ROUND, 0.68, 0.705, 0.008), m["paint"], at=(0, -0.3, TOP + 0.002))

    # ---------------------------------------------------------------- Niedergang, Reling, Luken auf dem Dorsaldeck
    # Vom Hauptdeck aufs Dorsaldeck zwischen Emitterband und Fensterreihe, oben in der Lücke der Reling mit Warnband;
    # die Deckluke hinter dem überhöhten Turm mit Schrauben, Scharnieren und Griff, die Kiste daneben verzurrt
    gangway(s, m, -0.42, 1.5, 0.93, hull_at(-0.42)[1], TOP - 0.004)
    stripes(s, m, (0.866, -0.42, TOP), 0.06, 0.22, mirror=True)
    for y0, y1, posts in ((-1.25, -0.52, 4), (0.3, 1.2, 5)):
        rail(s, m, (0.815 + 0.2 * (y0 + 2.1) / 4.5, y0, TOP), (0.815 + 0.2 * (y1 + 2.1) / 4.5, y1, TOP), posts)
    bolts(s, m["steel_flat"], [(sx * 0.14, 0.78 + sy * 0.11, 1.3) for sx in (-1, 1) for sy in (-1, 1)], r=0.01)
    s.add(box(0.012, 0.1, 0.012), m["steel_flat"], at=(0.09, 0.78, 1.304))
    for dy in (-0.08, 0.08):
        s.add(tube(8, 0.014, 0.014, -0.035, 0.035), m["steel_flat"], at=(-0.2, 0.78 + dy, 1.292))
    s.add(box(0.03, 0.208, 0.146), m["dark"], at=(0.3, 1.12, 1.336))

    # ---------------------------------------------------------------- Hauptdeck: Luke, Bereitschaftsmunition, Laufsteg
    # Wartungsluke vor dem Niedergang, je ein Munitionskasten hinter jeder Flak; achtern ein Laufsteg aus Gitterrost
    # außen am Maschinenlüfter, dessen Rahmen verschraubt ist, innen ein Warnband; Poller an den Heckecken
    hatch(s, m, (1.25, -1.05, hull_at(-1.05)[1]), 0.3, 0.36, rot=(slopes(-1.05)[0], 0, 0), mirror=True)
    for x, y in ((1.63, -0.6), (1.87, 0.18), (1.85, 1.5)):
        deck = hull_at(y)[1]
        s.add(chamfer(box(0.12, 0.16, 0.09), 0.012), m["dark"], at=(x, y, deck + 0.035), mirror=True)
        s.add(box(0.1, 0.14, 0.012), m["armor"], at=(x, y, deck + 0.082), mirror=True)
        s.add(box(0.012, 0.06, 0.012), m["steel_flat"], at=(x, y, deck + 0.092), mirror=True)
    g = Ship("walk")
    grating(g, m, 1.71, 1.81, -0.24, 0.24, 0.0)
    s.place(g, at=(0, 2.15, hull_at(2.15)[1]), rot=(slopes(2.15)[0], 0, 0))
    g.bm.free()
    deck = hull_at(2.2)[1]
    bolts(s, m["steel"], [(1.45 + sx * 0.215, 2.2 + sy * 0.325, deck + 0.05) for sx in (-1, 1) for sy in (-1, 1)], mirror=True)
    stripes(s, m, (1.14, 2.2, deck), 0.06, 0.6, rot=(slopes(2.2)[0], 0, 0), mirror=True)
    deck = hull_at(2.78)[1]
    s.add(box(0.268, 0.03, 0.206), m["dark"], at=(1.05, 2.78, deck + 0.081), mirror=True)
    deck = hull_at(3.2)[1]
    s.add(post(10, 0.035, 0.042, deck - 0.01, deck + 0.09), m["steel"], at=(1.1, 3.2, 0), mirror=True)

    # ---------------------------------------------------------------- Aufbau: Simse, Konsolen, Dach, Mast, Schott
    # Fenstersimse unter den oberen Reihen, Konsolen unter den Brückenflügeln, Aggregat und Radom auf dem Brückendach,
    # Steigeisen am Mast; je ein Schott mit Handrad an der Rückwand und vorn zum Dorsaldeck, an der Rückwand die
    # Kennnummer, lesbar von hinten oben
    s.add(box(0.03, 0.9, 0.014), m["dark"], at=(0.858, 2.27, 1.564), mirror=True)
    s.add(box(1.34, 0.03, 0.014), m["dark"], at=(0, 2.758, 1.564))
    s.add(box(0.03, 0.66, 0.014), m["dark"], at=(0.633, 2.25, 1.994), mirror=True)
    s.add(box(0.94, 0.03, 0.014), m["dark"], at=(0, 2.608, 1.994))
    for y0, y1 in ((-0.34, 0.34), (1.56, 2.48)):
        s.add(beam((wall_x(y0) + 0.008, y0, 0.954), (wall_x(y1) + 0.008, y1, 0.954), 0.03, 0.014), m["dark"], mirror=True)
    for y in (1.84, 2.02):
        s.add(plate([(0.6, z - 0.41), (0.93, z - 0.41), (0.6, z - 0.64)], 0.03, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    s.add(chamfer(box(0.2, 0.15, 0.09), 0.015), m["armor"], at=(-0.26, 2.02, z + 0.035))
    s.add(box(0.13, 0.09, 0.012), m["dark"], at=(-0.26, 2.02, z + 0.083))
    s.add(sphere(0.06, 2), m["steel"], at=(0.27, 2.27, z + 0.02))
    for k in range(8):
        h = 0.08 + 0.065 * k
        s.add(box(0.07, 0.03, 0.012), m["steel_flat"], at=(0, 2.15 + 0.098 - 0.03 * h / 0.95, z + h))
    door(s, m, (-0.4, 2.75, 1.395))
    door(s, m, (-0.02, 1.25, 1.395), (0, 0, 180))
    number(s, m["paint"], "17", (0.25, 2.75, 1.4), 0.2, (90, 0, 180))

    # ---------------------------------------------------------------- Schürze und Flanke
    # Klammern mit Schrauben an den Außenkanten beider Schürzen, zwischen den vorderen der oberen die Kennnummer, auf
    # jeder Seite lesbar; auf der Schürze eine Luke vorn, zwei Poller und eine Luke achtern; Schellen um den Kabelkanal,
    # Schrauben an seinen Anschlusskästen
    for (ax, ay), (bx, by), ys in ((SKIRT[1], SKIRT[2], (-0.78, -0.02, 0.7)), (SKIRT[2], SKIRT[3], (1.6, 2.35))):
        yaw = math.degrees(math.atan2(ax - bx, by - ay))
        for y in ys:
            x = ax + (bx - ax) * (y - ay) / (by - ay)
            s.add(box(0.03, 0.07, 0.2), m["dark"], at=(x + 0.005 * cos(yaw), y + 0.005 * sin(yaw), 0.3), rot=(0, 0, yaw), mirror=True)
            bolts(s, m["steel"], [(x + 0.02 * cos(yaw), y + 0.02 * sin(yaw), 0.3 + dz) for dz in (-0.06, 0.06)], (0, 90, yaw), mirror=True)
    yaw = math.degrees(math.atan2(SKIRT[1][0] - SKIRT[2][0], SKIRT[2][1] - SKIRT[1][1]))
    number(s, m["paint"], "17", (2.39, -0.4, 0.3), 0.16, (90, 0, 90 + yaw))
    number(s, m["paint"], "17", (-2.39, -0.4, 0.3), 0.16, (90, 0, -90 - yaw))
    for y in (-0.1, 0.6, 1.3):
        s.add(box(0.03, 0.07, 0.18), m["dark"], at=(2.505, y, -0.2), mirror=True)
        bolts(s, m["steel"], [(2.52, y, -0.2 + dz) for dz in (-0.05, 0.05)], "x", mirror=True)
    hatch(s, m, (2.1, -0.5, 0.48), 0.2, 0.26, mirror=True)
    hatch(s, m, (2.06, 2.45, 0.48), 0.18, 0.24, mirror=True)
    for y in (1.72, 1.98):
        s.add(post(10, 0.03, 0.036, 0.47, 0.56), m["steel"], at=(2.17, y, 0), mirror=True)
    for y in (-1.6, -0.5, 1.7, 2.1):
        hw, deck, _, _ = hull_at(y)
        s.add(box(0.1, 0.035, 0.14), m["armor"], at=(hw + 0.045, y, deck - 0.18), rot=(0, 0, slopes(y)[1]), mirror=True)
    for y in (-0.9, 2.7):
        hw, deck, _, _ = hull_at(y)
        bolts(s, m["steel"], [(hw + 0.13, y + dy, deck - 0.18 + dz) for dy in (-0.1, 0.1) for dz in (-0.07, 0.07)], "x", r=0.01, mirror=True)

    # ---------------------------------------------------------------- Kasematte: Luke und Schrauben im Deckel
    hatch(s, m, (2.32, 0.2, 0.65), 0.2, 0.26, mirror=True)
    bolts(s, m["steel"], [(2.45, -0.18 + 0.26 * k, 0.65) for k in range(7)], mirror=True)

    # ---------------------------------------------------------------- Schildemitter: Halter über dem Leuchtband
    for y in (-1.7, -1.3, -0.9):
        s.add(box(0.09, 0.035, 0.135), m["dark"], at=(wall_x(y) + 0.035, y, 1.02), rot=(0, 0, WALL_YAW), mirror=True)

    # ---------------------------------------------------------------- Bug: Kappe verschraubt, Rücklaufzylinder
    # Schrauben an der Hinterkante der Bugkappe; je Kanone zwei Rücklaufzylinder auf dem Kühlmantel, deren
    # Kolbenstangen an einer Lasche der Laufschelle enden, die Schelle selbst verschraubt
    cap = 0.1 + (0.6 + 0.24 * (-3.43 + 4.06) / 0.71) / 2
    bolts(s, m["steel"], [(x, -3.43, cap) for x in (0.07, 0.21)], (math.degrees(math.atan(0.12 / 0.71)), 0, 0), mirror=True)
    cx, cz = 0.52, 0.36
    for a in (45, 135):
        ox, oz = cx + 0.325 * cos(a), cz + 0.325 * sin(a)
        s.add(tube(12, 0.04, 0.04, -3.52, -3.07, ox, oz), m["dark"], mirror=True)
        s.add(tube(6, 0.016, 0.016, -3.97, -3.5, ox, oz), m["steel"], mirror=True)
        s.add(box(0.05, 0.07, 0.09), m["armor"], at=(cx + 0.305 * cos(a), -3.96, cz + 0.305 * sin(a)), rot=(0, 90 - a, 0), mirror=True)
    for a in range(22, 360, 45):
        bolts(s, m["steel"], [(cx + 0.27 * cos(a), -3.96, cz + 0.27 * sin(a))], (0, 90 - a, 0), mirror=True)

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    for dz in (0.45, -0.25):
        for a in range(0, 360, 60):
            s.add(segment((0.6 + 0.352 * cos(a), 3.72, dz + 0.352 * sin(a)), (0.6 + 0.298 * cos(a), 3.97, dz + 0.298 * sin(a)), 0.014, 6),
                  m["steel"], mirror=True)


def build():
    s = Ship("iron_dreadnought")
    m = vf.materials(s.name, "battleship")
    m["shield"] = vf.light(f"{s.name}_shield", palette.SHIELD)

    # ---------------------------------------------------------------- Rumpf und Panzerlagen
    s.add(chamfer(loft([sec(y, keel(w, h)) for y, w, h in HULL]), 0.05), m["hull"], at=(0, 0, HULL_Z))
    # Bugpanzer: Kappe um die Bugspitze
    s.add(chamfer(loft([sec(-4.06, keel(0.72, 0.6)), sec(-3.35, keel(1.36, 0.84))]), 0.04), m["armor"], at=(0, 0, HULL_Z))
    # Panzerschürzen in zwei Lagen, beide mit einer zweiten Platte (Materialwechsel, damit die Stufe als Ton liest)
    s.add(chamfer(plate(SKIRT, 0.24, "xy"), 0.03), m["armor"], at=(0, 0, 0.3), mirror=True)
    s.add(chamfer(plate(grow(SKIRT, -0.12), 0.07, "xy"), 0.02), m["hull"], at=(0, 0, 0.445), mirror=True)
    lower = [(0.8, -2.2), (2.5, -0.4), (2.5, 1.6), (0.8, 2.4)]
    s.add(chamfer(plate(lower, 0.22, "xy"), 0.03), m["hull"], at=(0, 0, -0.2), mirror=True)
    s.add(chamfer(plate(grow(lower, -0.14), 0.07, "xy"), 0.02), m["armor"], at=(0, 0, -0.33), mirror=True)
    # Dorsaldeck und Deckplatte
    s.add(chamfer(loft([sec(-3.0, octagon(0.6, 0.3)), sec(-2.2, octagon(1.8, 0.5)), sec(2.6, octagon(2.2, 0.55)), sec(3.2, octagon(1.4, 0.4))]), 0.04),
          m["hull"], at=(0, 0, 0.95))
    s.add(chamfer(plate([(-0.85, -2.1), (0.85, -2.1), (1.05, 2.4), (-1.05, 2.4)], 0.05, "xy"), 0.015), m["hull"], at=(0, 0, 1.24))
    s.add(chamfer(plate(octagon(0.42, 0.34, 0.4), 0.06, "xy"), 0.015), m["armor"], at=(0, 0.78, 1.27))

    # ---------------------------------------------------------------- Türme: vorn, überhöht auf Barbette, seitlich
    s.add(post(ROUND, 0.56, 0.54, TOP, 1.6), m["hull_round"], at=(0, -0.3, 0))
    for at, r, barrel in TURRETS:
        turret(s, m, at, r, barrel, mirror=at[0] > 0)
        turret_details(s, m, at, r, barrel, mirror=at[0] > 0)

    # ---------------------------------------------------------------- Aufbau: drei Blöcke, Brückenflügel, Mast
    z = 1.265
    for w, l, h, y in ((1.7, 1.5, 0.5, 2.0), (1.25, 1.1, 0.45, 2.05), (0.92, 0.75, 0.4, 2.0)):
        s.add(chamfer(wedge(w, l, h), 0.03), m["hull"], at=(0, y, z + h / 2))
        z += h
    s.add(box(0.8, 0.04, 0.09), m["window"], at=(0, 2.0 - 0.75 / 2 + 0.12, z - 0.17))
    s.add(box(0.04, 0.5, 0.09), m["window"], at=(0.465, 2.05, z - 0.17), mirror=True)
    # Brückenflügel: offene Nocken auf Höhe des Brückendecks, unter den seitlichen Fenstern
    s.add(box(0.65, 0.28, 0.16), m["hull"], at=(0.675, 1.92, z - 0.335), mirror=True)
    # Mast mit Querträger, Radarplatte und Licht
    s.add(post(16, 0.09, 0.06, z, z + 0.95), m["hull_round"], at=(0, 2.15, 0))
    s.add(box(0.86, 0.07, 0.07), m["hull"], at=(0, 2.15, z + 0.66))
    s.add(box(0.56, 0.05, 0.2), m["armor"], at=(0, 2.06, z + 0.36), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(0, 2.15, z + 0.99))

    # ---------------------------------------------------------------- Kasematten mit Lichtband, Deckel, Luke, Lüfter
    s.add(chamfer(box(0.7, 1.6, 0.5), 0.04), m["armor"], at=(2.1, 0.6, 0.35), mirror=True)
    s.add(chamfer(box(0.78, 1.68, 0.06), 0.02), m["hull"], at=(2.1, 0.6, 0.62), mirror=True)
    s.add(box(0.04, 1.2, 0.08), m["glow"], at=(2.46, 0.6, 0.42), mirror=True)
    s.add(box(0.36, 0.04, 0.28), m["hull"], at=(2.1, -0.21, 0.334), mirror=True)
    vent(s, m, m["hull"], (2.2, 0.95, 0.65), 0.36, 0.62, slats=2, mirror=True)

    # ---------------------------------------------------------------- Kabelkanäle an den Flanken
    for ys, box_y in (((-1.95, -1.2, -0.1), -0.9), ((1.3, 2.4, 2.7), 2.7)):
        pts = []
        for y in ys:
            hw, deck, _, _ = hull_at(y)
            pts.append((hw + 0.03, y, deck - 0.18))
        s.add(sweep(pts, rect(0.1, 0.1)), m["dark"], mirror=True)
        hw, deck, _, _ = hull_at(box_y)
        s.add(chamfer(box(0.18, 0.3, 0.22), 0.02), m["armor"], at=(hw + 0.04, box_y, deck - 0.18), mirror=True)

    # ---------------------------------------------------------------- Belagerungskanonen am Bug
    cx, cz = 0.52, 0.36
    s.add(rbox(0.62, 1.0, 0.62, 0.06, 1), m["dark"], at=(cx, -2.35, cz), mirror=True)
    s.add(bellows(ROUND, 0.34, -2.83, -3.05), m["armor_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.24, 0.21, -4.6, -2.7), m["steel"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.3, 0.3, -3.55, -3.03), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.27, 0.27, -4.02, -3.9), m["armor_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.2, 0.2, -4.8, -4.55), m["steel"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.31, 0.31, -4.9, -4.76), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.31, 0.31, -4.66, -4.5), m["dark_round"], at=(cx, 0, cz), mirror=True)
    s.add(tube(ROUND, 0.15, 0.15, -4.94, -4.88), m["shield"], at=(cx, 0, cz), mirror=True)

    # ---------------------------------------------------------------- Schildemitter (magenta)
    s.add(sweep([(wall_x(y) + 0.03, y, 1.02) for y in (-2.1, -0.5)], rect(0.08, 0.1)), m["shield"], mirror=True)
    for y in (-2.25, 0.05):
        s.add(post(24, 0.17, 0.15, 0.0, 0.12), m["dark_round"], at=(0.98, y, 1.06), mirror=True)
        s.add(sphere(0.13, 2), m["shield"], at=(0.98, y, 1.2), mirror=True)
    s.add(box(0.5, 0.06, 0.18), m["shield"], at=(0, -3.72, 0.5))

    # ---------------------------------------------------------------- Achterdeck: Maschinenlüfter, Lagerkisten
    vent(s, m, m["hull"], (0, 2.98, 1.17), 0.9, 0.34, slats=2, rot=(-7, 0, 0))
    vent(s, m, m["hull"], (1.45, 2.2, hull_at(2.2)[1]), 0.5, 0.72, mirror=True)
    s.add(chamfer(box(0.26, 0.32, 0.2), 0.02), m["armor"], at=(1.05, 2.78, hull_at(2.78)[1] + 0.08), mirror=True)

    # ---------------------------------------------------------------- RCS an den Rumpfecken
    hw, deck, _, _ = hull_at(-1.35)
    rcs(s, m, (hw - 0.06, -1.35, deck + 0.03), -1)
    hw, deck, _, _ = hull_at(3.1)
    rcs(s, m, (hw - 0.04, 3.1, deck + 0.03), 1)

    # ---------------------------------------------------------------- Unterseite: Kiel, Spanten, Hangartor
    s.add(chamfer(plate([(-2.0, 0.0), (2.8, 0.0), (2.9, -0.4), (-1.0, -0.35)], 0.12, "yz"), 0.03), m["dark"], at=(0, 0, -0.5))
    for y in (0.0, 0.55, 1.1, 1.65):
        x0, x1 = 0.04, 0.78
        z0, z1 = bottom_z(x0, y), bottom_z(x1, y)
        rib = [(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)]
        s.add(chamfer(plate(rib, 0.12, "xz"), 0.02), m["hull"], at=(0, y, 0), mirror=True)
    s.add(chamfer(wedge(1.0, 1.4, 0.42), 0.04), m["hull"], at=(0, -2.05, -0.4), rot=(180, 0, 0))
    s.add(box(0.34, 0.06, 0.26), m["dark"], at=(0.2, -2.75, -0.42), mirror=True)

    # ---------------------------------------------------------------- Vier Heckdüsen mit Gehäusesegment und Düsenkranz
    for dz in (0.45, -0.25):
        engine(s, m, 0.32, 3.3, 4.0, dx=0.6, dz=dz, n=JET, mirror=True)
        engine_details(s, m, 0.32, 3.3, 4.0, dx=0.6, dz=dz, n=JET, mirror=True)
    details(s, m, z)
    fine(s, m, z)
    return s.finish()
