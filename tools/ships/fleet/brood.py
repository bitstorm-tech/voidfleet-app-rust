"""Brut des Unbekannten Kontakts: was aus den Eiern an seinen Flanken schlüpft. Wie er kein Schiff, sondern ein Wesen;
die Stilregeln der Flotte gelten nicht, die Technik schon (fleet/unknown_contact.py, von dort kommen Farben, Kurven,
Schläuche und Tentakel). Im Spiel ist die Brut eine Jäger-Staffel: fünf Modelle im Maßstab 0,42, die eine Runde
ausweichen und dann rammen.

Aufbau (Nase −Y, Länge gut 4,5, Breite 1,4): ein eiförmiger Leib in Schuppengrau, die Schale des Eis, aus dem sie kommt;
an beiden Flanken leuchtet durch einen Riss das Innere. Vorn zwei Paar Knochenzangen wie die des Muttertiers (sie sind,
was rammt), auf dem Rücken ein Knochenschild mit Dorn, hinten drei kurze Tentakel mit Schuppenringen. Keine Düsen.

Materialien wie beim Unbekannten Kontakt (Schlüssel für look.WEATHER_MATERIALS): `bone`, `flesh`, `scale`, Licht
`organ`.
"""

import vf
from fleet.unknown_contact import BONE, FLESH, ORGAN, SCALE, limb, spline, tentacle
from vf import Ship, revolve, sphere

# Leib: Rotationskörper um die Längsachse (Radius, y), vorn stumpf, hinten spitzer; in der Höhe um FLAT gestaucht
EGG = [(0.0, -1.15), (0.28, -1.05), (0.5, -0.8), (0.64, -0.4), (0.68, 0.0), (0.62, 0.45), (0.46, 0.9), (0.25, 1.2), (0.0, 1.35)]
FLAT = 0.82


def build():
    s = Ship("brood")
    m = {
        "bone": vf.toon(f"{s.name}_bone", BONE, round=True),
        "flesh": vf.toon(f"{s.name}_flesh", FLESH, round=True),
        "scale": vf.toon(f"{s.name}_scale", SCALE, round=True, glint=vf.GLINT),
        "organ": vf.light(f"{s.name}_organ", ORGAN),
    }
    s.add(revolve(EGG, 20), m["scale"], scale=(1.0, 1.0, FLAT))
    # Risse in der Schale: das leuchtende Innere, je Flanke ein flacher Wulst knapp über der Haut
    s.add(sphere(1.0, 2), m["organ"], at=(0.56, -0.05, 0.04), scale=(0.16, 0.52, 0.2), rot=(0, 0, -6), mirror=True)

    # Rückenschild: liegt auf dem Scheitel, vorn und hinten spitz, dahinter ein Dorn nach hinten oben
    shield = [(0, -0.98, 0.3), (0, -0.3, 0.54), (0, 0.5, 0.51), (0, 1.15, 0.25)]
    s.add(limb(spline(shield, 12), [0.0, 0.3, 0.24, 0.0], [0.0, 0.08, 0.07, 0.0], 10), m["bone"])
    thorn = [(0, 0.2, 0.5), (0, 0.85, 0.72), (0, 1.5, 0.92)]
    s.add(limb(spline(thorn, 8), [0.09, 0.06, 0.0], n=8), m["bone"])

    # Zangen: das obere Paar greift nach vorn und innen, das untere hängt tiefer und ist kürzer
    upper = [(0.3, -0.75, 0.1), (0.52, -1.45, 0.14), (0.44, -2.1, 0.08), (0.1, -2.6, 0.0)]
    s.add(limb(spline(upper, 14), [0.16, 0.14, 0.1, 0.0], [0.12, 0.1, 0.07, 0.0], 10), m["bone"], mirror=True)
    lower = [(0.26, -0.7, -0.2), (0.38, -1.35, -0.36), (0.22, -1.9, -0.4)]
    s.add(limb(spline(lower, 10), [0.12, 0.1, 0.0], [0.09, 0.07, 0.0], 8), m["bone"], mirror=True)

    # Tentakel: einer in der Mitte, ein Paar, das sich spreizt und an den Spitzen hebt
    tentacle(s, m, [(0, 1.1, 0.0), (0, 1.75, -0.03), (0, 2.45, 0.08)], 0.16, False, steps=12, collars=3)
    tentacle(s, m, [(0.2, 1.0, 0.05), (0.5, 1.6, 0.12), (0.74, 2.15, 0.32)], 0.13, True, steps=12, collars=3)
    return s.finish()
