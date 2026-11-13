"""Bastion (Flaggschiff von Admiral Osei, „die Mauer“), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

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
Fugen und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern am Bergfried, an der Wand des
obersten Decks und an der Heckwand, Geländer um die Krone, am obersten Deck und am Sockel, Leiter an der Heckwand, Flak
auf dem Bugsims, der mittleren Terrasse und dem Achterdeck, Peitschenantennen, Schüssel und Gerätekasten am Bergfried,
Rohrleitungen vom Eckturm mit Anschlusskasten, Kisten, Luken, Kühlrippen an den Düsen, Positionslichter.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen, wie groß das Schiff ist, und wo man es betritt,
wartet und verschraubt): Treppen zwischen den Terrassen mit Laufstegen aus Gitterrost, Luken mit Rahmen, Scharnieren und
Griff, Schraubenköpfe am Wappen, an den Turmringen und Zinnen, Schießscharten, Gurtringe und Steigeisen an den Türmen,
Spulenringe und Leitungen an den Pylonen, Konsolen unter der Krone und Eckrippen am Bergfried, Fenstersimse,
Dachaggregate, Emitterknöpfe auf dem Schildfeld, Kabelkanal an der Wandrückseite, Spanten, Rohr, Dockluke und
Schürzenklammern an der Flanke, Stellstreben an den Düsenglocken, Poller und Reling am Bugsims. Dazu Beschriftung als
flache Geometrie: Wegelinien zum Tor des Bergfrieds, Warnbänder an Treppen, Lüftern und Dockluke, die Kennnummer an
der Schürze.
"""

import math

from mathutils import Vector

import vf
from parts import (badge, beam, bolts, engine, engine_details, flak, grating, hatch, number, rail, rcs, spaced, stairs, stripes, sym,
                   trim_line, trim_loop, vent, windows)
from vf import (Ship, box, chamfer, crystal, dish, grow, keel, loft, octagon, pipe, plate, post, rect, sec, segment, sphere, sweep, tube,
                wedge)

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
TOWER_R = (0.6, 0.57)  # Radius der Türme am Fuß und an der Oberkante
ROUND = 40  # Segmente der großen runden Teile (Türme, Kränze, schwere Düsen)


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
    s.add(post(ROUND, 0.635, 0.635, z, z + 0.16), m["accent_round"], at=(x, y, 0), mirror=True)
    s.add(post(ROUND, 0.64, 0.64, z + 0.13, z + 0.17), m["accent"], at=(x, y, 0), mirror=True)
    s.add(chamfer(post(8, 0.47, 0.45, z + 0.14, z + 0.46), 0.025), m["dark"], at=(x, y, 0), mirror=True)
    for a in range(0, 360, 90):
        c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
        s.add(chamfer(box(0.2, 0.26, 0.28), 0.025), m["armor"], at=(x + 0.5 * c, y + 0.5 * d, z + 0.24), rot=(0, 0, a), mirror=True)
    s.add(chamfer(post(8, 0.32, 0.13, z + 0.16, z + 1.45), 0.02), m["accent"], at=(x, y, 0), mirror=True)
    s.add(chamfer(post(8, 0.27, 0.27, z + 0.9, z + 1.0), 0.015), m["dark"], at=(x, y, 0), mirror=True)
    s.add(crystal(0.2, 0.34), m["glow"], at=(x, y, z + 1.66), mirror=True)


