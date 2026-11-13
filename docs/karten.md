# Das Befehlsdeck

Karten spielt nur der Spieler. Kosten in Befehlspunkten (BP); Kosten, Werte und Texte stehen je Karte in `assets/data/cards/<id>.json` ([Daten](daten.md#karten)), die Tabellen hier nennen die Startwerte. Angriffskarten treffen immer, mit festem Schaden und den Abzügen eines Schusses ([Schaden](gefecht.md#schaden)), wo nichts anderes steht.

## Schlüsselwörter

Ein Schlüsselwort ist ein Wort mit fester Bedeutung, wie in Slay the Spire oder Hearthstone. Auf Karten, Stationen, Waffen, Modulen und Gegnern steht nur das Wort, mit seiner Zahl, wo es eine hat; was es heißt, steht hier und nirgends sonst. Neue Karten benutzen diese Wörter, statt die Regel auszuschreiben.

| Schlüsselwort | Bedeutung |
| --- | --- |
| **Zielsuchend** | Ausweichen zählt nicht: Der Schaden wird nicht halbiert. |
| **Durchschlagend** | Deckung zählt nicht: Das Ziel nimmt den ganzen Schaden, der Vordermann nichts. |
| **Schilde N** | Schluckt N Schaden. Beim Flaggschiff im folgenden Gegnerzug, der Rest verfällt in der Auflösung; bei Gegnern bis zum Ende des nächsten Spielerzugs. Mehrere Schilde zählen zusammen. |
| **Panzerung N** | Jeder Treffer macht N weniger, nie unter 0. Mehrere Panzerungen zählen zusammen. |
| **Abwehrfeuer N** | Jeder Schütze, der das Flaggschiff im Gegnerzug trifft, verliert N Hülle. Mehrere Abwehrfeuer zählen zusammen. |
| **Brand** | Der Zustand aus [Zustände](gefecht.md#zustände); „Brand“ auf einer Karte setzt das Ziel in Brand. |
| **Notfall** | Der Zustand aus [Zustände](gefecht.md#zustände). |
| **Doktrin** | Der Kartentyp: Die Wirkung gilt für den Rest des Gefechts. |
| **Kennkarte** | Die Karte eines Admirals, zweimal in seinem Startdeck. |

Wo Ausweichen, Deckung, Schilde und Panzerung in der Rechnung stehen, sagt [Schaden](gefecht.md#schaden).

## Die Karten

| Karte | BP | Typ | Wirkung |
| --- | --- | --- | --- |
| Torpedo | 2 | Angriff | 6 Schaden auf ein gewähltes Schiff. Zielsuchend |
| Orbitalschlag | 3 | Angriff | 7 Schaden auf ein gewähltes Schiff. Durchschlagend |
| Salvenfeuer | 2 | Angriff | 2 Schaden auf jede gegnerische Einheit. Durchschlagend |
| Sperrfeuer | 1 | Angriff | 4 Schaden auf jedes gegnerische Schiff, das gerade ausweicht. Zielsuchend |
| Wärmesucher | 2 | Angriff | 5 Schaden auf ein gewähltes Schiff; 8, wenn es gerade ausweicht. Zielsuchend, durchschlagend |
| Gnadenstoß | 2 | Angriff | 3 Schaden auf ein gewähltes Schiff; 8, wenn es im Notfall ist |
| Bombardement | 2 | Angriff | 3 Schaden auf jede Einheit in der gegnerischen Back-Reihe. Durchschlagend |
| Rammkurs | 3 | Angriff | Das Flaggschiff rammt ein gewähltes Schiff: Das Ziel nimmt das Doppelte der ATK des Flaggschiffs, das Flaggschiff die ATK des Ziels. Zielsuchend, durchschlagend |
| Brandsatz | 1 | Angriff | 2 Schaden auf ein gewähltes Schiff, Brand. Kennkarte der Glut |
| Notschilde | 1 | Verstärkung | Schilde 5 |
| Standhalten | 1 | Verstärkung | Schilde 5, Abwehrfeuer 2. Kennkarte der Mauer |
| Überladung | 1 | Verstärkung | Das Flaggschiff feuert diese Runde mit +5 ATK und nimmt am Zugende 3 Schaden. Kennkarte des Hammers |
| Energiestoß | 2 | Verstärkung | Das Flaggschiff feuert diese Runde mit +3 ATK |
| Reparaturtrupp | 1 | Verstärkung | Brand gelöscht, +4 Hülle, zusätzlich zur Station |
| EMP-Stoß | 2 | Schwächung | Ein gegnerisches Schiff: Seine Schilde fallen aus, eine Aufladung bricht ab |
| Zielstörer | 1 | Schwächung | Alle gegnerischen Schüsse im Gegnerzug −1 ATK |
| Funkstille | 2 | Schwächung | Die Sonderaktion eines gegnerischen Schiffs (Aufladen, Zielerfassung, Verstärkung rufen, Riegel) fällt diese Runde aus, und es feuert auch nicht |
| Traktorstrahl | 2 | Schwächung | Ein gegnerisches Schiff außer einem Flaggschiff wird in einen freien Slot seiner Reihe gezogen; Deckung bricht |
| Kurzsprung | 2 | Taktik | Das Flaggschiff verlässt das Brett: Es führt danach in dieser Runde keine Station mehr aus, ist im Gegnerzug kein Ziel und kehrt in der Auflösung in seine Spalte zurück. Kennkarte des Uhrmachers |
| Ausweichmanöver | 1 | Taktik | Das Flaggschiff wechselt sofort die Spalte, ohne eine Station zu verbrauchen |
| Gegenmaßnahmen | 1 | Taktik | Der erste gegnerische Treffer im Gegnerzug, ob Zielfeuer, Spaltenfeuer oder Rammkurs, macht halben Schaden |
| Plündern | 1 | Taktik | Zwei Befehle ziehen |
| Alarmstufe Rot | 2 | Taktik | Das Flaggschiff darf diese Runde eine zweite, andere Station auslösen; die Admiralsstation darf eine davon sein |
| Ablativpanzerung | 2 | Doktrin | Panzerung 1 |
| Feuerleitdoktrin | 2 | Doktrin | Alle eigenen Schüsse und Angriffskarten +1 Schaden |
| Alarmbereitschaft | 2 | Doktrin | In jedem Gegnerzug Schilde 2 |

Doktrinen sind die Power-Karten von Slay the Spire: Sie bleiben liegen, bis das Gefecht endet.

## Startdeck und Aufwertung

Jeder Admiral beginnt mit einem **Startdeck** von zehn Karten: dreimal Torpedo, dreimal Notschilde, Reparaturtrupp, Plündern und zweimal seine Kennkarte. Alle übrigen Karten kommen als Beute; Dubletten sind erlaubt. Jede Karte hat eine **aufgewertete Fassung** für die Werft: Angriffskarten +2 Schaden, Schild- und Reparaturwerte +2, alle anderen kosten einen Befehlspunkt weniger, mindestens 0.

## Kartenpools

Für den Prototyp teilen sich die Admirale diesen Pool. Das Ziel für die Veröffentlichung sind eigene Pools wie in Slay the Spire: 15 bis 20 Karten je Admiral, die nur in seinem Run als Beute erscheinen (Flak und Vergeltung für die Mauer, Durchschlag und Sturmlauf für den Hammer, Vorhersage und Zeitfenster für den Uhrmacher, Brandbeschleuniger und Flächenbrand für die Glut), über einem kleineren gemeinsamen Sockel.
