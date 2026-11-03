# Regelentwurf v2: Voidfleet als PvE-Run, ein Flaggschiff gegen die Flotte

> Stand 03.11.2026, fünfter Schnitt: feste Zahlen statt Genauigkeit und Zufall, denn die Rechnung gegen die Ansage braucht exakte Werte; seit dem vierten Schnitt Züge im Wechsel statt gleichzeitiger Auflösung, denn gegen Muster-Gegner gibt es nichts mehr zu raten. Ergebnis der MDA-Session und der Steam-Recherche (`reports/Steam PvE oder PvP Deckbuilder.md`): Voidfleet wird ein PvE-Spiel: ein Einzelspieler-Run im Stil von Slay the Spire, ein Mensch gegen Gegner, die das Spiel nach festen Mustern steuert. Der Spieler führt genau ein Schiff, sein Flaggschiff, gegen Verbände, Schlachtschiffe und Bosse; ein zweiter Mensch sitzt nicht am Brett. Das bisherige Gameplay (Schiffsdeck, zwei Flotten, Spalten- und Schiffsfeuer, Grundmanöver, Overdrive, Drohnen und Trugbild) wird ersetzt; die Zwischenfassung mit vier Einheiten je Seite ist ebenfalls überholt. Brett, 3D-Szene, Schiffsmodelle, Porträts und Bordfunk bleiben nutzbar. Alle Zahlen sind Startwerte für den Prototyp, nicht abgestimmt. Koop und das asynchrone Duell folgen später auf denselben Gefechtsregeln. `GAMEPLAY_IDEEN.md` beschreibt die frühere, auf Yomi ausgerichtete Richtung und ist überholt.

Wer ist wer in diesem Text:

- **Der Spieler** ist ein Mensch. Er führt genau eine Einheit, sein Flaggschiff, besetzt dessen Stationen und spielt Karten.
- **Das Flaggschiff** ohne Zusatz ist immer das Schiff des Spielers. Ein gegnerisches Flaggschiff gibt es nur in Boss-Gefechten.
- **Die Gegner** sind alle Einheiten auf der anderen Seite. Sie steuert das Spiel nach festen Mustern mit angesagten Absichten (Abschnitt 12): kein zweiter Mensch, kein planender Bot. Sie spielen keine Karten und haben keine Stationen.
- **Die Runde** ist Spielerzug, Gegnerzug, Bilanz (Abschnitt 5). Im Spielerzug handelt nur der Spieler, im Gegnerzug nur das Spiel.

## 1. Was das Spiel sein soll

Nach dem MDA-Framework, von hinten nach vorn:

- **Ästhetik.** Kommandanten-Fantasie und Schauwert. Der Moment, für den man wiederkommt, ist „Was für eine Schlacht“: ein Schiff, eine Crew, eine Übermacht. Ton wie Battlestar Galactica und The Expanse: rau, militärisch, Schadensmeldungen von der Brücke, Verluste und Schaden tun weh. Der Ton kommt aus Funk, Text, Zuständen und Inszenierung, nicht aus dem Look; der Clean-Toon-Stil bleibt und wird detailreicher und schmutziger. Nicht im Zentrum: Mindgame; die offene Rechnung ist Mittel, nicht Zweck.
- **Dynamik.** Eines gegen viele, Mensch gegen Spiel, in Zügen im Wechsel. Die gegnerische Flotte steht in Formation und sagt an, was sie in ihrem Zug tun wird; der Spieler handelt zuerst, rechnet gegen die Ansage und entscheidet, welche Station seines Schiffs in dieser Runde besetzt wird: Geschütz, Schild, Werft oder Admiral; dann führt das Spiel aus, was von der Ansage noch steht. Der Spannungsbogen kommt aus Verfall: Gegner brennen, fallen in den Notfall, verlieren sichtbar Modelle, und das Flaggschiff trägt seine Hülle durch den ganzen Run. Kein Schuss geht ins Nichts, und jede Zahl steht fest, bevor der Spieler klickt. Jede Runde stellt die Frage: Wo gebe ich Feuerkraft auf, um zu überleben, und wofür gebe ich meine Befehlspunkte aus?
- **Mechanik.** Das Flaggschiff besetzt pro Runde eine Station. Befehlspunkte zahlen Karten aus einem Deck, das im Run wächst. Vier Admirale als Charaktere, jeder mit Waffe, Passiv, Admiralsstation und Startdeck. Gegner haben Muster und Absichten statt Karten. Drei Sektoren mit Sternenkarte, Beute und Bossen. Die Auflösung ist ein erzählter Gefechtsbericht. Fällt das Flaggschiff, endet der Run.

## 2. Brett und Formation

- **Gegnerseite**, vom Spiel gestellt. Zwei Reihen, **Front** und **Back**, mit je drei **Spalten**, höchstens vier Einheiten. Im Code heißen die Reihen bisher `strike` und `command`; die Umbenennung in `front` und `back` ist Teil der Umsetzung.
- **Deckung.** Eine gegnerische Einheit in der Back-Reihe ist gedeckt, wenn in ihrer Spalte vorn eine Einheit steht. Von jedem Schaden auf ein gedecktes Ziel fängt der Vordermann die aufgerundete Hälfte ab, das Ziel bekommt den Rest. Eine Jäger-Staffel deckt wie jede andere Einheit: Sie ist dann der Schirm, der zerrieben wird.
- **Spielerseite.** Drei Spalten, das Flaggschiff des Spielers steht in einer davon, sonst nichts. Die Back-Reihe bleibt leer; im Koop steht dort später das zweite Flaggschiff.
- **Zwei Arten gegnerischen Feuers.** **Zielfeuer** folgt dem Flaggschiff, wohin es auch fliegt; die Antwort sind Schilde und Karten. **Spaltenfeuer** sind die schweren Waffen: Sie werden mit Spalte angesagt und treffen, was im Gegnerzug dort steht; die Antwort ist Manöver oder Kurzsprung. Das Flaggschiff feuert immer auf ein gewähltes Schiff (Waffen-Station, Abschnitt 4); Spaltenfeuer gibt es nur auf der Gegnerseite.

## 3. Einheiten

### Klassen

| Klasse         | Seite                        | Darstellung                                                                 | Rolle                                                  |
| -------------- | ---------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------ |
| Jäger-Staffel  | nur Gegner                   | 5 kleine Modelle, die mit der Hülle verschwinden (je 20 % ein Modell)       | schwach, aber viele; weicht aus, deckt, rammt          |
| Kreuzer-Paar   | nur Gegner                   | 2 mittlere Modelle; unter 50 % brennt eines, im Notfall ist eines ein Wrack | das Rückgrat jedes Verbands, Feuerleitung, Plasma      |
| Schlachtschiff | nur Gegner                   | 1 großes Modell mit Bränden und Rumpfbrüchen                                | Spaltenfeuer mit Aufladen, Panzerung, ruft Verstärkung |
| Flaggschiff    | Spieler; Gegner nur als Boss | wie Schlachtschiff, größer, mit Ring und Krone                              | das Schiff des Spielers, der Held des Runs             |

