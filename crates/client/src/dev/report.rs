//! Dev-Szene `report`: Auf dem Test-Gefecht mit Reyes spielt die Wiedergabe in Dauerschleife einen erfundenen Strom ab,
//! Aktion für Aktion wie im Spiel: der Spielerzug mit jeder Art von Station und Karte, zwei Gegnerzüge mit jeder
//! Aktionsart und die Auflösung. Nach jeder Aktion gleicht die Szene sich dem (unveränderten) Gefecht wieder an.

use bevy::prelude::*;
use voidfleet_core::{
    admirals::{AdmiralId, Station},
    battle::{Battle as CoreBattle, Target},
    board::{EnemySlot, Row},
    cards::{CardId, DeckCard},
    enemies::{Action, EnemyId},
    events::{
        BattleEvent, Breakdown, DestroyCause, IdleReason, Intent, MissReason, UnitRef, UnitStats,
    },
};

use crate::{
    game::{Battle, start_test_battle},
    playback::PlayEvents,
    scene::sync::BoardMode,
};

/// So lange ruht die Planung zwischen zwei Aktionen.
const REST: f32 = 0.8;

pub(super) fn stage(mut commands: Commands) {
    commands.insert_resource(start_test_battle(AdmiralId::Reyes));
}

/// Schickt die nächste Aktion, sobald die Planung kurz geruht hat.
pub(super) fn run(
    battle: Res<Battle>,
    mode: Res<State<BoardMode>>,
    time: Res<Time>,
    mut rest: Local<f32>,
    mut next: Local<usize>,
    mut mode_next: ResMut<NextState<BoardMode>>,
    mut play: MessageWriter<PlayEvents>,
) {
    if *mode.get() != BoardMode::Planning {
        *rest = 0.0;
        return;
    }
    *rest += time.delta_secs();
    if *rest < REST {
        return;
    }
    *rest = 0.0;
    let actions = actions(&battle);
    info!("Dev-Szene report: Aktion {}", *next % actions.len());
    play.write(PlayEvents(actions[*next % actions.len()].clone()));
    mode_next.set(BoardMode::Playback);
    *next += 1;
}

fn card(id: CardId, target: Target) -> BattleEvent {
    BattleEvent::CardPlayed {
        card: DeckCard::new(id),
        target,
    }
}

fn hit(source: UnitRef, target: UnitRef, breakdown: Breakdown, hull: i32) -> BattleEvent {
    BattleEvent::Hit {
        source,
        target,
        breakdown,
        after: UnitStats { hull, shield: 0 },
    }
}

fn damage(base: i32, armor: i32) -> Breakdown {
    Breakdown {
        base,
        armor,
        hull: base - armor,
        ..Breakdown::default()
    }
}

fn act(unit: UnitRef, action: Action) -> BattleEvent {
    BattleEvent::EnemyAction { unit, action }
}

fn intents(unit: UnitRef, action: Action, damage: i32) -> BattleEvent {
    let intent = Intent { action, damage };
    BattleEvent::Intents {
        unit,
        intents: [intent, intent],
    }
}

