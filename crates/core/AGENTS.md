# Regelwerk (`voidfleet-core`)

- `cargo test -p voidfleet-core`, `cargo clippy -p voidfleet-core --all-targets -- -D warnings`. Abhängigkeiten nur `serde` und `rand`; kein I/O, keine Texte.
- Die Regeln stehen in `docs/PVE_DESIGN.md` (Abschnitte 2 bis 12); dieses Crate setzt das Gefecht und den Run (Abschnitte 11 und 12) um; der Client zeichnet, speichert und spielt ihn ab.
- Ids sind Enums und serialisieren als `snake_case`; der Client leitet daraus Dateinamen ab und hält Namen und Effekttexte.

## Aufbau

- `src/board.rs` — `Row` (Front, Back), `COLUMNS` (3), `EnemySlot` (`ALL` in Handlungs- und Ankunftsreihenfolge: Front vor Back, links nach rechts; `front` ist der deckende Slot), `MAX_ENEMIES` (4).
- `src/cards.rs` — die 26 Karten (`CardId`) mit `card_type`, `target` (`CardTarget`: keins, gegnerische Einheit, Spalte), `cost(upgraded)` und `value(upgraded)` (Hauptwert: Schaden, Schild, Reparatur, ATK-Bonus; aufgewertet +2 bei Angriff, Notschilde, Standhalten, Reparaturtrupp, Alarmbereitschaft, sonst 1 BP weniger). Zweitwerte und feste Zahlen als Konstanten (Wärmesucher +3, Gnadenstoß +5, Standhalten 2, Überladung 3, Plündern 2). `DeckCard` (Id und Aufwertung), `starter_deck(admiral)`, `LOOT_POOL`.
- `src/admirals.rs` — `AdmiralId` (Osei, Vance, Kerr, Reyes) mit `flagship`, `hull`, `attack`, `signature_card`, `station` (`AdmiralStation`) und `shield_station`; `Station`; die Zahlen der Stationen als Konstanten.
- `src/enemies.rs` — `EnemyId` mit `class`, `hull`, `attack`, `armor` und dem Muster `action(step, column)`: die Aktion im Schritt `step`, Spaltenfeuer auf die Spalte des Flaggschiffs bei der Ansage (Wächter dazu die linke Nachbarspalte, aus Spalte 0 die rechte). `Action` ist eine Ansage.
- `src/modules.rs` — `ModuleId` und die Zahlen der Module.
- `src/events.rs` — `BattleEvent` in Ausführungsreihenfolge; Einheiten als `UnitRef` samt Id und Slot, Treffer mit `Breakdown` (jede Zahl der Rechnung) und `UnitStats` danach. `Hit` ist ein Treffer der Vorschau, `Intent` eine Ansage samt live gerechnetem Schaden.
- `src/battle.rs` — `Battle`, die eine Schnittstelle für den Client. Der Zustand liegt in öffentlichen Feldern (`flagship`, `enemies`, `hand`, Stapel, `points`, `kills`, `result` …), abgeleitete Werte über Methoden (`flagship_attack`, `flagship_armor`, `enemy_attack`, `announced`, `covered`, `clock`). Spielerzug: `place`, `toggle_station`/`set_target`/`set_column`, `can_run`, `play_card`, `end_turn` (führt zuerst die besetzten Stationen aus); Vorschau `preview_stations`/`preview_card` spielt auf einer Kopie, rechnet also genau wie die Ausführung.
- `src/run.rs` — `Run`, der Run aus Sternenkarte, Verbänden, Beute, Werft, Schwarzmarkt und Ereignissen (Abschnitt „Run“ unten).
- `tests/battle.rs` — die Beispielrunde aus Abschnitt 10 (beide Züge) und je ein Test für die übrigen Regeln. Zustände werden über `Battle::new` und direkte Felder aufgesetzt.

## Eine Runde

1. **Spielerzug** (das Flaggschiff beginnt jedes Gefecht in der mittleren Spalte): Karten wirken sofort, die besetzten Stationen (`stations`) laufen beim Zugende in der Reihenfolge der Wahl. Eine Karte kostet Befehlspunkte (3, mit Reservereaktor in Runde 1 4), geht auf den Ablagestapel, Doktrinen bleiben bis zum Gefechtsende liegen (`doctrines`). Eine Station je Runde (Alarmstufe Rot: `station_slots` +1; sind alle Plätze belegt, weicht die zuerst gewählte), nicht im Kurzsprung, die Admiralsstation nicht im Notfall.
2. **Zugende** (`end_turn`): Lief noch keine Station, läuft die eingestellte, wenn sie kann. Dann der Schaden der Überladung.
3. **Gegnerzug**: Schilde und Ausweichen der Gegner aus dem letzten Gegnerzug enden, Alarmbereitschaft gibt Schild. Fällige Verstärkung trifft im ersten freien Slot ein (bei vier Einheiten entfällt sie), sagt an und handelt erst nächste Runde. Dann jede Einheit nach `intents[0]`, Front vor Back, links nach rechts; Stillstand und Funkstille lassen sie aussetzen. Zielfeuer und Spaltenfeuer verbrauchen die Aufladung; Kurzsprung lässt alles ins Leere gehen, Manöver nur Spaltenfeuer auf andere Spalten. Treffer am Flaggschiff: Gegenmaßnahmen halbieren, Panzerung, Schild, Hülle; danach Gegenfeuer und Standhalten am Schützen. Fällt das Flaggschiff (nach dem Notaggregat), endet das Gefecht sofort.
4. **Auflösung**: Brände ticken (Flaggschiff 2, außer mit Löschanlage; Gegner 2, bei Reyes 3), der Rundenzustand des Flaggschiffs verfällt, der Kurzsprung kehrt zurück, Vorstoß (Vance: +1 bis 4, wenn nur Waffen oder Fokusfeuer liefen, sonst 0). Runde +1, Gefechtsuhr (ab Runde 8 `runde − 7` ATK für alle Gegner). Hand abwerfen, 5 nachziehen (Nachschubkapsel 6, leerer Stapel: Ablage mischen), Befehlspunkte 3. Jede Einheit rückt ihre Ansagen nach (zwei tief), außer nach Stillstand oder Ankunft; getroffene Plünderer sagen Ausweichen an.