Jede Einheit hat **Hülle** und **ATK**. Schilde regenerieren nicht und stehen nicht auf den Karten; Schutz entsteht durch Stationen, Deckung und Karten.

### Schaden

Jeder Schuss trifft, und jede Zahl steht fest: Ein Schuss macht so viel Schaden wie die ATK des Schützen, ohne Zufall und ohne Trefferwahrscheinlichkeit. Was die Ansage zeigt, geschieht im Gegnerzug; was die Vorschau am Ziel zeigt, macht der Schuss des Spielers. Die Varianz des Runs liegt in der Hand, in der Beute und in den Mustern der Gegner (Abschnitt 12), nicht im Schuss.

Vom Schaden eines Schusses wird in dieser Reihenfolge abgezogen:

1. **Ausweichen** des Ziels: halber Schaden. Nur Gegner weichen aus; die Railgun und Karten mit „Ausweichen zählt nicht“ übergehen es.
2. **Panzerung** des Ziels: ein fester Abzug je Treffer, nie unter 0.
3. **Deckung**: Der Vordermann fängt die Hälfte ab, das Ziel bekommt den Rest (Abschnitt 2).
4. **Schilde** des Ziels schlucken, was übrig ist, bis sie aufgebraucht sind.

Alle Werte sind ganze Zahlen; wo gehälftelt wird, wird aufgerundet. Spaltenfeuer und Rammkurs treffen voll oder gar nicht; Ausweichen zählt für sie nicht.

### Zustände

Zustände sind sichtbar. Sie sind der Motor der Eskalation und ersetzen den Overdrive.

| Zustand                                       | Auslöser                                                                                                                                                  | Wirkung                                                                                   | Ende                                                                                    |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| **Brand** (Jäger: „Zersprengt“)               | ein Treffer bringt die Hülle auf 50 % oder weniger; beim Flaggschiff schon ab 60 %, wenn ein Plasmakreuzer trifft; bei Gegnern bei jedem Treffer der Glut | in der Bilanz −2 Hülle (Gegner der Glut −3)                                               | beim Flaggschiff Schadenskontrolle oder Reparaturtrupp; ein Gegner brennt, bis er fällt |
| **Notfall** (Flaggschiff: „Brücke getroffen“) | Hülle bei 25 % oder weniger                                                                                                                               | ATK halbiert. Gegner: kein Ausweichen. Flaggschiff: keine Admiralsstation                 | Hülle wieder über 25 %                                                                  |
| **Zerstört**                                  | Hülle 0                                                                                                                                                   | Gegner: verlässt das Brett, der Slot ist frei für Verstärkung. Flaggschiff: der Run endet | —                                                                                       |

Ein Brand kann in der Bilanz zerstören: „Ausgebrannt.“

### Das Flaggschiff im Run

Die Hülle des Flaggschiffs bleibt zwischen den Gefechten, wie sie ist; geheilt wird in der Werft und durch Ereignisse. Ein Brand erlischt nach dem Gefecht, der Notfall bleibt, bis die Hülle wieder über 25 % liegt. Das Flaggschiff ist der Held des Runs: Jeder Punkt Hülle ist eine Entscheidung, und ein Gefecht knapp zu gewinnen ist nicht dasselbe wie es sauber zu gewinnen.

## 4. Stationen des Flaggschiffs

Stationen hat nur das Flaggschiff; was die Gegner tun, sind angesagte Aktionen (Abschnitt 12). Das Flaggschiff besetzt pro Runde genau eine Station. Station, Ziel und Spalte bleiben eingestellt, bis der Spieler sie ändert; eine Runde kann deshalb einen Klick brauchen oder acht. Die Station wird ausgeführt, sobald der Spieler sie auslöst, spätestens beim Zugende; davor und danach dürfen Karten gespielt werden.

| Station                                     | Wirkung                                                                                                                 |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Waffen**, feuert auf ein gewähltes Schiff | beim Auslösen die ATK des Flaggschiffs auf das Ziel, mit den Abzügen aus Abschnitt 3                                    |
| **Schilde**, feuert nicht                   | schluckt im folgenden Gegnerzug 8 Schaden (Mauer 12), Zielfeuer wie Spaltenfeuer; was übrig ist, verfällt in der Bilanz |
| **Schadenskontrolle**, feuert nicht         | löscht den Brand, +5 Hülle, nie über die volle Hülle                                                                    |
| **Manöver**, feuert nicht                   | wechselt die Spalte und entgeht so dem Spaltenfeuer, das auf die alte Spalte angesagt ist; Zielfeuer folgt              |
| **Admiralsstation**                         | je Admiral, Abschnitt 7; im Notfall nicht möglich                                                                       |

Regeln im Detail:

- Eine Waffen-Station ohne Ziel feuert nicht. Es gibt keine automatische Zielwahl. Stirbt das Ziel, bleibt die Station auf Waffen, das Ziel ist leer und wird in der Checkliste angemahnt.
- Schilde schlucken allen Schaden, der im Gegnerzug auf das Flaggschiff gerichtet ist, Karten-Schilde (Notschilde, Standhalten, Alarmbereitschaft) kommen dazu.
- Das Flaggschiff weicht nicht aus; Panzerung hat es nur durch Ablativpanzerung und Module. Gegen gegnerisches Feuer hat der Spieler vier Antworten: Schilde gegen den Schaden, Manöver gegen Spaltenfeuer, Kurzsprung gegen alles, und den Schützen zerstören, bevor er handelt.
- Die Karte Alarmstufe Rot erlaubt zwei Stationen in einer Runde; das ist der einzige Weg zu zwei Stationen.

## 5. Die Runde: Spielerzug, Gegnerzug, Bilanz

Mensch und Spiel ziehen im Wechsel. Eine Runde besteht aus dem Spielerzug, in dem nur der Spieler handelt, dem Gegnerzug, in dem nur die Gegner handeln, und der Bilanz, die das Spiel von selbst ausführt. Vor Runde 1 sieht der Spieler Formation und Absichten der Gegner und wählt die Spalte seines Flaggschiffs.

