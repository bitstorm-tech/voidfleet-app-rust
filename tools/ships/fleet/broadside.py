"""Broadside (Schlachtschiff) im Clean-Toon-Stil (docs/stilregeln.md), nach einer Konzeptvorlage: ein langes, flaches
Linienschiff mit Pfeilbug, Mittelgrat, Plateau, Achterburg und vielen flachen Zwillingstürmen, olivgrau wie die Vorlage.

Primärformen: langer Rumpf mit senkrechten Flanken, der sich über das vordere Drittel gerade zu einem hohen, stumpfen
Bug verjüngt, darunter überall ein eingezogenes Unterschiff. Auf dem Deck läuft ein schmaler Grat vom Bug in die
Pfeilspitze des Plateaus (geschrägte Wände), achtern steigt die Achterburg in zwei Stufen an, die obere mit den
Brückenfenstern. Acht flache Zwillingstürme mit langen Rohren: ein Paar beiderseits des Grats am Bug, ein Paar auf
Sponsons an den Deckkanten vor dem Gürtel, einer auf dem Plateau, ein Paar auf den Seitendecks vor der Burg, einer
achtern auf der Burg nach hinten. Drei Heckdüsen.
Detailzonen mit Zweck, dazwischen ruhige Flächen (Bugdeck, Seitendecks, Gürtelplatten, Wände von Plateau und Burg):
- Bug: Bugpanzer-Kappe, Lüftungsgitter im Grat, Plattenlage vor den Bugtürmen; Kasematte an der Bugflanke mit Deckel
  und Lichtband, Kabelkanal bis zum Sponson
- Türme: Barbettenring, dunkle Dachplatte (`twin_details`); Rohrhülsen und Mündungsbremsen gehören zum Turm
- Mitteldeck: Lukenrahmen und Maschinenschacht mit zwei Walzen auf den Seitendecks, dunklere Lage auf der Pfeilspitze
  des Plateaus
- Flanke: Panzergürtel in zwei Gruppen, dazwischen eine Schleuse; RCS-Blöcke an den Heckecken
- Achterburg: Fensterband der Brücke, drei Strebepfeiler je Seite, Maschinenlüfter neben dem Heckturm
- Heck: Gehäusesegment und Düsenkranz je Düse
- Unterseite: Kiel, zweite Lage unter dem Boden, Spanten am eingezogenen Unterschiff
"""

import math

from mathutils import Vector

import palette
import vf
from parts import block, engine, engine_details, rcs, vent
from vf import Ship, box, loft, plate, post, tube, wedge

# Farben der Vorlage, fehlen in palette.py und stehen deshalb vorerst hier: Olivgrau statt des Blaugraus der Klasse
# (gemessen: Chroma 0,011–0,017, Farbton um 103), Türme in gedämpftem Stahlgrau statt des hellen Stahlblaus
OLIVE = palette.oklch(0.58, 0.018, 105)
OLIVE_ARMOR = palette.oklch(0.615, 0.018, 105)
GUNMETAL = palette.oklch(0.6, 0.022, 250)

ZD = 0.5  # Hauptdeck
ZP = 0.9  # Plateau
ZC = 1.1  # Achterburg, untere Stufe
ZS = 0.6  # Sponsons der Kantentürme
# Rumpf: Stationen (y, halbe Breite, Unterkante der senkrechten Flanke, Boden, halbe Bodenbreite). Die Deckkante läuft
# vom stumpfen Bug gerade bis y = -1,3 auf die volle Breite; darunter zieht das Unterschiff überall ein.
HULL = ((-5.3, 0.2, -0.25, -0.46, 0.1), (-3.2, 0.83, -0.4, -0.85, 0.5), (-1.3, 1.4, -0.25, -0.85, 0.95),
        (4.9, 1.4, -0.25, -0.85, 0.95), (5.4, 1.3, -0.2, -0.75, 0.86))
SPINE = [(0.0, -5.2), (0.16, -4.95), (0.28, -0.9), (-0.28, -0.9), (-0.16, -4.95)]
PLATEAU = [(0.28, -0.9), (0.66, -0.1), (0.66, 3.1), (-0.66, 3.1), (-0.66, -0.1), (-0.28, -0.9)]
CASTLE = [(0.7, 2.8), (1.05, 3.3), (1.05, 5.25), (-1.05, 5.25), (-1.05, 3.3), (-0.7, 2.8)]
# Zwillingstürme: Mitte der Barbette, Rohrlänge, Gierwinkel, gespiegelt
TURRETS = (((0.62, -2.75, ZD), 0.9, 0, True), ((1.12, -0.95, ZS), 0.9, 0, True), ((0.0, 1.05, ZP), 0.95, 0, False),
           ((1.04, 2.2, ZD), 0.7, 0, True), ((0.0, 4.88, ZC), 0.7, 180, False))
BELT = (-0.07, 0.57, 1.21, 1.85, 3.28, 3.92, 4.56)  # Mitten der Gürtelplatten, zwei Gruppen


