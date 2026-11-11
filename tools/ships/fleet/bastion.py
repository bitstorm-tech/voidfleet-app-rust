"""Bastion (Flaggschiff von Admiral Osei, „die Mauer“), im Clean-Toon-Stil in der detailreichen Fassung
(docs/stilregeln.md).

Fliegende Festung: breiter Kastenrumpf auf einem gestuften Sockel mit Kiel, davor ein Keilbug, drei Terrassendecks mit
goldener Zierlinie und Panzerschürzen in Bernstein. Am Bug eine zurückgeneigte Schildwand im Bogen aus ebenen Facetten
(Rahmen mit Zinnen und goldener Oberkante, Bernstein-Schildfeld, Kennband) zwischen zwei hohen Tortürmen; diese und zwei
Ecktürme achtern tragen die vier Schildpylone mit Emitterkristall. Achtern der Bergfried als achteckiger Turm mit Krone,
umlaufendem Fensterband und Mast, davor die Wappenplatte mit Oseis Bernstein-Schild, am Heck drei schwere Düsen und
zwei kleine an den Deckecken.
Detailzonen mit Zweck (Schildtechnik, Panzerlagen, Wartungszugänge), dazwischen ruhige Flächen (Schildfeld, Rumpfflanken
mit Schürze und Fensterband, Deck um das Wappen, Flächen des Bergfrieds):
- Schildwand und Bug: Strebepfeiler unter den äußeren Zinnen an der Rückseite, zweite Panzerlage auf dem Sims davor
- Tortürme: Emitterbank mit drei Linsen vorn außen, Gurtring unter dem Kranz, Wartungstür zur Terrasse
- Pylone: Bernstein-Kranz, dunkler Fuß mit vier Kondensatorblöcken, Fokusring
- Terrassen: Energieleitung vom Eckturm zum Torturm mit Anschlusskästen und zwei Schellen
- Bergfried: zweiflügeliges Tor nach vorn, Lüfter auf dem Sockel, Mast mit Querträger und Radarplatte
- Achterdeck und Heck: Maschinenlüfter über den Düsen, RCS an den hinteren Rumpfecken, Gehäusesegment und Düsenkranz je
  Düse (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import math

from mathutils import Vector

import vf
from parts import badge, engine, engine_details, rcs, sym, trim_line, trim_loop, vent
from vf import Ship, box, crystal, grow, keel, loft, octagon, plate, post, rect, sec, sphere, sweep, tube, wedge

# Schildwand: ein Bogen vom Radius WALL_R um den Bug, aus ebenen, senkrechten Facetten zwischen den Stützstellen WALL_X;
# in ihrer Baugruppe liegt die Vorderfläche bei y = wall_y(x), eingebaut wird sie bei WALL_AT, um WALL_LEAN Grad nach
# achtern geneigt
WALL_R = 2.6
WALL_X = (0.0, 1.2, 2.1, 2.5)
WALL_DEPTH = 0.45
WALL_AT = (0.0, -4.95, 0.0)
WALL_LEAN = 12.0
# Türme (Steuerbord): Mitte x, y, Fuß und Oberkante; vorn die hohen Tortürme an den Enden der Wand, achtern die Ecktürme
GATE = (2.5, -2.75, 0.35, 3.2)
CORNER = (2.3, 2.8, 1.5, 2.15)


def wall_y(x):
    """Rücklage der Wand an der Stelle x, linear zwischen den Stützstellen auf dem Bogen (die Facetten sind eben)."""
    ax = min(abs(x), WALL_X[-1])
    arc = [WALL_R - math.sqrt(WALL_R * WALL_R - k * k) for k in WALL_X]
    for (x0, y0), (x1, y1) in zip(zip(WALL_X, arc), zip(WALL_X[1:], arc[1:])):
        if ax <= x1:
            return y0 + (ax - x0) / (x1 - x0) * (y1 - y0)
    return arc[-1]


def facet(x):
    """Drehung (Grad um z) der Facette an der Stelle x, für Teile, die flach auf ihr sitzen."""
    return math.degrees(math.atan((wall_y(x + 0.01) - wall_y(x - 0.01)) / 0.02))


def wall_back(x, z):
    """y der Wandrückseite im Schiff an der Stelle (x, z): Decks und Sockel schließen dort an."""
    lean = math.radians(WALL_LEAN)
    return WALL_AT[1] + (wall_y(x) + WALL_DEPTH) / math.cos(lean) + (z - WALL_AT[2]) * math.tan(lean)


def wall_slab(rows, depth, dy=0.0):
    """Gewölbte Wand aus ebenen, senkrechten Facetten: `rows` sind Stützstellen (x, z unten, z oben) von Backbord nach
    Steuerbord; die Vorderfläche liegt bei y = dy + wall_y(x)."""
    rings = []
    for x, z0, z1 in rows:
        y = dy + wall_y(x)
        rings.append([Vector((x, y, z0)), Vector((x, y, z1)), Vector((x, y + depth, z1)), Vector((x, y + depth, z0))])
    return loft(rings)


def both(half):
    """Stützstellen der Steuerbordhälfte (von der Mitte nach außen) auf beide Seiten gespiegelt."""
    return [(-x, z0, z1) for x, z0, z1 in reversed(half) if x > 0] + list(half)


def between(rows, x):
    """z unten und oben der Wand an der Stelle x, linear zwischen den Stützstellen der Steuerbordhälfte `rows`."""
    ax = abs(x)
    for (x0, a0, b0), (x1, a1, b1) in zip(rows, rows[1:]):
        if ax <= x1:
            t = (ax - x0) / (x1 - x0)
            return a0 + t * (a1 - a0), b0 + t * (b1 - b0)
    return rows[-1][1:]


def panel(rows, x0, x1, inset=(0.28, 0.3)):
    """Stützstellen einer Schildplatte von x0 bis x1 (Facettenknicke dazwischen inklusive), oben und unten um `inset`
    kleiner als die Wand `rows`."""
    xs = [x0] + [x for x in sorted({*WALL_X, *(-x for x in WALL_X)}) if x0 < x < x1] + [x1]
    return [(x, between(rows, x)[0] + inset[0], between(rows, x)[1] - inset[1]) for x in xs]


# Abzeichen von Admiral Osei wie auf seiner Uniform: dunkle Platte mit Goldrand, darin der Bernstein-Schild; die Spitzen
# zeigen nach achtern (in der Ansicht von hinten oben steht der Schild aufrecht)
BADGE = [(-1.15, -0.95), (1.15, -0.95), (1.15, -0.1), (0.0, 1.05), (-1.15, -0.1)]
SHIELD = [(-0.55, -0.7), (0.55, -0.7), (0.55, -0.06), (0.32, 0.32), (0.0, 0.54), (-0.32, 0.32), (-0.55, -0.06)]


def deck_front(z, half_width, inset=0.08):
    """Vorderkante eines Decks auf Höhe z, knapp in der Wandrückseite (Steuerbordhälfte, von der Mitte nach außen)."""
    xs = [x for x in WALL_X if x < half_width] + [half_width]
    return [(x, wall_back(x, z) - inset) for x in xs]


def pylon(s, m, x, y, z):
    """Schildpylon auf einem Turm mit Oberkante z (Steuerbord, gespiegelt): Bernstein-Kranz, rund mit eckiger
    Deckscheibe (eine runde Deckfläche überschritt in `behind` mit dem Glanzpunkt als Ganzes die Schwelle und leuchtete
    weiß), dunkler Fuß mit vier Kondensatorblöcken, darauf der Pylon mit Fokusring und Emitterkristall."""
    s.add(post(20, 0.635, 0.635, z, z + 0.16), m["accent_round"], at=(x, y, 0), mirror=True)
    s.add(post(20, 0.64, 0.64, z + 0.13, z + 0.17), m["accent"], at=(x, y, 0), mirror=True)
    s.add(post(8, 0.47, 0.45, z + 0.14, z + 0.46), m["dark"], at=(x, y, 0), mirror=True)
    for a in range(0, 360, 90):
        c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
        s.add(box(0.2, 0.26, 0.28), m["armor"], at=(x + 0.5 * c, y + 0.5 * d, z + 0.24), rot=(0, 0, a), mirror=True)
    s.add(post(8, 0.32, 0.13, z + 0.16, z + 1.45), m["accent"], at=(x, y, 0), mirror=True)
    s.add(post(8, 0.27, 0.27, z + 0.9, z + 1.0), m["dark"], at=(x, y, 0), mirror=True)
    s.add(crystal(0.2, 0.34), m["glow"], at=(x, y, z + 1.66), mirror=True)


def emitter_bank(s, m, x, y, r, angle, z0, z1):
    """Emitterbank außen an einem runden Turm (Mitte x, y, Radius r; Steuerbord, gespiegelt): dunkles Gehäuse in
    Richtung `angle` (Grad, 0 = +x) von z0 bis z1, darauf drei Linsen in der Akzentfarbe übereinander."""
    c, d = math.cos(math.radians(angle)), math.sin(math.radians(angle))
    s.add(box(0.18, 0.34, z1 - z0), m["dark"], at=(x + (r + 0.02) * c, y + (r + 0.02) * d, (z0 + z1) / 2), rot=(0, 0, angle),
          mirror=True)
    for k in (1, 2, 3):
        s.add(tube(16, 0.1, 0.1, -0.03, 0.03), m["accent_round"], at=(x + (r + 0.11) * c, y + (r + 0.11) * d, z0 + (z1 - z0) * k / 4),
              rot=(0, 0, angle - 90), mirror=True)


def gate(s, m, mat, at, w, h, rot, mirror=False):
    """Tor in einer Wand, lokal Breite x, Höhe y und +z nach außen, `at` die Mitte auf der Wandfläche: flacher Rahmen
    aus `mat` (dem Material der Wand), dunkle Vertiefung und zwei Flügel (`armor`) mit 0,06 Spalt dazwischen."""
    g = Ship("gate")
    bar, gap = 0.07, 0.06
    g.add(box(w - bar, h - bar, 0.04), m["dark"], at=(0, 0, -0.01))
    for sy in (-1, 1):
        g.add(box(w, bar, 0.08), mat, at=(0, sy * (h - bar) / 2, 0.01))
    for sx in (-1, 1):
        g.add(box(bar, h - 2 * bar, 0.08), mat, at=(sx * (w - bar) / 2, 0, 0.01))
    leaf = (w - 2 * bar - gap) / 2
    for sx in (-1, 1):
        g.add(box(leaf - 0.02, h - 2 * bar - 0.04, 0.05), m["armor"], at=(sx * (gap + leaf) / 2, 0, 0))
    s.place(g, at=at, rot=rot, mirror=mirror)
    g.bm.free()


def build():
    s = Ship("bastion")
    m = vf.materials(s.name, "battleship")

    # ---------------------------------------------------------------- Rumpf, Keilbug, Sockel, Schürzen
    # Kastenrumpf auf einem gestuften Sockel (umgekehrte Stufenpyramide) mit Kiel; vorn ein Keilbug unter der Schildwand
    # mit Sims, goldener Kante und Bugemitter
    s.add(loft([
        sec(-4.3, octagon(2.4, 1.1, 0.4)),
        sec(-3.1, octagon(5.0, 1.4, 0.3)),
        sec(3.3, octagon(5.0, 1.4, 0.3)),
        sec(4.5, octagon(3.6, 1.2, 0.4)),
    ]), m["hull"], at=(0, 0, 0.5))
    prow = [(-5.6, 1.0, 0.8), (-4.6, 3.4, 1.6), (-2.8, 4.6, 1.7)]
    s.add(loft([sec(y, keel(w, h), dz=0.55 - h / 2) for y, w, h in prow]), m["armor"])
    ledge = grow(sym([(w / 2, y) for y, w, h in prow]), 0.24)
    s.add(plate(ledge, 0.24, "xy"), m["hull"], at=(0, 0, 0.41))
    s.add(plate(grow(ledge, -0.15), 0.07, "xy"), m["armor"], at=(0, 0, 0.55))
    trim_line(s, m, [(x + 0.02, y, 0.53) for x, y in ledge if x > 0.01 and y < -2.9])
    s.add(box(0.56, 0.06, 0.14), m["glow"], at=(0, prow[0][0] - 0.01, 0.06))
    step1 = sym([(0.0, -3.6), (1.5, -3.4), (2.15, -2.7), (2.15, 2.9), (1.6, 3.9)])
    s.add(plate(step1, 0.45, "xy"), m["armor"], at=(0, 0, -0.38))
    step2 = sym([(0.0, -3.1), (1.0, -2.9), (1.35, -2.4), (1.35, 2.4), (0.9, 3.3)])
    s.add(plate(step2, 0.4, "xy"), m["hull"], at=(0, 0, -0.8))
    s.add(plate([(-3.0, -0.9), (3.2, -0.9), (2.9, -1.22), (-2.2, -1.22)], 0.3, "yz"), m["dark"])
    # Panzerschürzen in Bernstein an den Flanken, darüber ein Fensterband
    s.add(plate([(2.35, -3.0), (2.8, -2.6), (2.8, 2.7), (2.35, 3.1)], 0.32, "xy"), m["accent"], at=(0, 0, 0.3), mirror=True)
    s.add(sweep([(2.51, -2.6, 0.8), (2.51, 2.6, 0.8)], rect(0.04, 0.08)), m["window"], mirror=True)

    # ---------------------------------------------------------------- Terrassendecks
    # Drei Terrassendecks, das unterste mit goldener Zierlinie
    deck1 = sym(deck_front(1.35, 2.85) + [(2.85, 3.3), (2.4, 4.1), (1.4, 4.45)])
    s.add(plate(deck1, 0.3, "xy"), m["armor"], at=(0, 0, 1.35))
    trim_loop(s, m, deck1, 1.5)
    deck2 = grow(deck1, -0.6)
    s.add(plate(deck2, 0.3, "xy"), m["hull"], at=(0, 0, 1.65))
    deck3 = sym([(0.0, -2.7), (1.1, -2.55), (1.55, -2.1), (1.55, 3.3), (1.2, 3.85)])
    s.add(plate(deck3, 0.26, "xy"), m["armor"], at=(0, 0, 1.93))

    # ---------------------------------------------------------------- Schildwand
    # Im Bogen um den Bug zwischen zwei Tortürmen: Rahmen mit Zinnen und goldener Oberkante, Bernstein-Schildfeld,
    # Emitterband; an der Rückseite Strebepfeiler unter den äußeren Zinnen, senkrecht zu ihrer Facette
    frame = [(0.0, 0.3, 3.3), (1.2, 0.36, 3.25), (2.1, 0.46, 3.12), (2.5, 0.55, 3.0)]
    g = Ship("bastion_wall")
    g.add(wall_slab(both(frame), WALL_DEPTH), m["hull"])
    g.add(wall_slab(both(panel(frame, 0.0, 2.4, (0.28, 0.28))), 0.2, dy=-0.12), m["accent"])
    g.add(sweep([(x, wall_y(x), z1) for x, _, z1 in both(frame)], rect(0.08, 0.08)), m["trim"])
    for x in (0.0, 0.8, 1.6):
        z1 = between(frame, x)[1]
        g.add(box(0.5, WALL_DEPTH, 0.5), m["hull"], at=(x, wall_y(x) + WALL_DEPTH / 2, z1 + 0.13), rot=(0, 0, facet(x)), mirror=x > 0)
    g.add(plate([(-0.1, 1.3), (-0.1, 2.75), (0.3, 2.75), (1.05, 1.3)], 0.3, "yz"), m["hull"], at=(1.6, wall_y(1.6) + WALL_DEPTH, 0),
          rot=(0, 0, facet(1.6)), mirror=True)
    band = 1.45
    g.add(sweep([(x, wall_y(x) - 0.12, band) for x in (-2.35, -2.1, -1.2, 0.0, 1.2, 2.1, 2.35)], rect(0.1, 0.16)), m["signal"])
    s.place(g, at=WALL_AT, rot=(-WALL_LEAN, 0, 0))
    g.bm.free()

    # ---------------------------------------------------------------- Türme und Schildpylone
    # Vier runde Türme, darauf die Pylone; die Tortürme mit Gurtring unter dem Kranz, einer Emitterbank vorn außen und
    # einer Wartungstür zur Terrasse
    for x, y, z0, z1 in (GATE, CORNER):
        s.add(post(20, 0.6, 0.57, z0, z1), m["hull_round"], at=(x, y, 0), mirror=True)
        pylon(s, m, x, y, z1)
    x, y, _, _ = GATE
    s.add(post(20, 0.63, 0.63, 2.62, 2.8), m["armor_round"], at=(x, y, 0), mirror=True)
    emitter_bank(s, m, x, y, 0.59, 315, 1.05, 2.45)
    gate(s, m, m["hull"], (x, y + 0.575, 2.15), 0.34, 0.5, (-90, 0, 0), mirror=True)

    # ---------------------------------------------------------------- Energieleitungen über die Terrassen
    # Vom Eckturm zum Torturm am Fuß des mittleren Decks, mit Anschlusskästen an beiden Türmen und zwei Schellen
    s.add(box(0.24, 4.5, 0.18), m["dark"], at=(2.52, 0.05, 1.57), mirror=True)
    for y in (-2.0, 2.06):
        s.add(box(0.34, 0.32, 0.3), m["armor"], at=(2.52, y, 1.63), mirror=True)
    for y in (-0.55, 0.65):
        s.add(box(0.32, 0.12, 0.24), m["armor"], at=(2.52, y, 1.6), mirror=True)

    # ---------------------------------------------------------------- Bergfried
    # Sockel mit Lüftern, achteckiger Turm mit Tor nach vorn, auskragende Krone (Brücke) mit umlaufendem Fensterband,
    # Dach, Mast mit Querträger, Radarplatte und Licht
    ky = 2.75
    s.add(wedge(2.5, 2.3, 0.45), m["hull"], at=(0, ky - 0.05, 2.06 + 0.225))
    vent(s, m, m["hull"], (0.85, 3.55, 2.51), 0.4, 0.42, slats=2, mirror=True)
    s.add(post(8, 0.86, 0.78, 2.5, 3.7), m["armor"], at=(0, ky, 0), rot=(0, 0, 22.5))
    gate(s, m, m["armor"], (0, ky - (0.795 - 0.0625 * 0.36), 2.86), 0.5, 0.62, (86.4, 0, 0))
    s.add(post(8, 1.12, 1.12, 3.7, 4.15), m["hull"], at=(0, ky, 0), rot=(0, 0, 22.5))
    s.add(post(8, 1.14, 1.14, 3.86, 3.96), m["window"], at=(0, ky, 0), rot=(0, 0, 22.5))
    s.add(post(8, 0.75, 0.6, 4.15, 4.35), m["dark"], at=(0, ky, 0), rot=(0, 0, 22.5))
    s.add(post(10, 0.09, 0.06, 4.35, 5.0), m["hull_round"], at=(0, ky + 0.2, 0))
    s.add(box(0.8, 0.07, 0.07), m["hull"], at=(0, ky + 0.2, 4.8))
    s.add(box(0.5, 0.05, 0.2), m["armor"], at=(0, ky + 0.11, 4.56), rot=(-15, 0, 0))
    s.add(sphere(0.1, 1), m["glow"], at=(0, ky + 0.2, 5.07))

    # Wappenplatte vor dem Bergfried
    badge(s, m, (0.0, -1.2, 2.06), BADGE, SHIELD, rim=0.09, dy=0.08)

    # ---------------------------------------------------------------- Achterdeck und Heck
    # Maschinenlüfter über den Düsen, RCS an den hinteren Rumpfecken; drei schwere Heckdüsen, zwei kleine an den
    # Deckecken, jede mit Gehäusesegment und Düsenkranz
    vent(s, m, m["armor"], (0, 4.15, 1.5), 0.36, 1.3, slats=3, rot=(0, 0, 90))
    rcs(s, m, (2.3, 3.55, 0.5), 1)
    for dx in (0.0, 1.15):
        engine(s, m, 0.5, 4.15, 5.45, dx=dx, dz=0.45, mirror=dx > 0)
        engine_details(s, m, 0.5, 4.15, 5.45, dx=dx, dz=0.45, mirror=dx > 0)
    engine(s, m, 0.3, 3.75, 4.75, dx=2.05, dz=1.0, mirror=True)
    engine_details(s, m, 0.3, 3.75, 4.75, dx=2.05, dz=1.0, mirror=True)
    return s.finish()
