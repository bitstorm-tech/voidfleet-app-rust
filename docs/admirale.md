# Die vier Admirale

Die Schlüsselwörter (Schilde, Abwehrfeuer, zielsuchend, durchschlagend) stehen in [Karten](karten.md#schlüsselwörter).

Ein Admiral ist der Charakter, den der Spieler zu Beginn des Runs wählt, wie in Slay the Spire: Flaggschiff mit Hülle, ATK und Waffe, ein **Passiv**, eine **Admiralsstation**, die das Flaggschiff statt Waffen, Schilde, Schadenskontrolle oder Manöver auslösen kann, und ein Startdeck mit zwei **Kennkarten** ([Karten](karten.md#startdeck-und-aufwertung)). Dass die Admiralsstation mit den Waffen oder dem Schutz des eigenen Schiffs konkurriert, ist die Entscheidung jedes Admirals in jeder Runde.

Hülle, ATK und Startdeck stehen in `assets/data/admirals.json` ([Daten](daten.md#admirale)), die Tabelle nennt die Startwerte.

| Admiral | Flaggschiff | Hülle | ATK | Waffe | Passiv | Admiralsstation | Kennkarte |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Osei, die Mauer | Bastion | 70 | 7 | Flakbatterien | Wächterschild: Schilde schlucken 12 statt 8 | Gegenfeuer | Standhalten |
| Vance, der Hammer | Vanguard | 50 | 11 | Schwere Geschütze | Vorstoß: +1 ATK je Runde in Folge mit Waffen oder Fokusfeuer, höchstens +4; jede andere Station setzt zurück | Fokusfeuer | Überladung |
| Kerr, der Uhrmacher | Pendulum | 56 | 8 | Railgun: zielsuchend, durchschlagend | Uhrwerk: sieht die Absichten der Gegner zwei Runden voraus | Stillstand | Kurzsprung |
| Reyes, die Glut | Pyre | 56 | 9 | Plasmawerfer: jeder Treffer setzt das Ziel in Brand | Glutkern: Brände bei Gegnern ticken 3 statt 2 | Nachsetzen | Brandsatz |

**Osei, die Mauer.** Admiralsstation **Gegenfeuer**: Die Bastion feuert nicht: Schilde 6, Abwehrfeuer 3. Gegen einen Verband, der mit allem auf sie feuert, ist das die beste Salve, die sie hat, und sie kostet keinen Schuss, den sie nicht ohnehin verliert. Ihr Dilemma: Gegenfeuer lebt davon, getroffen zu werden, und die Hülle ist die Währung des Runs. Spielt sich wie ein Panzer, der die Flotte an sich zerschellen lässt; schwach gegen Belagerer, deren Panzerung ihre 7 ATK auf 5 drückt, und gegen Spaltenfeuer, das sie lieber umfliegt.

**Vance, der Hammer.** Admiralsstation **Fokusfeuer**: Die Vanguard feuert auf ein Ziel mit doppeltem Vorstoß, zielsuchend; dafür nimmt sie im folgenden Gegnerzug je Treffer 2 mehr. Der Hammer will nichts als feuern: Jede Runde Waffen macht ihn stärker, jede Runde Schilde setzt ihn zurück. Sein Dilemma ist die Schwelle, ab der er nicht mehr ausweichen darf, weil der Vorstoß zu wertvoll ist. Seine Schwäche ist die dünne Hülle: Wer ihn an die Schilde zwingt, setzt den Vorstoß zurück.

**Kerr, der Uhrmacher.** Admiralsstation **Stillstand**: Die Pendulum feuert nicht. Eine gewählte gegnerische Einheit handelt in dieser Runde nicht; ihre Absicht verschiebt sich auf die nächste Runde, eine Aufladung bleibt erhalten. Mit dem Uhrwerk sieht Kerr zwei Runden voraus, und mit der Railgun trifft er, wen er will, durch Deckung und Ausweichen hindurch. Sein Dilemma: Stillstand verschiebt die Gefahr nur, und jede Runde Stillstand ist eine Runde ohne Railgun, während die Gefechtsuhr läuft. Der Uhrmacher ist der Admiral, der das Hauptgeschütz um eine Runde verschiebt, in der seine Angriffskarten den Belagerer fällen.

**Reyes, die Glut.** Admiralsstation **Nachsetzen**: Die Pyre feuert nicht. Jeder brennende Gegner nimmt sofort das Doppelte seines Brandschadens, und Angriffskarten auf brennende Ziele machen nach dem Auslösen +2 Schaden, für den Rest der Runde. Mit dem Plasmawerfer brennt alles, was sie trifft; drei brennende Gegner und Nachsetzen sind 18 Schaden ohne einen Schuss. Ihr Dilemma: Früh brennt nur, was der Plasmawerfer oder ein Brandsatz trifft, ein Ziel je Schuss, und Nachsetzen braucht eine Runde ohne Waffen. Gegen einzelne Schlachtschiffe ist sie die Schwächste, gegen Flotten die Beste.

Was die vier unterscheidet, in einem Satz: Die Mauer gewinnt, indem sie aushält, der Hammer, indem er nicht aufhört, der Uhrmacher, indem er die Uhr anhält, die Glut, indem sie wartet.
