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

| Karte | BP | Typ | Wirkung | Aufgewertet |
| --- | --- | --- | --- | --- |
| Torpedo | 2 | Angriff | 9 Schaden auf ein gewähltes Schiff. Zielsuchend | 12 Schaden |
| Orbitalschlag | 3 | Angriff | 16 Schaden auf ein gewähltes Schiff. Durchschlagend | 21 Schaden |
| Salvenfeuer | 1 | Angriff | 3 Schaden auf jede gegnerische Einheit. Durchschlagend | 5 Schaden |
| Sperrfeuer | 0 | Angriff | 6 Schaden auf jedes gegnerische Schiff, das gerade ausweicht. Zielsuchend | 9 Schaden |
| Wärmesucher | 2 | Angriff | 9 Schaden auf ein gewähltes Schiff; 13, wenn es gerade ausweicht. Zielsuchend, durchschlagend | 12 Schaden (16) |
| Gnadenstoß | 1 | Angriff | 4 Schaden auf ein gewähltes Schiff; 12, wenn es im Notfall ist | 6 Schaden (14) |
| Bombardement | 1 | Angriff | 5 Schaden auf jede Einheit in der gegnerischen Back-Reihe. Durchschlagend | 8 Schaden |
| Rammkurs | 2 | Angriff | Das Flaggschiff rammt ein gewähltes Schiff: Das Ziel nimmt das Doppelte der ATK des Flaggschiffs, das Flaggschiff die ATK des Ziels. Zielsuchend, durchschlagend | 4 Schaden mehr |
| Brandsatz | 1 | Angriff | 4 Schaden auf ein gewähltes Schiff, Brand. Kennkarte der Glut | 6 Schaden |
| Notschilde | 1 | Verstärkung | Schilde 5 | Schilde 8 |
| Standhalten | 1 | Verstärkung | Schilde 4, Abwehrfeuer 2. Kennkarte der Mauer | Schilde 6 |
| Überladung | 0 | Verstärkung | Das Flaggschiff feuert diese Runde mit +6 ATK und nimmt am Zugende 2 Schaden. Kennkarte des Hammers | +9 ATK |
| Energiestoß | 1 | Verstärkung | Das Flaggschiff feuert diese Runde mit +7 ATK | +10 ATK |
| Reparaturtrupp | 1 | Verstärkung | Brand gelöscht, +4 Hülle, zusätzlich zur Station; zählt zur [Reparaturgrenze](gefecht.md#stationen-des-flaggschiffs) | +6 Hülle |
| EMP-Stoß | 1 | Schwächung | Ein gegnerisches Schiff: Seine Schilde fallen aus, eine Aufladung bricht ab | 0 BP |
| Zielstörer | 1 | Schwächung | Alle gegnerischen Schüsse im Gegnerzug −3 ATK | −4 ATK |
| Funkstille | 3 | Schwächung | Die Sonderaktion eines gegnerischen Schiffs (Aufladen, Zielerfassung, Verstärkung rufen, Riegel) fällt diese Runde aus, und es feuert auch nicht | 2 BP |
| Traktorstrahl | 1 | Schwächung | Ein gegnerisches Schiff außer einem Flaggschiff wird in einen freien Slot seiner Reihe gezogen; Deckung bricht | 0 BP |
| Kurzsprung | 3 | Taktik | Einmal je Gefecht, statt einer Station: Das Flaggschiff verlässt das Brett, ist im Gegnerzug kein Ziel und kehrt in der Auflösung in seine Spalte zurück. Die Karte ist danach bis zum Gefechtsende aus dem Spiel. Kennkarte des Uhrmachers | 2 BP |
| Ausweichmanöver | 1 | Taktik | Das Flaggschiff wechselt sofort die Spalte, ohne eine Station zu verbrauchen | 0 BP |
| Gegenmaßnahmen | 1 | Taktik | Der erste gegnerische Treffer im Gegnerzug, ob Zielfeuer, Spaltenfeuer oder Rammkurs, macht halben Schaden | 0 BP |
| Plündern | 1 | Taktik | Drei Befehle ziehen | 0 BP |
| Alarmstufe Rot | 2 | Taktik | Das Flaggschiff darf diese Runde eine zweite, andere Station auslösen; die Admiralsstation darf eine davon sein | 1 BP |
| Ablativpanzerung | 1 | Doktrin | Panzerung 1 | Panzerung 2 |
| Feuerleitdoktrin | 1 | Doktrin | Alle eigenen Schüsse und Angriffskarten +1 Schaden | +2 Schaden |
| Alarmbereitschaft | 1 | Doktrin | In jedem Gegnerzug Schilde 2 | Schilde 3 |

Die Werte sind mit dem Simulator eingestellt ([Balance](balance.md)): Befehlspunkte sind knapper als Karten, also muss eine Beutekarte je Punkt mehr leisten als Torpedo und Notschilde, sonst verdünnt sie nur das Deck.

Doktrinen sind die Power-Karten von Slay the Spire: Sie bleiben liegen, bis das Gefecht endet.

## Startdeck und Aufwertung

Jeder Admiral beginnt mit einem **Startdeck** von zehn Karten: dreimal Torpedo, dreimal Notschilde, Reparaturtrupp, Plündern und zweimal seine Kennkarte. Alle übrigen Karten kommen als Beute; Dubletten sind erlaubt. Jede Karte hat eine **aufgewertete Fassung** für die Werft: mehr Schaden, Schild, Reparatur oder ATK, oder ein Befehlspunkt weniger; was genau, steht in der Tabelle und auf der Karte.

## Kartenpools

Für den Prototyp teilen sich die Admirale diesen Pool. Das Ziel für die Veröffentlichung sind eigene Pools wie in Slay the Spire: 15 bis 20 Karten je Admiral, die nur in seinem Run als Beute erscheinen (Flak und Vergeltung für die Mauer, Durchschlag und Sturmlauf für den Hammer, Vorhersage und Zeitfenster für den Uhrmacher, Brandbeschleuniger und Flächenbrand für die Glut), über einem kleineren gemeinsamen Sockel.
