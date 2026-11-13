"""Scrap Raider (Jäger, Brander), bewusst asymmetrisch, Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

Primärformen: kantiger Rumpf in warmem Schrottgrau mit stumpfem Bug, davor ein dunkler Rammbalken mit schwerem
Stahlsporn, Glaskanzel nach steuerbord versetzt, backbord eine schwere Kanonengondel in Koralle mit langem Lauf,
steuerbord ein Stummelflügel aus Rostblech mit Stachel, eine schiefe Rückenflosse, ein nach steuerbord versetzter Kiel,
drei ungleiche Düsen (die größte als Buckel obenauf) und große Rostflicken. Als Brander trägt er seine Ladung offen:
einen Brandstofftank auf dem Stummelflügel und einen Brandsatz unter dem Bug, beide in Koralle, damit die Klasse auch
von steuerbord (Gegnerseite) und von unten (Karte) liest.
Detailzonen (im Spiel steht er als Staffel zu je 0,42, dort zählen nur Umriss, Akzent und die großen Gliederungen),
dazwischen ruhig (Deck, Rückenflosse, Gondelflanke):
- Ramme: Stahllaschen an beiden Bugflanken, die den Rammbalken am Rumpf halten
- Stummelflügel: Brandstofftank mit zwei Spannbändern
- Unterseite: Brandsatz an einem Gehänge unter dem Bug, backbord neben dem Kiel
- Kanonengondel: Blendenbalg zwischen Verschluss und Kühlmantel
- Flicken: Rostbleche und ein Fremdblech im Blaugrau der Jäger an der Steuerbordflanke
- Heck: Maschinenlüfter auf dem Düsenbuckel, Gehäusesegment und Düsenkranz an jeder der drei Düsen
  (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`), seit der dreckige Look keine Konturringe mehr zeichnet: Überrollbügel über der Kanzel,
Steigeisen an der Steuerbordflanke, Wartungsluke, Schweißleiste und zwei ungleiche Peitschenantennen auf dem Rücken,
Einfüllstutzen und lose verlegte Brandstoffleitung am Tank, Stützstreben und eine Rakete unter dem Stummelflügel,
Klappenspalt, lose Leitung mit Schelle auf der Kanonengondel, Kühlrippen an Kanone und Buckeldüse, zwei Auspuffstutzen
an der Steuerbordflanke, Staurohr am Rammbalken, Klappe am Boden, Positionslichter an Flossen- und Flügelspitze.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme; an einem Jäger sparsam und nichts unter 0,03, hier das, was
ein Schrottschiff zusammenhält): Schweißraupen am Fuß der Flicken und am Flossenfuß, ein kleines Rostblech an der Wand
des Düsensockels, eine Strebe vom Deck an die Rückenflosse, zwei Laschen vom Deck auf die Kanonengondel, eine Stütze
vom Rammbalken an den Kühlmantel der Kanone, zwei Knotenbleche zwischen Rammbalken und Sporn, ein Kabelrohr mit zwei
Schellen von der Kanzel über das Deck in den Düsensockel, Stellstreben an der Buckeldüse (eine fehlt), Sättel unter dem
Brandstofftank, Rahmen und Scharnier an der Wartungsluke. Dazu Beschriftung als flache Geometrie: ein Warnband auf dem
Stummelflügel neben dem Tank und statt einer Kennnummer (er steht zu fünft) das Zeichen der Plünderer, drei helle
Krallenhiebe an der Flosse.
"""

import math

import palette
import vf
from mathutils import Vector
from parts import beam, bellows, engine, engine_details, ring_at, stripes, vent
from vf import (Ship, box, chamfer, hexagon, loft, octagon, pipe, plate, post, rbox, rect, revolve, sec, segment, smooth, sphere, superellipse,
                tube, wedge)

