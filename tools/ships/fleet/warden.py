"""Wächter (Boss von Sektor 2): unbewegliche automatische Verteidigungsstation, Schlachtschiff-Palette, Fassung für die
Nahaufnahme nach docs/stilregeln.md: gefaste Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

Kein Schiff, also keine Düsen: ein runder Stationskern mit Kragen, um ihn ein waagrechter Schildring aus vier Segmenten
auf dunklen Streben, mit Lichtband und stahlblauen Emitterblöcken. Vorn, durch die Lücke im Ring, das eine riesige
Hauptgeschütz in Nasenrichtung: Gehäuse in Stahlblau, Kühlmantel, langer Lauf, Mündungsbremse mit kaltem Kern. Oben eine
Deckplatte mit dem Sensorkopf, achtern der Energieblock, unten eine Bodenplatte mit Ankerdorn.
Detailzonen mit Zweck (eine Station ohne Besatzung, also Technik statt Brücke), dazwischen ruhige Flächen (Kernkuppel
vor und hinter der Deckplatte, Flanken des Geschützgehäuses, Energieblock):
- Hauptgeschütz: Lafettenring am Kern, Ladebänke an den Gehäuseflanken über der Ringebene mit Deckel, Ladeanzeige und
  drei Zellen, Lüfter auf dem Gehäuse, Blendenbalg vor dem Kühlmantel, Laufschelle, zweiteilige Mündungsbremse
  (Stahlkern im Spalt)
- Schildring: Deckel oben und unten auf jedem Segment (das Lichtband liegt auf dem oberen), Emitterlinsen nach außen
- Flanken: Andockarme in den seitlichen Ringlücken mit Flansch, Stutzen und Luke; Kabelkanäle vom Energieverteiler
  achtern unter dem Kragen durch bis in den Lafettenring
- Sensorkopf: zweite Lage auf der Deckplatte, Drehkranz, Trommel mit umlaufendem Sensorband (Licht ohne Glühen),
  Kappe, Mast mit Licht
- Energie und Kühlung achtern: zwei Kühlblöcke mit Lüftern auf dem Energieblock, Energieverteiler an seinen Seiten
- Unterseite: Bodenplatte mit zweiter Lage, Ankerflansch und Ankerdorn, Plattenlage und Lafettenkiel unter dem
  Geschützgehäuse
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought; Wartungstechnik statt Fenster, die Station hat keine Besatzung):
Kühlrippen am Kühlmantel, Kühlmittelleitungen von den Ladebänken zum Kühlmantel, Flak an den Ecken der Deckplatte,
Geländer an ihren Kanten und an den Andockstegen, Peitschenantennen auf der Kappe und den Emitterblöcken, Schüssel und
Gerätekasten hinter dem Sensorkopf, Kühlrippen am Energieblock, Sonden unter der Bodenplatte, Positionslichter an den
Andockflanschen und am Heck.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; sie zeigen, wie groß die Station ist, und wo Wartungstrupps
anlegen, aufsteigen und schrauben): Schrauben am Lafettenring, Zuganker durch die Mündungsbremse, Kühlringe,
Kühlmittelleitungen und verschraubte Schelle am Lauf, Luke auf dem Geschützgehäuse, Griffe und Schrauben an den Zellen
der Ladebänke; am Schildring verschraubte Deckel, Wartungsklappe und Spanten je Segment, Kupplungen an den Stirnseiten,
Schrauben um die Emitterlinsen, Leitungsschellen auf den Streben; Andockluke mit Handrad, Riegeln und Flanschschrauben,
Steigeisen vom Andockarm über den Kragen zur Deckplatte; Gurtbänder und Wartungsklappen auf der Kernkuppel, Schellen am
Kabelkanal; Schrauben am Drehkranz und am Mastfuß, vier Luken auf der Kappe des Sensorkopfs; Kühlmittelleitungen, Luke
und verschraubte Verteiler am Energieblock. Dazu Beschriftung als flache Geometrie: Warnbänder unter den Ladebänken, auf
den Andockarmen und an der Vorderkante der Deckplatte, die Kennnummer an den Flanken des Gehäusekragens und an der Stirn
der Deckplatte (der Wächter steht dem Spieler gegenüber, man liest sie von vorn oben).
"""

