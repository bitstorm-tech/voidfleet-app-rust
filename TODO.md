# Offene Punkte (Stand 14.11.2026, nach dem PvE-Umbau)

## Der Durchbruch (14.11.2026)

Der Boss von Sektor 3 hat eine zweite Hälfte: Fällt das Siegel, bricht der Unbekannte Kontakt durch, das einzige Alien-Schiff des Spiels (`docs/gegner.md`, „Der Durchbruch“; ansehen mit `just scene breach`).

- [ ] **Gemessen, nicht gespielt**: Die Zahlen beider Hälften stammen aus dem Simulator (`docs/balance.md`). Osei gewinnt am seltensten (53 % gegen 56 bis 59 %): Mit ATK 8 dauert sein Gefecht gegen das Fremde am längsten, und sein Abwehrfeuer trifft einen einzelnen Schützen nur einmal je Runde. Ob der Schlag mit 52 Schaden über alle Spalten für einen Menschen fair ist, zeigt das Spielen: Wer weder Kurzsprung noch Gegenmaßnahmen, EMP-Stoß oder Funkstille im Deck hat, schluckt ihn mit Schilden nur zum Teil.
- [ ] **Funksprüche** `seal_broken` und `unknown_contact` (je zwei Aufnahmen, ElevenLabs v3 über fal.ai) hat niemand angehört. Die beiden der Gegenflotte sind mit Regieanweisung aufgenommen (`[panicked]`, `[shouting]`); spricht die Stimme sie mit, neu aufnehmen.
- [ ] **Eigene Klänge fehlen**: Das Fremde feuert mit dem Plasmaschuss, der Riss klingt nach Reaktorbruch und EMP, und es brummt beim Eintreffen wie das Triebwerk eines Schlachtschiffs.
- [ ] **Der Schlag** über alle drei Spalten sieht aus wie jedes Spaltenfeuer (drei Strahlen), nur in seinem Türkis; ein eigenes Bild (ein Strahl aus dem Schlund, der über das Brett fegt) fehlt.
- [ ] **Texte nach dem Sieg**: Funk und Log sprechen auch nach dem Fall des Fremden von der „feindlichen Flotte“ (`radio_lines.victory`, `log_battle_ended.won`); ein Abspann fehlt, das Run-Ende zeigt nur zwei Sätze (`run_won_text`).
- [ ] **Tooltip der Gefechtsuhr** nennt „ab Runde 8“ (`hud_clock_hint`), auch wenn sie nach dem Durchbruch von vorn zählt.
- [ ] **Der Riss** ist das Sprung-Portal in Türkis und groß (`Fx::Warp`), dazu zwei Pulse; er schließt sich, bevor das Fremde hereinfällt. Ein eigenes Bild des Wurmlochs, das über die ganze Sequenz offen steht, fehlt, ebenso ein Wechsel der Musik.
- [ ] **Wiedergabe unter dem Software-Renderer**: Die Sequenz ist auf dem Linux-Server nur in Einzelbildern geprüft (etwa vier Bilder je Sekunde); Tempo und Reihenfolge der Takte (`BREACH_LEAD`, `BREACH_STAGGER` in `playback/plan.rs`) hat niemand in Echtzeit gesehen.
- [ ] **Karteneditor**: „Durchbruch“ setzt die Einheit; welcher Verband einen trägt, prüft niemand (sinnvoll ist er nur am Boss).
- [ ] **Modell des Fremden, Abstand zum Konzeptbild**: Von vorn stehen die Flügel flacher als die hohen Sicheln des Konzepts (so sind im Brettwinkel beide Leuchtfelder zu sehen); Länge zu Spannweite 1 : 0,50 statt 1 : 0,43; die Flughaut hat drei Lagen mit harter Grenze statt eines Verlaufs (die Zellen der Haut laufen über die Grenzen hinweg und mildern sie), und der Knochenschwamm ist ein Netz aus Bälkchen.
- [ ] **Shader für das Fremde**: Die Flughaut ist deckend (durchscheinend ist nur `hologram`); die großen Leuchtflächen wirken durch `GLOW_BOOST` (`toon/look.rs`) eher grell cyan als türkis (flach sind sie nicht mehr, die Haut zeichnet sie); lebend pulsiert nichts, auch nicht die Leuchtpunkte und Adern der Haut (erst im Todeskampf leuchten die Organe im Pulsschlag auf, `ToonState.lights` in `destruction.rs`).
- [ ] **Am Modell ungeprüft**: der größte Zoom (gesehen bis etwa 114 px je Einheit), die Brut außer in Brettgröße. In der Seitenansicht verdeckt der Lebensbalken den hinteren Flügel.
- [ ] **Haut des Fremden** (`tools/ships/skin.py`): gesehen nur unter dem Software-Renderer des Servers in der Szene `show`: auf dem Brett von oben und von der Seite, herangezoomt von der Seite (Kontakt und Brut) und von oben, zuletzt mit den BC7-Texturen. Ungeprüft: die Unterseite, Knochenschwamm und Maul ganz nah. Die Vorschau (`preview.py`) zeigt die Haut nicht. Der Schmutz der Maske in den Ritzen (`look.WEATHER_MATERIALS`) liegt auf dem Knochen als braune Flecken; ob er an einem Wesen bleiben soll, ist nicht entschieden.
- [ ] **Untergang der Wesen** (`scene/destruction.rs`, `BlastStyle::Flesh`): Anschwellen, Wunden, Platzen und blutende Brocken sind nur unter dem Software-Renderer in Einzelbildern gesehen (Szene `destroy` mit `VOIDFLEET_DEV_SHIP=unknown_contact` bzw. `brood`, etwa sechs Bilder je Sekunde), nicht in Echtzeit und nicht im Spiel nach dem Durchbruch. Die Klänge `flesh_strain` und `flesh_burst` (ElevenLabs über fal.ai) hat niemand angehört; Lautstärke und Tonhöhe (`rate` nach `tempo`) sind geschätzt.
- [ ] **Masken-Cache**: `look.py` hat Einträge für die Materialien des Fremden bekommen, und der Hash der Masken deckt die Datei ab: Der nächste Bau jedes anderen Schiffs backt neu (Bastion etwa 15 min). Der Inhalt sollte gleich bleiben, geprüft ist es nicht.

