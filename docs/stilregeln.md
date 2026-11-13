# Stilregeln für Neubauten (dreckiger Look)

Der Look ist „Used Future“: dunkle, kantige, abgenutzte Schiffe. Das Spiel zeichnet matte Töne als Verlauf, kräftige
Verwitterung aus der Maske, ein paar leuchtende Akzente und keine Konturen (`assets/shaders/toon.wesl`). Referenz ist
die Iron Dreadnought (`fleet/iron_dreadnought.py`): große, klare Primärformen, gegliedert durch Sekundär- und
Tertiärformen, die zu Detailzonen mit Zweck gruppiert sind, dazu Kleinteile, die den Maßstab zeigen (Fenster, Flak,
Antennen, Geländer). Die Kleinteile tragen alle Schiffe, die das Spiel lädt (`details` in ihren Modulen), und alle
stehen in der Fassung für die Nahaufnahme: Fasen, glatte Rundteile, Feinteile und Beschriftung (unten, „Nahaufnahme“);
Referenz dafür ist die Bastion (`fleet/bastion.py`). Hive, Mirage, Drohne, Repair Drone, Trugbild und der Entwurf
Broadside (im PvE-Run unbenutzt) stehen noch in der groben Fassung da.

Die Schiffe baut `tools/ships/` in Blender (`just ships`, `just ship-preview`); Dateinamen ohne Verzeichnis auf dieser
Seite (`build.py`, `fleet/<id>.py`, `renders/<id>/`) liegen dort.

Die Vorschau (`preview.py`) rendert noch Clean Toon mit Linien: Sie taugt für Form, Maße und Umriss, den Look zeigt nur
das Spiel (unten, „Prüfung“). Vor einem Umbau die Kontaktbögen ansehen (`preview.py -- --views board,hero,behind
--sheet`).

## Arbeitsweise

- Ein Umbau ändert nur `fleet/<id>.py`. Neue Helfer kommen in dieses Modul; erst wenn ein zweites Schiff sie braucht,
  wandern sie nach `parts.py`. `parts.py`, `vf.py` und `palette.py` ändert nur, wer die gemeinsamen Dateien betreut, und
  vergleicht dabei alle Schiffe vor und nach der Änderung (Dreiecke, Maße, GLB-Bytes bzw. Positionen). Braucht ein Schiff
  eine Variante eines gemeinsamen Helfers, kopiert es ihn ins Modul. Fehlt eine Farbe in `palette.py`, kommt sie vorerst
  als benannte Konstante ins Modul und wird gemeldet.
- `build()` beginnt mit `m = vf.materials(s.name, "<klasse>")` und endet mit `return s.finish()`.
- Gemeinsame Helfer in `parts.py`: Baugruppen `turret`, `engine` (`n` Segmente für kleine Düsen), `badge` (Wappenplatte
  der Flaggschiffe), `trim_loop`, `trim_line`, `on_surface`; Kleinteile `turret_details` und `engine_details` (mit
  denselben Argumenten wie `turret` bzw. `engine` aufrufen), `vent` (Lüftungsgitter), `rcs` (Steuerdüsenblock), `flak`
  (Zwillings-Flak), `rail` (Geländer), `windows` (Fensterreihe aus Einzelfenstern); Formen
  `sym` (symmetrischer Umriss aus einer Hälfte), `stealth` (Tarnkappenprofil), `block` (Facettenblock auf Lagen), `beam`
  (kantige Strebe), `axle` (Walze entlang X), `ring_at` (Querschnitt eines Lofts, für Bänder, die dem Rumpf folgen),
  `bellows` (Blendenbalg um ein Rohr). Fasen setzt `vf.chamfer` (unten, „Nahaufnahme“).