| Abschnitt      | Wer handelt                | Was geschieht                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Spielerzug** | der Spieler                | Karten und Station wirken sofort, in der Reihenfolge des Spielers (Abschnitt 6). Gegner mit Hülle 0 fallen auf der Stelle und handeln in dieser Runde nicht mehr.                                                                                                                                                                                                                                              |
| **Gegnerzug**  | das Spiel                  | Angesagte Verstärkung trifft ein und sagt ihre erste Absicht an. Dann handelt jede gegnerische Einheit nach ihrer Absicht, Front vor Back, links nach rechts: Spaltenfeuer, Zielfeuer, Rammkurs, Aufladen, Schilde, Ausweichen, Zielerfassung, Verstärkung rufen. Schilde des Flaggschiffs, Gegenfeuer, Standhalten und Gegenmaßnahmen werden je Treffer gerechnet. Fällt das Flaggschiff, endet der Run hier. |
| **Bilanz**     | niemand, das Spiel rechnet | Brände ticken auf beiden Seiten. Zustände werden neu bestimmt und ausgerufen. Schilde des Flaggschiffs verfallen, der Kurzsprung kehrt zurück. Die Hand wird abgeworfen und fünf Befehle nachgezogen, Befehlspunkte auf 3, die Station bleibt eingestellt. Die Gegner sagen ihre Absichten für die nächste Runde an (Abschnitt 12), und ab Runde 8 tickt die Gefechtsuhr.                                      |

Weil der Spieler zuerst handelt, feuert ein Gegner, der im Spielerzug fällt, nie: Das ist die wichtigste Regel des Runs. Jeder Angriff ist eine Rechnung gegen die Ansage: Was fällt, bevor es handelt, hat nie gehandelt; was in den Notfall geht, handelt mit halber ATK. Umgekehrt gibt es keinen Schutz, den man nicht vor dem Zugende aufgebaut hat.

## 6. Der Spielerzug

Im Spielerzug wirkt alles sofort, in der Reihenfolge, in der der Spieler es tut, wie in Slay the Spire:

1. **Karten.** Beliebig viele Befehlskarten, solange die **Befehlspunkte** reichen. Jede Runde 3 Punkte, sie verfallen am Zugende. Jede Karte wirkt beim Spielen; wer erst den Torpedo einschlagen sieht, wählt danach das Ziel seiner Station.
2. **Station.** Eine der fünf Stationen, bei Waffen ein Ziel, bei Manöver eine Spalte; voreingestellt ist, was in der Vorrunde galt. Die Station wird ausgeführt, sobald der Spieler sie auslöst, spätestens beim Zugende. Danach dürfen noch Karten gespielt werden.
3. **Zug beenden.** Der Gegnerzug beginnt.

Die Hand wird jede Runde neu gezogen: fünf Befehle vom Deck, nicht gespielte Karten gehen in der Bilanz auf den Ablagestapel, und ist das Deck leer, wird der Ablagestapel gemischt. So zählt jede Karte im Deck, und der Deckbau im Run hat eine Wirkung.

## 7. Die vier Admirale

Ein Admiral ist der Charakter, den der Spieler zu Beginn des Runs wählt, wie in Slay the Spire: Flaggschiff mit Hülle, ATK und Waffe, ein **Passiv**, eine **Admiralsstation**, die das Flaggschiff statt Waffen, Schilde, Schadenskontrolle oder Manöver besetzen kann, und ein Startdeck mit zwei **Kennkarten** (Abschnitt 8). Dass die Admiralsstation mit den Waffen oder dem Schutz des eigenen Schiffs konkurriert, ist die Entscheidung jedes Admirals in jeder Runde.

| Admiral             | Flaggschiff | Hülle | ATK | Waffe                                               | Passiv                                                                                                       | Admiralsstation | Kennkarte   |
| ------------------- | ----------- | ----- | --- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | --------------- | ----------- |
| Osei, die Mauer     | Bastion     | 70    | 7   | Flakbatterien                                       | Wächterschild: Schilde schlucken 12 statt 8                                                                  | Gegenfeuer      | Standhalten |
| Vance, der Hammer   | Vanguard    | 50    | 11  | Schwere Geschütze                                   | Vorstoß: +1 ATK je Runde in Folge mit Waffen oder Fokusfeuer, höchstens +4; jede andere Station setzt zurück | Fokusfeuer      | Überladung  |
| Kerr, der Uhrmacher | Pendulum    | 56    | 8   | Railgun: ignoriert Deckung und Ausweichen           | Uhrwerk: sieht die Absichten der Gegner zwei Runden voraus                                                   | Stillstand      | Kurzsprung  |
| Reyes, die Glut     | Pyre        | 56    | 9   | Plasmawerfer: jeder Treffer setzt das Ziel in Brand | Glutkern: Brände bei Gegnern ticken 3 statt 2                                                                | Nachsetzen      | Brandsatz   |

**Osei, die Mauer.** Admiralsstation **Gegenfeuer**: Die Bastion feuert nicht, schluckt 6, und jeder Schütze, der sie im Gegnerzug trifft, verliert 3 Hülle. Gegen einen Verband, der mit allem auf sie feuert, ist das die beste Salve, die sie hat, und sie kostet keinen Schuss, den sie nicht ohnehin verliert. Ihr Dilemma: Gegenfeuer lebt davon, getroffen zu werden, und die Hülle ist die Währung des Runs. Spielt sich wie ein Panzer, der die Flotte an sich zerschellen lässt; schwach gegen Belagerer, deren Panzerung ihre 7 ATK auf 5 drückt, und gegen Spaltenfeuer, das sie lieber umfliegt.

**Vance, der Hammer.** Admiralsstation **Fokusfeuer**: Die Vanguard feuert auf ein Ziel mit doppeltem Vorstoß, Ausweichen zählt nicht; dafür nimmt sie im folgenden Gegnerzug je Treffer 2 mehr. Der Hammer will nichts als feuern: Jede Runde Waffen macht ihn stärker, jede Runde Schilde setzt ihn zurück. Sein Dilemma ist die Schwelle, ab der er nicht mehr ausweichen darf, weil der Vorstoß zu wertvoll ist. Seine Schwäche ist die dünne Hülle: Wer ihn an die Schilde zwingt, setzt den Vorstoß zurück.

**Kerr, der Uhrmacher.** Admiralsstation **Stillstand**: Die Pendulum feuert nicht. Eine gewählte gegnerische Einheit handelt in dieser Runde nicht; ihre Absicht verschiebt sich auf die nächste Runde, eine Aufladung bleibt erhalten. Mit dem Uhrwerk sieht Kerr zwei Runden voraus, und mit der Railgun trifft er, wen er will, durch Deckung und Ausweichen hindurch. Sein Dilemma: Stillstand verschiebt die Gefahr nur, und jede Runde Stillstand ist eine Runde ohne Railgun, während die Gefechtsuhr läuft. Der Uhrmacher ist der Admiral, der das Hauptgeschütz um eine Runde verschiebt, in der seine Angriffskarten den Belagerer fällen.

**Reyes, die Glut.** Admiralsstation **Nachsetzen**: Die Pyre feuert nicht. Jeder brennende Gegner nimmt sofort das Doppelte seines Brandschadens, und Angriffskarten auf brennende Ziele machen in dieser Runde +2 Schaden. Mit dem Plasmawerfer brennt alles, was sie trifft; drei brennende Gegner und Nachsetzen sind 18 Schaden ohne einen Schuss. Ihr Dilemma: Brände legt nur ihr Plasmawerfer, ein Ziel je Runde, und Nachsetzen braucht eine Runde ohne Waffen. Gegen einzelne Schlachtschiffe ist sie die Schwächste, gegen Flotten die Beste.