/// Die erfundenen Aktionen auf der Test-Formation (Wachkreuzer Front Mitte, Plünderer Back links, Belagerer Front
/// rechts).
fn actions(battle: &CoreBattle) -> Vec<Vec<BattleEvent>> {
    let unit = |id: EnemyId| {
        battle
            .enemies
            .iter()
            .find(|enemy| enemy.id == id)
            .map(|enemy| enemy.unit())
            .expect("Test-Formation")
    };
    let (guard, raiders, besieger) = (
        unit(EnemyId::GuardCruiser),
        unit(EnemyId::Raiders),
        unit(EnemyId::Besieger),
    );
    let f = UnitRef::Flagship;
    let uid = |unit: UnitRef| match unit {
        UnitRef::Enemy { uid, .. } => uid,
        UnitRef::Flagship => 0,
    };
    let spotter = UnitRef::Enemy {
        uid: 50,
        id: EnemyId::Spotter,
        slot: EnemySlot::new(Row::Back, 2),
    };
    let fireship = UnitRef::Enemy {
        uid: 51,
        id: EnemyId::Fireship,
        slot: EnemySlot::new(Row::Front, 0),
    };
    let stats = |hull, shield| UnitStats { hull, shield };
    vec![
        vec![
            BattleEvent::StationTriggered {
                station: Station::Weapons,
            },
            hit(f, guard, damage(9, 1), 10),
            BattleEvent::Ignited { target: guard },
        ],
        vec![
            card(CardId::Torpedo, Target::Enemy(uid(besieger))),
            hit(f, besieger, damage(10, 2), 22),
        ],
        vec![
            card(CardId::CoupDeGrace, Target::Enemy(uid(raiders))),
            hit(
                f,
                raiders,
                Breakdown {
                    base: 8,
                    evaded: 4,
                    covered: Some((uid(guard), 2)),
                    hull: 2,
                    ..Breakdown::default()
                },
                6,
            ),
            hit(f, guard, damage(2, 0), 8),
        ],
        vec![
            card(CardId::VolleyFire, Target::None),
            hit(f, guard, damage(4, 1), 5),
            hit(f, raiders, damage(4, 0), 2),
            hit(f, besieger, damage(4, 2), 20),
        ],
        vec![
            card(CardId::CollisionCourse, Target::Enemy(uid(besieger))),
            BattleEvent::Rammed { target: besieger },
            hit(f, besieger, damage(14, 2), 8),
            hit(besieger, f, damage(6, 0), 50),
        ],
        vec![
            card(CardId::EmergencyShields, Target::None),
            BattleEvent::Shielded {
                target: f,
                amount: 6,
                after: stats(50, 6),
            },
        ],
        vec![card(CardId::EmpBlast, Target::Enemy(uid(besieger)))],
        vec![
            card(CardId::TractorBeam, Target::Enemy(uid(raiders))),
            BattleEvent::Tractored {
                unit: UnitRef::Enemy {
                    uid: uid(raiders),
                    id: EnemyId::Raiders,
                    slot: EnemySlot::new(Row::Back, 1),
                },
                from: EnemySlot::new(Row::Back, 0),
            },
        ],
        vec![
            card(CardId::EvasiveAction, Target::Column(0)),
            BattleEvent::Maneuvered { from: 1, to: 0 },
        ],
        vec![card(CardId::RedAlert, Target::None)],
        vec![card(CardId::Countermeasures, Target::None)],
        vec![
            BattleEvent::StationTriggered {
                station: Station::Admiral,
            },
            hit(f, guard, damage(6, 0), 4),
        ],
        vec![
            card(CardId::RepairCrew, Target::None),
            BattleEvent::Extinguished { target: f },
            BattleEvent::Healed {
                target: f,
                amount: 5,
                after: stats(55, 0),
            },
        ],
        // Zugende, Gegnerzug, Auflösung.
        vec![
            BattleEvent::SelfDamage {
                amount: 3,
                after: stats(52, 0),
            },
            BattleEvent::Arrived { unit: spotter },
            intents(spotter, Action::TargetLock, 0),
            BattleEvent::Arrived { unit: fireship },
            intents(fireship, Action::Ram, 10),
            act(guard, Action::Fire),
            hit(
                guard,
                f,
                Breakdown {
                    base: 8,
                    countermeasures: 4,
                    armor: 1,
                    shielded: 3,
                    hull: 0,
                    ..Breakdown::default()
                },
                52,
            ),
            BattleEvent::Counterfire {
                target: guard,
                amount: 3,
                after: stats(7, 0),
            },
            act(raiders, Action::Evade),
            act(spotter, Action::Evade),
            act(
                besieger,
                Action::ColumnFire {
                    columns: [true, false, false],
                },
            ),
            hit(besieger, f, damage(12, 1), 41),
            BattleEvent::Ignited { target: f },
            act(fireship, Action::Ram),
            hit(fireship, f, damage(10, 1), 32),
            BattleEvent::Emergency {
                target: f,
                active: true,
            },
            BattleEvent::FireshipSpent { unit: fireship },
            act(spotter, Action::TargetLock),
            BattleEvent::BurnTick {
                target: f,
                amount: 2,
                after: stats(30, 0),
            },
            BattleEvent::BurnTick {
                target: guard,
                amount: 3,
                after: stats(4, 0),
            },
            BattleEvent::RoundStarted { round: 2 },
            BattleEvent::Shuffled,
            BattleEvent::Drawn { cards: Vec::new() },
            intents(guard, Action::Shields(4), 0),
            intents(raiders, Action::Fire, 4),
            intents(besieger, Action::Charge, 0),
        ],
        vec![card(CardId::MicroJump, Target::None), BattleEvent::Jumped],
        vec![
            act(guard, Action::Shields(4)),
            BattleEvent::Shielded {
                target: guard,
                amount: 4,
                after: stats(10, 4),
            },
            BattleEvent::Idle {
                unit: raiders,
                reason: IdleReason::Silenced,
            },
            act(besieger, Action::Charge),
            act(spotter, Action::Fire),
            BattleEvent::Missed {
                source: spotter,
                reason: MissReason::Jumped,
            },
            act(
                besieger,
                Action::ColumnFire {
                    columns: [false, true, true],
                },
            ),
            BattleEvent::Missed {
                source: besieger,
                reason: MissReason::Maneuvered,
            },
            act(besieger, Action::Call(EnemyId::Raiders)),
            act(guard, Action::Bolt),
            BattleEvent::Shielded {
                target: guard,
                amount: 3,
                after: stats(10, 7),
            },
            BattleEvent::Shielded {
                target: besieger,
                amount: 3,
                after: stats(30, 3),
            },
            BattleEvent::HoldTheLine {
                target: besieger,
                amount: 2,
                after: stats(28, 3),
            },
            BattleEvent::BurnTick {
                target: guard,
                amount: 3,
                after: stats(0, 0),
            },
            BattleEvent::Destroyed {
                target: guard,
                cause: DestroyCause::BurnedOut,
            },
            BattleEvent::EmergencyGenerator { after: stats(1, 0) },
            BattleEvent::Returned,
            BattleEvent::RoundStarted { round: 8 },
            BattleEvent::Clock { bonus: 1 },
            intents(besieger, Action::Fire, 9),
        ],
    ]
}