ROUND = 24  # Segmente der großen runden Teile (Kanonengondel, Tank, große Düsen); mittlere (Kanzel, Lauf, kleine Düse) 16
# Rumpf: Querschnitte (y, Breite, Höhe) des Achtkants
HULL = ((-2.8, 0.46, 0.46), (-2.2, 0.86, 0.66), (-1.1, 1.32, 0.86), (0.8, 1.42, 0.9), (2.1, 1.18, 0.76), (2.45, 0.98, 0.62))
# Schiefe Rückenflosse (y, z), vor dem Neigen nach steuerbord
FIN = [(-0.6, -0.12), (0.85, -0.12), (0.75, 0.82), (0.05, 0.94)]
FIN_AT, FIN_LEAN = (-0.25, 0, 0.42), 15
# Flicken (Material, Breite, Länge, Schnitt, Ort, Drehung, Schweißraupen): Rostbleche auf dem Deck, an der Backbordkante
# und am Boden, ein Fremdblech im Blaugrau der Jäger an der Steuerbordflanke, ein kleines Rostblech an der
# Steuerbordwand des Düsensockels. Schweißraupen als Stücke (Kante, von, bis):
# Kante -1 bei x = -w/2, 1 bei x = w/2, 0 vorn bei y = -l/2, `von` und `bis` als Anteil der Kantenlänge
PATCHES = (
    ("rust", 0.36, 0.8, 0.18, (-0.26, -1.55, 0.415), (-5, 0, 174), ((-1, 0.05, 0.5), (-1, 0.62, 0.95), (0, 0.1, 0.8))),
    ("rust", 0.26, 1.2, 0.14, (-0.61, 0.55, 0.405), (2, -33, 0), ((1, 0.1, 0.45), (1, 0.6, 0.85), (0, 0.15, 0.85))),
    ("rust", 0.6, 1.1, 0.24, (-0.27, 0.35, -0.45), (0, 2, -6), ()),
    ("armor", 0.42, 0.85, 0.2, (0.57, -1.575, 0.0), (0, 90, -11.8), ((-1, 0.0, 0.4), (-1, 0.55, 1.0), (0, 0.2, 0.9), (1, 0.1, 0.6))),
    ("rust", 0.16, 0.38, 0.08, (0.386, 1.81, 0.52), (0, 90, 0), ((-1, 0.1, 0.9),)),
)


def hull_at(y):
    """Rumpf bei y: halbe Breite und halbe Höhe (Deck oben, Boden unten; die Flanke steht bis 70 % der Höhe senkrecht)."""
    for (y0, w0, h0), (y1, w1, h1) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            return (w0 + t * (w1 - w0)) / 2, (h0 + t * (h1 - h0)) / 2
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def patch(w, l, cut=0.18):
    """Aufgeschweißtes Blech (w × l, 0,06 dick) mit einer schräg abgeschnittenen Ecke, damit es nicht wie eine Luke wirkt."""
    return plate([(-w / 2, -l / 2), (w / 2, -l / 2), (w / 2, l / 2 - cut), (w / 2 - cut, l / 2), (-w / 2, l / 2)], 0.06, "xy")


def canister(r, l, nose=0.0, n=16):
    """Liegender Behälter entlang Y (Länge l, Radius r) mit gewölbten Enden; `nose` spitzt das vordere Ende zu."""
    h = l / 2
    return revolve([(0.0, -h - nose), (r * 0.55, -h + 0.04 - nose * 0.6), (r * 0.88, -h + r * 0.4), (r, -h + r * 0.9), (r, h - r * 0.9),
                    (r * 0.88, h - r * 0.4), (r * 0.55, h - 0.04), (0.0, h)], n)


def welds(s, m, w, l, at, rot, beads):
    """Schweißraupen (`steel_flat`) am Fuß der Kanten eines Flickens w × l, der mit `at` und `rot` aufgesetzt ist;
    `beads` wie in PATCHES. Sie ragen 0,035 über die Kante hinaus und bleiben unter der Oberseite des Blechs."""
    g = Ship("welds")
    for edge, t0, t1 in beads:
        if edge:
            g.add(box(0.05, (t1 - t0) * l, 0.035), m["steel_flat"], at=(edge * (w / 2 + 0.01), (t0 + t1 - 1) / 2 * l, -0.012))
        else:
            g.add(box((t1 - t0) * w, 0.05, 0.035), m["steel_flat"], at=((t0 + t1 - 1) / 2 * w, -l / 2 - 0.01, -0.012))
    s.place(g, at=at, rot=rot)
    g.bm.free()


