//! Vorschau der Rundenwiedergabe ohne Server (wie `client/dev/commands.ts`), mit `VOIDFLEET_DEV_SCENE=commands`: Die
//! echte Wiedergabe ([`crate::playback`]) spielt nacheinander je Befehl erfundene Ereignisse auf einem kleinen Brett
//! ab, mit Banner, Log, Schwebetexten und Bordfunk, in Dauerschleife; dazu Schüsse im Kampf, die die Deckung abfängt
//! oder die verpuffen (`combat`), und die übrigen Phasen einer Runde (`round`: Deploy, Fähigkeiten, Zielerfassung,
//! Manöver, Railgun, Taktvorsprung, Gegenfeuer).
//!
//! - `VOIDFLEET_DEV_CARD=<id>` zeigt nur die Szenen dieses Befehls (`torpedo`, `piercing_shot` …), `combat` bzw.
//!   `round` die Kampf- bzw. Rundenszenen.
//! - `VOIDFLEET_DEV_SHOTS=<ordner>` schreibt während jeder Wiedergabe alle 0,2 s einen Screenshot
//!   `<ordner>/<nr>-<szene>-<bild>.png` und beendet die App nach dem ersten Durchgang.
//!
//! Zwischen zwei Szenen stellt der Abgleich der Planung das Brett aus dem erfundenen Spielzustand neu auf.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use voidfleet_core::{
    admirals::{AdmiralId, PowerId},
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::{CommandId, ShipId},
    protocol::{AbilityEffect, CommandEffect, Miss, OpponentView, RoundEvent, ShipRef, ShipStats},
};

use crate::{
    assets::AppState,
    controller::Scripted,
    data::FileKey,
    game::{Match, ServerInbox},
    palette::admiral_color,
    playback::{PlayRound, PlaybackFinished},
    scene::{
        board::OnSlot,
        markers::{DropMarker, MarkerKind, MarkerSpec},
        ship::Ship,
        sync::BoardMode,
    },
};

/// Pause vor der ersten Szene (Modelle), vor jeder weiteren und nach jeder Wiedergabe.
const FIRST_DELAY: f32 = 1.5;
const SETUP_DELAY: f32 = 0.6;
const REST: f32 = 1.2;
/// Abstand der Screenshots einer Serie.
const SHOT_INTERVAL: f32 = 0.2;

/// Ob die Szene gewählt ist.
pub(super) fn requested() -> bool {
    std::env::var("VOIDFLEET_DEV_SCENE").is_ok_and(|scene| scene == "commands")
}

pub(super) fn plugin(app: &mut App) {
    let only = std::env::var("VOIDFLEET_DEV_CARD").ok();
    let scenes: Vec<Scene> = scenes()
        .into_iter()
        .filter(|scene| only.as_deref().is_none_or(|only| scene.group == only))
        .collect();
    assert!(!scenes.is_empty(), "keine Szene für {only:?}");
    app.insert_resource(Scripted)
        .insert_resource(Showcase {
            scenes,
            index: 0,
            round: 1,
            phase: Phase::Setting(pause(FIRST_DELAY)),
            shots: std::env::var_os("VOIDFLEET_DEV_SHOTS").map(PathBuf::from),
            shot: 0,
            next_shot: Timer::from_seconds(SHOT_INTERVAL, TimerMode::Repeating),
        })
        .add_systems(OnEnter(AppState::Match), stage)
        .add_systems(Update, run.run_if(in_state(AppState::Match)));
}

/// Eine Szene: die Schiffe auf dem Brett und die Ereignisse, die darauf abgespielt werden.
struct Scene {
    /// Der Befehl der Szene als Id (`torpedo`), sonst `combat` oder `round`.
    group: &'static str,
    name: &'static str,
    /// Schiffe mit ihren HP.
    ships: Vec<(SlotRef, ShipId, i32)>,
    /// Eine Marke, die vor der Wiedergabe fällt (etwa eine Admiralsfähigkeit, die Funkstille wieder wegnimmt).
    marker: Option<(SlotRef, MarkerKind, AdmiralId)>,
    events: Vec<RoundEvent>,
}

#[derive(Debug, PartialEq)]
enum Phase {
    /// Das Brett steht; die Wiedergabe beginnt, wenn die Pause um ist.
    Setting(Timer),
    Playing,
    /// Die Wiedergabe ist fertig; die nächste Szene kommt, wenn die Pause um ist.
    Resting(Timer),
}