## Spielgefühl und Balance (erst spielen, dann stellen)

- [ ] **Balance, Stand 13.11.2026**: Karten, Module, Admirale und die drei Sternenkarten sind mit dem Simulator eingestellt (`just sim`, `docs/balance.md`), gespielt hat sie niemand. Offen laut Messung: Reservereaktor, Löschanlage und Panzerplatten sind doppelt so stark wie die übrigen Module; „Logbücher sichern“ gibt ein Modul ohne Preis; Sperrfeuer, Traktorstrahl, EMP-Stoß und Rammkurs sind für den Bot tote Karten; einzelne Knoten mit Belagerer enden für angeschlagene Spieler zu oft.
- [ ] **Neue Regeln ohne Anzeige**: Die Reparaturgrenze (`rules.battle.repair_limit`) steht nur im Text der Station; wie viel in diesem Gefecht noch repariert werden kann, zeigt die Oberfläche nicht (`flagship.repaired`). Der Stillstand, der nachlädt, ist nur an der Meldung beim Klick zu erkennen, nicht an der Station. Ein gespielter Kurzsprung liegt bis zum Gefechtsende in `Battle::spent`; die Stapelzähler zeigen ihn nicht.
- [ ] **Länge des Gegnerzugs**: Der erfundene Gegnerzug der Szene `report` dauert rund 21 s; mit Leertaste überspringbar, vielleicht zu lang.
- [ ] **Funk-Dichte**: Die neuen Sprüche fallen vielleicht zu dicht; `direct_hit` ab 12 Hüllenschaden ist geschätzt, `incoming_fire` hängt am ersten Feuer eines Gegners (es gibt kein Ereignis für den Beginn des Gegnerzugs). Die Aufnahmen hat niemand angehört.

## Daten und Editor (begonnen 12.11.2026)

Alle Zahlen, Inhalte und Texte stehen als JSON in `assets/data/` (`docs/daten.md`): Gegner, Karten, Admirale, Stellgrößen, die Sternenkarten samt Verband je Knoten und die Texte beider Sprachen; der Kartengenerator ist weg, die neun Startkarten stammen noch aus ihm.

- [ ] **Texte, die Inhalte nacherzählen**: Das Kapitel „Karten“ der Spielregeln (`rules_section.cards` in `assets/data/text/`) beschreibt das Startdeck („zehn Karten: dreimal Torpedo …“) und die Aufwertung („2 mehr oder 1 weniger“) in Worten; beides steht jetzt je Admiral und je Karte in den Daten und kann davon abweichen. Ebenso feste Wörter wie „doppelt“ und „halbe ATK“.
- [ ] **Texte prüfen nur die Ids mit Liste**: Fehlt einer neuen Variante eines Enums ohne `ALL` der Text, fällt das erst im Test `every_id_has_text` auf, wenn die Variante dort eingetragen ist, sonst beim Anzeigen (Absturz mit „Text … fehlt“).
- [ ] **Neuladen ohne Neubauen**: `enemies_defaults.json`, `cards.json`, `admirals.json` und `rules.json` sind eingebettet (`crates/core/src/data.rs`), die Texte ebenso (`crates/client/src/i18n/mod.rs`); der Gegnereditor braucht sie zur Laufzeit.
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
