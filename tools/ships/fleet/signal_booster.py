"""Signal Booster (im Spiel der Spotter, ein Kreuzer), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

Primärformen: kantiger Rumpf mit dunklem Gerätedeck (blaue Deckplatte), blauer Bauchplatte und einem
Lichtband an jeder Flanke. Darauf eine große Schüssel mit leuchtendem Cyan-Rand, blauer Mitte und Sender, auf einem
Gabelmast mit blauem Joch nach vorn oben gekippt. Um den Rumpf ein Signalring (Stahlband mit Cyan-Streifen ohne
Glühen, damit der Rand der Schüssel der Blickfang bleibt), den Bug und Heck über den Gondeln tragen; zwei Relaismasten
(achtern Backbord hoch, vorn Steuerbord niedrig) mit Relaiskasten und Licht; zwei Solarpaneele als ruhige Flächen an
Auslegern, nach außen geneigt; Kanzel vorn Backbord, Bugemitter, Seitengondeln an Streben mit blauem Band und je
einer Düse.
Detailzonen mit Zweck, grob, weil das Schiff im Spiel als Paar zu 0,72 steht; dazwischen ruhige Flächen (Rumpfflanken,
Bug, Paneele, Gondeln). Breiter wird es nicht: Das Paar ragt bei 0,72 über das Pad (dafür dürfte es höchstens etwa 3,5
breit sein), deshalb sitzen die Paneele näher am Rumpf als in der groben Fassung.
- Schüssel: größer als in der groben Fassung, Drehkranz zwischen Mast und Joch
- Signalring: Ringlager dort, wo der Ring in Bug und Heck läuft
- Paneele: Holm über jedem Paneel als Fortsetzung des Auslegers
- Heck: Gehäusesegment und Düsenkranz an beiden Düsen (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought; Waffen trägt das Schiff keine): Fensterreihen aus Einzelfenstern,
Geländer und Leiter am Gerätedeck, kleine Schüssel und Gerätekasten mit Peitschenantenne auf der Deckplatte achtern,
drei Streben vom Rand der großen Schüssel zum Sender, Querträger mit Nadelantennen an den Relaismasten, Wellenleiter
vom Gerätedeck zum Bugemitter, Kühlrippen an den Düsen, Positionslichter an den Außenkanten der Paneele.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme, an einem Paar ab etwa 0,018; sie zeigen, wie groß das Schiff
ist, wie Schüssel, Ring und Paneele gehalten und gestellt werden und wo man es betritt und wartet): acht Rippen auf
dem Rücken der Schüssel, an den beiden unteren je ein Stellzylinder vom Vordeck, Schrauben im Drehkranz, Lagerdeckel
an der Gabel; am Gerätedeck Fenstersimse, Türen und Schrauben auf der Deckplatte; auf dem Vordeck Schellen am
Wellenleiter, Luke und Warnband unter dem Rand der Schüssel; auf dem Achterdeck ein Landefeld für Drohnen (Kreis und
Striche) und eine Luke; Steigeisen an den Relaismasten; an der Flanke Fassungen
des Lichtbands und eine Wartungsluke; am Signalring Stoßlaschen mit Schrauben, Schrauben auf den Ringlagern und je
Seite zwei Abspannungen von der Gondel; an den Paneelen Zellenraster, Rahmenschrauben, Gelenk und Stellstrebe; an den
Gondeln Schrauben im blauen Band, eine gewölbte Wartungsklappe und Stellstreben an den Düsenglocken; am Heck die runde
Dockluke zwischen zwei Warnbändern. Dazu Beschriftung als flache Geometrie: die Kennnummer 56 an beiden Flanken und auf
der Deckplatte (lesbar von hinten oben), Warnbänder auf dem Vordeck und am Heck.
"""

import math

