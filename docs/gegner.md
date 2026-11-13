# Die Gegner

Was ein Gegner tun kann und wie Absichten gelten, steht in [Gefecht](gefecht.md#absichten-und-aktionen). Hier stehen die einzelnen Gegner, ihre Muster und die Verbände der Sektoren.

## Muster

Jeder Gegnertyp folgt einem Muster aus Zielregel und Aktionsfolge. Ein Muster darf würfeln, welchen Zweig es als Nächstes ansagt; die Ansage selbst gilt exakt. Das ist neben der Hand der einzige Zufall im Gefecht. Bisher sind alle Muster feste Folgen.

Die Tabelle nennt die Vorlage aus `assets/data/enemies_defaults.json` ([Daten](daten.md#gegner)). Im Gefecht gelten die Werte und Muster, die jede Einheit in der Kartendatei ihres Sektors trägt ([Daten](daten.md#sternenkarten)); dort können sie von Knoten zu Knoten abweichen. Die Spalte Modell nennt den Rumpf in `assets/models/ships/`. Die Verbände fliegen in Gegner-Lackierung, die Bosse haben eigene Rümpfe.

| Gegner | Klasse | Hülle | ATK | Modell | Muster |
| --- | --- | --- | --- | --- | --- |
| Plünderer-Staffel | Jäger | 8 | 2 | `void_interceptor` | Zielfeuer. Wurde sie in dieser Runde getroffen, sagt sie für die nächste Ausweichen an. |
| Brander | Jäger | 6 | 2 | `scrap_raider` | Zwei Runden Anflug mit Ausweichen, dann Rammkurs: 10 Schaden, der Brander ist weg. Fällt er vorher, geschieht nichts. |
| Torpedo-Staffel | Jäger | 8 | 4 | `lance_bomber` | Spaltenfeuer 8 auf die angesagte Spalte, dann eine Runde Ausweichen, von vorn. |
| Wachkreuzer | Kreuzer | 16 | 4 | `horizon_cruiser` | Zielfeuer, Zielfeuer, Schilde 6, von vorn. |
| Plasmakreuzer | Kreuzer | 14 | 4 | `plasma_frigate` | Zielfeuer; seine Treffer zünden den Brand schon ab 60 % Hülle. |
| Spotter | Kreuzer | 12 | 2 | `signal_booster` | Zielerfassung statt Zielfeuer. Steht in der Back-Reihe, gedeckt. |
| Belagerer | Schlachtschiff | 30 | 8 | `iron_dreadnought` | Panzerung 2. Aufladen, dann Hauptgeschütz: Spaltenfeuer mit doppelter ATK, dann Schilde 10, von vorn. |
| Trägerschiff | Schlachtschiff | 26 | 5 | `titan_carrier` | Panzerung 1. Zielfeuer; alle zwei Runden ruft es eine Plünderer-Staffel in einen freien Slot. |

## Sektoren, Elite und Bosse

| Sektor | Verbände | Elite | Boss |
| --- | --- | --- | --- |
| 1 Das Trümmerfeld | 1 bis 2 Einheiten: Plünderer, Wachkreuzer, Brander | Belagerer allein | **Der Schrottkönig.** Ein zusammengeflicktes Schlachtschiff (`scrap_king`), Hülle 60, ATK 7, mit zwei Plünderer-Staffeln als Schirm. Kannibalisieren: Jedes zerstörte Schiff auf dem Brett repariert ihn um 8. Alle drei Runden ruft er eine neue Plünderer-Staffel. |
| 2 Die Stille | 2 bis 3 Einheiten in Formation mit Deckung, erste Spotter, Torpedo-Staffeln und Belagerer | Belagerer mit zwei Wachkreuzern in der Front | **Der Wächter.** Eine automatische Verteidigungsstation (`warden`), Hülle 90, Panzerung 1, unbeweglich, zwei Wachkreuzer davor. Hauptgeschütz: zwei Runden Aufladen, dann Spaltenfeuer 30 über zwei angesagte Spalten. Dazwischen Schilde 15. Wer nicht in der dritten Spalte steht, muss dorthin manövrieren, die Aufladung brechen oder springen. |
| 3 Die Schwelle | 3 bis 4 Einheiten, Verstärkung, Spotter mit Belagerern und Trägerschiffen | Zwei Belagerer mit Spotter | **Die Gegenflotte.** Kommandantin Vey mit ihrem Flaggschiff Siegel (`seal`), Hülle 50, ATK 8, und drei Eskorten; sie bewacht das Wurmloch nach Hause. Kommandofähigkeit Riegel, angesagt wie jede Aktion: Alle ihre Einheiten schlucken 4, ohne ihre eigene Aktion zu verlieren. Fällt eine Eskorte, springt eine neue ein, zweimal. Sie ist das einzige gegnerische Flaggschiff im Run und spielt mit Absichten, nach denselben Regeln für Schaden und Zustände wie der Spieler. |

Die Gegner werden je Sektor nicht automatisch stärker: Was ein Gegner in Sektor 2 oder 3 an Hülle, ATK und Panzerung hat, steht an seinem Knoten in der Kartendatei. Die Karten geben den Gegnern ab Sektor 2 ein Viertel mehr Hülle als die Vorlage, ab Sektor 3 die Hälfte, auch Elite und Boss.

Welcher Verband an welchem Knoten steht, legen die Sternenkarten fest ([Daten](daten.md#sternenkarten)); die Tabelle ist die Vorgabe für ihren Entwurf.
