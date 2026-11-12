# Grafikstil

Diese Seite legt fest, wie Voidfleet aussieht: Leitbild, Farbpalette, Licht, Effekte, Oberfläche und Bilder. Sie ist
die Quelle für den Look. Wer den Stil ändern will, ändert zuerst diese Seite und danach die Stellen im Code, die sie
unter [Wo was steht](#wo-was-steht) nennt; eine Farbe oder ein Lichtwert, der hier nicht steht, gehört nicht ins Spiel.
Wie ein einzelnes Schiff gebaut wird (Formen, Kleinteile, Verwitterung, Budget), steht in den
[Stilregeln der Schiffe](stilregeln.md).

## Leitbild

„Used Future“: dunkle, kantige, abgenutzte Kriegsschiffe vor einem ruhigen, fast schwarzen Weltraum, geführt über das
Steuerpult einer Brücke. Der Ton ist rau und militärisch ([Vision](vision.md)), der Look trägt ihn mit.

Fünf Grundsätze gelten überall, in der Szene, in der Oberfläche und in den Bildern:

1. **Der Grund ist dunkel und kühl.** Weltraum, Rümpfe und Pulte liegen in entsättigtem Blaugrau und Tintenblau.
   Helle oder bunte Flächen gibt es nicht; ein neues Element beginnt dunkel.
2. **Farbe ist Bedeutung.** Eine kräftige Farbe sagt immer etwas: wem etwas gehört (Admiral, Gegner), was geschieht
   (Schaden, Heilung, Schild, Bonus, Malus) oder was wählbar ist. Dieselbe Bedeutung hat überall dieselbe Farbe, in
   Schwebezahl, Balken, Banner, Knopf und Effekt. Schmuckfarben ohne Bedeutung gibt es nicht.
3. **Leuchten ist knapp.** Nur Lichter, Glut und Effekte leuchten, und nur sie glühen. Ein Licht ist ein kleiner
   Akzent, keine Fläche; je weniger leuchtet, desto mehr zählt das einzelne Licht.
4. **Matt und ohne Linien.** Flächen der Schiffe sind matt, mit weichem Verlauf über drei Töne, Schmutz und Abrieb.
   Konturen zeichnet die Szene nicht; die einzige Linie ist die rote Kontur am Ziel unter dem Fadenkreuz.
5. **Kantig.** Formen sind eckig und technisch: Facetten, Platten, Klammern, kleine Eckradien. Nichts ist rund und
   weich, außer es ist ein Rohr, ein Turm oder eine Düse.

## Farbpalette

Farben stehen an genau zwei Stellen im Code und nirgends sonst als Zahl: `crates/client/src/palette.rs` (alles mit
Bedeutung, für Szene und Oberfläche) und `tools/ships/palette.py` (Töne und Lichter der Schiffsmodelle). Die
Flächentöne der Oberfläche stehen in `crates/client/src/ui/theme.rs`. Alle Werte sind sRGB.

### Grund und Flächen

| Farbe                                                | Wert      | Name               | Wofür                                              |
| ---------------------------------------------------- | --------- | ------------------ | -------------------------------------------------- |
| <span style="color:#04050A">■</span> Weltraum        | `#04050A` | `BACKGROUND`       | Grund hinter allem                                 |
| <span style="color:#171D2B">■</span> Tintenblau      | `#171D2B` | Bilder             | Grundton der Hintergründe und Illustrationen       |
| <span style="color:#040609">■</span> Tinte           | `#040609` | `theme::INK`       | dunkelste Kante: Ränder der Pulte, Fugen           |
| <span style="color:#0B0E13">■</span> Schatten        | `#0B0E13` | `theme::SHADE`     | Grund unter Stapeln und Zählern                    |
| <span style="color:#1C2028">■</span> Feld            | `#1C2028` | `theme::PANEL`     | Felder und Banner, 95 % deckend                    |
| <span style="color:#282F3C">■</span> Pult            | `#282F3C` → `#11141B` | `theme::console` | Metallgehäuse der Dialoge, Verlauf von oben |
| <span style="color:#F2F7FF">■</span> Text            | `#F2F7FF` | `TEXT`             | Text der Oberfläche                                |
| <span style="color:#C7DBFA">■</span> Text gedämpft   | `#C7DBFA` | `TEXT_DIM`         | Nebentext, 92 % deckend                            |

### Bedeutungsfarben

| Farbe                                               | Wert      | Name               | Bedeutung                                            |
| --------------------------------------------------- | --------- | ------------------ | ---------------------------------------------------- |
| <span style="color:#78C2FF">■</span> Hellblau       | `#78C2FF` | `PLAYER`, `SHIELD` | eigene Seite, Akzent der Oberfläche, Schild          |
| <span style="color:#FF806B">■</span> Koralle        | `#FF806B` | `ENEMY`            | Gegnerseite: Schüsse, Banner, Lichter der Gegner     |
| <span style="color:#73EBFF">■</span> Cyan           | `#73EBFF` | `SELECT`           | Auswahl, gültige Ziele, Hauptaktion                  |
| <span style="color:#FF7059">■</span> Rot            | `#FF7059` | `DAMAGE`           | Schaden: Schwebezahl, Puls, Verlust im Balken        |
| <span style="color:#78ED8C">■</span> Grün           | `#78ED8C` | `HEAL`             | Reparatur                                            |
| <span style="color:#FFD666">■</span> Gold           | `#FFD666` | `BUFF`, `WON`      | Bonus, Sieg                                          |
| <span style="color:#C796FF">■</span> Violett        | `#C796FF` | `DEBUFF`           | Malus, Störung                                       |
| <span style="color:#FF614F">■</span> Warnrot        | `#FF614F` | `LOST`             | Niederlage, Gefahr-Knöpfe                            |
| <span style="color:#B0BDCF">■</span> Grau           | `#B0BDCF` | `DRAW`             | neutraler Ausgang                                    |
| <span style="color:#4CDB61">■</span> <span style="color:#F2BD33">■</span> <span style="color:#ED382E">■</span> Hülle | `#4CDB61` `#F2BD33` `#ED382E` | `HP_FULL`, `HP_HALF`, `HP_EMPTY` | Hüllenbalken von voll bis leer |
| <span style="color:#F4D68C">■</span> Zier           | `#F4D68C` | `FLAGSHIP_TRIM`    | Zierton aller Flaggschiffe: Zierlinie, Kartenrahmen, Sterne |

Schaden ist rot und warm, Schild und alles Eigene blau, Auswahl cyan. Rot und Koralle liegen nah beieinander und
bleiben getrennt: Koralle heißt „gehört dem Gegner“, Rot heißt „Schaden“.

### Admirale

Die Farbe eines Admirals trägt alles, was ihm gehört: Auswahlkarte, Banner, Schüsse seines Flaggschiffs, dessen
Lichter, Düsen und Kennzeichen, sein Untergang. Das Porträt trägt sie als einzige Akzentfarbe.

| Farbe                                         | Wert      | Admiral | Akzentfläche am Flaggschiff (Licht-Ton) |
| --------------------------------------------- | --------- | ------- | --------------------------------------- |
| <span style="color:#E8A33A">■</span> Bernstein | `#E8A33A` | Osei    | `#C1955E`                               |
| <span style="color:#E2483D">■</span> Rot       | `#E2483D` | Vance   | `#C25E53`                               |
| <span style="color:#5B8DFF">■</span> Blau      | `#5B8DFF` | Kerr    | `#7290D6`                               |
| <span style="color:#FF7A2A">■</span> Glutorange | `#FF7A2A` | Reyes   | `#CB713F`                               |

Die volle Farbe gibt es am Modell nur als Licht und Kennzeichen; Flächen tragen den gedämpften Ton gleichen Farbtons
(`ADMIRAL_ACCENT`). Ein neuer Admiral braucht einen Farbton, der von den vier vorhandenen, von Koralle (Gegner) und von
Cyan (Auswahl) klar zu unterscheiden ist.

### Gegnerklassen und Kartentypen

| Klasse         | UI-Farbe  | Akzentfläche am Modell | Lichter am Modell |
| -------------- | --------- | ---------------------- | ----------------- |
| Jäger          | <span style="color:#E64C33">■</span> `#E64C33` | Koralle `#D5725A` | Orange `#FF8A3D` |
| Kreuzer        | <span style="color:#4C80E6">■</span> `#4C80E6` | Kornblume `#7398DC` | Blau `#5CB2FF` |
| Schlachtschiff | <span style="color:#808099">■</span> `#808099` | Stahlblau `#8BA7C6` | Kühlblau `#5CC4FF` |
| Unterstützer   | (keine)   | Mint `#61C28E`, heller Rumpf `#A3AFBE` | Grün `#4DFFAA` |
| Flaggschiff    | <span style="color:#F4D68C">■</span> `#F4D68C` | Farbton des Admirals | Farbe des Admirals |

Im Gefecht tragen die Gegner mit den gemeinsamen Rümpfen die Lackierung der Gegnerseite: Rumpftöne bleiben, ihre
Lichter rücken zum Farbton von Koralle (`LIVERY_ACCENT` 0,7). Bosse und Flaggschiffe behalten ihre eigenen Farben.

| Kartentyp | Farbe |
| --------- | ----- |
| Angriff   | <span style="color:#E63333">■</span> `#E63333` |
| Bonus     | <span style="color:#33B24C">■</span> `#33B24C` |
| Malus     | <span style="color:#9933CC">■</span> `#9933CC` |
| Taktik    | <span style="color:#3399E6">■</span> `#3399E6` |
| Doktrin   | <span style="color:#E6B21A">■</span> `#E6B21A` |

Die Illustration einer Karte ist im Farbton ihres Typs gehalten.

### Schiffe

Jede beleuchtete Fläche hat drei Töne: Licht, Halbschatten, Schatten. Die beiden dunkleren leitet `tones()` aus dem
Licht-Ton ab (Farbton bleibt, Helligkeit und Chroma fallen); von Hand gesetzt sind nur Töne, die abgeleitet ins Oliv
kippen (Gold, Messing).

| Tonfamilie                                      | Licht     | Halbschatten | Schatten  | Wofür                                    |
| ----------------------------------------------- | --------- | ------------ | --------- | ---------------------------------------- |
| <span style="color:#56657A">■</span> `HULL`     | `#56657A` | `#3A4657`    | `#222A36` | Rumpf, große Flächen                     |
| <span style="color:#5F6F84">■</span> `ARMOR`    | `#5F6F84` | `#414E60`    | `#262F3C` | aufgesetzte Lagen, einen Hauch heller    |
| <span style="color:#323A47">■</span> `DARK`     | `#323A47` | `#252C37`    | `#161B23` | Gehäuse, Barbetten, Verschlüsse, Kiel    |
| <span style="color:#6D7C8F">■</span> `STEEL`    | `#6D7C8F` | `#4A5667`    | `#2B3340` | Läufe, Düsenglocken (mit Glanz)          |
| <span style="color:#2F5F7A">■</span> `GLASS`    | `#2F5F7A` | `#17445D`    | `#092A3C` | Kanzeln (mit Glanz)                      |
| <span style="color:#F4D68C">■</span> `TRIM`     | `#F4D68C` | `#C9A05A`    | `#7F5A2C` | Zierlinie der Flaggschiffe               |
| <span style="color:#3B3532">■</span> `WRECK`    | `#3B3532` | `#2C2624`    | `#1C1816` | verkohlte Bruchflächen                   |

Lichter haben genau eine Farbe: Positions- und Zierlichter `#5CC4FF`, Brennkammern `#C8F0FF`, Fenster `#9FE0FF`,
Schildemitter `#FF3FA6`, Plasma `#B45CFF`, Positionslichter rot `#FF5040` und grün `#50FF7A`, Glut der Bruchflächen
`#FF9945`. Welches Material wohin gehört und wie viel Akzent ein Schiff trägt, regeln die
[Stilregeln](stilregeln.md).

### Regeln für Farben

- Keine Farbe als Zahl außerhalb von `palette.rs`, `palette.py` und `theme.rs`. Wer eine Abstufung braucht, leitet sie
  ab: `theme::alpha` (Deckkraft), `palette::darkened`, `palette::lightened`, `fx::mix`; am Modell `tones()` und `pale()`.
- Eine neue Farbe braucht eine Bedeutung, die noch keine hat, bekommt dort einen Namen und eine Zeile auf dieser Seite.
- `palette.py` und `palette.rs` nennen dieselben Werte für Admirale und Zierton; wer einen ändert, ändert beide und
  baut die betroffenen Schiffe neu (`just ships`).
- Kräftige Farben stehen auf dunklem Grund. Text auf Bildern bekommt einen Schatten (`theme::text_shadow`).

## Licht und Kamera

Alles Licht der Schiffe ist gerechnet, nicht beleuchtet: Der Shader wählt aus den drei Tönen, die Szene hat keine
Lichtquellen und keine Schlagschatten.

- **Hauptlicht.** Von vorn oben, leicht von der Kameraseite: Deck hell, Flanke halbhell, Schatten nach achtern. Es
  steht fest am Brett und ist je Seite gespiegelt, auch Gegner sind also von vorn beleuchtet (`KEY` in
  `scene/toon/look.rs`, dasselbe in `tools/ships/look.py`).
- **Verlauf.** Matter, weicher Übergang zwischen den drei Tönen (Halb-Lambert), insgesamt auf 0,8 abgedunkelt
  (`SHADE_MID`, `SHADE_LIT`, `EXPOSURE` in `assets/shaders/toon.wesl`); die Töne sind im Spiel um 1,4 gespreizt
  (`TONE_SPREAD`), damit Deck, Flanke und Plattenlagen auf dem kleinen Brett auseinandergehen.
- **Kantenlicht statt Kontur.** Am Umriss ein kühles Blau `#86ACD2`, höchstens 30 % (`RIM_COLOR`, `RIM`); Glanz
  `#E4ECF4` nur auf Metall, Glas und abgeschabten Kanten.
- **Verwitterung.** Plattenton, Fugen, Schmutz, Ruß und Kantenabrieb aus der gebackenen Maske, Schmutz in warmem
  Graubraun; Fugen und Abrieb erscheinen erst in der Nahaufnahme.
- **Keine Tonkurve.** Die Kamera rechnet in HDR und gibt die Farben so aus, wie sie gerechnet sind
  (`Tonemapping::Linear`): Eine Farbe aus der Palette erscheint auf dem Schirm als diese Farbe.
- **Glühen.** Was heller als 1 ist, glüht im Bloom der Kamera (Schwelle 1,0, Stärke 0,35), sonst nichts. Lichter der
  Schiffe leuchten mit 1,8 (`GLOW_BOOST`), Effekte mit 2,2 (`fx::GLOW`, über `fx::glowing`). Große Lichtflächen
  glühen nicht (`glow=False` am Modell), damit kleine Lichter der Blickfang bleiben.

## Effekte und Animation

Effekte, Partikel, Marken, Balken und Pads sind selbstleuchtend und unbeleuchtet (`FxMaterial`,
`assets/shaders/fx.wesl`): Licht mischt additiv, Rauch nach Deckkraft.

- **Wem gehört der Schuss.** Schüsse des Flaggschiffs leuchten in der Farbe des Admirals, Schüsse der Gegner in
  Koralle. Die Waffe bestimmt die Form (Bolzen, Railgun-Strahl, Plasma, Rakete mit Leuchtspur), nicht die Farbe.
- **Wirkung in Bedeutungsfarbe.** Pulse, Schwebezahlen und Marken einer Karte tragen die Farbe ihrer Wirkung (Schaden,
  Heilung, Schild, Bonus, Malus), nicht eine eigene.
- **Feuer ist warm, Sprung ist kühl.** Einschläge, Brände, Explosionen und Glut laufen von Weißgelb über Orange nach
  dunklem Rot (Mündungsblitz `#FFCC80`, Blitzlicht der Explosionen orange, Glut `#FF9945`, erkaltend dunkelrot).
  Hyperraum und Ankunft sind kühles Blau (`#73BFFF`). Rauch ist zuerst vom Feuer angeleuchtet, dann grau und
  verweht; Energie blitzt weiß auf und verblasst in der Farbe des Schusses.
- **Fünf Einschläge.** Funken, Feuerball, Entladung, Durchschlag, Brand (`HitVariant`); eine neue Waffe wählt eine
  davon, statt eine sechste zu erfinden.
- **Zerstörung in drei Akten.** Todeskampf, Detonation, Wrack mit glühenden, abkühlenden Bruchstücken. Je größer das
  Schiff, desto größer die Detonation; Flaggschiffe gehen in der Farbe ihres Admirals unter.
- **Bewegung hat Gewicht.** Schiffe fliegen nur vorwärts, in Kurven und Schleifen mit Schräglage; Schlachtschiffe
  und Flaggschiffe gleiten behäbig (`scene/maneuver.rs`). Ein Treffer ist ein kurzer Blitz am Modell, am Flaggschiff
  mit Wackeln. Jede Animation ist ein Auftrag mit Ende und folgt der eingestellten Animationsgeschwindigkeit;
  überspringen beendet sie sofort ([Der Gefechtsbericht](inszenierung.md)).
- **Brett.** Pads sind dunkles Blau (`#2B548A`), gültige Ziele leuchten in der Auswahlfarbe, der Zeiger hellt auf
  (`#D1FAFF`). Über dem Schiff stehen Hüllen- und Schildbalken auf dunkler Spur (`#212638`).

Neue Effekte, Marken und Modelle gehören in die Generalprobe (`scene/rehearsal.rs`); ansehen lassen sie sich in den
Dev-Szenen (`just scene effects`, `hit`, `destroy`).

## Oberfläche

Die Oberfläche ist die Brücke eines Kriegsschiffs: Geräte aus dunklem Metall und getöntem Glas, die leuchten, wo man
etwas tun kann.

- **Pult.** Dialoge sind Steuerpulte (`theme::console`): Metallgehäuse mit Verlauf, Rahmen aus Ringen, Schrauben in
  den Ecken, Kopfleiste mit Kontrolllampe, eingelassene Felder (`console_inset`). Ein neuer Dialog baut darauf auf.
- **Knöpfe.** Kantige Fläche aus getöntem Glas mit feinem Rahmen und vier leuchtenden Eckklammern (`ui/button.rs`).
  Vier Varianten, die nur Farben setzen: gewöhnlich (Hellblau), Hauptaktion (Cyan), Gefahr (Warnrot), getönt in einer
  Akzentfarbe (etwa der des Admirals).
- **Karten.** Eine Befehlskarte ist ein Datapad: Illustration als Screen im Farbton des Typs, Kosten oben links,
  Tastenleiste unten (`ui/card.rs`). Gewählt heißt Rahmen in der Auswahlfarbe, nicht bezahlbar heißt gedimmt.
- **Schrift.** Fließtext in Exo 2; Titel, Knöpfe und Zahlen in Rajdhani (600, fett 700), groß geschrieben und um
  0,08 em gesperrt wie in einem HUD (`theme::caps`). Sonderzeichen aus Noto Sans Symbols. Weitere Schriften gibt es nicht.
- **Ecken.** Radien bleiben klein: 3 bis 4 px an Feldern und Knöpfen, 6 px am Pult, höchstens 8 px. Rund sind nur
  Lampen, Schrauben und Zähler.
- **Icons.** Weiße Strich-Icons im Raster 24 × 24 mit Strichstärke 2 und runden Enden (Lucide), gerastert mit
  `tools/rasterize` nach `assets/images/ui/` und im Spiel getönt. Ein neues Icon kommt aus demselben Satz oder ist in
  derselben Strichart gezeichnet.
- **Zustände.** Hover hellt auf, Drücken blitzt, gesperrt ist blass (`UiOpacity`), gewählt leuchtet in Cyan. Schein
  und Ringe kommen aus `theme::glow` und `theme::ring`.

## Bilder

Karten und Porträts sind Illustrationen, keine Renderings: flächige Cel-Schattierung mit dunkler Tintenlinie, viel
ruhiger tintenblauer Grund mit wenigen Sternen und genau eine Akzentfarbe, die leuchtet (die Farbe des Kartentyps
bzw. des Admirals). Das Motiv steht groß und allein im Bild, ohne Schrift und ohne Rahmen.

- **Format.** 736 × 1024, JPEG, nach `assets/images/cards/` und `assets/images/admirals/`.
- **Herstellung.** Über fal.ai, immer mit einem vorhandenen Bild oder einem Modell-Render (`hero`) als Stilvorlage,
  nie nur aus einem Text. Ein Schiff auf einer Karte zeigt das Modell, wie es im Spiel aussieht.
- **Hintergründe.** 2048 × 1152, PNG, nach `assets/images/backgrounds/`. Sie sind ruhige, ferne Kulissen: Grundton
  Tintenblau `#171D2B`, wenig Kontrast, wenige Elemente, alles weit weg, Mitte und unterer Bereich am dunkelsten, die
  Ränder ruhig (das Bild wird beschnitten und verschoben). Der gemeinsame Auftrag und der je Szene stehen in
  `prompts.json` daneben; ein neuer Hintergrund benutzt denselben Grundauftrag.
- **Zwei Register.** Die Szene ist matt, verwittert und ohne Linien, die Illustrationen sind flächig und haben
  Linien. Was sie zusammenhält, sind der dunkle, kühle Grund und die Bedeutungsfarben. Ein Bild, das eines von beiden
  verlässt (heller Grund, zweite Akzentfarbe, Fotorealismus), passt nicht.

## Wo was steht

| Was                                              | Wo                                                                   |
| ------------------------------------------------ | -------------------------------------------------------------------- |
| Bedeutungsfarben, Admirale, Klassen, Kartentypen | `crates/client/src/palette.rs`                                       |
| Töne und Lichter der Schiffe                     | `tools/ships/palette.py`                                             |
| Flächen, Schrift-Helfer, Pult der Oberfläche     | `crates/client/src/ui/theme.rs`, Knöpfe `ui/button.rs`, Karte `ui/card.rs` |
| Verlauf, Kantenlicht, Glanz, Verwitterung        | `assets/shaders/toon.wesl`                                           |
| Hauptlicht, Glühen der Lichter, Lackierung       | `crates/client/src/scene/toon/look.rs`                               |
| Bloom, Tonkurve                                  | `crates/client/src/scene/camera.rs`                                  |
| Effekte, Partikel, Zerstörung                    | `scene/fx.rs`, `effects.rs`, `particles.rs`, `destruction.rs`, `assets/shaders/fx.wesl` |
| Bau, Verwitterungsgrad, Vorschau der Schiffe     | `tools/ships/` (`look.py`, `weather.py`, `preview.py`)               |
| Aufträge der Hintergründe                        | `assets/images/backgrounds/prompts.json`                             |

Werte, die an zwei Stellen stehen und zusammenpassen müssen: Admiralsfarben und Zierton (`palette.rs` und
`palette.py`), das Hauptlicht (`look.rs` und `look.py`), die Schwellen der Verwitterung und `FINE_LOD` (`toon.wesl`
und `look.py`).

## Stil ändern

1. Die Änderung auf dieser Seite festhalten (Wert, Bedeutung, Regel); betrifft sie den Bau der Schiffe, auch in den
   [Stilregeln](stilregeln.md).
2. Die Werte an den Stellen aus der Tabelle ändern, Paare gemeinsam.
3. Bei Farben oder Tönen der Schiffe die Modelle neu bauen (`just ships`).
4. Ansehen: `just scene board`, `just scene effects`, `just scene report`, die Modellschau, und die Bildschirme um
   den Run (`VOIDFLEET_DEV_SCREEN`). Ein Stilwechsel gilt erst, wenn Brett, Nahaufnahme, Karte und Dialog zusammen
   stimmen.

## Prüfliste für Neues

- Beginnt es dunkel und kühl, und trägt jede kräftige Farbe eine Bedeutung aus der Palette?
- Kommt jede Farbe aus `palette.rs`, `palette.py` oder `theme.rs`, abgeleitet statt neu gemischt?
- Leuchtet nur, was Licht, Glut oder Effekt ist, und bleibt das Licht klein?
- Ist es kantig, matt und ohne Kontur (Szene) bzw. flächig mit einer Akzentfarbe (Bild)?
- Benutzt es Pult, Knopf, Karte, Schriften und Icons, die es schon gibt?
- Liest man es auf Brettgröße, und stört es dort nichts Wichtigeres?

## Offen

Stellen, an denen das Spiel dieser Seite noch nicht folgt; die Punkte zu Vorschau, Kartenbildern der Flaggschiffe und
den Namen `toon` stehen in `TODO.md`.

- **Farben außerhalb der Palette.** Einzelne Flächentöne stehen als Zahl in den Dialogen (vor allem
  `settings_dialog.rs`, dazu `admiral_select.rs`, `rules.rs`, `star_map.rs`, `card.rs`), Effektfarben in
  `destruction.rs`, `effects.rs`, `motion.rs`, `board.rs` und `status.rs`.
- **Hintergründe.** Ihr Grundauftrag in `prompts.json` beschreibt noch den früheren Clean-Toon-Look mit bunten
  Schiffen davor; die Bilder selbst passen, der Text gehört beim nächsten neuen Hintergrund angepasst.
- **Illustrationen mit Linien.** Karten und Porträts stammen aus der Zeit des Clean-Toon-Looks. Ob sie die Linien
  behalten (zwei Register, wie oben beschrieben) oder dem dreckigen Look folgen, ist nicht entschieden.
- **Tonspreizung.** `TONE_SPREAD` spreizt die Töne nur im Client; bewährt sie sich, wandert sie nach `tones()` in
  `palette.py`.
