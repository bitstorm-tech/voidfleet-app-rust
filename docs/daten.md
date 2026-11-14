# Daten

Alle Zahlen, Inhalte und Texte des Spiels stehen als JSON in `assets/data/`. Im Code bleiben die Mechanik (was eine Karte, eine Station, ein Muster oder ein Modul tut), die 3D-Modelle, Animationen, Effekte und der Ton. Die Regelseiten erklären die Regeln und nennen die Startwerte; es gilt, was in den Dateien steht (für die Gegner: in den Kartendateien), und wer dort einen Wert ändert, zieht die Tabelle der Regelseite nach.

| Datei | Inhalt | Gilt ab |
| --- | --- | --- |
| `enemies_defaults.json` | Klasse jedes Gegners und die Vorlage seiner Werte und seines Musters für den Karteneditor ([Gegner](gegner.md)) | nächster Start |
| `cards/<id>.json` | je Karte eine Datei: Art, Ziel, Kosten, Wert und Zweitwert samt aufgewerteter Fassung, Name und Effekttext je Sprache ([Karten](karten.md)) | nächster Start, aus dem Befehlskarten-Editor sofort |
| `admirals.json` | Hülle, ATK, Schild-Station, Kennkarte und Startdeck jedes Admirals ([Admirale](admirale.md)) | nächster Start, aus dem Admirals-Editor sofort |
| `rules.json` | alle übrigen Stellgrößen: Gefecht, Stationen, Gegner, Module, Run | nächster Start, die Zahlen der Module aus dem Modul-Editor sofort |
| `maps/sector1.json` bis `sector3.json` | die Sternenkarte je Sektor samt Verband je Knoten, jede Einheit mit allen Werten, mit denen sie kämpft ([Der Run](run.md#die-sternenkarte)) | nächster neuer Run |
| `text/de.json`, `text/en.json` | jeder sichtbare Text außer denen der Karten, je Sprache eine Datei ([Texte](#texte)) | nach dem Neubauen |

Gegner, Karten, Admirale und Regeln liest das Spiel beim Start, die Sternenkarten bei jedem neuen Run; die Texte sind in das Programm eingebettet (`cargo run` baut nach einer Änderung von selbst neu). `cargo test -p voidfleet-core` prüft Daten und Sternenkarten: Jeder Gegner, jede Karte und jeder Admiral braucht seinen Eintrag, ein unbekanntes oder fehlendes Feld ist ein Fehler mit Dateiname und Stelle. `cargo test -p voidfleet-client i18n` prüft die Texte. Neue Gegner, Karten, Admirale oder Module entstehen nicht in den Dateien: Sie brauchen einen Namen im Code, Mechanik, Modell und Texte.

Namen in den Dateien sind die Ids des Codes in Kleinschreibung mit Unterstrich. Gegner: `raiders`, `fireship`, `torpedo_wing`, `guard_cruiser`, `plasma_cruiser`, `spotter`, `besieger`, `carrier`, `scrap_king`, `warden`, `seal`, `unknown_contact`, `brood`. Admirale: `osei`, `vance`, `kerr`, `reyes`. Karten: der Dateiname in `cards/` (`torpedo`, `heat_seeker` …).

## Gegner

```json
"warden": { "class": "battleship", "hull": 100, "attack": 17, "armor": 1, "pattern": ["fire", "charge", { "column_fire": 2 }, { "shields": 15 }] }
```

- `class`: `fighter`, `cruiser`, `battleship` oder `flagship`; sie bestimmt die Modellgruppe auf dem Brett. Sie gilt im Spiel.
- `hull`, `attack`, `armor` und `pattern` sind nur die **Vorlage**: Der Karteneditor füllt damit eine Einheit, die er neu in einen Verband setzt. Das Gefecht liest sie nie; es gelten die Werte der Einheit in der Kartendatei ([Sternenkarten](#sternenkarten)). Eine Änderung hier ändert also keine Karte.
- `hull`: Hülle. `attack`: ATK. `armor`: fester Abzug je Treffer.
- `pattern`: die Aktionsfolge, Schritt für Schritt angesagt, danach von vorn. Endet sie mit `ram`, bleibt sie dort stehen. Schritte: `"fire"` (Zielfeuer), `{ "column_fire": 1 }` (Spaltenfeuer auf so viele Spalten: die des Flaggschiffs, dann die linke Nachbarspalte, aus der linken Spalte die rechte), `"ram"`, `{ "shields": 6 }`, `"evade"`, `"charge"`, `"target_lock"`, `{ "call": <Einheit> }` (ruft diese Einheit mit allen ihren Werten, etwa `{ "call": { "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] } }`), `"bolt"` (Riegel).

Was an einem Gegner Mechanik ist, steht im Code und hängt an seinem Namen: Plünderer weichen nach einem Treffer aus, Torpedo-Staffeln feuern mit doppelter ATK, der Plasmakreuzer zündet früher, der Spotter ist Unterstützer, der Schrottkönig kannibalisiert, mit dem Siegel fällt die Gegenflotte, der Unbekannte Kontakt wächst nach, lässt sich nicht verschieben, und mit ihm fällt seine Brut.

## Karten

Je Karte eine Datei `cards/<id>.json` mit allem, was auf der Karte steht; das Bild liegt unter `images/cards/`.

```json
{
  "type": "attack",
  "target": "enemy",
  "cost": 2,
  "value": 9,
  "extra": 4,
  "upgraded": { "value": 12 },
  "text": {
    "de": { "name": "Wärmesucher", "effect": "{v} Schaden auf ein gewähltes Schiff, {evading}, wenn es ausweicht. Zielsuchend, durchschlagend." },
    "en": { "name": "Heat Seeker", "effect": "{v} damage to a chosen ship, {evading} if it is evading. Homing, piercing." }
  }
}
```

- `type`: `attack`, `buff`, `debuff`, `tactic` oder `doctrine`. `target`: `none`, `enemy` oder `column`.
- `cost`: Befehlspunkte. `value`: der Hauptwert (Schaden, Schild, Reparatur, ATK-Bonus, Panzerung), 0 bei Karten ohne Zahl.
- `extra`: der Zweitwert, nur bei diesen fünf Karten und bei ihnen Pflicht: Zuschlag des Wärmesuchers auf ein ausweichendes Ziel, des Gnadenstoßes auf eines im Notfall, Abwehrfeuer von Standhalten, Schaden der Überladung am eigenen Flaggschiff, Karten, die Plündern zieht.
- `upgraded`: was die aufgewertete Fassung anders hat, `cost`, `value` oder beides; `{}` für keine Änderung.
- `text`: je Sprache (`de`, `en`) der Name und der Effekttext; jede Sprache des Spiels ist Pflicht. Der Effekttext kennt `{v}` (Hauptwert, aufgewertet und mit Zuschlägen im Gefecht), `{extra}` (Zweitwert), `{evading}` und `{emergency}` (beide zusammen: der Wert gegen ein ausweichendes Ziel und eines im Notfall) und `{bonus}` (der Zuschlag auf den Rammkurs als „ +2“ oder nichts). Die Schlüsselwörter, die er nennt („Zielsuchend“), erklärt `keywords` in den Textdateien.

### Der Befehlskarten-Editor

Der Befehlskarten-Editor im Spiel bearbeitet die Kartendateien: Einstellungen → Tab „Dev“ → „Befehlskarten-Editor“. Links stehen alle Karten in der Farbe ihrer Art, in der Mitte die gewählte als Vorschau je Sprache (Grundfassung und aufgewertet), rechts Art, Ziel, Kosten, Wert und Zweitwert mit `-` und `+` (die aufgewertete Fassung daneben; was sie ändert, leuchtet), darunter je Sprache Name und Effekttext als Textfelder. „Speichern“ schreibt nur, was die Prüfung besteht (kein leerer Name, jeder Platzhalter im Effekt geht auf), und nur die Dateien der geänderten Karten, Zeichen für Zeichen im Format der vorhandenen; danach gelten die Karten sofort im Spiel, schon gezeigte Karten bekommen den neuen Stand, sobald sie neu entstehen. Neue Karten legt der Editor nicht an, die Bilder bleiben, wie sie sind.

## Admirale

```json
"osei": {
  "hull": 70, "attack": 7, "shield_station": 12, "signature_card": "hold_the_line",
  "deck": ["torpedo", "torpedo", "torpedo", "emergency_shields", "emergency_shields", "emergency_shields", "repair_crew", "scavenge", "hold_the_line", "hold_the_line"]
}
```

`shield_station` ist, was die Schild-Station schluckt; `signature_card` die Kennkarte, die die Admiralswahl zeigt; `deck` das Startdeck, Karte für Karte; die Kennkarte muss darin stecken, die Hülle ist mindestens 1. Flaggschiff, Passiv und Admiralsstation hängen im Code am Admiral.

### Der Admirals-Editor

Der Admirals-Editor im Spiel bearbeitet `admirals.json`: Einstellungen → Tab „Dev“ → „Admirals-Editor“. Links stehen die vier Admirale, in der Mitte der gewählte als Karte der Admiralswahl mit den Werten des Editors, rechts Hülle und ATK des Flaggschiffs und die Schild-Station mit `-` und `+`, darunter das Startdeck je Karte mit ihrer Anzahl (ein Klick auf den Namen macht sie zur Kennkarte, sie leuchtet) und alle Karten zum Hinzufügen. „Speichern“ schreibt nur, was die Prüfung besteht, Zeichen für Zeichen im Format der Datei; danach gelten die Werte sofort im Spiel für jeden neuen Run. Zahlen, die Texte aus den Daten einsetzen (`{admirals.osei.shield_station}`), folgen erst beim nächsten Start. Namen und Texte der Admirale stehen in den Textdateien, die Zahlen ihrer Admiralsstationen in `rules.json`.

## Stellgrößen

`rules.json` hat fünf Abschnitte; jeder Name sagt, was die Zahl stellt.

- `battle`: Handgröße, Befehlspunkte, Brandschaden (`ember_burn_damage` für Gegner von Reyes), die Schwellen für Brand und Notfall in Prozent der Hülle, der Beginn der Gefechtsuhr, die Reparaturgrenze je Gefecht (`repair_limit`).
- `stations`: Schadenskontrolle, die Grenze des Vorstoßes, Gegenfeuer, Fokusfeuer, Nachsetzen.
- `enemies`: Rammschaden, Riegel, Kannibalisieren, die Brandschwelle des Plasmakreuzers, die Lebende Hülle des Unbekannten Kontakts (`regeneration`: was er in einer Runde ohne Treffer nachwächst).
- `modules`: die Zahl jedes Moduls, das eine hat.
- `run`: Schrott, Angebote, Preise, Reparatur, die Zahlen der Ereignisse. Spannen stehen als `[von, bis]`, beide Enden eingeschlossen.

### Der Modul-Editor

Der Modul-Editor im Spiel bearbeitet die Zahlen der Module in `rules.json`: Einstellungen → Tab „Dev“ → „Modul-Editor“. Links stehen die neun Module, in der Mitte das gewählte als Tafel wie in Beute und Schwarzmarkt, ihr Text schon mit den Zahlen des Editors, rechts seine Zahl mit `-` und `+` (nicht unter 0) und ihr Ort in der Datei: die Felder unter `modules`, beim Kampfsimulator `run.combat_simulator_offer`. Löschanlage und Notaggregat haben keine Zahl. „Speichern“ schreibt `rules.json` Zeichen für Zeichen im Format der Datei; danach gelten die Zahlen sofort im Spiel. Die Texte der Module (`{rules.modules.salvage_crane_scrap}`) folgen erst beim nächsten Start. Was ein Modul tut, steht im Code, Namen und Texte in den Textdateien, die Preise unter `run`.

## Sternenkarten

Je Sektor eine JSON-Datei in `assets/data/maps/`: `sector1.json`, `sector2.json`, `sector3.json`. Die Abwechslung kommt aus der Gestaltung des Sektors, nicht aus mehreren Dateien.

```json
{
  "rows": [
    [
      { "kind": "battle", "next": [0, 1], "encounter": { "front": [null, { "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] }, null], "back": [null, null, null] } }
    ],
    [
      { "kind": "event", "next": [0] },
      { "kind": "shipyard", "next": [0] }
    ],
    [
      { "kind": "boss", "next": [], "encounter": { "front": [{ "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] }, null, null], "back": [{ "id": "scrap_king", "hull": 60, "attack": 7, "armor": 0, "pattern": ["fire", "fire", { "call": { "id": "raiders", "hull": 8, "attack": 2, "armor": 0, "pattern": ["fire"] } }] }, null, null] } }
    ]
  ]
}
```

- `sector`: 1 bis 3.
- `rows`: die Reihen in Flugrichtung (die erste liegt auf dem Schirm unten), in jeder die Knoten von links nach rechts. Die letzte Reihe ist genau ein Boss, sonst steht nirgends einer.
- `kind`: `battle`, `elite`, `event`, `shipyard`, `market` oder `boss`.
- `next`: die Knoten der nächsten Reihe, zu denen eine Kante führt, als Positionen ab 0, aufsteigend. Jeder Knoten hat eine Kante hinaus (außer dem Boss) und eine hinein (außer in der ersten Reihe).
- `encounter`: der Verband des Knotens, Pflicht an `battle`, `elite` und `boss`, sonst verboten. `front` und `back` sind die beiden Reihen des Bretts mit ihren drei Spalten von links nach rechts, `null` ist ein leerer Slot, sonst steht dort eine Einheit. Ein bis vier Einheiten; hinten steht nur, wer einen Vordermann hat. `back` darf fehlen.
- Eine **Einheit** nennt immer alle ihre Werte: `{ "id": "guard_cruiser", "hull": 20, "attack": 5, "armor": 0, "pattern": ["evade", { "shields": 6 }, "fire"] }`, Felder wie in `enemies_defaults.json` ohne `class`. Die Kartendatei ist die einzige Quelle dieser Werte: Nichts wird je Sektor hochgerechnet, nichts kommt aus `enemies_defaults.json`. `id` ist einer von `raiders`, `fireship`, `torpedo_wing`, `guard_cruiser`, `plasma_cruiser`, `spotter`, `besieger`, `carrier`, `scrap_king`, `warden`, `seal`, `unknown_contact` und `brood` ([Gegner](gegner.md)); an ihm hängen Modell, Klasse und die Sonderregeln (Ausweichen der Plünderer und so weiter). Hülle mindestens 1, ATK und Panzerung nicht negativ, das Muster nicht leer, Spaltenfeuer 1 bis 3 Spalten, Schilde positiv, gerufene Einheiten ebenso gültig.
- `reinforcements` im Verband (darf fehlen): angesagte Verstärkung, etwa `[{ "round": 3, "unit": <Einheit> }]`; sie trifft im Gegnerzug dieser Runde ein.
- `replacements` im Verband (darf fehlen): die Nachrücker der Gegenflotte in ihrer Reihenfolge, als Liste von Einheiten.
- `breach` im Verband (darf fehlen): der [Durchbruch](gegner.md#der-durchbruch), eine Einheit mit allen ihren Werten. Wäre das Gefecht gewonnen, trifft stattdessen sie ein, und erst ihr Fall gewinnt es. Nur der Boss von Sektor 3 trägt einen.

Das Spiel liest die Karten bei jedem neuen Run, ohne Neubauen; eine falsche Datei beendet es mit ihrem Namen und dem Fehler. Ein laufender Run behält die Karten, mit denen er begonnen hat.

### Der Karteneditor

Der Karteneditor im Spiel bearbeitet dieselben Dateien: Einstellungen → Tab „Dev“ → „Karteneditor“. Wer die Dateien lieber von Hand oder per Agent ändert, braucht ihn nicht; beide Wege schreiben dasselbe Format, der Editor Zeichen für Zeichen so wie die vorhandenen Dateien.

- **Datei**: „Sektor 1–3“ öffnet die Karte des Sektors. „Speichern“ schreibt nur eine Karte, die die Prüfung besteht, in die Datei ihres Sektors. Mit ungespeicherten Änderungen bleibt die Karte offen, bis gespeichert oder verworfen ist.
- **Karte**: Linksklick wählt einen Knoten, Rechtsklick auf einen Knoten der nächsten Reihe setzt oder löscht die Kante dorthin; dasselbe tun rechts am gewählten Knoten die Tasten „Kanten zu Reihe n“, eine je Knoten der nächsten Reihe, die leuchtet, wenn die Kante steht. Das „+“ rechts neben einer Reihe legt dort einen Knoten an, „Reihe +“ eine Reihe vor dem Boss; beide hängen das Neue so ein, dass die Karte gültig bleibt.
- **Knoten**: rechts die Art (die letzte Reihe ist der Boss und bleibt es) und „Knoten löschen“; fällt der letzte Knoten einer Reihe, fällt die Reihe. An Gefecht, Elite und Boss steht der Verband als zwei Reihen mit drei Spalten: erst ein Feld wählen, dann die Einheit aus der Palette („Leer“ räumt das Feld). „Verstärkung +“ und „Nachrücker +“ sind Ziele, an die die Palette einen neuen Eintrag hängt, „Durchbruch“ eines, dem sie seine eine Einheit setzt („Leer“ räumt sie); die Runde einer Verstärkung stellen `-` und `+`, `x` nimmt einen Eintrag heraus, ein Klick auf seinen Namen macht ihn zum Ziel (die Palette ersetzt ihn dann). Eine neue Einheit kommt mit den Werten ihrer Vorlage aus `enemies_defaults.json`.
- **Einheit**: Ist ein Feld oder Eintrag mit einer Einheit das Ziel, folgen ihre Werte und ihr Ablauf: Hülle, ATK und Panzerung mit `-` und `+` (was von der Vorlage abweicht, leuchtet; der Name trägt dann ein `*`), darunter die Schritte des Musters in ihrer Folge, je mit `-` und `+` für ihren Wert (Breite des Spaltenfeuers, Schild, gerufene Einheit) und `x` (der letzte Schritt bleibt), dann die Schritte zum Anhängen. Ein Ruf hat dazu „Werte“: Der Editor zeigt dann die gerufene Einheit, über ihr den Weg dorthin („Trägerschiff › Plünderer“) und „Zurück“. „Zurücksetzen“ gibt der gezeigten Einheit wieder Werte und Muster ihrer Vorlage.
- **Prüfung**: Die Zeile unter den Tasten nennt, was an der Karte nicht stimmt, der Knoten dazu trägt einen roten Ring. Es ist dieselbe Prüfung wie im Test.
- **Probespielen**: spielt den gewählten Knoten mit dem gewählten Admiral und seinem Startdeck im Sektor der Karte, ohne zu speichern, auch die Karte nicht. Ist der Knoten erledigt (nach der Beute, der Werft, dem Markt, dem Ereignis) oder führt der Weg über „Hauptmenü“ hinaus, steht wieder der Editor da. Ein Run, der vorher lief, bleibt gespeichert und geht im Hauptmenü weiter.

Der Editor verschiebt keine Knoten innerhalb einer Reihe und keine Schritte innerhalb eines Musters, legt keine Dateien an und ändert `enemies_defaults.json` nicht.

## Texte

`text/en.json` und `text/de.json` enthalten jeden sichtbaren Text; Englisch ist die Ausgangssprache. Beide Dateien haben dieselben Schlüssel. Ein Schlüssel heißt wie die Stelle, an der der Text steht (`hud_end_turn`, `market_title`); Gruppen fassen zusammen, was an einer Id hängt:

```json
"hud_round": "Runde {round}",
"window_mode": { "windowed": "Fenster", "maximized": "Maximiert", "fullscreen": "Vollbild" },
"checklist_points": { "one": "1 Befehlspunkt offen", "other": "{points} Befehlspunkte offen" },
"module": { "reserve_reactor": { "name": "Reservereaktor", "text": "{rules.modules.reserve_reactor_points} Befehlspunkte in Runde 1." } }
```

- `{name}` ohne Punkt ist ein Wert, den das Spiel an dieser Stelle einsetzt (Runde, Schaden, Name einer Einheit). Welche es je Text gibt, zeigt die englische Datei; ein anderer Name bleibt als `{name}` stehen, der Test meldet ihn.
- `{rules.battle.clock_start}`, `{admirals.osei.hull}`, `{cards.torpedo.value}` oder `{enemies.warden.hull}` setzen eine Zahl aus den Datendateien ein: Dateiname ohne Endung, dann der Weg zum Feld. So stimmt ein Text weiter, wenn der Wert sich ändert. Ein Pfad, den es nicht gibt, ist ein Fehler.
- `one` und `other` sind Einzahl und Mehrzahl desselben Textes.
- Die Texte der Karten stehen in ihren Dateien ([Karten](#karten)). `admiral`, `module`, `enemy`, `event` und `event_option` tragen die Texte je Id, `rules_section` die Kapitel der Spielregeln (`heading`, `paragraphs`), `keywords` die Schlüsselwörter der Karten (das `word`, wie es im Effekttext steht, und seine Erklärung).
- `radio_lines` sind die Untertitel der Funksprüche. Die Aufnahmen sind englisch, der englische Text ist das Gesprochene; je Spruch braucht jede Sprache so viele Zeilen wie es Aufnahmen gibt.

Der Test prüft: gültiges JSON, dieselben Schlüssel und dieselben Platzhalter in beiden Sprachen, ein Text je Id jeder Gruppe, kein Platzhalter, den das Spiel nicht füllt. Ein neuer Text an einer neuen Stelle braucht eine Zeile im Code (`crates/client/src/i18n/mod.rs`), eine neue Sprache eine Datei und einen Eintrag in `Language`.