/// Eine Pause von so vielen Sekunden.
fn pause(seconds: f32) -> Timer {
    Timer::from_seconds(seconds, TimerMode::Once)
}

#[derive(Resource)]
struct Showcase {
    scenes: Vec<Scene>,
    index: usize,
    round: u32,
    phase: Phase,
    shots: Option<PathBuf>,
    shot: u32,
    /// Takt der Screenshot-Serie.
    next_shot: Timer,
}

impl Showcase {
    fn scene(&self) -> &Scene {
        &self.scenes[self.index]
    }

    /// Ein Match in Runde 3, Admiral Osei gegen Admiralin Kerr, mit den Schiffen der Szene.
    fn staged(&self) -> Match {
        let mut game = Match::default();
        game.round = 3;
        game.admiral = Some(AdmiralId::Osei);
        game.opponent = Some(OpponentView {
            admiral: Some(AdmiralId::Kerr),
            submitted: true,
            ship_hand_size: 3,
            command_hand_size: 3,
            ship_deck_size: 10,
            command_deck_size: 10,
            bot: true,
            played_commands: Vec::new(),
        });
        game.submitted = true;
        for &(slot, card, hp) in &self.scene().ships {
            game.board.put(slot, card).hp = hp;
        }
        game
    }
}

fn stage(mut inbox: ResMut<ServerInbox>, showcase: Res<Showcase>, mut game: ResMut<Match>) {
    inbox.hold();
    *game = showcase.staged();
}