import palette
import vf
from mathutils import Euler, Vector
from parts import axle, beam, bolts, cover, engine, engine_details, hatch, number, on_ring, rail, ring_at, stripes, windows
from vf import (Ship, box, chamfer, dish, hoop, loft, octagon, plate, post, rect, rounded, sec, segment, smooth, sphere, superellipse, sweep, tip,
                tube, wedge)

RING = (0.15, 0.26, 1.79)  # Signalring: Mitte (y, z) und mittlerer Radius
ROUND = 48  # Segmente der großen Schüssel (Radius 1,22)
HOOP = 64  # Segmente des Signalrings (Radius 1,79)
MID = 24  # Segmente der Gondeln, Düsen, des Drehkranzes und des Bugemitters
DISH = (1.22, 0.4, 0.28)  # große Schüssel: Radius, Tiefe, Abstand des Scheitels vor der Kippachse
TILT = -40  # Kippung der Schüssel um ihre Achse (Grad um x): Öffnung nach vorn oben
PANEL = ((1.85, 1.15, 0.64), (0, 25, 0))  # Solarpaneel (Steuerbord): Mitte und Neigung nach außen
ENGINE = (0.24, 1.5, 2.1)  # Düse einer Gondel: Radius, y vorn, y Düsenende
MASTS = ((-0.5, 1.35, 2.3), (0.5, -1.95, 1.55))  # Relaismasten (x, y, Spitze): achtern Backbord hoch, vorn Steuerbord niedrig


def relay_mast(s, m, x, y, z0, z1):
    """Relaismast: Stab mit Relaiskasten auf halber Höhe und einem Licht an der Spitze."""
    s.add(post(16, 0.11, 0.075, z0, z1), m["hull_round"], at=(x, y, 0))
    s.add(chamfer(box(0.24, 0.24, 0.34), 0.025), m["dark"], at=(x, y, z0 + (z1 - z0) * 0.5))
    s.add(sphere(0.09, 1), m["glow"], at=(x, y, z1 + 0.05))


def door(s, m, at, w, h, rot, mirror=False):
    """Tür in einer Wand, lokal Breite x, Höhe y und +z nach außen, `at` die Mitte auf der Wandfläche: Rahmen (`armor`),
    dunkles Blatt, Griff."""
    g = Ship("door")
    g.add(box(w, h, 0.02), m["armor"], at=(0, 0, 0.002))
    g.add(box(w - 0.03, h - 0.03, 0.02), m["hull"], at=(0, 0, 0.008))
    g.add(box(0.012, 0.035, 0.012), m["steel_flat"], at=(w / 2 - 0.035, 0, 0.022))
    s.place(g, at=at, rot=rot, mirror=mirror)
    g.bm.free()


