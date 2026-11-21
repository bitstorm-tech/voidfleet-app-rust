"""Brut v2 (Vergleichsmodell für die Modellschau): verfeinerte Fassung der Brut aus fleet/brood.py für die Nahaufnahme
und Brettgröße gleichermaßen (~3k Dreiecke, Staffel zu 5 Modellen im Maßstab 0,42).

Verbesserungen gegenüber brood.py:
- Echter aufgebrochener Riss in der Eischale statt platter ovaler Leuchtpillen: schlanke, geschwungene Leuchtspalte
  (`organ` mit `soft_glow` und schnellerem Biopuls `pulse=1.35`), oben und unten gerahmt von dunklen, fleischigen
  Bruchkanten (`sinew`), dazu glimmen in `skin.py` die Schuppenfugen um den Riss mit.
- Fleischiger Maulansatz vorn an der Schale mit schwachem Rachen-Glimmen und inneren Widerhaken an den oberen
  Rammzangen.
- Profilierter Rückenkamm aus sauberem Knochen (`dirt=0.0`) mit zweitem kleineren Heckdorn und ventraler Schuppenleiste
  für die Unterseite im Zoom.
"""

import math

from mathutils import Vector

import vf
from fleet.unknown_contact import BONE, FLESH, SCALE, SINEW, WET, limb, spline, tentacle
from fleet.unknown_contact_v2 import ORGAN_V2
from vf import Ship, revolve, sphere

# Leib: eiförmige Schale, vorn etwas straffer in die Zangen laufend, hinten organisch verjüngt
EGG = [
    (0.0, -1.16),
    (0.28, -1.04),
    (0.51, -0.78),
    (0.65, -0.38),
    (0.69, 0.02),
    (0.62, 0.48),
    (0.45, 0.92),
    (0.24, 1.22),
    (0.0, 1.36),
]
FLAT = 0.82
PULSE = 1.35  # flinker Biopuls, passend zum hastigen `tempo` (1,3) der Brut


def _egg_radius(y):
    """Radius der Eischale bei Längskoordinate `y` (linear aus EGG)."""
    for (r0, y0), (r1, y1) in zip(EGG, EGG[1:]):
        if y0 <= y <= y1:
            t = (y - y0) / max(y1 - y0, 1e-6)
            return r0 + (r1 - r0) * t
    return 0.0


def _shell_point(y, angle_deg, off=0.0):
    """Punkt auf der gestauchten Eischale (Steuerbord, angle_deg: 0 = oben, 90 = Flanke, 180 = unten)."""
    r = _egg_radius(y)
    a = math.radians(angle_deg)
    d = Vector((math.sin(a), 0.0, math.cos(a) * FLAT))
    n = Vector((math.sin(a), 0.0, math.cos(a) / FLAT)).normalized()
    return Vector((0.0, y, 0.0)) + d * r + n * off, n


