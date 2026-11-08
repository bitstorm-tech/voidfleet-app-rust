# Stilregeln für Neubauten (Clean Toon)

Referenz ist die Iron Dreadnought (`fleet/iron_dreadnought.py`): große, klare Primärformen, gegliedert durch Sekundär-
und Tertiärformen, die zu Detailzonen mit Zweck gruppiert sind; flache Farbflächen in drei Tönen, harte Schlagschatten,
dunkle Konturen, ein paar leuchtende Akzente. Alle Schiffe, die das Spiel lädt, sind nach ihrem Vorbild gebaut; nur
Hive, Mirage, Drohne, Repair Drone und Trugbild (im PvE-Run unbenutzt) stehen noch in der groben Fassung. Vor einem
Umbau die Kontaktbögen ansehen (`preview.py -- --views board,hero,behind --sheet`).

## Arbeitsweise

- Ein Umbau ändert nur `fleet/<id>.py`. Neue Helfer kommen in dieses Modul; erst wenn ein zweites Schiff sie braucht,
  wandern sie nach `parts.py`. `parts.py`, `vf.py` und `palette.py` ändert nur, wer die gemeinsamen Dateien betreut, und
  vergleicht dabei alle Schiffe vor und nach der Änderung (Dreiecke, Maße, GLB-Bytes bzw. Positionen). Braucht ein Schiff
  eine Variante eines gemeinsamen Helfers, kopiert es ihn ins Modul. Fehlt eine Farbe in `palette.py`, kommt sie vorerst
  als benannte Konstante ins Modul und wird gemeldet.
- `build()` beginnt mit `m = vf.materials(s.name, "<klasse>")` und endet mit `return s.finish()`.
- Gemeinsame Helfer in `parts.py`: Baugruppen `turret`, `engine` (`n` Segmente für kleine Düsen), `badge` (Wappenplatte
  der Flaggschiffe), `trim_loop`, `trim_line`, `on_surface`; Kleinteile `turret_details` und `engine_details` (mit
  denselben Argumenten wie `turret` bzw. `engine` aufrufen), `vent` (Lüftungsgitter), `rcs` (Steuerdüsenblock); Formen
  `sym` (symmetrischer Umriss aus einer Hälfte), `stealth` (Tarnkappenprofil), `block` (Facettenblock auf Lagen), `beam`
  (kantige Strebe), `axle` (Walze entlang X), `ring_at` (Querschnitt eines Lofts, für Bänder, die dem Rumpf folgen),
  `bellows` (Blendenbalg um ein Rohr).
- Vor einem Umbau die jetzige Fassung rendern und die Bilder wegkopieren (die Vorschau überschreibt `renders/<id>/`).
- Kreislauf: `build.py -- <id>` (Dreiecke, KB, Maske, Warnungen) und `preview.py -- <id> --views board,zoom,behind,hero,wreck`,
  bei asymmetrischen Schiffen auch `enemy_zoom`. Fertig ist ein Schiff erst ohne Warnungen und nach dem Blick auf `board`
  in voller Größe (unten).

## Was wegfällt

Die grobe Fassung hatte nach einem zu kleinteiligen Vorgänger alles Feine gestrichen; zurück kommen nur Formen mit Zweck
in Detailzonen (unten). Weiter weg bleibt, was auf dem Brett nur Rauschen ist oder im Spiel zum schwarzen Fleck wird:

- Plattenfugen, Nieten, Schmutz, Kantenabrieb und Plattentöne als Geometrie: Sie kommen aus der Verwitterungs-Textur
  (gebackene Maske, im Shader hart gestuft). Die Geometrie liefert die Kanten und Stufen, an denen sie greift.
- Bevel; Fasen nur über `rbox` an Verschlüssen und Blenden.
- Zufalls-Greebles: Kleinteile ohne erkennbaren Zweck, gestreut statt gruppiert, und gleich große Teile in gleichen
  Abständen über das ganze Schiff (gleichmäßiges Rauschen).
- Fensterreihen aus Einzelkästen (ein durchgehendes Fensterband), Nadel-Antennen, Antennen mit Scheiben, Geländer,
  Leitern.