- Vor einem Umbau die jetzige Fassung rendern und die Bilder wegkopieren (die Vorschau überschreibt `renders/<id>/`).
- Kreislauf: `build.py -- <id>` (Dreiecke, KB, Maske, Warnungen) und `preview.py -- <id> --views board,zoom,behind,hero,wreck`,
  bei asymmetrischen Schiffen auch `enemy_zoom`. Fertig ist ein Schiff erst ohne Warnungen, nach dem Blick auf `board`
  in voller Größe und nach dem Blick ins Spiel (unten).

## Was wegfällt

Weg bleibt, was als Geometrie nichts bringt oder nur Rauschen ist:

- Plattenfugen, Nietenreihen, Schmutz, Kantenabrieb und Plattentöne als Geometrie: Sie kommen aus der
  Verwitterungs-Textur (gebackene Maske). Die Geometrie liefert die Kanten und Stufen, an denen sie greift.
  Schraubenköpfe an einer Baugruppe (Lukenrahmen, Turmring, Wappenrand) sind dagegen Feinteile (unten).
- Gerundete Kanten (Bevel mit mehreren Segmenten). Fasen sind erwünscht: `vf.chamfer` an Primär- und Sekundärformen,
  `rbox` an Verschlüssen und Blenden.
- Zufalls-Greebles: Kleinteile ohne erkennbaren Zweck, gestreut statt gruppiert, und gleich große Teile in gleichen
  Abständen über das ganze Schiff (gleichmäßiges Rauschen).
- Nebenwaffen, die mit der Hauptbewaffnung konkurrieren: Flak und Nebentürme bleiben deutlich kleiner als die Waffe, die
  die Klasse zeigt (an der Dreadnought Flak-Gehäuse 0,17 gegen Turmradius 0,5).
- Lichter, die den Blick von der Klasse wegziehen: Fenster und Positionslichter bleiben klein (Fenster 0,07 × 0,045,
  Positionslichter Radius 0,03 bis 0,04). Ein rotes Positionslicht am Brückenflügel der Dreadnought zog in einer
  früheren Fassung auf dem Brett mehr Blick als die Klasse.

Früher fielen auch Fensterreihen aus Einzelfenstern, Nadel- und Schüsselantennen, Geländer, Leitern, Rohrleitungen,
Kühlrippen und alle abstehenden Teile unter 0,15 weg: Das Spiel zeichnete um jedes erhabene Teil einen Konturring, und
ein Bündel davon wurde zum schwarzen Fleck. Der dreckige Look zeichnet keine Konturen; diese Teile sind jetzt erwünscht
(nächster Abschnitt).

Ausnahme bei der Größe: Ist ein Teil das Kennzeichen des Schiffs (die Schüssel des Signal Boosters, die Arme der Repair
Drone), bleibt es groß und grob.

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
- **Kleinteile für den Maßstab.** In den Detailzonen sitzen Teile, die zeigen, wie groß das Schiff ist: Fensterreihen
  aus Einzelfenstern an Aufbauten und Deckswänden, Flak-Türmchen an der Deckkante, Nadel-, Peitschen- und
  Schüsselantennen am Mast und auf Dächern, Geländer und Leitern am Aufbau, Rohrleitungen und Kisten auf Decks,
  Kühlrippen an Düsen und Läufen, Positionslichter an den äußersten Punkten; an Jägern Raketen an Pylonen,
  Flügelspitzen-Schienen, Klappenspalt, Wartungsluken. Sie stehen in einer eigenen Funktion `details` am Ende von
  `build()` (Vorbild: Dreadnought und Void Interceptor). Auch sie brauchen einen Ort mit Zweck und bleiben in Zonen;
  zwischen den Zonen bleiben ruhige Flächen.

Referenz für Kleinteile und Zonen sind die Detailzonen der Iron Dreadnought:

- **Bug:** Bugpanzer-Kappe (`armor`) um die Spitze; Kanonen mit Blendenbalg vor dem Verschluss, Kühlmantel, Laufschelle
  und zweiteiliger Mündungsbremse (Stahlkern im Spalt).
