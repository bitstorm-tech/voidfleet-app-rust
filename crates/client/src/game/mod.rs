//! Das Gefecht aus Sicht des Clients, ohne Darstellung: die Ressource [`Battle`] um
//! [`voidfleet_core::battle::Battle`] und die Aktionen des Spielers als [`PlayerAction`].
//!
//! Ablauf: Wer etwas tun will (Controller, Oberfläche, Rauchtest), schreibt eine [`PlayerAction`]; [`GameSystems`]
//! führt sie am Regelwerk aus. Gelingt sie, wirkt sie sofort im Zustand, und ihre Ereignisse gehen als
//! [`BattleEvents`] hinaus; lehnt das Regelwerk ab, meldet es [`RuleRejected`]. [`StartBattle`] beginnt ein Gefecht
//! ([`BattleStarted`]), [`LeaveBattle`] räumt es ab. Welches Gefecht ansteht, bestimmt der Run ([`crate::run`]).

use bevy::prelude::*;
use voidfleet_core::{
    admirals::Station,
    battle::{self, RuleError, Target},
    events::BattleEvent,
};

/// Das laufende Gefecht; fehlt außerhalb der Gefechte des Runs.
#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct Battle(pub battle::Battle);

/// Was der Spieler im Spielerzug tun kann.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerAction {
    /// Startspalte vor der ersten Aktion in Runde 1.
    Place(usize),
    PlayCard {
        hand_index: usize,
        target: Target,
    },
    SetStation(Station),
    SetTarget(u32),
    SetColumn(usize),
    TriggerStation,
    EndTurn,
}

/// Die Ereignisse einer gelungenen Aktion in Ausführungsreihenfolge; der Zustand in [`Battle`] ist schon der danach.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct BattleEvents(pub Vec<BattleEvent>);

/// Das Regelwerk hat eine Aktion abgelehnt; der Text steht in [`crate::i18n::Locale`] (`rule_error`).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleRejected(pub RuleError);

/// Beginnt dieses Gefecht (das des aktuellen Knotens im Run).
#[derive(Message, Clone, Debug)]
pub struct StartBattle(pub Box<battle::Battle>);

/// Ein neues Gefecht steht in [`Battle`].
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleStarted;

/// Räumt das Gefecht ab; zu sehen ist wieder der Run (Beute, Run-Ende) bzw. das Hauptmenü.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LeaveBattle;

/// Hier führt das Plugin die Aktionen aus; wer auf seine Messages reagiert, läuft danach.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameSystems;

/// Führt eine Aktion am Gefecht aus; `Ok` mit ihren Ereignissen (Einstellungen haben keine).
pub fn apply(
    battle: &mut battle::Battle,
    action: PlayerAction,
) -> Result<Vec<BattleEvent>, RuleError> {
    match action {
        PlayerAction::Place(column) => battle.place(column).map(|()| Vec::new()),
        PlayerAction::PlayCard { hand_index, target } => battle.play_card(hand_index, target),
        PlayerAction::SetStation(station) => battle.set_station(station).map(|()| Vec::new()),
        PlayerAction::SetTarget(uid) => battle.set_target(uid).map(|()| Vec::new()),
        PlayerAction::SetColumn(column) => {
            battle.set_column(column);
            Ok(Vec::new())
        }
        PlayerAction::TriggerStation => battle.trigger_station(),
        PlayerAction::EndTurn => Ok(battle.end_turn()),
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerAction>()
            .add_message::<BattleEvents>()
            .add_message::<RuleRejected>()
            .add_message::<StartBattle>()
            .add_message::<BattleStarted>()
            .add_message::<LeaveBattle>()
            .add_systems(
                Update,
                (start_battle, leave_battle, run_actions)
                    .chain()
                    .in_set(GameSystems),
            );
    }
}

fn start_battle(
    mut commands: Commands,
    mut start: MessageReader<StartBattle>,
    mut started: MessageWriter<BattleStarted>,
) {
    if let Some(StartBattle(battle)) = start.read().last() {
        info!("Gefecht beginnt: {:?}", battle.flagship.admiral);
        commands.insert_resource(Battle((**battle).clone()));
        started.write(BattleStarted);
    }
}

fn leave_battle(mut commands: Commands, mut leave: MessageReader<LeaveBattle>) {
    if leave.read().last().is_some() {
        commands.remove_resource::<Battle>();
    }
}

/// Führt die Aktionen dieses Bilds der Reihe nach aus.
fn run_actions(
    mut actions: MessageReader<PlayerAction>,
    battle: Option<ResMut<Battle>>,
    mut events: MessageWriter<BattleEvents>,
    mut rejected: MessageWriter<RuleRejected>,
) {
    let Some(mut battle) = battle else {
        actions.clear();
        return;
    };
    for &action in actions.read() {
        match apply(&mut battle, action) {
            Ok(happened) => {
                if !happened.is_empty() {
                    events.write(BattleEvents(happened));
                }
            }
            Err(error) => {
                info!("Abgelehnt: {action:?} → {error:?}");
                rejected.write(RuleRejected(error));
            }
        }
    }
}

/// Ein festes Gefecht für die Tests und die Dev-Szene `report`: Startdeck, volle Hülle, Wachkreuzer Front Mitte,
/// Plünderer Back links, Belagerer Front rechts.
pub fn fixture_battle(admiral: voidfleet_core::admirals::AdmiralId) -> battle::Battle {
    use voidfleet_core::{
        battle::{BattleSetup, Encounter},
        board::{EnemySlot, Row},
        cards::starter_deck,
        enemies::EnemyId,
    };
    let setup = BattleSetup {
        admiral,
        hull: admiral.hull(),
        max_hull: admiral.hull(),
        deck: starter_deck(admiral),
        modules: Vec::new(),
        emergency_generator_available: false,
        encounter: Encounter {
            units: vec![
                (EnemyId::GuardCruiser, EnemySlot::new(Row::Front, 1)),
                (EnemyId::Raiders, EnemySlot::new(Row::Back, 0)),
                (EnemyId::Besieger, EnemySlot::new(Row::Front, 2)),
            ],
            hull_percent: 100,
            reinforcements: Vec::new(),
            replacements: Vec::new(),
        },
    };
    battle::Battle::new(setup, 1)
}

#[cfg(test)]
mod tests {
    use voidfleet_core::admirals::AdmiralId;

    use super::*;

    /// Das Gefecht beginnt mit drei Gegnern und voller Hand; Waffen auf den Wachkreuzer lösen aus und treffen.
    #[test]
    fn test_battle_plays() {
        let mut battle = fixture_battle(AdmiralId::Osei);
        assert_eq!(battle.enemies.len(), 3);
        assert_eq!(battle.hand.len(), 5);
        let uid = battle.enemies[0].uid;
        apply(&mut battle, PlayerAction::SetTarget(uid)).unwrap();
        let events = apply(&mut battle, PlayerAction::TriggerStation).unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, BattleEvent::Hit { .. }))
        );
        assert_eq!(
            apply(&mut battle, PlayerAction::TriggerStation),
            Err(RuleError::NoStationLeft)
        );
        let events = apply(&mut battle, PlayerAction::EndTurn).unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, BattleEvent::RoundStarted { round: 2 }))
        );
    }
}