Was die vier unterscheidet, in einem Satz: Die Mauer gewinnt, indem sie aushält, der Hammer, indem er nicht aufhört, der Uhrmacher, indem er die Uhr anhält, die Glut, indem sie wartet.

## 8. Das Befehlsdeck

Karten spielt nur der Spieler; die Gegner haben keine. Kosten in Befehlspunkten. Angriffskarten treffen immer, mit festem Schaden und den Abzügen eines Schusses (Abschnitt 3), wo nichts anderes steht. Portiert aus den heutigen 32 Befehlen, Bewegungskarten, Drohnen und alles, was eine zweite eigene Einheit braucht, entfallen.

| Karte             | BP  | Typ         | Wirkung                                                                                                                                                                         |
| ----------------- | --- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Torpedo           | 2   | Angriff     | 6 Schaden auf ein gewähltes Schiff, Ausweichen zählt nicht                                                                                                                      |
| Orbitalschlag     | 3   | Angriff     | 7 Schaden auf ein gewähltes Schiff, ignoriert Deckung                                                                                                                           |
| Salvenfeuer       | 2   | Angriff     | 2 Schaden auf jede gegnerische Einheit, ignoriert Deckung                                                                                                                       |
| Sperrfeuer        | 1   | Angriff     | 4 Schaden auf jedes gegnerische Schiff, das gerade ausweicht                                                                                                                    |
| Wärmesucher       | 2   | Angriff     | 5 Schaden auf ein gewähltes Schiff, ignoriert Deckung; 8, wenn es gerade ausweicht                                                                                              |
| Gnadenstoß        | 2   | Angriff     | 3 Schaden auf ein gewähltes Schiff; 8, wenn es im Notfall ist                                                                                                                   |
| Bombardement      | 2   | Angriff     | 3 Schaden auf jede Einheit in der gegnerischen Back-Reihe                                                                                                                       |
| Rammkurs          | 3   | Angriff     | Das Flaggschiff rammt ein gewähltes Schiff: Das Ziel nimmt das Doppelte der ATK des Flaggschiffs, das Flaggschiff die ATK des Ziels                                             |
| Brandsatz         | 1   | Angriff     | 2 Schaden auf ein gewähltes Schiff und setzt es in Brand. Kennkarte der Glut                                                                                                    |
| Notschilde        | 1   | Verstärkung | Das Flaggschiff schluckt im Gegnerzug 5 zusätzlich                                                                                                                              |
| Standhalten       | 1   | Verstärkung | Das Flaggschiff schluckt im Gegnerzug 5 zusätzlich; jeder Schütze, der es trifft, verliert 2 Hülle. Kennkarte der Mauer                                                         |
| Ablativpanzerung  | 2   | Verstärkung | Das Flaggschiff nimmt für den Rest des Gefechts je Treffer 1 weniger                                                                                                            |
| Überladung        | 1   | Verstärkung | Das Flaggschiff feuert diese Runde mit +5 ATK und nimmt am Zugende 3 Schaden. Kennkarte des Hammers                                                                             |
| Energiestoß       | 2   | Verstärkung | Das Flaggschiff feuert diese Runde mit +3 ATK                                                                                                                                   |
| Reparaturtrupp    | 1   | Verstärkung | Brand gelöscht, +4 Hülle, zusätzlich zur Station                                                                                                                                |
| EMP-Stoß          | 2   | Schwächung  | Ein gegnerisches Schiff: Seine Schilde fallen aus, eine Aufladung bricht ab                                                                                                     |
| Zielstörer        | 1   | Schwächung  | Alle gegnerischen Schüsse im Gegnerzug −1 ATK                                                                                                                                   |
| Funkstille        | 2   | Schwächung  | Die Sonderaktion eines gegnerischen Schiffs (Aufladen, Zielerfassung, Verstärkung rufen, Riegel) fällt diese Runde aus, und es feuert auch nicht                                |
| Traktorstrahl     | 2   | Schwächung  | Ein gegnerisches Schiff außer einem Flaggschiff wird in einen freien Slot seiner Reihe gezogen; Deckung bricht                                                                  |
| Kurzsprung        | 2   | Taktik      | Das Flaggschiff verlässt das Brett: Es führt diese Runde keine Station aus, ist im Gegnerzug kein Ziel und kehrt in der Bilanz in seine Spalte zurück. Kennkarte des Uhrmachers |
| Ausweichmanöver   | 1   | Taktik      | Das Flaggschiff wechselt sofort die Spalte, ohne seine Station zu verlieren                                                                                                     |
| Gegenmaßnahmen    | 1   | Taktik      | Der erste gegnerische Angriff im Gegnerzug, Spaltenfeuer oder Schuss, macht halben Schaden                                                                                      |
| Plündern          | 1   | Taktik      | Zwei Befehle ziehen                                                                                                                                                             |
| Alarmstufe Rot    | 2   | Taktik      | Das Flaggschiff besetzt diese Runde zwei Stationen; die Admiralsstation darf eine davon sein                                                                                    |
| Feuerleitdoktrin  | 2   | Doktrin     | Für den Rest des Gefechts alle eigenen Schüsse und Angriffskarten +1 Schaden                                                                                                    |
| Alarmbereitschaft | 2   | Doktrin     | Für den Rest des Gefechts schluckt das Flaggschiff in jedem Gegnerzug 2                                                                                                         |

Entfallen gegenüber dem heutigen Spiel: Flankenmanöver, Abfangen, Rückruf, Durchschlag, Kreuzfeuer, Minenfeld, Drohnenstaffel, Feuerschutz, Freie Hand, Schrottverwertung, Schildgitter, Ablenkung, Schnellverlegung. Doktrinen sind die Power-Karten von Slay the Spire: Sie bleiben liegen, bis das Gefecht endet.

Jeder Admiral beginnt mit einem **Startdeck** von zehn Karten: dreimal Torpedo, dreimal Notschilde, Reparaturtrupp, Plündern und zweimal seine Kennkarte. Alle übrigen Karten kommen als Beute; Dubletten sind erlaubt. Jede Karte hat eine **aufgewertete Fassung** für die Werft: Angriffskarten +2 Schaden, Schild- und Reparaturwerte +2, alle anderen kosten einen Befehlspunkt weniger, mindestens 0.

Für den Prototyp teilen sich die Admirale diesen Pool. Das Ziel für die Veröffentlichung sind eigene Pools wie in Slay the Spire: 15 bis 20 Karten je Admiral, die nur in seinem Run als Beute erscheinen (Flak und Vergeltung für die Mauer, Durchschlag und Sturmlauf für den Hammer, Vorhersage und Zeitfenster für den Uhrmacher, Brandbeschleuniger und Flächenbrand für die Glut), über einem kleineren gemeinsamen Sockel.

## 9. Der Gefechtsbericht

