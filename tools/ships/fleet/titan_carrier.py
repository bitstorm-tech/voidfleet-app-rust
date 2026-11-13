"""Titan Carrier (Schlachtschiff), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste Primär- und
Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

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

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an der Insel, Flak am Deckrand,
Peitschenantennen, Schüssel und Gerätekasten auf der Insel, Geländer und Leitern an der Insel, Geländer und
Betankungsleitung am Deckrand, Fangseile mit Pfosten vor der Aufsetzschwelle, zwei Deckschlepper, Kisten vor der Insel,
Kühlrippen an den Düsen, Positionslichter an den Deckecken.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen, wie groß das Schiff ist, und wo man es betritt,
wartet und verschraubt). Auf dem Flugdeck, der größten ruhigen Fläche des Spiels: Spur und verschraubte Schienen des
Katapults mit eingefahrenem Strahlabweiser, Zurrpunkte auf den Aufzügen, Zurrketten an den geparkten Drohnen, ein
freier Stellplatz, Umlenkrollen der Fangseile, Luken, Munitionskästen an der Flak, Poller auf dem Deckrand. Am
Deckrand Rettungsinseln und die Schlauchtrommel der Betankung; an der Insel Fenstersimse, Türblatt mit Handrad,
Streben unter dem Flugleitstand, Dachaggregat und Radom, Steigeisen am Mast; an den Hangars Lamellen am Rolltor,
Schrauben in den Torpfosten, Steigeisen und Lüfter; am Bug Schrauben im Rahmen des Reaktorkerns und Kühlleitungen;
Stellstreben an den Düsenglocken. Dazu Beschriftung als flache Geometrie: Seitenlinien der Landebahn, Warnbänder an
Aufzügen und Landeklappen, die Kennnummer groß auf dem Deck achtern (lesbar von hinten oben) und an beiden Deckkanten.
"""

import math

import palette
import vf
from parts import axle, beam, bolts, engine, engine_details, flak, hatch, number, rail, rcs, stripes, sym, vent, windows
from vf import Ship, box, chamfer, dish, grow, loft, ngon, pipe, plate, post, sec, segment, sphere, tube, wedge

# Flugdeck (x, y), Steuerbordhälfte von vorn nach achtern; gespiegelt ergibt das den ganzen Umriss
DECK = [(0.95, -3.45), (2.0, -2.35), (2.55, -0.4), (2.55, 2.75), (2.15, 3.85)]
CROWN = math.tan(math.radians(3))  # Neigung der Deckhälften: Auf dem Brett trennt das Kantenlicht sie an der Mittellinie
TILT = math.degrees(CROWN)  # dieselbe Neigung als Drehung um die Längsachse (Grad) für Platten auf der Steuerbordhälfte
ENGINES = ((0.36, 3.45, 4.45, 0.6), (0.28, 3.4, 4.25, 1.4))  # Heckdüsen (r, y0, y1, dx), gespiegelt
ENGINE_Z = -0.12
JET = 24  # Segmente der Heckdüsen
DOORS = [(-2.1, -0.1), (0.35, 2.6)]  # Hangartore (y-Bereiche)


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
    s.add(chamfer(plate(wing, 0.07, "xy"), 0.015), m["drone"], at=(x, y, z + 0.02), rot=(0, 0, yaw))
    s.add(chamfer(wedge(0.24, 0.7, 0.16, front=0.45, taper=0.6), 0.02), m["hull"], at=(x, y, z + 0.1), rot=(0, 0, yaw))


