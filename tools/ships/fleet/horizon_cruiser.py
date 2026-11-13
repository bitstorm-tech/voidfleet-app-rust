"""Horizon Cruiser (Kreuzer, im Spiel der Wachkreuzer), Fassung für die Nahaufnahme nach docs/stilregeln.md: gefaste
Primär- und Sekundärformen, glatte runde Teile, Kleinteile und Feinteile.

Primärformen: langer, schlanker Facettenrumpf, der zum Bug flach wie ein Spatel ausläuft; darauf ein großer
Zwillingsturm in Kornblumenblau und ein Rückendeck mit dem Brückenturm achtern (drei Blöcke, Fensterband, Mast mit
Licht). Zwei runde Gondeln an gepfeilten Pylonen mit blauem Lichtband und Akzent-Einlauf, zwei schräge Heckflossen,
dunkler Kiel, vier Düsen.
Detailzonen mit Zweck, gröber und sparsamer als an der Dreadnought, weil das Schiff im Spiel als Paar zu 0,72 steht;
dazwischen ruhige Flächen (Bugdeck vor dem Turm, Rückendeck, Gondeln, Flossen):
- Bug: Bugpanzer-Kappe um die Spatelspitze
- Turm: größer als in der groben Fassung, mit `parts.turret_details`
- Rückendeck: Deckplatte vor der Brücke, Maschinenlüfter hinter ihr
- Brücke: Brückenflügel unter den Seitenfenstern, Mast mit Radarplatte
- Heck: Gehäusesegment und Düsenkranz an allen vier Düsen (`parts.engine_details`)
- Unterseite: Kinn unter dem Bug mit zweiflügeligem Bootstor nach vorn, aus dem der Kiel wächst; drei Spanten je Seite
  auf der unteren Fase vor den Pylonen
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.

Kleinteile (`details`, wie an der Iron Dreadnought): Fensterreihen aus Einzelfenstern am Brückenturm und an der Wand
des Rückendecks, Flak hinter dem Turm, Querträger mit Nadelantennen am Mast, Peitschenantenne und Schüssel auf dem
Brückenturm, Geländer und Leiter, Kisten hinter der Brücke, Kabelkanal an der Flanke zum Pylon, Kühlrippen an allen
vier Düsen, Positionslichter an Bug und Flossenspitzen.

Feinteile (`fine`, die dritte Stufe für die Nahaufnahme, an einem Paar ab etwa 0,018; sie zeigen, wie groß das Schiff
ist, und wo man es betritt, wartet und verschraubt): am Bug Schrauben an der Kappe, Poller, Warnband unter den Mündungen
und die Luke zur Vorpiek; Schrauben im Barbettenring; an der Flanke Munitionsluke neben dem Turm, Schellen am Kabelkanal,
Kühlmittelrohr vom Anschlusskasten über die Schulter, achtern die runde Dockluke zwischen zwei Warnbändern; Laufgang mit
Reling und Tür neben dem Rückendeck; Luke und Schrauben auf der Deckplatte; am Brückenturm Fenstersimse, Konsolen unter
den Brückenflügeln, Tür zum Dach und Steigeisen am Mast; an den Gondeln Spannbänder mit Schrauben, eine gewölbte
Wartungsklappe, Fassungen des Lichtbands, Einlaufkegel mit Strebenkreuz; an den Flossen Ruderspalt mit Scharnieren und
Wurzelbeschlag; Stellstreben an den Düsenglocken. Dazu Beschriftung als flache Geometrie: die Kennnummer 31 an den
Innenseiten der Flossen (von der Seite liest man die der abgewandten Flosse) und an der Rückwand des Brückenturms (von
hinten oben), Warnbänder am Bug, an der Dockluke und hinter dem Maschinenlüfter.
"""

import math

