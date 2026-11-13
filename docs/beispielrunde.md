# Beispielrunde

Ein Lehrbeispiel mit festen Lehrwerten: Runde 5 eines Gefechts gegen einen Belagerer mit einer Plünderer-Staffel. Die Aufstellung dient der Rechnung und ist kein Verband aus der Sektortabelle. Ich spiele den Hammer. Die Zahlen sind die des ersten Entwurfs (Vanguard 50 Hülle und 11 ATK, Torpedo 6, Überladung +5 für 3 Hülle, Fokusfeuer +2 je Treffer, Belagerer 30 Hülle und 8 ATK mit Schilden 10 nach dem Hauptgeschütz) und nicht mehr die des Spiels; sie liegen als Prüfwerte in `crates/core/tests/data/`, und `crates/core/tests/battle.rs` rechnet beide Züge damit nach. Wie gerechnet wird, gilt unverändert; die heutigen Werte stehen in [Gefecht](gefecht.md), [Admirale](admirale.md), [Karten](karten.md) und [Gegner](gegner.md).

Vanguard 38/50, Spalte Mitte, Vorstoß +2 nach zwei Runden an der Waffen-Station, also ATK 13. Hand: Torpedo, Torpedo, Notschilde, Überladung, Plündern. 3 Befehlspunkte.

Gegner: Belagerer 22/30, Front Mitte, hat in Runde 4 aufgeladen und sagt an: Hauptgeschütz, Spaltenfeuer 16 auf Spalte Mitte. Plünderer-Staffel 8/8, Back links, ungedeckt, sagt an: Zielfeuer 2 auf die Vanguard.

Der sichere Zug: Torpedo auf den Belagerer (6, Panzerung −2, also 4, er steht bei 18), Notschilde gegen die Plünderer, Manöver nach links. Das Hauptgeschütz geht ins Leere, die Vanguard bleibt bei 38, aber der Vorstoß fällt auf null, und der Belagerer geht auf Schilde, lädt wieder und sagt in zwei Runden das nächste Hauptgeschütz an.

Der Zug des Hammers: Überladung, Torpedo, dann Fokusfeuer auf den Belagerer. Die Rechnung vor dem Klick, und die Vorschau zeigt sie: Fokusfeuer mit doppeltem Vorstoß macht ATK 15, mit Überladung 20, minus 2 Panzerung sind 18. Der Belagerer steht nach dem Torpedo bei 18. Das reicht auf den Punkt: Er fällt, bevor er feuert, und die Vanguard zahlt dafür 3 Hülle an die Überladung und 4 an die Plünderer. Stünde er bei 23, fehlte ein Punkt, und auch das stünde vor dem Klick in der Vorschau; dann bliebe der sichere Zug.

| Zug | Ereignis | Ergebnis |
| --- | --- | --- |
| Spieler | Überladung | ATK diese Runde +5 |
| Spieler | Torpedo auf den Belagerer | 6, Panzerung −2: 4, Belagerer 18/30 |
| Spieler | Fokusfeuer, 11 + 4 Vorstoß + 5 Überladung | 20, Panzerung −2: 18, Belagerer zerstört, das Hauptgeschütz feuert nie |
| Spieler | Zugende: Überladung | Vanguard 35/50 |
| Gegner | Plünderer auf die Vanguard, 2, Fokusfeuer +2 | 4, Vanguard 31/50 |
| Auflösung |  | Belagerer gefallen. Vanguard 31/50, Vorstoß +3. Plünderer sagen Zielfeuer an |

Was der Bericht erzählt: Der Torpedo hat die Panzerung aufgerissen, das Fokusfeuer hat den Belagerer zerlegt, und sein Hauptgeschütz, zwei Runden lang angekündigt, ist nie losgegangen. Der sichere Zug hätte 38 Hülle behalten und dafür zwei weitere Runden Belagerer eingehandelt. Beides ist richtig; was richtiger war, entscheidet die Karte dahinter.