- Abstehende Kleinstteile unter 0,15 (Düsenkegel, Stummel, Bolzen): Im Spiel bekommt jedes erhabene Teil einen
  Konturring fester Breite, ein Bündel davon wird zum schwarzen Fleck. Die ersten RCS-Blöcke der Dreadnought mit drei
  Kegeldüsen sahen genau so aus; jetzt liegen die Düsenöffnungen flach auf (`parts.rcs`).
- Rohrleitungsbündel und lose Leitungen, Radiatoren aus Lamellen, Kühlrippen an Düsen, mehr als zwei Ringe je Lauf.
- Nebentürme, Flak-Türmchen und Raketenwerfer: Sie konkurrieren mit der Hauptbewaffnung, die die Klasse zeigt.
- Eigene Lichter an Kleinteilen (Positionslichter, Lampen an Kisten): Ein rotes Positionslicht am Brückenflügel zog auf
  dem Brett mehr Blick als die Klasse.

Ausnahme: Ein solches Teil ist das Kennzeichen des Schiffs (die Schüssel des Signal Boosters, die Arme der Repair
Drone); dann bleibt es, aber groß und grob.

## Was groß bleibt und was dazukommt

Die Primärformen bleiben groß und grob: wenige, große Hauptwaffen (die Dreadnought hat drei Zwillingstürme mit Radius
0,5 bzw. 0,46 und zwei Bugkanonen mit Lauf-Radius 0,24), Aufbauten aus zwei bis drei Blöcken (`vf.wedge`, schräge Front)
mit einem durchgehenden Fensterband, Düsen mit `parts.engine`, Emitter als flache Kuppeln am Deckrand statt Kugeln auf
Stielen, Kiel oder Schürzen unter dem Rumpf. Dazu kommen Sekundär- und Tertiärformen, die zeigen, wie das Schiff gebaut
ist und benutzt wird:

- **Drei Stufen.** Primärformen (Rumpf, Türme, Aufbau, Kanonen) ab etwa 0,5; Sekundärformen 0,15–0,6 (Plattenlage,
  Deckel, Ohren, Brückenflügel, Lüfter als Ganzes, RCS-Block, Düsenkranz, Hangarkinn); Tertiärformen unter 0,15, nur
  für Zoom und Karte (Stege, Linsen, Luken, Balgfalten, Mündungsringe, Düsenöffnungen, Torflügel).
- **Zweck aus dem Ort.** Lüfter über Maschinenräumen (achtern), RCS an den Rumpfecken, Kabelkanäle an der Flanke
  zu einer Kasematte, Blendenbälge an Rohrwurzeln, Entfernungsmesser an Türmen, Hangartor unten, Lagerkisten an
  einer Wand. Was keinen Ort mit Zweck findet, fällt weg.
- **Zonen statt Streuung.** Kleinteile sammeln sich um eine Sekundärform (eine Zone: eine Sekundärform, zwei bis fünf
  Tertiärformen). Zwischen den Zonen bleiben ruhige Flächen von mindestens etwa einer Einheit, nur mit Plattenlagen;
  dort arbeitet die Verwitterungs-Textur. Gleiche Teile höchstens zu dritt oder viert in einer Reihe (Stege, Spanten).
- **Plattenlagen statt Fugen.** Eine zweite Lage (0,05–0,07 dick, 0,1–0,15 vom Rand der ersten eingezogen), eine
  Kappe um die Bugspitze, ein Deckel auf einem Block. Die Lage wechselt das Material (`hull`, `armor`, `dark`), damit die
  Stufe auch ohne Linie als Hell-Dunkel-Wechsel liest; auf dem Brett ist sie die feinere Gliederung der Fläche.
- **Vertiefungen als Rahmen.** Löcher gibt es nicht (die Teile sind geschlossene, sich überlappende Körper): Eine dunkle
  Platte in einem flachen Rahmen aus dem Material der Fläche liest als Vertiefung (`parts.vent`, Hangartor).
- **Kleinteile an Baugruppen.** Wer `turret` oder `engine` benutzt, setzt `turret_details` bzw. `engine_details` mit
  denselben Argumenten dazu; Lichter bekommen die neuen Teile nicht.

Referenz sind die Detailzonen der Iron Dreadnought:

- **Bug:** Bugpanzer-Kappe (`armor`) um die Spitze; Kanonen mit Blendenbalg vor dem Verschluss, Kühlmantel, Laufschelle
  und zweiteiliger Mündungsbremse (Stahlkern im Spalt).
