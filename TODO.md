# Offene Punkte (Stand 05.11.2026, nach dem PvE-Umbau)

## Nachziehen

- [ ] **Sichtprüfung des Runs**: Sternenkarte (Kanten, Pfad, gesperrte Knoten), Beute, Ereignis, Werft, Schwarzmarkt und Run-Ende sind nur per Rauchtest-Log geprüft, nicht im Bild (die Screenshots entstanden bei gesperrtem Bildschirm).
- [ ] **Icons auf der Sternenkarte**: Ereignis, Werft und Schwarzmarkt tragen nur Zeichen (`?`, `+`, `$`).
- [ ] **Funksprüche**: Nur Gefechtsbeginn, Abschuss, Hülle kritisch, Sieg und Niederlage haben Aufnahmen; Brücke, Geschützleitung und Gegnerfunk nach Abschnitt 9 des Entwurfs fehlen (Aufnahmen über fal.ai erzeugen, Katalog in `crates/client/src/data/radio.rs`).
- [ ] **Farben der neuen Flaggschiffe** in `tools/ships/palette.py`: Reyes (`#ff7a2a`) und Vey (`#ff4f6e`) stehen als Konstanten in `fleet/pyre.py` und `fleet/seal.py`; in `ADMIRALS`/`FLAGSHIPS` eintragen, dann bekommen Vorschau und `build.py` die Flaggschiff-Behandlung (Faktor 1,1, Slot-Prüfung) und die Konstanten können raus. `crates/client/src/palette.rs` hat Reyes schon.
- [ ] **Flackern-Formel** `(max_hull + 3) / 4` ist im Core nicht öffentlich; der Client rechnet sie für den Ereignistext nach (`crates/core/src/run.rs`, `EventOption::Embrace`).
- [ ] **`tools/rasterize`** rastert noch die Regel-Figuren und das Aufgeben-Icon, die der Client nicht mehr lädt.
- [ ] **Toon-Material** kann weiterhin Geister zeichnen, obwohl die Planung keine Geister mehr kennt (`crates/client/src/scene/toon/`).
- [ ] **`docs/architektur.html`** beschreibt die PvP-Architektur mit Server und ist veraltet.

## Aus dem Audit vom 02.11.2026, noch gültig

- [ ] **Mausrad-Zoom schreibt in jedem Bild `settings.json`** — `crates/client/src/scene/camera.rs`: auch früh abbrechen, solange `zoom_offset` noch nicht bei `zoom_target` ist.
- [ ] **Ein kaputtes Feld löscht alle Einstellungen** — `crates/client/src/settings.rs`: Parse-Fehler → Standardwerte → nächste Änderung überschreibt die Datei; `fs::write` ist nicht atomar. Über `settings.json.tmp` + `fs::rename` schreiben; unlesbare Datei vorher nach `.bad` umbenennen. Dasselbe gilt für `run.json` (`crates/client/src/run/save.rs`).
- [ ] **Tooltips bleiben nach dem Sprachwechsel alt** — die Tooltips der Icon-Knöpfe (`crates/client/src/ui/hud.rs`, `view_toggle.rs`) in `follow_locale` mit neu beschriften.
