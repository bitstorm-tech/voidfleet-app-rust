//! Module: dauerhafte Passiva des Runs (`docs/run.md`); ihre Zahlen stehen in `assets/data/rules.json` (`modules`),
//! die Kommentare hier nennen die Startwerte. Das Gefecht wertet aus, was im Gefecht wirkt;
//! Kampfsimulator und Verstärkte Spanten wertet der Run aus, den Schrott des Bergungskrans auch (aus den Abschüssen).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleId {
    /// Mehr ATK auf die Schüsse des Flaggschiffs.
    FireControlComputer,
    /// Der eigene Brand tickt nicht.
    FireSuppression,
    /// 4 Befehlspunkte in Runde 1.
    ReserveReactor,
    /// Panzerung +1.
    ArmorPlates,
    /// Einmal im Run: Ein tödlicher Schaden lässt 1 Hülle.
    EmergencyGenerator,
    /// Jede zerstörte gegnerische Einheit bringt 5 Schrott (der Run rechnet aus den Abschüssen).
    SalvageCrane,
    /// 6 statt 5 Karten je Runde.
    SupplyPod,
    /// Kartenangebote zeigen vier statt drei (Run).
    CombatSimulator,
    /// +10 Hülle dauerhaft (Run).
    ReinforcedFrames,
}