- **Türme:** `parts.turret_details`, also Barbettenring, Entfernungsmesser-Ohren mit Linse, Turmluke, Blendenbälge,
  Mündungsringe.
- **Flanke:** zweite Plattenlage auf der Schürze; Kabelkanal (`dark`, 0,1 × 0,1) unter der Deckkante bis zur Kasematte,
  mit Anschlusskasten; Kasematte mit Deckel, Luke, Lüfter und Lichtband; `parts.rcs` an den vorderen und hinteren
  Rumpfecken.
- **Brücke:** Brückenflügel als offene Nocken unter den Seitenfenstern; Mast mit Radarplatte (15° zurückgeneigt),
  Querträger und Mastlicht.
- **Achterdeck und Heck:** Maschinenlüfter (`parts.vent`) hinter dem Aufbau und an den Seiten, je eine Lagerkiste an der
  Wand des Dorsaldecks; `parts.engine_details` an jeder der vier Düsen.
- **Unterseite:** Kinn unter dem Bug mit zweiflügeligem Hangartor nach vorn (in `hero` sichtbar), vier Spanten je Seite
  am V-Boden, zweite Lage unter der unteren Schürze.
- **Ruhig:** Hauptdeck vor den Seitentürmen, Dorsaldeck zwischen den Türmen (nur eine Luke), Schürzenflanken, Flächen
  des Aufbaus.

## Verwitterung

Fugen, Plattenton, Schmutz, Ruß und Kantenabrieb kommen für alle Schiffe gleich aus `weather.py` (nach dem Bau, vor dem
Wrack; Kanäle, Ablauf und Cache im Modulkommentar), nicht aus dem Schiffsmodul. Auf dem Brett bleiben davon Plattenton,
Schmutz und Ruß, im Zoom und auf der Karte kommen Fugen und Abrieb dazu.

- Schmutz sammelt sich in Ritzen (wo Teile aufeinandertreffen), läuft als Schlieren hinter allem nach achtern, was
  mindestens 0,06 hoch ist, und Ruß sitzt an Düsen (`core`) und Mündungen: kreisrunde Deckel ab acht Ecken an runden
  Materialien, waagrecht und vorn frei (wie `parts.turret`). Eine Waffe ohne solchen Deckel bekommt keinen Ruß.
- Abrieb sitzt an sichtbaren konvexen Kanten ab 40° lackierter Flächen, Platten liegen an den Schiffsachsen.
- Grad je Schiff in `look.WEATHER` (Schrott 2,2, Mirage 0,5, Trugbild 0), Anteile je Material in
  `look.WEATHER_MATERIALS`. Ein neuer Materialschlüssel gilt als lackiert (eckig mit Platten, rund ohne); unlackierte
  Sonderfarben (Metall, Glas, Rost) dort eintragen.
- Die erste Vorschau eines geänderten Schiffs backt seine Maske (bis etwa 20 s), danach kommt sie aus dem Cache.

## Mindestgröße bei Brettgröße

Auf dem Brett ist eine Blender-Einheit etwa 22 px (Flaggschiffe 24 px), im Zoom etwa 70 px, auf der Karte 80–90 px. Die
Vorschau zeichnet Linien von 0,7–1,8 px (Außenkontur, Silhouetten, Knicke über 40°); das Spiel zeichnet keine Knicke,
aber um jedes erhabene Teil, das vor etwas anderem liegt, einen Konturring (Inverted Hull je Material,
`crates/client/src/scene/toon/outline.rs`) in einem abgedunkelten Ton der Fläche, auf dem Brett etwa 1 px, im Zoom bis
1,8 px. Daraus:

- Flächen mit Tönen (Primär- und Sekundärformen): kleinste Abmessung mindestens 0,15 (≈ 3,3 px, neben der Kontur
  bleiben etwa 1,5 px Ton), besser 0,2.
- Flach aufliegend, Relief bis 0,05 (≈ 1 px): ab 0,06, also Stege, Luken, Linsen, Düsenöffnungen, Torflügel. Auf dem
  Brett verschieben sie nur den Ton ihrer Fläche (ein Gitter wird ein dunkles Rechteck), deshalb nur auf einer
  Sekundärform von mindestens 0,3, deren Ton als Ganzes liest; im Zoom sind 0,06 gut 4 px.
