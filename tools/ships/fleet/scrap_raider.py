"""Scrap Raider (Jäger, Brander) im Clean-Toon-Stil, bewusst asymmetrisch, detailreiche Fassung nach der Iron
Dreadnought (docs/stilregeln.md).

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
"""

import palette
import vf
from mathutils import Vector
from parts import beam, bellows, engine, engine_details, ring_at, vent
from vf import Ship, box, hexagon, loft, octagon, pipe, plate, post, rbox, rect, revolve, sec, segment, smooth, sphere, superellipse, tube, wedge

# Rumpf: Querschnitte (y, Breite, Höhe) des Achtkants
HULL = ((-2.8, 0.46, 0.46), (-2.2, 0.86, 0.66), (-1.1, 1.32, 0.86), (0.8, 1.42, 0.9), (2.1, 1.18, 0.76), (2.45, 0.98, 0.62))


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


def canister(r, l, nose=0.0):
    """Liegender Behälter entlang Y (Länge l, Radius r) mit gewölbten Enden; `nose` spitzt das vordere Ende zu."""
    h = l / 2
    return revolve([(0.0, -h - nose), (r * 0.55, -h + 0.04 - nose * 0.6), (r * 0.88, -h + r * 0.4), (r, -h + r * 0.9), (r, h - r * 0.9),
                    (r * 0.88, h - r * 0.4), (r * 0.55, h - 0.04), (0.0, h)], 16)


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
        s.add(tube(12, 0.235, 0.235, y, y + 0.035), m["steel"], at=(px, 0, pz))
    s.add(pipe([(px + 0.12, -1.4, pz + 0.2), (px + 0.06, -1.05, pz + 0.35), (px + 0.08, -0.55, pz + 0.35), (px + 0.3, -0.3, pz + 0.43)], 0.03, 6),
          m["dark_round"])
    s.add(box(0.11, 0.06, 0.07), m["steel_flat"], at=(px + 0.07, -0.8, pz + 0.35))

    # ---------------------------------------------------------------- Heck: Kühlrippen an der Buckeldüse, Auspuffstutzen steuerbord
    for y in (2.21, 2.255, 2.3):
        s.add(tube(12, 0.365, 0.365, y, y + 0.02, 0.06, 0.3), m["steel"])
    s.add(tube(8, 0.045, 0.045, 1.55, 2.02), m["steel"], at=(0.6, 0, 0.14))
    s.add(tube(8, 0.045, 0.045, 1.55, 1.95), m["steel"], at=(0.6, 0, 0.02))

    # ---------------------------------------------------------------- Bug und Boden: Staurohr am Rammbalken, Klappe
    s.add(segment((0.3, -2.75, 0.1), (0.36, -3.35, 0.14), 0.012, 6), m["steel"])
    s.add(box(0.22, 0.5, 0.03), m["armor"], at=(-0.14, 1.25, -deck(1.25)))