def details(s, m, canopy, px, pz):
    """Kleinteile (Modulkommentar); `canopy` sind die Ringe der Kanzel (um 0,27 nach steuerbord versetzt), (px, pz) die
    Achse der Kanonengondel."""

    def deck(y):
        return hull_at(y)[1]

    def wing_top(x):
        """Oberseite des Stummelflügels bei x (er steigt um 6° nach außen)."""
        return 0.28 + 0.1045 * x

    # ---------------------------------------------------------------- Kanzel: Überrollbügel, Steigeisen, Schweißleiste dahinter
    bow = []
    for y in (-1.56, -1.5):
        ring = ring_at(canopy, y)
        c = sum(ring, Vector()) / len(ring)
        bow.append([p + Vector((p.x - c.x, 0.0, p.z - c.z)).normalized() * 0.02 for p in ring])
    s.add(loft(bow), m["dark_round"], at=(0.27, 0, 0))
    for z in (-0.2, -0.08, 0.04, 0.16):
        s.add(box(0.05, 0.12, 0.02), m["steel_flat"], at=(hull_at(-0.95)[0] + 0.01, -0.95, z))
    s.add(box(0.9, 0.035, 0.03), m["steel_flat"], at=(0, -0.62, deck(-0.62) + 0.005))

    # ---------------------------------------------------------------- Rücken: Wartungsluke, zwei ungleiche Antennen, Flossenlicht
    s.add(box(0.3, 0.34, 0.03), m["armor"], at=(0.2, 0.15, deck(0.15) + 0.005))
    s.add(post(6, 0.012, 0.005, deck(0.62) - 0.03, deck(0.62) + 0.7, 0.3, 0.62), m["dark_round"])
    s.add(segment((0.38, 0.55, 0.42), (0.52, 0.8, 0.85), 0.012, 6), m["dark_round"])
    s.add(sphere(0.035, 1), m["glow"], at=(-0.007, 0.1, 1.32))

    # ---------------------------------------------------------------- Brandstofftank: Einfüllstutzen, lose Leitung in den Rumpf
    tx, ty, tz = 1.02, 0.3, wing_top(1.02) + 0.19
    s.add(post(8, 0.055, 0.055, tz + 0.2, tz + 0.27, tx, ty - 0.2), m["dark_round"])
    s.add(pipe([(0.86, 0.72, tz + 0.1), (0.7, 1.0, 0.62), (0.5, 1.25, 0.36)], 0.035, 6), m["dark_round"])

    # ---------------------------------------------------------------- Stummelflügel: Stützstreben, Rakete, Klappenspalt, Positionslicht
    s.add(beam((0.6, -0.2, -0.15), (1.15, -0.2, 0.26), 0.05, 0.05), m["steel_flat"])
    s.add(beam((0.6, 1.0, -0.15), (1.3, 1.0, 0.27), 0.05, 0.05), m["steel_flat"])
    rx, rz = 1.35, wing_top(1.35) - 0.16 - 0.13  # Unterseite des Flügels, darunter die Rakete
    s.add(box(0.05, 0.4, 0.09), m["dark"], at=(rx, 0.45, rz + 0.095))
    s.add(tube(8, 0.055, 0.055, 0.0, 0.85), m["armor_round"], at=(rx, 0, rz))
    s.add(tube(8, 0.0, 0.055, -0.2, 0.0), m["steel"], at=(rx, 0, rz))
    s.add(box(0.2, 0.1, 0.012), m["dark"], at=(rx, 0.78, rz))
    s.add(box(0.012, 0.1, 0.2), m["dark"], at=(rx, 0.78, rz))

    def hinge(x):
        return 1.3 - (x - 0.5) * 0.1724 - 0.12  # 0,12 vor der Hinterkante

    gap = [(0.65, hinge(0.65) - 0.015), (1.55, hinge(1.55) - 0.015), (1.55, hinge(1.55) + 0.015), (0.65, hinge(0.65) + 0.015)]
    s.add(plate(gap, 0.175, "xy"), m["dark"], at=(0, 0, 0.2), rot=(0, -6, 0))
    s.add(sphere(0.035, 1), m["glow"], at=(1.62, 1.0, wing_top(1.62)))

    # ---------------------------------------------------------------- Kanonengondel: Kühlrippen, lose Leitung mit Schelle
    for y in (-2.62, -2.46, -2.3):
        s.add(tube(16, 0.235, 0.235, y, y + 0.035), m["steel"], at=(px, 0, pz))
    s.add(pipe([(px + 0.12, -1.4, pz + 0.2), (px + 0.06, -1.05, pz + 0.35), (px + 0.08, -0.55, pz + 0.35), (px + 0.3, -0.3, pz + 0.43)], 0.03, 6),
          m["dark_round"])
    s.add(box(0.11, 0.06, 0.07), m["steel_flat"], at=(px + 0.07, -0.8, pz + 0.35))

    # ---------------------------------------------------------------- Heck: Kühlrippen an der Buckeldüse, Auspuffstutzen steuerbord
    for y in (2.21, 2.255, 2.3):
        s.add(tube(ROUND, 0.365, 0.365, y, y + 0.02, 0.06, 0.3), m["steel"])
    s.add(tube(8, 0.045, 0.045, 1.55, 2.02), m["steel"], at=(0.6, 0, 0.14))
    s.add(tube(8, 0.045, 0.045, 1.55, 1.95), m["steel"], at=(0.6, 0, 0.02))

    # ---------------------------------------------------------------- Bug und Boden: Staurohr am Rammbalken, Klappe
    s.add(segment((0.3, -2.75, 0.1), (0.36, -3.35, 0.14), 0.012, 6), m["steel"])
    s.add(box(0.22, 0.5, 0.03), m["armor"], at=(-0.14, 1.25, -deck(1.25)))