import math

from mathutils.bvhtree import BVHTree

import palette
import vf
from parts import beam, bellows, bolts, flak, hatch, number, on_surface, rail, ring_at, spaced, stripes, vent
from vf import (Ship, box, chamfer, dish, flank, grow, hoop, loft, octagon, pipe, plate, post, rect, sec, segment, smooth, sphere, superellipse,
                sweep, tube)

ROUND = 40  # Segmente der großen runden Teile (Kern, Kragen, Geschütz, Sensorkopf)
ZC = 0.5  # Höhe der Kernmitte und des Rings
RING = (2.55, 3.0)  # Innen- und Außenradius des Rings
SEGMENTS = (45, 135, 225, 315)  # Mitten der Ringsegmente (Grad, 0 = Steuerbord); Lücken vorn, achtern und seitlich
SPAN = 40  # Bogen je Segment in Grad
CABLE_Z = 1.03  # Höhe der Kabelkanäle an den Kernflanken
BANK_Y = -2.825  # Mitte der Ladebänke an den Gehäuseflanken


def arc(r_in, r_out, a0, a1, n=16):
    """Kreisringsektor (x, y) von a0 bis a1 Grad."""
    angles = [math.radians(a0 + (a1 - a0) * k / n) for k in range(n + 1)]
    outer = [(r_out * math.cos(a), r_out * math.sin(a)) for a in angles]
    inner = [(r_in * math.cos(a), r_in * math.sin(a)) for a in reversed(angles)]
    return outer + inner


def polar(r, a, z=0.0):
    """Punkt um die Hochachse: Radius r, Winkel a in Grad (0 = Steuerbord, 90 = achtern), Höhe z."""
    t = math.radians(a)
    return (r * math.cos(t), r * math.sin(t), z)


def flank_at(rings, y, z, dx=0.03):
    """Punkt auf der Steuerbordflanke des Kerns bei (y, z), um dx nach außen versetzt."""
    return flank([ring_at(rings, y)], z, dx)[0]


