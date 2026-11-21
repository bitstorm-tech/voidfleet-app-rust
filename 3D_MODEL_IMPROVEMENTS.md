# Modell-Verbesserungen (3D-Schiffe & Pipeline)

Bestandsaufnahme und konkrete Ansatzpunkte zur Verbesserung der 3D-Schiffsmodelle in `tools/ships/` sowie der zugehörigen Textur- und Shader-Pipeline (`assets/shaders/toon.wesl`).

Die bestehende Pipeline ist bereits auf einem hohen Niveau: Alle 15 im Spiel geladenen Schiffe sind in der Fassung für die Nahaufnahme gebaut (Fasen, geglättete Rundteile, Kleinteile `details`, Feinteile `fine` bis ~34k Dreiecke), tragen gebackene Verwitterungsmasken (`weather.py`), triplanare Metall-Feinstruktur (`detail.py`) und automatische Wracks (`wreck.py`); der Unbekannte Kontakt und seine Brut nutzen ein eigenes organisches System samt prozeduraler Haut (`unknown_contact.py`, `skin.py`).

Die folgenden Punkte sammeln die größten konkreten Verbesserungshebel.

---

## 1. Konkrete Modell- und Detailfehler beheben

Gezielte Korrekturen an den bestehenden Schiffsmodulen mit direktem visuellem Gewinn:

- **Durchdringungen beseitigen:** Kühlrippen an `vanguard` und `pendulum` schneiden aktuell in die Nachbardüse.
- **Kennnummern lesbarer machen (`parts.py`):** Ziffern aus `parts.number` unter 0,16 Höhe sind an den Flanken von `vanguard`, `iron_dreadnought` und `titan_carrier` zu dünn und kontrastarm; für kleine Höhen braucht `number` eine kräftigere Strichstärke.
- **Kennzeichen schärfen:**
  - `vanguard`: Die dunklen Halter auf den leuchtenden Schneiden des Rammbugs greifen in das Hauptkennzeichen ein.
  - `seal`: Das Wappen wird in der zweiten Kamera (Gegner von vorn oben) teilweise vom Turm verdeckt.
  - `signal_booster`: Die drei Streben in der Schüssel verändern das Kennzeichen.
  - `horizon_cruiser`: Die Rückwand-Fenster leuchten von hinten zu kräftig.
  - `scrap_king`: Die Flacheisen lesen eher als Stange denn als Naht; die hellen Heftnähte um die Flicken dominieren in der Nahaufnahme zu stark.
  - Flaggschiffe allgemein: Flak-Gehäuse in der Akzentfarbe (Bernstein, Glutorange, Rot) ziehen stärker den Blick auf sich als am Belagerer (`iron_dreadnought`).
- **Maße und Wartbarkeit bereinigen:**
  - `titan_carrier`: Durch die Rettungsinseln ~3 % breiter geworden; Silhouette/Slot-Breite prüfen.
  - `iron_dreadnought`: Teile auf den Turmdächern rechnen die Dachhöhe aus `parts.turret` von Hand nach statt sie aus einer gemeinsamen Quelle zu beziehen.
- **Unbenutzte Modelle entscheiden:** `hive`, `mirage`, `drone`, `repair_drone`, `decoy` und `broadside` stehen noch in der groben Clean-Toon-Fassung – entweder auf Nahaufnahme-Stand heben (falls sie wieder genutzt werden) oder bewusst archivieren/entfernen.

---

## 2. Unbekannter Kontakt & Brut organisch verfeinern

Größtes kreatives Potenzial bei `fleet/unknown_contact.py`, `fleet/brood.py`, `skin.py` und `assets/shaders/toon.wesl`:

- **Flughaut ohne harte Stufen:** Derzeit bestehen die Flügel aus drei übereinandergelegten geschlossenen Lofts (`web`, `membrane`, `lumen`) mit harten Materialgrenzen. Ein weicher, organischer Übergang über Hauttextur/Shader (ggf. mit subtiler Transluzenz statt vollständig deckender Fläche) wirkt deutlich lebendiger.
- **Leuchten und Biopuls:** Große Leuchtflächen kippen durch `GLOW_BOOST` (`toon/look.rs`) von Türkis Richtung grelles Cyan. Ein feineres Leuchtprofil sowie ein langsames, lebendiges Pulsieren von Organen, Adern und Leuchtpunkten (nicht erst im Todeskampf) würden das Wesen stark von den menschlichen Schiffen abheben.
- **Silhouette & Anatomie näher ans Konzept:** Flügelwinkel/-höhe noch stärker Richtung Sichelform ausloten, das Knochenschwamm-Netz an den Wangen feiner verzweigen und Maul/Unterseite im maximalen Zoom prüfen.
- **Masken-Schmutz auf Knochen prüfen:** Der Schmutz der Verwitterungsmaske in Ritzen (`look.WEATHER_MATERIALS`) erzeugt braune Flecken auf den Knochen; prüfen, ob das am Alien-Wesen erwünscht ist oder reduziert/entfernt werden sollte.
- **Brut im Zoom prüfen:** `brood.py` bisher vor allem in Brettgröße geprüft, Nahaufnahme und Hautskalierung (`FEATURE`) im Detail kontrollieren.