fn run(
    mut commands: Commands,
    mut showcase: ResMut<Showcase>,
    time: Res<Time>,
    mut game: ResMut<Match>,
    mut mode: ResMut<NextState<BoardMode>>,
    mut play: MessageWriter<PlayRound>,
    mut finished: MessageReader<PlaybackFinished>,
    ships: Query<(Entity, &Ship, &OnSlot)>,
    mut exit: MessageWriter<AppExit>,
) {
    // Weiter, sobald die Pause um bzw. die Wiedergabe fertig ist.
    let over = match &mut showcase.phase {
        Phase::Setting(timer) | Phase::Resting(timer) => timer.tick(time.delta()).just_finished(),
        Phase::Playing => !finished.is_empty(),
    };
    match showcase.phase {
        Phase::Setting(_) if over => {
            let scene = showcase.scene();
            info!("Dev-Szene {}: {}", scene.group, scene.name);
            if let Some((slot, kind, admiral)) = scene.marker
                && let Some((ship, ..)) = ships
                    .iter()
                    .find(|(_, ship, on)| !ship.ghost && on.0 == slot)
            {
                commands.spawn(DropMarker {
                    ship,
                    spec: MarkerSpec::new(kind, admiral_color(admiral)),
                });
            }
            mode.set(BoardMode::Playback);
            play.write(PlayRound {
                round: showcase.round,
                events: scene.events.clone(),
            });
            showcase.phase = Phase::Playing;
            showcase.shot = 0;
            showcase.next_shot.reset();
        }
        Phase::Playing if over => {
            finished.clear();
            showcase.phase = Phase::Resting(pause(REST));
        }
        Phase::Resting(_) if over => {
            showcase.index += 1;
            if showcase.index == showcase.scenes.len() {
                if showcase.shots.is_some() {
                    exit.write(AppExit::Success);
                }
                showcase.index = 0;
                showcase.round += 1;
            }
            *game = showcase.staged();
            mode.set(BoardMode::Planning);
            showcase.phase = Phase::Setting(pause(SETUP_DELAY));
        }
        _ => {}
    }
    // Screenshot-Serie, solange abgespielt wird: gleich zu Beginn, dann alle `SHOT_INTERVAL`.
    if showcase.phase == Phase::Playing
        && let Some(dir) = showcase.shots.clone()
        && (showcase.next_shot.tick(time.delta()).just_finished() || showcase.shot == 0)
    {
        let path = dir.join(format!(
            "{:02}-{}-{:02}.png",
            showcase.index,
            showcase.scene().group,
            showcase.shot
        ));
        showcase.shot += 1;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

// --- Die Szenen ---

const P: Side = Side::Player;
const E: Side = Side::Enemy;
use LaneType::{Command, Strike};

fn at(side: Side, lane: LaneType, index: usize, card: ShipId) -> ShipRef {
    ShipRef::new(SlotRef::new(side, lane, index), card)
}

/// Volle Werte abzüglich `damage`, mit `shield` Schild mehr.
fn stats(card: ShipId, damage: i32, shield: i32) -> ShipStats {
    ShipStats {
        hp: card.hp() - damage,
        shield: card.card().shield + shield,
    }
}

fn effect(side: Side, card: CommandId, ship: ShipRef, value: CommandEffect) -> RoundEvent {
    effect_after(side, card, ship, value, stats(ship.card_id, 0, 0))
}

fn effect_after(
    side: Side,
    card_id: CommandId,
    ship: ShipRef,
    effect: CommandEffect,
    after: ShipStats,
) -> RoundEvent {
    RoundEvent::Effect {
        side,
        card_id,
        ship,
        effect,
        after,
    }
}

fn attack(attacker: ShipRef, target: ShipRef, amount: i32, after: ShipStats) -> RoundEvent {
    RoundEvent::Attack {
        attacker,
        target,
        amount,
        after,
        focus: false,
        swarm: false,
        head_start: false,
        through: false,
        taunt: false,
    }
}

/// Eine Besonderheit eines Schusses im Kampf.
#[derive(Clone, Copy)]
enum Flag {
    Taunt,
    Through,
    HeadStart,
    Focus,
}

fn flagged(mut event: RoundEvent, flag: Flag) -> RoundEvent {
    if let RoundEvent::Attack {
        taunt,
        through,
        head_start,
        focus,
        ..
    } = &mut event
    {
        *match flag {
            Flag::Taunt => taunt,
            Flag::Through => through,
            Flag::HeadStart => head_start,
            Flag::Focus => focus,
        } = true;
    }
    event
}

fn fizzle(attacker: ShipRef, miss: Miss) -> RoundEvent {
    RoundEvent::Fizzle {
        attacker,
        head_start: false,
        miss,
    }
}

const P_STRIKE_2: ShipRef = ShipRef::new(SlotRef::new(P, Strike, 2), ShipId::VoidInterceptor);
const P_STRIKE_3: ShipRef = ShipRef::new(SlotRef::new(P, Strike, 3), ShipId::HorizonCruiser);
const P_COMMAND_3: ShipRef = ShipRef::new(SlotRef::new(P, Command, 3), ShipId::RepairDrone);
const E_STRIKE_2: ShipRef = ShipRef::new(SlotRef::new(E, Strike, 2), ShipId::ScrapRaider);
const E_STRIKE_3: ShipRef = ShipRef::new(SlotRef::new(E, Strike, 3), ShipId::IronDreadnought);
const E_COMMAND_3: ShipRef = ShipRef::new(SlotRef::new(E, Command, 3), ShipId::TitanCarrier);
const E_COMMAND_1: ShipRef = ShipRef::new(SlotRef::new(E, Command, 1), ShipId::PlasmaFrigate);

const FLEET: [ShipRef; 7] = [
    P_STRIKE_2,
    P_STRIKE_3,
    P_COMMAND_3,
    E_STRIKE_2,
    E_STRIKE_3,
    E_COMMAND_3,
    E_COMMAND_1,
];

/// Die Flotte mit vollen HP, dazu `extra`; `hp` setzt einzelne HP.
fn fleet(extra: &[ShipRef], hp: &[(ShipRef, i32)]) -> Vec<(SlotRef, ShipId, i32)> {
    FLEET
        .iter()
        .chain(extra)
        .map(|ship| {
            let hp = hp
                .iter()
                .find(|(other, _)| other == ship)
                .map_or(ship.card_id.hp(), |(_, hp)| *hp);
            (ship.slot(), ship.card_id, hp)
        })
        .collect()
}

fn scene(card: CommandId, name: &'static str, events: Vec<RoundEvent>) -> Scene {
    Scene {
        group: card.file_key(),
        name,
        ships: fleet(&[], &[]),
        marker: None,
        events,
    }
}

fn special(
    group: &'static str,
    name: &'static str,
    ships: Vec<(SlotRef, ShipId, i32)>,
    events: Vec<RoundEvent>,
) -> Scene {
    Scene {
        group,
        name,
        ships,
        marker: None,
        events,
    }
}

fn scenes() -> Vec<Scene> {
    use CommandId::*;
    let destroyed = |ship| RoundEvent::Destroyed { ship };
    let pendulum = at(E, Command, 4, ShipId::Pendulum);
    let mirage = at(P, Strike, 1, ShipId::Mirage);
    let decoy = at(E, Strike, 0, ShipId::Decoy);
    let bastion = at(P, Command, 2, ShipId::Bastion);
    vec![
        scene(
            EvasiveAction,
            "Ausweichmanöver",
            vec![
                effect(
                    P,
                    EvasiveAction,
                    P_STRIKE_2,
                    CommandEffect::Move {
                        to: SlotTarget::new(Strike, 0),
                    },
                ),
                effect_after(
                    P,
                    EvasiveAction,
                    at(P, Strike, 0, ShipId::VoidInterceptor),
                    CommandEffect::Shield {
                        amount: 2,
                        permanent: false,
                    },
                    stats(ShipId::VoidInterceptor, 0, 2),
                ),
            ],
        ),
        scene(
            MicroJump,
            "Kurzsprung",
            vec![
                effect(P, MicroJump, P_STRIKE_3, CommandEffect::Jump),
                attack(
                    E_STRIKE_3,
                    P_STRIKE_2,
                    3,
                    stats(ShipId::VoidInterceptor, 3, 0),
                ),
                RoundEvent::Return {
                    ship: P_STRIKE_3,
                    after: stats(ShipId::HorizonCruiser, 0, 0),
                },
            ],
        ),
        scene(
            RapidDeployment,
            "Schnellverlegung",
            vec![RoundEvent::Rule {
                side: P,
                card_id: RapidDeployment,
            }],
        ),
        scene(
            CarteBlanche,
            "Freie Hand",
            vec![RoundEvent::Rule {
                side: E,
                card_id: CarteBlanche,
            }],
        ),
        scene(
            DrawFire,
            "Ablenkung im Kampf",
            vec![
                effect(P, DrawFire, P_STRIKE_3, CommandEffect::Taunt),
                flagged(
                    attack(
                        E_STRIKE_2,
                        P_STRIKE_3,
                        2,
                        stats(ShipId::HorizonCruiser, 2, 0),
                    ),
                    Flag::Taunt,
                ),
                flagged(
                    attack(
                        E_COMMAND_3,
                        P_STRIKE_3,
                        1,
                        stats(ShipId::HorizonCruiser, 3, 0),
                    ),
                    Flag::Taunt,
                ),
            ],
        ),
        special(
            "combat",
            "Schiffsfeuer verpufft an der Deckung",
            fleet(&[], &[]),
            vec![
                fizzle(
                    P_STRIKE_2,
                    Miss::Target {
                        target: Some(E_COMMAND_3),
                        covered: true,
                    },
                ),
                fizzle(
                    E_STRIKE_2,
                    Miss::Target {
                        target: Some(P_COMMAND_3),
                        covered: true,
                    },
                ),
            ],
        ),
        special(
            "combat",
            "Spaltenfeuer geht durch",
            fleet(&[], &[(E_STRIKE_3, 3)]),
            vec![
                attack(P_STRIKE_2, E_STRIKE_3, 3, ShipStats { hp: 0, shield: 0 }),
                flagged(
                    attack(
                        P_STRIKE_3,
                        E_COMMAND_3,
                        2,
                        stats(ShipId::TitanCarrier, 2, 0),
                    ),
                    Flag::Through,
                ),
                destroyed(E_STRIKE_3),
            ],
        ),
        special(
            "combat",
            "Spaltenfeuer verpufft: leere Spalte",
            fleet(&[], &[]),
            vec![
                fizzle(
                    P_STRIKE_3,
                    Miss::Column {
                        column: 5,
                        cleared: false,
                    },
                ),
                fizzle(
                    E_STRIKE_3,
                    Miss::Column {
                        column: 0,
                        cleared: false,
                    },
                ),
            ],
        ),
        special(
            "combat",
            "Verpufft: schon zerstört",
            fleet(&[], &[(E_STRIKE_2, 3)]),
            vec![
                attack(P_STRIKE_2, E_STRIKE_2, 3, ShipStats { hp: 0, shield: 0 }),
                attack(
                    E_STRIKE_3,
                    P_STRIKE_2,
                    2,
                    stats(ShipId::VoidInterceptor, 2, 0),
                ),
                fizzle(
                    P_STRIKE_3,
                    Miss::Target {
                        target: Some(E_STRIKE_2),
                        covered: false,
                    },
                ),
                destroyed(E_STRIKE_2),
            ],
        ),
        special(
            "combat",
            "Verpufft: Ziel nicht mehr auf dem Brett",
            fleet(&[], &[]),
            vec![
                effect(P, MicroJump, P_STRIKE_3, CommandEffect::Jump),
                fizzle(
                    E_STRIKE_3,
                    Miss::Target {
                        target: None,
                        covered: false,
                    },
                ),
                fizzle(
                    E_COMMAND_3,
                    Miss::Target {
                        target: None,
                        covered: false,
                    },
                ),
                RoundEvent::Return {
                    ship: P_STRIKE_3,
                    after: stats(ShipId::HorizonCruiser, 0, 0),
                },
            ],
        ),
        scene(
            ShieldGrid,
            "Schildgitter",
            [P_STRIKE_2, P_STRIKE_3, P_COMMAND_3]
                .map(|ship| {
                    effect_after(
                        P,
                        ShieldGrid,
                        ship,
                        CommandEffect::Shield {
                            amount: 1,
                            permanent: false,
                        },
                        stats(ship.card_id, 0, 1),
                    )
                })
                .into(),
        ),
        scene(
            AblativeArmor,
            "Ablativpanzerung",
            vec![effect_after(
                P,
                AblativeArmor,
                P_STRIKE_3,
                CommandEffect::Shield {
                    amount: 2,
                    permanent: true,
                },
                stats(ShipId::HorizonCruiser, 0, 2),
            )],
        ),
        scene(
            CoveringFire,
            "Feuerschutz",
            [P_STRIKE_3, P_COMMAND_3]
                .into_iter()
                .flat_map(|ship| {
                    [
                        effect(P, CoveringFire, ship, CommandEffect::Attack { delta: 1 }),
                        effect_after(
                            P,
                            CoveringFire,
                            ship,
                            CommandEffect::Shield {
                                amount: 1,
                                permanent: false,
                            },
                            stats(ship.card_id, 0, 1),
                        ),
                    ]
                })
                .collect(),
        ),
        scene(
            DroneSquadron,
            "Drohnenstaffel",
            [2, 4]
                .map(|index| {
                    effect(
                        P,
                        DroneSquadron,
                        at(P, Command, index, ShipId::Drone),
                        CommandEffect::Spawn,
                    )
                })
                .into(),
        ),
        scene(
            CollisionCourse,
            "Rammkurs",
            vec![effect_after(
                P,
                CollisionCourse,
                P_STRIKE_3,
                CommandEffect::Ram {
                    target: E_STRIKE_3,
                    dealt: 5,
                    taken: 3,
                    target_after: stats(ShipId::IronDreadnought, 5, 0),
                },
                stats(ShipId::HorizonCruiser, 3, 0),
            )],
        ),
        scene(
            CollisionCourse,
            "Rammkurs, Rammer fällt",
            vec![
                effect_after(
                    E,
                    CollisionCourse,
                    E_STRIKE_2,
                    CommandEffect::Ram {
                        target: P_STRIKE_2,
                        dealt: 3,
                        taken: 9,
                        target_after: stats(ShipId::VoidInterceptor, 3, 0),
                    },
                    ShipStats { hp: 0, shield: 0 },
                ),
                destroyed(E_STRIKE_2),
            ],
        ),
        scene(
            Countermeasures,
            "Gegenmaßnahmen",
            vec![RoundEvent::Countered {
                side: P,
                card_id: Countermeasures,
                countered: Torpedo,
            }],
        ),
        Scene {
            marker: Some((E_STRIKE_3.slot(), MarkerKind::FocusFire, AdmiralId::Vance)),
            ..scene(
                RadioSilence,
                "Funkstille",
                vec![effect(
                    P,
                    RadioSilence,
                    E_STRIKE_3,
                    CommandEffect::Jam {
                        power: PowerId::FocusFire,
                    },
                )],
            )
        },
        Scene {
            ships: fleet(&[decoy], &[]),
            ..scene(
                RadioSilence,
                "Funkstille gegen das Trugbild",
                vec![effect(
                    P,
                    RadioSilence,
                    decoy,
                    CommandEffect::Jam {
                        power: PowerId::Decoy,
                    },
                )],
            )
        },
        scene(
            Minefield,
            "Minenfeld",
            [E_STRIKE_2, E_COMMAND_3]
                .map(|ship| {
                    effect_after(
                        P,
                        Minefield,
                        ship,
                        CommandEffect::Damage { amount: 3 },
                        stats(ship.card_id, 3, 0),
                    )
                })
                .into(),
        ),
        scene(
            TractorBeam,
            "Traktorstrahl",
            vec![effect(
                P,
                TractorBeam,
                E_STRIKE_2,
                CommandEffect::Move {
                    to: SlotTarget::new(Strike, 5),
                },
            )],
        ),
        scene(
            HeatSeeker,
            "Wärmesucher",
            vec![effect_after(
                P,
                HeatSeeker,
                E_COMMAND_1,
                CommandEffect::Damage { amount: 5 },
                stats(ShipId::PlasmaFrigate, 5, 0),
            )],
        ),
        scene(
            HeatSeeker,
            "Wärmesucher des Gegners",
            vec![effect_after(
                E,
                HeatSeeker,
                P_COMMAND_3,
                CommandEffect::Damage { amount: 3 },
                stats(ShipId::RepairDrone, 3, 0),
            )],
        ),
        scene(
            CoupDeGrace,
            "Gnadenstoß",
            vec![effect_after(
                P,
                CoupDeGrace,
                E_STRIKE_2,
                CommandEffect::Damage { amount: 4 },
                stats(ShipId::ScrapRaider, 4, 0),
            )],
        ),
        scene(
            PiercingShot,
            "Durchschlag",
            vec![
                effect_after(
                    P,
                    PiercingShot,
                    E_STRIKE_3,
                    CommandEffect::Damage { amount: 3 },
                    stats(ShipId::IronDreadnought, 3, 0),
                ),
                effect_after(
                    P,
                    PiercingShot,
                    E_COMMAND_3,
                    CommandEffect::Damage { amount: 2 },
                    stats(ShipId::TitanCarrier, 2, 0),
                ),
            ],
        ),
        scene(
            PiercingShot,
            "Durchschlag ins Leere",
            vec![
                RoundEvent::Miss {
                    side: P,
                    card_id: PiercingShot,
                    target: SlotRef::new(E, Strike, 1),
                },
                effect_after(
                    P,
                    PiercingShot,
                    E_COMMAND_1,
                    CommandEffect::Damage { amount: 2 },
                    stats(ShipId::PlasmaFrigate, 2, 0),
                ),
            ],
        ),
        scene(
            Bombardment,
            "Bombardement",
            [E_COMMAND_1, E_COMMAND_3]
                .map(|ship| {
                    effect_after(
                        P,
                        Bombardment,
                        ship,
                        CommandEffect::Damage { amount: 2 },
                        stats(ship.card_id, 2, 0),
                    )
                })
                .into(),
        ),
        scene(
            Crossfire,
            "Kreuzfeuer",
            [E_STRIKE_2, E_COMMAND_1]
                .map(|ship| {
                    effect_after(
                        P,
                        Crossfire,
                        ship,
                        CommandEffect::Damage { amount: 3 },
                        stats(ship.card_id, 3, 0),
                    )
                })
                .into(),
        ),
        scene(
            Torpedo,
            "Torpedo trifft und geht daneben",
            vec![
                effect_after(
                    P,
                    Torpedo,
                    E_STRIKE_3,
                    CommandEffect::Damage { amount: 4 },
                    stats(ShipId::IronDreadnought, 4, 0),
                ),
                RoundEvent::Miss {
                    side: P,
                    card_id: Torpedo,
                    target: SlotRef::new(E, Strike, 5),
                },
            ],
        ),
        special(
            "round",
            "Deploy und Drohnenstart",
            fleet(&[], &[]),
            vec![
                RoundEvent::Deploy {
                    side: P,
                    card_id: ShipId::IronDreadnought,
                    target: SlotTarget::new(Strike, 4),
                },
                RoundEvent::Deploy {
                    side: E,
                    card_id: ShipId::SignalBooster,
                    target: SlotTarget::new(Command, 5),
                },
                RoundEvent::Spawn {
                    source: E_COMMAND_3,
                    ship: at(E, Command, 2, ShipId::Drone),
                },
            ],
        ),
        special(
            "round",
            "Admiralsfähigkeiten und Trugbild",
            fleet(&[], &[]),
            vec![
                RoundEvent::Power {
                    side: P,
                    power: PowerId::Counterfire,
                    target: P_STRIKE_3.slot(),
                },
                RoundEvent::Power {
                    side: E,
                    power: PowerId::Decoy,
                    target: decoy.slot(),
                },
                RoundEvent::Decoy { ship: decoy },
                RoundEvent::Vanish { ship: decoy },
            ],
        ),
        special(
            "round",
            "Zielerfassung und Railgun",
            fleet(&[pendulum], &[]),
            vec![
                RoundEvent::Mark {
                    side: E,
                    ship: P_STRIKE_3,
                },
                RoundEvent::Railgun {
                    source: pendulum,
                    target: P_STRIKE_3,
                    amount: 4,
                    after: stats(ShipId::HorizonCruiser, 4, 0),
                    blocked: false,
                },
                RoundEvent::Mark {
                    side: E,
                    ship: P_COMMAND_3,
                },
                RoundEvent::Railgun {
                    source: pendulum,
                    target: P_COMMAND_3,
                    amount: 0,
                    after: stats(ShipId::RepairDrone, 0, 0),
                    blocked: true,
                },
            ],
        ),
        special(
            "round",
            "Grundmanöver und Phasentausch",
            fleet(&[mirage], &[]),
            vec![
                RoundEvent::Maneuver {
                    side: E,
                    ship: E_STRIKE_2,
                    to: SlotTarget::new(Command, 0),
                },
                RoundEvent::Swap {
                    side: P,
                    a: mirage,
                    b: P_COMMAND_3,
                },
            ],
        ),
        special(
            "round",
            "Fähigkeiten der Schiffe",
            fleet(&[bastion], &[(P_STRIKE_3, 3)]),
            vec![
                RoundEvent::Ability {
                    source: E_COMMAND_1,
                    ship: P_STRIKE_2,
                    effect: AbilityEffect::Damage { amount: 1 },
                    after: stats(ShipId::VoidInterceptor, 1, 0),
                },
                RoundEvent::Ability {
                    source: P_COMMAND_3,
                    ship: P_STRIKE_3,
                    effect: AbilityEffect::Heal { amount: 2 },
                    after: ShipStats { hp: 5, shield: 0 },
                },
                RoundEvent::Ability {
                    source: bastion,
                    ship: at(P, Strike, 2, ShipId::VoidInterceptor),
                    effect: AbilityEffect::Shield { amount: 2 },
                    after: stats(ShipId::VoidInterceptor, 1, 2),
                },
                RoundEvent::Ability {
                    source: E_STRIKE_2,
                    ship: E_STRIKE_2,
                    effect: AbilityEffect::Salvage,
                    after: stats(ShipId::ScrapRaider, 0, 0),
                },
            ],
        ),
        special(
            "round",
            "Taktvorsprung, Salve und Gegenfeuer",
            fleet(&[], &[(E_STRIKE_2, 2)]),
            vec![
                flagged(
                    attack(P_STRIKE_2, E_STRIKE_2, 2, ShipStats { hp: 0, shield: 0 }),
                    Flag::HeadStart,
                ),
                RoundEvent::Silenced { ship: E_STRIKE_2 },
                attack(
                    P_STRIKE_3,
                    E_STRIKE_3,
                    3,
                    stats(ShipId::IronDreadnought, 3, 0),
                ),
                flagged(
                    attack(
                        E_STRIKE_3,
                        P_STRIKE_3,
                        4,
                        stats(ShipId::HorizonCruiser, 4, 0),
                    ),
                    Flag::Focus,
                ),
                attack(
                    E_COMMAND_3,
                    P_STRIKE_3,
                    2,
                    stats(ShipId::HorizonCruiser, 6, 0),
                ),
                attack(
                    E_COMMAND_1,
                    P_STRIKE_2,
                    2,
                    stats(ShipId::VoidInterceptor, 2, 0),
                ),
                RoundEvent::Counterfire {
                    source: P_STRIKE_3,
                    ship: E_STRIKE_3,
                    amount: 1,
                    after: stats(ShipId::IronDreadnought, 4, 0),
                },
                destroyed(E_STRIKE_2),
            ],
        ),
    ]
}
