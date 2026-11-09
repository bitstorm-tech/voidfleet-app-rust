# Vision

Voidfleet ist ein PvE-Spiel: ein Einzelspieler-Run im Stil von Slay the Spire, ein Mensch gegen Gegner, die das Spiel nach festen Mustern steuert. Der Spieler führt genau ein Schiff, sein Flaggschiff, gegen Verbände, Schlachtschiffe und Bosse; ein zweiter Mensch sitzt nicht am Brett. Koop und Duell folgen später auf denselben Gefechtsregeln.

Alle Zahlen in diesen Seiten sind Startwerte, nicht abgestimmt. Jede Regel und jede Zahl steht auf genau einer Seite; die anderen Seiten verweisen dorthin.

## Begriffe

- **Der Spieler** ist ein Mensch. Er führt genau eine Einheit, sein Flaggschiff, löst dessen Stationen aus und spielt Karten.
- **Das Flaggschiff** ohne Zusatz ist immer das Schiff des Spielers. Ein gegnerisches Flaggschiff gibt es nur im Boss-Gefecht von Sektor 3.
- **Die Gegner** sind alle Einheiten auf der anderen Seite. Sie steuert das Spiel nach festen Mustern mit angesagten Absichten ([Gefecht](gefecht.md#absichten-und-aktionen), [Gegner](gegner.md)): kein zweiter Mensch, kein planender Bot. Sie spielen keine Karten und haben keine Stationen.
- **Die Runde** ist Spielerzug, Gegnerzug, Auflösung ([Gefecht](gefecht.md#die-runde)). Im Spielerzug handelt nur der Spieler, im Gegnerzug nur das Spiel.

## Was das Spiel sein soll

Nach dem MDA-Framework, von hinten nach vorn:

- **Ästhetik.** Kommandanten-Fantasie und Schauwert. Der Moment, für den man wiederkommt, ist „Was für eine Schlacht“: ein Schiff, eine Crew, eine Übermacht. Ton wie Battlestar Galactica und The Expanse: rau, militärisch, Schadensmeldungen von der Brücke, Verluste und Schaden tun weh. Der Ton kommt aus Funk, Text, Zuständen und Inszenierung, nicht aus dem Look; der Clean-Toon-Stil bleibt und wird detailreicher und schmutziger. Nicht im Zentrum: Mindgame; die offene Rechnung ist Mittel, nicht Zweck.
- **Dynamik.** Eines gegen viele, Mensch gegen Spiel, in Zügen im Wechsel. Die gegnerische Flotte steht in Formation und sagt an, was sie in ihrem Zug tun wird; der Spieler handelt zuerst, rechnet gegen die Ansage und entscheidet, welche Station seines Schiffs in dieser Runde läuft: Waffen, Schilde, Schadenskontrolle, Manöver oder Admiral; dann führt das Spiel aus, was von der Ansage noch steht. Der Spannungsbogen kommt aus Verfall: Gegner brennen, fallen in den Notfall, verlieren sichtbar Modelle, und das Flaggschiff trägt seine Hülle durch den ganzen Run. Kein Schuss geht ins Nichts, und jede Zahl steht fest, bevor der Spieler klickt. Jede Runde stellt die Frage: Wo gebe ich Feuerkraft auf, um zu überleben, und wofür gebe ich meine Befehlspunkte aus?
- **Mechanik.** Das Flaggschiff löst pro Runde eine Station aus, die wie eine Karte sofort wirkt. Befehlspunkte zahlen Karten aus einem Deck, das im Run wächst. Vier Admirale als Charaktere, jeder mit Waffe, Passiv, Admiralsstation und Startdeck. Gegner haben Muster und Absichten statt Karten. Drei Sektoren mit Sternenkarte, Beute und Bossen. Der Gegnerzug ist ein erzählter Gefechtsbericht. Fällt das Flaggschiff, endet der Run.

## Grundentscheidungen

- **Feste Zahlen statt Genauigkeit und Zufall.** Die Rechnung gegen die Ansage braucht exakte Werte.
- **Züge im Wechsel statt gleichzeitiger Auflösung.** Gegen Muster-Gegner gibt es nichts zu raten.

## Die Geschichte

Während einer gewaltigen Schlacht reißt ein versehentlich geöffnetes Wurmloch das Flaggschiff des Spielers aus der Formation und wirft es an einen fremden Ort im Universum. Die Flotte ist weg, die Karten sind nutzlos, der Funk schweigt. Der Weg nach Hause führt durch drei Sektoren, und jeder, der dort fliegt, will das fremde Schiff: Plünderer, die es ausschlachten wollen, Wachflotten, die es für einen Angreifer halten, und am Ende eine Kommandantin, die das Wurmloch nach Hause bewacht.

## Modi

Alle drei Modi nutzen dieselben Gefechtsregeln ([Gefecht](gefecht.md)). Sie unterscheiden sich darin, wie viele Menschen am Brett sitzen und wer der Gegner ist: Run und Koop sind PvE, das Duell ist PvP.

|  | Run (zuerst) | Koop (später) | Duell (später) |
| --- | --- | --- | --- |
| Menschen | einer, gegen das Spiel | zwei, gemeinsam gegen das Spiel | zwei, gegeneinander |
| Gegner | Muster mit Absichten | Muster mit Absichten | das Flaggschiff des anderen Menschen; Bot als Ersatz |
| Flotte | ein Flaggschiff | zwei Flaggschiffe nebeneinander, jedes ein Ziel; beide handeln im Spielerzug | ein Flaggschiff je Seite, Züge im Wechsel |
| Deck | Startdeck des Admirals, wächst durch Beute | je Spieler eigenes Deck und eigene 3 Befehlspunkte, gemeinsame Beute | feste Decks, gleich für beide |
| Läuft | im Client, offline, mit Seed | über den Server | über den Server |
| Sieg | das Wurmloch nach Hause | wie Run | gegnerisches Flaggschiff fällt |

Der Run braucht keinen Server: Die Gefechtsregeln liegen in `crates/core`, die Muster der Gegner sind Daten, und ein Seed macht jeden Run wiederholbar. Ohne Server entstehen für den Run keine laufenden Kosten. Das Duell braucht später einen eigenen Entwurf: Im Wechsel gezogen dauert eine asynchrone Runde beide Züge nacheinander, das gleichzeitige blinde Ziehen des alten Spiels passt nicht mehr zu diesen Regeln, und Flaggschiff gegen Flaggschiff ohne Formation ist ein anderes Spiel als der Run.
