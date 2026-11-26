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
| **Markiert** | Gegner-Zustand bis zur Auflösung: Jeder Treffer macht +50 % Schaden (aufgerundet, nach Ausweichen und Deckung, vor Schilden und Panzerung). |
| **Panzerbruch** | Senkt die Panzerung des Ziels dauerhaft für dieses Gefecht (mindestens 0). |
| **Einmalig** | Nach dem Spielen bis zum Gefechtsende aus dem Spiel, statt auf den Ablagestapel. |
| **Behalten** | Bleibt in der Auflösung auf der Hand, statt abgeworfen zu werden. |
| **Doktrin** | Der Kartentyp: Die Wirkung gilt für den Rest des Gefechts. |
| **Kennkarte** | Die Karte eines Admirals, zweimal in seinem Startdeck und nur in seinem Kartenpool. |

Wo Ausweichen, Deckung, Markiert, Schilde und Panzerung in der Rechnung stehen, sagt [Schaden](gefecht.md#schaden).

## Die Karten

Alle 48 Karten verteilen sich auf den gemeinsamen Sockel (28 neutrale Karten) und vier Admiral-Pools (je 5 Karten).

### Gemeinsamer Sockel (28 neutrale Karten)

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
| Autokanonen-Salve | 1 | Angriff | 3× 1 Schaden auf ein gewähltes Schiff. Zielsuchend | 3× 2 Schaden |
| Bereitschaftsfeuer | 2 | Angriff | 5 Schaden auf ein gewähltes Schiff. Behalten; kostet 0 BP, wenn sie eine Runde behalten wurde | 8 Schaden |
| Notschilde | 1 | Verstärkung | Schilde 5 | Schilde 8 |
| Energiestoß | 1 | Verstärkung | Das Flaggschiff feuert diese Runde mit +7 ATK | +10 ATK |
| Reparaturtrupp | 1 | Verstärkung | Brand gelöscht, +4 Hülle, zusätzlich zur Station; zählt zur [Reparaturgrenze](gefecht.md#stationen-des-flaggschiffs) | +6 Hülle |
| EMP-Stoß | 1 | Schwächung | Ein gegnerisches Schiff: Seine Schilde fallen aus, eine Aufladung bricht ab | 0 BP |
| Zielstörer | 1 | Schwächung | Alle gegnerischen Schüsse im Gegnerzug −3 ATK | −4 ATK |
| Funkstille | 3 | Schwächung | Die Sonderaktion eines gegnerischen Schiffs (Aufladen, Zielerfassung, Verstärkung rufen, Riegel) fällt diese Runde aus, und es feuert auch nicht | 2 BP |
| Traktorstrahl | 1 | Schwächung | Ein gegnerisches Schiff außer einem Flaggschiff wird in einen freien Slot seiner Reihe gezogen; Deckung bricht | 0 BP |
| Zielbeleuchtung | 1 | Schwächung | Ein gewähltes Schiff wird Markiert und verliert dauerhaft 1 Panzerung | 2 Panzerung |
| Ausweichmanöver | 1 | Taktik | Das Flaggschiff wechselt sofort die Spalte, ohne eine Station zu verbrauchen | 0 BP |
| Gegenmaßnahmen | 1 | Taktik | Der erste gegnerische Treffer im Gegnerzug, ob Zielfeuer, Spaltenfeuer oder Rammkurs, macht halben Schaden | 0 BP |
| Plündern | 1 | Taktik | Drei Befehle ziehen | 0 BP |
| Alarmstufe Rot | 2 | Taktik | Das Flaggschiff darf diese Runde eine zweite, andere Station auslösen; die Admiralsstation darf eine davon sein | 1 BP |
| Notenergie | 0 | Taktik | +1 BP für diese Runde. Einmalig | +1 BP, 1 Befehl ziehen |
| Kettenbefehl | 2 | Taktik | 3 Befehle ziehen; wurde diese Runde schon eine Station ausgelöst, +1 BP | 1 BP |
| Ablativpanzerung | 1 | Doktrin | Panzerung 1 | Panzerung 2 |
| Feuerleitdoktrin | 1 | Doktrin | Alle eigenen Schüsse und Angriffskarten +1 Schaden | +2 Schaden |
| Alarmbereitschaft | 1 | Doktrin | In jedem Gegnerzug Schilde 2 | Schilde 3 |
| Brückenkoordination | 2 | Doktrin | Immer wenn eine Station ausgelöst wird: 1 Befehl ziehen | 1 BP |

### Osei, die Mauer (5 Admiral-Karten)

| Karte | BP | Typ | Wirkung | Aufgewertet |
| --- | --- | --- | --- | --- |
| Standhalten | 1 | Verstärkung | Schilde 4, Abwehrfeuer 2. Kennkarte der Mauer | Schilde 6 |
| Schildstoß | 2 | Angriff | Schaden in Höhe der aktuellen Schilde des Flaggschiffs auf ein gewähltes Schiff. Zielsuchend | 1 BP |
| Bollwerk | 3 | Verstärkung | Schilde 8. Behalten | Schilde 11 |
| Flakbrecher | 2 | Angriff | 2× 3 Schaden auf ein gewähltes Schiff; senkt seine Panzerung dauerhaft um 1 | 2× 4 Schaden |
| Bastionsdoktrin | 3 | Doktrin | Jede gespielte Schild-Karte (`Notschilde`, `Standhalten`, `Bollwerk`) gibt zusätzlich Schilde 1 und Abwehrfeuer 1 | 2 BP |

### Vance, der Hammer (5 Admiral-Karten)

| Karte | BP | Typ | Wirkung | Aufgewertet |
| --- | --- | --- | --- | --- |
| Überladung | 0 | Verstärkung | Das Flaggschiff feuert diese Runde mit +6 ATK und nimmt am Zugende 2 Schaden. Kennkarte des Hammers | +9 ATK |
| Sturmlauf | 1 | Angriff | 4 Schaden plus doppelter Vorstoß auf ein gewähltes Schiff. Durchschlagend | 6 Schaden |
| Durchbruchsalve | 2 | Angriff | 9 Schaden auf ein gewähltes Schiff; fällt es dadurch, +1 BP und 1 Befehl ziehen | 12 Schaden |
| Brechergeschoss | 1 | Angriff | 3 Schaden auf ein gewähltes Schiff, durchschlagend. Markiert, −1 Panzerung dauerhaft | 5 Schaden |
| Offensivdoktrin | 2 | Doktrin | Immer wenn Waffen oder Fokusfeuer ausgelöst werden: Schilde 4 | Schilde 6 |

### Kerr, der Uhrmacher (5 Admiral-Karten)

| Karte | BP | Typ | Wirkung | Aufgewertet |
| --- | --- | --- | --- | --- |
| Kurzsprung | 3 | Taktik | Statt einer Station: Das Flaggschiff verlässt das Brett, ist im Gegnerzug kein Ziel und kehrt in der Auflösung zurück. Einmalig. Kennkarte des Uhrmachers | 2 BP |
| Zeitfenster | 3 | Taktik | Die nächste Karte dieser Runde wirkt zweimal. Behalten | 2 BP |
| Präzisionsmarkierung | 2 | Schwächung | Ein gewähltes Schiff wird Markiert. Befehle ziehen: 2. Behalten | 1 BP |
| Chrono-Schlag | 2 | Angriff | 7 Schaden auf ein gewähltes Schiff (14, wenn es im Stillstand ist oder auflädt). Zielsuchend, durchschlagend | 10 Schaden (20) |
| Uhrwerkdoktrin | 3 | Doktrin | Zu Rundenbeginn +1 Befehl; die erste Taktik oder Schwächung je Runde kostet 1 BP weniger | 2 BP |

### Reyes, die Glut (5 Admiral-Karten)

| Karte | BP | Typ | Wirkung | Aufgewertet |
| --- | --- | --- | --- | --- |
| Brandsatz | 1 | Angriff | 4 Schaden auf ein gewähltes Schiff, Brand. Kennkarte der Glut | 6 Schaden |
| Flächenbrand | 2 | Angriff | 2 Schaden auf jede gegnerische Einheit, Brand. Durchschlagend | 4 Schaden |
| Brandbeschleuniger | 2 | Schwächung | Ein brennendes gewähltes Schiff nimmt sofort 2× seinen Brandschaden und wird Markiert | 1 BP |
| Schmelzsalve | 2 | Angriff | 3 Schaden plus 3 je brennendem Gegner auf ein gewähltes Schiff. Zielsuchend | 5 Schaden (+4) |
| Glutdoktrin | 3 | Doktrin | Brände bei Gegnern ticken +2 stärker; fällt ein brennender Gegner, Schilde 4 | 2 BP |

Die Werte sind mit dem Simulator eingestellt ([Balance](balance.md)): Befehlspunkte sind knapper als Karten, also muss eine Beutekarte je Punkt mehr leisten als Torpedo und Notschilde, sonst verdünnt sie nur das Deck.

Doktrinen sind die Power-Karten von Slay the Spire: Sie bleiben liegen, bis das Gefecht endet.

## Startdeck und Aufwertung

Jeder Admiral beginnt mit einem **Startdeck** von zehn Karten: dreimal Torpedo, dreimal Notschilde, Reparaturtrupp, Plündern und zweimal seine Kennkarte. Alle übrigen Karten kommen als Beute; Dubletten sind erlaubt. Jede Karte hat eine **aufgewertete Fassung** für die Werft: mehr Schaden, Schild, Reparatur, ATK oder Karten, ein höherer Zweitwert oder ein Befehlspunkt weniger; was genau, steht in der Tabelle und auf der Karte.

## Kartenpools

Das Befehlsdeck umfasst **48 Karten** ohne Seltenheitsstufen: einen gemeinsamen **neutralen Sockel aus 28 Karten** sowie vier eigene **Admiral-Pools aus je 5 Karten** (die bestehende Kennkarte plus 4 Admiral-Karten). Kennkarten erscheinen damit nur noch im Pool ihres eigenen Admirals. Sowohl in der Beute nach einem Gefecht (3 Karten, mit Kampfsimulator 4) als auch im Schwarzmarkt (3 Karten) ist mindestens **1 Karte aus dem Pool des geführten Admirals** garantiert; die übrigen Plätze füllen zufällige verschiedene Karten aus dem restlichen Admiral-Pool und dem neutralen Sockel.