Im Spielerzug zeigt das Spiel jede Karte und die Station sofort: Salve, Einschlag, Zahl, dann ist der Spieler wieder dran. Der Gegnerzug wird als Erzählung gezeigt: ein Ereignis nach dem anderen, Kamera und Funk führen, Nebensächliches wird gebündelt. Der Spieler kann jederzeit überspringen.

- **Ein Ereignis, ein Bild.** Je Ereignis fährt die Kamera zum Handelnden, dann zum Ziel. Beim Feuer: Schütze, Salve, Einschlag, Zahl. Gebündelt werden nur gleichartige Dinge ohne Entscheidungswert: Verstärkung fliegt zusammen ein, drei Staffeln auf Ausweichen schwärmen zusammen aus.
- **Reihenfolge im Gegnerzug.** Verstärkung zuerst, dann die Gegner Front vor Back, links nach rechts. Wer im Spielerzug gefallen ist, hat keinen Auftritt mehr, und der Bericht sagt es: „Der Wachkreuzer bricht auseinander, bevor er feuern kann.“
- **Jede Zahl erklärt sich.** Am Einschlag steht der Schaden, darunter in Kurzform, wie er entstand: „11“, dazu Marken wie „weicht aus: 6“, „Panzerung −2“, „gedeckt: 4 an den Wachkreuzer“, „Schild schluckt 8“. Dieselbe Rechnung steht als Vorschau am Ziel, bevor der Spieler auslöst: Was die Vorschau zeigt, geschieht. Das Log führt dieselben Zeilen in Klartext.
- **Zustände sind Auftritte.** Ein neuer Brand, ein Notfall, ein ausgebranntes Schiff bekommen Banner, Funkspruch und Kamerablick. Beim Flaggschiff spricht die Brücke, beim Gegner der abgehörte Funk.
- **Rundenbilanz.** Nach dem Gegnerzug ein kurzes Tableau: Schaden genommen und ausgeteilt, Verluste des Gegners, neue Zustände, die Absichten der Gegner für die nächste Runde, die neue Hand. Erst dann beginnt die Planung.
- **Funk.** Jeder Zug hat Ankündigung und Reaktion. Die eigene Brücke meldet Schäden, Brände und den Notfall; die Geschützleitung meldet Treffer; der abgehörte Gegnerfunk verrät Angst, Verstärkung und den Moment, in dem ein Verband bricht.

## 10. Beispielrunde

Sektor 1, Elite-Knoten, Runde 3. Ich spiele den Hammer. Platzhalterwerte aus den Abschnitten 3, 7 und 12.

Vanguard 38/50, Spalte Mitte, Vorstoß +2 nach zwei Runden an der Waffen-Station, also ATK 13. Hand: Torpedo, Torpedo, Notschilde, Überladung, Plündern. 3 Befehlspunkte.

Gegner: Belagerer 22/30, Front Mitte, hat in Runde 2 aufgeladen und sagt an: Hauptgeschütz, Spaltenfeuer 16 auf Spalte Mitte. Plünderer-Staffel 8/8, Back links, ungedeckt, sagt an: Zielfeuer 2 auf die Vanguard.

Der sichere Zug: Torpedo auf den Belagerer (6, Panzerung −2, also 4, er steht bei 18), Notschilde gegen die Plünderer, Manöver nach links. Das Hauptgeschütz geht ins Leere, die Vanguard bleibt bei 38, aber der Vorstoß fällt auf null, und der Belagerer geht auf Schilde, lädt wieder und sagt in zwei Runden das nächste Hauptgeschütz an.

Der Zug des Hammers: Überladung, Torpedo, dann Fokusfeuer auf den Belagerer. Die Rechnung vor dem Klick, und die Vorschau zeigt sie: Fokusfeuer mit doppeltem Vorstoß macht ATK 15, mit Überladung 20, minus 2 Panzerung sind 18. Der Belagerer steht nach dem Torpedo bei 18. Das reicht auf den Punkt: Er fällt, bevor er feuert, und die Vanguard zahlt dafür 3 Hülle an die Überladung und 4 an die Plünderer. Stünde er bei 23, fehlte ein Punkt, und auch das stünde vor dem Klick in der Vorschau; dann bliebe der sichere Zug.

| Zug     | Ereignis                                     | Ergebnis                                                                                      |
| ------- | -------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Spieler | Überladung                                   | ATK diese Runde +5                                                                            |
| Spieler | Torpedo auf den Belagerer                    | 6, Panzerung −2: 4, Belagerer 18/30                                                           |
| Spieler | Fokusfeuer, 11 + 4 Vorstoß + 5 Überladung    | 20, Panzerung −2: 18, Belagerer zerstört, das Hauptgeschütz feuert nie                        |
| Spieler | Zugende: Überladung                          | Vanguard 35/50                                                                                |
| Gegner  | Plünderer auf die Vanguard, 2, Fokusfeuer +2 | 4, Vanguard 31/50                                                                             |
| Bilanz  |                                              | Belagerer gefallen, Modul gesichert. Vanguard 31/50, Vorstoß +3. Plünderer sagen Zielfeuer an |

Was der Bericht erzählt: Der Torpedo hat die Panzerung aufgerissen, das Fokusfeuer hat den Belagerer zerlegt, und sein Hauptgeschütz, zwei Runden lang angekündigt, ist nie losgegangen. Der sichere Zug hätte 38 Hülle behalten und dafür zwei weitere Runden Belagerer eingehandelt. Beides ist richtig; was richtiger war, entscheidet die Karte dahinter.

## 11. Der Run

Der erste Modus: ein Mensch gegen das Spiel. Vorbild ist Slay the Spire: ein Held, ein wachsendes Deck, eine Karte mit Abzweigungen, ein Boss am Ende jedes Akts, und nach dem Tod beginnt alles von vorn.

### Die Geschichte

Während einer gewaltigen Schlacht reißt ein versehentlich geöffnetes Wurmloch das Flaggschiff des Spielers aus der Formation und wirft es an einen fremden Ort im Universum. Die Flotte ist weg, die Karten sind nutzlos, der Funk schweigt. Der Weg nach Hause führt durch drei Sektoren, und jeder, der dort fliegt, will das fremde Schiff: Plünderer, die es ausschlachten wollen, Wachflotten, die es für einen Angreifer halten, und am Ende eine Kommandantin, die das Wurmloch nach Hause bewacht.

Der Held ist das Flaggschiff. Der Spieler wählt zu Beginn einen der vier Admirale (Abschnitt 7), so wie man in Slay the Spire den Charakter wählt. Es gibt keine Eskorten und keine zweite eigene Einheit: Was der Spieler gewinnt, sind Karten, Module und Schrott, und was er verliert, ist Hülle.

### Die Sternenkarte