- Erhabene Kleinteile unter 0,15 (Relief über 0,05) nur an einer größeren Form, deren Umriss sie verlängern (Ohren,
  Bälge, Mündungsringe, Querträger), nie freistehend und höchstens zwei oder drei je Teil.
- Rohre und Läufe: Durchmesser mindestens 0,18. Leisten und Kanäle dünner als 0,15 nur, wenn sie mindestens 0,8 lang
  sind (Kabelkanal 0,1, Querträger 0,07).
- Zwischen zwei erhabenen Teilen mindestens 0,1 Luft, sonst verschmelzen ihre Konturringe zu einem schwarzen Fleck.
  Spalte innerhalb einer Sekundärform (Stege, Torflügel, Bremsscheiben) ab 0,06: Sie schließen sich auf dem Brett
  absichtlich und öffnen sich im Zoom.
- Lichter dürfen kleiner sein, weil sie leuchten: Punktlichter ab Radius 0,08, Lichtbänder ab 0,04 dick und 0,5 lang.
- Neue Kleinteile einmal im Spiel ansehen, nur dort zeigen sich die Konturringe: Dev-Szenen in
  `crates/client/src/dev/scenes.rs` (`hit` zeigt den Belagerer, also die Dreadnought), Screenshot mit
  `VOIDFLEET_SMOKE_FRAMES` und `VOIDFLEET_SMOKE_SCREENSHOT`.

## Palette und Akzent

`vf.materials` liefert alles; Farben stehen zentral in `palette.py` und kommen nie als Hex in ein Schiffsmodul.

- `hull` für den Rumpf und die großen Flächen, `armor` (je Klasse einen Hauch heller als der Rumpf, auch beim hellen
  Rumpf der Unterstützer) für aufgesetzte Lagen wie Schürzen und Kasematten, `dark` für Gehäuse, Verschlüsse, Barbetten,
  Kiel. Hell-Dunkel-Wechsel zwischen Lagen macht die Form.
- `accent` nur für das, was die Klasse zeigt (bei der Dreadnought die Türme): etwa ein Zehntel bis ein Fünftel der
  sichtbaren Fläche. Klassen: Jäger koralle mit orangen Lichtern, Kreuzer kornblumenblau mit blauen, Schlachtschiffe
  stahlblau mit kühlem Blau, Unterstützer mint mit grünen Lichtern und hellem Rumpf. Flaggschiffe: Akzent im Farbton des
  Admirals, `signal` (exakt die UI-Farbe, ohne Glühen) für Kennzeichen, `trim` (Gold, etwa 0,07 breit) als Zierlinie
  entlang des Rumpfs, `glow` und `core` in der Admiralsfarbe.
- `steel` für Läufe und Düsenglocken, `glass` für Kanzeln. Lichter sparsam und mit Absicht: Brennkammern (`core`), ein
  Mastlicht, ein bis zwei Lichtbänder (`glow`), je Brücke ein Fensterband; Sonderlichter aus der Palette über `vf.light`
  (etwa `palette.SHIELD`, `palette.PLASMA`).
- Sonderfarben einzelner Schiffe stehen in `palette.py`: Rost und Schrottgrau (Scrap Raider), Messing (Pendulum),
  Warnstreifen (Repair Drone), Cyan (Signal Booster), Violett der Drohne, `PHANTOM` (Tarnrumpf der Mirage).

## Normalen

Rund oder eckig entscheidet das Material: eckig (`hull`, `armor`, `dark`, `accent`, `trim`) gibt einen Ton je Facette, rund
(`hull_round`, `armor_round`, `dark_round`, `accent_round`, `steel`, `glass`) saubere Bänder; Kanten über 38° bleiben immer
hart, Bevel gibt es nicht. Rohre, Türme, Barbetten, Masten, Düsen und runde Rümpfe also rund, Kisten, Platten, Keil- und
Facettenrümpfe eckig. Nie einen runden Rumpf eckig (Facettenstreifen) und nie einen Keilrumpf rund (Verläufe quer über
die Facetten).

## Dreiecksbudget

