//! Vorschau-Szenen ohne Spieler (wie `client/dev/*.html` im Original), gewählt mit `VOIDFLEET_DEV_SCENE`:
//!
//! - `board` — das erste Gefecht eines Runs mit Kerr (fester Seed): Absichten, Hand, Stationen.
//! - `hit` — zwei Schiffe beschießen sich abwechselnd: Mündungsimpuls, Bolzen, Treffer-Blitz, Einschlag, Schwebetext;
//!   daneben ein Belagerer mit Rumpfbrüchen, das Kreuzer-Paar im Notfall (ein Modell ausgebrannt und brennend);
//!   `VOIDFLEET_DEV_HIT=sparks|fireball|discharge|pierce|fire` erzwingt eine Variante.
//! - `destroy` — die Schiffe gehen nacheinander unter (darunter zwei Staffeln) und landen danach neu.
//! - `arrive` — Schiffe fallen auf beiden Seiten aus dem Hyperraum in ihre Slots.
//! - `move` — Spaltenwechsel des Flaggschiffs und Traktorstrahl auf der Gegnerseite.
//! - `effects` — Railgun-Strahl, Rakete, Traktorstrahl, Schildhülle an und aus, Zielmarke und verpuffender Bolzen im Wechsel.
//! - `report` — die erzählte Wiedergabe in Dauerschleife mit erfundenen Ereignissen ([`super::report`]).
//! - `show` — ein Modell zum Ansehen, als eigenes Schiff in der Mitte und als Gegner gegenüber (in seiner Gruppe:
//!   Staffel, Paar oder einzeln); `VOIDFLEET_DEV_SHIP=<dateiname>` wählt es (wie `models/ships/<dateiname>.glb`, auch
//!   Entwürfe), ohne Angabe die Bastion.
//!
//! Außer `board` und `report` stellen die Szenen ihre Schiffe selbst und legen Controller und Abgleich still ([`Scripted`]).

use bevy::{diagnostic::FrameCount, prelude::*, time::Stopwatch};
use voidfleet_core::{
    admirals::{AdmiralId, FlagshipId},
    board::{EnemySlot, Row},
    enemies::EnemyId,
};

use crate::{
    assets::AppState,
    controller::Scripted,
    data::ship_models::{ShipKind, ShipModels, model_key, ships},
    game::Battle,
    palette,
    scene::{
        animation::{Animation, AnimationFinished},
        board::{BoardEntities, OnSlot, anchor_position, place},
        camera::BoardLayout,
        coords::SlotRef,
        destruction::DestroyShip,
        effects::{
            Beam, Bolt, Fizzle, HitVariant, Impact, Missile, Pulse, ReleaseTether, TargetRing,
            Tether,
        },
        floating_text::{FloatTone, FloatingText},
        motion::{Arrive, HitFlash, MoveShip},
        ship::{Ship, spawn_ship},
        status::ShipStatus,
    },
};

/// Die gewählte Szene.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevScene {
    Board,
    Hit(Option<HitVariant>),
    Destroy,
    Arrive,
    Move,
    Effects,
    Report,
    Show(ShipKind),
}

impl DevScene {
    pub fn from_env() -> Option<Self> {
        let hit = std::env::var("VOIDFLEET_DEV_HIT").ok().and_then(|name| {
            HitVariant::ALL
                .into_iter()
                .find(|variant| format!("{variant:?}").eq_ignore_ascii_case(&name))
        });
        match std::env::var("VOIDFLEET_DEV_SCENE").ok()?.as_str() {
            "board" => Some(Self::Board),
            "hit" => Some(Self::Hit(hit)),
            "destroy" => Some(Self::Destroy),
            "arrive" => Some(Self::Arrive),
            "move" => Some(Self::Move),
            "effects" => Some(Self::Effects),
            "report" => Some(Self::Report),
            "show" => {
                let name = std::env::var("VOIDFLEET_DEV_SHIP").unwrap_or_else(|_| "bastion".into());
                let kind = ships().find(|kind| model_key(*kind) == name);
                Some(Self::Show(
                    kind.unwrap_or_else(|| panic!("unbekanntes Modell: {name}")),
                ))
            }
            other => panic!("unbekannte Dev-Szene: {other}"),
        }
    }
}

/// Takt der Schleife: Zeit seit dem Start und wie viele Schritte schon liefen.
#[derive(Resource, Default)]
struct Loop {
    stopwatch: Stopwatch,
    step: u32,
}

/// Abstand zwischen zwei Schritten der Schleife je Szene.
fn period(scene: DevScene) -> f32 {
    match scene {
        DevScene::Board | DevScene::Report | DevScene::Show(_) => f32::INFINITY,
        DevScene::Hit(_) => 1.3,
        DevScene::Destroy => 2.6,
        DevScene::Arrive => 2.2,
        DevScene::Move => 2.4,
        DevScene::Effects => 1.6,
    }
}

