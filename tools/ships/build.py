"""Baut die Schiffe und exportiert sie nach assets/models/ships/<id>.glb, dazu ihre Wrackteile (wreck.py) nach
<id>_wreck.glb (nicht für die Geschosse, fleet.PROJECTILES). Vor dem Export bekommt jedes Schiff seine UVs und die Maske
der Verwitterung (weather.py, gebacken oder aus dem Cache), die als <id>_weather.ktx2 daneben liegt, die Wesen dazu ihre
Haut (skin.py) als <id>_skin.ktx2; die Stücke des Wracks erben UVs, Maske und Haut. Die drehbaren Türme
(`vf.Ship.turret`) hängen im GLB als eigene Knoten `<id>_turret_<k>` unter dem Schiff (`vf.split_turrets`), das Wrack
zerlegt sie mit dem Rumpf. Dazu entsteht die Feinstruktur aller Rümpfe, detail.ktx2 (detail.py).

Headless:   <blender> -b --factory-startup -P tools/ships/build.py -- [ids...]
            <blender> ist Blender 5.2 LTS, auf dem Mac /Applications/Blender.app/Contents/MacOS/Blender, auf dem
            Linux-Server ohne GPU das portable ~/.local/opt/blender-5.2.4-linux-x64/blender (die Vorschau braucht dort
            libEGL, Paket libegl1). 5.2.3 und 5.2.4 liefern dieselben Bytes; 5.3 packt die UV-Inseln anders
            (weather.unwrap), ein Neubau damit änderte Masken und Haut.
Interaktiv: in Blender `import build; build.build_all()` (lädt die Module vorher neu).
Ohne Ids werden alle Schiffe aus fleet.SHIPS gebaut. Die Texturen schreibt der Encoder `ctt` (im Pfad oder als CTT,
siehe weather.py). Meldet je Schiff Dreiecke, Maße, Maske und GLB-Größe samt Wrack und warnt
bei Teilen unter dem Slot (vf.slot), bei schwebenden Teilen (vf.loose_parts) und bei Hüllen, die offen oder falsch herum
orientiert sind (vf.bad_shells, an Schiff und Wrackstücken; die Materialien sind einseitig).
"""

import importlib
import os
import sys
import time

import bpy

ROOT = os.path.dirname(os.path.abspath(__file__))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

import detail  # noqa: E402
import fleet  # noqa: E402
import skin  # noqa: E402
import vf  # noqa: E402
import weather  # noqa: E402
import wreck  # noqa: E402

OUT_DIR = os.path.normpath(os.path.join(ROOT, "..", "..", "assets", "models", "ships"))


def reload_modules():
    """Lädt die Schiffsmodule in Abhängigkeitsreihenfolge neu (für eine laufende Blender-Instanz)."""
    names = ["palette", "vf", "parts", "look", "weather", "skin", "wreck", "detail"] + sorted(n for n in sys.modules if n.startswith("fleet.")) + ["fleet"]
    for name in names:
        if name in sys.modules:
            importlib.reload(sys.modules[name])


def build_all(ids=None):
    reload_modules()
    os.makedirs(OUT_DIR, exist_ok=True)
    detail.write(os.path.join(OUT_DIR, "detail.ktx2"))
    report = {}
    for ship_id in ids or fleet.SHIPS:
        obj = fleet.build(ship_id)
        started = time.time()
        mask = weather.apply(obj)
        weathered = time.time() - started
        path = os.path.join(OUT_DIR, f"{ship_id}.glb")
        if mask:
            weather.compress(bpy.path.abspath(mask.filepath), os.path.join(OUT_DIR, f"{ship_id}_weather.ktx2"))
            grown = skin.apply(obj, mask.size[0])
            if grown:
                weather.compress(grown, os.path.join(OUT_DIR, f"{ship_id}_skin.ktx2"))
        pieces = [] if ship_id in fleet.PROJECTILES else wreck.build(obj)
        wreck_path = os.path.join(OUT_DIR, f"{ship_id}_wreck.glb")
        if pieces:
            vf.export_glb(pieces, wreck_path)
        shells = vf.bad_shells(obj) + [shell for piece in pieces for shell in vf.bad_shells(piece)]
        report[ship_id] = {**vf.stats(obj), "loose": vf.loose_parts(obj), "shells": shells,
                           "mask": f"Maske {mask.size[0]} px ({weathered:.1f} s)" if mask else "ohne Maske",
                           "wreck": {**wreck.stats(pieces), "kb": os.path.getsize(wreck_path) / 1024} if pieces else None}
        vf.export_glb([obj, *vf.split_turrets(obj)], path)
        report[ship_id]["kb"] = os.path.getsize(path) / 1024
    return report


if __name__ == "__main__":
    requested = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    for ship_id, info in build_all(requested or None).items():
        w = info["wreck"]
        print(f"{ship_id}: {info['tris']} Dreiecke, {info['kb']:.0f} KB, {info['mask']}, Maße {info['size']}, z {info['z']}"
              + (f"; Wrack {w['pieces']} Stücke, {w['tris']} Dreiecke, {w['kb']:.0f} KB" if w else ""))
        for center, tris in info["loose"]:
            print(f"  WARNUNG {ship_id}: schwebendes Teil bei ({center.x:.2f}, {center.y:.2f}, {center.z:.2f}), {tris} Dreiecke")
        for center, tris, problem in info["shells"]:
            print(f"  WARNUNG {ship_id}: Hülle {problem} bei ({center.x:.2f}, {center.y:.2f}, {center.z:.2f}), {tris} Dreiecke")
