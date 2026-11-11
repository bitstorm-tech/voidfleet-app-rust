"""Titan Carrier (Schlachtschiff), Clean Toon, detailreiche Fassung nach der Iron Dreadnought (docs/stilregeln.md).

Breiter Rumpf unter einem großen Flugdeck: Deckrand in Stahlblau, dunkle Landebahn mit zwei Lichtbändern, drei geparkte
Drohnen im Violett der Drohne, Insel achtern an Steuerbord aus drei Blöcken mit Fensterband und Mast. An beiden Flanken
unter dem Deck ein Hangardeck mit zwei offenen Toren (stahlblauer Rahmen, dunkle Rückwand, Lichtband). Vor dem Deck fällt
der Bug als Schräge ab, darin der violette Reaktorkern als flache Kuppel in einem dunklen Rahmen. Unten Schürzen und
Kiel, achtern vier Düsen.
Detailzonen mit Zweck, dazwischen ruhige Flächen (Deck zwischen Aufzügen und Bug, Mitte der Landebahn, Flächen der
Insel, Hangarwände). Was auf dem Deck liegt, liegt flach auf (Relief höchstens 0,02, die Kartenansicht kommt von oben):
- Bug und Start: Katapultschienen auf der Landebahn unter der startbereiten Drohne, Bugschild unter dem Reaktor,
  RCS-Blöcke an den vorderen Rumpfecken
- Deck: Aufzugsplattformen über den vorderen Hangartoren (eine trägt eine geparkte Drohne), Mittellinie und
  Aufsetzschwelle am Heckende der Landebahn
- Insel: Flugleitstand mit Fensterband zum Deck und Blende, Luke zum Deck, Mast mit Radarplatte und Querträger
- Hangars: Torrahmen mit vorgezogener Landeklappe, halb hochgefahrene Rolltore, darunter das Licht aus dem Hangar
- Heck: Gehäusesegment und Düsenkranz je Düse (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import math

import palette
import vf
from parts import engine, engine_details, rcs, sym
from vf import Ship, box, grow, loft, ngon, plate, post, sec, sphere, wedge

# Flugdeck (x, y), Steuerbordhälfte von vorn nach achtern; gespiegelt ergibt das den ganzen Umriss
DECK = [(0.95, -3.45), (2.0, -2.35), (2.55, -0.4), (2.55, 2.75), (2.15, 3.85)]
CROWN = math.tan(math.radians(3))  # Neigung der Deckhälften: Auf dem Brett trennt das Kantenlicht sie an der Mittellinie
TILT = math.degrees(CROWN)  # dieselbe Neigung als Drehung um die Längsachse (Grad) für Platten auf der Steuerbordhälfte


def hull(w, h):
    """Rumpfprofil: flaches Deck, senkrechte Flanke, steile untere Fase (in der Karte halbhell), flacher Boden."""
    return [(w / 2, 0.0), (w / 2, h / 2), (-w / 2, h / 2), (-w / 2, 0.0), (-0.3 * w, -h / 2), (0.3 * w, -h / 2)]


def crowned(half, ridge, bottom, k):
    """Deckplatte mit flachem First: Umriss aus der Steuerbordhälfte `half` (x, y), oben zwei Ebenen
    z = ridge - k·|x|, unten flach bei `bottom`."""
    return loft([sec(y, [(x, bottom), (x, ridge - k * x), (0.0, ridge), (-x, ridge - k * x), (-x, bottom)]) for x, y in half])


def mark(s, mat, x0, x1, y0, y1, surface, relief=0.02):
    """Flach aufliegende Platte auf beiden Deckhälften (gespiegelt), x0..x1 auf der Steuerbordhälfte (x > 0), y0..y1;
    `surface` ist die Höhe der Fläche darunter auf der Mittellinie. Steckt 0,01 in ihr und steht `relief` heraus."""
    cx = (x0 + x1) / 2
    s.add(box(x1 - x0, y1 - y0, relief + 0.01), mat, at=(cx, (y0 + y1) / 2, surface - CROWN * cx + (relief - 0.01) / 2),
          rot=(0, TILT, 0), mirror=True)


def stripe(s, mat, w, y0, y1, surface, relief=0.02):
    """Flach aufliegender Streifen quer über die Mittellinie, w breit, y0..y1, mit dem First der Fläche `surface`."""
    s.add(crowned([(w / 2, y0), (w / 2, y1)], surface + relief, surface - 0.01, CROWN), mat)


def hangar(s, m, y0, y1, doors):
    """Hangardeck an der Steuerbordflanke (gespiegelt), y0 bis y1, mit offenen Toren (y-Bereiche): dunkle Rückwand,
    Rahmen in Stahlblau, in halber Torhöhe ein Lichtband (von oben wie von unten zu sehen), darüber das halb
    hochgefahrene Rolltor (`armor`), etwas hinter der Wand, und um jedes Tor ein Rahmen (`armor`), unten als
    Landeklappe vorgezogen."""
    xi, xo, back = 1.6, 2.42, 2.26  # innen, Außenkante des Rahmens, Rückwand der Tore
    z0, z1, zd0, zd1 = -0.42, 0.47, -0.24, 0.27  # Hangardeck unten/oben, Tor unten/oben
    length, mid = y1 - y0, (y0 + y1) / 2
    s.add(box(back - xi, length - 0.06, z1 - z0 - 0.04), m["dark"], at=((xi + back) / 2, mid, (z0 + z1) / 2), mirror=True)
    w = xo - xi
    s.add(box(w, length, z1 - zd1), m["accent"], at=(xi + w / 2, mid, (zd1 + z1) / 2), mirror=True)
    s.add(box(w, length, zd0 - z0), m["accent"], at=(xi + w / 2, mid, (z0 + zd0) / 2), mirror=True)
    edges = [y0] + [y for door in doors for y in door] + [y1]
    for a, b in zip(edges[::2], edges[1::2]):
        s.add(box(w, b - a, zd1 - zd0), m["accent"], at=(xi + w / 2, (a + b) / 2, (zd0 + zd1) / 2), mirror=True)
    fw = 0.07  # Breite des Torrahmens, oben 0,1 unter dem Deckrand
    for a, b in doors:
        mid = (a + b) / 2
        s.add(box(0.05, b - a - 0.14, 0.07), m["glow"], at=(back + 0.02, mid, (zd0 + zd1) / 2), mirror=True)
        s.add(box(0.06, b - a, 0.2), m["armor"], at=(xo - 0.09, mid, zd1 - 0.1), mirror=True)
        # Torrahmen, 0,04 vor der Wand; unten als Landeklappe weiter vorgezogen
        s.add(box(0.06, b - a + 2 * fw, fw), m["armor"], at=(xo + 0.01, mid, zd1 + fw / 2), mirror=True)
        s.add(box(0.16, b - a + 2 * fw, fw), m["armor"], at=(xo + 0.06, mid, zd0 - fw / 2), mirror=True)
        for y in (a - fw / 2, b + fw / 2):
            s.add(box(0.06, fw, zd1 - zd0), m["armor"], at=(xo + 0.01, y, (zd0 + zd1) / 2), mirror=True)


def drone(s, m, at, yaw):
    """Geparkte Drohne, flach: Pfeilflügel im Violett der Drohne, darauf ein schmaler Rumpf."""
    x, y, z = at
    wing = [(0.0, -0.5), (0.48, 0.22), (0.22, 0.32), (0.0, 0.18), (-0.22, 0.32), (-0.48, 0.22)]
    s.add(plate(wing, 0.07, "xy"), m["drone"], at=(x, y, z + 0.02), rot=(0, 0, yaw))
    s.add(wedge(0.24, 0.7, 0.16, front=0.45, taper=0.6), m["hull"], at=(x, y, z + 0.1), rot=(0, 0, yaw))


def build():
    s = Ship("titan_carrier")
    m = vf.materials(s.name, "battleship")
    m["reactor"] = vf.light(f"{s.name}_reactor", palette.PLASMA)
    m["drone"] = vf.toon(f"{s.name}_drone", palette.DRONE_ACCENT)

    # ---------------------------------------------------------------- Rumpf, vorn als Schräge unter die Deckskante
    s.add(loft([
        sec(-4.1, hull(1.6, 0.5), dz=-0.5),
        sec(-3.45, hull(2.3, 1.3)),
        sec(-2.4, hull(3.2, 1.3)),
        sec(-0.6, hull(3.8, 1.4)),
        sec(2.4, hull(3.8, 1.4)),
        sec(3.5, hull(3.1, 1.15)),
        sec(3.85, hull(2.5, 0.95)),
    ]), m["hull"], at=(0, 0, -0.2))

    # ---------------------------------------------------------------- Flugdeck: stahlblaue Grundplatte als Rand,
    # Deckfläche, dunkle Landebahn mit zwei Lichtbändern
    k = CROWN
    inner = sorted([p for p in grow(sym(DECK), -0.2) if p[0] > 1e-6], key=lambda p: p[1])
    ridge = 0.68 + DECK[2][0] * k
    s.add(crowned(DECK, ridge, 0.44, k), m["accent"])
    s.add(crowned(inner, ridge + 0.05, ridge - 0.2, k), m["hull"])
    deck = ridge + 0.05  # Deckfläche auf der Mittellinie; bei x liegt sie bei deck - k·|x|
    top = deck + 0.04
    s.add(crowned([(0.75, -3.0), (0.75, 3.4)], top, top - 0.12, k), m["dark"])
    s.add(box(0.07, 5.0, 0.03), m["glow"], at=(0.58, -0.5, top - k * 0.58 + 0.005), rot=(0, TILT, 0), mirror=True)

    # ---------------------------------------------------------------- Deck, flach aufliegend: Katapultschienen unter
    # der startbereiten Drohne, Mittellinie, Aufsetzschwelle, Aufzugsplattformen über den vorderen Hangartoren
    mark(s, m["armor"], 0.24, 0.32, -2.95, -1.15, top)
    for y in (-0.55, 0.25, 1.05):
        stripe(s, m["hull"], 0.06, y - 0.2, y + 0.2, top)
    for y in (2.55, 2.85, 3.15):
        stripe(s, m["accent"], 1.2, y - 0.07, y + 0.07, top)
    mark(s, m["dark"], 0.92, 2.02, -1.2, -0.1, deck, relief=0.01)
    mark(s, m["armor"], 1.0, 1.94, -1.12, -0.18, deck)

    # ---------------------------------------------------------------- Geparkte Drohnen: eine auf dem Aufzug, eine
    # daneben, eine startbereit auf den Katapultschienen
    for x, y, z, yaw in ((-1.47, -0.65, deck - k * 1.47 + 0.02, 28), (-1.47, 0.65, deck - k * 1.47, 28), (0.0, -2.3, top, 0)):
        drone(s, m, (x, y, z), yaw)

    # ---------------------------------------------------------------- Insel achtern an Steuerbord: drei Blöcke mit
    # schräger Front, Fensterband vorn, Flugleitstand zum Deck, Luke, Mast mit Radarplatte, Querträger und Licht
    ix, iy, z = 1.6, 2.45, deck - k * 2.25
    for w, l, h, dy in ((1.3, 2.2, 0.55, 0.0), (1.0, 1.6, 0.5, 0.1), (0.72, 1.0, 0.42, 0.08)):
        s.add(wedge(w, l, h), m["hull"], at=(ix, iy + dy, z + h / 2))
        z += h
    s.add(box(0.6, 0.04, 0.1), m["window"], at=(ix, iy + dy - l / 2 + 0.14, z - 0.18))
    # Flugleitstand an der Backbordseite des mittleren Blocks, mit Blende über dem Fensterband
    s.add(box(0.4, 0.7, 0.32), m["hull"], at=(0.97, 2.85, 1.58))
    s.add(box(0.41, 0.78, 0.05), m["armor"], at=(0.915, 2.85, 1.765))
    s.add(box(0.04, 0.55, 0.1), m["window"], at=(0.77, 2.85, 1.63))
    s.add(box(0.04, 0.32, 0.4), m["dark"], at=(ix - 0.655, 3.05, deck - k * 0.95 + 0.2))
    s.add(post(10, 0.09, 0.06, z, z + 0.68), m["hull_round"], at=(ix, iy + 0.25, 0))
    s.add(box(0.6, 0.07, 0.07), m["hull"], at=(ix, iy + 0.25, z + 0.5))
    s.add(box(0.44, 0.05, 0.18), m["armor"], at=(ix, iy + 0.16, z + 0.27), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(ix, iy + 0.25, z + 0.72))

    # ---------------------------------------------------------------- Hangardecks an den Flanken unter dem Deck
    hangar(s, m, -2.45, 2.95, [(-2.1, -0.1), (0.35, 2.6)])

    # ---------------------------------------------------------------- Bug: Reaktorkern in dunklem Rahmen, Bugschild
    # darunter, RCS an den vorderen Rumpfecken
    s.add(plate(ngon(8, 0.54, phase=0.3927), 0.16, "xz"), m["dark"], at=(0, -3.8, 0.02), rot=(-36, 0, 0), scale=(1.35, 1, 1))
    s.add(sphere(0.4, 2), m["reactor"], at=(0, -3.82, 0.04), rot=(-36, 0, 0), scale=(1.35, 0.6, 1))
    s.add(plate([(0.86, -0.72), (0.86, -0.49), (-0.86, -0.49), (-0.86, -0.72), (-0.52, -0.99), (0.52, -0.99)], 0.08, "xz"),
          m["armor"], at=(0, -4.12, 0))
    rcs(s, m, (1.33, -2.9, 0.12), -1)

    # ---------------------------------------------------------------- Schürzen unter dem Hangardeck und Kiel
    s.add(plate([(0.7, -3.0), (1.75, -2.6), (2.25, -1.8), (2.25, 2.7), (1.8, 3.3), (0.7, 3.3)], 0.22, "xy"), m["armor"], at=(0, 0, -0.53), mirror=True)
    s.add(plate([(-3.7, 0.0), (3.0, 0.0), (3.15, -0.3), (0.5, -0.4), (-2.6, -0.45), (-3.3, -0.28)], 0.22, "yz"), m["dark"], at=(0, 0, -0.78))

    # ---------------------------------------------------------------- Vier Heckdüsen mit Gehäusesegment und Düsenkranz
    for r, y0, y1, dx in ((0.36, 3.45, 4.45, 0.6), (0.28, 3.4, 4.25, 1.4)):
        engine(s, m, r, y0, y1, dx=dx, dz=-0.12, mirror=True)
        engine_details(s, m, r, y0, y1, dx=dx, dz=-0.12, mirror=True)
    return s.finish()