def emitter_bank(s, m, x, y, r, angle, z0, z1):
    """Emitterbank außen an einem runden Turm (Mitte x, y, Radius r; Steuerbord, gespiegelt): dunkles Gehäuse in
    Richtung `angle` (Grad, 0 = +x) von z0 bis z1, darauf drei Linsen in der Akzentfarbe übereinander."""
    c, d = math.cos(math.radians(angle)), math.sin(math.radians(angle))
    s.add(box(0.18, 0.34, z1 - z0), m["dark"], at=(x + (r + 0.02) * c, y + (r + 0.02) * d, (z0 + z1) / 2), rot=(0, 0, angle),
          mirror=True)
    for k in (1, 2, 3):
        s.add(tube(24, 0.1, 0.1, -0.03, 0.03), m["accent_round"], at=(x + (r + 0.11) * c, y + (r + 0.11) * d, z0 + (z1 - z0) * k / 4),
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


def details(s, m, ky):
    """Kleinteile (Modulkommentar); `ky` ist die Achse des Bergfrieds."""

    def keep(z):
        """Abstand der Wandflächen des Bergfrieds von seiner Achse in Höhe z (er verjüngt sich nach oben)."""
        return (0.86 - 0.08 * (z - 2.5) / 1.2) * math.cos(math.radians(22.5))

    # ---------------------------------------------------------------- Bergfried: Fenster, Geländer um die Krone, Antennen
    four = (-0.18, -0.06, 0.06, 0.18)
    for z in (3.12, 3.36):
        windows(s, m, [(keep(z), ky + d, z) for d in four])
        windows(s, m, [(d, ky + keep(z), z) for d in four], out="y")
    corners = [(1.06 * math.cos(a), ky + 1.06 * math.sin(a)) for a in (math.radians(22.5 + 45 * k) for k in range(8))]
    for (ax, ay), (bx, by) in zip(corners, corners[1:] + corners[:1]):
        s.add(beam((ax, ay, 4.24), (bx, by, 4.24), 0.014, 0.014), m["dark"])
        for x, y in ((ax, ay), ((ax + bx) / 2, (ay + by) / 2)):
            s.add(box(0.014, 0.014, 0.1), m["dark"], at=(x, y, 4.195))
    s.add(post(6, 0.014, 0.006, 4.33, 4.92), m["dark_round"], at=(0.36, ky - 0.12, 0), mirror=True)
    s.add(segment((0.4, ky + 0.98, 4.12), (0.5, ky + 1.5, 4.72), 0.012, 6), m["dark_round"], mirror=True)
    # Sockel: Schüssel backbord (zeigt nach achtern außen), Gerätekasten mit Nadel steuerbord
    s.add(post(8, 0.03, 0.025, 2.49, 2.65), m["dark_round"], at=(-1.02, ky - 0.15, 0))
    s.add(dish(0.13, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-1.02, ky - 0.15, 2.64), rot=(-40, 0, -150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(1.02, ky - 0.15, 2.57))
    s.add(post(6, 0.012, 0.006, 2.61, 3.0), m["dark_round"], at=(1.02, ky - 0.09, 0))

    # ---------------------------------------------------------------- Heckwand (Decks und Sockel bündig bei y 3,85)
    for z in (1.93, 2.3):
        windows(s, m, [(sx * (0.3 + 0.2 * k), 3.85, z) for sx in (-1, 1) for k in range(4)], out="y")
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 1.0), m["dark"], at=(dx, 3.857, 2.0))
    for k in range(12):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0, 3.857, 1.56 + 0.08 * k))
    rail(s, m, (-1.15, 3.8, 2.51), (1.15, 3.8, 2.51), 7, mirror=False)

    # ---------------------------------------------------------------- Oberstes Deck: Fenster in der Wand, Geländer, Luken
    windows(s, m, [(1.55, y + 0.12 * k, 1.93) for y in (-1.5, -0.1) for k in range(6)])
    rail(s, m, (1.5, -1.9, 2.06), (1.5, -0.3, 2.06), 5)
    rail(s, m, (1.5, 0.5, 2.06), (1.5, 3.1, 2.06), 7)

    # ---------------------------------------------------------------- Flak: Bugsims, mittlere Terrasse, Achterdeck
    for at, yaw in (((0.45, -5.25, 0.585), 15), ((1.95, -1.25, 1.8), 50), ((1.95, 0.35, 1.8), 100), ((1.75, 4.0, 1.5), 150)):
        flak(s, m, at, yaw)

    # ---------------------------------------------------------------- Mittlere Terrasse: Rohre vom Eckturm, Kasten, Kisten
    for x, (ex, ey) in ((1.62, (1.8, 2.7)), (1.69, (1.84, 2.62))):
        s.add(pipe([(x, 1.1, 1.828), (x, ey - 0.25, 1.828), (ex, ey, 1.828)], 0.028, 6), m["steel"], mirror=True)
    s.add(box(0.22, 0.24, 0.14), m["armor"], at=(1.66, 1.0, 1.87), mirror=True)
    s.add(box(0.22, 0.2, 0.14), m["armor"], at=(-2.0, 0.95, 1.87))
    s.add(box(0.2, 0.22, 0.16), m["dark"], at=(-2.0, 1.2, 1.88))
    s.add(box(0.14, 0.14, 0.1), m["armor"], at=(-2.0, 1.19, 2.01))

    # ---------------------------------------------------------------- Kühlrippen an den Düsen
    for dx in (0.0, 1.15):
        for y in (4.63, 4.69, 4.75):
            s.add(tube(ROUND, 0.57, 0.57, y, y + 0.03, dx, 0.45), m["steel"], mirror=dx > 0)
    for y in (4.12, 4.17, 4.22):
        s.add(tube(24, 0.342, 0.342, y, y + 0.025, 2.05, 1.0), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter: Tortürme, hintere Deckecken, Heck
    s.add(sphere(0.035, 1), m["glow"], at=(3.09, GATE[1], 0.9), mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(2.83, 3.28, 1.51), mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(0, 4.44, 1.36))


def fine(s, m, ky, ledge):
    """Feinteile und Beschriftung (Modulkommentar); `ky` ist die Achse des Bergfrieds, `ledge` der Umriss des
    Bugsimses."""
    # ---------------------------------------------------------------- Terrassen: Treppen, Laufsteg, Luken
    # Vom unteren Deck (1,5) aufs mittlere (1,8) an dessen Wand, vom mittleren aufs oberste (2,06) an dessen Wand, oben
    # in der Lücke der Reling; am Rand des mittleren Decks ein Laufsteg aus Gitterrost
    stairs(s, m, 2.25, 0.86, 1.26, 1.5, 1.8)
    stairs(s, m, 1.55, -0.64, -0.28, 1.8, 2.06)
    grating(s, m, 2.07, 2.19, -0.75, 1.3, 1.8)
    hatch(s, m, (0.85, 0.75, 2.06), 0.3, 0.36, mirror=True)
    hatch(s, m, (1.9, -0.45, 1.8), 0.22, 0.26, mirror=True)
    hatch(s, m, (1.05, 4.15, 1.5), 0.24, 0.28, mirror=True)

    # ---------------------------------------------------------------- Wappen: Schrauben im Goldrand
    bolts(s, m["steel"], [(x, -1.2 + y, 2.12) for x, y in spaced(grow(BADGE, 0.045), 0.27)])

    # ---------------------------------------------------------------- Türme: Gurtringe, Scharten, Steigeisen
    for (x, y, z0, z1), bands, slits in ((GATE, (0.95, 1.95), ((-25, 1.2), (20, 1.2), (-40, 2.3), (0, 2.3), (140, 2.35), (180, 2.35))),
                                         (CORNER, (1.98,), ((-35, 1.78), (25, 1.78), (150, 1.92)))):
        def radius(z):
            return TOWER_R[0] + (TOWER_R[1] - TOWER_R[0]) * (z - z0) / (z1 - z0)

        for z in bands:
            s.add(post(ROUND, radius(z) + 0.014, radius(z + 0.05) + 0.014, z, z + 0.05), m["armor_round"], at=(x, y, 0), mirror=True)
            for a in range(0, 360, 24):
                c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
                bolts(s, m["steel"], [(x + (radius(z) + 0.014) * c, y + (radius(z) + 0.014) * d, z + 0.025)], (0, 90, a), mirror=True)
        for a, z in slits:
            c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
            s.add(box(0.014, 0.024, 0.13), m["dark"], at=(x + (radius(z) + 0.001) * c, y + (radius(z) + 0.001) * d, z), rot=(0, 0, a),
                  mirror=True)
    x, y, z0, z1 = GATE
    c, d = math.cos(math.radians(50)), math.sin(math.radians(50))
    for k in range(18):
        z = 1.56 + 0.06 * k
        r = TOWER_R[0] + (TOWER_R[1] - TOWER_R[0]) * (z - z0) / (z1 - z0) + 0.008
        s.add(box(0.03, 0.07, 0.012), m["steel_flat"], at=(x + r * c, y + r * d, z), rot=(0, 0, 50), mirror=True)

    # ---------------------------------------------------------------- Pylone: Spulenringe, Leitungen, Schrauben
    for x, y, _, z in (GATE, CORNER):
        def obelisk(h):
            return 0.32 - 0.19 * (h - 0.16) / 1.29

        for h in (0.5, 0.62, 0.74):
            s.add(post(8, obelisk(h) + 0.018, obelisk(h + 0.03) + 0.018, z + h, z + h + 0.03), m["dark"], at=(x, y, 0), mirror=True)
        for a in (22.5, 202.5):
            c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
            low, high = 0.924 * obelisk(0.46) + 0.008, 0.924 * obelisk(0.9) + 0.008
            s.add(segment((x + low * c, y + low * d, z + 0.44), (x + high * c, y + high * d, z + 0.92), 0.014, 6), m["steel"], mirror=True)
        bolts(s, m["steel"], [(x + 0.58 * math.cos(math.radians(a + k)), y + 0.58 * math.sin(math.radians(a + k)), z + 0.17)
                              for a in range(0, 360, 90) for k in (30, 45, 60)], mirror=True)

    # ---------------------------------------------------------------- Bergfried: Konsolen, Rippen, Simse, Dach
    for k in range(8):
        a = math.radians(45 * k)
        c, d = math.cos(a), math.sin(a)
        for side in (-0.2, 0.2):
            s.add(plate([(0.0, 0.0), (0.26, 0.0), (0.0, -0.24)], 0.05, "yz"), m["hull"],
                  at=(0.71 * c - side * d, ky + 0.71 * d + side * c, 3.7), rot=(0, 0, 45 * k - 90))
        c, d = math.cos(a + math.radians(22.5)), math.sin(a + math.radians(22.5))
        s.add(beam((0.86 * c, ky + 0.86 * d, 2.5), (0.78 * c, ky + 0.78 * d, 3.7), 0.06, 0.06), m["hull"])
    for z in (3.12, 3.36):
        face = (0.86 - 0.08 * (z - 2.5) / 1.2) * math.cos(math.radians(22.5))
        s.add(box(0.03, 0.44, 0.014), m["dark"], at=(face + 0.008, ky, z - 0.036), mirror=True)
        s.add(box(0.44, 0.03, 0.014), m["dark"], at=(0, ky + face + 0.008, z - 0.036))
    s.add(chamfer(box(0.22, 0.16, 0.09), 0.015), m["armor"], at=(-0.2, ky - 0.28, 4.39))
    s.add(box(0.14, 0.1, 0.012), m["dark"], at=(-0.2, ky - 0.28, 4.438))
    s.add(sphere(0.07, 2), m["steel"], at=(0.22, ky - 0.3, 4.36))

    # ---------------------------------------------------------------- Flanke: Spanten, Rohr, Dockluke, Nummer
    # Zwischen Schürze (Oberkante 0,46) und Fensterband (0,76): drei Gruppen Spanten, ein Rohr, das in die Schürze
    # abtaucht, die runde Dockluke (eckig schattiert wie die Scharniere in `hatch`) mit Trittfläche (Warnband) auf der
    # Schürze; außen an der Schürze Klammern und zwischen den vorderen die Kennnummer, auf jeder Seite lesbar
    for y in (-1.9, 0.0, 1.9):
        for k in (-1, 0, 1):
            s.add(box(0.035, 0.04, 0.27), m["armor"], at=(2.51, y + 0.22 * k, 0.61), mirror=True)
    s.add(pipe([(2.52, -2.5, 0.53), (2.52, 0.55, 0.53), (2.52, 0.55, 0.45)], 0.02, 6), m["steel"], mirror=True)
    s.add(tube(24, 0.12, 0.12, -0.02, 0.03), m["armor"], at=(2.5, 0.95, 0.61), rot=(0, 0, -90), mirror=True)
    s.add(tube(24, 0.095, 0.095, -0.02, 0.04), m["dark"], at=(2.5, 0.95, 0.61), rot=(0, 0, -90), mirror=True)
    s.add(tube(12, 0.035, 0.035, 0.03, 0.055), m["steel_flat"], at=(2.5, 0.95, 0.61), rot=(0, 0, -90), mirror=True)
    bolts(s, m["steel_flat"], [(2.53, 0.95 + 0.107 * math.cos(math.radians(a)), 0.61 + 0.107 * math.sin(math.radians(a)))
                               for a in range(0, 360, 45)], "x", r=0.01, mirror=True)
    stripes(s, m, (2.65, 0.95, 0.46), 0.14, 0.5, mirror=True)
    for y in (-1.55, -0.5, 0.55, 1.6):
        s.add(box(0.03, 0.07, 0.3), m["dark"], at=(2.805, y, 0.3), mirror=True)
        bolts(s, m["steel"], [(2.82, y, 0.21), (2.82, y, 0.39)], "x", mirror=True)
    number(s, m["dark"], "01", (2.8, -1.03, 0.3), 0.18, (90, 0, 90))
    number(s, m["dark"], "01", (-2.8, -1.03, 0.3), 0.18, (90, 0, -90))

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    for dx, dz, r, y0, y1, count in ((0.0, 0.45, 0.5, 4.15, 5.45, 8), (1.15, 0.45, 0.5, 4.15, 5.45, 8), (2.05, 1.0, 0.3, 3.75, 4.75, 6)):
        ya = y0 + 0.6 * (y1 - y0)
        for k in range(count):
            a = 2 * math.pi * (k + 0.5) / count
            c, d = math.cos(a), math.sin(a)
            s.add(segment((dx + 1.1 * r * c, ya, dz + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, dz + 0.93 * r * d), 0.016, 6), m["steel"],
                  mirror=dx > 0)

    # ---------------------------------------------------------------- Bugsims: Reling vor der Wand, Poller
    front = [(x, y) for x, y in grow(ledge, -0.06) if x > 0.01 and y < -2.9]
    for (ax, ay), (bx, by) in zip(front, front[1:]):
        rail(s, m, (ax, ay, 0.53), (bx, by, 0.53), max(2, round(math.hypot(bx - ax, by - ay) / 0.3)))
    s.add(post(10, 0.035, 0.042, 0.57, 0.66), m["steel"], at=(0.3, -5.52, 0), mirror=True)

    # ---------------------------------------------------------------- Beschriftung: Wegelinien, Warnbänder, Nummer
    # Zwei Linien vom Wappen zum Tor des Bergfrieds; Warnbänder am Treppenkopf in der Reling und hinter dem
    # Maschinenlüfter. Die große Nummer steht an der Rückseite der Schildwand (`build`), lesbar von hinten oben
    s.add(box(0.025, 1.45, 0.006), m["paint"], at=(0.28, 0.72, 2.062), mirror=True)
    stripes(s, m, (1.42, 0.1, 2.06), 0.1, 0.62, mirror=True)
    stripes(s, m, (0, 4.385, 1.5), 0.08, 1.3, rot=(0, 0, 90))


def build():
    s = Ship("bastion")
    m = vf.materials(s.name, "battleship")

    # ---------------------------------------------------------------- Rumpf, Keilbug, Sockel, Schürzen
    # Kastenrumpf auf einem gestuften Sockel (umgekehrte Stufenpyramide) mit Kiel; vorn ein Keilbug unter der Schildwand
    # mit Sims, goldener Kante und Bugemitter
    s.add(chamfer(loft([
        sec(-4.3, octagon(2.4, 1.1, 0.4)),
        sec(-3.1, octagon(5.0, 1.4, 0.3)),
        sec(3.3, octagon(5.0, 1.4, 0.3)),
        sec(4.5, octagon(3.6, 1.2, 0.4)),
    ]), 0.05), m["hull"], at=(0, 0, 0.5))
    prow = [(-5.6, 1.0, 0.8), (-4.6, 3.4, 1.6), (-2.8, 4.6, 1.7)]
    s.add(chamfer(loft([sec(y, keel(w, h), dz=0.55 - h / 2) for y, w, h in prow]), 0.04), m["armor"])
    ledge = grow(sym([(w / 2, y) for y, w, h in prow]), 0.24)
    s.add(chamfer(plate(ledge, 0.24, "xy"), 0.03), m["hull"], at=(0, 0, 0.41))
    s.add(chamfer(plate(grow(ledge, -0.15), 0.07, "xy"), 0.02), m["armor"], at=(0, 0, 0.55))
    trim_line(s, m, [(x + 0.02, y, 0.53) for x, y in ledge if x > 0.01 and y < -2.9])
    s.add(box(0.56, 0.06, 0.14), m["glow"], at=(0, prow[0][0] - 0.01, 0.06))
    step1 = sym([(0.0, -3.6), (1.5, -3.4), (2.15, -2.7), (2.15, 2.9), (1.6, 3.9)])
    s.add(chamfer(plate(step1, 0.45, "xy"), 0.05), m["armor"], at=(0, 0, -0.38))
    step2 = sym([(0.0, -3.1), (1.0, -2.9), (1.35, -2.4), (1.35, 2.4), (0.9, 3.3)])
    s.add(chamfer(plate(step2, 0.4, "xy"), 0.05), m["hull"], at=(0, 0, -0.8))
    s.add(chamfer(plate([(-3.0, -0.9), (3.2, -0.9), (2.9, -1.22), (-2.2, -1.22)], 0.3, "yz"), 0.04), m["dark"])
    # Panzerschürzen in Bernstein an den Flanken, darüber ein Fensterband
    s.add(chamfer(plate([(2.35, -3.0), (2.8, -2.6), (2.8, 2.7), (2.35, 3.1)], 0.32, "xy"), 0.04), m["accent"], at=(0, 0, 0.3), mirror=True)
    s.add(sweep([(2.51, -2.6, 0.8), (2.51, 2.6, 0.8)], rect(0.04, 0.08)), m["window"], mirror=True)

    # ---------------------------------------------------------------- Terrassendecks
    # Drei Terrassendecks, das unterste mit goldener Zierlinie (sie deckt seine Kante, die anderen sind gefast)
    deck1 = sym(deck_front(1.35, 2.85) + [(2.85, 3.3), (2.4, 4.1), (1.4, 4.45)])
    s.add(plate(deck1, 0.3, "xy"), m["armor"], at=(0, 0, 1.35))
    trim_loop(s, m, deck1, 1.5)
    deck2 = grow(deck1, -0.6)
    s.add(chamfer(plate(deck2, 0.3, "xy"), 0.04), m["hull"], at=(0, 0, 1.65))
    deck3 = sym([(0.0, -2.7), (1.1, -2.55), (1.55, -2.1), (1.55, 3.3), (1.2, 3.85)])
    s.add(chamfer(plate(deck3, 0.26, "xy"), 0.04), m["armor"], at=(0, 0, 1.93))

    # ---------------------------------------------------------------- Schildwand
    # Im Bogen um den Bug zwischen zwei Tortürmen: Rahmen mit Zinnen und goldener Oberkante, Bernstein-Schildfeld,
    # Emitterband; an der Rückseite Strebepfeiler unter den äußeren Zinnen, senkrecht zu ihrer Facette
    frame = [(0.0, 0.3, 3.3), (1.2, 0.36, 3.25), (2.1, 0.46, 3.12), (2.5, 0.55, 3.0)]
    g = Ship("bastion_wall")
    g.add(chamfer(wall_slab(both(frame), WALL_DEPTH), 0.04), m["hull"])
    field = panel(frame, 0.0, 2.4, (0.28, 0.28))
    g.add(chamfer(wall_slab(both(field), 0.2, dy=-0.12), 0.03), m["accent"])
    # Emitterknöpfe auf dem Schildfeld in dessen Farbe (auf dem Brett bleibt das Feld ruhig), versetzte Reihen; jeder
    # schaut wie seine Facette
    for row, z in enumerate((0.95, 1.9, 2.35, 2.8)):
        for k in range(-5, 6):
            x = 0.4 * k + 0.2 * (row % 2)
            if abs(x) < 2.25 and between(field, x)[0] + 0.12 < z < between(field, x)[1] - 0.12:
                g.add(post(6, 0.04, 0.03, -0.006, 0.02), m["accent"], at=(x, wall_y(x) - 0.12, z), rot=(90, 0, facet(x)))
    g.add(sweep([(x, wall_y(x), z1) for x, _, z1 in both(frame)], rect(0.08, 0.08)), m["trim"])
    for x in (0.0, 0.8, 1.6):
        z1 = between(frame, x)[1]
        g.add(chamfer(box(0.5, WALL_DEPTH, 0.5), 0.04), m["hull"], at=(x, wall_y(x) + WALL_DEPTH / 2, z1 + 0.13), rot=(0, 0, facet(x)),
              mirror=x > 0)
        bolts(g, m["steel"], [(x + dx, wall_y(x) + WALL_DEPTH / 2 + dy, z1 + 0.38) for dx in (-0.17, 0.17) for dy in (-0.13, 0.13)],
              mirror=x > 0)
    g.add(chamfer(plate([(-0.1, 1.3), (-0.1, 2.75), (0.3, 2.75), (1.05, 1.3)], 0.3, "yz"), 0.04), m["hull"],
          at=(1.6, wall_y(1.6) + WALL_DEPTH, 0), rot=(0, 0, facet(1.6)), mirror=True)
    # Kabelkanal an der Rückseite von Pfeiler zu Pfeiler, mit Anschlusskasten in der Mitte
    g.add(sweep([(x, wall_y(x) + WALL_DEPTH + 0.03, 1.75) for x in (-1.45, -1.2, 0.0, 1.2, 1.45)], rect(0.07, 0.07)), m["dark"])
    g.add(chamfer(box(0.3, 0.1, 0.22), 0.02), m["armor"], at=(0, wall_y(0) + WALL_DEPTH + 0.04, 1.75))
    bolts(g, m["steel"], [(sx * 0.11, wall_y(0) + WALL_DEPTH + 0.09, 1.75 + sz * 0.07) for sx in (-1, 1) for sz in (-1, 1)], "y")
    number(g, m["paint"], "01", (-0.6, wall_y(-0.6) + WALL_DEPTH, 2.6), 0.45, (90, 0, 180 + facet(-0.6)))
    band = 1.45
    g.add(sweep([(x, wall_y(x) - 0.12, band) for x in (-2.35, -2.1, -1.2, 0.0, 1.2, 2.1, 2.35)], rect(0.1, 0.16)), m["signal"])
    s.place(g, at=WALL_AT, rot=(-WALL_LEAN, 0, 0))
    g.bm.free()

    # ---------------------------------------------------------------- Türme und Schildpylone
    # Vier runde Türme, darauf die Pylone; die Tortürme mit Gurtring unter dem Kranz, einer Emitterbank vorn außen und
    # einer Wartungstür zur Terrasse
    for x, y, z0, z1 in (GATE, CORNER):
        s.add(post(ROUND, *TOWER_R, z0, z1), m["hull_round"], at=(x, y, 0), mirror=True)
        pylon(s, m, x, y, z1)
    x, y, _, _ = GATE
    s.add(post(ROUND, 0.63, 0.63, 2.62, 2.8), m["armor_round"], at=(x, y, 0), mirror=True)
    emitter_bank(s, m, x, y, 0.59, 315, 1.05, 2.45)
    gate(s, m, m["hull"], (x, y + 0.575, 2.15), 0.34, 0.5, (-90, 0, 0), mirror=True)

    # ---------------------------------------------------------------- Energieleitungen über die Terrassen
    # Vom Eckturm zum Torturm am Fuß des mittleren Decks, mit Anschlusskästen an beiden Türmen und zwei Schellen
    s.add(chamfer(box(0.24, 4.5, 0.18), 0.03), m["dark"], at=(2.52, 0.05, 1.57), mirror=True)
    for y in (-2.0, 2.06):
        s.add(chamfer(box(0.34, 0.32, 0.3), 0.03), m["armor"], at=(2.52, y, 1.63), mirror=True)
    for y in (-0.55, 0.65):
        s.add(chamfer(box(0.32, 0.12, 0.24), 0.02), m["armor"], at=(2.52, y, 1.6), mirror=True)

    # ---------------------------------------------------------------- Bergfried
    # Sockel mit Lüftern, achteckiger Turm mit Tor nach vorn, auskragende Krone (Brücke) mit umlaufendem Fensterband,
    # Dach, Mast mit Querträger, Radarplatte und Licht
    ky = 2.75
    s.add(chamfer(wedge(2.5, 2.3, 0.45), 0.05), m["hull"], at=(0, ky - 0.05, 2.06 + 0.225))
    vent(s, m, m["hull"], (0.85, 3.55, 2.51), 0.4, 0.42, slats=2, mirror=True)
    s.add(chamfer(post(8, 0.86, 0.78, 2.5, 3.7), 0.03), m["armor"], at=(0, ky, 0), rot=(0, 0, 22.5))
    gate(s, m, m["armor"], (0, ky - (0.795 - 0.0625 * 0.36), 2.86), 0.5, 0.62, (86.4, 0, 0))
    s.add(chamfer(post(8, 1.12, 1.12, 3.7, 4.15), 0.04), m["hull"], at=(0, ky, 0), rot=(0, 0, 22.5))
    s.add(chamfer(post(8, 1.14, 1.14, 3.86, 3.96), 0.04), m["window"], at=(0, ky, 0), rot=(0, 0, 22.5))
    s.add(chamfer(post(8, 0.75, 0.6, 4.15, 4.35), 0.03), m["dark"], at=(0, ky, 0), rot=(0, 0, 22.5))
    s.add(post(16, 0.09, 0.06, 4.35, 5.0), m["hull_round"], at=(0, ky + 0.2, 0))
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
        engine(s, m, 0.5, 4.15, 5.45, dx=dx, dz=0.45, n=ROUND, mirror=dx > 0)
        engine_details(s, m, 0.5, 4.15, 5.45, dx=dx, dz=0.45, n=ROUND, mirror=dx > 0)
    engine(s, m, 0.3, 3.75, 4.75, dx=2.05, dz=1.0, n=24, mirror=True)
    engine_details(s, m, 0.3, 3.75, 4.75, dx=2.05, dz=1.0, n=24, mirror=True)
    details(s, m, ky)
    fine(s, m, ky, ledge)
    return s.finish()