def build():
    s = Ship("brood_v2")
    m = {
        "bone": vf.toon(f"{s.name}_bone", BONE, round=True, glint=WET["bone"], dirt=0.0),
        "flesh": vf.toon(f"{s.name}_flesh", FLESH, round=True, glint=WET["flesh"], dirt=0.08, pulse=PULSE),
        "sinew": vf.toon(f"{s.name}_sinew", SINEW, round=True, glint=WET["sinew"], dirt=0.08, pulse=PULSE),
        "scale": vf.toon(f"{s.name}_scale", SCALE, round=True, glint=WET["scale"], dirt=0.08, pulse=PULSE),
        "organ": vf.light(f"{s.name}_organ", ORGAN_V2, glow=True, soft_glow=True, pulse=PULSE),
    }

    # Eischale
    s.add(revolve(EGG, 18), m["scale"], scale=(1.0, 1.0, FLAT))

    # Geschwungener Riss an jeder Flanke: spitz auslaufender Leuchtspalt statt ovaler Kugel
    fissure_specs = [(-0.72, 96.0), (-0.42, 88.0), (-0.06, 84.0), (0.30, 88.0), (0.64, 95.0)]
    fissure_pts, fissure_ups = [], []
    for y, ang in fissure_specs:
        p, n = _shell_point(y, ang, -0.02)
        fissure_pts.append(p)
        fissure_ups.append(n)
    fissure_path = spline(fissure_pts, 7)
    fissure_n = spline(fissure_ups, 7)
    # Schlank und spitz auslaufend; ragt knapp aus der Schale
    s.add(
        limb(fissure_path, [0.0, 0.078, 0.118, 0.098, 0.050, 0.0], [0.0, 0.056, 0.080, 0.070, 0.040, 0.0], 6, ups=fissure_n),
        m["organ"],
        mirror=True,
    )
    # Aufgeworfene dunkle Bruchkanten (ober- und unterhalb des Risses), fassen den Leuchtspalt organisch ein
    for sign, d_ang in ((1, -13.0), (-1, 13.0)):
        lip_pts, lip_ups = [], []
        for k, (y, ang) in enumerate(fissure_specs):
            spread = 0.25 + 0.75 * math.sin(math.pi * k / (len(fissure_specs) - 1))
            p, n = _shell_point(y, ang + d_ang * spread, 0.006)
            lip_pts.append(p)
            lip_ups.append(n)
        s.add(
            limb(spline(lip_pts, 6), [0.0, 0.048, 0.056, 0.045, 0.0], [0.0, 0.036, 0.044, 0.035, 0.0], 5, ups=spline(lip_ups, 6)),
            m["sinew"],
            mirror=True,
        )

    # Fleischiger Maulansatz vorn an der Schale + tiefes Rachen-Glimmen zwischen den Zangen
    s.add(sphere(1.0, 1), m["flesh"], at=(0.0, -0.99, -0.05), scale=(0.38, 0.28, 0.29))
    s.add(sphere(1.0, 1), m["organ"], at=(0.0, -1.11, -0.07), scale=(0.13, 0.10, 0.10))

    # Profilierter Rückenschild mit Mittelkamm, Hauptdorn und kleinem hinteren Sekundärdorn
    shield = [(0, -1.04, 0.24), (0, -0.42, 0.52), (0, 0.38, 0.51), (0, 1.14, 0.24)]
    s.add(limb(spline(shield, 10), [0.0, 0.30, 0.25, 0.0], [0.0, 0.088, 0.072, 0.0], 8), m["bone"])
    crest = [(0, -0.82, 0.38), (0, -0.15, 0.58), (0, 0.45, 0.55), (0, 0.98, 0.35)]
    s.add(limb(spline(crest, 6), [0.0, 0.07, 0.06, 0.0], [0.0, 0.065, 0.06, 0.0], 5), m["bone"])
    thorn = [(0, 0.12, 0.52), (0, 0.82, 0.75), (0, 1.52, 0.98)]
    s.add(limb(spline(thorn, 7), [0.092, 0.058, 0.0], [0.095, 0.062, 0.0], 7), m["bone"])
    rear_thorn = [(0, 0.65, 0.42), (0, 1.05, 0.54), (0, 1.42, 0.64)]
    s.add(limb(spline(rear_thorn, 4), [0.052, 0.034, 0.0], n=5), m["bone"])

    # Ventrale Kiel-Leiste unter der Schale (gibt der Unterseite im Zoom Profil)
    keel = [(0, -0.82, -0.35), (0, -0.10, -0.56), (0, 0.62, -0.45), (0, 1.06, -0.22)]
    s.add(limb(spline(keel, 6), [0.0, 0.14, 0.11, 0.0], [0.0, 0.05, 0.04, 0.0], 5), m["sinew"])

    # Zangen: obere kräftiger geschwungen mit kleinem inneren Widerhaken, untere kürzer darunter
    upper = [(0.30, -0.78, 0.10), (0.54, -1.48, 0.16), (0.44, -2.14, 0.09), (0.10, -2.62, 0.0)]
    s.add(limb(spline(upper, 12), [0.16, 0.14, 0.095, 0.0], [0.12, 0.10, 0.068, 0.0], 9), m["bone"], mirror=True)
    barb = [(0.46, -1.78, 0.12), (0.28, -1.98, 0.06), (0.18, -2.12, 0.02)]
    s.add(limb(spline(barb, 4), [0.055, 0.036, 0.0], n=5), m["bone"], mirror=True)
    lower = [(0.26, -0.72, -0.20), (0.39, -1.36, -0.37), (0.20, -1.92, -0.41)]
    s.add(limb(spline(lower, 8), [0.12, 0.095, 0.0], [0.09, 0.07, 0.0], 7), m["bone"], mirror=True)

    # Drei kurze Tentakel mit Schuppenringen, eleganter geschwungen
    tentacle(s, m, [(0, 1.08, 0.0), (0, 1.78, -0.04), (0, 2.48, 0.10)], 0.155, False, steps=9, collars=3)
    tentacle(s, m, [(0.20, 0.98, 0.05), (0.52, 1.62, 0.14), (0.78, 2.20, 0.36)], 0.125, True, steps=9, collars=3)
    return s.finish()