pub(super) fn plugin(app: &mut App, scene: DevScene) {
    if scene == DevScene::Report {
        app.add_systems(OnEnter(AppState::Match), super::report::stage)
            .add_systems(
                Update,
                super::report::run.run_if(resource_exists::<crate::game::Battle>),
            );
        return;
    }
    if scene != DevScene::Board {
        app.insert_resource(Scripted);
    }
    app.insert_resource(scene)
        .init_resource::<Loop>()
        .add_systems(OnEnter(AppState::Match), stage)
        // Vor dem Abgleich, damit er neue und umgehängte Schiffe schon auf ihren Slots sieht.
        .add_systems(
            Update,
            run.run_if(in_state(AppState::Match))
                .before(crate::scene::sync::SyncSystems),
        );
}

fn own(column: usize) -> SlotRef {
    SlotRef::player(column)
}

fn enemy(row: Row, column: usize) -> SlotRef {
    SlotRef::enemy(EnemySlot::new(row, column))
}

/// Mitte eines Schiffs auf einem Slot (wie `centerOf` in der Rundenwiedergabe).
fn center(layout: &BoardLayout, slot: SlotRef) -> Vec3 {
    anchor_position(layout, slot) + Vec3::Y * 0.15
}

/// Stellt ein Schiff mit Hülle und Schild auf einen Slot.
fn stage_ship(
    commands: &mut Commands,
    models: &ShipModels,
    board: &BoardEntities,
    kind: ShipKind,
    slot: SlotRef,
    (hull, max_hull, shield): (i32, i32, i32),
) -> Entity {
    let ship = spawn_ship(commands, models, kind);
    place(commands, board, ship, slot);
    commands
        .entity(ship)
        .insert(ShipStatus::new(hull, max_hull, shield));
    ship
}

const BASTION: ShipKind = ShipKind::Flagship(FlagshipId::Bastion);
const CRUISER: ShipKind = ShipKind::Enemy(EnemyId::GuardCruiser);
const BESIEGER: ShipKind = ShipKind::Enemy(EnemyId::Besieger);

/// Die Slots der Szene `destroy`, reihum.
fn destroy_slots() -> [SlotRef; 6] {
    [
        own(0),
        enemy(Row::Front, 0),
        enemy(Row::Back, 1),
        enemy(Row::Front, 2),
        own(2),
        enemy(Row::Back, 2),
    ]
}

fn stage(
    mut commands: Commands,
    scene: Res<DevScene>,
    models: Res<ShipModels>,
    board: Res<BoardEntities>,
) {
    let mut put = |kind, slot, stats| {
        stage_ship(&mut commands, &models, &board, kind, slot, stats);
    };
    match *scene {
        DevScene::Report => {}
        DevScene::Show(kind) => {
            put(kind, own(1), (10, 10, 0));
            put(kind, enemy(Row::Front, 1), (10, 10, 0));
        }
        DevScene::Board => {
            let run = super::dev_run(AdmiralId::Kerr, true);
            let battle = Battle(run.start_battle());
            commands.insert_resource(run);
            commands.insert_resource(battle);
        }
        DevScene::Hit(_) => {
            put(BASTION, own(1), (52, 70, 6));
            // Im Notfall: ein Modell des Paars brennt ausgebrannt.
            put(CRUISER, enemy(Row::Front, 1), (3, 16, 0));
            put(BESIEGER, enemy(Row::Back, 2), (6, 30, 0));
        }
        DevScene::Destroy => {
            let kinds: Vec<ShipKind> = crate::data::ship_models::ships().collect();
            for (index, slot) in destroy_slots().into_iter().enumerate() {
                put(kinds[index * 2 % kinds.len()], slot, (10, 10, 0));
            }
        }
        DevScene::Arrive | DevScene::Effects => {
            put(BASTION, own(1), (70, 70, 0));
            put(
                ShipKind::Enemy(EnemyId::Spotter),
                enemy(Row::Back, 2),
                (12, 12, 0),
            );
            put(CRUISER, enemy(Row::Front, 1), (16, 16, 0));
        }
        DevScene::Move => {
            put(
                ShipKind::Flagship(FlagshipId::Vanguard),
                own(0),
                (50, 50, 0),
            );
            put(
                ShipKind::Enemy(EnemyId::Raiders),
                enemy(Row::Back, 0),
                (8, 8, 0),
            );
            put(
                ShipKind::Enemy(EnemyId::Carrier),
                enemy(Row::Front, 2),
                (26, 26, 0),
            );
        }
    }
}