Auf dem Holodeck der Brücke liegt der Sektor als Netz von Knoten, von unten nach oben, mit Abzweigungen; der Spieler sieht den ganzen Sektor und wählt an jeder Verzweigung. Drei Sektoren mit je neun Reihen, die letzte ist der Boss. Ein Pfad durch einen Sektor hat etwa fünf Gefechte, ein Run damit rund fünfzehn, und das Ziel sind 60 bis 90 Minuten. Das ist zu messen, nicht zu setzen.

| Knoten       | Anteil      | Was dort geschieht                                                                                                         |
| ------------ | ----------- | -------------------------------------------------------------------------------------------------------------------------- |
| Gefecht      | ~50 %       | Ein Verband des Sektors, von der einzelnen Staffel bis zur Flotte. Beute: Schrott und eine Befehlskarte aus drei.          |
| Elite        | ~12 %       | Ein mächtiges Schlachtschiff, oft mit Schirm, der Zwischenboss des Sektors. Beute: Schrott und ein Modul aus drei.         |
| Ereignis     | ~20 %       | Eine Szene über Funk mit zwei oder drei Entscheidungen.                                                                    |
| Werft        | ~12 %       | Eines von dreien: Reparieren (+30 % Hülle), Aufwerten (eine Karte), Ausmustern (eine Karte aus dem Deck entfernen).        |
| Schwarzmarkt | ~6 %        | Für Schrott: Befehlskarten, Module, eine Karte entfernen, Reparatur.                                                       |
| Boss         | 1 je Sektor | Der Abschluss des Sektors (Abschnitt 12). Beute: ein Modul aus drei; nach Sektor 1 und 2 dazu Reparieren wie in der Werft. |

### Beute und Fortschritt

- **Befehlskarten.** Nach jedem Gefecht drei Karten zur Wahl, eine kommt ins Deck, oder keine. Das ist der Nachschub-Draft des alten Spiels an seinem richtigen Platz.
- **Schrott** ist die Währung. Gefecht 10 bis 20, Elite 25 bis 40, Boss 60. Preise im Schwarzmarkt: Karte 40 bis 70, Modul 120 bis 180, Karte entfernen 60, 10 Hülle 25.
- **Module** sind dauerhafte Passiva, die Relikte von Slay the Spire. Beispiele: Feuerleitrechner (alle eigenen Schüsse +1 ATK), Löschanlage (der eigene Brand tickt nicht), Reservereaktor (4 Befehlspunkte in Runde 1 jedes Gefechts), Panzerplatten (je Treffer −1 Schaden), Notaggregat (einmal im Run überlebt das Flaggschiff den tödlichen Treffer mit 1 Hülle), Bergungskran (jedes zerstörte gegnerische Schiff bringt 5 Schrott), Nachschubkapsel (jede Runde sechs statt fünf Befehle), Kampfsimulator (Kartenangebote zeigen vier statt drei), Verstärkte Spanten (+10 Hülle, dauerhaft).
- **Zwischen den Gefechten** bleibt die Hülle, wie sie ist. Der Brand erlischt, der Notfall bleibt, bis die Hülle wieder über 25 % liegt. Das Deck wird neu gemischt, Doktrinen enden, Module wirken weiter, Befehlspunkte starten bei 3.

### Ereignisse

Drei Beispiele für den Ton:

- **Das Wrack der Resolute.** Ein Kreuzer der eigenen Flotte, den das Wurmloch ebenfalls verschluckt hat, treibt ausgebrannt im Feld. Bergen (+40 Schrott, das Flaggschiff beginnt das nächste Gefecht brennend), die Logbücher sichern (ein zufälliges Modul) oder weiterfliegen und die Crew bestatten (nichts, aber ein Funkspruch, den man nicht vergisst).
- **Der Lotse.** Ein Händler bietet den sicheren Weg durch das Minenfeld: 50 Schrott, und der nächste Knoten wird eine Werft. Oder man fliegt selbst: −6 Hülle, dafür ein Kartenangebot.
- **Flackern.** Das Wurmloch meldet sich. Das Flaggschiff verliert ein Viertel seiner Hülle, dafür wird eine Karte des Decks aufgewertet; oder nichts geschieht, und man hört zum ersten Mal wieder den heimischen Funk.

## 12. Die Gegner

Die Gegner steuert das Spiel; im Run gibt es keinen zweiten Menschen und keinen planenden Bot. Gegner spielen keine Karten, haben keine Befehlspunkte und keine Stationen. Sie haben Aktionen, Eigenheiten und Muster, und sie sagen an, was sie tun.

### Absichten

In der Bilanz jeder Runde zeigt jede gegnerische Einheit ihre Absicht für die nächste Runde: Aktion, bei Zielfeuer den Schaden, bei Spaltenfeuer die Spalte und den Schaden, bei Aufladen den Zähler. Das ist der Kern des Runs: Der Spieler plant mit offenem Gegner, der Gegner weiß nichts vom Spieler. Vor Runde 1 wählt der Spieler seine Spalte, nachdem er Formation und Absichten gesehen hat; der Uhrmacher sieht zwei Runden. Der Spieler handelt zuerst: Ein Gegner, der im Spielerzug fällt, führt seine Absicht nie aus, einer im Notfall mit halber ATK.

Absichten werden bei der Ansage bestimmt und nicht mehr geändert. Ein Kurzsprung lässt jedes Feuer ins Leere gehen, ein Manöver nur das Spaltenfeuer: die beiden Fälle, in denen ein Schuss nichts trifft, und beide sind verdient. Damit Schilde und Manöver nicht zur sicheren Antwort werden, feuern Verbände mit mehreren Waffen, Schlachtschiffe laden auf, Spotter erhöhen den Preis jeder Runde, und die Gefechtsuhr bestraft das Zögern.

### Aktionen

Was ein Gegner in seinem Zug tun kann, und was der Spieler dagegen hat:

| Aktion                | Wer                                 | Was geschieht                                                                             | Antwort des Spielers                                  |
| --------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| **Zielfeuer**         | jeder Schütze                       | feuert mit seiner ATK auf das Flaggschiff und folgt ihm in jede Spalte                    | Schilde, Schildkarten, den Schützen vorher fällen     |
| **Spaltenfeuer**      | Torpedo-Staffel, Belagerer, Wächter | trifft, was im Gegnerzug in der angesagten Spalte steht                                   | Manöver, Kurzsprung, die Aufladung brechen            |
| **Rammkurs**          | Brander                             | fester Schaden, der Brander ist danach weg                                                | vorher fällen, Kurzsprung                             |
| **Schilde**           | Wachkreuzer, Belagerer, Wächter     | schluckt Schaden bis zum Ende des nächsten Spielerzugs                                    | abwarten, EMP-Stoß, ein anderes Ziel                  |
| **Ausweichen**        | Jäger, Kreuzer                      | halbiert die Schüsse des Spielers auf diese Einheit bis zum Ende des nächsten Spielerzugs | Torpedo, Railgun, Fokusfeuer, Sperrfeuer, Wärmesucher |
| **Aufladen**          | Schlachtschiffe                     | feuert nicht, nächste Runde doppelte ATK                                                  | EMP-Stoß, Funkstille, Stillstand, vorher fällen       |
| **Zielerfassung**     | Spotter                             | alle gegnerischen Schüsse +1 ATK, solange er steht                                        | den Spotter fällen, Traktorstrahl, Funkstille         |
| **Verstärkung rufen** | Trägerschiff, Schrottkönig          | eine neue Einheit trifft im nächsten Gegnerzug ein                                        | den Rufer fällen, Funkstille                          |