def details(s, m, deck, hub, nacelle):
    """Kleinteile (Modulkommentar); `deck(y)` ist die Höhe des Rumpfdecks, `hub` die Achse der Schüssel (x, y, z),
    `nacelle` die Achse der Gondeln (x, z)."""
    roof = deck(0.3) + 0.29  # Dach des Gerätedecks
    top = roof + 0.04  # Deckplatte darauf

    # ---------------------------------------------------------------- Gerätedeck: Fensterreihen, Geländer, Leiter
    windows(s, m, [(0.55, 0.3 + 0.12 * k, roof - 0.14) for k in range(5)])
    windows(s, m, [(-0.2 + 0.2 * k, 1.25, roof - 0.14) for k in range(3)], out="y")
    rail(s, m, (0.51, 0.05, roof), (0.51, 1.21, roof), 4)
    rail(s, m, (-0.51, 1.21, roof), (0.51, 1.21, roof), 5, mirror=False)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.34), m["dark"], at=(0.42 + dx, 1.257, roof - 0.16))
    for k in range(4):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.42, 1.257, roof - 0.28 + 0.08 * k))

    # ---------------------------------------------------------------- Geräte auf der Deckplatte achtern
    s.add(post(8, 0.03, 0.025, top - 0.02, top + 0.12), m["dark_round"], at=(0.3, 0.98, 0))
    s.add(dish(0.11, 0.045, n=12, rings=3, thickness=0.02), m["armor_round"], at=(0.3, 0.98, top + 0.11), rot=(-40, 0, 150))
    s.add(box(0.2, 0.24, 0.14), m["armor"], at=(-0.26, 0.95, top + 0.065))
    s.add(post(6, 0.012, 0.006, top + 0.1, top + 0.6), m["dark_round"], at=(-0.26, 1.02, 0))

    # ---------------------------------------------------------------- Große Schüssel: drei Streben vom Rand zum Sender
    g = Ship("feed")
    for a in (90, 210, 330):
        x, z = 1.2 * math.cos(math.radians(a)), 1.2 * math.sin(math.radians(a))
        g.add(segment((x, -0.68, z), (0, -1.14, 0), 0.018, 6), m["dark_round"])
    s.place(g, at=hub, rot=(TILT, 0, 0))
    g.bm.free()

    # ---------------------------------------------------------------- Relaismasten: Querträger mit Nadelantennen
    for x, y, z in ((-0.5, 1.35, 2.02), (0.5, -1.95, 1.32)):
        s.add(box(0.34, 0.035, 0.035), m["hull"], at=(x, y, z))
        for dx in (-0.15, 0.15):
            s.add(post(6, 0.01, 0.005, z, z + 0.3), m["dark_round"], at=(x + dx, y, 0))

    # ---------------------------------------------------------------- Wellenleiter vom Gerätedeck zum Bugemitter
    s.add(sweep([(0.2, y, deck(y) + 0.02) for y in (-0.5, -1.75, -2.48)], rect(0.06, 0.05)), m["dark"])

    # ---------------------------------------------------------------- Kühlrippen an den Düsen, Lichter an den Paneelen
    r, y0, y1 = ENGINE
    for t in (0.37, 0.44):
        y = y0 + (y1 - y0) * t
        s.add(tube(MID, r * 1.14, r * 1.14, y, y + 0.022, *nacelle), m["steel"], mirror=True)
    s.add(sphere(0.035, 1), m["glow"], at=(2.27, 1.15, 0.445), mirror=True)


