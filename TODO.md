# Offene Punkte (Stand 05.11.2026, nach dem PvE-Umbau)

## Spielgefühl und Balance (erst spielen, dann stellen)

- [ ] **Alle Zahlen** des Entwurfs sind Startwerte (Hülle, ATK, Karten, Gegner, Schrott, Preise); ohne Bot-Simulation entscheidet jetzt das Spielen. Der Rauchtest gewinnt Gefechte in Sektor 1 oft in Runde 1.
- [ ] **Länge des Gegnerzugs**: Der erfundene Gegnerzug der Szene `report` dauert rund 21 s; mit Leertaste überspringbar, vielleicht zu lang.
- [ ] **Funk-Dichte**: Die neuen Sprüche fallen vielleicht zu dicht; `direct_hit` ab 12 Hüllenschaden ist geschätzt, `incoming_fire` hängt am ersten Feuer eines Gegners (es gibt kein Ereignis für den Beginn des Gegnerzugs). Die Aufnahmen hat niemand angehört.

## Daten und Editor (begonnen 12.11.2026)

Alle Zahlen, Inhalte und Texte stehen als JSON in `assets/data/` (`docs/daten.md`): Gegner, Karten, Admirale, Stellgrößen, die Sternenkarten samt Verband je Knoten und die Texte beider Sprachen; der Kartengenerator ist weg, die neun Startkarten stammen noch aus ihm.

- [ ] **Texte, die Inhalte nacherzählen**: Das Kapitel „Karten“ der Spielregeln (`rules_section.cards` in `assets/data/text/`) beschreibt das Startdeck („zehn Karten: dreimal Torpedo …“) und die Aufwertung („2 mehr oder 1 weniger“) in Worten; beides steht jetzt je Admiral und je Karte in den Daten und kann davon abweichen. Ebenso feste Wörter wie „doppelt“ und „halbe ATK“.
- [ ] **Texte prüfen nur die Ids mit Liste**: Fehlt einer neuen Variante eines Enums ohne `ALL` der Text, fällt das erst im Test `every_id_has_text` auf, wenn die Variante dort eingetragen ist, sonst beim Anzeigen (Absturz mit „Text … fehlt“).
- [ ] **Neuladen ohne Neubauen**: `enemies.json`, `cards.json`, `admirals.json` und `rules.json` sind eingebettet (`crates/core/src/data.rs`), die Texte ebenso (`crates/client/src/i18n/mod.rs`); der Gegnereditor braucht sie zur Laufzeit.
- [ ] **Gegnereditor**: Hülle, ATK, Panzerung und Muster der Gegner im Spiel stellen und probespielen; setzt das Neuladen voraus.
- [ ] **Karteneditor, was fehlt** (`docs/daten.md`, „Der Karteneditor“): Der Verband wird in einem Raster aus Tasten gestellt, nicht auf dem 3D-Brett. Knoten lassen sich in ihrer Reihe nicht verschieben (löschen und neu anlegen). Das Feld des Knotens rollt nicht: Ein Verband mit vielen Verstärkungen ragt in einem kleinen Fenster unten hinaus. Kein Rückgängig außer „Verwerfen“. Das Probespiel nimmt immer das Startdeck, ohne Module. Eine Datei, die sich außen ändert, während sie offen ist, lädt er nicht neu; „Speichern“ überschreibt sie.
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