def details(s, m, top):
    """Kleinteile (Modulkommentar); `top` ist die Bezugshöhe des Sensorkopfs."""
    rim, lid = ZC + 1.52, ZC + 1.56  # Rand und zweite Lage der Deckplatte (Oberseiten)

    # ---------------------------------------------------------------- Hauptgeschütz: Kühlrippen am Kühlmantel,
    # Kühlmittelleitungen von den Ladebänken über das Gehäuse an den Mantel
    for y in (-4.55, -4.35, -4.15, -3.95):
        s.add(tube(ROUND, 0.77, 0.77, y, y + 0.04, dz=ZC), m["armor_round"])
    s.add(pipe([(1.05, -3.2, 1.14), (1.05, -3.45, 1.14), (0.567, -3.8, ZC + 0.476), (0.567, -4.6, ZC + 0.476)], 0.035, 6), m["steel"],
          mirror=True)

    # ---------------------------------------------------------------- Deckplatte: Flak an den Ecken, Geländer an der
    # Vorderkante und den Fasen, Schüssel und Gerätekasten mit Nadel achtern
    for y, yaw in ((-0.98, 35), (0.98, 145)):
        flak(s, m, (0.78, y, lid), yaw)
    rail(s, m, (-0.75, -1.29, rim), (0.75, -1.29, rim), 5, mirror=False)  # hinter der Fase der Plattenkante
    for side in (-1, 1):
        rail(s, m, (1.105, side * 0.907, rim), (0.76, side * 1.311, rim), 3)
    s.add(post(8, 0.03, 0.025, lid - 0.02, lid + 0.14), m["dark_round"], at=(-0.4, 1.14, 0))
    s.add(dish(0.14, 0.05, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.4, 1.14, lid + 0.13), rot=(-40, 0, 180))
    s.add(box(0.16, 0.14, 0.12), m["armor"], at=(0.4, 1.14, lid + 0.055))
    s.add(post(6, 0.012, 0.006, lid + 0.1, lid + 0.5), m["dark_round"], at=(0.4, 1.14, 0))

    # ---------------------------------------------------------------- Peitschenantennen: Kappe des Sensorkopfs,
    # Emitterblöcke
    s.add(post(6, 0.012, 0.005, top + 0.5, top + 1.2), m["dark_round"], at=(0.3, 0.25, 0), mirror=True)
    for a in (45, 315):
        x, y, _ = polar(3.05, a)
        s.add(post(6, 0.012, 0.005, ZC + 0.28, ZC + 0.75), m["dark_round"], at=(x, y, 0), mirror=True)

    # ---------------------------------------------------------------- Andockstege: Geländer, Positionslicht am Flansch
    for y in (-0.11, 0.11):
        rail(s, m, (1.92, y, ZC + 0.15), (2.34, y, ZC + 0.15), 3)
    s.add(sphere(0.035, 1), m["glow"], at=(2.45, 0, ZC + 0.41), mirror=True)

    # ---------------------------------------------------------------- Energieblock: Kühlrippen an den Flanken, Hecklicht
    for y in (2.45, 2.6, 2.75, 2.9):
        s.add(box(0.16, 0.03, 0.6), m["armor"], at=(0.86, y, ZC), mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(0, 3.58, ZC + 0.44))

    # ---------------------------------------------------------------- Sonden unter der Bodenplatte
    for y in (-0.75, 0.75):
        s.add(post(6, 0.006, 0.012, -1.38, -1.15), m["dark_round"], at=(0.55, y, 0), mirror=True)