## Schaden

Schuss auf einen Gegner (`shoot`): Ausweichen halbiert (aufgerundet, nicht im Notfall), Panzerung zieht ab, Deckung leitet die aufgerundete Hälfte an den Vordermann (eigener Treffer), Schild schluckt, Rest auf die Hülle. Brand bei Hülle ≤ 50 % nach einem Treffer, durch Plasmawerfer und Brandsatz; Notfall bei ≤ 25 % (ATK aufgerundet halbiert). Mit Hülle 0 fällt eine Einheit sofort: Abschuss zählen, Ziel leeren, Schrottkönige heilen 8, ein Nachrücker übernimmt den Slot, Sieg prüfen (keine Einheit, keine Verstärkung, kein Nachrücker; oder das Siegel fällt). Gegnerschüsse: `enemy_attack` (Grundwert + Gefechtsuhr + Spotter − Zielstörer, im Notfall halbiert), Zielfeuer einfach, Spaltenfeuer doppelt bei Torpedo-Staffel oder Aufladung, Aufladung verdoppelt auch Zielfeuer, Rammkurs fest 10; nach Fokusfeuer +2 je Treffer.

## Run

- **Zustand** in öffentlichen Feldern von `Run` (Serde, der Client speichert ihn als JSON): Admiral, `hull`/`max_hull` (samt Verstärkter Spanten), `scrap`, `deck`, `modules`, `emergency_generator_used`, `sector` (1 bis 3), `map` (Reihen × Knoten, `MapNode { kind, next }`, Reihe und Spalte sind die Indizes, `next` die Spalten der nächsten Reihe), `path` (besuchte Spalte je Reihe, der letzte ist der aktuelle Knoten), `stage` (was ansteht), `burning` (nächstes Gefecht brennend), `pilot` (nächster Knoten wird Werft), `result`.
- **Zufall**: kein RNG im Zustand. Jede Entscheidung zieht `StdRng::seed_from_u64(seed ^ rolls * 0x9E37_79B9_7F4A_7C15)` und zählt `rolls` hoch; gleicher Seed und gleiche Wahl ergeben denselben Run, auch nach dem Laden. Der Seed des Gefechts steht in `Stage::Battle`, also baut `start_battle(&self)` nach dem Laden dasselbe Gefecht.
- **Ablauf**: `Stage::Map` → `advance(column)` (nur `choices()`) → je nach `NodeKind`:
  - `Battle`/`Elite`/`Boss`: `Stage::Battle { encounter, seed }` → `start_battle` → Gefecht → `finish_battle(&battle) -> Loot` (Hülle, Notaggregat, Schrott sofort verbucht; Boss nach Sektor 1 und 2 repariert wie die Werft) → `Stage::Loot` → `take_loot(Option<index>)` (Karte bei Gefechten, Modul bei Elite und Boss, `None` nimmt nichts).
  - `Event`: `Stage::Event(EventId)` → `choose(index in EventId::options()) -> Gain`; „Selbst fliegen“ führt in ein Kartenangebot (`Stage::Loot`).
  - `Shipyard`: `repair_at_shipyard`, `upgrade_at_shipyard(deck_index)` oder `remove_at_shipyard(deck_index)`.
  - `Market`: `buy_card`, `buy_module`, `buy_removal(deck_index)` (einmal), `buy_repair` (beliebig oft), `leave_market`.
  - Danach zurück auf `Stage::Map`; nach dem Boss beginnt der nächste Sektor mit neuer Karte und leerem `path`. Der Boss von Sektor 3 setzt `result: Won`, eine Niederlage `Lost`.
- **Fehler**: `RunError` (falsche Phase, nicht erreichbarer Knoten, zu wenig Schrott, Index außerhalb, schon aufgewertet, ausverkauft).
- **Zahlen** als Konstanten oben in `run.rs` (Reihen, Schrott, Preise, Reparatur, Ereignisse, Sektor-Skalierung), die Anteile der Knoten in `generate_map`, die Verbände und die festen Aufstellungen von Elite und Bossen in `encounter`, die Aufstellungsregel in `formation` (Jäger und Schlachtschiffe vorn, Spotter hinten, Kreuzer hinter Jägern, hinten nur gedeckt, Mitte zuerst).
- `tests/run.rs` — Kartenerzeugung, Verbände und Aufstellungen, Beute, Werft, Schwarzmarkt, die drei Ereignisse, Sektorwechsel und Sieg, Determinismus, Serde-Roundtrip und ein durchgespielter Run.