- **Türme:** `parts.turret_details`, also Barbettenring, Entfernungsmesser-Ohren mit Linse, Turmluke, Blendenbälge,
  Mündungsringe. Türme drehen sich im Spiel zum Flaggschiff: Was auf einem Turm sitzt, entsteht im Block
  `with s.turret(at, …)` (`vf.Ship.turret`), sonst bleibt es beim Schwenken stehen.
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

## Nahaufnahme: Fasen, Rundungen, Feinteile

Die Kamera kommt viel näher heran, als das Brett zeigt: Bei größtem Zoom ist eine Blender-Einheit 190 bis 245 px groß
(Full HD), auf Retina und 4K 345 bis 490 px, also das Zehn- bis Zwanzigfache der Brettgröße. Dort tragen drei Dinge,
die auf dem Brett nichts kosten: Fasen an den großen Kanten, glatte Rundteile und eine vierte Stufe von Teilen unter
0,05. Dazu kommen aus dem Shader die Fasen der Plattenkanten und die Feinstruktur (nächster Abschnitt). Die Bastion ist
dafür die Referenz. Wie weit ein Schiff geht, hängt daran, wie nah die Kamera ihm kommt, also an seiner Gruppe auf dem
Brett (`scene/formation.rs`):

| Gruppe                                   | Schiffe                                                    | größter Zoom (Retina) | Feinteile          | Dreiecke | Maske |
| ---------------------------------------- | ---------------------------------------------------------- | --------------------- | ------------------ | -------- | ----- |
| einzeln (Flaggschiffe, Schlachtschiffe)  | Bastion, Vanguard, Pendulum, Pyre, Siegel, Dreadnought, Titan Carrier, Scrap King, Warden | 345–490 px je Einheit | alle, ab 0,012     | bis 40k  | 128   |
| Paar (Kreuzer, Unterstützer), Maßstab 0,72 | Horizon Cruiser, Plasma Frigate, Signal Booster          | 250–350 px            | ab 0,018           | bis 22k  | 96    |
| Staffel (Jäger), Maßstab 0,42            | Void Interceptor, Scrap Raider, Lance Bomber               | 145–205 px            | ab 0,03, sparsam   | bis 10k  | 64    |

Ein Teil unter drei Pixeln bringt nichts: An Jägern entfallen Schraubenköpfe, Treppen und Gitterstäbe, dort tragen
Fasen, glatte Rundteile, Wartungsklappen, Warnbänder und die Kennnummer. Die Staffel steht fünffach auf dem Brett, ihr
Budget zählt also fünfmal.

- **Fasen.** `vf.chamfer(teil, breite)` gibt jeder konvexen harten Kante eine eigene Fläche: Sie bekommt im Verlauf
  einen eigenen Ton und an beiden Rändern Kantenabrieb. An der Bastion 0,05 an Rumpf, Sockel und Aufbau, 0,04 an Decks,
  Schildwand, Zinnen und Schürzen, 0,015 bis 0,03 an Sekundärformen (Pylone, Anschlusskästen, Lukendeckel). Scharf
  bleiben Kleinteile, Kanten unter einer Zierlinie (sie säße sonst auf der Fase) und Teile, deren Fase dicker wäre als
  das halbe Teil.
- **Rundungen.** Große runde Teile (Türme, Kränze, schwere Düsen, runde Rümpfe im Querschnitt) 40 Segmente, mittlere
  (kleine Düsen, Linsen, Luken) 24, Masten 16; an Paaren 32 und 20, an Jägern 24 und 16. Formen, die kantig gemeint sind
  (der achteckige Bergfried, die Pylone), bleiben kantig und werden gefast.
