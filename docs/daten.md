# Daten

Alle Zahlen, Inhalte und Texte des Spiels stehen als JSON in `assets/data/`. Im Code bleiben die Mechanik (was eine Karte, eine Station, ein Muster oder ein Modul tut), die 3D-Modelle, Animationen, Effekte und der Ton. Die Regelseiten erklären die Regeln und nennen die Startwerte; es gilt, was in den Dateien steht, und wer dort einen Wert ändert, zieht die Tabelle der Regelseite nach.

| Datei | Inhalt | Gilt ab |
| --- | --- | --- |
| `enemies.json` | Klasse, Hülle, ATK, Panzerung und Muster jedes Gegners ([Gegner](gegner.md)) | nach dem Neubauen |
| `cards.json` | Art, Ziel, Kosten und Wert jeder Karte samt aufgewerteter Fassung ([Karten](karten.md)) | nach dem Neubauen |
| `admirals.json` | Hülle, ATK, Schild-Station, Kennkarte und Startdeck jedes Admirals ([Admirale](admirale.md)) | nach dem Neubauen |
| `rules.json` | alle übrigen Stellgrößen: Gefecht, Stationen, Zweitwerte der Karten, Gegner, Module, Run | nach dem Neubauen |
| `maps/*.json` | die Sternenkarten samt Verband je Knoten ([Der Run](run.md#die-sternenkarte)) | nächster neuer Run |
| `text/de.json`, `text/en.json` | jeder sichtbare Text, je Sprache eine Datei ([Texte](#texte)) | nach dem Neubauen |

Die ersten vier Dateien und die Texte sind in das Programm eingebettet (`cargo run` baut nach einer Änderung von selbst neu), die Sternenkarten liest das Spiel zur Laufzeit. `cargo test -p voidfleet-core` prüft Daten und Sternenkarten: Jeder Gegner, jede Karte und jeder Admiral braucht seinen Eintrag, ein unbekanntes oder fehlendes Feld ist ein Fehler mit Dateiname und Stelle. `cargo test -p voidfleet-client i18n` prüft die Texte. Neue Gegner, Karten, Admirale oder Module entstehen nicht in den Dateien: Sie brauchen einen Namen im Code, Mechanik, Modell und Texte.

Namen in den Dateien sind die Ids des Codes in Kleinschreibung mit Unterstrich. Gegner: `raiders`, `fireship`, `torpedo_wing`, `guard_cruiser`, `plasma_cruiser`, `spotter`, `besieger`, `carrier`, `scrap_king`, `warden`, `seal`. Admirale: `osei`, `vance`, `kerr`, `reyes`. Die Karten stehen als Schlüssel in `cards.json`.

## Gegner

```json
"warden": { "class": "battleship", "hull": 90, "attack": 15, "armor": 1, "pattern": ["charge", "charge", { "column_fire": 2 }, { "shields": 15 }] }
```

- `class`: `fighter`, `cruiser`, `battleship` oder `flagship`; sie bestimmt die Modellgruppe auf dem Brett.
- `hull`: Hülle vor dem Zuschlag je Sektor. `attack`: ATK. `armor`: fester Abzug je Treffer.
- `pattern`: die Aktionsfolge, Schritt für Schritt angesagt, danach von vorn. Endet sie mit `ram`, bleibt sie dort stehen. Schritte: `"fire"` (Zielfeuer), `{ "column_fire": 1 }` (Spaltenfeuer auf so viele Spalten: die des Flaggschiffs, dann die linke Nachbarspalte, aus der linken Spalte die rechte), `"ram"`, `{ "shields": 6 }`, `"evade"`, `"charge"`, `"target_lock"`, `{ "call": "raiders" }` (ruft diese Einheit), `"bolt"` (Riegel).

Was an einem Gegner Mechanik ist, steht im Code und hängt an seinem Namen: Plünderer weichen nach einem Treffer aus, Torpedo-Staffeln feuern mit doppelter ATK, der Plasmakreuzer zündet früher, der Spotter ist Unterstützer, der Schrottkönig kannibalisiert, mit dem Siegel fällt die Gegenflotte.

## Karten

```json
"torpedo": { "type": "attack", "target": "enemy", "cost": 2, "value": 6, "upgraded": { "value": 8 } },
"emp_blast": { "type": "debuff", "target": "enemy", "cost": 2, "value": 0, "upgraded": { "cost": 1 } }
```

- `type`: `attack`, `buff`, `debuff`, `tactic` oder `doctrine`. `target`: `none`, `enemy` oder `column`.
- `cost`: Befehlspunkte. `value`: der Hauptwert (Schaden, Schild, Reparatur, ATK-Bonus, Panzerung), 0 bei Karten ohne Zahl.
- `upgraded`: was die aufgewertete Fassung anders hat, `cost`, `value` oder beides; `{}` für keine Änderung.

Die Zweitwerte (etwa der Zuschlag des Wärmesuchers) stehen in `rules.json` unter `cards`.

## Admirale

```json
"osei": {
  "hull": 70, "attack": 7, "shield_station": 12, "signature_card": "hold_the_line",
  "deck": ["torpedo", "torpedo", "torpedo", "emergency_shields", "emergency_shields", "emergency_shields", "repair_crew", "scavenge", "hold_the_line", "hold_the_line"]
}
```

`shield_station` ist, was die Schild-Station schluckt; `signature_card` die Kennkarte, die die Admiralswahl zeigt; `deck` das Startdeck, Karte für Karte. Flaggschiff, Passiv und Admiralsstation hängen im Code am Admiral.

## Stellgrößen

`rules.json` hat sechs Abschnitte; jeder Name sagt, was die Zahl stellt.

- `battle`: Handgröße, Befehlspunkte, Brandschaden (`ember_burn_damage` für Gegner von Reyes), die Schwellen für Brand und Notfall in Prozent der Hülle, der Beginn der Gefechtsuhr.
- `stations`: Schadenskontrolle, die Grenze des Vorstoßes, Gegenfeuer, Fokusfeuer, Nachsetzen.
- `cards`: die Zweitwerte von Wärmesucher, Gnadenstoß, Standhalten, Überladung und Plündern.
- `enemies`: Rammschaden, Riegel, Kannibalisieren, die Brandschwelle des Plasmakreuzers.
- `modules`: die Zahl jedes Moduls, das eine hat.
- `run`: Zuschlag auf die Hülle der Gegner je Sektor, Schrott, Angebote, Preise, Reparatur, die Zahlen der Ereignisse. Spannen stehen als `[von, bis]`, beide Enden eingeschlossen.

## Sternenkarten

Je Sternenkarte eine JSON-Datei in `assets/data/maps/`; der Dateiname ist frei, mehrere Dateien mit demselben `sector` sind Varianten.

```json
{
  "sector": 1,
  "rows": [
    [
      { "kind": "battle", "next": [0, 1], "encounter": { "front": [null, "raiders", null], "back": [null, null, null] } }
    ],
    [
      { "kind": "event", "next": [0] },
      { "kind": "shipyard", "next": [0] }
    ],
    [
      { "kind": "boss", "next": [], "encounter": { "front": ["raiders", "raiders", null], "back": [null, "scrap_king", null] } }
    ]
  ]
}
```

- `sector`: 1 bis 3.
- `rows`: die Reihen in Flugrichtung (die erste liegt auf dem Schirm unten), in jeder die Knoten von links nach rechts. Die letzte Reihe ist genau ein Boss, sonst steht nirgends einer.
- `kind`: `battle`, `elite`, `event`, `shipyard`, `market` oder `boss`.
- `next`: die Knoten der nächsten Reihe, zu denen eine Kante führt, als Positionen ab 0, aufsteigend. Jeder Knoten hat eine Kante hinaus (außer dem Boss) und eine hinein (außer in der ersten Reihe).
- `encounter`: der Verband des Knotens, Pflicht an `battle`, `elite` und `boss`, sonst verboten. `front` und `back` sind die beiden Reihen des Bretts mit ihren drei Spalten von links nach rechts, `null` ist ein leerer Slot. Ein bis vier Einheiten; hinten steht nur, wer einen Vordermann hat. Die Gegner heißen `raiders`, `fireship`, `torpedo_wing`, `guard_cruiser`, `plasma_cruiser`, `spotter`, `besieger`, `carrier`, `scrap_king`, `warden` und `seal` ([Gegner](gegner.md)). `back` darf fehlen.
- `reinforcements` im Verband (darf fehlen): angesagte Verstärkung, etwa `[{ "round": 3, "unit": "guard_cruiser" }]`; sie trifft im Gegnerzug dieser Runde ein.
- `replacements` im Verband (darf fehlen): die Nachrücker der Gegenflotte in ihrer Reihenfolge, etwa `["guard_cruiser", "guard_cruiser"]`.

Das Spiel liest die Karten bei jedem neuen Run, ohne Neubauen; eine falsche Datei beendet es mit ihrem Namen und dem Fehler. Ein laufender Run behält die Karten, mit denen er begonnen hat.

### Der Karteneditor

Der Karteneditor im Spiel bearbeitet dieselben Dateien: Einstellungen → Tab „Dev“ → „Karteneditor“. Wer die Dateien lieber von Hand oder per Agent ändert, braucht ihn nicht; beide Wege schreiben dasselbe Format, der Editor Zeichen für Zeichen so wie die vorhandenen Dateien.

- **Datei**: `<` und `>` blättern durch die Karten des Ordners, „Neu“ beginnt eine Karte aus einem Gefecht und dem Boss. „Speichern“ schreibt nur eine Karte, die die Prüfung besteht; eine neue heißt `sector<n>_<x>.json` mit dem ersten freien Buchstaben. Mit ungespeicherten Änderungen bleibt die Karte offen, bis gespeichert oder verworfen ist.
- **Karte**: Linksklick wählt einen Knoten, Rechtsklick auf einen Knoten der nächsten Reihe setzt oder löscht die Kante dorthin. Das „+“ rechts neben einer Reihe legt dort einen Knoten an, „Reihe +“ eine Reihe vor dem Boss; beide hängen das Neue so ein, dass die Karte gültig bleibt. „Sektor 1–3“ stellt den Sektor.
- **Knoten**: rechts die Art (die letzte Reihe ist der Boss und bleibt es) und „Knoten löschen“; fällt der letzte Knoten einer Reihe, fällt die Reihe. An Gefecht, Elite und Boss steht der Verband als zwei Reihen mit drei Spalten: erst ein Feld wählen, dann die Einheit aus der Palette („Leer“ räumt das Feld). „Verstärkung +“ und „Nachrücker +“ sind Ziele wie ein Feld; die Runde einer Verstärkung stellen `-` und `+`, `x` nimmt einen Eintrag heraus.
- **Prüfung**: Die Zeile unter den Tasten nennt, was an der Karte nicht stimmt, der Knoten dazu trägt einen roten Ring. Es ist dieselbe Prüfung wie im Test.
- **Probespielen**: spielt den gewählten Knoten mit dem gewählten Admiral und seinem Startdeck im Sektor der Karte, ohne zu speichern, auch die Karte nicht. Ist der Knoten erledigt (nach der Beute, der Werft, dem Markt, dem Ereignis) oder führt der Weg über „Hauptmenü“ hinaus, steht wieder der Editor da. Ein Run, der vorher lief, bleibt gespeichert und geht im Hauptmenü weiter.

Der Editor verschiebt keine Knoten innerhalb einer Reihe, benennt und löscht keine Dateien und stellt keine Werte der Gegner; die stehen in `enemies.json`.

## Texte

`text/en.json` und `text/de.json` enthalten jeden sichtbaren Text; Englisch ist die Ausgangssprache. Beide Dateien haben dieselben Schlüssel. Ein Schlüssel heißt wie die Stelle, an der der Text steht (`hud_end_turn`, `market_title`); Gruppen fassen zusammen, was an einer Id hängt:

```json
"hud_round": "Runde {round}",
"window_mode": { "windowed": "Fenster", "maximized": "Maximiert", "fullscreen": "Vollbild" },
"checklist_points": { "one": "1 Befehlspunkt offen", "other": "{points} Befehlspunkte offen" },
"card": { "torpedo": { "name": "Torpedo", "effect": "{v} Schaden auf ein gewähltes Schiff. Zielsuchend." } },
"module": { "reserve_reactor": { "name": "Reservereaktor", "text": "{rules.modules.reserve_reactor_points} Befehlspunkte in Runde 1." } }
```

- `{name}` ohne Punkt ist ein Wert, den das Spiel an dieser Stelle einsetzt (Runde, Schaden, Name einer Einheit). Welche es je Text gibt, zeigt die englische Datei; ein anderer Name bleibt als `{name}` stehen, der Test meldet ihn.
- `{rules.battle.clock_start}`, `{admirals.osei.hull}`, `{cards.torpedo.value}` oder `{enemies.warden.hull}` setzen eine Zahl aus den Datendateien ein: Dateiname ohne Endung, dann der Weg zum Feld. So stimmt ein Text weiter, wenn der Wert sich ändert. Ein Pfad, den es nicht gibt, ist ein Fehler.
- `one` und `other` sind Einzahl und Mehrzahl desselben Textes.
- Kartentexte kennen `{v}` (Hauptwert der Karte, aufgewertet und mit Zuschlägen), `{evading}` und `{emergency}` (der Wert gegen ein ausweichendes Ziel und eines im Notfall) und `{bonus}` (der Zuschlag auf den Rammkurs als „ +2“ oder nichts).
- `card`, `admiral`, `module`, `enemy`, `event` und `event_option` tragen die Texte je Id, `rules_section` die Kapitel der Spielregeln (`heading`, `paragraphs`), `keywords` die Schlüsselwörter der Karten (das `word`, wie es im Effekttext steht, und seine Erklärung).
- `radio_lines` sind die Untertitel der Funksprüche. Die Aufnahmen sind englisch, der englische Text ist das Gesprochene; je Spruch braucht jede Sprache so viele Zeilen wie es Aufnahmen gibt.

Der Test prüft: gültiges JSON, dieselben Schlüssel und dieselben Platzhalter in beiden Sprachen, ein Text je Id jeder Gruppe, kein Platzhalter, den das Spiel nicht füllt. Ein neuer Text an einer neuen Stelle braucht eine Zeile im Code (`crates/client/src/i18n/mod.rs`), eine neue Sprache eine Datei und einen Eintrag in `Language`.