Gegner manövrieren nicht, reparieren nicht und spielen keine Karten. Ein Gegner, der eintrifft, sagt sofort seine erste Absicht an und handelt in der nächsten Runde. Ausweichen und Schilde eines Gegners sind als Zustand an ihm zu sehen, solange sie gelten.

### Muster

Jeder Gegnertyp folgt einem Muster aus Zielregel und Aktionsfolge. Ein Muster darf würfeln, welchen Zweig es als Nächstes ansagt; die Ansage selbst gilt exakt. Das ist neben der Hand der einzige Zufall im Gefecht. Die Modelle sind die heutigen acht Schiffe in fremder Lackierung, dazu einzelne neue Rümpfe für Bosse.

| Gegner            | Klasse         | Hülle | ATK | Modell                                           | Muster                                                                                                                |
| ----------------- | -------------- | ----- | --- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| Plünderer-Staffel | Jäger          | 8     | 2   | `void_interceptor`                               | Zielfeuer. Wurde sie in dieser Runde getroffen, sagt sie für die nächste Ausweichen an.                               |
| Brander           | Jäger          | 6     | 2   | `scrap_raider`                                   | Zwei Runden Anflug mit Ausweichen, dann Rammkurs: 10 Schaden, der Brander ist weg. Fällt er vorher, geschieht nichts. |
| Torpedo-Staffel   | Jäger          | 8     | 4   | Lanzen-Bomber, neues Modell statt `repair_drone` | Spaltenfeuer 8 auf die angesagte Spalte, dann eine Runde Ausweichen, von vorn.                                        |
| Wachkreuzer       | Kreuzer        | 16    | 4   | `horizon_cruiser`                                | Zielfeuer, Zielfeuer, Schilde 6, von vorn.                                                                            |
| Plasmakreuzer     | Kreuzer        | 14    | 4   | `plasma_frigate`                                 | Zielfeuer; seine Treffer zünden den Brand schon ab 60 % Hülle.                                                        |
| Spotter           | Kreuzer        | 12    | 2   | `signal_booster`                                 | Zielerfassung statt Zielfeuer: Solange er steht, alle gegnerischen Schüsse +1 ATK. Steht in der Back-Reihe, gedeckt.  |
| Belagerer         | Schlachtschiff | 30    | 8   | `iron_dreadnought`                               | Panzerung: jeder Treffer −2. Aufladen, dann Hauptgeschütz: Spaltenfeuer mit doppelter ATK, dann Schilde 10, von vorn. |
| Trägerschiff      | Schlachtschiff | 26    | 5   | `titan_carrier`                                  | Zielfeuer; alle zwei Runden ruft es eine Plünderer-Staffel in einen freien Slot.                                      |

**Aufladen** gibt es nur bei Gegnern: Die Einheit feuert nicht, nächste Runde feuert sie mit doppelter ATK. EMP-Stoß und Funkstille brechen die Aufladung ab, Stillstand verschiebt sie, und wer den Schützen im Spielerzug fällt, hat sie nie gesehen.

**Verstärkung.** Manche Gefechte sagen Verstärkung an: „Zwei Wachkreuzer in zwei Runden.“ Sie trifft in freie Slots ein, höchstens vier Einheiten stehen auf der Gegnerseite. Verstärkung macht ein Gefecht zum Wettlauf und gibt den Angriffskarten ein Ziel.

**Gefechtsuhr.** Ab Runde 8 erhält jede gegnerische Einheit zu Beginn jeder Runde +1 ATK, dauerhaft für dieses Gefecht. Das ist der alte Overdrive, nur für die Gegner: Er verhindert, dass der Spieler ein gewonnenes Gefecht in die Länge zieht, um sich mit Schadenskontrolle auf volle Hülle zu reparieren, und er beendet jedes Gefecht.

### Sektoren, Elite und Bosse

| Sektor            | Verbände                                                                                  | Elite                                        | Boss                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ----------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 Das Trümmerfeld | 1 bis 2 Einheiten: Plünderer, Wachkreuzer, Brander                                        | Belagerer allein                             | **Der Schrottkönig.** Ein zusammengeflicktes Schlachtschiff (Hive-Modell, geflickt), Hülle 60, ATK 7, mit zwei Plünderer-Staffeln als Schirm. Kannibalisieren: Jedes zerstörte Schiff auf dem Brett repariert ihn um 8. Alle drei Runden ruft er eine neue Plünderer-Staffel.                                                                                                                                                                                                            |
| 2 Die Stille      | 2 bis 3 Einheiten in Formation mit Deckung, erste Spotter, Torpedo-Staffeln und Belagerer | Belagerer mit zwei Wachkreuzern in der Front | **Der Wächter.** Eine automatische Verteidigungsstation (neues Modell), Hülle 90, unbeweglich, zwei Wachkreuzer davor. Hauptgeschütz: zwei Runden Aufladen, dann Spaltenfeuer 30 über zwei angesagte Spalten. Dazwischen Schilde 15. Wer nicht in der dritten Spalte steht, muss die Aufladung brechen, abfangen oder springen.                                                                                                                                                          |
| 3 Die Schwelle    | 3 bis 4 Einheiten, Verstärkung, Spotter mit Belagerern und Trägerschiffen                 | Zwei Belagerer mit Spotter                   | **Die Gegenflotte.** Kommandantin Vey mit ihrem Flaggschiff Siegel (Mirage-Modell), Hülle 50, ATK 8, und drei Eskorten; sie bewacht das Wurmloch nach Hause. Kommandofähigkeit Riegel, angesagt wie jede Aktion: Alle ihre Einheiten schlucken 4, ohne ihre eigene Aktion zu verlieren. Fällt eine Eskorte, springt eine neue ein, zweimal. Sie ist das einzige gegnerische Flaggschiff im Run und spielt mit Absichten, nach denselben Regeln für Schaden und Zustände wie der Spieler. |

Die Hülle der Verbände steigt je Sektor um ein Viertel. Nach dem Prototyp folgen Aufstiegsstufen, die die Startbedingungen verschärfen, und ein täglicher Run mit festem Seed.

### Was die Absichten mit dem Spielgefühl machen