- **Feinteile** (Funktion `fine` am Ende von `build()`, nach `details`): Teile, die zeigen, wie groß das Schiff ist und
  wo man es betritt, wartet und verschraubt. Treppen zwischen den Decks (Stufen 0,026 hoch, also etwa 16 für 0,4 Höhe),
  Laufstege aus Gitterrost, Luken mit Rahmen, Scharnieren und Griff, Schraubenköpfe (Sechskant, Radius 0,013) an
  Rahmen, Ringen und Zinnen, Schießscharten, Gurtringe und Steigeisen an Türmen, Konsolen unter Auskragungen,
  Eckrippen, Fenstersimse, Dachaggregate, Spanten und Rohre an Flanken, Dockluken, Stellstreben an Düsenglocken, Poller.
  Auch sie brauchen einen Ort mit Zweck und stehen in Zonen. Die ruhigen Flächen dazwischen dürfen kleiner werden (etwa
  eine halbe Einheit): In der Nahaufnahme tragen sie die Plattenfasen und die Feinstruktur.
- **Beschriftung als flache Geometrie** (Platten 0,006 hoch, 0,002 eingesenkt): Wegelinien und Kennnummern in `paint`
  (gebrochenes Weiß) auf dunklem Grund, in `dark` auf der Akzentfarbe und auf hellem Stahlblau (dort trägt `paint`
  nicht); Warnbänder aus schrägen Balken (`accent` auf `dark`). Kennnummern in Segmentschrift, unter 0,16 Höhe werden
  sie dünn; an Flanken auf jeder Seite für sich gesetzt und lesbar, auf Decks und Rückwänden so, dass man sie von hinten
  oben liest (wie das Wappen). Jedes Einzelschiff und jedes Paar trägt eine eigene Nummer (Bastion 01, Vanguard 02,
  Pendulum 03, Pyre 04, Warden 09, Dreadnought 17, Titan Carrier 23, Horizon Cruiser 31, Plasma Frigate 42, Signal
  Booster 56), Jäger ein Staffelzeichen aus Streifen, der Scrap King nur die Reste einer fremden Nummer, das Siegel
  keine. Groß bleibt nur, was Kennzeichen ist.
- **Zum Charakter des Schiffs.** Die Feinteile erzählen, was das Schiff ist: an der Pendulum Messingfassungen,
  Rundinstrumente und Wicklungen, an der Pyre Lamellen, Kühlmittelleitungen und Tankarmaturen, am Träger Katapult,
  Fangseile und Zurrpunkte, am Scrap King und am Scrap Raider Heftnähte, Flicken, Ketten und Streben statt Luken und
  Warnbändern. Das Siegel bleibt als Tarnkappenschiff glatt: bündige Klappen, Senkschrauben und Sägezahnnähte, keine
  Fasen an den Facetten.
- **Helfer** in `parts.py`: `bolts` (Schraubenköpfe), `spaced` (Punkte entlang eines Umrisses), `hatch` (Luke),
  `stairs`, `grating` (Gitterrost), `stripes` (Warnband), `number` (Kennnummer), `deckhouse` (gefaster Brückenblock, der
  im Deck steckt), `cover` (gewölbte Wartungsklappe auf einem runden Rumpf), `on_ring` (Ecke eines Querschnitts mit der
  Richtung der Fläche); dazu die Materialien `steel_flat` und `paint` im Standardsatz. `stairs` und `grating` laufen
  längs (y) und sind gespiegelt; wer sie anders braucht, kopiert sie ins Modul (so `gangway` an der Dreadnought,
  `walkway` an der Vanguard, `catwalk` an der Pyre).
- **Maske.** Die Texeldichte je Schiff steht in `look.WEATHER_DENSE` (Tabelle oben).

## Verwitterung

Fugen, Plattenton, Verdeckung, Schmutz, Ruß und Kantenabrieb kommen für alle Schiffe gleich aus `weather.py` (nach dem
Bau, vor dem Wrack; Kanäle, Ablauf und Cache im Modulkommentar), nicht aus dem Schiffsmodul. Auf dem Brett bleiben davon
Plattenton, Verdeckung, Schmutz und Ruß, im Zoom und auf der Karte kommen Fugen, die Fasen der Plattenkanten, der Abrieb
und die Feinstruktur dazu.

