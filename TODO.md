# Offene Punkte (Stand 05.11.2026, nach dem PvE-Umbau)

## Spielgefühl und Balance (erst spielen, dann stellen)

- [ ] **Alle Zahlen** des Entwurfs sind Startwerte (Hülle, ATK, Karten, Gegner, Schrott, Preise); ohne Bot-Simulation entscheidet jetzt das Spielen. Der Rauchtest gewinnt Gefechte in Sektor 1 oft in Runde 1.
- [ ] **Länge des Gegnerzugs**: Der erfundene Gegnerzug der Szene `report` dauert rund 21 s; mit Leertaste überspringbar, vielleicht zu lang.
- [ ] **Funk-Dichte**: Die neuen Sprüche fallen vielleicht zu dicht; `direct_hit` ab 12 Hüllenschaden ist geschätzt, `incoming_fire` hängt am ersten Feuer eines Gegners (es gibt kein Ereignis für den Beginn des Gegnerzugs). Die Aufnahmen hat niemand angehört.

## Daten und Editor (begonnen 12.11.2026)

Alle Zahlen, Inhalte und Texte stehen als JSON in `assets/data/` (`docs/daten.md`): Gegner, Karten, Admirale, Stellgrößen, die Sternenkarten samt Verband je Knoten und die Texte beider Sprachen; der Kartengenerator ist weg, die neun Startkarten stammen noch aus ihm.

- [ ] **Texte, die Inhalte nacherzählen**: Das Kapitel „Karten“ der Spielregeln (`rules_section.cards` in `assets/data/text/`) beschreibt das Startdeck („zehn Karten: dreimal Torpedo …“) und die Aufwertung („2 mehr oder 1 weniger“) in Worten; beides steht jetzt je Admiral und je Karte in den Daten und kann davon abweichen. Ebenso feste Wörter wie „doppelt“ und „halbe ATK“.
- [ ] **Texte prüfen nur die Ids mit Liste**: Fehlt einer neuen Variante eines Enums ohne `ALL` der Text, fällt das erst im Test `every_id_has_text` auf, wenn die Variante dort eingetragen ist, sonst beim Anzeigen (Absturz mit „Text … fehlt“).
- [ ] **Neuladen ohne Neubauen**: `enemies.json`, `cards.json`, `admirals.json` und `rules.json` sind eingebettet (`crates/core/src/data.rs`), die Texte ebenso (`crates/client/src/i18n/mod.rs`); der Editor im Spiel braucht sie zur Laufzeit.
- [ ] **Dev-Einstieg auf eine Karte oder einen Knoten**: eine Dev-Szene, die eine bestimmte Kartendatei lädt und direkt das Gefecht eines Knotens startet, mit Neuladen bei Dateiänderung.
- [ ] **Editor im Spiel** hinter einem Dev-Schalter: Knoten setzen, Kanten ziehen, Verband auf dem Brett stellen, Werte der Gegner stellen, probespielen; schreibt dieselben Dateien. Braucht einen Schreiber, der kurze Listen in einer Zeile lässt (`serde_json` bricht jede Liste um).
- [ ] **Abwechslung**: Jeder Knoten hat genau einen Verband, mit drei Karten je Sektor wiederholen sich Runs. Mehr Karten entwerfen oder je Knoten mehrere Verbände zur Wahl erlauben.
- [ ] **Mechanik mit festen Zahlen im Code**: was Regel und nicht Stellgröße ist, etwa Halbieren (Ausweichen, Deckung, Gegenmaßnahmen, Notfall), Verdoppeln (Aufladen, Torpedo-Staffel, Fokusfeuer auf den Vorstoß, Rammkurs), der Zielstörer mit −1 ATK je Karte, ein Stationsplatz je Alarmstufe Rot, und die Beute aus allen Karten.

## Nachziehen