import vf
from parts import bolts, cover, engine, engine_details, flak, hatch, number, rail, ring_at, stripes, turret, turret_details, vent, windows
from vf import Ship, box, chamfer, dish, loft, ngon, octagon, pipe, plate, post, rect, sec, segment, smooth, sphere, sweep, tube, wedge

NX = 1.18  # Gondeln: Abstand zur Mittelachse
NZ = 0.02  # Gondeln: Höhe der Achse
# Rumpf: Querschnitte (y, Breite, Höhe, Versatz in z) des Profils `hull`, linear verbunden
HULL = ((-3.85, 0.5, 0.1, 0.0), (-3.0, 1.0, 0.32, 0.06), (-2.45, 1.2, 0.5, 0.05), (-1.5, 1.45, 0.72, 0.0), (-0.4, 1.56, 0.8, 0.0),
        (1.7, 1.56, 0.8, 0.0), (2.9, 1.3, 0.7, 0.0), (3.4, 1.0, 0.56, 0.0))
TURRET = {"at": (0, -2.05, 0.3), "r": 0.42, "barrel": 1.12}
ROUND = 32  # Segmente der großen runden Teile (Gondeln, Düsen)
# Düsen (Radius, y vorn, y Düsenende, Achse x und z): an den Gondeln und am Heck
ENGINES = ((0.27, 2.35, 3.2, NX, NZ), (0.24, 3.25, 4.0, 0.34, -0.02))
# Heckflossen (Steuerbord): Umriss (y, z) in ihrer Ebene, Dicke, Fuß und Neigung nach außen
FIN = [(1.45, 0.0), (2.95, 0.0), (3.35, 0.9), (2.55, 0.94)]
FIN_AT = (0.4, 0, 0.3)
FIN_LEAN = 28


def hull(w, h):
    """Rumpfprofil: flaches Deck, obere Fase, senkrechte Flanke, steile untere Fase (fängt in der Karte Licht),
    schmaler Boden."""
    return [(w / 2, -0.05 * h), (w / 2, 0.3 * h), (0.33 * w, h / 2), (-0.33 * w, h / 2), (-w / 2, 0.3 * h), (-w / 2, -0.05 * h),
            (-0.22 * w, -h / 2), (0.22 * w, -h / 2)]