- Verdeckung: Wo Nachbarteile Licht wegnehmen (Reichweite 0,4), dunkelt die Maske die Fläche ab (`look.OCCLUSION`). So
  sitzt ein Kleinteil mit einem Schatten auf seiner Fläche, und Ritzen, Stufen und Nischen bekommen Tiefe, ohne dass
  das Spiel Schatten rechnet.
- Plattenfasen: Im Spiel neigt sich die Normale am Rand jeder Platte zur Fuge hin (`BEVEL` in `toon.wesl`); die Platten
  lesen als Lagen, eine Kante im Licht, eine im Schatten. Die Geometrie braucht dafür nichts.
- Feinstruktur: `detail.py` schreibt eine kachelbare Textur für alle Rümpfe (Kratzer, Poren, Beulen, Flecken, Korn),
  die das Spiel aus den drei Achsen des Schiffs projiziert. Sie ist feiner als jede Maske (512 Texel je Einheit) und
  hängt nicht an der Abwicklung; ein Schiff muss nichts dafür tun.
- Schmutz sammelt sich in Ritzen (wo Teile aufeinandertreffen), läuft als Schlieren hinter allem nach achtern, was
  mindestens 0,06 hoch ist, und Ruß sitzt an Düsen (`core`) und Mündungen: kreisrunde Deckel ab acht Ecken an runden
  Materialien, waagrecht und vorn frei (wie `parts.turret`). Eine Waffe ohne solchen Deckel bekommt keinen Ruß.
- Abrieb sitzt an sichtbaren konvexen Kanten ab 40° lackierter Flächen, Platten liegen an den Schiffsachsen.
- Grad je Schiff in `look.WEATHER` (Schrott 2,2, Mirage 0,5, Trugbild 0), Anteile je Material in
  `look.WEATHER_MATERIALS`. Ein neuer Materialschlüssel gilt als lackiert (eckig mit Platten, rund ohne); unlackierte
  Sonderfarben (Metall, Glas, Rost) dort eintragen.
- Die erste Vorschau eines geänderten Schiffs backt seine Maske (Jäger etwa 15 s, Paare 1 min, Einzelschiffe 3 bis
  4 min, die Bastion 10 min), danach kommt sie aus dem Cache. Für Probeläufe setzt `VF_DENSITY=32` die Dichte aller
  Schiffe herab (die Bastion backt dann in 45 s). Bilder über 3072 px backt `weather.py` in Kacheln, damit der Speicher
  nicht mit der Bildgröße wächst (Einzelschiffe 2 bis 3 GB, die Bastion 5 GB).
- Dichte der Maske nach der Gruppe (`look.WEATHER_DENSE`, Tabelle oben): 128 Texel je Einheit für Einzelschiffe, 96 für
  Paare, sonst 64 (`look.WEATHER_DENSITY`). Das Spiel liest Schmutz als Verlauf; bei 32 wirkte er in der Nahaufnahme
  weich. Die Masken sind 864 px (Lance Bomber) bis 6048 px (Bastion) groß; mehr als 8192 px darf keine werden (Grenze
  vieler Grafikkarten).
- Die Maske liegt als `<id>_weather.ktx2` neben dem GLB, komprimiert (BC7, ein Byte je Texel statt vier) und mit
  Mip-Kette; Schiff und Wrack nennen sie im Vertrag (`mask`) und teilen das eine Bild. Zusammen belegen die Masken
  etwa 238 MB Grafikspeicher: die Bastion 49 MB, die übrigen Einzelschiffe 11 bis 23 MB, ein Paar 4 bis 5 MB, ein
  Jäger gut 1 MB. `build.py` ruft dafür den Encoder `ctt` (`cargo install ctt-cli --locked --version 0.6.1`).

## Größen auf dem Brett

Auf dem Brett ist eine Blender-Einheit etwa 22 px (Flaggschiffe 24 px), im Zoom etwa 70 px, auf der Karte 80–90 px;
Jäger stehen als Staffel im Maßstab 0,42, also bei etwa 9 px je Einheit. Konturen zeichnet das Spiel nicht mehr, Teile
dürfen deshalb klein sein und dicht stehen. Was man wo sieht (geprüft an Dreadnought und Void Interceptor, in
Standbildern):

