# Simulator (`voidfleet-sim`)

- `just sim [runs|cards|modules|trace] [-n <runs je Admiral>] [--admiral <id>] [--seed <n>] [--data <ordner>]`; immer im Release-Build (der Dev-Build ist zehnmal langsamer). Rund 50 Runs je Sekunde auf acht Kernen; `cards` spielt 27-mal so viele Runs wie `runs`. Was die Zahlen bedeuten und welche Ziele gelten, steht in `docs/balance.md`.
- Abhängigkeiten nur `voidfleet-core`, `serde`, `serde_json` und `rand`. Der Simulator ändert nichts am Regelwerk: Er spielt `Battle` und `Run` über dieselbe Schnittstelle wie der Client.
- `--data <ordner>` nimmt eine Kopie von `assets/data` (samt `maps/`), um Zahlen zu probieren, ohne die des Spiels anzufassen.

## Aufbau

- `src/bot.rs` — der Bot. Im Gefecht probiert `plan` jede Folge aus Karten und Stationen des Zugs auf einer Kopie aus (Tiefensuche, gleiche Stellungen über `key` nur einmal, höchstens `NODES` Stellungen und `STEPS` Schritte), lässt den Gegnerzug laufen und bewertet mit `eval`: Hülle minus `THREAT` mal den geschätzten Schaden bis zum Gefechtsende (`threat`: die Gegner fallen in der Reihenfolge Schaden je Hülle, jeder feuert nach seinem Muster, bis er fällt; Schildkarten schlucken je Runde, was das Deck im Schnitt hergibt). `THREAT` ist geeicht: der Wert, mit dem der Bot am häufigsten gewinnt; nach großen Regeländerungen neu suchen. Im Run entscheidet `play_run` nach festen Regeln: Beute zufällig (eine Karte oder keine, ein Modul immer), Werft repariert unter 60 % Hülle und wertet sonst auf, angeschlagen um die Elite herum und zur Werft, im Ereignis die Logbücher. `Start` gibt einem Run zusätzliche Karten oder Module mit, `RunLog` ist, was er ergeben hat.
- `src/main.rs` — die Modi und ihre Tabellen: `simulate` spielt auf allen Kernen dieselben Seeds, `report_admirals`, `report_nodes`, `report_cards`, `compare` (mit und ohne eine Karte oder ein Modul).

## Grenzen

- Der Bot rechnet einen Zug genau und den Rest geschätzt. Karten, deren Wert erst über Runden entsteht oder an der Aufstellung hängt (Traktorstrahl, EMP-Stoß, Ausweichmanöver), spielt er selten und misst sie deshalb eher zu schwach.
- Er wählt Beute zufällig. Ein Spieler, der sein Deck baut, gewinnt öfter; ein Spieler, der schlechter rechnet, seltener.
- Plündern zieht auf der Kopie dieselben Karten wie im Gefecht, der Bot kennt sie also vor dem Spielen.