def hull_at(y):
    """Breite, Höhe und Versatz in z des Rumpfprofils bei y."""
    for (y0, *a), (y1, *b) in zip(HULL, HULL[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / (y1 - y0)
            return tuple(p + t * (q - p) for p, q in zip(a, b))
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def details(s, m, z):
    """Kleinteile (Modulkommentar); `z` ist das Dach des Brückenturms."""
    roof1, roof2 = 1.07, 1.43  # Dächer der beiden unteren Blöcke des Brückenturms

    # ---------------------------------------------------------------- Fensterreihen: Brückenturm und Wand des Rückendecks
    windows(s, m, [(0.49, 1.5 + 0.12 * k, 0.9) for k in range(5)])
    windows(s, m, [(0.37, 1.6 + 0.12 * k, 1.27) for k in range(4)])
    windows(s, m, [(-0.3 + 0.2 * k, 2.2, 0.9) for k in range(4)], out="y")
    for y0, n in ((-0.6, 4), (0.25, 3)):
        windows(s, m, [(0.41 + 0.02 * (y0 + 0.12 * k + 0.9) / 3.4, y0 + 0.12 * k, 0.5) for k in range(n)])

    # ---------------------------------------------------------------- Flak hinter dem Turm, an der Deckkante
    w, h, dz = hull_at(-1.2)
    flak(s, m, (0.39, -1.2, dz + h / 2), 25)

    # ---------------------------------------------------------------- Antennen: Querträger am Mast, Peitsche, Schüssel
    s.add(box(0.56, 0.05, 0.05), m["hull"], at=(0, 1.72, z + 0.62))
    s.add(post(6, 0.012, 0.006, z + 0.6, z + 1.0), m["dark_round"], at=(0.26, 1.72, 0), mirror=True)
    s.add(segment((0.28, 2.0, roof2 - 0.02), (0.32, 2.4, roof2 + 0.55), 0.012, 6), m["dark_round"])
    s.add(post(8, 0.03, 0.025, roof2 - 0.02, roof2 + 0.12), m["dark_round"], at=(-0.2, 2.0, 0))
    s.add(dish(0.11, 0.045, n=12, rings=3, thickness=0.02), m["armor_round"], at=(-0.2, 2.0, roof2 + 0.11), rot=(-40, 0, 150))

    # ---------------------------------------------------------------- Geländer und Leiter am Brückenturm, Kisten dahinter
    rail(s, m, (0.45, 1.38, roof1), (0.45, 2.16, roof1), 4)
    rail(s, m, (-0.45, 2.16, roof1), (0.45, 2.16, roof1), 5, mirror=False)
    rail(s, m, (0.63, 1.435, z - 0.15), (0.63, 1.645, z - 0.15), 2)
    for dx in (-0.05, 0.05):
        s.add(box(0.014, 0.014, 0.42), m["dark"], at=(0.42 + dx, 2.207, 0.87))
    for k in range(5):
        s.add(box(0.1, 0.014, 0.014), m["dark"], at=(0.42, 2.207, 0.7 + 0.08 * k))
    s.add(box(0.18, 0.2, 0.13), m["armor"], at=(0.12, 2.36, 0.72))
    s.add(box(0.14, 0.16, 0.1), m["dark"], at=(-0.14, 2.35, 0.705))

    # ---------------------------------------------------------------- Kabelkanal an der Flanke, vom Turm zum Pylon
    pts = []
    for y in (-1.9, -1.5, -0.45):
        w, h, dz = hull_at(y)
        pts.append((w / 2 + 0.02, y, dz + 0.3 * h - 0.09))
    s.add(sweep(pts, rect(0.06, 0.06)), m["dark"], mirror=True)
    s.add(box(0.1, 0.2, 0.12), m["armor"], at=(pts[-1][0] + 0.01, -0.38, pts[-1][2]), mirror=True)

    # ---------------------------------------------------------------- Kühlrippen an den Düsen (Gondeln und Heck)
    for r, y0, y1, dx, dz in ENGINES:
        for t in (0.38, 0.44, 0.5):
            y = y0 + (y1 - y0) * t
            s.add(tube(ROUND, r * 1.14, r * 1.14, y, y + 0.025, dx, dz), m["steel"], mirror=True)

    # ---------------------------------------------------------------- Positionslichter: Flossenspitzen und Bug
    s.add(sphere(0.035, 1), m["glow"], at=(0.822, 3.35, 1.095), mirror=True)
    s.add(sphere(0.03, 1), m["glow"], at=(0, -3.93, 0.0))


def top(y):
    """Höhe des Rumpfdecks bei y."""
    _, h, dz = hull_at(y)
    return dz + h / 2


def shoulder(y, t=0.5, up=0.0):
    """Punkt auf der oberen Fase des Rumpfs (der Schulter, Steuerbord) bei y, `t` von der Flanke (0) zur Deckkante (1),
    um `up` über der Fläche, und ihre Neigung in Grad (für `rot`)."""
    w, h, dz = hull_at(y)
    tilt = math.atan2(0.2 * h, 0.17 * w)
    return (w * (0.5 - 0.17 * t) + up * math.sin(tilt), y, dz + h * (0.3 + 0.2 * t) + up * math.cos(tilt)), math.degrees(tilt)


def door(s, m, at, w, h, rot, mirror=False):
    """Tür in einer Wand, lokal Breite x, Höhe y und +z nach außen, `at` die Mitte auf der Wandfläche: dunkler Rahmen,
    Blatt (`armor`), Griff."""
    g = Ship("door")
    g.add(box(w, h, 0.02), m["dark"], at=(0, 0, 0.002))
    g.add(box(w - 0.03, h - 0.03, 0.02), m["armor"], at=(0, 0, 0.008))
    g.add(box(0.012, 0.035, 0.012), m["steel_flat"], at=(w / 2 - 0.035, 0, 0.022))
    s.place(g, at=at, rot=rot, mirror=mirror)
    g.bm.free()


def fine(s, m, z, nac):
    """Feinteile und Beschriftung (Modulkommentar); `z` ist das Dach des Brückenturms, `nac` sind die Ringe einer Gondel
    um ihre Achse."""
    roof1, roof2 = 1.07, 1.43  # Dächer der beiden unteren Blöcke des Brückenturms

    # ---------------------------------------------------------------- Bug: Schrauben der Kappe, Poller, Warnband, Luke
    # Das Bugdeck steigt nach achtern an, alles liegt in seiner Neigung. Das Warnband markiert die Zone unter den
    # Mündungen, dahinter die Luke zur Vorpiek
    a, b = top(HULL[0][0]) + 0.04, top(-3.2) + 0.04  # Oberseite der Bugkappe an ihren Enden (−3,92 und −3,2)
    bolts(s, m["steel"], [(x, -3.25, a + (b - a) * 0.67 / 0.72) for x in (-0.22, -0.11, 0.0, 0.11, 0.22)],
          (math.degrees(math.atan2(b - a, 0.72)), 0, 0))
    s.add(post(8, 0.022, 0.028, -0.01, 0.05), m["steel"], at=(0.24, -3.1, top(-3.1)), mirror=True)
    lean = math.degrees(math.atan2(top(-2.45) - top(-3.0), 0.55))
    stripes(s, m, (0, -2.9, top(-2.9)), 0.07, 0.56, rot=(0, -lean, 90))
    hatch(s, m, (0, -2.7, top(-2.7)), 0.2, 0.2, rot=(lean, 0, 0))

    # ---------------------------------------------------------------- Turm: Schrauben im Barbettenring
    x, y, zt = TURRET["at"]
    r = 1.13 * TURRET["r"]
    bolts(s, m["steel"], [(x + r * math.cos(math.radians(a)), y + r * math.sin(math.radians(a)), zt + 0.05) for a in range(15, 360, 30)])

    # ---------------------------------------------------------------- Flanke vorn: Munitionsluke, Schellen, Rohr
    # Die Schulter (obere Fase) schaut die Seitenkamera fast senkrecht an: Munitionsluke neben dem Turm (der Rumpf
    # läuft hier zum Bug zu, sie folgt ihm), Schellen am Kabelkanal zwischen den Spanten, Schrauben am Anschlusskasten;
    # von ihm läuft ein Kühlmittelrohr über die Schulter nach achtern und taucht vor der Flosse in den Rumpf
    p, tilt = shoulder(-1.72, 0.55)
    (xa, _, za), (xb, _, zb) = shoulder(-1.84, 0.55)[0], shoulder(-1.6, 0.55)[0]
    hatch(s, m, p, 0.17, 0.24, rot=(math.degrees(math.atan2(zb - za, 0.24)), tilt, -math.degrees(math.atan2(xb - xa, 0.24))), mirror=True)
    for y in (-1.72, -1.25, -0.8):
        w, h, dz = hull_at(y)
        s.add(box(0.08, 0.035, 0.09), m["armor"], at=(w / 2 + 0.015, y, dz + 0.3 * h - 0.09), mirror=True)
        bolts(s, m["steel_flat"], [(w / 2 + 0.055, y, dz + 0.3 * h - 0.09)], "x", r=0.01, mirror=True)
    w, h, dz = hull_at(-0.45)
    bolts(s, m["steel_flat"], [(w / 2 + 0.08, -0.38 + dy, dz + 0.3 * h - 0.09 + du) for dy in (-0.07, 0.07) for du in (-0.035, 0.035)], "x",
          r=0.01, mirror=True)
    (px, _, pz), tilt = shoulder(0.0, 0.3, 0.014)
    s.add(pipe([(w / 2 + 0.02, -0.34, pz - 0.11), (px, -0.34, pz), (px, 1.35, pz), (px - 0.03, 1.38, pz - 0.05)], 0.018, 6), m["steel"],
          mirror=True)
    for y in (0.15, 0.6, 1.05):
        s.add(box(0.07, 0.03, 0.044), m["armor"], at=(px, y, pz), rot=(0, tilt, 0), mirror=True)

    # ---------------------------------------------------------------- Flanke achtern: Dockluke zwischen zwei Warnbändern
    # Rund, eckig schattiert (einen runden Deckel an rundem Material hielte die Verwitterung für eine Mündung); die
    # Flanke läuft hier zum Heck zu
    yaw = math.degrees(math.atan2((HULL[5][1] - HULL[6][1]) / 2, HULL[6][0] - HULL[5][0]))
    c, d = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
    w, h, dz = hull_at(2.25)
    at = (w / 2, 2.25, dz + 0.125 * h)
    for n, r, y0, y1, mat in ((20, 0.09, -0.02, 0.025, "armor"), (20, 0.07, -0.02, 0.033, "dark"), (10, 0.028, 0.025, 0.046, "steel_flat")):
        s.add(tube(n, r, r, y0, y1), m[mat], at=at, rot=(0, 0, yaw - 90), mirror=True)
    bolts(s, m["steel_flat"], [(at[0] + 0.025 * c - 0.08 * math.cos(a) * d, at[1] + 0.025 * d + 0.08 * math.cos(a) * c, at[2] + 0.08 * math.sin(a))
                               for a in (math.radians(22.5 + 45 * k) for k in range(8))], (0, 90, yaw), r=0.009, mirror=True)
    for dy in (-0.16, 0.16):
        w, h, dz = hull_at(2.25 + dy)
        stripes(s, m, (w / 2, 2.25 + dy, dz + 0.125 * h), 0.04, 0.16, rot=(90, 0, 90 + yaw), mirror=True)

    # ---------------------------------------------------------------- Laufgang neben dem Rückendeck: Reling, Tür
    rail(s, m, (0.5, -0.38, 0.4), (0.5, 1.3, 0.4), 6)
    door(s, m, (0.41 + 0.02 * 0.9 / 3.4, 0.0, 0.48), 0.1, 0.16, (90, 0, 90), mirror=True)

    # ---------------------------------------------------------------- Rückendeck: Luke, Schrauben, Warnband
    # Auf der Deckplatte die Ladeluke und Schrauben in den Ecken; hinter dem Maschinenlüfter ein Warnband auf dem
    # abfallenden Ende
    hatch(s, m, (0, 0.36, 0.685), 0.22, 0.26)
    bolts(s, m["steel"], [(sx * 0.155, y, 0.685) for sx in (-1, 1) for y in (-0.715, 0.665)])
    stripes(s, m, (0, 2.93, 0.66 - 0.1 * (2.93 - 2.5) / 0.55), 0.06, 0.34, rot=(0, math.degrees(math.atan2(0.1, 0.55)), 90))

    # ---------------------------------------------------------------- Brückenturm: Simse, Konsolen, Tür, Steigeisen, Nummer
    # Simse unter allen Fensterreihen (auch an der Wand des Rückendecks), Konsolen unter den Brückenflügeln, eine Tür
    # vom Dach des unteren Blocks in den mittleren, Steigeisen an der Rückseite des Masts; an der Rückwand des obersten
    # Blocks die Kennnummer, lesbar von hinten oben (neben der Schüssel, die davor steht)
    number(s, m["paint"], "31", (0.05, 1.91, 1.585), 0.18, (90, 0, 180))
    s.add(box(0.03, 0.6, 0.014), m["dark"], at=(0.498, 1.74, 0.864), mirror=True)
    s.add(box(0.03, 0.48, 0.014), m["dark"], at=(0.378, 1.78, 1.234), mirror=True)
    s.add(box(0.7, 0.03, 0.014), m["dark"], at=(0, 2.208, 0.864))
    for y0, n in ((-0.6, 4), (0.25, 3)):
        y = y0 + 0.06 * (n - 1)
        s.add(box(0.03, 0.12 * n, 0.014), m["dark"], at=(0.418 + 0.02 * (y + 0.9) / 3.4, y, 0.464), mirror=True)
    for y in (1.47, 1.61):
        s.add(plate([(0.0, 0.0), (0.24, 0.0), (0.0, -0.2)], 0.03, "xz"), m["hull"], at=(0.365, y, roof2 + 0.005), mirror=True)
    door(s, m, (0, 2.095, roof1 + 0.115), 0.1, 0.17, (90, 0, 180))
    for k in range(8):
        zk = z + 0.12 + 0.08 * k
        s.add(box(0.07, 0.03, 0.012), m["steel_flat"], at=(0, 1.828 - 0.03 * (zk - z) / 0.85, zk))

    # ---------------------------------------------------------------- Gondeln: Spannbänder, Klappe, Fassungen, Einlauf
    # Zwei Spannbänder mit Schrauben vor und hinter dem Lichtband, dessen Enden in dunklen Fassungen stecken; eine
    # gewölbte Wartungsklappe oben außen; im Einlauf ein Kegel auf einem Strebenkreuz
    for y in (-0.46, 1.66):
        s.add(loft([ring_at(nac, y - 0.035, 0.012), ring_at(nac, y + 0.035, 0.012)]), m["armor_round"], at=(NX, 0, NZ), mirror=True)
        r = ring_at(nac, y)[0].x + 0.012
        for a in range(0, 360, 45):
            c, d = math.cos(math.radians(a + 22.5)), math.sin(math.radians(a + 22.5))
            bolts(s, m["steel_flat"], [(NX + r * c, y, NZ + r * d)], (0, 67.5 - a, 0), r=0.011, mirror=True)
    s.add(cover(nac, 0.55, 0.85, 4, 7), m["armor_round"], at=(NX, 0, NZ), mirror=True)
    for y in (0.58, 0.82):
        r = ring_at(nac, y)[0].x + 0.01
        for a in (49, 75):
            bolts(s, m["steel_flat"], [(NX + r * math.cos(math.radians(a)), y, NZ + r * math.sin(math.radians(a)))], (0, 90 - a, 0), r=0.01,
                  mirror=True)
    for y in (-0.37, 1.47):
        s.add(box(0.07, 0.05, 0.11), m["dark"], at=(NX + 0.3, y, NZ + 0.04), mirror=True)
    s.add(tube(12, 0.0, 0.06, -1.34, -1.2), m["steel"], at=(NX, 0, NZ), mirror=True)
    for size in ((0.33, 0.02, 0.02), (0.02, 0.02, 0.33)):
        s.add(box(*size), m["steel_flat"], at=(NX, -1.225, NZ), mirror=True)

    # ---------------------------------------------------------------- Flossen: Ruderspalt, Beschlag, Kennnummer
    # In der Ebene der Flosse gebaut und mit ihr geneigt: Ruderspalt parallel zur Hinterkante mit drei Scharnieren (beide
    # gehen durch die Flosse und zeigen sich auf beiden Seiten), Beschlag an der Wurzel mit Schrauben außen. Die
    # Kennnummer steht an den Innenseiten, je Seite für sich gesetzt: Die Seitenkamera sieht die Innenseite der
    # abgewandten Flosse fast senkrecht, die Außenseite der nahen nur streifend
    def gap(zz):
        return 2.71 + 0.4 * zz / 0.9

    g = Ship("fin")
    g.add(plate([(gap(0.14) - 0.008, 0.14), (gap(0.14) + 0.008, 0.14), (gap(0.84) + 0.008, 0.84), (gap(0.84) - 0.008, 0.84)], 0.112, "yz"),
          m["dark"])
    for zz in (0.25, 0.48, 0.71):
        g.add(box(0.124, 0.05, 0.028), m["steel_flat"], at=(0, gap(zz), zz))
    g.add(box(0.124, 0.97, 0.08), m["armor"], at=(0, 2.215, 0.15))
    bolts(g, m["steel_flat"], [(0.062, y, 0.15) for y in (1.83, 2.02, 2.215, 2.41, 2.6)], "x", r=0.011)
    s.place(g, at=FIN_AT, rot=(0, FIN_LEAN, 0), mirror=True)
    g.bm.free()
    for side, turn in ((1, -90), (-1, 90)):
        g = Ship("tail")
        number(g, m["paint"], "31", (-side * 0.05, 2.5, 0.5), 0.3, (90, 0, turn))
        s.place(g, at=(side * FIN_AT[0], FIN_AT[1], FIN_AT[2]), rot=(0, side * FIN_LEAN, 0))
        g.bm.free()

    # ---------------------------------------------------------------- Düsen: Stellstreben über der Taille der Glocke
    for r, y0, y1, dx, dz in ENGINES:
        ya = y0 + 0.6 * (y1 - y0)
        for k in range(6):
            c, d = math.cos(math.pi * (2 * k + 1) / 6), math.sin(math.pi * (2 * k + 1) / 6)
            s.add(segment((dx + 1.1 * r * c, ya, dz + 1.1 * r * d), (dx + 0.93 * r * c, y1 - 0.03, dz + 0.93 * r * d), 0.012, 6), m["steel"],
                  mirror=True)


def build():
    s = Ship("horizon_cruiser")
    m = vf.materials(s.name, "cruiser")
    # Langer Facettenrumpf, vorn flach wie ein Spatel; Bugpanzer als Kappe um die Spitze, rundum 0,04 darüber
    s.add(chamfer(loft([sec(y, hull(w, h), dz=dz) for y, w, h, dz in HULL]), 0.035), m["hull"])
    cap = [(y, *hull_at(max(y, HULL[0][0]))) for y in (-3.92, -3.2)]
    s.add(chamfer(loft([sec(y, hull(w + 0.08, h + 0.08), dz=dz) for y, w, h, dz in cap]), 0.015), m["armor"])
    # Rückendeck vom Turm bis achtern, Deckplatte vor der Brücke, Maschinenlüfter dahinter
    s.add(chamfer(loft([sec(-1.45, octagon(0.4, 0.2), dz=-0.05), sec(-0.9, octagon(0.82, 0.36)), sec(2.5, octagon(0.86, 0.38)),
                        sec(3.05, octagon(0.5, 0.26), dz=-0.04)]), 0.025), m["armor"], at=(0, 0, 0.47))
    s.add(chamfer(plate([(-0.19, -0.75), (0.19, -0.75), (0.19, 0.7), (-0.19, 0.7)], 0.05, "xy"), 0.012), m["hull"], at=(0, 0, 0.66))
    vent(s, m, m["armor"], (0, 2.66, 0.631), 0.42, 0.34, slats=2, rot=(-10, 0, 0))
    # Großer Zwillingsturm auf dem flachen Bug
    turret(s, m, **TURRET)
    turret_details(s, m, **TURRET)
    # Brückenturm achtern: drei Blöcke mit schräger Front, Fensterband, Brückenflügel, Mast mit Radarplatte und Licht
    z = 0.65
    for w, l, h, y, edge in ((0.98, 1.3, 0.42, 1.55, 0.03), (0.74, 0.95, 0.36, 1.62, 0.03), (0.52, 0.62, 0.3, 1.6, 0.02)):
        s.add(chamfer(wedge(w, l, h), edge), m["hull"], at=(0, y, z + h / 2))
        z += h
    s.add(box(0.42, 0.04, 0.08), m["window"], at=(0, y - l / 2 + 0.1, z - 0.12))
    s.add(box(0.04, 0.34, 0.08), m["window"], at=(w / 2 + 0.01, y, z - 0.12), mirror=True)
    s.add(chamfer(box(0.5, 0.26, 0.15), 0.02), m["hull"], at=(0.41, 1.54, z - h + 0.075), mirror=True)
    s.add(post(16, 0.1, 0.07, z, z + 0.85), m["hull_round"], at=(0, 1.72, 0))
    s.add(box(0.46, 0.05, 0.2), m["armor"], at=(0, 1.63, z + 0.42), rot=(-15, 0, 0))
    s.add(sphere(0.09, 1), m["glow"], at=(0, 1.72, z + 0.9))
    # Gondeln an gepfeilten Pylonen: Einlauf in Akzentfarbe, Lichtband außen, Düse
    s.add(chamfer(plate([(0.55, -0.4), (NX, 0.0), (NX, 1.9), (0.55, 1.75)], 0.18, "xy"), 0.03), m["hull"], at=(0, 0, NZ), mirror=True)
    nac = smooth([sec(-0.95, ngon(ROUND, 0.2)), sec(-0.55, ngon(ROUND, 0.29)), sec(1.6, ngon(ROUND, 0.3)), sec(2.5, ngon(ROUND, 0.27))], 1)
    s.add(loft(nac), m["hull_round"], at=(NX, 0, NZ), mirror=True)
    s.add(loft([sec(-1.2, ngon(ROUND, 0.27)), sec(-0.95, ngon(ROUND, 0.31)), sec(-0.62, ngon(ROUND, 0.32))]), m["accent_round"], at=(NX, 0, NZ),
          mirror=True)
    s.add(loft([sec(-1.22, ngon(ROUND, 0.17)), sec(-1.0, ngon(ROUND, 0.17))]), m["dark_round"], at=(NX, 0, NZ), mirror=True)
    s.add(box(0.05, 1.8, 0.08), m["glow"], at=(NX + 0.3, 0.55, NZ + 0.04), mirror=True)
    # Heckflossen, schräg nach außen
    s.add(chamfer(plate(FIN, 0.1, "yz"), 0.025), m["accent"], at=FIN_AT, rot=(0, FIN_LEAN, 0), mirror=True)
    # Unterseite: Kinn unter dem Bug mit zweiflügeligem Bootstor nach vorn, der Kiel wächst aus ihm
    s.add(chamfer(wedge(0.56, 0.9, 0.26), 0.025), m["hull"], at=(0, -2.45, -0.21), rot=(180, 0, 0))
    s.add(box(0.2, 0.06, 0.13), m["dark"], at=(0.13, -2.9, -0.24), mirror=True)
    s.add(chamfer(plate([(-2.5, 0.0), (2.6, 0.0), (2.85, -0.56), (0.5, -0.6), (-1.6, -0.44)], 0.16, "yz"), 0.03), m["dark"], at=(0, 0, -0.22))
    # Drei Spanten je Seite auf der unteren Fase vor den Pylonen, zwischen deren gefasten Kanten
    for y in (-1.9, -1.45, -1.0):
        w, h, dz = hull_at(y)
        x0, z0, x1, z1 = 0.22 * w + 0.05, dz - h / 2, w / 2 - 0.05, dz - 0.05 * h
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.1, "xz"), m["hull"], at=(0, y, 0), mirror=True)
    # Vier Düsen: an den Gondeln und zwei am Heck, so weit nach achtern, dass ihr Gehäusesegment hinter dem Heck liegt
    for r, y0, y1, dx, dz in ENGINES:
        engine(s, m, r, y0, y1, dx=dx, dz=dz, n=ROUND, mirror=True)
        engine_details(s, m, r, y0, y1, dx=dx, dz=dz, n=ROUND, mirror=True)
    details(s, m, z)
    fine(s, m, z, nac)
    return s.finish()