def fine(s, m, px, pz):
    """Feinteile und Beschriftung (Modulkommentar); (px, pz) ist die Achse der Kanonengondel."""

    def deck(y):
        return hull_at(y)[1]

    def wing_top(x):
        return 0.28 + 0.1045 * x

    # ---------------------------------------------------------------- Schweißnähte: Flicken, Flossenfuß
    # Raupen in Stücken am Fuß der Flicken (der am Boden bleibt glatt) und eine am Fuß der Flosse steuerbord
    for _, w, l, _, at, rot, beads in PATCHES:
        welds(s, m, w, l, at, rot, beads)
    s.add(box(0.05, 1.25, 0.04), m["steel_flat"], at=(-0.168, 0.12, 0.45))

    # ---------------------------------------------------------------- Streben und Laschen
    # Strebe vom Deck an die Steuerbordseite der Flosse; zwei Laschen vom Deckrand auf den Rücken der Kanonengondel (die
    # vordere über der losen Leitung); Stütze vom Rammbalken an den Kühlmantel der Kanone; zwei Knotenbleche zwischen
    # Rammbalken und Sporn
    s.add(beam((0.3, -0.1, deck(-0.1) - 0.02), (-0.09, -0.1, 0.845), 0.045, 0.045), m["steel_flat"])
    for y in (-1.2, 1.2):
        hw, hh = hull_at(y)
        x0 = -(0.7 * hw - 0.03)
        s.add(plate([(x0, hh - 0.015), (px - 0.02, pz + 0.3), (px - 0.02, pz + 0.35), (x0, hh + 0.03)], 0.11, "xz"), m["steel_flat"], at=(0, y, 0))
    s.add(beam((-0.36, -2.8, -0.08), (-0.74, -2.5, -0.1), 0.05, 0.05), m["steel_flat"])
    for x in (-0.1, 0.08):
        s.add(plate([(-2.84, 0.21), (-3.32, -0.01), (-2.84, 0.03)], 0.05, "yz"), m["steel_flat"], at=(x, 0, 0))

    # ---------------------------------------------------------------- Leitung: Kabelrohr von der Kanzel in den Düsensockel
    s.add(pipe([(0.36, -0.72, 0.45), (0.45, -0.45, deck(-0.45) + 0.02), (0.45, 0.4, deck(0.4) + 0.02), (0.42, 0.85, deck(0.85) + 0.02),
                (0.22, 1.04, 0.5)], 0.03, 6), m["dark_round"])
    for y in (-0.25, 0.28):
        s.add(box(0.12, 0.06, 0.07), m["steel_flat"], at=(0.45, y, deck(y) + 0.03))

    # ---------------------------------------------------------------- Buckeldüse: Stellstreben über der Glocke, eine fehlt
    r, y0, y1, dx, dz = 0.34, 1.9, 2.75, 0.06, 0.3
    for k in (0, 1, 2, 3, 5):
        a = 2 * math.pi * (k + 0.5) / 6
        c, d = math.cos(a), math.sin(a)
        s.add(segment((dx + 1.1 * r * c, y0 + 0.6 * (y1 - y0), dz + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, dz + 0.93 * r * d), 0.016, 6),
              m["steel"])

    # ---------------------------------------------------------------- Stummelflügel: Sättel unter dem Tank, Warnband
    for dy in (-0.42, 0.42):
        s.add(box(0.62, 0.16, 0.07), m["steel_flat"], at=(1.02, 0.3 + dy, wing_top(1.02) + 0.02), rot=(0, -6, 0))
    stripes(s, m, (1.41, 0.35, wing_top(1.41)), 0.08, 0.7, rot=(0, -6, 0))

    # ---------------------------------------------------------------- Deck: Rahmen und Scharnier der Luke, Krallenhiebe an der Flosse
    # Das Zeichen der Plünderer geht durch die Flosse und ist von beiden Seiten zu sehen
    s.add(box(0.36, 0.4, 0.012), m["dark"], at=(0.2, 0.15, deck(0.15) + 0.004))
    s.add(box(0.04, 0.22, 0.03), m["steel_flat"], at=(0.05, 0.15, deck(0.15) + 0.02))
    for y in (0.2, 0.33, 0.46):
        s.add(plate([(y, 0.3), (y + 0.06, 0.3), (y + 0.21, 0.68), (y + 0.15, 0.68)], 0.132, "yz"), m["paint"], at=FIN_AT, rot=(0, FIN_LEAN, 0))