fn run(
    mut commands: Commands,
    scene: Res<DevScene>,
    mut clock: ResMut<Loop>,
    time: Res<Time>,
    frame: Res<FrameCount>,
    layout: Res<BoardLayout>,
    board: Res<BoardEntities>,
    models: Res<ShipModels>,
    ships: Query<(Entity, &Ship, &OnSlot)>,
) {
    clock.stopwatch.tick(time.delta());
    // Kurz Luft lassen, bis Brett und Modelle stehen.
    let due = 1.0 + clock.step as f32 * period(*scene);
    if clock.stopwatch.elapsed_secs() < due {
        return;
    }
    let step = clock.step;
    clock.step += 1;
    let ship_on = |slot: SlotRef| {
        ships
            .iter()
            .find(|(_, _, on)| on.0 == slot)
            .map(|(entity, ship, _)| (entity, ship.kind))
    };
    match *scene {
        DevScene::Board | DevScene::Report | DevScene::Show(_) => {}
        DevScene::Hit(variant) => {
            let (from, to, color) = if step.is_multiple_of(2) {
                (own(1), enemy(Row::Front, 1), palette::PLAYER)
            } else {
                (enemy(Row::Front, 1), own(1), palette::ENEMY)
            };
            let origin = center(&layout, from);
            let target = center(&layout, to);
            let impact = target - (target - origin).normalize() * 0.45;
            let Some((victim, _)) = ship_on(to) else {
                return;
            };
            commands.spawn(Pulse {
                position: origin,
                color,
                size: 0.7,
                duration: 0.18,
            });
            commands
                .spawn(Bolt {
                    from: origin,
                    to: impact,
                    color,
                })
                .observe(move |_: On<AnimationFinished>, mut commands: Commands| {
                    commands.spawn(HitFlash::new(victim));
                    commands.spawn(Impact {
                        position: impact,
                        origin,
                        color,
                        variant,
                    });
                    commands.spawn(FloatingText::new(
                        target + Vec3::Y * 0.9,
                        "-3",
                        FloatTone::Damage,
                    ));
                });
        }
        DevScene::Destroy => {
            let slots = destroy_slots();
            let slot = slots[step as usize % slots.len()];
            if let Some((ship, kind)) = ship_on(slot) {
                info!("Dev-Szene: {slot:?} geht unter (Bild {})", frame.0);
                commands.spawn(DestroyShip { ship }).observe(
                    move |_: On<AnimationFinished>, mut commands: Commands| {
                        // Nach dem Verglühen der Trümmer landet ein neues Schiff.
                        commands.spawn(Animation::new(3.0)).observe(
                            move |_: On<AnimationFinished>,
                                  mut commands: Commands,
                                  models: Res<ShipModels>,
                                  board: Res<BoardEntities>| {
                                let ship = stage_ship(
                                    &mut commands,
                                    &models,
                                    &board,
                                    kind,
                                    slot,
                                    (10, 10, 0),
                                );
                                commands.spawn(Arrive { ship });
                            },
                        );
                    },
                );
            }
        }
        DevScene::Arrive => {
            let slots = [own(2), enemy(Row::Front, 0), own(0), enemy(Row::Back, 1)];
            let slot = slots[step as usize % slots.len()];
            if let Some((old, _)) = ship_on(slot) {
                commands.entity(old).despawn();
            }
            let kinds: Vec<ShipKind> = crate::data::ship_models::ships().collect();
            let kind = kinds[step as usize % kinds.len()];
            let ship = stage_ship(&mut commands, &models, &board, kind, slot, (10, 10, 0));
            commands.spawn(Arrive { ship });
        }
        DevScene::Move => {
            let (from, to, towed) = match step % 4 {
                0 => (own(0), own(2), false),
                1 => (own(2), own(1), false),
                2 => (own(1), own(0), false),
                _ => (enemy(Row::Back, 0), enemy(Row::Back, 1), true),
            };
            if let Some((ship, _)) = ship_on(from) {
                commands.spawn(MoveShip { ship, to, towed });
            } else if let Some((ship, _)) = ship_on(to) {
                commands.spawn(MoveShip {
                    ship,
                    to: from,
                    towed,
                });
            }
        }
        DevScene::Effects => {
            let origin = center(&layout, own(1));
            let target = center(&layout, enemy(Row::Front, 1));
            match step % 6 {
                0 => {
                    commands.spawn(Beam {
                        from: origin,
                        to: target,
                        color: palette::PLAYER,
                        width: 1.0,
                    });
                }
                1 => {
                    commands.spawn(Missile {
                        from: origin,
                        to: target,
                        color: palette::ENEMY,
                    });
                }
                2 => {
                    if let Some((ship, _)) = ship_on(enemy(Row::Front, 1)) {
                        let tether = commands
                            .spawn(Tether {
                                from: origin,
                                target: ship,
                                offset: Vec3::Y * 0.15,
                                color: palette::PLAYER,
                            })
                            .id();
                        commands.spawn(Animation::new(1.2)).observe(
                            move |_: On<AnimationFinished>, mut commands: Commands| {
                                commands.trigger(ReleaseTether { entity: tether });
                            },
                        );
                    }
                }
                3 => {
                    // Schildhülle an und aus.
                    if let Some((ship, _)) = ship_on(own(1)) {
                        commands
                            .entity(ship)
                            .entry::<ShipStatus>()
                            .and_modify(|mut status| {
                                *status = status.with_shield(if status.shield > 0 { 0 } else { 4 });
                            });
                    }
                }
                4 => {
                    commands.spawn(TargetRing {
                        position: target,
                        color: palette::ENEMY,
                    });
                }
                _ => {
                    commands.spawn(Fizzle {
                        from: origin,
                        to: center(&layout, enemy(Row::Back, 2)),
                        color: palette::PLAYER,
                    });
                }
            }
        }
    }
}
