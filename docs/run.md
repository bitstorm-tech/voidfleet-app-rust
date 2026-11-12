# Der Run

Der erste Modus: ein Mensch gegen das Spiel. Vorbild ist Slay the Spire: ein Held, ein wachsendes Deck, eine Karte mit Abzweigungen, ein Boss am Ende jedes Akts, und nach dem Tod beginnt alles von vorn.

Der Held ist das Flaggschiff. Der Spieler wählt zu Beginn einen der vier Admirale ([Admirale](admirale.md)), so wie man in Slay the Spire den Charakter wählt. Es gibt keine Eskorten und keine zweite eigene Einheit: Was der Spieler gewinnt, sind Karten, Module und Schrott, und was er verliert, ist Hülle. Warum es dort draußen ist, erzählt [Die Geschichte](vision.md#die-geschichte).

## Die Sternenkarte

Auf dem Holodeck der Brücke liegt der Sektor als Netz von Knoten, von unten nach oben, mit Abzweigungen; der Spieler sieht den ganzen Sektor und wählt an jeder Verzweigung. Drei Sektoren mit je neun Reihen, die letzte ist der Boss. Ein Pfad durch einen Sektor hat etwa fünf Gefechte, ein Run damit rund fünfzehn, und das Ziel sind 60 bis 90 Minuten. Das ist zu messen, nicht zu setzen.

Die Sternenkarten sind entworfen, nicht gewürfelt: Jede Karte ist eine Datei ([Daten](daten.md#sternenkarten)), und ein neuer Run zieht je Sektor eine der Karten dieses Sektors. Die Zahl der Reihen und die Anteile der Tabelle sind die Vorgabe für den Entwurf, keine Regel, die das Spiel prüft.

| Knoten | Anteil | Was dort geschieht |
| --- | --- | --- |
| Gefecht | ~50 % | Ein Verband des Sektors, von der einzelnen Staffel bis zur Flotte. Beute: Schrott und eine Befehlskarte aus drei. |
| Elite | ~12 % | Ein mächtiges Schlachtschiff, oft mit Schirm, der Zwischenboss des Sektors. Beute: Schrott und ein Modul aus drei. |
| Ereignis | ~20 % | Eine Szene über Funk mit zwei oder drei Entscheidungen. |
| Werft | ~12 % | Eines von dreien: Reparieren (+30 % Hülle), Aufwerten (eine Karte), Ausmustern (eine Karte aus dem Deck entfernen). |
| Schwarzmarkt | ~6 % | Für Schrott: Befehlskarten, Module, eine Karte entfernen, Reparatur. |
| Boss | 1 je Sektor | Der Abschluss des Sektors ([Gegner](gegner.md#sektoren-elite-und-bosse)). Beute: Schrott und ein Modul aus drei; nach Sektor 1 und 2 dazu Reparieren wie in der Werft. |

## Beute und Fortschritt

- **Befehlskarten.** Nach jedem Gefecht-Knoten drei Karten zur Wahl, eine kommt ins Deck, oder keine. Elite und Boss geben stattdessen ein Modul.
- **Schrott** ist die Währung. Gefecht 10 bis 20, Elite 25 bis 40, Boss 60. Preise im Schwarzmarkt: Karte 40 bis 70, Modul 120 bis 180, Karte entfernen 60, 10 Hülle 25.
- **Module** sind dauerhafte Passiva, die Relikte von Slay the Spire. Beispiele: Feuerleitrechner (alle eigenen Schüsse +1 ATK), Löschanlage (der eigene Brand tickt nicht), Reservereaktor (4 Befehlspunkte in Runde 1 jedes Gefechts), Panzerplatten (je Treffer −1 Schaden), Notaggregat (einmal im Run überlebt das Flaggschiff den tödlichen Treffer mit 1 Hülle), Bergungskran (jedes zerstörte gegnerische Schiff bringt 5 Schrott), Nachschubkapsel (jede Runde sechs statt fünf Befehle), Kampfsimulator (Kartenangebote zeigen vier statt drei), Verstärkte Spanten (+10 Hülle, dauerhaft).

## Die Hülle im Run

Die Hülle des Flaggschiffs bleibt zwischen den Gefechten, wie sie ist. Ein Brand erlischt nach dem Gefecht, der Notfall bleibt, bis die Hülle wieder über 25 % liegt. Das Flaggschiff ist der Held des Runs: Jeder Punkt Hülle ist eine Entscheidung, und ein Gefecht knapp zu gewinnen ist nicht dasselbe wie es sauber zu gewinnen.

Geheilt wird außerhalb des Gefechts an drei Stellen: in der Werft, im Schwarzmarkt und nach dem Boss von Sektor 1 und 2. Im Gefecht heilen Schadenskontrolle und Reparaturtrupp. Ereignisse heilen bisher nicht.

Zwischen den Gefechten wird das Deck neu gemischt, Doktrinen enden, Module wirken weiter.

## Ereignisse

Drei Beispiele für den Ton:

- **Das Wrack der Resolute.** Ein Kreuzer der eigenen Flotte, den das Wurmloch ebenfalls verschluckt hat, treibt ausgebrannt im Feld. Bergen (+40 Schrott, das Flaggschiff beginnt das nächste Gefecht brennend), die Logbücher sichern (ein zufälliges Modul) oder weiterfliegen und die Crew bestatten (nichts, aber ein Funkspruch, den man nicht vergisst).
- **Der Lotse.** Ein Händler bietet den sicheren Weg durch das Minenfeld: 50 Schrott, und der nächste Knoten wird eine Werft. Oder man fliegt selbst: −6 Hülle, dafür ein Kartenangebot.
- **Flackern.** Das Wurmloch meldet sich. Das Flaggschiff verliert ein Viertel seiner Hülle, dafür wird eine Karte des Decks aufgewertet; oder nichts geschieht, und man hört zum ersten Mal wieder den heimischen Funk.

## Später

Nach dem Prototyp folgen Aufstiegsstufen, die die Startbedingungen verschärfen, und ein täglicher Run mit festem Seed.