- Primär- und Sekundärformen (ab etwa 0,15) lesen auf dem Brett als Form.
- Fenster und Positionslichter lesen auf dem Brett als Lichtpunkte, auch bei 0,03 bis 0,07, weil sie leuchten.
- Flak (Gehäuse 0,17, Rohre Radius 0,02) und Antennen (Radius 0,012 bis 0,014) lesen an Schlachtschiffen auf dem Brett
  als kleine Formen und Striche; Geländer, Leitern und Rohrleitungen (0,014 bis 0,03) erst in der Nahaufnahme.
- An Jägern in der Staffel sieht man auf dem Brett keines der Kleinteile. Sie stören dort nicht und zahlen sich aus,
  wenn die Kamera herangeht.
- Feinteile (Schrauben, Stufen, Gitterstäbe, Beschriftung) sieht man erst im Zoom; auf dem Brett liest von ihnen nur,
  was eine eigene Farbe hat (Warnbänder als Strich).
- Ob dünne Teile in Bewegung flimmern, ist ungeprüft. Flimmert eines, wird es dicker oder fällt weg.
- Neue Kleinteile einmal im Spiel ansehen: Dev-Szenen in `crates/client/src/dev/scenes.rs` (`hit` zeigt den Belagerer,
  also die Dreadnought, `board` eine Staffel Void Interceptor), Screenshot mit `VOIDFLEET_SMOKE_FRAMES` und
  `VOIDFLEET_SMOKE_SCREENSHOT`.

## Palette und Akzent

`vf.materials` liefert alles; Farben stehen zentral in `palette.py` und kommen nie als Hex in ein Schiffsmodul. Die
Werte der Palette und der Look des ganzen Spiels stehen im [Grafikstil](grafikstil.md); hier steht, welches Material
wohin gehört.

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

Rund oder eckig entscheidet das Material: eckig (`hull`, `armor`, `dark`, `accent`, `trim`) gibt einen Ton je Facette,
rund (`hull_round`, `armor_round`, `dark_round`, `accent_round`, `steel`, `glass`) weiche Verläufe (in der Vorschau
Bänder); Kanten über 38° bleiben immer hart, eine Fase (`vf.chamfer`) ist deshalb eine eigene Fläche mit eigenem Ton.
Rohre, Türme, Barbetten, Masten, Düsen und runde Rümpfe also rund, Kisten, Platten, Keil- und Facettenrümpfe eckig. Nie
einen runden Rumpf eckig (Facettenstreifen) und nie einen Keilrumpf rund (Verläufe quer über die Facetten).

## Dreiecksbudget

Obergrenzen nach der Gruppe (Tabelle im Abschnitt „Nahaufnahme“): Einzelschiffe 40k, Paare 22k, Jäger 10k; Schiffe in
der groben Fassung Drohne 3k, Trugbild 1,5k (bleibt flach). Stand vom 13.11.2026:

| Schiff           | Dreiecke vorher | nachher | Wrack | Maske   |
| ---------------- | --------------- | ------- | ----- | ------- |
| bastion          | 10,6k           | 33,5k   | 40,7k | 6048 px |
| pendulum         | 10,7k           | 34,4k   | 40,6k | 4096 px |
| iron_dreadnought | 14,7k           | 33,8k   | 41,7k | 4096 px |
| vanguard         | 11,3k           | 31,4k   | 38,4k | 4064 px |
| pyre             | 8,7k            | 28,1k   | 35,0k | 4128 px |
| scrap_king       | 12,1k           | 28,1k   | 35,1k | 3872 px |
| warden           | 6,3k            | 25,4k   | 30,8k | 3616 px |
| titan_carrier    | 6,8k            | 16,9k   | 22,3k | 4064 px |
| seal             | 3,5k            | 8,2k    | 10,7k | 2880 px |
| plasma_frigate   | 7,5k            | 18,9k   | 24,0k | 2016 px |
| horizon_cruiser  | 7,2k            | 17,9k   | 21,0k | 1792 px |
| signal_booster   | 5,0k            | 15,1k   | 18,8k | 1856 px |
| void_interceptor | 5,9k            | 9,2k    | 11,7k | 1056 px |
| scrap_raider     | 4,6k            | 6,1k    | 8,0k  | 992 px  |
| lance_bomber     | 3,4k            | 6,0k    | 7,4k  | 864 px  |

