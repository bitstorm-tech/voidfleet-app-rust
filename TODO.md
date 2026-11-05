# Offene Punkte (Stand 05.11.2026, nach dem PvE-Umbau)

## Spielgefühl und Balance (erst spielen, dann stellen)

- [ ] **Alle Zahlen** des Entwurfs sind Startwerte (Hülle, ATK, Karten, Gegner, Schrott, Preise); ohne Bot-Simulation entscheidet jetzt das Spielen. Der Rauchtest gewinnt Gefechte in Sektor 1 oft in Runde 1.
- [ ] **Länge des Gegnerzugs**: Der erfundene Gegnerzug der Szene `report` dauert rund 21 s; mit Leertaste überspringbar, vielleicht zu lang.
- [ ] **Funk-Dichte**: Die neuen Sprüche fallen vielleicht zu dicht; `direct_hit` ab 12 Hüllenschaden ist geschätzt, `incoming_fire` hängt am ersten Feuer eines Gegners (es gibt kein Ereignis für den Beginn des Gegnerzugs). Die Aufnahmen hat niemand angehört.

## Nachziehen

- [ ] **Icons auf der Sternenkarte**: Ereignis, Werft und Schwarzmarkt tragen nur Zeichen (`?`, `+`, `$`).
- [ ] **Modellgruppen**: In der Ansicht von hinten oben verdeckt der Balken das vordere Jägerpaar etwas; das Spotter-Paar (breiter Signal Booster) ragt bei Maßstab 0,72 vermutlich über das Pad.
- [ ] **Wiedergabe**: Ausweichen zeigt nur Triebwerksblitz und Marke, keinen Schlenker als Flugbahn; die Kamera neigt sich nur zum Handelnden, nicht zum Ziel; die Zeile „bricht auseinander, bevor er feuern kann“ für im Spielerzug gefallene Gegner fehlt; der Schild aus Alarmbereitschaft zu Beginn des Gegnerzugs hängt optisch am vorigen Schritt.
- [ ] **Flackern-Formel** `(max_hull + 3) / 4` ist im Core nicht öffentlich; der Client rechnet sie für den Ereignistext nach (`crates/core/src/run.rs`, `EventOption::Embrace`).
- [ ] **Toon-Material** kann weiterhin Geister zeichnen, obwohl die Planung keine Geister mehr kennt; alte PvP-Markenarten (Taunt, HeadStart …) in `scene/markers.rs` sind unbenutzt.
- [ ] **`docs/architektur.html`** beschreibt die PvP-Architektur mit Server und ist veraltet.

## Aus dem Audit vom 02.11.2026, noch gültig

- [ ] **Mausrad-Zoom schreibt in jedem Bild `settings.json`** — `crates/client/src/scene/camera.rs`: auch früh abbrechen, solange `zoom_offset` noch nicht bei `zoom_target` ist.
- [ ] **Ein kaputtes Feld löscht alle Einstellungen** — `crates/client/src/settings.rs`: Parse-Fehler → Standardwerte → nächste Änderung überschreibt die Datei; `fs::write` ist nicht atomar. Über `settings.json.tmp` + `fs::rename` schreiben; unlesbare Datei vorher nach `.bad` umbenennen. Dasselbe gilt für `run.json` (`crates/client/src/run/save.rs`).
- [ ] **Tooltips bleiben nach dem Sprachwechsel alt** — die Tooltips der Icon-Knöpfe (`crates/client/src/ui/hud.rs`, `view_toggle.rs`) in `follow_locale` mit neu beschriften.
