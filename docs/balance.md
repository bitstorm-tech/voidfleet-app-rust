# Balance und Simulator

Die Zahlen des Spiels sind gemessen, nicht geschätzt: Ein Bot spielt Runs mit den Daten aus `assets/data/`, und seine Tabellen zeigen, wo eine Zahl nicht stimmt. Diese Seite sagt, wie man misst, welche Ziele gelten und was die letzte Messung ergab. Wer eine Zahl, eine Karte, ein Modul, einen Admiral oder einen Verband ändert, misst nach und zieht die Tabellen hier nach.

## Messen

```
just sim                      Runs je Admiral: Siegquote, Verlust je Knoten, gespielte Karten
just sim cards -n 300         jede Karte: dieselben Runs mit einer Karte mehr im Startdeck
just sim modules -n 300       jedes Modul: dieselben Runs mit dem Modul von Beginn an
just sim trace --admiral kerr ein Run Zug für Zug, um dem Bot zuzusehen
```

`-n` ist die Zahl der Runs je Admiral (Vorgabe 300), `--admiral` misst nur einen, `--data <ordner>` nimmt eine Kopie von `assets/data`, um Zahlen zu probieren, ohne das Spiel anzufassen. 1000 Runs je Admiral dauern rund dreieinhalb Minuten, `cards` (48 Karten) rund fünfzig, `modules` etwa fünfzehn.

**Der Bot** probiert in jedem Zug jede Folge aus Karten und Stationen aus, lässt den Gegnerzug laufen und nimmt den Zug mit der höchsten Hülle abzüglich des Schadens, den die übrigen Gegner voraussichtlich noch anrichten. Er rechnet also jede Vorschau, die das Spiel zeigt, und plant nicht über die Runde hinaus. Im Run entscheidet er schlicht: Beute zufällig, Werft nach Hülle, angeschlagen um die Elite herum. Er ist damit im Gefecht genauer als ein Mensch und beim Deckbau schlechter; die Zahlen sind Vergleichswerte zwischen Admiralen, Karten und Knoten, keine Vorhersage für einen Spieler. Wie vorsichtig er spielt, ist geeicht: Er nimmt die Gewichtung, mit der er am häufigsten gewinnt (`crates/sim/AGENTS.md`).

**Lesen.** `-%max` ist die Hülle, die ein Gefecht im Schnitt kostet, in Prozent der vollen Hülle; `Tod%` der Anteil der Besuche, die dort enden. Ein Knoten mit wenig Verlust und viel Tod liegt hinter einem harten Knoten: Dort kommt man angeschlagen an. Bei `cards` und `modules` zählt `Δ Siege/Run` (gewonnene Gefechte je Run gegen die Runs ohne); unter 300 Runs je Admiral schwankt der Wert um etwa ±0,25.

## Ziele

