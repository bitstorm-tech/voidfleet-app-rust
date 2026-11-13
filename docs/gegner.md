# Die Gegner

Was ein Gegner tun kann und wie Absichten gelten, steht in [Gefecht](gefecht.md#absichten-und-aktionen). Hier stehen die einzelnen Gegner, ihre Muster und die Verbände der Sektoren.

## Muster

Jeder Gegnertyp folgt einem Muster aus Zielregel und Aktionsfolge. Ein Muster darf würfeln, welchen Zweig es als Nächstes ansagt; die Ansage selbst gilt exakt. Das ist neben der Hand der einzige Zufall im Gefecht. Bisher sind alle Muster feste Folgen.

Die Tabelle nennt die Vorlage aus `assets/data/enemies_defaults.json` ([Daten](daten.md#gegner)); sie trägt die Werte von Sektor 1. Im Gefecht gelten die Werte und Muster, die jede Einheit in der Kartendatei ihres Sektors trägt ([Daten](daten.md#sternenkarten)); dort können sie von Knoten zu Knoten abweichen. Die Spalte Modell nennt den Rumpf in `assets/models/ships/`. Die Verbände fliegen in Gegner-Lackierung, die Bosse haben eigene Rümpfe.

| Gegner | Klasse | Hülle | ATK | Modell | Muster |
| --- | --- | --- | --- | --- | --- |
| Plünderer-Staffel | Jäger | 7 | 6 | `void_interceptor` | Zielfeuer. Wurde sie in dieser Runde getroffen, sagt sie für die nächste Ausweichen an. |
| Brander | Jäger | 6 | 2 | `scrap_raider` | Zwei Runden Anflug mit Ausweichen, dann Rammkurs: 12 Schaden, der Brander ist weg. Fällt er vorher, geschieht nichts. |
| Torpedo-Staffel | Jäger | 10 | 7 | `lance_bomber` | Spaltenfeuer 14 auf die angesagte Spalte, dann eine Runde Ausweichen, von vorn. |
| Wachkreuzer | Kreuzer | 16 | 9 | `horizon_cruiser` | Zielfeuer, Zielfeuer, Schilde 6, von vorn. |
| Plasmakreuzer | Kreuzer | 14 | 8 | `plasma_frigate` | Zielfeuer; seine Treffer zünden den Brand schon ab 60 % Hülle. |
| Spotter | Kreuzer | 12 | 2 | `signal_booster` | Zielerfassung statt Zielfeuer. Steht in der Back-Reihe, gedeckt. |
| Belagerer | Schlachtschiff | 30 | 13 | `iron_dreadnought` | Panzerung 2. Aufladen, dann Hauptgeschütz: Spaltenfeuer mit doppelter ATK, dann Zielfeuer, von vorn. |
| Trägerschiff | Schlachtschiff | 28 | 9 | `titan_carrier` | Panzerung 1. Zielfeuer; alle zwei Runden ruft es eine Plünderer-Staffel in einen freien Slot. |

Die Gegner sind zerbrechlich und schlagen hart zu: Ein Verband, der eine Runde lang vollzählig feuert, kostet mehr, als Schild-Station und Notschilde schlucken, und jede Einheit, die im Spielerzug fällt, nimmt ihren Schaden mit. So zählt der Angriff, nicht nur die Deckung. Der Belagerer feuert nach dem Hauptgeschütz weiter, statt sich hinter Schilden zu verstecken: Wer ihm nur ausweicht, hat ihn nicht entschärft.

## Sektoren, Elite und Bosse

Was eine Einheit je Sektor an Hülle und ATK hat (Panzerung wie in der Vorlage, Brander und Spotter mit ATK 2):

| Sektor | Plünderer | Brander | Torpedo-Staffel | Wachkreuzer | Plasmakreuzer | Spotter | Belagerer | Trägerschiff |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 Das Trümmerfeld | 7 / 6 | 6 | – | 16 / 9 | – | – | 30 / 13 | – |
| 2 Die Stille | 9 / 8 | 8 | 10 / 9 | 20 / 11 | 16 / 10 | 14 | 36 / 16 | 30 / 10 |
| 3 Die Schwelle | 11 / 8 | – | 12 / 9 | 24 / 11 | 18 / 10 | 16 | 42 / 16 | 34 / 10 |

Die ATK wächst von Sektor 1 zu 2 und dann kaum noch, die Hülle weiter: Die Verteidigung des Spielers wächst im Run wenig, sein Schaden deutlich. Sektor 3 wird schwerer durch größere Verbände, Spotter und Verstärkung, nicht durch höhere Zahlen je Schuss.

| Sektor | Verbände | Elite | Boss |
| --- | --- | --- | --- |
| 1 Das Trümmerfeld | 3 bis 4 Einheiten: Plünderer, Wachkreuzer, Brander. Die erste Reihe ist ein Auftakt ohne Verlust, ab Reihe 3 stehen zwei Wachkreuzer mit Begleitung | Belagerer zwischen zwei Wachkreuzern, dahinter Plünderer | **Der Schrottkönig.** Ein zusammengeflicktes Schlachtschiff (`scrap_king`), Hülle 70, ATK 15, mit zwei Plünderer-Staffeln als Schirm. Kannibalisieren: Jedes zerstörte Schiff auf dem Brett repariert ihn um 8. Alle drei Runden ruft er eine neue Plünderer-Staffel. |
| 2 Die Stille | 3 bis 4 Einheiten in Formation mit Deckung: Plasmakreuzer, Torpedo-Staffeln, Spotter, Belagerer und das erste Trägerschiff | Belagerer zwischen zwei Wachkreuzern, dahinter ein Spotter | **Der Wächter.** Eine automatische Verteidigungsstation (`warden`), Hülle 100, ATK 17, Panzerung 1, unbeweglich, zwei Wachkreuzer davor. Zielfeuer, Aufladen, dann das Hauptgeschütz: Spaltenfeuer 34 über zwei angesagte Spalten, danach Schilde 15. Wer nicht in der dritten Spalte steht, muss dorthin manövrieren, die Aufladung brechen oder springen. |
| 3 Die Schwelle | 3 bis 4 Einheiten, dazu Verstärkung, Spotter mit Belagerern und Trägerschiffen | Zwei Belagerer mit Spotter, dazwischen ein Kreuzer, und ein weiterer Kreuzer als Verstärkung in Runde 3 | **Die Gegenflotte.** Kommandantin Vey mit ihrem Flaggschiff Siegel (`seal`), Hülle 70, ATK 11, mit Plasmakreuzer, Wachkreuzer und Spotter; sie bewacht das Wurmloch nach Hause. Kommandofähigkeit Riegel, angesagt wie jede Aktion: Alle ihre Einheiten schlucken 4, ohne ihre eigene Aktion zu verlieren. Fällt eine Eskorte, springt eine neue ein, erst ein Wachkreuzer, dann eine Plünderer-Staffel. Sie ist das einzige gegnerische Flaggschiff im Run und spielt mit Absichten, nach denselben Regeln für Schaden und Zustände wie der Spieler. |

Wie schwer ein Verband ist, hängt vor allem daran, wie viele Einheiten in derselben Runde Zielfeuer geben: Drei Schützen kosten wenig, vier viel. Spaltenfeuer allein kostet nur die Station für das Manöver; gefährlich wird es neben Zielfeuer, weil dann Schilde und Manöver um dieselbe Station streiten. Was jeder Knoten im Simulator kostet, steht in [Balance](balance.md).

Welcher Verband an welchem Knoten steht, legen die Sternenkarten fest ([Daten](daten.md#sternenkarten)); die Tabelle ist die Vorgabe für ihren Entwurf.
