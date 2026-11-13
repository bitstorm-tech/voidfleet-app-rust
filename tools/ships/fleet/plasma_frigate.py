"""Plasma Frigate (Kreuzer, im Spiel der Plasmakreuzer), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

Primärformen: gedrungener runder Rumpf, dessen Bug die Plasmakanone ist: dunkler Lagerkragen, Stahllauf mit drei breiten
violetten Leuchtringen, Mündung mit Plasmakern. Vier runde Gondeln an kurzen Trägern, je mit Nase in Kornblumenblau und
blauem Leuchtring dahinter. Auf dem Rücken ein erhöhtes Deck mit kleiner Brücke (Fensterband, Mast mit Licht) und
achtern der Reaktorwulst in Kornblumenblau mit violettem Band. Sechs Düsen (vier Gondeln, zwei am Heck).
Detailzonen mit Zweck, grob, weil das Schiff im Spiel als Paar zu 0,72 steht; dazwischen ruhige Flächen (Rumpfflanken
vor und hinter dem Tragring, Rückendeck, Reaktor, Gondeln):
- Bug: Panzerkappe um die Rumpfspitze, aus der die Kanone wächst
- Mittschiff: Tragring um den Rumpf, an dem die Träger der Gondeln hängen, und an jeder Gondel eine Schelle
- Rücken: Luke hinter der Brücke, Mast mit Radarplatte
- Heck: Gehäusesegment und Düsenkranz an allen sechs Düsen (`parts.engine_details`)
- Unterseite: Kinn mit der Kondensatorbank der Kanone und Lüfter nach vorn, aus dem der Kiel wächst; Tragring und
  Schellen laufen unten durch
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern an der Brücke und an der Wand des
Rückendecks, Flak vor der Brücke, Geländer, Leiter, Querträger mit Nadelantennen am Mast, Schüssel und Gerätekasten auf
dem Brückendach, Leitungen vom Rückendeck in den Reaktor, Kisten, vier Leitungen am Lauf der Kanone unter den
Leuchtringen, Zieloptik auf dem Lagerkragen, Kühlrippen an allen sechs Düsen, Positionslichter an den oberen Gondeln.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme, an einem Paar ab etwa 0,018; sie zeigen, wie groß das Schiff
ist, wie die Kanone gespeist wird und wo man es wartet und verschraubt): an der Kanone Kühlrippen und Schrauben am
Lagerkragen, Fassungen an den Leuchtringen, Schellen an den Leitungen des Laufs; Schrauben an der Bugkappe; je Seite
eine Plasmaleitung über die Schulter, vom Reaktor in den Tragring und von dort in die Bugkappe, mit Schellen; Schrauben
am Tragring; auf dem Rückendeck Scharniere, Griff und Schrauben an der Luke; an der Brücke Fenstersimse und Steigeisen
am Mast; am Reaktor Fassungen am Leuchtband, eine gewölbte Wartungsklappe und zwei Speiseleitungen zu den Heckdüsen; an
den Gondeln Schrauben an den Schellen, Fassungen am Leuchtring und eine gewölbte Wartungsklappe; Stellstreben an den
Düsenglocken. Dazu Beschriftung als flache Geometrie: die Kennnummer 42 auf einem Kennschild an jeder Bugflanke und auf
dem Rückendeck hinter der Brücke (lesbar von hinten oben), ein Warnband auf der Deckschräge hinter der Kanone.
"""

import math

import palette
import vf
from mathutils import Vector
from parts import bolts, cover, engine, engine_details, flak, number, on_ring, rail, ring_at, stripes, vent, windows
from vf import (Ship, box, chamfer, dish, loft, ngon, octagon, pipe, plate, post, revolve, sec, segment, smooth, sphere, superellipse, sweep, tip,
                tube, wedge)

