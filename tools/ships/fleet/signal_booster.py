"""Signal Booster (im Spiel der Spotter, ein Kreuzer in den Farben der Unterstützer) im Clean-Toon-Stil, detailreiche
Fassung nach der Iron Dreadnought.

Primärformen: kantiger, heller Rumpf mit dunklem Gerätedeck (mintgrüne Deckplatte), mintgrüner Bauchplatte und einem
Lichtband an jeder Flanke. Darauf eine große Schüssel mit leuchtendem Cyan-Rand, mintgrüner Mitte und Sender, auf einem
Gabelmast mit mintgrünem Joch nach vorn oben gekippt. Um den Rumpf ein Signalring (Stahlband mit Cyan-Streifen ohne
Glühen, damit der Rand der Schüssel der Blickfang bleibt), den Bug und Heck über den Gondeln tragen; zwei Relaismasten
(achtern Backbord hoch, vorn Steuerbord niedrig) mit Relaiskasten und Licht; zwei Solarpaneele als ruhige Flächen an
Auslegern, nach außen geneigt; Kanzel vorn Backbord, Bugemitter, Seitengondeln an Streben mit mintgrünem Band und je
einer Düse.
Detailzonen mit Zweck, grob, weil das Schiff im Spiel als Paar zu 0,72 steht und die Konturen dabei gleich breit
bleiben; dazwischen ruhige Flächen (Rumpfflanken, Bug, Paneele, Gondeln). Breiter wird es nicht: Das Paar ragt bei 0,72
über das Pad (dafür dürfte es höchstens etwa 3,5 breit sein), deshalb sitzen die Paneele näher am Rumpf als in der
groben Fassung.
- Schüssel: größer als in der groben Fassung, Drehkranz zwischen Mast und Joch
- Signalring: Ringlager dort, wo der Ring in Bug und Heck läuft
- Paneele: Holm über jedem Paneel als Fortsetzung des Auslegers
- Heck: Gehäusesegment und Düsenkranz an beiden Düsen (`parts.engine_details`)
Fugen, Nieten und Schmutz kommen aus der Verwitterungs-Textur, nicht aus der Geometrie.
"""

import math

import palette
import vf
from parts import axle, beam, engine, engine_details, ring_at
from vf import Ship, box, dish, hoop, loft, octagon, plate, post, rounded, sec, smooth, sphere, superellipse, tip, tube, wedge

RING = (0.15, 0.26, 1.79)  # Signalring: Mitte (y, z) und mittlerer Radius


def relay_mast(s, m, x, y, z0, z1):
    """Relaismast: Stab mit Relaiskasten auf halber Höhe und einem Licht an der Spitze."""
    s.add(post(10, 0.11, 0.075, z0, z1), m["hull_round"], at=(x, y, 0))
    s.add(box(0.24, 0.24, 0.34), m["dark"], at=(x, y, z0 + (z1 - z0) * 0.5))
    s.add(sphere(0.09, 1), m["glow"], at=(x, y, z1 + 0.05))