- [ ] **Vorschau** (`tools/ships/preview.py`, `look.py`) rendert noch Clean Toon mit Linien und zeigt nicht, was das Spiel zeigt; auf den dreckigen Look umstellen (Verlauf, Verwitterung mit Plattenfasen und Feinstruktur, keine Linien), dann passen auch die `hero`-Renders als Kartenvorlage wieder.
- [ ] **Kleinteile im Spiel ansehen**: Die dreizehn am 12.11.2026 umgebauten Schiffe sind seit dem 13.11.2026 je Schiff bei größtem Zoom angesehen (Dev-Szene `show`), die Zweifel aus jenem Umbau dabei nicht eigens: Flak-Gehäuse in der Akzentfarbe fallen an den Flaggschiffen (Bernstein, Glutorange, Rot) stärker auf als am Belagerer; die Flacheisen des Scrap King lesen eher als Stange denn als Naht; die drei Streben in der Schüssel des Signal Booster verändern sein Kennzeichen; die Rückwand-Fenster des Horizon Cruiser leuchten von hinten kräftig; Kühlrippen durchdringen an Vanguard und Pendulum die Nachbardüse.
- [ ] **Start scheitert bei knappem Arbeitsspeicher** (Asahi Linux, Mesa-Treiber): Der Treiber lehnt Grafikspeicher ab, sobald der verfügbare Arbeitsspeicher (ohne Swap) unter etwa 1 GB fällt („Not enough free system memory“, `hk_device_memory.c`), Bevy beendet das Spiel dann. Der Client holt beim Start etwa 0,8 GB im Prozess und 1,9 GB Grafikspeicher, davon 255 MB für das Bild mit 4-facher Kantenglättung in 3840 × 2160, 130 MB für die Tiefe, 130 MB für die beiden HDR-Bilder, 240 MB für die Masken (gemessen vor dem 13.11.2026; seither sind sie als BC7 komprimiert und dafür an den großen Schiffen doppelt so dicht, zusammen 238 MB; neu gemessen ist der Start nicht). Die Kantenglättung ist deshalb eine Einstellung (Tab „Video“, Vorgabe 4-fach): 2-fach spart etwa 190 MB, aus etwa 350 MB. Offener Hebel: nach einem Fehlstart neu versuchen statt zu beenden. `glibc` hält außerdem etwa 250 MB freigegebene Puffer fest (`MALLOC_MMAP_THRESHOLD_=1048576` gibt sie zurück, ändert an den Fehlstarts aber nichts).
- [ ] **Flimmern** dünner Kleinteile (Antennen, Geländer, Raketenflossen), der Feinteile (Gitterstäbe, Steigeisen, Wegelinien, Ketten, Handläufe) und der Feinstruktur in Bewegung: an der Bastion auf dem Mac ohne Befund (13.11.2026), an den übrigen Schiffen ungeprüft.
- [ ] **Namen**: Modul `scene/toon`, `ToonMaterial`, `toon.wesl` und die Vertragsart `toon` heißen noch nach dem früheren Look.

## Schiffsmodelle (Detailpass und Verwitterung, 06.11.2026)

- [ ] **Kartenbilder der Flaggschiffe** (`assets/images/cards/bastion.jpg`, `vanguard.jpg`, `pendulum.jpg`, `pyre.jpg`) zeigen noch die glatten Modelle; neu über fal.ai mit dem `hero`-Render als Vorlage.
- [ ] **Unbenutzte Modelle**: Hive, Mirage, Drohne, Repair Drone und Trugbild lädt das Spiel nicht mehr; sie stehen noch in der groben Fassung (löschen oder umbauen).
- [ ] **Feine Verwitterung auf hochauflösenden Bildschirmen**: Fugen, Plattenfasen und Kantenabrieb blenden nach dem Maßstab in physischen Pixeln aus (`FINE_LOD` in `toon.wesl`), auf Retina zeigt schon das Brett sie; Flimmern in Bewegung ist ungeprüft. Abhilfe wäre ein Bias nach dem Skalierungsfaktor des Fensters.
- [ ] **Ruß an Waffen** gibt es nur an kreisrunden Mündungsdeckeln (`weather.py`); Läufe ohne Deckel bleiben sauber.
- [ ] **Verdeckte Texel**: Etwa ein Viertel der Texel einer Maske liegt auf verdeckten Flächen (Unterseiten aufgesetzter Teile, Durchdringungen), an der Bastion mit ihren Feinteilen eher mehr; ihre Maske ist mit 6048 px die größte (Grenze 8192 px). Inseln verdeckter Flächen klein zu skalieren spart Speicher und Backzeit.
- [ ] **Wappen des Siegels**: In der zweiten Kamera (Gegner von vorn oben) verdeckt der Turm es zum Teil.

## Nahaufnahme (13.11.2026)

Alle 15 Schiffe, die das Spiel lädt, tragen Fasen, glatte Rundteile, Feinteile und Beschriftung; alle haben Verdeckung in der Maske, Plattenfasen und Feinstruktur aus dem Shader und komprimierte Masken (`docs/stilregeln.md`, „Nahaufnahme“).

