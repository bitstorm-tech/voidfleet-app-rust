# Balance und Simulator

Die Zahlen des Spiels sind gemessen, nicht geschätzt: Ein Bot spielt Runs mit den Daten aus `assets/data/`, und seine Tabellen zeigen, wo eine Zahl nicht stimmt. Diese Seite sagt, wie man misst, welche Ziele gelten und was die letzte Messung ergab. Wer eine Zahl, eine Karte, ein Modul, einen Admiral oder einen Verband ändert, misst nach und zieht die Tabellen hier nach.

## Messen

```
just sim                      Runs je Admiral: Siegquote, Verlust je Knoten, gespielte Karten
just sim cards -n 300         jede Karte: dieselben Runs mit einer Karte mehr im Startdeck
just sim modules -n 300       jedes Modul: dieselben Runs mit dem Modul von Beginn an
just sim trace --admiral kerr ein Run Zug für Zug, um dem Bot zuzusehen
```

`-n` ist die Zahl der Runs je Admiral (Vorgabe 300), `--admiral` misst nur einen, `--data <ordner>` nimmt eine Kopie von `assets/data`, um Zahlen zu probieren, ohne das Spiel anzufassen. 1000 Runs je Admiral dauern rund anderthalb Minuten, `cards` etwa zehn.

**Der Bot** probiert in jedem Zug jede Folge aus Karten und Stationen aus, lässt den Gegnerzug laufen und nimmt den Zug mit der höchsten Hülle abzüglich des Schadens, den die übrigen Gegner voraussichtlich noch anrichten. Er rechnet also jede Vorschau, die das Spiel zeigt, und plant nicht über die Runde hinaus. Im Run entscheidet er schlicht: Beute zufällig, Werft nach Hülle, angeschlagen um die Elite herum. Er ist damit im Gefecht genauer als ein Mensch und beim Deckbau schlechter; die Zahlen sind Vergleichswerte zwischen Admiralen, Karten und Knoten, keine Vorhersage für einen Spieler. Wie vorsichtig er spielt, ist geeicht: Er nimmt die Gewichtung, mit der er am häufigsten gewinnt (`crates/sim/AGENTS.md`).

**Lesen.** `-%max` ist die Hülle, die ein Gefecht im Schnitt kostet, in Prozent der vollen Hülle; `Tod%` der Anteil der Besuche, die dort enden. Ein Knoten mit wenig Verlust und viel Tod liegt hinter einem harten Knoten: Dort kommt man angeschlagen an. Bei `cards` und `modules` zählt `Δ Siege/Run` (gewonnene Gefechte je Run gegen die Runs ohne); unter 300 Runs je Admiral schwankt der Wert um etwa ±0,25.

## Ziele

| Was | Ziel |
| --- | --- |
| Siegquote des Bots | 50 bis 60 %, die vier Admirale höchstens 8 Punkte auseinander |
| Wo Runs enden | etwa 8 % in Sektor 1, 15 % in Sektor 2, 22 % in Sektor 3 |
| Gefecht | 3 bis 6 Runden; Verlust 3 bis 5 % in Sektor 1, 5 bis 7 % in Sektor 2, 7 bis 9 % in Sektor 3, kein Knoten über 16 % |
| Elite | 6 bis 9 Runden, 11 bis 18 % |
| Boss | 8 bis 11 Runden; 18, 23 und 32 % in Sektor 1, 2 und 3 |
| Karte | eine Karte mehr im Startdeck ändert die Siege je Run um −0,6 bis +0,6; keine Karte ist für jedes Deck richtig, keine ist toter Ballast |
| Modul | +0,5 bis +1,5 Siege je Run |

Die erste Reihe von Sektor 1 kostet mit Absicht nichts: Sie ist der Auftakt. Innerhalb eines Sektors steigen die Kosten von Reihe zu Reihe, und jeder Sektor hat ein, zwei leichte Knoten als Atempause.

## Stand der Messung (13.11.2026, 1000 Runs je Admiral)

| Admiral | Siegquote | endet in Sektor 1 / 2 / 3 | Hülle vor dem Boss 1 / 2 / 3 |
| --- | --- | --- | --- |
| Osei | 53 % | 12 / 12 / 23 % | 73 / 68 / 67 % |
| Vance | 58 % | 10 / 17 / 15 % | 71 / 67 / 70 % |
| Kerr | 52 % | 6 / 16 / 26 % | 75 / 70 / 69 % |
| Reyes | 56 % | 5 / 16 / 24 % | 72 / 66 / 67 % |