Wohin die Dreiecke gehen, an der Bastion: 6,1k für glatte Rundteile und die Emitterknöpfe der Schildwand, 2,8k für
Fasen, 12,2k für Feinteile und Beschriftung (davon etwa 4k Schraubenköpfe zu je 20 Dreiecken), 1,8k für die feineren
gemeinsamen Baugruppen (Flak, Geländer, Steuerdüsen). Das meiste geht in runde Teile und in Mengen kleiner Teile
(Schrauben, Fenster, Gitterstäbe); Kasten, Platten und Stege kosten einzeln fast nichts. Die Wracks (wreck.py) entstehen
automatisch und haben etwa das 1,2- bis 1,3-Fache; ansehen (`wreck`) muss man sie trotzdem. Die schwerste Szene (sechs
Staffeln Void Interceptor und ein Flaggschiff) hat gut 300k Dreiecke.

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
| broadside        | olivgrau, langer Pfeilrumpf mit stumpfem Bug, Grat, Plateau mit Pfeilspitze, acht flache Zwillingstürme, Gürtel, Achterburg  |

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
- **Flächen in einer Ebene:** Zwei Teile, deren Flächen deckungsgleich übereinanderliegen (sich überlappende flache
  Platten einer Beschriftung), flimmern: Jede trägt ihre eigene Verwitterung. Segmente einer Ziffer stoßen deshalb
  aneinander, statt sich zu überdecken; Flaches liegt immer ein Stück höher als seine Fläche.
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

## Prüfung

Zuerst die Form im `board`-Render: `renders/<id>/board.png` in voller Größe ansehen, nicht vergrößert, am besten neben
der Dreadnought. Die Vorschau zeigt noch Clean Toon mit Linien; Linienklumpen an Kleinteilen sind dort kein Befund mehr,
das Spiel zeichnet keine Linien.

1. Umriss und Klasse sind sofort lesbar, die Kennzeichen aus der Tabelle sind zu sehen.
2. Deck hell, Flanke halbhell, Schatten fallen nach achtern; keine fleckigen Schatten kleiner Teile.
3. Lichter sind Akzente, keine Flächen; der Akzent ist zu sehen, ohne zu dominieren.
4. Danach `zoom` (keine leeren Flächen, keine schwebenden Teile), `behind` (Wappen, Aufbauten), `enemy` (Steuerbordseite)
   und `hero` (Kartenvorlage, Unterseite nicht leer, keine weißen Flächen).

Dann der Look im Spiel (Dev-Szene mit dem Schiff, Screenshot auf Brettgröße und herangezoomt):

5. Die Kleinteile lesen in der Nahaufnahme, auf dem Brett stört keines.
6. Schmutz und Fugen gliedern die ruhigen Flächen, ohne die Akzentfarbe zu verschlucken.
7. Schiffe mit Feinteilen bei größtem Zoom ansehen, in beiden Ansichten: Den Kamerastand trägt `settings.json`
   (`camera.view` 0 von der Seite, 1 von hinten oben; `camera.zoom` der Versatz aus der Ruhelage), mit
   `XDG_CONFIG_HOME` auf einen eigenen Ordner gelegt. Keine leere Fläche über etwa einer halben Einheit, kein Teil, das
   schwebt oder dessen Zweck man nicht erkennt, Beschriftung lesbar.