def hull_at(y):
    """Rumpf bei y: halbe Breite, Unterkante der Flanke, Boden, halbe Bodenbreite."""
    for a, b in zip(HULL, HULL[1:]):
        if a[0] <= y <= b[0]:
            t = (y - a[0]) / (b[0] - a[0])
            return tuple(p + t * (q - p) for p, q in zip(a[1:], b[1:]))
    raise ValueError(f"y = {y} liegt außerhalb des Rumpfs")


def ring(y, hw, side, low, keel):
    """Querschnitt des Rumpfs bei y: Deck, senkrechte Flanke bis `side`, eingezogener Boden auf `low`."""
    return [Vector(p) for p in ((hw, y, side), (hw, y, ZD), (-hw, y, ZD), (-hw, y, side), (-keel, y, low), (keel, y, low))]


SWELL = ((1, -1), (1, 1), (-1, 1), (-1, -1), (-1, -1), (1, -1))  # Richtung (x, z), in der jeder Ringpunkt wächst


def swell(points, d, dy=0.0):
    """Ring um d nach außen gewachsen (Flanken, Deck, Boden), um dy längs verschoben: Kappen über dem Rumpf."""
    return [p + Vector((sx * d, dy, sz * d)) for p, (sx, sz) in zip(points, SWELL)]


def twin(s, m, at, barrel, yaw=0, mirror=False):
    """Flacher Zwillingsturm wie in der Vorlage: runde Barbette, langes, niedriges Gehäuse mit schräger Front in der
    Akzentfarbe, zwei lange Rohre (Durchmesser 0,18, Luft 0,11) mit dicker Hülse an der Wurzel und Mündungsbremse;
    `barrel` zählt ab der Gehäusefront."""
    x, y, z = at
    s.add(post(20, 0.26, 0.245, 0.0, 0.08), m["dark_round"], at=(x, y, z), mirror=mirror)
    g = Ship("twin")
    g.add(wedge(0.5, 0.9, 0.22, front=0.32, taper=0.78), m["accent"], at=(0, 0, 0.11))
    for dx in (-0.145, 0.145):
        g.add(tube(12, 0.1, 0.1, -0.72, -0.34), m["armor_round"], at=(dx, 0, 0.1))
        g.add(tube(14, 0.09, 0.085, -0.45 - barrel, -0.66), m["steel"], at=(dx, 0, 0.1))
        g.add(tube(12, 0.11, 0.11, -0.49 - barrel, -0.35 - barrel), m["dark_round"], at=(dx, 0, 0.1))
    s.place(g, at=(x, y, z + 0.08), rot=(0, 0, yaw), mirror=mirror)
    g.bm.free()


def twin_details(s, m, at, barrel, yaw=0, mirror=False):
    """Kleinteile zu `twin` mit denselben Argumenten: Barbettenring (`armor_round`) bis knapp unter das Deck, dunkle
    Dachplatte hinten auf dem Gehäuse."""
    x, y, z = at
    s.add(post(20, 0.29, 0.29, -0.06, 0.03), m["armor_round"], at=(x, y, z), mirror=mirror)
    g = Ship("twin_details")
    g.add(box(0.28, 0.38, 0.05), m["dark"], at=(0, 0.16, 0.22))
    s.place(g, at=(x, y, z + 0.08), rot=(0, 0, yaw), mirror=mirror)
    g.bm.free()


