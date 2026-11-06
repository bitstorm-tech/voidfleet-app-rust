# Offene Punkte (Stand 05.11.2026, nach dem PvE-Umbau)

## Spielgefühl und Balance (erst spielen, dann stellen)

- [ ] **Alle Zahlen** des Entwurfs sind Startwerte (Hülle, ATK, Karten, Gegner, Schrott, Preise); ohne Bot-Simulation entscheidet jetzt das Spielen. Der Rauchtest gewinnt Gefechte in Sektor 1 oft in Runde 1.
- [ ] **Länge des Gegnerzugs**: Der erfundene Gegnerzug der Szene `report` dauert rund 21 s; mit Leertaste überspringbar, vielleicht zu lang.
- [ ] **Funk-Dichte**: Die neuen Sprüche fallen vielleicht zu dicht; `direct_hit` ab 12 Hüllenschaden ist geschätzt, `incoming_fire` hängt am ersten Feuer eines Gegners (es gibt kein Ereignis für den Beginn des Gegnerzugs). Die Aufnahmen hat niemand angehört.

## Nachziehen

- [ ] **Icons auf der Sternenkarte**: Ereignis, Werft und Schwarzmarkt tragen nur Zeichen (`?`, `+`, `$`).
- [ ] **Modellgruppen**: In der Ansicht von hinten oben verdeckt der Balken das vordere Jägerpaar etwas; das Spotter-Paar ragt bei Maßstab 0,72 trotz schmalerem Signal Booster (4,59 breit, der Signalring allein 3,72) noch etwa 0,11 je Seite über das Pad, ganz beheben nur über die Versätze in `formation.rs`.
- [ ] **Beschriftungen**: In der Draufsicht steht die Absicht einer Back-Einheit über der Front-Einheit davor (etwa „Zielfeuer 2“ der Plünderer über dem Wachkreuzer-Paar); Lage der Captions je Reihe prüfen (`scene/labels.rs`).
- [ ] **Wiedergabe**: Ausweichen zeigt nur Triebwerksblitz und Marke, keinen Schlenker als Flugbahn; die Kamera neigt sich nur zum Handelnden, nicht zum Ziel; die Zeile „bricht auseinander, bevor er feuern kann“ für im Spielerzug gefallene Gegner fehlt; der Schild aus Alarmbereitschaft zu Beginn des Gegnerzugs hängt optisch am vorigen Schritt.
- [ ] **Flackern-Formel** `(max_hull + 3) / 4` ist im Core nicht öffentlich; der Client rechnet sie für den Ereignistext nach (`crates/core/src/run.rs`, `EventOption::Embrace`).
- [ ] **Toon-Material** kann weiterhin Geister zeichnen, obwohl die Planung keine Geister mehr kennt; alte PvP-Markenarten (Taunt, HeadStart …) in `scene/markers.rs` sind unbenutzt.
- [ ] **`docs/architektur.html`** beschreibt die PvP-Architektur mit Server und ist veraltet.

## Schiffsmodelle (Detailpass und Verwitterung, 06.11.2026)

- [ ] **Kartenbilder der Flaggschiffe** (`assets/images/cards/bastion.jpg`, `vanguard.jpg`, `pendulum.jpg`, `pyre.jpg`) zeigen noch die glatten Modelle; neu über fal.ai mit dem `hero`-Render als Vorlage.
- [ ] **Unbenutzte Modelle**: Hive, Mirage, Drohne, Repair Drone und Trugbild lädt das Spiel nicht mehr; sie stehen noch in der groben Fassung (löschen oder umbauen).
- [ ] **Feine Verwitterung auf hochauflösenden Bildschirmen**: Fugen und Kantenabrieb blenden nach der Mip-Stufe in physischen Pixeln aus (`FINE_LOD` in `toon.wesl`), auf Retina zeigt schon das Brett sie; Flimmern in Bewegung ist ungeprüft. Abhilfe wäre ein Bias nach dem Skalierungsfaktor des Fensters.
- [ ] **Ruß an Waffen** gibt es nur an kreisrunden Mündungsdeckeln (`weather.py`); Läufe ohne Deckel bleiben sauber.
- [ ] **Größe der GLBs**: 28 MB statt 8 MB, jedes Wrack bettet seine Maske ein zweites Mal ein, und etwa ein Viertel der Texel liegt auf verdeckten Flächen.
- [ ] **Wappen des Siegels**: In der zweiten Kamera (Gegner von vorn oben) verdeckt der Turm es zum Teil.
- [ ] **Masken-Loader** (`scene/toon/gltf.rs`) ersetzt das Bild unter demselben Label durch die lineare Fassung mit Mip-Kette; Bevy 0.20 erlaubt das, beim nächsten Bevy-Update prüfen.

## Aus dem Audit vom 02.11.2026, noch gültig

- [ ] **Mausrad-Zoom schreibt in jedem Bild `settings.json`** — `crates/client/src/scene/camera.rs`: auch früh abbrechen, solange `zoom_offset` noch nicht bei `zoom_target` ist.
- [ ] **Ein kaputtes Feld löscht alle Einstellungen** — `crates/client/src/settings.rs`: Parse-Fehler → Standardwerte → nächste Änderung überschreibt die Datei; `fs::write` ist nicht atomar. Über `settings.json.tmp` + `fs::rename` schreiben; unlesbare Datei vorher nach `.bad` umbenennen. Dasselbe gilt für `run.json` (`crates/client/src/run/save.rs`).
- [ ] **Tooltips bleiben nach dem Sprachwechsel alt** — die Tooltips der Icon-Knöpfe (`crates/client/src/ui/hud.rs`, `view_toggle.rs`) in `follow_locale` mit neu beschriften.
