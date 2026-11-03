# TODO aus dem Audit (2026-11-02)

## Must fix

- [ ] **1. Railgun-Abschuss eines Flaggschiffs beendet die Partie nicht** — `crates/core/src/resolution.rs:1174-1176`
  Nach der Railgun fehlt der Flaggschiff-Check: Die Railgun zerstört die Vanguard, die schießt trotzdem noch, die Pendulum
  fällt → Unentschieden statt Sieg Kerr (nachgestellt).
  Fix: nach der Railgun-Schleife `if flagship_destroyed(board) { return; }`, dazu ein Test nach dem Muster von
  `faellt_ein_flaggschiff_durch_taktvorsprung_…`.
- [ ] **2. PvP: Jede Aktion des Gegners löscht die angefangene Auswahl** — `crates/client/src/controller/mod.rs:220-233`
  `follow_state` setzt bei jedem Zustand `Selection`/`Notices` zurück; der Server schickt nach jeder Aktion beider
  Spieler einen Zustand.
  Fix: nur zurücksetzen, wenn Runde oder Admiral sich geändert haben oder der Zug nicht mehr offen ist (`before`).
- [ ] **3. Keine Wiederverbindung; Neustart sperrt aus dem Match aus** — `crates/client/src/net/launch.rs:82`, `crates/client/src/net/mod.rs`
  Nach einem Verbindungsabbruch bleibt der Client getrennt; nach einem Neustart gibt es ein neues Token → `match_full`.
  Fix: bei `ConnectionLost` mit Backoff `Connection::open(server.endpoint.url())` erneut (gleiches Token, ~10 Zeilen).

## Should fix

- [ ] **4. Server löscht keine Räume** — `crates/server/src/server.rs:139-150`
  Jede Match-Id (auch ausgedachte, auch abgelehnte Beitritte) bleibt für immer im Speicher.
  Fix: in `leave` den Raum entfernen, wenn keine Verbindung mehr offen ist und das Match vorbei ist oder nie begann.
- [ ] **5. Mausrad-Zoom schreibt in jedem Bild `settings.json`** — `crates/client/src/scene/camera.rs:680-691`
  Fix: auch früh abbrechen, solange `zoom_offset` noch nicht bei `zoom_target` ist.
- [ ] **6. Ein kaputtes Feld löscht alle Einstellungen** — `crates/client/src/settings.rs:144-161`
  Parse-Fehler → Standardwerte → nächste Änderung überschreibt die Datei; `fs::write` ist nicht atomar.
  Fix: über `settings.json.tmp` + `fs::rename` schreiben; unlesbare Datei vorher nach `.bad` umbenennen.
- [ ] **7. „Neues Match“ nimmt Reste mit** — `crates/client/src/controller/mod.rs:504-518`, `crates/client/src/ui/hud.rs:293-313`
  Rundenprotokoll (`LogText`) bleibt und wächst; `ResultReport` spielt „Sieg/Niederlage“ über der Admiralswahl.
  Fix: in `start_new_match` `LogText` leeren und `commands.remove_resource::<ResultReport>()`.
- [ ] **8. Verbindungsverlust während der Wiedergabe lässt das Brett in `Playback` hängen** — `crates/client/src/controller/mod.rs:395-399, 566-580`
  Fix: in `finish_playback` direkt zur Planung, wenn die Verbindung schon getrennt ist.
- [ ] **9. Bot: `unplayed` wird für die Schrottverwertung negativ** — `crates/server/src/bot/mod.rs:394-402`
  Verzerrt `hostile_chance` (z. B. 10/17 statt 10/20). Fix: `.max(0)`.
- [ ] **10. Simulation zählt „auf der Hand“ je Kopie statt je Runde** — `crates/server/src/bin/simulate.rs:747-749`
  Betrifft Torpedo, Sperrfeuer, Schrottverwertung. Fix: jede Id einmal je Runde zählen (`HashSet`).
- [ ] **11. Ghosts flackern bei jeder Mausbewegung** — `crates/client/src/scene/sync.rs:186-256`
  Eine Abweichung → alle Ghosts neu geladen. Fix: je `(Slot, Id)` diffen; der Sonderfall `same` entfällt.
- [ ] **12. Tooltips bleiben nach dem Sprachwechsel alt** — `crates/client/src/ui/hud.rs:252-257`, `crates/client/src/ui/view_toggle.rs:15-17`, `crates/client/src/ui/power_card.rs:152,166`
  Fix: die vier Tooltips in `follow_locale` (`crates/client/src/ui/dialogs/mod.rs:250`) mit neu beschriften.
- [ ] **13. Rundenvorschau wird je Änderung zweimal berechnet** — `crates/client/src/controller/mod.rs:844`, `crates/client/src/scene/sync.rs:178`
  Fix: einmal in `update_focus` berechnen, in `BoardFocus` ablegen, HUD und Szene lesen dort.

## Nice to have (Lean, zusammen ~160 Zeilen)

- [ ] **14. Toten Code löschen (~55 Zeilen)**: `Board::lane_switch_destinations`; i18n-Schlüssel `hud_under_fire`,
  `settings_install`, `settings_install_button`, `figures_round`, `figures_maneuver` (samt `de.rs`/`en.rs`);
  `card_name` (nur im Test); `Animation::duration`; `Key::ICON`.
- [ ] **15. Fast gleiche Bot-Helfer zusammenlegen (~38 Zeilen)**: `take_cover` → `Situation::shelter`;
  `escape_route` (`threat.rs:112-141`) und `Situation::evasions` (`maneuver.rs:157-175`) → eine `evasions`;
  `covering` (`threat.rs:95`) und `Situation::uncovers` (`situation.rs:335`) → eine `uncovers`;
  `lowest` als `highest` mit `Reverse` (`bot/mod.rs:289-317`).
- [ ] **16. Pass-throughs und Methoden nur für Tests im Client (~28 Zeilen)**: `to_turn_plan` → `game.plan().clone()`;
  `try_plan`, `planned_ships` (nur Tests) → `game.clone()`; `ServerInbox::hold`/`release` → `held` direkt setzen.
- [ ] **17. Simulation: „erstes Flaggschiff fällt“ streichen (~10 Zeilen)** — `crates/server/src/bin/simulate.rs:258, 726, 786-798, 1086`
  Ist immer die Ø-Rundenzahl der entschiedenen Partien.
- [ ] **18. Doppelte Dev-Szenen (~19 Zeilen)**: `staged_match` (`crates/client/src/dev/scenes.rs:120-140`) und
  `Showcase::staged` (`crates/client/src/dev/commands.rs:118-138`) → ein Helfer mit `submitted`;
  `toggle_view`-Kopie in `stage_demo` (`crates/client/src/dev/mod.rs:155-157`) entfernen.
- [ ] **19. HUD: dreifache Abbruch-Bedingung (~9 Zeilen)** — `crates/client/src/ui/hud.rs:325, 357, 486`
  Ein gemeinsames `run_if(resource_changed::<Match>.or(…))`.

## Nicht geprüft

Client nicht mit Fenster gestartet. Nur überflogen: `scene/{maneuver,paint,fx,coords}.rs`, `scene/toon/`, die Dialoge,
`dev/autoplay.rs`, `audio/{sample,ui_sounds}.rs`, `data/{backgrounds,ship_models,sounds,radio}.rs`,
`tools/rasterize/src/figures.rs`, die Testdateien.