| Sektor | Gefecht | Elite | Boss |
| --- | --- | --- | --- |
| 1 | 3,9 % in 3,6 Runden | 14,2 % in 6,7 Runden | 18,7 % in 9,4 Runden, 6 % enden dort |
| 2 | 5,7 % in 4,4 Runden | 11,6 % in 6,3 Runden | 23,3 % in 10,8 Runden, 10 % enden dort |
| 3 | 7,9 % in 5,7 Runden | 16,3 % in 8,6 Runden | 32,5 % in 8,4 Runden, 15 % enden dort |

**Karten** (Δ Siege je Run mit einer Karte mehr im Startdeck, 300 Runs je Admiral):

| Bereich | Karten |
| --- | --- |
| +0,3 bis +0,6 | Kurzsprung, Gegenmaßnahmen, Zielstörer, Alarmstufe Rot |
| 0 bis +0,3 | Alarmbereitschaft, Orbitalschlag, Notschilde, Brandsatz, Standhalten, Funkstille, Salvenfeuer, Gnadenstoß |
| −0,7 bis −0,3 | Wärmesucher, Ausweichmanöver, Energiestoß, Ablativpanzerung, Torpedo, Bombardement, Plündern, Überladung, Feuerleitdoktrin |
| unter −0,8 | Reparaturtrupp, Sperrfeuer, Traktorstrahl, EMP-Stoß, Rammkurs |

Eine tote Karte kostet etwa −1,0: Die Hand hat fünf Karten und drei Befehlspunkte, jede Karte, die man nicht spielt, verdrängt eine, die man gespielt hätte. Die vier untersten sind für den Bot tot: Sperrfeuer, Traktorstrahl und EMP-Stoß wirken nur gegen bestimmte Gegner, den Rammkurs spielt er fast nie (5 % der Hände), weil er Hülle kostet. Ob sie für einen Menschen besser sind, zeigt das Spielen. Der Reparaturtrupp teilt sich die Reparaturgrenze mit der Schadenskontrolle; als Startkarte darf er schwach sein.

**Module** (Δ Siege je Run mit dem Modul von Beginn an): Reservereaktor +2,3, Löschanlage +2,0, Panzerplatten +1,8, Nachschubkapsel +1,5, Verstärkte Spanten +1,2, Feuerleitrechner +1,0, Bergungskran +0,9, Notaggregat +0,6, Kampfsimulator +0,3 (der Bot wählt Beute zufällig, ein größeres Angebot nützt ihm nichts).

## Was die Messung gezeigt hat

- **Verteidigung schlägt Angriff.** Schild-Station und Notschilde schlucken einen Verband mit drei Schützen fast ganz, ein vierter Schütze kostet sprunghaft mehr. Deshalb sind die Gegner zerbrechlich und schlagen hart zu, und deshalb stellt die Zahl der Schützen einen Knoten ein, nicht die Hülle.
- **Zeit kostet wenig.** Die Gefechtsuhr beginnt in Runde 8 und erhöht nur die ATK. Drei Regeln schließen, was sich damit ausnutzen ließ: die Reparaturgrenze ([Gefecht](gefecht.md#stationen-des-flaggschiffs)), der Stillstand lädt eine Runde nach ([Admirale](admirale.md)), der Kurzsprung läuft einmal je Gefecht und statt einer Station ([Karten](karten.md)). Vorher gewann der Bot jeden Run, indem er den schwächsten Gegner stehen ließ und sich reparierte; Kerr gewann mit zwei Kurzsprüngen 69 % der Runs und ohne sie 6 %.
- **Befehlspunkte sind knapper als Karten.** Eine Beutekarte hilft nur, wenn sie je Punkt mehr leistet als Torpedo und Notschilde. Die Angriffskarten machen deshalb rund die Hälfte mehr Schaden als im ersten Entwurf oder kosten einen Punkt weniger.

## Offen

- Reservereaktor, Löschanlage und Panzerplatten sind doppelt so stark wie die übrigen Module und lassen sich mit ihrer Zahl nicht schwächen (ein Befehlspunkt, ein Punkt Panzerung, Brand an oder aus). Sie bräuchten einen Preis oder eine andere Wirkung.
- „Logbücher sichern“ im Wrack der Resolute gibt ein Modul ohne Preis und ist immer die beste Wahl.
- Die Knoten Sektor 2 Reihe 6 rechts und Sektor 3 Reihe 5 und 6 kosten im Schnitt wenig, beenden aber 7 bis 10 % der Besuche: Dort trifft das Hauptgeschütz eines Belagerers, wer angeschlagen ankommt.
- Zwei Knoten kosten den Bot nichts (Sektor 1 Reihe 2 Mitte, Sektor 2 Reihe 5 mit den Brandern).