def details(s, m, deck, top, ridge, z):
    """Kleinteile (Modulkommentar); `deck`, `top` und `ridge` sind die Höhen von Deckfläche, Landebahn und Deckrand auf
    der Mittellinie, `z` ist das Dach der Insel."""
    k = CROWN
    ix, roof1, roof2 = 1.6, z - 0.92, z - 0.42  # Mitte der Insel, Dächer ihrer beiden unteren Blöcke

    # ---------------------------------------------------------------- Fensterreihen an der Insel
    def port_windows(x, ys, wz):
        """Fenster an einer Wand, die nach Backbord schaut (`parts.windows` kennt nur die Steuerbordwand)."""
        for y in ys:
            s.add(box(0.03, 0.07, 0.045), m["window"], at=(x - 0.004, y, wz))

    row = [2.3 + 0.12 * j for j in range(10)]
    windows(s, m, [(2.25, y, roof1 - 0.2) for y in row], mirror=False)
    windows(s, m, [(2.25, y, roof1 - 0.38) for y in row[1::2]], mirror=False)
    port_windows(0.95, row[:5], roof1 - 0.2)
    windows(s, m, [(1.5 + 0.2 * j, 3.55, roof1 - 0.2) for j in range(4)], out="y")
    windows(s, m, [(2.1, 2.45 + 0.12 * j, roof2 - 0.2) for j in range(7)], mirror=False)
    windows(s, m, [(1.3 + 0.2 * j, 3.35, roof2 - 0.2) for j in range(3)], out="y")
    bridge = [2.5 + 0.12 * j for j in range(4)]
    windows(s, m, [(1.96, y, z - 0.18) for y in bridge], mirror=False)
    port_windows(1.24, bridge, z - 0.18)

    # ---------------------------------------------------------------- Flak am Deckrand: Bug, vorderes Viertel, Heck
    for (x, y), yaw in (((1.45, -2.75), 25), ((2.07, -1.6), 60), ((1.95, 3.7), 150)):
        flak(s, m, (x, y, ridge - k * x), yaw)

    # ---------------------------------------------------------------- Antennen auf der Insel
    for x, y in ((1.35, 2.9), (1.85, 2.95)):
        s.add(post(6, 0.012, 0.005, z - 0.02, z + 0.55), m["dark_round"], at=(x, y, 0))
    for dx in (-0.27, 0.27):
        s.add(post(6, 0.012, 0.006, z + 0.5, z + 0.9), m["dark_round"], at=(ix + dx, 2.7, 0))
    # Schüssel und Gerätekasten mit Nadel auf dem mittleren Block, hinter dem Brückenblock
    s.add(post(8, 0.03, 0.025, roof2 - 0.02, roof2 + 0.14), m["dark_round"], at=(1.85, 3.15, 0))
    s.add(dish(0.13, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(1.85, 3.15, roof2 + 0.13), rot=(-40, 0, 150))
    s.add(box(0.14, 0.2, 0.12), m["armor"], at=(1.35, 3.15, roof2 + 0.06))
    s.add(post(6, 0.012, 0.006, roof2 + 0.1, roof2 + 0.5), m["dark_round"], at=(1.35, 3.2, 0))

    # ---------------------------------------------------------------- Geländer und Leitern an der Insel
    def ladder(x, y, z0, z1):
        """Leiter an einer Rückwand bei y von z0 bis z1."""
        for dx in (-0.05, 0.05):
            s.add(box(0.014, 0.014, z1 - z0), m["dark"], at=(x + dx, y + 0.007, (z0 + z1) / 2))
        for j in range(int((z1 - z0) / 0.08)):
            s.add(box(0.1, 0.014, 0.014), m["dark"], at=(x, y + 0.007, z0 + 0.05 + 0.08 * j))

    # 0,045 von der Dachkante: Die Blöcke sind gefast, an der Kante stünden die Pfosten in der Luft
    rail(s, m, (2.205, 2.2, roof1), (2.205, 3.505, roof1), 5, mirror=False)
    rail(s, m, (1.35, 3.505, roof1), (2.205, 3.505, roof1), 4, mirror=False)
    rail(s, m, (2.055, 2.4, roof2), (2.055, 3.305, roof2), 4, mirror=False)
    rail(s, m, (1.13, 3.305, roof2), (1.85, 3.305, roof2), 4, mirror=False)
    ladder(1.2, 3.55, deck - k * 1.2, roof1)
    ladder(1.95, 3.35, roof1, roof2)

    # ---------------------------------------------------------------- Deckrand mittschiffs: Geländer, Betankungsleitung
    # mit Ventilkasten
    rz = ridge - k * 2.5
    rail(s, m, (2.5, -0.3, rz), (2.5, 1.2, rz), 5)
    rail(s, m, (-2.5, 1.6, rz), (-2.5, 2.7, rz), 4, mirror=False)
    pz = ridge - k * 2.39
    s.add(segment((2.39, -0.25, pz + 0.02), (2.39, 1.2, pz + 0.02), 0.025, 6), m["steel"], mirror=True)
    s.add(box(0.12, 0.16, 0.1), m["armor"], at=(2.39, 1.25, pz + 0.04), mirror=True)

    # ---------------------------------------------------------------- Fangseile mit Pfosten vor der Aufsetzschwelle
    for y in (1.5, 1.8, 2.1):
        mark(s, m["armor"], 0.0, 0.81, y - 0.008, y + 0.008, top, relief=0.03)
        s.add(box(0.06, 0.06, 0.1), m["dark"], at=(0.81, y, deck - k * 0.81 + 0.04), mirror=True)

    # ---------------------------------------------------------------- Deckschlepper bei den geparkten Drohnen und vor
    # der Insel, Kisten vor der Insel
    def tug(x, y, yaw):
        """Deckschlepper: flacher Wagen mit Fahrerhaus und Schleppstange."""
        g = Ship("tug")
        g.add(chamfer(box(0.2, 0.32, 0.09), 0.015), m["armor"], at=(0, 0, 0.035))
        g.add(chamfer(box(0.16, 0.12, 0.08), 0.015), m["dark"], at=(0, 0.08, 0.11))
        g.add(box(0.03, 0.2, 0.03), m["dark"], at=(0, -0.24, 0.02))
        s.place(g, at=(x, y, deck - k * abs(x)), rot=(0, 0, yaw))
        g.bm.free()

    tug(-2.1, 0.0, 28)
    tug(1.75, 0.75, -15)
    for (x, y), (w, l, h), mat in (((1.3, 1.22), (0.22, 0.2, 0.14), "armor"), ((1.58, 1.2), (0.2, 0.24, 0.18), "dark"),
                                   ((1.85, 1.24), (0.16, 0.16, 0.12), "armor")):
        s.add(chamfer(box(w, l, h), 0.015), m[mat], at=(x, y, deck - k * x + h / 2 - 0.01))

    # ---------------------------------------------------------------- Kühlrippen an den Düsen
    for r, y0, y1, dx in ENGINES:
        for t in (0.41, 0.455, 0.5):
            y = y0 + (y1 - y0) * t
            s.add(tube(JET, r * 1.14, r * 1.14, y, y + 0.025, dx, ENGINE_Z), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter an den Deckecken
    for x, y in ((0.93, -3.38), (2.1, 3.76)):
        s.add(sphere(0.03, 1), m["glow"], at=(x, y, ridge - k * x + 0.015), mirror=True)


def fine(s, m, deck, top, ridge, z, drones):
    """Feinteile und Beschriftung (Modulkommentar); Höhen wie in `details`, `drones` sind die geparkten Drohnen
    (x, y, z, Drehung)."""
    k = CROWN
    ix, roof1, roof2 = 1.6, z - 0.92, z - 0.42
    flat = (0, TILT, 0)  # legt ein Teil auf die Steuerbordhälfte des Decks

    # ---------------------------------------------------------------- Landezone: Seitenlinien, Umlenkrollen der Fangseile
    # Seitenlinien der Landebahn von den Fangseilen bis zur Aufsetzschwelle; außen an jedem Seilpfosten der verschraubte
    # Deckel der Umlenkrolle
    mark(s, m["paint"], 0.665, 0.69, 1.3, 3.35, top, relief=0.006)
    for y in (1.5, 1.8, 2.1):
        mark(s, m["armor"], 0.85, 0.97, y - 0.07, y + 0.07, deck, relief=0.012)
        bolts(s, m["steel"], [(0.91, y + dy, deck - k * 0.91 + 0.012) for dy in (-0.045, 0.045)], flat, r=0.01, mirror=True)

    # ---------------------------------------------------------------- Katapult: Spur, Schrauben, Strahlabweiser
    # Spur des Schlittens zwischen den Schienen, die Schienen verschraubt; dahinter der eingefahrene Strahlabweiser aus
    # drei Klappen mit Scharnierleiste zur Drohne hin
    stripe(s, m["steel_flat"], 0.035, -2.95, -1.15, top, relief=0.008)
    bolts(s, m["steel"], [(0.28, y, top - k * 0.28 + 0.02) for y in (-2.9, -1.85, -1.55, -1.25)], flat, r=0.012, mirror=True)
    stripe(s, m["armor"], 0.3, -1.08, -0.86, top, relief=0.012)
    mark(s, m["armor"], 0.17, 0.5, -1.08, -0.86, top, relief=0.012)
    stripe(s, m["steel_flat"], 1.0, -1.115, -1.09, top, relief=0.018)

    # ---------------------------------------------------------------- Aufzüge: Warnbänder, Zurrpunkte
    # Warnband an der vorderen und hinteren Kante jedes Aufzugs, neun Zurrpunkte auf der Plattform
    for y in (-1.16, -0.14):
        stripes(s, m, (1.47, y, deck - k * 1.47 + 0.01), 0.05, 1.0, rot=(TILT, 0, 90), mirror=True)
    for x in (1.2, 1.47, 1.74):
        for y in (-0.92, -0.65, -0.38):
            s.add(post(6, 0.018, 0.018, -0.004, 0.005), m["dark"], at=(x, y, deck - k * x + 0.02), rot=flat, mirror=True)

    # ---------------------------------------------------------------- Parkzone: Zurrketten der Drohnen, Kennnummer
    # Jede geparkte Drohne ist an beiden Flügelspitzen aufs Deck gezurrt; achtern an Backbord die Kennnummer, lesbar
    # von hinten oben
    for x, y, dz, yaw in drones:
        c, d = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
        for sx in (-1, 1):
            (ax, ay), (bx, by) = [(x + sx * u * c - v * d, y + sx * u * d + v * c) for u, v in ((0.43, 0.2), (0.6, 0.3))]
            s.add(beam((ax, ay, dz + 0.03), (bx, by, deck - k * abs(bx) - 0.004), 0.012, 0.012), m["steel_flat"])
    number(s, m["paint"], "23", (-1.55, 2.5, deck - k * 1.55), 0.45, (0, TILT, 180))
    # davor ein freier Stellplatz: Eckwinkel und vier Zurrpunkte
    px, py = -1.47, 1.65
    for sx in (-1, 1):
        for sy in (-1, 1):
            x, y = px + sx * 0.42, py + sy * 0.42
            s.add(box(0.14, 0.02, 0.016), m["paint"], at=(x - sx * 0.06, y, deck + k * (x - sx * 0.06) - 0.002), rot=(0, -TILT, 0))
            s.add(box(0.02, 0.12, 0.016), m["paint"], at=(x, y - sy * 0.07, deck + k * x - 0.002), rot=(0, -TILT, 0))
            s.add(post(6, 0.018, 0.018, -0.004, 0.005), m["dark"], at=(px + sx * 0.2, py + sy * 0.2, deck + k * (px + sx * 0.2)), rot=(0, -TILT, 0))

    # ---------------------------------------------------------------- Vordeck: Luken, Munitionskästen, Poller
    # Wartungsluke und je ein Munitionskasten hinter den beiden vorderen Flak, eine weitere Luke beim Deckschlepper vor
    # der Insel; Poller auf dem Deckrand an Bug und Heck
    hatch(s, m, (1.25, -1.9, deck - k * 1.25), 0.3, 0.36, rot=flat, mirror=True)
    hatch(s, m, (1.25, 0.4, deck - k * 1.25), 0.3, 0.36, rot=flat)
    for x, y in ((1.2, -2.55), (1.85, -1.45)):
        g = Ship("locker")
        g.add(chamfer(box(0.12, 0.16, 0.09), 0.012), m["dark"], at=(0, 0, 0.035))
        g.add(box(0.1, 0.14, 0.012), m["armor"], at=(0, 0, 0.082))
        g.add(box(0.012, 0.06, 0.012), m["steel_flat"], at=(0, 0, 0.092))
        s.place(g, at=(x, y, deck - k * x), rot=flat, mirror=True)
        g.bm.free()
    for (ax, ay), (bx, by), ts in ((DECK[0], DECK[1], (0.25, 0.4)), (DECK[3], DECK[4], (0.35, 0.5))):
        length = math.hypot(bx - ax, by - ay)
        nx, ny = -(by - ay) / length, (bx - ax) / length  # nach innen
        for u in ts:
            x, y = ax + (bx - ax) * u + 0.1 * nx, ay + (by - ay) * u + 0.1 * ny
            s.add(post(10, 0.03, 0.036, -0.01, 0.08), m["steel"], at=(x, y, ridge - k * x), mirror=True)

    # ---------------------------------------------------------------- Deckrand: Rettungsinseln, Schlauchtrommel
    # Drei Rettungsinseln auf einem Träger außen an der Deckkante; an der Betankungsleitung hinter dem Ventilkasten die
    # Schlauchtrommel (eckig schattiert, sonst hielte die Verwitterung ihre Scheiben für Mündungen)
    for y in (1.5, 1.68, 1.86):
        s.add(tube(8, 0.045, 0.045, -0.07, 0.07), m["paint"], at=(2.59, y, 0.57), mirror=True)
    s.add(box(0.04, 0.52, 0.03), m["dark"], at=(2.565, 1.68, 0.535), mirror=True)
    pz = ridge - k * 2.39
    s.add(axle(0.05, 0.08, 8), m["armor"], at=(2.39, 1.45, pz + 0.07), mirror=True)
    for dx in (-0.045, 0.045):
        s.add(axle(0.068, 0.012, 8), m["dark"], at=(2.39 + dx, 1.45, pz + 0.07), mirror=True)
    s.add(box(0.11, 0.05, 0.05), m["dark"], at=(2.39, 1.45, pz + 0.015), mirror=True)

    # ---------------------------------------------------------------- Insel: Simse, Schott, Streben, Dach, Mast
    # Fenstersimse unter den oberen Reihen, Türblatt mit Handrad und Scharnieren in der Luke zum Deck, Streben unter
    # dem Flugleitstand, Aggregat und Radom auf dem Brückendach, Steigeisen am Mast
    for x, y0, y1, wz, side in ((2.25, 2.26, 3.42, roof1 - 0.2, 1), (0.95, 2.26, 2.82, roof1 - 0.2, -1), (2.1, 2.41, 3.21, roof2 - 0.2, 1),
                                (1.96, 2.46, 2.9, z - 0.18, 1), (1.24, 2.46, 2.9, z - 0.18, -1)):
        s.add(box(0.03, y1 - y0, 0.014), m["dark"], at=(x + side * 0.008, (y0 + y1) / 2, wz - 0.036))
    s.add(box(0.68, 0.03, 0.014), m["dark"], at=(1.8, 3.558, roof1 - 0.236))
    s.add(box(0.48, 0.03, 0.014), m["dark"], at=(1.5, 3.358, roof2 - 0.236))
    dz = deck - k * 0.95 + 0.2
    s.add(chamfer(box(0.03, 0.24, 0.32), 0.008), m["armor"], at=(0.918, 3.05, dz))
    s.add(tube(6, 0.03, 0.03, 0.0, 0.014), m["steel_flat"], at=(0.906, 3.05, dz + 0.02), rot=(0, 0, 90))
    for dh in (-0.1, 0.1):
        s.add(box(0.012, 0.02, 0.04), m["steel_flat"], at=(0.9, 3.18, dz + dh))
    for y in (2.58, 3.12):
        s.add(segment((1.0, y, roof1 - 0.01), (0.82, y, 1.43), 0.016, 6), m["dark"])
    s.add(chamfer(box(0.2, 0.15, 0.09), 0.015), m["armor"], at=(1.82, 2.5, z + 0.035))
    s.add(box(0.13, 0.09, 0.012), m["dark"], at=(1.82, 2.5, z + 0.083))
    s.add(sphere(0.06, 2), m["steel"], at=(1.38, 2.52, z + 0.02))
    for j in range(6):
        h = 0.08 + 0.065 * j
        s.add(box(0.07, 0.03, 0.012), m["steel_flat"], at=(ix, 2.7 + 0.098 - 0.03 * h / 0.68, z + h))

    # ---------------------------------------------------------------- Hangars: Warnbänder, Rolltore, Schrauben, Steigeisen, Lüfter
    # Warnband an beiden Enden jeder Landeklappe, Lamellen und Griffleiste am Rolltor, Schrauben in den Torpfosten; an
    # der Wand vorn Steigeisen, achtern ein Hangarlüfter. Die Kennnummer steht außen an der Deckkante, auf jeder Seite
    # lesbar (dunkel wie an der Bastion: Auf dem Stahlblau geht die helle Farbe unter)
    for a, b in DOORS:
        for y in (a + 0.2, b - 0.2):
            stripes(s, m, (2.49, y, -0.24), 0.08, 0.4, mirror=True)
        bolts(s, m["steel"], [(2.46, y, bz) for y in (a - 0.035, b + 0.035) for bz in (-0.2, 0.015, 0.23)], "x", mirror=True)
        for bz in (0.12, 0.17, 0.22):
            s.add(box(0.012, b - a - 0.02, 0.014), m["dark"], at=(2.362, (a + b) / 2, bz), mirror=True)
        s.add(box(0.02, b - a - 0.02, 0.018), m["steel_flat"], at=(2.364, (a + b) / 2, 0.082), mirror=True)
    for j in range(8):
        s.add(box(0.03, 0.08, 0.012), m["steel_flat"], at=(2.428, -2.31, -0.2 + 0.075 * j), mirror=True)
    vent(s, m, m["accent"], (2.42, 2.81, 0.02), 0.22, 0.24, slats=2, rot=(0, 90, 0), mirror=True)
    number(s, m["dark"], "23", (2.55, 0.5, 0.56), 0.14, (90, 0, 90))
    number(s, m["dark"], "23", (-2.55, 0.5, 0.56), 0.14, (90, 0, -90))

    # ---------------------------------------------------------------- Bug: Rahmen des Reaktorkerns, Kühlleitungen
    # Schrauben in den Ecken des Rahmens; beidseits eine Kühlleitung auf der Bugschräge, die unter dem Deck im Rumpf
    # verschwindet, mit zwei Schellen
    g = Ship("reactor")
    bolts(g, m["steel_flat"], [(1.35 * 0.47 * math.cos(a), -0.08, 0.47 * math.sin(a)) for a in (0.3927 + math.pi / 4 * j for j in range(8))],
          "-y", r=0.016)
    s.place(g, at=(0, -3.8, 0.02), rot=(-36, 0, 0))
    g.bm.free()

    def bow(y, up):
        """Punkt auf der Bugschräge bei x = 0,8, um `up` über ihr."""
        return (0.8, y - 0.811 * up, 0.45 - (-3.45 - y) * 0.9 / 0.65 + 0.585 * up)

    s.add(pipe([bow(-3.92, 0.022), bow(-3.52, 0.022), bow(-3.46, -0.05)], 0.028, 6), m["steel"], mirror=True)
    for y in (-3.82, -3.62):
        s.add(box(0.1, 0.04, 0.07), m["armor"], at=bow(y, 0.02), rot=(54.2, 0, 0), mirror=True)

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    for r, y0, y1, dx in ENGINES:
        ya = y0 + 0.6 * (y1 - y0)
        for a in range(0, 360, 60):
            c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
            s.add(segment((dx + 1.1 * r * c, ya, ENGINE_Z + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, ENGINE_Z + 0.93 * r * d), 0.014, 6),
                  m["steel"], mirror=True)


def build():
    s = Ship("titan_carrier")
    m = vf.materials(s.name, "battleship")
    m["reactor"] = vf.light(f"{s.name}_reactor", palette.PLASMA)
    m["drone"] = vf.toon(f"{s.name}_drone", palette.DRONE_ACCENT)

    # ---------------------------------------------------------------- Rumpf, vorn als Schräge unter die Deckskante
    s.add(chamfer(loft([
        sec(-4.1, hull(1.6, 0.5), dz=-0.5),
        sec(-3.45, hull(2.3, 1.3)),
        sec(-2.4, hull(3.2, 1.3)),
        sec(-0.6, hull(3.8, 1.4)),
        sec(2.4, hull(3.8, 1.4)),
        sec(3.5, hull(3.1, 1.15)),
        sec(3.85, hull(2.5, 0.95)),
    ]), 0.05), m["hull"], at=(0, 0, -0.2))

    # ---------------------------------------------------------------- Flugdeck: stahlblaue Grundplatte als Rand,
    # Deckfläche, dunkle Landebahn mit zwei Lichtbändern
    k = CROWN
    inner = sorted([p for p in grow(sym(DECK), -0.2) if p[0] > 1e-6], key=lambda p: p[1])
    ridge = 0.68 + DECK[2][0] * k
    s.add(chamfer(crowned(DECK, ridge, 0.44, k), 0.04), m["accent"])
    s.add(chamfer(crowned(inner, ridge + 0.05, ridge - 0.2, k), 0.02), m["hull"])
    deck = ridge + 0.05  # Deckfläche auf der Mittellinie; bei x liegt sie bei deck - k·|x|
    top = deck + 0.04
    s.add(chamfer(crowned([(0.75, -3.0), (0.75, 3.4)], top, top - 0.12, k), 0.015), m["dark"])
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
    drones = ((-1.47, -0.65, deck - k * 1.47 + 0.02, 28), (-1.47, 0.65, deck - k * 1.47, 28), (0.0, -2.3, top, 0))
    for x, y, dz, yaw in drones:
        drone(s, m, (x, y, dz), yaw)

    # ---------------------------------------------------------------- Insel achtern an Steuerbord: drei Blöcke mit
    # schräger Front, Fensterband vorn, Flugleitstand zum Deck, Luke, Mast mit Radarplatte, Querträger und Licht
    ix, iy, z = 1.6, 2.45, deck - k * 2.25
    for w, l, h, dy in ((1.3, 2.2, 0.55, 0.0), (1.0, 1.6, 0.5, 0.1), (0.72, 1.0, 0.42, 0.08)):
        s.add(chamfer(wedge(w, l, h), 0.03), m["hull"], at=(ix, iy + dy, z + h / 2))
        z += h
    s.add(box(0.6, 0.04, 0.1), m["window"], at=(ix, iy + dy - l / 2 + 0.14, z - 0.18))
    # Flugleitstand an der Backbordseite des mittleren Blocks, mit Blende über dem Fensterband
    s.add(chamfer(box(0.4, 0.7, 0.32), 0.02), m["hull"], at=(0.97, 2.85, 1.58))
    s.add(box(0.41, 0.78, 0.05), m["armor"], at=(0.915, 2.85, 1.765))
    s.add(box(0.04, 0.55, 0.1), m["window"], at=(0.77, 2.85, 1.63))
    s.add(box(0.04, 0.32, 0.4), m["dark"], at=(ix - 0.655, 3.05, deck - k * 0.95 + 0.2))
    s.add(post(16, 0.09, 0.06, z, z + 0.68), m["hull_round"], at=(ix, iy + 0.25, 0))
    s.add(box(0.6, 0.07, 0.07), m["hull"], at=(ix, iy + 0.25, z + 0.5))
    s.add(box(0.44, 0.05, 0.18), m["armor"], at=(ix, iy + 0.16, z + 0.27), rot=(-15, 0, 0))
    s.add(sphere(0.08, 1), m["glow"], at=(ix, iy + 0.25, z + 0.72))

    # ---------------------------------------------------------------- Hangardecks an den Flanken unter dem Deck
    hangar(s, m, -2.45, 2.95, DOORS)

    # ---------------------------------------------------------------- Bug: Reaktorkern in dunklem Rahmen, Bugschild
    # darunter, RCS an den vorderen Rumpfecken
    s.add(chamfer(plate(ngon(8, 0.54, phase=0.3927), 0.16, "xz"), 0.03), m["dark"], at=(0, -3.8, 0.02), rot=(-36, 0, 0), scale=(1.35, 1, 1))
    s.add(sphere(0.4, 3), m["reactor"], at=(0, -3.82, 0.04), rot=(-36, 0, 0), scale=(1.35, 0.6, 1))
    s.add(chamfer(plate([(0.86, -0.72), (0.86, -0.49), (-0.86, -0.49), (-0.86, -0.72), (-0.52, -0.99), (0.52, -0.99)], 0.08, "xz"), 0.02),
          m["armor"], at=(0, -4.12, 0))
    rcs(s, m, (1.33, -2.9, 0.12), -1)

    # ---------------------------------------------------------------- Schürzen unter dem Hangardeck und Kiel
    s.add(chamfer(plate([(0.7, -3.0), (1.75, -2.6), (2.25, -1.8), (2.25, 2.7), (1.8, 3.3), (0.7, 3.3)], 0.22, "xy"), 0.03), m["armor"], at=(0, 0, -0.53),
          mirror=True)
    s.add(chamfer(plate([(-3.7, 0.0), (3.0, 0.0), (3.15, -0.3), (0.5, -0.4), (-2.6, -0.45), (-3.3, -0.28)], 0.22, "yz"), 0.04), m["dark"],
          at=(0, 0, -0.78))

    # ---------------------------------------------------------------- Vier Heckdüsen mit Gehäusesegment und Düsenkranz
    for r, y0, y1, dx in ENGINES:
        engine(s, m, r, y0, y1, dx=dx, dz=ENGINE_Z, n=JET, mirror=True)
        engine_details(s, m, r, y0, y1, dx=dx, dz=ENGINE_Z, n=JET, mirror=True)
    details(s, m, deck, top, ridge, z)
    fine(s, m, deck, top, ridge, z, drones)
    return s.finish()