def build():
    s = Ship("signal_booster")
    m = vf.materials(s.name, "support")
    m["cyan"] = vf.light(f"{s.name}_cyan", palette.CYAN)
    m["ring"] = vf.light(f"{s.name}_ring", palette.CYAN, glow=False)
    m["solar"] = vf.toon(f"{s.name}_solar", palette.GLASS)
    # Kantiger Rumpf mit flachem Deck; Oberkante bei y nach den Profilen
    sections = [(-2.5, 0.7, 0.42), (-1.75, 1.4, 0.8), (0.6, 1.72, 0.96), (1.8, 1.36, 0.82), (2.35, 0.96, 0.6)]
    s.add(loft([sec(y, octagon(w, h)) for y, w, h in sections]), m["hull"])

    def profile(y):
        """Halbe Breite und halbe Höhe des Rumpfs bei y."""
        for (y0, w0, h0), (y1, w1, h1) in zip(sections, sections[1:]):
            if y0 <= y <= y1:
                t = (y - y0) / (y1 - y0)
                return (w0 + (w1 - w0) * t) / 2, (h0 + (h1 - h0) * t) / 2

    def deck(y):
        return profile(y)[1]

    # Dunkles Gerätedeck mit schräger Front und mintgrüner Deckplatte, mintgrüne Bauchplatte
    s.add(wedge(1.1, 1.9, 0.34), m["dark"], at=(0, 0.3, deck(0.3) + 0.12))
    s.add(plate([(-0.46, 0.1), (0.46, 0.1), (0.46, 1.16), (-0.46, 1.16)], 0.06, "xy"), m["accent"], at=(0, 0, deck(0.3) + 0.3))
    s.add(plate([(-0.42, -1.4), (0.42, -1.4), (0.5, 1.3), (-0.5, 1.3)], 0.12, "xy"), m["accent"], at=(0, 0, -0.5))
    # Gabelmast: Stab, Drehkranz, Joch und zwei Zinken in Mintgrün, Achse
    my, yoke, hub = -0.25, deck(0.3) + 0.44, deck(0.3) + 0.69
    s.add(post(16, 0.17, 0.14, deck(my), yoke), m["dark_round"], at=(0, my, 0))
    s.add(post(24, 0.25, 0.25, yoke - 0.21, yoke - 0.08), m["armor_round"], at=(0, my, 0))
    s.add(box(1.0, 0.26, 0.18), m["accent"], at=(0, my, yoke))
    s.add(box(0.15, 0.22, hub - yoke + 0.14), m["accent"], at=(0.44, my, (yoke + hub) / 2 + 0.02), mirror=True)
    s.add(axle(0.13, 0.82), m["dark_round"], at=(0, my, hub))
    # Große Schüssel mit Cyan-Rand, mintgrüner Mitte und Sender, um die Achse nach vorn oben gekippt
    g = Ship("dish")
    r, depth, off = 1.22, 0.4, 0.28
    g.add(dish(r, depth, 24, 4, 0.06), m["hull_round"], at=(0, -off, 0))
    g.add(dish(0.48, depth * (0.48 / r) ** 2, 24, 2, 0.04), m["accent_round"], at=(0, -off - 0.03, 0))
    g.add(hoop(24, r - 0.04, r + 0.08, 0.1), m["cyan"], at=(0, -off - depth, 0), rot=(90, 0, 0))
    g.add(tube(16, 0.24, 0.2, -off + 0.02, 0.12), m["dark_round"])
    g.add(tube(12, 0.1, 0.09, -off - 0.84, -off), m["dark_round"])
    g.add(tube(12, 0.12, 0.12, -off - 1.04, -off - 0.8), m["cyan"])
    s.place(g, at=(0, my, hub), rot=(-40, 0, 0))
    g.bm.free()
    # Signalring um den Rumpf über den Gondeln: Stahlband mit Cyan-Streifen oben, Bug und Heck tragen ihn in Ringlagern
    ry, rz, rr = RING
    s.add(hoop(36, rr - 0.07, rr + 0.07, 0.16), m["steel"], at=(0, ry, rz))
    s.add(hoop(36, rr - 0.05, rr + 0.05, 0.04), m["ring"], at=(0, ry, rz + 0.08))
    for x, side in ((0.75, -1), (0.72, 1)):
        angle = math.degrees(math.asin(x / rr))
        s.add(box(0.3, 0.24, 0.26), m["armor"], at=(x, ry + side * math.sqrt(rr * rr - x * x), rz), rot=(0, 0, -side * angle), mirror=True)
    # Relaismasten: achtern Backbord hoch, vorn Steuerbord niedrig
    relay_mast(s, m, -0.5, 1.35, deck(1.35) - 0.05, 2.3)
    relay_mast(s, m, 0.5, -1.95, deck(-1.95) - 0.05, 1.55)
    # Solarpaneele an Auslegern über dem Ring: ruhige Fläche im Rahmen, der Ausleger läuft als Holm darüber weiter
    s.add(beam((0.7, 1.15, deck(1.15) + 0.02), (1.81, 1.15, 0.6), 0.16, 0.16), m["dark"], mirror=True)
    for size, mat in (((0.92, 0.8, 0.08), m["dark"]), ((0.82, 0.7, 0.12), m["solar"]), ((0.92, 0.08, 0.14), m["dark"])):
        s.add(box(*size), mat, at=(1.85, 1.15, 0.64), rot=(0, 25, 0), mirror=True)
    # Lichtband an jeder Flanke
    flank = math.degrees(math.atan2(profile(-0.1)[0] - profile(-1.4)[0], 1.3))
    s.add(box(0.05, 1.3, 0.07), m["glow"], at=(profile(-0.75)[0], -0.75, 0.02), rot=(0, 0, -flank), mirror=True)
    # Kanzel vorn Backbord, Bugemitter
    canopy = [(-2.36, 0.24, 0.14), (-2.18, 0.5, 0.26), (-1.98, 0.52, 0.27), (-1.84, 0.36, 0.16)]
    s.add(loft(smooth([sec(y, rounded(w, h, 0.08, 1), dx=-0.12, dz=deck(y) + h * 0.3) for y, w, h in canopy], 1)), m["glass"])
    s.add(tube(16, 0.22, 0.2, -2.64, -2.42), m["dark_round"])
    s.add(tube(16, 0.15, 0.11, -2.72, -2.6), m["cyan"])
    # Seitengondeln an Streben mit mintgrünem Band und Düse
    gx, gz = 1.3, -0.25
    nacelle = smooth([tip(-1.0), sec(-0.85, superellipse(0.36, 0.36, 16, 2.4)), sec(-0.6, superellipse(0.54, 0.54, 16, 2.4)),
                      sec(-0.3, superellipse(0.6, 0.6, 16, 2.4)), sec(1.0, superellipse(0.62, 0.62, 16, 2.4)),
                      sec(1.6, superellipse(0.56, 0.56, 16, 2.4))], 1)
    s.add(loft(nacelle), m["hull_round"], at=(gx, 0, gz), mirror=True)
    s.add(loft([ring_at(nacelle, -0.55, 0.035), ring_at(nacelle, -0.32, 0.035)]), m["accent_round"], at=(gx, 0, gz), mirror=True)
    s.add(box(0.46, 0.8, 0.22), m["dark"], at=(0.9, 0.55, gz - 0.02), mirror=True)
    engine(s, m, 0.24, 1.5, 2.1, dx=gx, dz=gz, n=14, mirror=True)
    engine_details(s, m, 0.24, 1.5, 2.1, dx=gx, dz=gz, n=14, mirror=True)
    return s.finish()