def fine(s, m, top, core, tree):
    """Feinteile und Beschriftung (Modulkommentar); `top` ist die Bezugshöhe des Sensorkopfs, `core` sind die Ringe des
    Kerns, `tree` sein BVH-Baum."""
    rim, lid = ZC + 1.52, ZC + 1.56  # Rand und zweite Lage der Deckplatte (Oberseiten)

    def around(r, step, start=0):
        """Punkte (Winkel in Grad, r · cos, r · sin) im Abstand von `step` Grad."""
        return [(a, r * math.cos(math.radians(a)), r * math.sin(math.radians(a))) for a in range(start, 360, step)]

    # ---------------------------------------------------------------- Hauptgeschütz: Lafettenring, Gehäuse, Ladebänke
    # Schrauben auf der Stirn des Lafettenrings (der Wächter zeigt dem Spieler den Bug), Wartungsluke auf der Schräge
    # vor dem Lüfter (sie fällt um 20° zum Kühlmantel), Griff und Schrauben an jeder Zelle, Schrauben am Deckel
    bolts(s, m["steel_flat"], [(x, -2.3, ZC + z) for x, z in spaced(octagon(2.03, 1.83, 0.35), 0.25)], "-y")
    hatch(s, m, (0, -3.39, 1.281), 0.3, 0.22, rot=(20, 0, 0))
    for dy in (-0.275, 0.0, 0.275):
        s.add(box(0.02, 0.1, 0.014), m["steel_flat"], at=(1.256, BANK_Y + dy, 1.04), mirror=True)
        bolts(s, m["steel_flat"], [(1.25, BANK_Y + dy + ey, 1.04 + ez) for ey in (-0.075, 0.075) for ez in (-0.06, 0.06)], "x", r=0.009,
              mirror=True)
    bolts(s, m["steel_flat"], [(1.06 + ex, BANK_Y + ey, 1.41) for ex in (-0.16, 0.16) for ey in (-0.41, 0.41)], mirror=True)

    # ---------------------------------------------------------------- Lauf: Kühlringe, Leitungen, Schelle, Zuganker
    # Drei Kühlringe hinter dem Blendenbalg, zwei Kühlmittelleitungen oben am Lauf vom Balg bis in die Mündungsbremse,
    # Schrauben rund um die Laufschelle, vier Zuganker durch beide Platten der Bremse
    def barrel(y):
        return 0.5 - 0.06 * (y + 6.72) / 2.07

    for y in (-5.14, -5.26, -5.38):
        s.add(tube(ROUND, barrel(y) + 0.025, barrel(y) + 0.025, y, y + 0.035, dz=ZC), m["armor_round"])
    for a in (-25, 25):
        c, d = math.sin(math.radians(a)), math.cos(math.radians(a))
        r0, r1 = barrel(-4.9) + 0.02, barrel(-6.12) + 0.02
        s.add(segment((r0 * c, -4.9, ZC + r0 * d), (r1 * c, -6.12, ZC + r1 * d), 0.022, 6), m["steel"])
    for a, x, z in around(0.56, 30, 15):
        bolts(s, m["steel_flat"], [(x, -5.61, ZC + z)], (0, 90 - a, 0))
    for dz in (-0.42, 0.42):
        s.add(tube(6, 0.03, 0.03, -6.78, -6.07, 0.42, ZC + dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Schildring: Deckel, Klappe, Spanten, Kupplungen
    # Je Segment: Schrauben am Außenrand des Deckels, außen eine Wartungsklappe und drei Spanten beiderseits des
    # Emitters, an beiden Stirnseiten eine Kupplung zum Nachbarsegment; Schrauben um die Linse und auf dem Emitterblock,
    # eine verschraubte Leitungsschelle auf der Strebe
    for a in SEGMENTS:
        bolts(s, m["steel"], [polar(2.86, a + t, ZC + 0.26) for t in range(-16, 17, 4)])
        hatch(s, m, polar(3.0, a - 12, ZC), 0.24, 0.3, rot=(0, 90, a - 12))
        for t in (9, 12, 15):
            s.add(box(0.03, 0.04, 0.3), m["armor"], at=polar(3.005, a + t, ZC), rot=(0, 0, a + t))
        for end, turn in ((a + SPAN / 2, 0), (a - SPAN / 2, 180)):
            g = Ship("coupling")
            g.add(box(0.22, 0.03, 0.2), m["dark"], at=(0, 0.005, 0))
            g.add(tube(8, 0.05, 0.05, 0.0, 0.04), m["steel_flat"])
            bolts(g, m["steel_flat"], [(ex, 0.02, ez) for ex in (-0.08, 0.08) for ez in (-0.07, 0.07)], "y", r=0.01)
            s.place(g, at=polar((RING[0] + RING[1]) / 2, end, ZC), rot=(0, 0, end + turn))
            g.bm.free()
        g = Ship("emitter")
        bolts(g, m["steel_flat"], [(ex, 0, ez) for ex in (-0.18, 0.18) for ez in (-0.23, 0.23)], "y")
        bolts(g, m["steel_flat"], [(ex, -0.21 + ey, 0.3) for ex in (-0.17, 0.17) for ey in (-0.13, 0.13)])
        s.place(g, at=polar(3.26, a, ZC), rot=(0, 0, a - 90))
        g.bm.free()
        g = Ship("strut")
        g.add(box(0.3, 0.1, 0.03), m["armor"], at=(0, 0, 0.008))
        bolts(g, m["steel_flat"], [(ex, 0, 0.023) for ex in (-0.11, 0.11)], r=0.01)
        s.place(g, at=polar(2.36, a, ZC + 0.15), rot=(0, 0, a - 90))
        g.bm.free()

    # ---------------------------------------------------------------- Andockarme: Flansch, Luke, Riegel, Steigeisen
    # Schrauben auf dem Flansch, Handrad auf der Luke, vier Riegel am Stutzen; vom Arm führen Steigeisen über den Kragen
    # (jedes senkrecht auf der Fläche) und die Kante der Deckplatte hinauf
    g = Ship("dock")
    bolts(g, m["steel_flat"], [(x, 0.08, z) for _, x, z in around(0.34, 30)], "y")
    g.add(tube(6, 0.02, 0.02, 0.23, 0.275), m["steel_flat"])
    g.add(hoop(8, 0.05, 0.07, 0.014), m["steel_flat"], at=(0, 0.268, 0), rot=(90, 0, 0))
    g.add(box(0.13, 0.01, 0.012), m["steel_flat"], at=(0, 0.266, 0))
    g.add(box(0.012, 0.01, 0.13), m["steel_flat"], at=(0, 0.27, 0))
    for a, x, z in around(0.285, 90, 45):
        g.add(box(0.06, 0.1, 0.07), m["armor"], at=(x, 0.17, z), rot=(0, -a, 0))
    s.place(g, at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)
    g.bm.free()
    for k in range(19):
        z = 0.3 + 0.055 * k
        x = 1.9 * (1 - (z / 1.6) ** 2.4) ** (1 / 2.4)
        tilt = math.atan2((z / 1.6) ** 1.4 / 1.6, (x / 1.9) ** 1.4 / 1.9)
        s.add(box(0.03, 0.09, 0.012), m["steel_flat"], at=(x + 0.006 * math.cos(tilt), 0, ZC + z + 0.006 * math.sin(tilt)),
              rot=(0, -math.degrees(tilt), 0), mirror=True)
    for z in (rim - 0.1, rim - 0.045):
        s.add(box(0.03, 0.09, 0.012), m["steel_flat"], at=(1.156, 0, z), mirror=True)

    # ---------------------------------------------------------------- Kern: Gurtbänder, Wartungsklappen, Kabelschellen
    # Zwei Gurtbänder um die Kuppel vor und hinter der Deckplatte, zwei Klappen je Seite auf der Schulter, vier
    # verschraubte Schellen je Kabelkanal
    for y in (-1.5, 1.42):
        s.add(loft([ring_at(core, y, 0.016), ring_at(core, y + 0.08, 0.016)]), m["armor_round"])
    for y in (-0.8, 0.8):
        hit, rot = on_surface(tree, 1.42, y)
        hatch(s, m, tuple(hit), 0.16, 0.3, rot=rot, mirror=True)
    for y in (-1.3, -0.75, 0.75, 1.3):
        x, x0, x1 = (flank_at(core, y + e, CABLE_Z)[0] for e in (0.0, -0.05, 0.05))
        g = Ship("clamp")
        g.add(box(0.15, 0.05, 0.15), m["armor"])
        bolts(g, m["steel_flat"], [(0.075, 0, ez) for ez in (-0.05, 0.05)], "x", r=0.01)
        s.place(g, at=(x, y, CABLE_Z), rot=(0, 0, math.degrees(math.atan2(x0 - x1, 0.1))), mirror=True)
        g.bm.free()

    # ---------------------------------------------------------------- Sensorkopf: Drehkranz, Wartungsluken, Mastfuß
    # (die Kappe fällt um 35,5°; die vier Luken liegen auf ihr)
    bolts(s, m["steel"], [(x, y, top - 0.1) for _, x, y in around(1.0, 15)])
    for a, x, y in around(0.525, 90, 45):
        hatch(s, m, (x, y, top + 0.475), 0.2, 0.16, rot=(0, 35.5, a))
    s.add(post(16, 0.13, 0.13, top + 0.595, top + 0.63), m["armor_round"])
    bolts(s, m["steel_flat"], [(x, y, top + 0.63) for _, x, y in around(0.11, 60)], r=0.01)

    # ---------------------------------------------------------------- Energieblock: Leitungen, Verteiler, Luke
    # Zwei Kühlmittelleitungen je Kühlblock nach vorn in die Kuppel, Schrauben an den Verteilern, Wartungsluke in der
    # Flanke hinter den Kühlrippen
    for x in (0.24, 0.46):
        s.add(segment((x, 2.25, 1.34), (x, 1.85, 1.34), 0.035, 8), m["steel"], mirror=True)
    bolts(s, m["steel_flat"], [(1.26, 1.98 + ey, CABLE_Z + 0.03 + ez) for ey in (-0.14, 0.14) for ez in (-0.11, 0.11)], "x", mirror=True)
    hatch(s, m, (0.8, 3.13, ZC), 0.26, 0.26, rot=(0, 90, 0), mirror=True)

    # ---------------------------------------------------------------- Beschriftung: Warnbänder, Kennnummer
    # Warnbänder unter den Ladebänken, auf den Andockarmen zwischen den Geländern und an der Vorderkante der Deckplatte;
    # die Kennnummer an beiden Flanken des Gehäusekragens (er verjüngt sich um 24,4° zum Kühlmantel; die Flanken dahinter
    # verdeckt der Schildring) und an der Stirn der Deckplatte, lesbar von vorn oben
    stripes(s, m, (0.95, BANK_Y, 0.84), 0.07, 0.85, rot=(0, 90, 0), mirror=True)
    stripes(s, m, (2.12, 0, ZC + 0.15), 0.1, 0.36, rot=(0, 0, 90), mirror=True)
    stripes(s, m, (0, -1.145, lid), 0.05, 0.8, rot=(0, 0, 90))
    number(s, m["dark"], "09", (0.825, -3.475, ZC - 0.05), 0.28, (90, 0, 65.6))
    number(s, m["dark"], "09", (-0.825, -3.475, ZC - 0.05), 0.28, (90, 0, -65.6))
    number(s, m["paint"], "09", (0, -1.35, ZC + 1.4), 0.15, (90, 0, 0))  # über der Kuppel, die den Fuß der Stirn deckt


def build():
    s = Ship("warden")
    m = vf.materials(s.name, "battleship")
    # Das Sensorband läuft ganz um die Trommel: großes Licht ohne Glühen (docs/stilregeln.md, Große Lichter)
    m["sensor"] = vf.light(f"{s.name}_sensor", palette.WINDOW, glow=False)

    # ---------------------------------------------------------------- Kern und Kragen
    body = lambda w, h: superellipse(w, h, ROUND, 2.4)  # noqa: E731
    core = smooth([
        sec(-2.3, body(1.6, 1.4), dz=ZC),
        sec(-1.6, body(3.0, 2.5), dz=ZC),
        sec(0.0, body(3.5, 2.9), dz=ZC),
        sec(1.6, body(3.0, 2.5), dz=ZC),
        sec(2.3, body(1.6, 1.4), dz=ZC),
    ], 3)
    part = loft(core)
    tree = BVHTree.FromBMesh(part)
    s.add(part, m["hull_round"])
    s.add(loft([sec(-0.22, body(3.8, 3.2), dz=ZC), sec(0.22, body(3.8, 3.2), dz=ZC)]), m["armor_round"])

    # ---------------------------------------------------------------- Schildring: Segmente mit Deckeln, Lichtband,
    # Streben, Emitter mit Linse
    # (die Linse ist eckig schattiert: Einen runden Deckel an rundem Material schwärzte die Verwitterung als Mündung)
    for a in SEGMENTS:
        s.add(chamfer(plate(arc(*RING, a - SPAN / 2, a + SPAN / 2), 0.42, "xy"), 0.04), m["armor"], at=(0, 0, ZC))
        lid = arc(RING[0] + 0.09, RING[1] - 0.09, a - SPAN / 2 + 1.9, a + SPAN / 2 - 1.9)
        for dz in (0.225, -0.225):
            s.add(chamfer(plate(lid, 0.07, "xy"), 0.02), m["hull"], at=(0, 0, ZC + dz))
        band = [polar(2.72, a + t, ZC + 0.28) for t in range(-18, 19, 3)]
        s.add(sweep(band, rect(0.08, 0.06)), m["glow"])
        s.add(chamfer(beam(polar(1.2, a, ZC), polar(2.7, a, ZC), 0.34, 0.3), 0.03), m["dark"])
        s.add(chamfer(box(0.5, 0.42, 0.6), 0.04), m["accent"], at=polar(3.05, a, ZC), rot=(0, 0, a + 90))
        s.add(tube(24, 0.15, 0.15, -0.02, 0.03), m["dark"], at=polar(3.26, a, ZC), rot=(0, 0, a - 90))

    # ---------------------------------------------------------------- Hauptgeschütz durch die vordere Lücke
    s.add(chamfer(loft([sec(-1.95, octagon(2.15, 1.95, 0.35), dz=ZC), sec(-2.3, octagon(2.15, 1.95, 0.35), dz=ZC)]), 0.04), m["dark"])
    s.add(chamfer(loft([sec(-1.2, octagon(1.9, 1.7, 0.35), dz=ZC), sec(-3.2, octagon(1.9, 1.7, 0.35), dz=ZC),
                        sec(-3.75, octagon(1.4, 1.3, 0.35), dz=ZC)]), 0.05), m["accent"])
    s.add(plate([(-0.62, -2.45), (0.62, -2.45), (0.62, -3.15), (-0.62, -3.15)], 0.07, "xy"), m["armor"], at=(0, 0, ZC - 0.86))
    # Lafettenkiel: stützt das Gehäuse gegen den Kernboden
    s.add(chamfer(plate([(-1.2, -0.62), (-2.35, -0.3), (-2.35, -0.5), (-1.45, -0.92)], 0.3, "yz"), 0.04), m["dark"])
    vent(s, m, m["accent"], (0, -2.8, ZC + 0.82), 0.7, 0.62, slats=2)
    # Ladebänke an den Gehäuseflanken über der Ringebene: Block mit Deckel, Ladeanzeige und drei Zellen
    bank_y = BANK_Y
    s.add(chamfer(box(0.34, 0.85, 0.44), 0.03), m["armor"], at=(1.05, bank_y, 1.14), mirror=True)
    s.add(box(0.4, 0.91, 0.05), m["hull"], at=(1.06, bank_y, 1.385), mirror=True)
    s.add(box(0.04, 0.6, 0.07), m["glow"], at=(1.23, bank_y, 1.25), mirror=True)
    for dy in (-0.275, 0.0, 0.275):
        s.add(box(0.04, 0.2, 0.17), m["dark"], at=(1.23, bank_y + dy, 1.04), mirror=True)
    # Kühlmantel, Blendenbalg, Lauf mit Laufschelle, zweiteilige Mündungsbremse mit Stahlkern, kalter Kern
    s.add(tube(ROUND, 0.72, 0.72, -4.75, -3.6, dz=ZC), m["dark_round"])
    s.add(bellows(ROUND, 0.62, -4.72, -5.02), m["armor_round"], at=(0, 0, ZC))
    s.add(tube(ROUND, 0.5, 0.44, -6.72, -4.65, dz=ZC), m["steel"])
    s.add(tube(ROUND, 0.56, 0.56, -5.67, -5.55, dz=ZC), m["armor_round"])
    for y0, y1 in ((-6.32, -6.1), (-6.75, -6.5)):
        s.add(chamfer(loft([sec(y1, octagon(1.3, 1.3, 0.4), dz=ZC), sec(y0, octagon(1.3, 1.3, 0.4), dz=ZC)]), 0.04), m["dark"])
    s.add(tube(24, 0.3, 0.3, -6.8, -6.7, dz=ZC), m["glow"])

    # ---------------------------------------------------------------- Andockarme in den seitlichen Ringlücken
    # (die Luke ist eckig schattiert, siehe Linsen)
    s.add(chamfer(beam((1.6, 0, ZC), (2.45, 0, ZC), 0.3, 0.3), 0.03), m["dark"], mirror=True)
    s.add(tube(ROUND, 0.4, 0.4, -0.08, 0.08), m["armor_round"], at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)
    s.add(tube(24, 0.28, 0.28, -0.12, 0.22), m["dark_round"], at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)
    s.add(tube(24, 0.17, 0.17, 0.2, 0.24), m["armor"], at=(2.45, 0, ZC), rot=(0, 0, -90), mirror=True)

    # ---------------------------------------------------------------- Kabelkanäle vom Verteiler bis in den Lafettenring
    path = [flank_at(core, y, CABLE_Z) for y in (-2.0, -1.6, -1.2, -0.6, 0.0, 0.6, 1.2, 1.6, 1.9)]
    s.add(sweep(path, rect(0.1, 0.1)), m["dark"], mirror=True)

    # ---------------------------------------------------------------- Sensorkopf: Deckplatte mit zweiter Lage,
    # Drehkranz, Trommel mit Sensorband, Kappe, Mast mit Licht
    deck = octagon(2.3, 2.7)
    s.add(chamfer(plate(deck, 0.32, "xy"), 0.04), m["armor"], at=(0, 0, ZC + 1.36))
    s.add(chamfer(plate(grow(deck, -0.14), 0.07, "xy"), 0.02), m["hull"], at=(0, 0, ZC + 1.525))
    top = ZC + 1.72
    s.add(post(ROUND, 1.08, 1.08, top - 0.2, top - 0.1), m["armor_round"])
    s.add(post(ROUND, 0.95, 0.8, top - 0.2, top + 0.35), m["dark_round"])
    s.add(post(ROUND, 0.9, 0.88, top + 0.12, top + 0.2), m["sensor"])
    s.add(post(ROUND, 0.7, 0.35, top + 0.35, top + 0.6), m["hull_round"])
    s.add(post(16, 0.09, 0.06, top + 0.6, top + 1.5), m["hull_round"])
    s.add(sphere(0.09, 2), m["glow"], at=(0, 0, top + 1.55))

    # ---------------------------------------------------------------- Energieblock achtern mit Kühlblöcken und
    # Energieverteilern
    s.add(chamfer(loft([sec(1.9, octagon(1.6, 1.2), dz=ZC), sec(3.3, octagon(1.6, 1.2), dz=ZC), sec(3.65, octagon(1.1, 0.8), dz=ZC)]), 0.05),
          m["dark"])
    s.add(chamfer(box(0.54, 1.0, 0.5), 0.04), m["armor"], at=(0.35, 2.7, 1.25), mirror=True)
    vent(s, m, m["armor"], (0.35, 2.7, 1.5), 0.4, 0.8, slats=2, mirror=True)
    s.add(chamfer(box(0.36, 0.4, 0.34), 0.03), m["armor"], at=(1.08, 1.98, CABLE_Z + 0.03), mirror=True)

    # ---------------------------------------------------------------- Unterseite: Bodenplatte, Ankerflansch, Ankerdorn
    s.add(chamfer(plate(octagon(2.0, 2.4), 0.3, "xy"), 0.04), m["armor"], at=(0, 0, -0.97))
    s.add(plate(grow(octagon(2.0, 2.4), -0.13), 0.07, "xy"), m["hull"], at=(0, 0, -1.135))
    s.add(post(ROUND, 0.62, 0.62, -1.26, -1.15), m["armor_round"])
    s.add(post(24, 0.5, 0.0, -1.24, -1.39), m["dark_round"])
    details(s, m, top)
    fine(s, m, top, core, tree)
    return s.finish()