GX = 1.27  # Gondeln: Abstand zur Mittelachse
GZ = (0.43, -0.43)  # Gondeln: Höhen der Achsen
ROUND = 32  # Segmente der großen runden Teile (Rumpf, Kanone, Reaktor)
MID = 24  # Segmente der Gondeln und aller Düsen
RINGS = (-2.96, -3.32, -3.68)  # Leuchtringe der Kanone: hintere Kante, jeder 0,16 lang
# Düsen (Radius, y vorn, y Düsenende, Achse x, Höhen der Achsen): an den Gondeln und am Heck
ENGINES = ((0.26, 1.65, 2.3, GX, GZ), (0.3, 2.4, 3.05, 0.42, (-0.1,)))
SHOULDER = 5  # Ecke des Rumpfquerschnitts (von der Flanke nach oben gezählt), über die die Plasmaleitung läuft


def details(s, m):
    """Kleinteile (Modulkommentar)."""
    deck, roof = 0.68, 0.99  # Rückendeck und Dach der Brücke

    # ---------------------------------------------------------------- Fensterreihen: Brücke, Wand des Rückendecks
    windows(s, m, [(0.35, -0.75 + 0.12 * k, 0.84) for k in range(4)])
    windows(s, m, [(-0.15 + 0.15 * k, -0.25, 0.84) for k in range(3)], out="y")
    windows(s, m, [(0.46, -1.3 + 0.12 * k, 0.585) for k in range(5)])

    # ---------------------------------------------------------------- Flak vor der Brücke
    flak(s, m, (0.18, -1.4, deck), 15)

    # ---------------------------------------------------------------- Brücke: Geländer, Leiter, Antennen, Schüssel, Kasten
    rail(s, m, (0.31, -0.8, roof), (0.31, -0.29, roof), 3)
    rail(s, m, (-0.31, -0.29, roof), (0.31, -0.29, roof), 4, mirror=False)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.32), m["dark"], at=(0.27 + dx, -0.243, 0.835))
    for k in range(4):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.27, -0.243, 0.72 + 0.08 * k))
    s.add(box(0.44, 0.04, 0.04), m["hull"], at=(0, -0.5, 1.34))
    s.add(post(6, 0.012, 0.006, 1.32, 1.72), m["dark_round"], at=(0.2, -0.5, 0), mirror=True)
    s.add(post(8, 0.03, 0.025, roof - 0.02, roof + 0.12), m["dark_round"], at=(-0.23, -0.38, 0))
    s.add(dish(0.1, 0.04, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.23, -0.38, roof + 0.11), rot=(-40, 0, 150))
    s.add(box(0.12, 0.16, 0.1), m["armor"], at=(0.22, -0.4, roof + 0.045))

    # ---------------------------------------------------------------- Rückendeck: Leitungen in den Reaktor, Kisten
    for x in (-0.22, -0.29):
        s.add(sweep([(x, -0.3, deck + 0.02), (x, 0.8, deck + 0.02), (x, 1.02, deck + 0.12)], ngon(6, 0.025)), m["steel"])
    s.add(box(0.18, 0.22, 0.13), m["armor"], at=(0.2, 0.66, deck + 0.06))
    s.add(box(0.14, 0.16, 0.1), m["dark"], at=(0.24, 0.47, deck + 0.045))

    # ---------------------------------------------------------------- Kanone: Leitungen am Lauf, Zieloptik auf dem Kragen
    for a in (45, 135, 225, 315):
        dx, dz = 0.315 * math.cos(math.radians(a)), 0.315 * math.sin(math.radians(a))
        s.add(tube(6, 0.022, 0.022, -3.97, -2.84, dx, dz), m["dark_round"])
    s.add(box(0.12, 0.2, 0.08), m["armor"], at=(0, -2.72, 0.46))
    s.add(box(0.07, 0.02, 0.045), m["dark"], at=(0, -2.825, 0.465))

    # ---------------------------------------------------------------- Kühlrippen an den Düsen (Gondeln und Heck)
    for r, y0, y1, dx, heights in ENGINES:
        for dz in heights:
            for t in (0.37, 0.44):
                y = y0 + (y1 - y0) * t
                s.add(tube(MID, r * 1.14, r * 1.14, y, y + 0.025, dx, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter an den oberen Gondeln
    s.add(sphere(0.03, 1), m["glow"], at=(GX + 0.34, 0.25, GZ[0]), mirror=True)


def fine(s, m, rings, core, pod, band):
    """Feinteile und Beschriftung (Modulkommentar); `rings` sind die Ringe des Rumpfs, `core` die des Reaktors, `pod`
    und `band` die einer Gondel und ihrer Schelle (jeweils um die eigene Achse)."""
    deck = 0.68

    def turn(angle):
        """`rot` für ein Teil, das lokal nach +z schaut und in Richtung `angle` (Grad um die Längsachse) schauen soll."""
        return (0, 90 - angle, 0)

    def lifted(ring, k, up):
        """Ecke k eines Querschnitts, um `up` über der Fläche, und deren Richtung."""
        p, angle = on_ring(ring, k)
        return Vector((p.x + up * math.cos(math.radians(angle)), p.y, p.z + up * math.sin(math.radians(angle)))), angle

    # ---------------------------------------------------------------- Kanone: Kühlrippen, Schrauben, Fassungen, Schellen
    # Am Lagerkragen zwölf Kühlrippen längs (in seiner Kegelneigung, oben bleibt Platz für die Zieloptik) und Schrauben
    # in der Stirnfläche; jeder Leuchtring steckt zwischen zwei dunklen Fassungen; die vier Leitungen am Lauf halten
    # Schellen zwischen den Ringen
    g = Ship("rib")
    g.add(box(0.03, 0.36, 0.02), m["armor"], at=(0.458, -2.6, 0), rot=(0, 0, -math.degrees(math.atan2(0.04, 0.5))))
    for a in range(15, 360, 30):
        s.place(g, rot=(0, -a, 0))
    g.bm.free()
    bolts(s, m["steel_flat"], [(0.385 * math.cos(math.radians(a)), -2.85, 0.385 * math.sin(math.radians(a))) for a in range(0, 360, 30)], "-y",
          r=0.012)
    for y in RINGS:
        s.add(tube(ROUND, 0.405, 0.405, y + 0.02, y - 0.004), m["dark_round"])
        s.add(tube(ROUND, 0.405, 0.405, y - 0.156, y - 0.18), m["dark_round"])
    for a in (45, 135, 225, 315):
        c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
        for y in (-3.22, -3.58):
            s.add(box(0.045, 0.03, 0.07), m["armor"], at=(0.322 * c, y, 0.322 * d), rot=(0, -a, 0))

    # ---------------------------------------------------------------- Bug: Schrauben der Kappe, Kennschild mit Nummer
    # Schrauben am hinteren Rand der Bugkappe über die obere Hälfte; an jeder Bugflanke vor den Gondeln ein Kennschild
    # (eine ebene Platte auf der fast ebenen Flanke) mit der Kennnummer, je Seite für sich gesetzt
    cap = ring_at(rings, -2.29, 0.04)
    for k in range(2, ROUND // 2, 2):
        p, angle = on_ring(cap, k)
        bolts(s, m["steel_flat"], [p], turn(angle))
    x0, x1, x2 = (ring_at(rings, y)[0].x for y in (-2.17, -2.0, -1.83))
    yaw = math.degrees(math.atan2(x2 - x0, 0.34))
    for side in (1, -1):
        g = Ship("plate")
        g.add(chamfer(box(0.34, 0.26, 0.03), 0.01), m["armor"], at=(0, 0, -0.004))
        bolts(g, m["steel_flat"], [(sx * 0.148, sy * 0.108, 0.011) for sx in (-1, 1) for sy in (-1, 1)], r=0.01)
        number(g, m["paint"], "42", (0, 0, 0.011), 0.17, (0, 0, 0))
        s.place(g, at=(side * x1, -2.0, 0.02), rot=(90, 0, side * (90 - yaw)))
        g.bm.free()

    # ---------------------------------------------------------------- Schultern: Plasmaleitungen, Schrauben am Tragring
    # Was die Kanone verfeuert, kommt aus dem Reaktor: je Seite eine Leitung über die Schulter, vom Reaktor in den
    # Tragring und von dessen Vorderkante in die Bugkappe hinter dem Lagerkragen, mit drei Schellen; am Tragring
    # Schrauben an beiden Rändern
    front = [lifted(ring_at(rings, y), SHOULDER, 0.022)[0] for y in (-0.24, -0.8, -1.3, -1.75, -2.1, -2.3)] + [Vector((0.31, -2.42, 0.31))]
    s.add(sweep(front, ngon(6, 0.028)), m["steel"], mirror=True)
    back = [lifted(ring_at(rings, y), SHOULDER, 0.022)[0] for y in (0.74, 1.05)] + [Vector((0.42, 1.3, 0.56))]
    s.add(sweep(back, ngon(6, 0.028)), m["steel"], mirror=True)
    for y in (-0.55, -1.05, -1.55):
        p, angle = lifted(ring_at(rings, y), SHOULDER, 0.022)
        s.add(box(0.085, 0.03, 0.064), m["armor"], at=p, rot=turn(angle), mirror=True)
    for y in (-0.21, 0.71):
        for k in (3, 4):
            p, angle = on_ring(ring_at(rings, y, 0.04), k)
            bolts(s, m["steel_flat"], [p], turn(angle), mirror=True)

    # ---------------------------------------------------------------- Rückendeck: Luke, Nummer, Warnband
    # Die Luke hinter der Brücke bekommt Scharniere, Griff und Schrauben; davor die Kennnummer, lesbar von hinten
    # oben; auf der Deckschräge hinter der Kanone ein Warnband vor der Flak
    lid = deck + 0.03
    bolts(s, m["steel"], [(0.8 * x, 0.25 + 0.8 * y, lid) for x, y in octagon(0.32, 0.28, 0.4)], r=0.01)
    for sx in (-1, 1):
        s.add(tube(8, 0.014, 0.014, -0.04, 0.04), m["steel_flat"], at=(sx * 0.07, 0.385, lid - 0.004), rot=(0, 0, 90))
        s.add(box(0.012, 0.012, 0.02), m["steel_flat"], at=(sx * 0.044, 0.19, lid + 0.006))
    s.add(box(0.1, 0.012, 0.012), m["steel_flat"], at=(0, 0.19, lid + 0.018))
    number(s, m["paint"], "42", (0, -0.07, deck), 0.2, (0, 0, 180))
    stripes(s, m, (0, -1.78, 0.59 + 0.09 * 0.27 / 0.55), 0.06, 0.42, rot=(0, -math.degrees(math.atan2(0.09, 0.55)), 90))

    # ---------------------------------------------------------------- Brücke: Fenstersimse, Steigeisen am Mast
    s.add(box(0.03, 0.48, 0.014), m["dark"], at=(0.358, -0.57, 0.804), mirror=True)
    s.add(box(0.38, 0.03, 0.014), m["dark"], at=(0, -0.242, 0.804))
    for z in (1.03, 1.11, 1.19, 1.27):
        s.add(box(0.07, 0.03, 0.012), m["steel_flat"], at=(0, -0.392 - 0.02 * (z - 0.95), z))

    # ---------------------------------------------------------------- Reaktor: Fassungen, Wartungsklappe, Speiseleitungen
    # Dunkle Fassungen beiderseits des Leuchtbands, eine gewölbte Wartungsklappe mit Schrauben oben auf dem hinteren
    # Kegel, von dort je Seite eine Speiseleitung hinunter ins Gehäuse der Heckdüse
    for y0, y1 in ((1.525, 1.554), (1.746, 1.775)):
        s.add(tube(ROUND, 0.615, 0.615, y0, y1), m["dark_round"], at=(0, 0, 0.5))
    s.add(cover(core, 1.99, 2.3, ROUND // 4 - 1, ROUND // 4 + 1), m["accent_round"], at=(0, 0, 0.5))
    for y in (2.02, 2.27):
        r = ring_at(core, y)[0].x + 0.01
        for a in (82, 98):
            bolts(s, m["steel_flat"], [(r * math.cos(math.radians(a)), y, 0.5 + r * math.sin(math.radians(a)))], turn(a), r=0.01)
    s.add(pipe([(0.3, 2.15, 0.84), (0.42, 2.42, 0.62), (0.42, 2.58, 0.15)], 0.03, 6), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Gondeln: Schrauben, Fassungen, Wartungsklappen
    # Schrauben an beiden Rändern der Schelle (nicht dort, wo der Träger ansetzt), dunkle Fassungen beiderseits des
    # Leuchtrings, hinter der Schelle oben außen eine gewölbte Wartungsklappe mit Schrauben
    for gz in GZ:
        for y in (-0.16, 0.66):
            ring = ring_at(band, y)
            for k in (0, 3, 6, 18, 21):
                p, angle = on_ring(ring, k)
                bolts(s, m["steel_flat"], [(GX + p.x, y, gz + p.z)], turn(angle), r=0.011, mirror=True)
        for y0, y1 in ((-1.085, -1.055), (-1.265, -1.235)):
            s.add(tube(MID, 0.365, 0.365, y0, y1), m["dark_round"], at=(GX, 0, gz), mirror=True)
        s.add(cover(pod, 0.85, 1.15, 2, 4), m["armor_round"], at=(GX, 0, gz), mirror=True)
        for y in (0.88, 1.12):
            ring = ring_at(pod, y, 0.01)
            for k in (2, 4):
                p = ring[k].lerp(ring[3], 0.25)
                bolts(s, m["steel_flat"], [(GX + p.x, y, gz + p.z)], turn(on_ring(ring, k)[1]), r=0.01, mirror=True)

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    for r, y0, y1, dx, heights in ENGINES:
        ya = y0 + 0.6 * (y1 - y0)
        for dz in heights:
            for k in range(6):
                c, d = math.cos(math.pi * (2 * k + 1) / 6), math.sin(math.pi * (2 * k + 1) / 6)
                s.add(segment((dx + 1.1 * r * c, ya, dz + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, dz + 0.93 * r * d), 0.012, 6), m["steel"],
                      mirror=True)


def build():
    s = Ship("plasma_frigate")
    m = vf.materials(s.name, "cruiser")
    m["plasma"] = vf.light(f"{s.name}_plasma", palette.PLASMA)
    # Gedrungener runder Rumpf; Panzerkappe um die Spitze und Tragring als zweite Lage, 0,04 über dem Rumpf
    rings = smooth([
        sec(-2.6, superellipse(0.96, 0.84, ROUND, 3.2)),
        sec(-2.1, superellipse(1.42, 1.04, ROUND, 3.2)),
        sec(-0.8, superellipse(1.74, 1.14, ROUND, 3.2)),
        sec(1.0, superellipse(1.74, 1.14, ROUND, 3.2)),
        sec(2.15, superellipse(1.4, 0.96, ROUND, 3.2)),
        sec(2.5, superellipse(1.16, 0.8, ROUND, 3.2)),
    ], 3)
    s.add(loft(rings), m["hull_round"])
    front = [Vector((p.x, -2.62, p.z)) for p in ring_at(rings, -2.599, 0.04)]
    s.add(loft([front, ring_at(rings, -2.4, 0.04), ring_at(rings, -2.25, 0.04)]), m["armor_round"])
    s.add(loft([ring_at(rings, y, 0.04) for y in (-0.25, 0.25, 0.75)]), m["armor_round"])
    # Erhöhtes Rückendeck als zweite Lage, darauf die Brücke und eine Luke
    s.add(chamfer(loft([sec(-2.05, octagon(0.62, 0.26), dz=0.46), sec(-1.5, octagon(0.92, 0.32), dz=0.52),
                        sec(0.85, octagon(0.92, 0.32), dz=0.52)]), 0.025), m["armor"])
    s.add(chamfer(plate(octagon(0.32, 0.28, 0.4), 0.05, "xy"), 0.012), m["hull"], at=(0, 0.25, 0.685))
    # Unterseite: Kinn mit der Kondensatorbank der Kanone, Lüfter nach vorn; der dunkle Kiel wächst aus ihm
    s.add(chamfer(wedge(0.6, 0.9, 0.28), 0.025), m["hull"], at=(0, -1.7, -0.58), rot=(180, 0, 0))
    vent(s, m, m["hull"], (0, -2.15, -0.61), 0.42, 0.18, slats=1, rot=(90, 0, 0))
    s.add(chamfer(plate([(-1.9, 0.05), (2.1, 0.05), (1.9, -0.26), (-1.5, -0.3)], 0.16, "yz"), 0.03), m["dark"], at=(0, 0, -0.5))
    # Plasmakanone als Bug: Lagerkragen, Lauf mit drei Leuchtringen, Mündung mit Plasmakern
    s.add(tube(ROUND, 0.47, 0.43, -2.35, -2.85), m["dark_round"])
    # Wenig Überstand und breite Lücken, damit die Ringe auch in der Kartenansicht (30° zur Laufachse) getrennt bleiben
    s.add(tube(ROUND, 0.31, 0.29, -2.8, -4.0), m["steel"])
    for y in RINGS:
        s.add(tube(ROUND, 0.39, 0.39, y, y - 0.16), m["plasma"])
    s.add(revolve([(0.3, -3.96), (0.42, -4.06), (0.42, -4.16), (0.25, -4.16), (0.0, -4.14)], ROUND), m["dark_round"])
    s.add(tube(ROUND, 0.24, 0.24, -4.15, -4.19), m["plasma"])
    # Vier Gondeln an kurzen Trägern, Schelle am Träger, Leuchtring hinter der Nase, Düse am Ende
    pod = [sec(-1.2, superellipse(0.56, 0.56, MID)), sec(-0.95, superellipse(0.62, 0.62, MID)), sec(1.35, superellipse(0.62, 0.62, MID)),
           sec(1.7, superellipse(0.54, 0.54, MID))]
    band = [sec(-0.2, superellipse(0.68, 0.68, MID)), sec(0.7, superellipse(0.68, 0.68, MID))]
    for gz in GZ:
        s.add(loft(pod), m["hull_round"], at=(GX, 0, gz), mirror=True)
        s.add(loft(band), m["armor_round"], at=(GX, 0, gz), mirror=True)
        s.add(loft(smooth([tip(-1.68), sec(-1.38, superellipse(0.46, 0.46, MID)), sec(-1.12, superellipse(0.56, 0.56, MID))], 1)), m["accent_round"],
              at=(GX, 0, gz), mirror=True)
        s.add(tube(MID, 0.35, 0.35, -1.08, -1.24), m["glow"], at=(GX, 0, gz), mirror=True)
        s.add(chamfer(box(0.56, 0.8, 0.16), 0.03), m["dark"], at=(GX - 0.38, 0.25, gz), mirror=True)
    # Reaktorwulst achtern auf dem Rücken, mit violettem Band
    core = [tip(0.75), sec(0.95, ngon(ROUND, 0.4)), sec(1.35, ngon(ROUND, 0.56)), sec(1.95, ngon(ROUND, 0.56)), sec(2.35, ngon(ROUND, 0.44)),
            tip(2.5)]
    s.add(loft(core), m["accent_round"], at=(0, 0, 0.5))
    s.add(tube(ROUND, 0.6, 0.6, 1.55, 1.75), m["plasma"], at=(0, 0, 0.5))
    # Brücke mit Fensterband, Mast mit Radarplatte und Licht
    s.add(chamfer(wedge(0.7, 0.9, 0.34), 0.03), m["hull"], at=(0, -0.7, 0.82))
    s.add(box(0.5, 0.12, 0.05), m["window"], at=(0, -1.0, 0.915), rot=(24.8, 0, 0))
    s.add(post(16, 0.1, 0.09, 0.95, 1.45), m["hull_round"], at=(0, -0.5, 0))
    s.add(box(0.36, 0.05, 0.16), m["armor"], at=(0, -0.59, 1.22), rot=(-15, 0, 0))
    s.add(sphere(0.11, 1), m["glow"], at=(0, -0.5, 1.5))
    # Sechs Düsen: an den vier Gondeln und zwei am Heck, so weit nach achtern, dass ihr Gehäusesegment hinter dem Heck
    # liegt
    for r, y0, y1, dx, heights in ENGINES:
        for dz in heights:
            engine(s, m, r, y0, y1, dx=dx, dz=dz, n=MID, mirror=True)
            engine_details(s, m, r, y0, y1, dx=dx, dz=dz, n=MID, mirror=True)
    details(s, m)
    fine(s, m, rings, core, pod, band)
    return s.finish()