- [ ] **Icons auf der Sternenkarte**: Ereignis, Werft und Schwarzmarkt tragen nur Zeichen (`?`, `+`, `$`).
- [ ] **Modellgruppen**: In der Ansicht von hinten oben verdeckt der Balken das vordere Jägerpaar etwas; das Spotter-Paar ragt bei Maßstab 0,72 trotz schmalerem Signal Booster (4,59 breit, der Signalring allein 3,72) noch etwa 0,11 je Seite über das Pad, ganz beheben nur über die Versätze in `formation.rs`.
- [ ] **Beschriftungen**: In der Draufsicht steht die Absicht einer Back-Einheit über der Front-Einheit davor (etwa „Zielfeuer 2“ der Plünderer über dem Wachkreuzer-Paar); Lage der Captions je Reihe prüfen (`scene/labels.rs`).
- [ ] **Wiedergabe**: Ausweichen zeigt nur den Triebwerksblitz, keinen Schlenker als Flugbahn; die Kamera neigt sich nur zum Handelnden, nicht zum Ziel; die Zeile „bricht auseinander, bevor er feuern kann“ für im Spielerzug gefallene Gegner fehlt; der Schild aus Alarmbereitschaft zu Beginn des Gegnerzugs hängt optisch am vorigen Schritt.
- [ ] **Toon-Material** kann weiterhin Geister zeichnen, obwohl die Planung keine Geister mehr kennt; die Marken in `scene/markers.rs` (auch `DropMarker`) liegen nur noch in der Probe, das Gefecht legt keine.
- [ ] **`docs/architektur.html`** beschreibt die PvP-Architektur mit Server und ist veraltet.

## Dreckiger Look (Umstellung vom 11.11.2026)

- [ ] **Vorschau** (`tools/ships/preview.py`, `look.py`) rendert noch Clean Toon mit Linien und zeigt nicht, was das Spiel zeigt; auf den dreckigen Look umstellen (Verlauf, Verwitterung mit Korn, keine Linien), dann passen auch die `hero`-Renders als Kartenvorlage wieder.
- [ ] **Kleinteile im Spiel ansehen**: Die dreizehn am 12.11.2026 umgebauten Schiffe sind in der Blender-Vorschau und in der Modellschau auf Brettgröße geprüft, nicht je Schiff in der Nahaufnahme. Zweifel aus dem Umbau: Flak-Gehäuse in der Akzentfarbe fallen an den Flaggschiffen (Bernstein, Glutorange, Rot) stärker auf als am Belagerer; die Flacheisen des Scrap King lesen eher als Stange denn als Naht; die drei Streben in der Schüssel des Signal Booster verändern sein Kennzeichen; die Rückwand-Fenster des Horizon Cruiser leuchten von hinten kräftig; Kühlrippen durchdringen an Vanguard und Pendulum die Nachbardüse.
- [ ] **Start scheitert bei knappem Arbeitsspeicher** (Asahi Linux, Mesa-Treiber): Der Treiber lehnt Grafikspeicher ab, sobald der verfügbare Arbeitsspeicher (ohne Swap) unter etwa 1 GB fällt („Not enough free system memory“, `hk_device_memory.c`), Bevy beendet das Spiel dann. Der Client holt beim Start etwa 0,8 GB im Prozess und 1,9 GB Grafikspeicher, davon 255 MB für das Bild mit 4-facher Kantenglättung in 3840 × 2160, 130 MB für die Tiefe, 130 MB für die beiden HDR-Bilder, 240 MB für die Masken. Die Kantenglättung ist deshalb eine Einstellung (Tab „Video“, Vorgabe 4-fach): 2-fach spart etwa 190 MB, aus etwa 350 MB. Offene Hebel: Masken komprimiert (BC7/ASTC), nach einem Fehlstart neu versuchen statt zu beenden. `glibc` hält außerdem etwa 250 MB freigegebene Puffer fest (`MALLOC_MMAP_THRESHOLD_=1048576` gibt sie zurück, ändert an den Fehlstarts aber nichts).
- [ ] **Flimmern** dünner Kleinteile (Antennen, Geländer, Raketenflossen) und des Korns in Bewegung ist ungeprüft.
- [ ] **Namen**: Modul `scene/toon`, `ToonMaterial`, `toon.wesl` und die Vertragsart `toon` heißen noch nach dem früheren Look.

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
- [ ] **Ein kaputtes Feld löscht alle Einstellungen** — `crates/client/src/settings.rs`: Parse-Fehler → Standardwerte → nächste Änderung überschreibt die Datei; `fs::write` ist nicht atomar. Über `settings.json.tmp` + `fs::rename` schreiben; unlesbare Datei vorher nach `.bad` umbenennen. Dasselbe gilt für die Speicherstände `runs/<seed>.json` (`crates/client/src/run/save.rs`).
- [ ] **Tooltips bleiben nach dem Sprachwechsel alt** — die Tooltips der Icon-Knöpfe (`crates/client/src/ui/hud.rs`, `view_toggle.rs`) in `follow_locale` mit neu beschriften.
