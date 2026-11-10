"""Torpedo, das Geschoss der Torpedo-Karten (Effekt `Missile` in crates/client/src/scene/effects.rs), kein Schiff und
ohne Wrack. Die Lanze des Lanzen-Bombers allein: runder Körper mit Stahlspitze, leuchtendem Suchkopf und Zünderring,
zwei Lichtbänder an den Flanken, vier dunkle Leitflossen und die Brennkammer im Düsenring am Heck. Farben neutral
(Klasse `battleship`), denn ihn feuern Spieler und Gegner; die Leuchtspur im Client trägt die Farbe der Seite.
Die Mitte der Länge liegt im Ursprung, die Spitze zeigt nach -y wie bei den Schiffen.
"""

import vf
from vf import Ship, box, loft, plate, revolve, sec, smooth, superellipse, tube

Y = 1.5  # schiebt die Lanze (y -4,4 … 1,35 im Lanzen-Bomber) auf die Mitte


def build():
    s = Ship("torpedo")
    m = vf.materials(s.name, "battleship")
    N = 20
    s.add(loft(smooth([
        sec(-3.3 + Y, superellipse(0.5, 0.5, N)),
        sec(-2.7 + Y, superellipse(0.62, 0.62, N)),
        sec(0.5 + Y, superellipse(0.62, 0.62, N)),
        sec(1.0 + Y, superellipse(0.44, 0.44, N)),
    ], 1)), m["accent_round"])
    s.add(revolve([(0.26, -3.2 + Y), (0.26, -3.4 + Y), (0.12, -4.15 + Y), (0.0, -4.3 + Y)], N), m["steel"])
    s.add(tube(12, 0.09, 0.0, -4.26 + Y, -4.4 + Y), m["glow"])
    s.add(tube(N, 0.29, 0.305, -3.33 + Y, -3.13 + Y), m["dark_round"])
    s.add(box(0.05, 1.4, 0.09), m["glow"], at=(0.315, -1.6 + Y, 0))
    s.add(box(0.05, 1.4, 0.09), m["glow"], at=(-0.315, -1.6 + Y, 0))
    for rot in (45, -45, 135, -135):
        s.add(plate([(0.4 + Y, 0.0), (1.15 + Y, 0.0), (1.35 + Y, 0.55), (0.95 + Y, 0.55)], 0.1, "yz"), m["dark"], rot=(0, rot, 0))
    s.add(tube(16, 0.24, 0.1, 0.95 + Y, 1.25 + Y), m["core"])
    s.add(tube(N, 0.25, 0.27, 0.9 + Y, 1.1 + Y), m["dark_round"])
    return s.finish()