- [ ] **Im Spiel prüfen**: Auf dem Mac lief nur die Bastion in dieser Fassung. Die schwerste Szene hat jetzt gut 300k Dreiecke statt etwa 180k, die Masken belegen 238 statt 106 MB. Wird der Speicher knapp, die Einzelschiffe der Gegner in `look.WEATHER_DENSE` auf 96 setzen (etwa 60 MB weniger). Der Shader liest je Pixel einer verwitterten Fläche elf Texel; wird es eng, die grobe Lage der Feinstruktur streichen (`DETAIL.y` in `toon.wesl`).
- [ ] **Wracks ansehen**: Die Wracks der neuen Fassungen (bis 41,7k Dreiecke) sind gebaut und ohne Warnung, angesehen hat sie niemand. `wreck.py` meldete am Scrap Raider einmal „Hülle offen“ an einem Stück mit einer dünnen Schweißraupe; das Teil ist gestrichen, die Ursache nicht untersucht.
- [ ] **Kennnummern** unter 0,16 Höhe (Flanken von Vanguard, Dreadnought, Titan Carrier) sind dünn und kontrastarm; `parts.number` braucht für kleine Höhen einen kräftigeren Strich.
- [ ] **Helfer, die jetzt mehrfach in den Modulen stehen** und nach `parts.py` gehören: die Stellstreben an Düsenglocken (dieselbe Schleife in zehn Modulen), eine Tür in einer Wand (`door` in drei Fassungen), ein Blech auf rundem Rumpf (`skin` in drei Fassungen), `stairs` quer und ungespiegelt (`gangway`), Gitterrost auf geneigtem Deck (`walkway`, `catwalk`), Schweißnähte (`welds`), der Munitionskasten.
- [ ] **Zweifel aus dem Umbau**: Die dunklen Halter auf den leuchtenden Schneiden der Vanguard greifen ins Kennzeichen ein; an der Pyre liegt das Fensterband des Werfers jetzt auf der oberen Schrägfläche (an der Flanke steckte es im Rumpf); die hellen Heftnähte um die Flicken des Scrap King prägen ihn in der Nahaufnahme stark; der Titan Carrier ist durch die Rettungsinseln 3 % breiter; an der Dreadnought rechnen die Teile auf den Turmdächern die Dachhöhe aus `parts.turret` nach.
- [ ] **Kartenbilder und `hero`-Renders** aller Schiffe zeigen die alten Modelle (siehe „Schiffsmodelle“).
- [ ] **Größe im Repository**: Die Masken sind als KTX2 zusammen 80 MB groß (`assets/models/ships` 120 MB) und wachsen mit jedem Neubau um ihre volle Größe in der Git-Historie.
- [ ] **Schlagschatten** rechnet das Spiel weiter nicht; die gebackene Verdeckung reicht 0,4 weit und kennt die Lichtrichtung nicht.
- [ ] **Wrackstücke**: Die Feinstruktur hängt an den Koordinaten des Stücks, nicht des Schiffs, und springt deshalb beim Zerbrechen; im Blitz der Explosion sieht man es nicht, geprüft ist es nicht.
- [ ] **Screenshots auf dem Linux-Server**: Der Software-Renderer dekodiert BC7 sehr langsam, für Probeläufe die Maske unkomprimiert schreiben (`crates/client/AGENTS.md`, „Werkzeuge“). Zum Ansehen eines Schiffs dient die Dev-Szene `show`.

## Aus dem Audit vom 02.11.2026, noch gültig

- [ ] **Mausrad-Zoom schreibt in jedem Bild `settings.json`** — `crates/client/src/scene/camera.rs`: auch früh abbrechen, solange `zoom_offset` noch nicht bei `zoom_target` ist.
- [ ] **Ein kaputtes Feld löscht alle Einstellungen** — `crates/client/src/settings.rs`: Parse-Fehler → Standardwerte → nächste Änderung überschreibt die Datei; `fs::write` ist nicht atomar. Über `settings.json.tmp` + `fs::rename` schreiben; unlesbare Datei vorher nach `.bad` umbenennen. Dasselbe gilt für die Speicherstände `runs/<seed>.json` (`crates/client/src/run/save.rs`).
- [ ] **Tooltips bleiben nach dem Sprachwechsel alt** — die Tooltips der Icon-Knöpfe (`crates/client/src/ui/hud.rs`, `view_toggle.rs`) in `follow_locale` mit neu beschriften.