Obergrenzen: Schlachtschiffe 12k, Flaggschiffe 14k, Kreuzer 9k, Jäger und Unterstützer 7k, Drohne 3k, Trugbild 1,5k
(bleibt flach). Die Dreadnought liegt bei 11,2k (grobe Fassung 6,5k); von den 4,7k für Details gehen etwa 2k in die vier
Türme, 0,8k in die Bugkanonen, 0,6k in die Düsen, 0,4k in die Lüfter, 0,4k in die RCS-Blöcke und 0,5k in den Rest. Das
meiste geht in runde Teile: große mit 20–24 Segmenten, Ringe und Bälge mit 12–18, Düsenöffnungen mit 8, `smooth` mit
`subdiv` 1–2; Kasten, Platten und Stege kosten fast nichts. Die Wracks (wreck.py) entstehen automatisch und haben etwa
das 1,5-Fache (Dreadnought 16,5k Dreiecke, 0,75 MB); ansehen (`wreck`) muss man sie trotzdem.

## Silhouette und Wiedererkennbarkeit

Jedes Schiff muss auf dem Brett an Umriss und Akzentfarbe erkennbar bleiben: Länge, Breite und Höhe bleiben bei einem
Umbau (±10 %), ebenso die Grundform des Rumpfs, die Anordnung der Hauptmerkmale und die Zahl der Düsen. Die Bastion ist
mit 6,3 Einheiten das breiteste Schiff (im Spiel 2,07 bei Slots im Abstand 2,2); breiter darf kein Flaggschiff werden.
Kennzeichen bleiben und werden eher größer:

| Schiff           | Kennzeichen                                                                                                                  |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| void_interceptor | Nadelnase, runder Rumpf, Klingenflügel mit Vorderkantenstreifen, V-Leitwerk, Gondeln unter den Flügeln                       |
| scrap_raider     | asymmetrisch: Rammsporn, Kanzel steuerbord, schwere Kanonengondel backbord, Stummelflügel, drei ungleiche Düsen, Rostflicken |
| horizon_cruiser  | lang und schlank, flacher Bug mit Turm, Brückenturm achtern, zwei Gondeln an Pylonen, Heckflossen                            |
| plasma_frigate   | gedrungen, Plasmakanone als Bug mit leuchtenden Ringen, vier Gondeln mit Leuchtringen, Reaktorwulst                          |
| titan_carrier    | Flugdeck mit Landebahnlichtern, Insel achtern, Hangars an den Flanken, violetter Reaktorkern im Bug                          |
| drone            | kleiner Keil mit Pfeilflügeln, violettes Sensorauge statt Kanzel, zwei Düsen                                                 |
| repair_drone     | runder Körper mit grünem und orangem Band, gefaltete Arme oben, Schweißarm, Seitengondeln                                    |
| signal_booster   | große Schüssel auf Gabelmast mit Cyan-Rand, Signalring um den Rumpf, Relaismasten, Solarpaneele                              |
| bastion          | fliegende Festung: gewölbte Schildwand am Bug, Terrassendecks, vier Schildpylone, Bergfried, drei schwere Düsen              |
| vanguard         | Rammbug als Speerspitze mit leuchtenden Schneiden, zwei Bugkanonen, gepfeilte Panzerklingen, zwei Türme, Düsencluster        |
| hive             | Zwei-Kammer-Rumpf, leuchtende Brutwaben auf dem Rücken, Hangar-Sponsons mit Drohnen, Sechseckturm                            |
| mirage           | flach, facettiert, dunkler Rumpf mit türkisen Nähten, zwei schräge Projektorflossen, Diamantflügel                           |
| pendulum         | Railgun als Rückgrat mit Spulen weit über den Bug, Gimbal aus Messing-Zahnkranz und Spulenring, Pendel unter dem Kiel        |
| decoy            | Hologramm eines Mirage-Jägers aus Flächen (`hologram`) und leuchtenden Kanten, wenige Dreiecke                               |

## Fallen

- **Dunkle Unterseite in der Karte:** `hero` schaut von schräg unten. Unterseiten liegen dort im Halbschatten oder
  Schatten, dunkle Materialien (`dark`, `PHANTOM`) versinken im dunklen Grund. Unterseite in `hull` oder `armor` halten,
  mit Kiel oder Plattenlage brechen; eigene dunkle Töne brauchen einen nicht zu dunklen Schatten-Ton (wie `PHANTOM`).