def build():
    s = Ship("broadside")
    m = vf.materials(s.name, "battleship")
    m["hull"] = vf.toon(f"{s.name}_hull", OLIVE)
    m["armor"] = vf.toon(f"{s.name}_armor", OLIVE_ARMOR)
    m["hull_round"] = vf.toon(f"{s.name}_hull_round", OLIVE, round=True)
    m["armor_round"] = vf.toon(f"{s.name}_armor_round", OLIVE_ARMOR, round=True)
    m["accent"] = vf.toon(f"{s.name}_accent", GUNMETAL)

    # ---------------------------------------------------------------- Rumpf, Bugkappe, Grat, Plateau, Achterburg
    rings = [ring(*station) for station in HULL]
    s.add(loft(rings), m["hull"])
    s.add(loft([swell(rings[0], 0.05, -0.05), swell(ring(-4.95, *hull_at(-4.95)), 0.05)]), m["armor"])
    s.add(block(SPINE, [(ZD - 0.02, 0.0), (ZD + 0.2, 0.05)]), m["armor"])
    vent(s, m, m["armor"], (0, -4.15, ZD + 0.2), 0.22, 0.6, slats=2)
    s.add(plate([(0.285, -4.1), (0.44, -4.1), (0.73, -3.15), (0.3, -3.15)], 0.05, "xy"), m["armor"], at=(0, 0, ZD + 0.02), mirror=True)
    s.add(block(PLATEAU, [(ZD - 0.02, 0.0), (ZP, 0.1)]), m["armor"])
    s.add(plate([(0.14, -0.62), (0.42, 0.0), (0.42, 0.25), (-0.42, 0.25), (-0.42, 0.0), (-0.14, -0.62)], 0.06, "xy"), m["hull"],
          at=(0, 0, ZP + 0.01))
    s.add(block(CASTLE, [(ZD - 0.02, 0.0), (ZC, 0.12)]), m["hull"])

    # ---------------------------------------------------------------- Obere Stufe mit Brückenfenstern, Strebepfeiler der Burg
    s.add(wedge(1.36, 1.3, 0.36, front=0.35, taper=0.82), m["armor"], at=(0, 3.7, ZC + 0.18))
    s.add(box(0.8, 0.04, 0.08), m["window"], at=(0, 3.045, ZC + 0.11))
    for y in (3.6, 4.1, 4.6):
        s.add(plate([(0.97, ZD - 0.02), (1.25, ZD - 0.02), (1.0, ZC - 0.06), (0.86, ZC - 0.06)], 0.1, "xz"), m["armor"],
              at=(0, y, 0), mirror=True)

    # ---------------------------------------------------------------- Sponsons und Zwillingstürme
    s.add(box(0.72, 0.9, 0.62), m["armor"], at=(1.16, -1.0, ZS - 0.31), mirror=True)
    for at, barrel, yaw, mirror in TURRETS:
        twin(s, m, at, barrel, yaw, mirror)
        twin_details(s, m, at, barrel, yaw, mirror)

    # ---------------------------------------------------------------- Bugflanke: Kasematte mit Deckel und Lichtband, Kabelkanal
    d = Vector((HULL[2][1] - HULL[0][1], HULL[2][0] - HULL[0][0])).normalized()  # Bugflanke steuerbord, nach achtern
    n = Vector((d.y, -d.x))  # nach außen
    yaw = -math.degrees(math.atan2(d.x, d.y))
    c = Vector((hull_at(-3.4)[0], -3.4)) + n * 0.1
    s.add(box(0.22, 0.9, 0.4), m["armor"], at=(c.x, c.y, 0.05), rot=(0, 0, yaw), mirror=True)
    s.add(box(0.28, 0.98, 0.06), m["hull"], at=(*(c + n * 0.02), 0.28), rot=(0, 0, yaw), mirror=True)
    s.add(box(0.04, 0.6, 0.08), m["glow"], at=(*(c + n * 0.115), 0.05), rot=(0, 0, yaw), mirror=True)
    k = Vector((hull_at(-2.2)[0], -2.2)) + n * 0.04
    s.add(box(0.1, 1.5 / d.y, 0.1), m["dark"], at=(k.x, k.y, 0.3), rot=(0, 0, yaw), mirror=True)

    # ---------------------------------------------------------------- Flanke: Panzergürtel in zwei Gruppen, Schleuse
    for y in BELT:
        s.add(box(0.12, 0.56, 0.82), m["armor"], at=(1.44, y, ZD + 0.06 - 0.41), mirror=True)
    vent(s, m, m["hull"], (1.37, 2.57, 0.13), 0.3, 0.36, slats=0, rot=(0, 90, 0), mirror=True)

    # ---------------------------------------------------------------- Seitendecks: Lukenrahmen, Maschinenschacht, RCS
    vent(s, m, m["hull"], (1.03, -0.02, ZD), 0.54, 0.75, slats=0, mirror=True)
    vent(s, m, m["hull"], (1.03, 0.82, ZD), 0.54, 0.55, slats=0, mirror=True)
    for dx in (-0.12, 0.12):
        s.add(tube(12, 0.085, 0.085, -0.2, 0.2), m["armor_round"], at=(1.03 + dx, 0.82, ZD + 0.02), mirror=True)
    vent(s, m, m["hull"], (0.68, 4.88, ZC), 0.3, 0.5, slats=2, mirror=True)
    rcs(s, m, (1.2, 5.2, ZD + 0.03), 1)

    # ---------------------------------------------------------------- Unterseite: Kiel, zweite Lage, Spanten
    s.add(plate([(-3.4, 0.1), (4.7, 0.1), (4.9, -0.12), (0.0, -0.16), (-2.8, -0.06)], 0.16, "yz"), m["dark"], at=(0, 0, -0.85))
    s.add(plate([(0.75, -1.0), (0.75, 4.7), (-0.75, 4.7), (-0.75, -1.0)], 0.07, "xy"), m["armor"], at=(0, 0, -0.88))
    hw, side, low, keel = hull_at(1.0)
    x0, x1 = keel + 0.04, hw - 0.1
    z0, z1 = (low + (side - low) * (x - keel) / (hw - keel) for x in (x0, x1))
    for y in (0.3, 0.9, 1.5, 2.1):
        s.add(plate([(x0, z0 + 0.04), (x1, z1 + 0.04), (x1, z1 - 0.08), (x0, z0 - 0.08)], 0.12, "xz"), m["hull"], at=(0, y, 0), mirror=True)

    # ---------------------------------------------------------------- Drei Heckdüsen mit Gehäusesegment und Düsenkranz
    for r, ya, yb, dx, dz, mirror in ((0.4, 5.0, 5.95, 0.0, -0.1, False), (0.3, 5.1, 5.85, 0.9, -0.05, True)):
        engine(s, m, r, ya, yb, dx=dx, dz=dz, mirror=mirror)
        engine_details(s, m, r, ya, yb, dx=dx, dz=dz, mirror=mirror)
    return s.finish()
