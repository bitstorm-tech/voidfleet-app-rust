# Das Gefecht

Die Regeln eines Gefechts: Brett, Einheiten, Schaden, Zustände, Stationen, die Runde und die Absichten der Gegner. Die Karten stehen in [Karten](karten.md), die Admirale in [Admirale](admirale.md), die einzelnen Gegner und ihre Muster in [Gegner](gegner.md).

## Brett und Formation

- **Gegnerseite**, vom Spiel gestellt. Zwei Reihen, **Front** und **Back**, mit je drei **Spalten**, höchstens vier Einheiten.
- **Deckung.** Eine gegnerische Einheit in der Back-Reihe ist gedeckt, wenn in ihrer Spalte vorn eine Einheit steht. Von jedem Schaden auf ein gedecktes Ziel fängt der Vordermann die aufgerundete Hälfte ab, das Ziel bekommt den Rest. Eine Jäger-Staffel deckt wie jede andere Einheit: Sie ist dann der Schirm, der zerrieben wird.
- **Spielerseite.** Drei Spalten, das Flaggschiff des Spielers steht in einer davon, sonst nichts. Die Back-Reihe bleibt leer; im Koop steht dort später das zweite Flaggschiff.
- **Feuer.** Das Flaggschiff feuert immer auf ein gewähltes Schiff. Spaltenfeuer gibt es nur auf der Gegnerseite ([Aktionen](#aktionen)).

## Einheiten

| Klasse | Seite | Darstellung | Rolle |
| --- | --- | --- | --- |
| Jäger-Staffel | nur Gegner | 5 kleine Modelle, die mit der Hülle verschwinden (je 20 % ein Modell) | schwach, aber viele; weicht aus, deckt, rammt |
| Kreuzer-Paar | nur Gegner | 2 mittlere Modelle; bei 50 % Hülle oder weniger brennt eines, im Notfall ist eines ein Wrack | das Rückgrat jedes Verbands, Feuerleitung, Plasma |
| Schlachtschiff | nur Gegner | 1 großes Modell mit Bränden und Rumpfbrüchen | Spaltenfeuer mit Aufladen, Panzerung, ruft Verstärkung |
| Flaggschiff | Spieler; Gegner nur als Boss | wie Schlachtschiff, größer, mit Ring und drei Sternen | das Schiff des Spielers, der Held des Runs |

Jede Einheit hat **Hülle** und **ATK**, keinen Schildwert. Schilde gibt es nur auf Zeit. Beim Flaggschiff kommen sie aus der Schild-Station und aus Befehlskarten, gelten für den folgenden Gegnerzug und verfallen in der Auflösung. Bei Gegnern kommen sie aus der Aktion Schilde und gelten bis zum Ende des nächsten Spielerzugs. Dazu kommt Deckung.

## Schaden

Jeder Schuss trifft, und jede Zahl steht fest: Ein Schuss macht so viel Schaden wie die ATK des Schützen, ohne Zufall und ohne Trefferwahrscheinlichkeit. Was die Ansage zeigt, geschieht im Gegnerzug; was die Vorschau am Ziel zeigt, macht der Schuss des Spielers. Die Varianz des Runs liegt in der Hand, in der Beute und in den Mustern der Gegner ([Gegner](gegner.md)), nicht im Schuss.

Ein **Schuss** ist jeder Angriff mit ATK: Waffen-Station und Fokusfeuer des Flaggschiffs, Zielfeuer und Spaltenfeuer der Gegner. Angriffskarten nennen ihren Schaden selbst, der Rammkurs des Branders macht festen Schaden.

Vom Schaden eines Schusses auf einen Gegner wird in dieser Reihenfolge abgezogen:

1. **Ausweichen** des Ziels: halber Schaden. Nur Gegner weichen aus; was zielsuchend ist, übergeht es ([Schlüsselwörter](karten.md#schlüsselwörter)).
2. **Panzerung** des Ziels: ein fester Abzug je Treffer, nie unter 0.
3. **Deckung**: Der Vordermann fängt die Hälfte ab, das Ziel bekommt den Rest ([Brett und Formation](#brett-und-formation)); was durchschlagend ist, übergeht sie.
4. **Schilde** des Ziels schlucken, was übrig ist, bis sie aufgebraucht sind.

Beim Flaggschiff gibt es weder Ausweichen noch Deckung: Gegenmaßnahmen halbieren zuerst, dann zieht die Panzerung ab, dann schlucken die Schilde.

Alle Werte sind ganze Zahlen; wo gehälftelt wird, wird aufgerundet. Spaltenfeuer und Rammkurs treffen voll oder gar nicht; Ausweichen zählt für sie nicht.

## Zustände

Zustände sind sichtbar. Sie sind der Motor der Eskalation.

| Zustand | Auslöser | Wirkung | Ende |
| --- | --- | --- | --- |
| **Brand** (Jäger: „Zersprengt“) | ein Treffer bringt die Hülle auf 50 % oder weniger; beim Flaggschiff früher, wenn ein Plasmakreuzer trifft; bei Gegnern bei jedem Treffer des Plasmawerfers und durch Brandsatz | in der Auflösung −2 Hülle | beim Flaggschiff Schadenskontrolle, Reparaturtrupp oder das Gefechtsende; ein Gegner brennt, bis er fällt |
| **Notfall** (Flaggschiff: „Brücke getroffen“) | Hülle bei 25 % oder weniger | ATK halbiert. Gegner: kein Ausweichen. Flaggschiff: keine Admiralsstation | Hülle wieder über 25 % |
| **Zerstört** | Hülle 0 | Gegner: verlässt das Brett, der Slot ist frei für Verstärkung. Flaggschiff: der Run endet | — |

Ein Brand kann in der Auflösung zerstören: „Ausgebrannt.“ Was mit Hülle, Brand und Notfall zwischen den Gefechten geschieht, steht in [Run](run.md#die-hülle-im-run).

## Stationen des Flaggschiffs

Stationen hat nur das Flaggschiff; was die Gegner tun, sind angesagte Aktionen ([Aktionen](#aktionen)). Das Flaggschiff löst pro Runde eine Station aus, jede höchstens einmal. Sie wirkt sofort beim Auslösen, wie eine Karte, und zwischen Karten und Station zählt die Reihenfolge: Ein Torpedo vor dem Schuss schwächt das Ziel, ein Schuss vor dem Gnadenstoß kann es in den Notfall bringen. Eine Station wird wie eine Karte ausgelöst und bekommt dabei ihr Ziel: Waffen, Fokusfeuer und Stillstand ein gegnerisches Schiff, Manöver eine andere Spalte; die übrigen Stationen brauchen keins.

| Station | Wirkung |
| --- | --- |
| **Waffen**, feuert auf ein gewähltes Schiff | sofort die ATK des Flaggschiffs auf das Ziel, mit den Abzügen aus [Schaden](#schaden) |
| **Schilde**, feuert nicht | schluckt im folgenden Gegnerzug 8 Schaden, Zielfeuer wie Spaltenfeuer; was übrig ist, verfällt in der Auflösung |
| **Schadenskontrolle**, feuert nicht | löscht den Brand, +5 Hülle, nie über die volle Hülle |
| **Manöver**, feuert nicht | wechselt die Spalte und entgeht so dem Spaltenfeuer, das auf die alte Spalte angesagt ist; Zielfeuer folgt |
| **Admiralsstation** | je Admiral, siehe [Admirale](admirale.md); im Notfall nicht möglich |

Regeln im Detail:

- Waffen ohne Ziel lassen sich nicht auslösen, Manöver nicht ohne andere Spalte. Es gibt keine automatische Zielwahl; die Checkliste mahnt die freie Station an.
- Eine Runde ohne Station ist erlaubt.
- Schilde schlucken allen Schaden, der im Gegnerzug auf das Flaggschiff gerichtet ist, Karten-Schilde (Notschilde, Standhalten, Alarmbereitschaft) kommen dazu.
- Das Flaggschiff weicht nicht aus; Panzerung hat es nur durch Ablativpanzerung und Module. Gegen gegnerisches Feuer hat der Spieler vier Antworten: Schilde gegen den Schaden, Manöver gegen Spaltenfeuer, Kurzsprung gegen alles, und den Schützen zerstören, bevor er handelt.
- Die Karte Alarmstufe Rot erlaubt eine zweite, andere Station in dieser Runde; das ist der einzige Weg zu zwei Stationen.

## Die Runde

Mensch und Spiel ziehen im Wechsel. Eine Runde besteht aus dem Spielerzug, in dem nur der Spieler handelt, dem Gegnerzug, in dem nur die Gegner handeln, und der Auflösung, die das Spiel von selbst ausführt. Das Flaggschiff beginnt jedes Gefecht in der mittleren Spalte.

| Abschnitt | Wer handelt | Was geschieht |
| --- | --- | --- |
| **Spielerzug** | der Spieler | Karten und Station wirken sofort, in der Reihenfolge des Spielers ([Der Spielerzug](#der-spielerzug)). Gegner mit Hülle 0 fallen auf der Stelle und handeln in dieser Runde nicht mehr. |
| **Gegnerzug** | das Spiel | Angesagte Verstärkung trifft ein und sagt ihre erste Absicht an. Dann handelt jede gegnerische Einheit nach ihrer Absicht, Front vor Back, links nach rechts; wer Ausweichen angesagt hat, tut jetzt nichts. Schilde des Flaggschiffs, Gegenfeuer, Standhalten und Gegenmaßnahmen werden je Treffer gerechnet. Fällt das Flaggschiff, endet der Run hier. |
| **Auflösung** | niemand, das Spiel rechnet | Brände ticken auf beiden Seiten. Zustände werden neu bestimmt und ausgerufen. Schilde des Flaggschiffs verfallen, der Kurzsprung kehrt zurück. Ab Runde 8 tickt die Gefechtsuhr, die Gegner sagen ihre Absichten für die nächste Runde an ([Absichten](#absichten)). Ganz am Ende wird die Hand abgeworfen und fünf Befehle nachgezogen, Befehlspunkte auf 3. |

Weil der Spieler zuerst handelt, feuert ein Gegner, der im Spielerzug fällt, nie: Das ist die wichtigste Regel des Runs. Jeder Angriff ist eine Rechnung gegen die Ansage: Was fällt, bevor es handelt, hat nie gehandelt; was in den Notfall geht, handelt mit halber ATK. Umgekehrt gibt es keinen Schutz, den man nicht vor dem Zugende aufgebaut hat.

### Der Spielerzug

Im Spielerzug wirkt alles sofort, in der Reihenfolge, in der der Spieler es tut, wie in Slay the Spire:

1. **Karten.** Beliebig viele Befehlskarten, solange die **Befehlspunkte** reichen. Jede Runde 3 Punkte, sie verfallen am Zugende. Jede Karte wirkt beim Spielen; wer erst den Torpedo einschlagen sieht, wählt danach das Ziel seiner Station. Überladung und Energiestoß gelten nur für Schüsse danach.
2. **Station.** Ein Klick auf eine Station löst sie aus, vor, zwischen oder nach den Karten. Braucht sie ein Ziel, leuchten die gültigen Ziele, und der Klick auf eines führt sie aus, genau wie bei einer Karte; sonst wirkt sie sofort.
3. **Zug beenden.** Der Gegnerzug beginnt.

Die Hand wird jede Runde neu gezogen: Nicht gespielte Karten gehen in der Auflösung auf den Ablagestapel, und ist das Deck leer, wird der Ablagestapel gemischt. So zählt jede Karte im Deck, und der Deckbau im Run hat eine Wirkung.

## Absichten und Aktionen

Gegner haben Aktionen, Eigenheiten und Muster, und sie sagen an, was sie tun.

### Absichten

In der Auflösung jeder Runde zeigt jede gegnerische Einheit ihre Absicht für die nächste Runde: Aktion, bei Zielfeuer den Schaden, bei Spaltenfeuer die Spalte und den Schaden, bei Aufladen den Zähler. Das ist der Kern des Runs: Der Spieler plant mit offenem Gegner, der Gegner weiß nichts vom Spieler.

Die Aktion einer Absicht wird bei der Ansage bestimmt und nicht mehr geändert; der angezeigte Schaden folgt dem aktuellen Stand, etwa Notfall, Zielstörer oder Spotter. Ein Kurzsprung lässt jedes Feuer ins Leere gehen, ein Manöver nur das Spaltenfeuer: die beiden Fälle, in denen ein Schuss nichts trifft, und beide sind verdient. Damit Schilde und Manöver nicht zur sicheren Antwort werden, feuern Verbände mit mehreren Waffen, Schlachtschiffe laden auf, Spotter erhöhen den Preis jeder Runde, und die Gefechtsuhr bestraft das Zögern.

Ohne menschlichen Gegner fällt das blinde Raten weg, und das ist gut so: Gegen eine Maschine wirkt Blindheit wie Zufall. Die Spannung entsteht stattdessen wie in Slay the Spire aus der Rechnung mit offenem Gegner. Ich sehe, dass der Belagerer nächste Runde 16 in meine Spalte feuert und zwei Wachkreuzer je 4 auf mich. Manöver entgeht den 16, aber nicht den Kreuzern, und ich feuere nicht. Schilde schlucken 8 von 24. Kurzsprung entgeht allem, für 2 Punkte; die Station muss dann vorher laufen. Oder ich fokussiere den Belagerer: Fällt er im Spielerzug, feuert er nie, und ob er fällt, steht in der Vorschau, bevor ich klicke. Der Gefechtsbericht erzählt danach, wie es aussah.

### Aktionen

Was ein Gegner in seinem Zug tun kann, und was der Spieler dagegen hat:

| Aktion | Wer | Was geschieht | Antwort des Spielers |
| --- | --- | --- | --- |
| **Zielfeuer** | jeder Schütze | feuert mit seiner ATK auf das Flaggschiff und folgt ihm in jede Spalte | Schilde, Schildkarten, den Schützen vorher fällen |
| **Spaltenfeuer** | Torpedo-Staffel, Belagerer, Wächter | trifft, was im Gegnerzug in der angesagten Spalte steht | Manöver, Kurzsprung, die Aufladung brechen |
| **Rammkurs** | Brander | fester Schaden, der Brander ist danach weg | vorher fällen, Kurzsprung |
| **Schilde** | Wachkreuzer, Belagerer, Wächter | schluckt Schaden bis zum Ende des nächsten Spielerzugs | abwarten, EMP-Stoß, ein anderes Ziel |
| **Ausweichen** | Jäger | gilt ab der Ansage: halbiert die Schüsse des Spielers in diesem Spielerzug, feuert nicht | Torpedo, Railgun, Fokusfeuer, Sperrfeuer, Wärmesucher |
| **Aufladen** | Belagerer, Wächter | feuert nicht, nächste Runde doppelte ATK | EMP-Stoß, Funkstille, Stillstand, vorher fällen |
| **Zielerfassung** | Spotter | alle gegnerischen Schüsse +1 ATK, solange er steht | den Spotter fällen, Traktorstrahl, Funkstille |
| **Verstärkung rufen** | Trägerschiff, Schrottkönig | eine neue Einheit trifft sofort ein und steht in der nächsten Runde auf dem Brett | den Rufer fällen, Funkstille |

Gegner manövrieren nicht, reparieren nicht und spielen keine Karten. Ausweichen ist die eine Absicht, die schon mit der Ansage gilt und mit ihr endet; alle anderen wirken erst im Gegnerzug. Ausweichen und Schilde eines Gegners sind als Zustand an ihm zu sehen, solange sie gelten.

Zielerfassung, Zielstörer, Gefechtsuhr und Notfall ändern die ATK, also Zielfeuer und Spaltenfeuer gleichermaßen. Der Rammkurs bleibt bei seinem festen Schaden.

### Verstärkung

Verstärkung kommt auf zwei Wegen: Manche Gefechte sagen sie an („Zwei Wachkreuzer in zwei Runden“), und manche Gegner rufen sie mit der Aktion Verstärkung rufen. Angesagte Verstärkung trifft zu Beginn des Gegnerzugs ein, gerufene sofort mit dem Ruf, beide in freie Slots; die Grenze von vier Einheiten gilt auch für sie. Wer eintrifft, sagt sofort seine erste Absicht an und handelt in der nächsten Runde. Verstärkung macht ein Gefecht zum Wettlauf und gibt den Angriffskarten ein Ziel.

### Gefechtsuhr

Ab Runde 8 erhält jede gegnerische Einheit +1 ATK je Runde, dauerhaft für dieses Gefecht: +1 in Runde 8, +2 in Runde 9. Die Uhr tickt in der Auflösung, bevor die Gegner ansagen; der Zuschlag steht also schon in der Ansage. Sie verhindert, dass der Spieler ein gewonnenes Gefecht in die Länge zieht, um sich mit Schadenskontrolle auf volle Hülle zu reparieren, und sie beendet jedes Gefecht.