| Was | Ziel |
| --- | --- |
| Siegquote des Bots | 50 bis 60 %, die vier Admirale höchstens 8 Punkte auseinander |
| Wo Runs enden | etwa 8 % in Sektor 1, 15 % in Sektor 2, 22 % in Sektor 3 |
| Gefecht | 3 bis 6 Runden; Verlust 3 bis 5 % in Sektor 1, 5 bis 7 % in Sektor 2, 7 bis 9 % in Sektor 3, kein Knoten über 16 % |
| Elite | 6 bis 9 Runden, 11 bis 18 % |
| Boss | 8 bis 11 Runden; 18, 23 und 32 % in Sektor 1, 2 und 3. Der Boss von Sektor 3 hat zwei Hälften ([Der Durchbruch](gegner.md#der-durchbruch)): die Gegenflotte etwa 5 Runden und 13 %, das Fremde den Rest |
| Karte | eine Karte mehr im Startdeck ändert die Siege je Run um −0,6 bis +0,6; keine Karte ist für jedes Deck richtig, keine ist toter Ballast |
| Modul | +0,5 bis +1,5 Siege je Run |

Die erste Reihe von Sektor 1 kostet mit Absicht nichts: Sie ist der Auftakt. Innerhalb eines Sektors steigen die Kosten von Reihe zu Reihe, und jeder Sektor hat ein, zwei leichte Knoten als Atempause.

## Stand der Messung (26.11.2026, 1000 Runs je Admiral)

| Admiral | Siegquote | endet in Sektor 1 / 2 / 3 | Hülle vor dem Boss 1 / 2 / 3 |
| --- | --- | --- | --- |
| Osei | 58,5 % | 9 / 10 / 23 % | 74 / 69 / 67 % |
| Vance | 58,7 % | 6 / 17 / 18 % | 71 / 66 / 69 % |
| Kerr | 59,2 % | 5 / 15 / 20 % | 74 / 69 / 69 % |
| Reyes | 54,0 % | 4 / 19 / 23 % | 71 / 65 / 66 % |

| Sektor | Gefecht | Elite | Boss |
| --- | --- | --- | --- |
| 1 | 3,9 % in 3,5 Runden | 13,6 % in 6,3 Runden | 17,2 % in 8,7 Runden, 5 % enden dort |
| 2 | 6,0 % in 4,0 Runden | 12,2 % in 5,8 Runden | 23,7 % in 9,3 Runden, 9 % enden dort |
| 3 | 8,6 % in 5,2 Runden | 14,7 % in 8,0 Runden | 29,0 % in 10,6 Runden, 11 % enden dort |

Gemessen über alle 48 Befehlskarten mit Admiral-Pools (mindestens 1 garantierte Admiral-Karte je Beute- und Schwarzmarkt-Angebot) und dem Durchbruch im Boss von Sektor 3. Die vier Admirale liegen mit 54,0 bis 59,2 % (Mittel 57,6 %) innerhalb des Zielkorridors von 50 bis 60 % und nur 5,2 Prozentpunkte auseinander.

**Karten** (Δ Siege je Run mit einer Karte mehr im Startdeck, 300 Runs je Admiral):

| Bereich | Karten |
| --- | --- |
| +0,6 bis +1,1 | Notenergie (+1,06), Durchbruchsalve (+1,05), Bereitschaftsfeuer (+0,66) |
| +0,1 bis +0,6 | Brechergeschoss (+0,52), Kurzsprung (+0,46), Gegenmaßnahmen (+0,38), Orbitalschlag (+0,37), Alarmstufe Rot (+0,26), Alarmbereitschaft (+0,24), Zielstörer (+0,21), Schildstoß (+0,19), Standhalten (+0,15), Brandsatz (+0,15) |
| −0,3 bis +0,1 | Gnadenstoß (+0,03), Sturmlauf (+0,03), Salvenfeuer (+0,02), Notschilde (0,00), Funkstille (−0,25) |
| −0,8 bis −0,3 | Wärmesucher (−0,31), Autokanonen-Salve (−0,32), Ausweichmanöver (−0,34), Zielbeleuchtung (−0,46), Chrono-Schlag (−0,48), Bombardement (−0,51), Präzisionsmarkierung (−0,53), Ablativpanzerung (−0,54), Torpedo (−0,55), Energiestoß (−0,60), Feuerleitdoktrin (−0,64), Plündern (−0,68), Bollwerk (−0,72), Überladung (−0,73), Flächenbrand (−0,75), Offensivdoktrin (−0,75), Kettenbefehl (−0,76), Brandbeschleuniger (−0,78) |
| unter −0,8 | Flakbrecher (−0,87), Reparaturtrupp (−0,88), Zeitfenster (−0,89), Schmelzsalve (−0,89), Traktorstrahl (−1,00), Sperrfeuer (−1,00), Brückenkoordination (−1,02), EMP-Stoß (−1,03), Bastionsdoktrin (−1,03), Uhrwerkdoktrin (−1,07), Rammkurs (−1,14), Glutdoktrin (−1,15) |

Eine tote Karte kostet etwa −1,0: Die Hand hat fünf Karten und drei Befehlspunkte, jede Karte, die man nicht spielt, verdrängt eine, die man gespielt hätte. Bei den Admiral-Karten ist zusätzlich die Spalte des eigenen Admirals maßgeblich, weil sie im normalen Run nur dort erscheinen: `Schildstoß` bringt Osei +1,72 (bei anderen negativ), `Flächenbrand` (−0,11) und `Brandbeschleuniger` (−0,23) drücken bei Reyes kaum, während sie andere Decks ohne Brand-Synergie belasten. Teure Doktrinen (2–3 BP) und Aufbaukarten (`Zeitfenster`, `Uhrwerkdoktrin`, `Glutdoktrin`, `Brückenkoordination`) schätzt der einrundige Bot zu schwach ein, weil sie im Zug ihrer Ausspielung unmittelbare Hülle/Schilde kosten. Sperrfeuer, Traktorstrahl und EMP-Stoß wirken nur gegen bestimmte Gegner, den Rammkurs spielt der Bot fast nie (4,4 % der Hände), weil er Hülle kostet. Der Reparaturtrupp teilt sich die Reparaturgrenze mit der Schadenskontrolle; als Startkarte darf er schwach sein.

**Module** (Δ Siege je Run mit dem Modul von Beginn an, 300 Runs je Admiral): Reservereaktor +2,04, Löschanlage +1,77, Panzerplatten +1,57, Nachschubkapsel +1,40, Verstärkte Spanten +1,05, Bergungskran +0,72, Feuerleitrechner +0,66, Notaggregat +0,42, Kampfsimulator +0,15 (der Bot wählt Beute zufällig, ein größeres Angebot nützt ihm nichts).

## Was die Messung gezeigt hat

- **Verteidigung schlägt Angriff.** Schild-Station und Notschilde schlucken einen Verband mit drei Schützen fast ganz, ein vierter Schütze kostet sprunghaft mehr. Deshalb sind die Gegner zerbrechlich und schlagen hart zu, und deshalb stellt die Zahl der Schützen einen Knoten ein, nicht die Hülle.
- **Zeit kostet wenig.** Die Gefechtsuhr beginnt in Runde 8 und erhöht nur die ATK. Drei Regeln schließen, was sich damit ausnutzen ließ: die Reparaturgrenze ([Gefecht](gefecht.md#stationen-des-flaggschiffs)), der Stillstand lädt eine Runde nach ([Admirale](admirale.md)), der Kurzsprung läuft einmal je Gefecht und statt einer Station ([Karten](karten.md)). Vorher gewann der Bot jeden Run, indem er den schwächsten Gegner stehen ließ und sich reparierte; Kerr gewann mit zwei Kurzsprüngen 69 % der Runs und ohne sie 6 %.
- **Ein einzelner Gegner droht wenig.** Ein Deck mit Schildkarten schluckt jeden Einzeltreffer bis etwa 20 mit Station und zwei Karten; was einen Verband gefährlich macht, sind mehrere Schützen in derselben Runde. Der Unbekannte Kontakt steht allein und braucht deshalb eine ATK, die doppelt so hoch ist wie die jedes Verbands (26), einen Schlag, dem kein Manöver ausweicht (52 über alle Spalten), und Brut, die in der Runde rammt, in der er auflädt. Mit ATK 20 kostete er den Bot 8 % Hülle in sechs Runden, die Gegenflotte davor, deren Schiffe einzeln halb so hart treffen, 13 % in fünf.
- **Die Lebende Hülle bestraft das Mauern, nicht die kleine Kanone.** Als fester Zuwachs je Runde (5) kostete sie Osei 6 Punkte Siegquote und Reyes keinen: Wer langsam tötet, zahlt jede Runde, und der Plasmawerfer zündet sie sofort weg. Seit sie nur in Runden ohne Treffer wächst, liegen die Admirale dicht beisammen, und ihre Zahl ist fast gleichgültig: Der Bot trifft in jeder Runde. Ein Mensch, der eine Runde nur Schilde hebt, zahlt sie.
- **Admiral-Pools und BP-Wirtschaft verschieben die Deckkurve stark.** Mit der Einführung der 22 neuen Karten und dem garantierten Admiral-Slot in Beute und Schwarzmarkt stieg die Bot-Siegquote im ersten Entwurf auf 84,5 % (Osei 88,8 %, Vance 78,2 %, Kerr 91,4 %, Reyes 79,7 %; 13,2 Punkte Abstand), weil billige BP-/Ziehkarten (`Notenergie`, `Kettenbefehl`, `Präzisionsmarkierung`, `Bereitschaftsfeuer`) und starke Admiral-Kombos (`Schildstoß`, `Zeitfenster`, `Flakbrecher`) den Schaden am Sektor-3-Boss halbierten. Erst die Anhebung der BP-Kosten bzw. Anpassung der Werte an diesen 22 neuen Karten brachte alle vier Admirale zurück in den Zielkorridor von 54,0 bis 59,2 % (5,2 Punkte Abstand).
- **Befehlspunkte sind knapper als Karten.** Eine Beutekarte hilft nur, wenn sie je Punkt mehr leistet als Torpedo und Notschilde — gleichzeitig schlagen Karten mit 0 BP oder BP-Rückgewinn (`Notenergie` +1,06, `Durchbruchsalve` +1,05, `Bereitschaftsfeuer` +0,66) in der Einzelkarten-Messung besonders stark aus.

## Offen

- `Notenergie` (+1,06) und `Durchbruchsalve` (+1,05) liegen in der Einzelkarten-Messung über dem Zielband (−0,6 bis +0,6), und Oseis `Schildstoß` schlägt spezifisch auf Osei mit +1,72 aus; ein menschlicher Spieltest sollte prüfen, ob diese Karten in gebauten Decks zu stark dominieren.
- Die teuren Doktrinen (`Bastionsdoktrin`, `Uhrwerkdoktrin`, `Glutdoktrin`, `Brückenkoordination`) sowie `Zeitfenster` liegen beim einrundigen Bot unter −0,8, weil er den Mehrrunden-Ertrag gegen den unmittelbaren BP-Verlust im Spielzug unterschätzt.
- Der Schlag des Fremden (52 über alle Spalten) ist der größte Einzeltreffer des Spiels. Der Bot beantwortet ihn mit Kurzsprung, Gegenmaßnahmen oder Schilden und Hülle; ob ein Mensch ohne diese Karten ihn als fair empfindet, zeigt das Spielen.
- Reservereaktor, Löschanlage und Panzerplatten bleiben doppelt so stark wie die übrigen Module und lassen sich mit ihrer Zahl nicht schwächen (ein Befehlspunkt, ein Punkt Panzerung, Brand an oder aus). Sie bräuchten einen Preis oder eine andere Wirkung.
- „Logbücher sichern“ im Wrack der Resolute gibt ein Modul ohne Preis und ist immer die beste Wahl.
- Zwei Knoten kosten den Bot fast nichts (Sektor 1 Reihe 2 Mitte, Sektor 2 Reihe 5 mit den Brandern).