Ohne menschlichen Gegner fällt das blinde Raten weg, und das ist gut so: Gegen eine Maschine wirkt Blindheit wie Zufall. Die Spannung entsteht stattdessen wie in Slay the Spire aus der Rechnung mit offenem Gegner. Ich sehe, dass der Belagerer nächste Runde 16 in meine Spalte feuert und zwei Wachkreuzer je 4 auf mich. Manöver entgeht den 16, aber nicht den Kreuzern, und ich feuere nicht. Schilde schlucken 8 von 24. Kurzsprung alles, für 2 Punkte und die Salve. Oder ich fokussiere den Belagerer: Fällt er im Spielerzug, feuert er nie, und ob er fällt, steht in der Vorschau, bevor ich klicke. Der Gefechtsbericht erzählt danach, wie es aussah.

## 13. Modi

Alle drei Modi nutzen dieselben Gefechtsregeln (Abschnitte 2 bis 9). Sie unterscheiden sich darin, wie viele Menschen am Brett sitzen und wer der Gegner ist: Run und Koop sind PvE, das Duell ist PvP.

|          | Run (zuerst)                               | Koop (später)                                                                | Duell (später)                                       |
| -------- | ------------------------------------------ | ---------------------------------------------------------------------------- | ---------------------------------------------------- |
| Menschen | einer, gegen das Spiel                     | zwei, gemeinsam gegen das Spiel                                              | zwei, gegeneinander                                  |
| Gegner   | Muster mit Absichten                       | Muster mit Absichten                                                         | das Flaggschiff des anderen Menschen; Bot als Ersatz |
| Flotte   | ein Flaggschiff                            | zwei Flaggschiffe nebeneinander, jedes ein Ziel; beide handeln im Spielerzug | ein Flaggschiff je Seite, Züge im Wechsel            |
| Deck     | Startdeck des Admirals, wächst durch Beute | je Spieler eigenes Deck und eigene 3 Befehlspunkte, gemeinsame Beute         | feste Decks, gleich für beide                        |
| Läuft    | im Client, offline, mit Seed               | über den Server                                                              | über den Server                                      |
| Sieg     | das Wurmloch nach Hause                    | wie Run                                                                      | gegnerisches Flaggschiff fällt                       |

Der Run braucht keinen Server: Die Auflösung liegt in `shared/`, die Muster der Gegner sind Daten, und ein Seed macht jeden Run wiederholbar. Das hält die Kosten dort, wo die Recherche sie haben will. Das Duell braucht später einen eigenen Entwurf: Im Wechsel gezogen dauert eine asynchrone Runde beide Züge nacheinander, das gleichzeitige blinde Ziehen des alten Spiels passt nicht mehr zu diesen Regeln, und Flaggschiff gegen Flaggschiff ohne Formation ist ein anderes Spiel als der Run.

## 14. Offene Punkte und Risiken

- **Entscheidungstiefe.** Eine Station plus bis zu drei Karten aus fünf ist dieselbe Breite wie ein Zug in Slay the Spire. Wenn sich die Runden trotzdem gleich anfühlen, sind die Reserven: Doktrinen und Alarmstufe Rot häufiger, oder zwei Stationen je Runde als Regel (Brücke und Geschützleitung).
- **Erstschlag.** Der Spieler handelt immer zuerst und mit festen Zahlen, also entscheidet er oft, ob ein Gegner fällt, bevor er handelt. Das ist der Motor von Slay the Spire, und die Vorschau zeigt die Rechnung, bevor der Spieler klickt; niemand muss sie im Kopf führen. Damit eine Runde nicht gelöst ist, bevor sie beginnt, gibt es Panzerung, angesagte Schilde, Verstärkung und die unbekannte nächste Hand.
- **Hülle über den Run.** Das Flaggschiff heilt nur in der Werft. Mit 50 bis 70 Hülle und Gegnern, die 10 bis 25 je Runde austeilen, entscheidet die Werft-Dichte über den Run. Startwerte sind geraten; die ersten Runs werden zeigen, ob die Hülle oder die Gegner zu hoch sind.
- **Spaltenfeuer und Manöver.** Manöver ist eine sichere Antwort auf ein einzelnes Spaltenfeuer, zum Preis der Salve. Das ist gewollt; zu beobachten ist, ob Spieler in Sektor 2 nur noch fliehen. Gegenmittel sind Torpedo-Staffeln und Spotter, die das Fliehen teuer machen.
- **Lösbarkeit.** Ohne Zufall und mit festen Aktionsfolgen ist ein Gefecht bei bekannter Hand vollständig vorhersehbar, und das Uhrwerk des Uhrmachers ist nur so viel wert, wie die Muster würfeln. Reichen Hand und Verstärkung nicht, bekommen die Muster Zweige, zwischen denen sie würfeln (Abschnitt 12); die Ansage bleibt exakt.
- **Kartenzahl.** 26 Karten sind für einen Run wenig; Slay the Spire hat je Charakter über 70. Der Prototyp läuft damit, die Veröffentlichung braucht 40 bis 60 im gemeinsamen Sockel plus 15 bis 20 je Admiral.
- **Run-Länge.** Rund fünfzehn Gefechte zu drei bis acht Runden. Der Gefechtsbericht muss sich schnell überspringen lassen, sonst wird der Run zäh. Messen.
- **Modelle.** Gegner als die heutigen acht Schiffe in fremder Lackierung, die Darstellung als 5, 2 und 1 Modelle mit Zerstörungsanimation je Modell. Neu: Lanzen-Bomber, Pyre, ein Porträt für Reyes, der Wächter, das Holodeck mit der Sternenkarte; Hive und Mirage werden Bossrümpfe.
- **Computergegner.** Der Gegner im Run ist kein Planer, sondern Muster und Ansage; der heutige Bot entfällt. Ein planender Bot wird erst für das Duell gebraucht.
- **Alle Zahlen** sind Startwerte. Vor der Abstimmung steht das Spielgefühl: Sieht man mehrere plausible Züge, erzählt der Bericht, was warum passiert ist, und fühlt sich Runde 6 anders an als Runde 2?

## 15. Umsetzungsreihenfolge

1. `shared/`: Flaggschiff, Stationen, Zustände, Schaden und Abzüge, Spielerzug mit sofortiger Wirkung, Gegnerzug und Bilanz, Gegnermuster und Absichten als Datenmodell und Ereignisstrom. Tests gegen die Beispielrunde.
2. `client/`: ein Gefecht gegen Muster-Gegner: Station am Flaggschiff wählen, Ziel und Spalte per Klick, Karten mit Befehlspunkten, Absichten über den Gegnern, Checkliste; dann der Gefechtsbericht mit Rundenbilanz; Regel-Dialog neu.
3. `client/`: der Run: Admiralswahl, Sternenkarte, Knoten, Beute, Werft, Schwarzmarkt, Ereignisse, Speicherstand.
4. Assets: Modellgruppen und Gegnerlackierungen, Pyre, Reyes, Wächter, Holodeck, Funksprüche für Brücke, Geschützleitung und Gegnerfunk.
5. `server/`: Koop und Duell auf derselben Auflösung; Bot für das Duell.