---

## 3. Mehr Plastizität in der extremen Nahaufnahme

Bei maximalem Zoom (bis ~345–490 px/Einheit auf Retina/4K) fallen Bauweisen auf, die für Brettgröße entworfen wurden:

- **Echte Tiefe statt gefakter Vertiefungen:** Bisher sind alle Bauteile geschlossene, sich überlappende Körper; Vertiefungen (`vent`, `gate`, Hangartore) werden als dunkle Platte im aufgesetzten Rahmen angedeutet. An großen Blickfängen (Hangars, Kasematten, Triebwerkseinlässe) lassen sich echte, plastisch zurückgesetzte Buchten und sauberere Übergänge bauen.
- **Geteilte Baugruppen in `parts.py` aufwerten:** Verbesserungen an `turret`, `turret_details`, `engine`, `engine_details`, `flak` und `rcs` verbessern sofort alle Schiffe gleichzeitig.
- **Duplizierte Helfer nach `parts.py` ziehen:** Mehrfach kopierte Hilfsfunktionen zentralisieren, damit zukünftige Detail-Upgrades konsistent greifen:
  - Stellstreben an Düsenglocken (derzeit in zehn Modulen)
  - Tür in einer Wand (`door`, drei Fassungen)
  - Blech auf rundem Rumpf (`skin`, drei Fassungen)
  - Quertreppen (`gangway`), Gitterrost auf geneigtem Deck (`walkway`, `catwalk`), Schweißnähte (`welds`), Munitionskästen

---

## 4. Pipeline-Hebel für Schärfe, Speicher und schnelleres Iterieren

- **Verdeckte UV-Texel schrumpfen (`weather.py`):** Etwa ein Viertel (an `bastion` eher mehr) der Masken-Texel landet auf verdeckten Flächen (Unterseiten aufgesetzter Teile, Durchdringungen). Verdeckte UV-Inseln vor dem Packen kleinzuskalieren bringt bei gleicher Bildgröße sichtbar schärfere Verwitterung – oder senkt VRAM (aktuell ~238 MB gesamt, `bastion` allein 6048 px / 49 MB) und Backzeit deutlich.
- **Vorschau auf den dreckigen Look umstellen (`preview.py`, `look.py`):** Die Blender-Vorschau rendert derzeit noch Clean Toon mit Linien und zeigt weder Plattenfasen, Metall-Feinstruktur noch die Alien-Haut (`skin.py`). Eine Vorschau im echten Spiel-Look ermöglicht zuverlässige visuelle Iteration direkt in Blender und liefert wieder brauchbare `hero`-Renders als Vorlage für neue Kartenbilder.
- **Kartenbilder der Flaggschiffe erneuern:** `assets/images/cards/bastion.jpg`, `vanguard.jpg`, `pendulum.jpg` und `pyre.jpg` zeigen noch die alten glatten Modelle und sollten nach dem Preview-Update neu über `hero`-Renders erzeugt werden.
- **Ruß & Wracks abrunden:**
  - `weather.py`: Ruß entsteht aktuell nur an kreisrunden Mündungsdeckeln; Läufe ohne solchen Deckel bleiben sauber.
  - `wreck.py`: Die neuen hochdetaillierten Wracks (bis 41,7k Dreiecke) systematisch visuell prüfen; außerdem die Koordinaten-Bindung der Feinstruktur an Wrackstücken prüfen (springt beim Zerbrechen, da objektlokal statt schiffslokal).
- **Flimmern & Retina-LOD testen:** Dünne Klein- und Feinteile (Antennen, Geländer, Gitterstäbe, Steigeisen, Ketten) sowie `FINE_LOD` (`toon.wesl`) auf hochauflösenden Displays in Bewegung auf allen Schiffen prüfen.

---

## Empfohlene Reihenfolge

1. **Vorschau-Pipeline (`preview.py`) & UV-Packing (`weather.py`)** – verbessert sofort den Feedback-Loop und spart Backzeit/VRAM bei allen folgenden Änderungen.
2. **Konkrete Fehler & `parts.py`-Konsolidierung** – schnelle, risikoarme Qualitätssteigerung über die gesamte Flotte.
3. **Unbekannter Kontakt (Flughaut, Shader, Pulsieren, Silhouette)** – größter sichtbarer Wow-Effekt am Klimax des Runs.
4. **Gezielter Nahaufnahme-Pass an den Flaggschiffen** (inkl. aktualisierter `hero`-Renders und Kartenbilder).