def build():
    s = Ship("scrap_raider")
    m = vf.materials(s.name, "fighter")
    m["hull"] = vf.toon(f"{s.name}_hull", palette.SCRAP_HULL)  # Schrottgrau statt des Blaugraus der Klasse
    # Rost als rundes Material: flache Bleche bleiben flach (alle Kanten über 38°), bekommen aber nur das schmale
    # Kantenlicht; das breite der eckigen Materialien färbt sie auf dem Brett grau
    m["rust"] = vf.toon(f"{s.name}_rust", palette.RUST, round=True)

    # ---------------------------------------------------------------- Kantiger Rumpf, vorn stumpf
    s.add(chamfer(loft([sec(y, octagon(w, h), dz=-0.04 if k == 0 else 0.0) for k, (y, w, h) in enumerate(HULL)]), 0.03), m["hull"])

    # ---------------------------------------------------------------- Ramme: Rammbalken, Stahlkeil, Laschen an den Flanken
    s.add(rbox(0.8, 0.2, 0.58, 0.05, 1), m["dark"], at=(0, -2.8, -0.06))
    s.add(loft([sec(-2.6, hexagon(0.7, 0.42), dz=-0.08), sec(-3.8, hexagon(0.24, 0.07), dz=-0.13)]), m["steel_flat"])
    # Laschen: Stahlbänder vom Balken schräg an die Bugflanke, steuerbord länger (zusammengeschustert, nicht gespiegelt)
    for sx, y1 in ((1, -1.75), (-1, -2.05)):
        x1 = 0.43 + (0.66 - 0.43) * (-2.2 - y1) / 1.1 + 0.01
        s.add(loft([sec(-2.86, rect(0.06, 0.3), dx=sx * 0.39, dz=-0.04), sec(y1, rect(0.06, 0.26), dx=sx * x1, dz=-0.02)]), m["steel_flat"])

    # ---------------------------------------------------------------- Kanzel steuerbord
    canopy = smooth([
        sec(-2.25, superellipse(0.14, 0.08, 16), dz=0.3),
        sec(-1.85, superellipse(0.38, 0.3, 16), dz=0.4),
        sec(-1.25, superellipse(0.42, 0.36, 16), dz=0.44),
        sec(-0.7, superellipse(0.3, 0.16, 16), dz=0.45),
    ], 2)
    s.add(loft(canopy), m["glass"], at=(0.27, 0, 0))

    # ---------------------------------------------------------------- Backbord: schwere Kanonengondel in Koralle
    # Lichtband, Verschluss, Blendenbalg, Lauf mit Kühlmantel und Mündungsbremse
    px, pz = -0.92, -0.1
    s.add(loft(smooth([
        sec(-1.35, superellipse(0.62, 0.6, ROUND, 3)),
        sec(-0.95, superellipse(0.68, 0.66, ROUND, 3)),
        sec(1.25, superellipse(0.68, 0.66, ROUND, 3)),
    ], 1)), m["accent_round"], at=(px, 0, pz))
    s.add(loft([sec(1.2, superellipse(0.64, 0.62, ROUND, 3)), sec(1.85, superellipse(0.44, 0.42, ROUND, 3))]), m["dark_round"], at=(px, 0, pz))
    s.add(box(0.05, 1.2, 0.09), m["glow"], at=(px - 0.335, 0.0, pz))
    s.add(rbox(0.5, 0.56, 0.5, 0.06, 1), m["dark"], at=(px, -1.55, pz))
    s.add(bellows(16, 0.2, -1.8, -2.02), m["armor_round"], at=(px, 0, pz))
    s.add(tube(16, 0.14, 0.12, -1.8, -3.35), m["steel"], at=(px, 0, pz))
    s.add(tube(16, 0.21, 0.21, -2.1, -2.7), m["dark_round"], at=(px, 0, pz))
    s.add(tube(16, 0.22, 0.22, -3.3, -3.62), m["dark_round"], at=(px, 0, pz))
    s.add(tube(12, 0.11, 0.11, -3.62, -3.66), m["glow"], at=(px, 0, pz))

    # ---------------------------------------------------------------- Steuerbord: Stummelflügel mit Stachel und Brandstofftank
    s.add(chamfer(plate([(0.5, -0.7), (1.55, -0.25), (1.66, 1.1), (0.5, 1.3)], 0.16, "xy"), 0.03), m["rust"], at=(0, 0, 0.2), rot=(0, -6, 0))
    s.add(tube(16, 0.11, 0.0, -0.2, -1.25), m["steel"], at=(1.5, 0, 0.36))
    tx, ty = 1.02, 0.3
    tz = 0.28 + 0.1045 * tx + 0.19  # Oberseite des Flügels plus Radius, 0,03 eingesenkt
    s.add(canister(0.22, 1.5, n=ROUND), m["accent_round"], at=(tx, ty, tz))
    for dy in (-0.42, 0.42):
        s.add(tube(ROUND, 0.24, 0.24, ty + dy - 0.05, ty + dy + 0.05), m["dark_round"], at=(tx, 0, tz))

    # ---------------------------------------------------------------- Schiefe Rückenflosse, nach steuerbord geneigt
    s.add(chamfer(plate(FIN, 0.12, "yz"), 0.025), m["hull"], at=FIN_AT, rot=(0, FIN_LEAN, 0))

    # ---------------------------------------------------------------- Flicken: Rostbleche, Fremdblech an der Steuerbordflanke
    for mat, w, l, cut, at, rot, _ in PATCHES:
        s.add(patch(w, l, cut), m[mat], at=at, rot=rot)

    # ---------------------------------------------------------------- Unterseite: Kiel, Brandsatz am Gehänge unter dem Bug
    s.add(chamfer(plate([(-2.1, 0.02), (1.5, 0.02), (1.3, -0.26), (-1.5, -0.3)], 0.16, "yz"), 0.03), m["dark"], at=(0.25, 0, -0.42))
    s.add(box(0.2, 0.8, 0.08), m["dark"], at=(-0.2, -1.4, -0.4))
    s.add(canister(0.19, 1.1, nose=0.12), m["accent_round"], at=(-0.2, -1.4, -0.58))
    for y in (-1.7, -1.1):
        s.add(tube(16, 0.205, 0.205, y - 0.05, y + 0.05), m["dark_round"], at=(-0.2, 0, -0.58))

    # ---------------------------------------------------------------- Drei ungleiche Düsen
    # Die größte als Buckel obenauf auf einem kantigen Sockel, eine mittlere backbord unten und eine kleine steuerbord,
    # die unter dem Rumpf hängt und am weitesten hinausragt, damit sie von backbord zu sehen ist
    s.add(chamfer(wedge(0.6, 1.1, 0.34, front=0.6), 0.03), m["hull"], at=(0.06, 1.5, 0.5))
    vent(s, m, m["hull"], (0.06, 1.83, 0.67), 0.42, 0.38, slats=2)
    for r, y0, y1, dx, dz in ((0.34, 1.9, 2.75, 0.06, 0.3), (0.27, 2.0, 2.55, -0.42, -0.24), (0.2, 2.05, 2.88, 0.4, -0.4)):
        engine(s, m, r, y0, y1, dx=dx, dz=dz, n=ROUND if r > 0.25 else 16)
        engine_details(s, m, r, y0, y1, dx=dx, dz=dz, n=ROUND if r > 0.25 else 16)
    details(s, m, canopy, px, pz)
    fine(s, m, px, pz)
    return s.finish()