- **Kennzeichen oben:** Die Standard-Kartenkamera kommt von schräg unten und versteckt alles, was auf dem Deck liegt.
  Solche Schiffe bekommen in `look.HERO_SHIPS` eine Kartenkamera von vorn oben mit eigenem Licht (Deck hell, Flanke
  halbhell): heute Titan Carrier (Flugdeck, Insel), Hive (Waben), Bastion (Terrassen, Bergfried, Pylone statt nur der
  Bugwand). Die Pendulum nutzt die Standardansicht von schräg unten, passend zu den übrigen Schiffskarten.
- **Weiße Ringflächen:** Runde Materialien mit Glanzpunkt (`steel`, `accent_round`, `glass`, Messing) auf großen ebenen
  Flächen wie Stirnseiten von Rohren oder Ringscheiben überschreiten in `hero` als Ganzes die Glanzschwelle und leuchten
  weiß. Große ebene Flächen bekommen ein Material ohne Glanz (`dark_round`, `hull_round` oder die eckige Variante).
- **Wappen:** Sie müssen in `behind` (zweite Spielkamera, von hinten oben) aufrecht stehen und dürfen von keinem Turm,
  Mast oder Aufbau verdeckt sein; das Oben des Zeichens zeigt deshalb zum Bug. Von der Seite (`board`) sieht man das nicht.
- **Große Lichter:** Große Lichtelemente (Signalring, Leuchtband über die halbe Schiffslänge) dürfen `vf.light(…,
  glow=False)` nehmen: exakt die Farbe, aber ohne Glühen, damit ein kleines Licht der Blickfang bleibt.
- **Gegnerseite:** Gegner stehen um 180° gedreht, man sieht ihre Steuerbordseite; das Licht ist wie im Spiel je Seite
  gespiegelt, kommt also auch bei ihnen von vorn (`enemy`, `enemy_zoom`). Asymmetrische Schiffe dort prüfen.
- **Feste Bauergebnisse:** Ein Bau muss bei jedem Lauf dieselben Bytes liefern. bmesh-Operatoren wie `solidify` ordnen
  ihre Ausgabe je Lauf anders an (`vf.dish` fängt das ab); wer solche Operatoren im Modul nutzt, baut zweimal und vergleicht.
- Die Freestyle-Meldung „strokes set empty“ ist harmlos: Eine Linienart hat in der Ansicht nichts zu zeichnen, etwa die
  Außenkontur eines kleinen Schiffs, das in `behind` ganz vor seinem Pad liegt.

## Regeln, die `build.py` prüft

- `lifted`: Kein Teil unter dem Slot, Blender-z −1,4, bei Flaggschiffen −1,27 (sie werden im Spiel um 1,1 vergrößert).
  Sonst das ganze Schiff mit `s.lift(dz)` anheben.
- Keine schwebenden Teile: Jedes Teil berührt, schneidet oder steckt in einem anderen (`vf.loose_parts`, Lücken unter 0,03
  zählen als Berührung).
- Geschlossene Hüllen: Die Materialien sind einseitig (nur Hologramme beidseitig), jedes Teil muss also geschlossen und
  nach außen orientiert sein, auch an den Wrackstücken (`vf.bad_shells`). Die Primitive aus `vf.py` erfüllen das von
  selbst, gespiegelte Teile auch; Gefahr droht bei eigenen offenen Lofts (`cap_start`/`cap_end`), umgedrehten Flächen
  und negativer Skalierung.
- Budget und Größe stehen in der Ausgabe (Dreiecke, KB, Maße).

## Prüfung im `board`-Render

`renders/<id>/board.png` in voller Größe ansehen, nicht vergrößert, am besten neben der Dreadnought:

1. Umriss und Klasse sind sofort lesbar, die Kennzeichen aus der Tabelle sind zu sehen.
2. Keine Linienklumpen: Wo Konturen zu schwarzen Flecken verschmelzen, ist ein Teil zu klein oder zu dicht.
3. Deck hell, Flanke halbhell, Schatten fallen nach achtern; keine fleckigen Schatten kleiner Teile.
4. Lichter sind Akzente, keine Flächen; der Akzent ist zu sehen, ohne zu dominieren.
5. Danach `zoom` (keine leeren Flächen, keine schwebenden Teile), `behind` (Wappen, Aufbauten), `enemy` (Steuerbordseite)
   und `hero` (Kartenvorlage, Unterseite nicht leer, keine weißen Flächen).