def fine(s, m, profile, hub, pod, nacelle):
    """Feinteile und Beschriftung (Modulkommentar); `profile(y)` gibt halbe Breite und halbe Höhe des Rumpfs, `hub` ist
    die Achse der Schüssel (x, y, z), `pod` sind die Ringe einer Gondel um ihre Achse `nacelle` (x, z)."""

    def deck(y):
        return profile(y)[1]

    roof = deck(0.3) + 0.29  # Dach des Gerätedecks
    top = roof + 0.04  # Deckplatte darauf
    my = hub[1]
    yoke = hub[2] - 0.25
    r, depth, off = DISH
    # Neigung des Decks und der Flanke im vorderen Rumpf (er wird nach achtern höher und breiter)
    rise = math.degrees(math.atan2(deck(0.0) - deck(-1.0), 1.0))
    flare = math.degrees(math.atan2(profile(0.0)[0] - profile(-1.0)[0], 1.0))

    # ---------------------------------------------------------------- Schüssel: Rippen, Stellzylinder, Drehkranz, Lager
    # Acht Rippen auf dem Rücken, von der Nabe zum Rand; an den beiden unteren greift je ein Stellzylinder vom Vordeck
    # an (Hülse, Kolbenstange, Fuß), der die Schüssel kippt. Schrauben rund um den Drehkranz, Lagerdeckel außen an
    # den Zinken der Gabel
    def back(rho, up=0.0):
        """Tiefe (y in der Baugruppe der Schüssel) ihrer Schale im Abstand `rho` von der Achse, um `up` dahinter."""
        return -off - depth * (rho / r) ** 2 + up

    def height(rho):
        """Höhe der Rippen über der Schale: an der Nabe hoch, am Rand flach."""
        return 0.15 - 0.085 * (rho - 0.2) / 0.96

    g = Ship("ribs")
    steps = [0.2 + 0.96 * k / 5 for k in range(6)]
    rib = [(back(rho, -0.004), rho) for rho in steps] + [(back(rho, height(rho)), rho) for rho in reversed(steps)]
    for a in range(8):
        g.add(plate(rib, 0.03, "yz"), m["armor"], rot=(0, 22.5 + 45 * a, 0))
    s.place(g, at=hub, rot=(TILT, 0, 0))
    g.bm.free()
    tilt = Euler((math.radians(TILT), 0, 0)).to_matrix()
    rho = 0.78
    for side in (1, -1):
        eye = Vector(hub) + tilt @ Vector((side * rho * math.sin(math.radians(22.5)), back(rho, 0.5 * height(rho)),
                                           -rho * math.cos(math.radians(22.5))))
        foot = Vector((eye.x, eye.y + 0.06, deck(eye.y + 0.06)))
        s.add(chamfer(box(0.12, 0.12, 0.05), 0.012), m["armor"], at=foot + Vector((0, 0, 0.015)))
        s.add(segment(foot, foot.lerp(eye, 0.6), 0.034, 8), m["dark_round"])
        s.add(segment(foot, eye, 0.018, 6), m["steel"])
    for a in range(15, 360, 30):
        c, d = math.cos(math.radians(a)), math.sin(math.radians(a))
        bolts(s, m["steel_flat"], [(0.25 * c, my + 0.25 * d, yoke - 0.145)], (0, 90, a), r=0.011)
    s.add(tube(12, 0.07, 0.07, -0.01, 0.02), m["steel_flat"], at=(0.515, my, hub[2]), rot=(0, 0, -90), mirror=True)
    s.add(tube(6, 0.025, 0.025, 0.02, 0.035), m["dark"], at=(0.515, my, hub[2]), rot=(0, 0, -90), mirror=True)

    # ---------------------------------------------------------------- Gerätedeck: Simse, Türen, Deckplatte
    # Simse unter den Fensterreihen, je Seite eine Tür hinter den Fenstern; auf der Deckplatte Schrauben in den Ecken und
    # die Kennnummer, lesbar von hinten oben (eine Luke daneben las sich als dritte Ziffer)
    s.add(box(0.03, 0.6, 0.014), m["armor"], at=(0.558, 0.54, roof - 0.176), mirror=True)
    s.add(box(0.5, 0.03, 0.014), m["armor"], at=(0, 1.258, roof - 0.176))
    door(s, m, (0.55, 1.0, deck(1.0) + 0.095), 0.1, 0.17, (90, 0, 90), mirror=True)
    bolts(s, m["steel"], [(sx * 0.42, y, top) for sx in (-1, 1) for y in (0.14, 1.12)])
    number(s, m["paint"], "56", (0, 0.45, top), 0.3, (0, 0, 180))

    # ---------------------------------------------------------------- Relaismasten: Steigeisen
    # An der Rückseite, vom Deck bis unter den Relaiskasten
    for x, y, z1 in MASTS:
        z0 = deck(y) - 0.05
        for k in range(int(((z1 - z0) * 0.5 - 0.32) / 0.09)):
            z = z0 + 0.14 + 0.09 * k
            s.add(box(0.07, 0.03, 0.012), m["steel_flat"], at=(x, y + 0.118 - 0.035 * (z - z0) / (z1 - z0), z))

    # ---------------------------------------------------------------- Vordeck: Schellen am Wellenleiter, Luke, Warnband
    # Unter dem unteren Rand der Schüssel ein Warnband quer über das Deck (ihr Schwenkbereich), dahinter eine Luke
    for y in (-0.8, -1.2, -1.6):
        s.add(box(0.11, 0.035, 0.062), m["armor"], at=(0.2, y, deck(y) + 0.02))
    hatch(s, m, (-0.28, -1.3, deck(-1.3)), 0.22, 0.26, rot=(rise, 0, 0))
    stripes(s, m, (0, -1.66, deck(-1.66)), 0.06, 0.7, rot=(0, -rise, 90))

    # ---------------------------------------------------------------- Achterdeck: Drohnenfeld, Luke
    # Hinter dem Gerätedeck ein Landefeld für Drohnen (Kreis und vier Striche in `paint`, in der Neigung des Decks), auf
    # der Schräge zum Heck eine Luke
    fall = math.degrees(math.atan2(deck(1.7) - deck(0.7), 1.0))
    g = Ship("pad")
    g.add(hoop(32, 0.23, 0.25, 0.008), m["paint"], at=(0, 0, 0.002))
    for a in range(0, 360, 90):
        g.add(box(0.02, 0.1, 0.008), m["paint"], at=(0.15 * math.sin(math.radians(a)), 0.15 * math.cos(math.radians(a)), 0.002), rot=(0, 0, -a))
    s.place(g, at=(0, 1.52, deck(1.52)), rot=(fall, 0, 0))
    g.bm.free()
    hatch(s, m, (0, 2.07, deck(2.07)), 0.22, 0.26, rot=(math.degrees(math.atan2(deck(2.3) - deck(1.85), 0.45)), 0, 0))

    # ---------------------------------------------------------------- Flanke: Fassungen des Lichtbands, Luke, Kennnummer
    # Das Lichtband steckt an beiden Enden in einer dunklen Fassung, darüber eine Wartungsluke; hinter dem Band die
    # Kennnummer, je Seite für sich gesetzt (die Flanke läuft zum Bug zu)
    for y in (-1.41, -0.09):
        s.add(box(0.07, 0.05, 0.1), m["dark"], at=(profile(y)[0], y, 0.02), rot=(0, 0, -flare), mirror=True)
    hatch(s, m, (profile(-0.7)[0], -0.7, 0.165), 0.17, 0.24, rot=(0, 90, -flare), mirror=True)
    for side in (1, -1):
        number(s, m["paint"], "56", (side * profile(0.35)[0], 0.35, 0.07), 0.28, (90, 0, side * (90 - flare)))

    # ---------------------------------------------------------------- Signalring: Stoßlaschen, Lagerschrauben, Abspannung
    # Der Ring ist aus Segmenten verschraubt: sechs Stoßlaschen außen mit je vier Schrauben (nicht an Bug und Heck, wo
    # ihn die Ringlager fassen), Schrauben auf den Ringlagern; je Seite zwei Abspannungen von der Gondel zum Ring
    ry, rz, rr = RING
    g = Ship("joint")
    g.add(box(0.02, 0.14, 0.12), m["armor"], at=(rr + 0.072, 0, 0))
    bolts(g, m["steel_flat"], [(rr + 0.082, dy, dz) for dy in (-0.045, 0.045) for dz in (-0.035, 0.035)], "x", r=0.01)
    for a in (0, 45, 135, 180, 225, 315):
        s.place(g, at=(0, ry, rz), rot=(0, 0, a))
    g.bm.free()
    g = Ship("bearing")
    bolts(g, m["steel"], [(dx, dy, 0.13) for dx in (-0.11, 0.11) for dy in (-0.08, 0.08)])
    for x, side in ((0.75, -1), (0.72, 1)):
        s.place(g, at=(x, ry + side * math.sqrt(rr * rr - x * x), rz), rot=(0, 0, -side * math.degrees(math.asin(x / rr))), mirror=True)
    g.bm.free()
    gx, gz = nacelle
    for y in (-0.2, 0.6):
        s.add(segment((gx + 0.17, y, gz + 0.2), (rr - 0.03, y, rz - 0.01), 0.016, 6), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Solarpaneele: Zellenraster, Schrauben, Gelenk, Strebe
    # In der Ebene des Paneels gebaut und mit ihm geneigt: dunkle Stege zwischen den Zellen (die längs liegen tiefer als
    # die quer, damit sich nichts in einer Ebene überdeckt), Schrauben in den Ecken des Rahmens; darunter das Gelenk
    # am Ende des Auslegers und eine Stellstrebe vom Ausleger zur Innenkante
    g = Ship("panel")
    for x in (-0.205, 0.0, 0.205):
        g.add(box(0.01, 0.7, 0.006), m["dark"], at=(x, 0, 0.061))
    for y in (-0.19, 0.19):
        g.add(box(0.82, 0.01, 0.006), m["dark"], at=(0, y, 0.065))
    bolts(g, m["steel_flat"], [(sx * 0.428, sy * 0.368, 0.04) for sx in (-1, 1) for sy in (-1, 1)], r=0.011)
    s.place(g, at=PANEL[0], rot=PANEL[1], mirror=True)
    g.bm.free()
    s.add(tube(12, 0.07, 0.07, -0.12, 0.12), m["steel_flat"], at=(1.83, 1.15, 0.59), mirror=True)
    s.add(segment((1.15, 1.15, 0.55), (1.5, 1.15, 0.79), 0.018, 6), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Gondeln: Schrauben, Wartungsklappe, Stellstreben
    # Schrauben im blauen Band über die äußere Hälfte, hinter ihm oben außen eine gewölbte Wartungsklappe mit Schrauben
    ring = ring_at(pod, -0.435, 0.035)
    for k in range(-MID // 4, MID // 4 + 1, MID // 8):
        p, angle = on_ring(ring, k % MID)
        bolts(s, m["steel_flat"], [(gx + p.x, -0.435, gz + p.z)], (0, 90 - angle, 0), r=0.011, mirror=True)
    s.add(cover(pod, 0.05, 0.35, 2, 4), m["armor_round"], at=(gx, 0, gz), mirror=True)
    for y in (0.08, 0.32):
        ring = ring_at(pod, y, 0.01)
        for k in (2, 4):
            p = ring[k].lerp(ring[3], 0.25)
            bolts(s, m["steel_flat"], [(gx + p.x, y, gz + p.z)], (0, 90 - on_ring(ring, k)[1], 0), r=0.01, mirror=True)
    r, y0, y1 = ENGINE
    ya = y0 + 0.6 * (y1 - y0)
    for k in range(6):
        c, d = math.cos(math.pi * (2 * k + 1) / 6), math.sin(math.pi * (2 * k + 1) / 6)
        s.add(segment((gx + 1.1 * r * c, ya, gz + 1.1 * r * d), (gx + 0.93 * r * c, y1 - 0.03, gz + 0.93 * r * d), 0.012, 6), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Heck: Dockluke zwischen zwei Warnbändern
    # Rund, eckig schattiert (einen runden Deckel an rundem Material hielte die Verwitterung für eine Mündung)
    for n, radius, d0, d1, mat in ((20, 0.13, -0.02, 0.03, "armor"), (20, 0.1, -0.02, 0.04, "dark"), (10, 0.035, 0.03, 0.055, "steel_flat")):
        s.add(tube(n, radius, radius, d0, d1), m[mat], at=(0, 2.35, 0))
    bolts(s, m["steel_flat"], [(0.115 * math.cos(math.radians(a)), 2.38, 0.115 * math.sin(math.radians(a))) for a in range(0, 360, 45)], "y", r=0.01)
    stripes(s, m, (0.24, 2.35, 0), 0.05, 0.3, rot=(-90, 0, 0), mirror=True)


def build():
    s = Ship("signal_booster")
    m = vf.materials(s.name, "cruiser")
    m["cyan"] = vf.light(f"{s.name}_cyan", palette.CYAN)
    m["ring"] = vf.light(f"{s.name}_ring", palette.CYAN, glow=False)
    m["solar"] = vf.toon(f"{s.name}_solar", palette.GLASS)
    # Kantiger Rumpf mit flachem Deck; Oberkante bei y nach den Profilen
    sections = [(-2.5, 0.7, 0.42), (-1.75, 1.4, 0.8), (0.6, 1.72, 0.96), (1.8, 1.36, 0.82), (2.35, 0.96, 0.6)]
    s.add(chamfer(loft([sec(y, octagon(w, h)) for y, w, h in sections]), 0.035), m["hull"])

    def profile(y):
        """Halbe Breite und halbe Höhe des Rumpfs bei y."""
        for (y0, w0, h0), (y1, w1, h1) in zip(sections, sections[1:]):
            if y0 <= y <= y1:
                t = (y - y0) / (y1 - y0)
                return (w0 + (w1 - w0) * t) / 2, (h0 + (h1 - h0) * t) / 2

    def deck(y):
        return profile(y)[1]

    # Dunkles Gerätedeck mit schräger Front und blauer Deckplatte, blaue Bauchplatte
    s.add(chamfer(wedge(1.1, 1.9, 0.34), 0.03), m["dark"], at=(0, 0.3, deck(0.3) + 0.12))
    s.add(chamfer(plate([(-0.46, 0.1), (0.46, 0.1), (0.46, 1.16), (-0.46, 1.16)], 0.06, "xy"), 0.015), m["accent"], at=(0, 0, deck(0.3) + 0.3))
    s.add(chamfer(plate([(-0.42, -1.4), (0.42, -1.4), (0.5, 1.3), (-0.5, 1.3)], 0.12, "xy"), 0.03), m["accent"], at=(0, 0, -0.5))
    # Gabelmast: Stab, Drehkranz, Joch und zwei Zinken in Mintgrün, Achse
    my, yoke, hub = -0.25, deck(0.3) + 0.44, deck(0.3) + 0.69
    s.add(post(20, 0.17, 0.14, deck(my), yoke), m["dark_round"], at=(0, my, 0))
    s.add(post(MID, 0.25, 0.25, yoke - 0.21, yoke - 0.08), m["armor_round"], at=(0, my, 0))
    s.add(chamfer(box(1.0, 0.26, 0.18), 0.03), m["accent"], at=(0, my, yoke))
    s.add(chamfer(box(0.15, 0.22, hub - yoke + 0.14), 0.025), m["accent"], at=(0.44, my, (yoke + hub) / 2 + 0.02), mirror=True)
    s.add(axle(0.13, 0.82, 20), m["dark_round"], at=(0, my, hub))
    # Große Schüssel mit Cyan-Rand, blauer Mitte und Sender, um die Achse nach vorn oben gekippt
    g = Ship("dish")
    r, depth, off = DISH
    g.add(dish(r, depth, ROUND, 6, 0.06), m["hull_round"], at=(0, -off, 0))
    g.add(dish(0.48, depth * (0.48 / r) ** 2, ROUND, 3, 0.04), m["accent_round"], at=(0, -off - 0.03, 0))
    g.add(hoop(ROUND, r - 0.04, r + 0.08, 0.1), m["cyan"], at=(0, -off - depth, 0), rot=(90, 0, 0))
    g.add(tube(MID, 0.24, 0.2, -off + 0.02, 0.12), m["dark_round"])
    g.add(tube(16, 0.1, 0.09, -off - 0.84, -off), m["dark_round"])
    g.add(tube(16, 0.12, 0.12, -off - 1.04, -off - 0.8), m["cyan"])
    s.place(g, at=(0, my, hub), rot=(TILT, 0, 0))
    g.bm.free()
    # Signalring um den Rumpf über den Gondeln: Stahlband mit Cyan-Streifen oben, Bug und Heck tragen ihn in Ringlagern
    ry, rz, rr = RING
    s.add(hoop(HOOP, rr - 0.07, rr + 0.07, 0.16), m["steel"], at=(0, ry, rz))
    s.add(hoop(HOOP, rr - 0.05, rr + 0.05, 0.04), m["ring"], at=(0, ry, rz + 0.08))
    for x, side in ((0.75, -1), (0.72, 1)):
        angle = math.degrees(math.asin(x / rr))
        s.add(chamfer(box(0.3, 0.24, 0.26), 0.025), m["armor"], at=(x, ry + side * math.sqrt(rr * rr - x * x), rz), rot=(0, 0, -side * angle),
              mirror=True)
    # Relaismasten: achtern Backbord hoch, vorn Steuerbord niedrig
    for x, y, z1 in MASTS:
        relay_mast(s, m, x, y, deck(y) - 0.05, z1)
    # Solarpaneele an Auslegern über dem Ring: ruhige Fläche im Rahmen, der Ausleger läuft als Holm darüber weiter; er
    # beginnt im Rumpf an der Wand des Gerätedecks (gefast hing er sonst knapp über der Rumpffase)
    s.add(chamfer(beam((0.55, 1.15, deck(1.15)), (1.81, 1.15, 0.6), 0.16, 0.16), 0.025), m["dark"], mirror=True)
    for size, mat, edge in (((0.92, 0.8, 0.08), m["dark"], 0.02), ((0.82, 0.7, 0.12), m["solar"], 0.0), ((0.92, 0.08, 0.14), m["dark"], 0.015)):
        s.add(chamfer(box(*size), edge) if edge else box(*size), mat, at=PANEL[0], rot=PANEL[1], mirror=True)
    # Lichtband an jeder Flanke
    flank = math.degrees(math.atan2(profile(-0.1)[0] - profile(-1.4)[0], 1.3))
    s.add(box(0.05, 1.3, 0.07), m["glow"], at=(profile(-0.75)[0], -0.75, 0.02), rot=(0, 0, -flank), mirror=True)
    # Kanzel vorn Backbord, Bugemitter
    canopy = [(-2.36, 0.24, 0.14), (-2.18, 0.5, 0.26), (-1.98, 0.52, 0.27), (-1.84, 0.36, 0.16)]
    s.add(loft(smooth([sec(y, rounded(w, h, 0.08, 3), dx=-0.12, dz=deck(y) + h * 0.3) for y, w, h in canopy], 2)), m["glass"])
    s.add(tube(MID, 0.22, 0.2, -2.64, -2.42), m["dark_round"])
    s.add(tube(MID, 0.15, 0.11, -2.72, -2.6), m["cyan"])
    # Seitengondeln an Streben mit blauem Band und Düse
    gx, gz = 1.3, -0.25
    pod = smooth([tip(-1.0), sec(-0.85, superellipse(0.36, 0.36, MID, 2.4)), sec(-0.6, superellipse(0.54, 0.54, MID, 2.4)),
                  sec(-0.3, superellipse(0.6, 0.6, MID, 2.4)), sec(1.0, superellipse(0.62, 0.62, MID, 2.4)),
                  sec(1.6, superellipse(0.56, 0.56, MID, 2.4))], 2)
    s.add(loft(pod), m["hull_round"], at=(gx, 0, gz), mirror=True)
    s.add(loft([ring_at(pod, -0.55, 0.035), ring_at(pod, -0.32, 0.035)]), m["accent_round"], at=(gx, 0, gz), mirror=True)
    s.add(chamfer(box(0.46, 0.8, 0.22), 0.03), m["dark"], at=(0.9, 0.55, gz - 0.02), mirror=True)
    engine(s, m, *ENGINE, dx=gx, dz=gz, n=MID, mirror=True)
    engine_details(s, m, *ENGINE, dx=gx, dz=gz, n=MID, mirror=True)
    details(s, m, deck, (0, my, hub), (gx, gz))
    fine(s, m, profile, (0, my, hub), pod, (gx, gz))
    return s.finish()