def build():
    s = Ship("scrap_raider")
    m = vf.materials(s.name, "fighter")
    m["hull"] = vf.toon(f"{s.name}_hull", palette.SCRAP_HULL)  # Schrottgrau statt des Blaugraus der Klasse
    m["steel_flat"] = vf.toon(f"{s.name}_steel_flat", palette.STEEL)
    # Rost als rundes Material: flache Bleche bleiben flach (alle Kanten über 38°), bekommen aber nur das schmale
    # Kantenlicht; das breite der eckigen Materialien färbt sie auf dem Brett grau
    m["rust"] = vf.toon(f"{s.name}_rust", palette.RUST, round=True)

    # ---------------------------------------------------------------- Kantiger Rumpf, vorn stumpf
    s.add(loft([sec(y, octagon(w, h), dz=-0.04 if k == 0 else 0.0) for k, (y, w, h) in enumerate(HULL)]), m["hull"])

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
        sec(-1.35, superellipse(0.62, 0.6, 16, 3)),
        sec(-0.95, superellipse(0.68, 0.66, 16, 3)),
        sec(1.25, superellipse(0.68, 0.66, 16, 3)),
    ], 1)), m["accent_round"], at=(px, 0, pz))
    s.add(loft([sec(1.2, superellipse(0.64, 0.62, 16, 3)), sec(1.85, superellipse(0.44, 0.42, 16, 3))]), m["dark_round"], at=(px, 0, pz))
    s.add(box(0.05, 1.2, 0.09), m["glow"], at=(px - 0.335, 0.0, pz))
    s.add(rbox(0.5, 0.56, 0.5, 0.06, 1), m["dark"], at=(px, -1.55, pz))
    s.add(bellows(12, 0.2, -1.8, -2.02), m["armor_round"], at=(px, 0, pz))
    s.add(tube(16, 0.14, 0.12, -1.8, -3.35), m["steel"], at=(px, 0, pz))
    s.add(tube(16, 0.21, 0.21, -2.1, -2.7), m["dark_round"], at=(px, 0, pz))
    s.add(tube(16, 0.22, 0.22, -3.3, -3.62), m["dark_round"], at=(px, 0, pz))
    s.add(tube(12, 0.11, 0.11, -3.62, -3.66), m["glow"], at=(px, 0, pz))

    # ---------------------------------------------------------------- Steuerbord: Stummelflügel mit Stachel und Brandstofftank
    s.add(plate([(0.5, -0.7), (1.55, -0.25), (1.66, 1.1), (0.5, 1.3)], 0.16, "xy"), m["rust"], at=(0, 0, 0.2), rot=(0, -6, 0))
    s.add(tube(12, 0.11, 0.0, -0.2, -1.25), m["steel"], at=(1.5, 0, 0.36))
    tx, ty = 1.02, 0.3
    tz = 0.28 + 0.1045 * tx + 0.19  # Oberseite des Flügels plus Radius, 0,03 eingesenkt
    s.add(canister(0.22, 1.5), m["accent_round"], at=(tx, ty, tz))
    for dy in (-0.42, 0.42):
        s.add(tube(16, 0.24, 0.24, ty + dy - 0.05, ty + dy + 0.05), m["dark_round"], at=(tx, 0, tz))

    # ---------------------------------------------------------------- Schiefe Rückenflosse, nach steuerbord geneigt
    s.add(plate([(-0.6, -0.12), (0.85, -0.12), (0.75, 0.82), (0.05, 0.94)], 0.12, "yz"), m["hull"], at=(-0.25, 0, 0.42), rot=(0, 15, 0))

    # ---------------------------------------------------------------- Flicken: Rostbleche, Fremdblech an der Steuerbordflanke
    s.add(patch(0.36, 0.8), m["rust"], at=(-0.26, -1.55, 0.415), rot=(-5, 0, 174))
    s.add(patch(0.26, 1.2, 0.14), m["rust"], at=(-0.61, 0.55, 0.405), rot=(2, -33, 0))
    s.add(patch(0.6, 1.1, 0.24), m["rust"], at=(-0.27, 0.35, -0.45), rot=(0, 2, -6))
    s.add(patch(0.42, 0.85, 0.2), m["armor"], at=(0.57, -1.575, 0.0), rot=(0, 90, -11.8))

    # ---------------------------------------------------------------- Unterseite: Kiel, Brandsatz am Gehänge unter dem Bug
    s.add(plate([(-2.1, 0.02), (1.5, 0.02), (1.3, -0.26), (-1.5, -0.3)], 0.16, "yz"), m["dark"], at=(0.25, 0, -0.42))
    s.add(box(0.2, 0.8, 0.08), m["dark"], at=(-0.2, -1.4, -0.4))
    s.add(canister(0.19, 1.1, nose=0.12), m["accent_round"], at=(-0.2, -1.4, -0.58))
    for y in (-1.7, -1.1):
        s.add(tube(16, 0.205, 0.205, y - 0.05, y + 0.05), m["dark_round"], at=(-0.2, 0, -0.58))

    # ---------------------------------------------------------------- Drei ungleiche Düsen
    # Die größte als Buckel obenauf auf einem kantigen Sockel, eine mittlere backbord unten und eine kleine steuerbord,
    # die unter dem Rumpf hängt und am weitesten hinausragt, damit sie von backbord zu sehen ist
    s.add(wedge(0.6, 1.1, 0.34, front=0.6), m["hull"], at=(0.06, 1.5, 0.5))
    vent(s, m, m["hull"], (0.06, 1.83, 0.67), 0.42, 0.38, slats=2)
    for r, y0, y1, dx, dz in ((0.34, 1.9, 2.75, 0.06, 0.3), (0.27, 2.0, 2.55, -0.42, -0.24), (0.2, 2.05, 2.88, 0.4, -0.4)):
        engine(s, m, r, y0, y1, dx=dx, dz=dz)
        engine_details(s, m, r, y0, y1, dx=dx, dz=dz)
    details(s, m, canopy, px, pz)
    return s.finish()
