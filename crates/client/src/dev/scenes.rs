//! Vorschau-Szenen ohne Spieler (wie `client/dev/*.html` im Original), gewählt mit `VOIDFLEET_DEV_SCENE`:
//!
//! - `board` — ein Brett mitten in der Planung: Schiffe beider Seiten mit Schaden und Schild, ein eingeplantes Schiff als
//!   Geist, Grundmanöver mit Pfeil, Torpedo mit Fadenkreuz, Feuerbefehle mit Bögen der Zielvorschau, Marken
//!   (Flaggschiff-Ringe, Zielerfassung, Bereitschaft, Visier), Pfeile vor den Spalten, leuchtende Ziel-Slots, Gitter.
//! - `hit` — zwei Schiffe beschießen sich abwechselnd: Mündungsimpuls, Bolzen, Treffer-Blitz, Einschlag, Schwebetext;
//!   `VOIDFLEET_DEV_HIT=sparks|fireball|discharge|pierce|fire` erzwingt eine Variante.
//! - `destroy` — die Schiffe gehen nacheinander unter und landen danach neu (Abgleich mit dem Spielzustand, Deploy).
//! - `arrive` — Schiffe fallen auf beiden Seiten aus dem Hyperraum in ihre Slots.
//! - `move` — Lane-Wechsel nach vorn und hinten, seitlich innerhalb der Lane, gezogen.
//! - `effects` — Railgun-Strahl, Rakete, Traktorstrahl, Schildblase, Zielmarke und verpuffender Bolzen im Wechsel.
//!
//! Der Server bleibt angehalten ([`ServerInbox::hold`]); der Spielzustand ist erfunden.

use bevy::{diagnostic::FrameCount, prelude::*, time::Stopwatch};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::{CommandId, SHIPS, ShipId},
    protocol::{CommandPlan, FireOrder, FireTarget, Maneuver, OpponentView},
};

use crate::{
    assets::AppState,
    data::ship_models::ShipModels,
    game::{Match, ServerInbox, ShipPick},
    palette,
    scene::{
        animation::{Animation, AnimationFinished},
        board::{BoardEntities, Highlights, OnSlot, anchor_position, place},
        camera::BoardLayout,
        destruction::DestroyShip,
        effects::{
            Beam, Bolt, Bubble, Fizzle, HitVariant, Impact, Missile, Pulse, ReleaseTether,
            TargetRing, Tether,
        },
        floating_text::{FloatTone, FloatingText},
        motion::{Arrive, HitFlash, MoveShip},
        ship::{Ship, spawn_ship},
        sync::BoardFocus,
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
        DevScene::Board => f32::INFINITY,
        DevScene::Hit(_) => 1.3,
        DevScene::Destroy => 2.6,
        DevScene::Arrive => 2.2,
        DevScene::Move => 2.4,
        DevScene::Effects => 1.6,
    }
}

pub(super) fn plugin(app: &mut App, scene: DevScene) {
    app.insert_resource(scene)
        .insert_resource(crate::controller::Scripted)
        .init_resource::<Loop>()
        .add_systems(OnEnter(AppState::Match), stage)
        // Vor dem Abgleich, damit er neue und umgehängte Schiffe schon auf ihren Slots sieht.
        .add_systems(
            Update,
            run.run_if(in_state(AppState::Match))
                .before(crate::scene::sync::SyncSystems),
        );
}

fn own(lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(Side::Player, lane, index)
}

fn enemy(lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(Side::Enemy, lane, index)
}

/// Mitte eines Schiffs auf einem Slot (wie `centerOf` in der Rundenwiedergabe).
fn center(layout: &BoardLayout, slot: SlotRef) -> Vec3 {
    anchor_position(layout, slot) + Vec3::Y * 0.15
}

/// Ein Match in der dritten Runde, Admiral Osei gegen Admiralin Kerr; `ships` steht auf dem Brett.
fn staged_match(ships: &[(SlotRef, ShipId, i32, i32)]) -> Match {
    let mut game = Match::default();
    game.round = 3;
    game.admiral = Some(AdmiralId::Osei);
    game.opponent = Some(OpponentView {
        admiral: Some(AdmiralId::Kerr),
        submitted: false,
        ship_hand_size: 3,
        command_hand_size: 3,
        ship_deck_size: 10,
        command_deck_size: 10,
        bot: true,
        played_commands: Vec::new(),
    });
    for &(slot, card, hp, shield) in ships {
        let ship = game.board.put(slot, card);
        ship.hp = hp;
        ship.shield = shield;
    }
    game
}

fn stage(
    mut inbox: ResMut<ServerInbox>,
    scene: Res<DevScene>,
    mut game: ResMut<Match>,
    mut focus: ResMut<BoardFocus>,
    mut highlights: ResMut<Highlights>,
) {
    inbox.hold();
    use LaneType::{Command, Strike};
    match *scene {
        DevScene::Board => {
            let mut staged = staged_match(&[
                (own(Strike, 1), ShipId::IronDreadnought, 5, 2),
                (own(Strike, 3), ShipId::PlasmaFrigate, 4, 0),
                (own(Command, 1), ShipId::RepairDrone, 3, 0),
                (own(Command, 2), ShipId::Bastion, 12, 3),
                (own(Command, 4), ShipId::SignalBooster, 2, 0),
                (enemy(Strike, 1), ShipId::VoidInterceptor, 2, 0),
                (enemy(Strike, 2), ShipId::HorizonCruiser, 5, 1),
                (enemy(Strike, 4), ShipId::TitanCarrier, 6, 0),
                (enemy(Command, 3), ShipId::Pendulum, 9, 0),
                (enemy(Command, 4), ShipId::ScrapRaider, 3, 0),
            ]);
            staged.board[enemy(Strike, 2)]
                .as_mut()
                .expect("Kreuzer")
                .marked = true;
            staged.ship_hand = vec![ShipId::VoidInterceptor, ShipId::ScrapRaider];
            staged.command_hand = vec![CommandId::Torpedo, CommandId::FlankingManeuver];
            staged.plan_ship(ShipPick::Hand(0), SlotTarget::new(Strike, 5));
            staged.set_command(Some(CommandPlan {
                hand_index: 0,
                target: Some(SlotTarget::new(Strike, 4)),
                destination: None,
            }));
            staged.set_maneuver(Some(Maneuver {
                ship: SlotTarget::new(Strike, 3),
                destination: SlotTarget::new(Strike, 2),
            }));
            let dreadnought = SlotTarget::new(Strike, 1);
            staged.set_fire(
                dreadnought,
                Some(FireOrder {
                    ship: dreadnought,
                    target: FireTarget::Ship(SlotTarget::new(Strike, 1)),
                }),
            );
            let bastion = SlotTarget::new(Command, 2);
            staged.set_fire(
                bastion,
                Some(FireOrder {
                    ship: bastion,
                    target: FireTarget::Column(4),
                }),
            );
            focus.shooter = Some(dreadnought);
            highlights.slots = staged.board.ships(Side::Enemy).collect();
            highlights.grid = true;
            *game = staged;
        }
        DevScene::Hit(_) => {
            *game = staged_match(&[
                (own(Strike, 3), ShipId::IronDreadnought, 7, 2),
                (enemy(Strike, 3), ShipId::HorizonCruiser, 5, 1),
                (own(Command, 2), ShipId::PlasmaFrigate, 4, 0),
                (enemy(Command, 4), ShipId::RepairDrone, 3, 0),
            ]);
        }
        DevScene::Destroy => {
            let slots = [
                own(Strike, 1),
                enemy(Strike, 1),
                own(Strike, 3),
                enemy(Strike, 3),
                own(Command, 2),
                enemy(Command, 2),
                own(Command, 4),
                enemy(Command, 4),
            ];
            let ships: Vec<_> = slots
                .into_iter()
                .zip(SHIPS)
                .map(|(slot, ship)| (slot, ship, ship.hp(), 0))
                .collect();
            *game = staged_match(&ships);
        }
        DevScene::Arrive | DevScene::Effects => {
            *game = staged_match(&[
                (own(Command, 2), ShipId::RepairDrone, 3, 0),
                (enemy(Command, 4), ShipId::SignalBooster, 2, 0),
                (enemy(Strike, 2), ShipId::HorizonCruiser, 5, 0),
            ]);
        }
        DevScene::Move => {
            *game = staged_match(&[
                (own(Strike, 2), ShipId::VoidInterceptor, 3, 0),
                (own(Command, 4), ShipId::IronDreadnought, 7, 0),
                (enemy(Strike, 3), ShipId::TitanCarrier, 6, 0),
            ]);
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
    mut game: ResMut<Match>,
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
            .find(|(_, ship, on)| !ship.ghost && on.0 == slot)
            .map(|(entity, ..)| entity)
    };
    use LaneType::{Command, Strike};
    match *scene {
        DevScene::Board => {}
        DevScene::Hit(variant) => {
            let (from, to, color) = if step.is_multiple_of(2) {
                (own(Strike, 3), enemy(Strike, 3), palette::PLAYER)
            } else {
                (enemy(Strike, 3), own(Strike, 3), palette::ENEMY)
            };
            let origin = center(&layout, from);
            let target = center(&layout, to);
            let impact = target - (target - origin).normalize() * 0.45;
            let Some(victim) = ship_on(to) else {
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
                    commands.spawn(FloatingText {
                        at: target + Vec3::Y * 0.9,
                        text: "-3".into(),
                        tone: FloatTone::Damage,
                    });
                });
        }
        DevScene::Destroy => {
            let slots = [
                own(Strike, 1),
                enemy(Strike, 1),
                own(Strike, 3),
                enemy(Strike, 3),
                own(Command, 2),
                enemy(Command, 2),
                own(Command, 4),
                enemy(Command, 4),
            ];
            let slot = slots[step as usize % slots.len()];
            if let (Some(ship), Some(card)) = (ship_on(slot), game.board.card(slot)) {
                info!("Dev-Szene: {slot:?} geht unter (Bild {})", frame.0);
                game.board[slot] = None;
                commands.spawn(DestroyShip { ship }).observe(
                    move |_: On<AnimationFinished>, mut commands: Commands| {
                        // Nach dem Verglühen der Trümmer stellt der Abgleich das Schiff wieder hin.
                        commands.spawn(Animation::new(3.0)).observe(
                            move |_: On<AnimationFinished>, mut game: ResMut<Match>| {
                                game.board.put(slot, card);
                            },
                        );
                    },
                );
            }
        }
        DevScene::Arrive => {
            let slots = [
                own(Strike, 2),
                enemy(Strike, 4),
                own(Command, 0),
                enemy(Strike, 1),
            ];
            let slot = slots[step as usize % slots.len()];
            if let Some(old) = ship_on(slot) {
                commands.entity(old).despawn();
            }
            let card = SHIPS[step as usize % SHIPS.len()];
            let ship = spawn_ship(&mut commands, &models, card, false);
            place(&mut commands, &board, ship, slot);
            commands.spawn(Arrive { ship });
            // Der Spielzustand kennt das neue Schiff, damit der Abgleich es stehen lässt.
            game.board.put(slot, card);
        }
        DevScene::Move => {
            let (from, to, towed) = match step % 4 {
                0 => (own(Strike, 2), own(Command, 0), false),
                1 => (own(Command, 0), own(Strike, 4), false),
                2 => (own(Strike, 4), own(Strike, 2), false),
                _ => (own(Command, 4), own(Command, 1), true),
            };
            let back = (to, from);
            if let Some(ship) = ship_on(from) {
                let card = game.board.card(from).expect("Schiff");
                game.board[from] = None;
                game.board.put(to, card);
                commands.spawn(MoveShip { ship, to, towed });
            } else if let Some(ship) = ship_on(back.0) {
                let card = game.board.card(back.0).expect("Schiff");
                game.board[back.0] = None;
                game.board.put(back.1, card);
                commands.spawn(MoveShip {
                    ship,
                    to: back.1,
                    towed,
                });
            }
        }
        DevScene::Effects => {
            let origin = center(&layout, own(Strike, 2));
            let target = center(&layout, enemy(Strike, 2));
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
                    if let Some(ship) = ship_on(enemy(Strike, 2)) {
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
                    commands.spawn(Bubble {
                        position: center(&layout, own(Command, 2)),
                        color: palette::admiral_color(AdmiralId::Osei),
                        size: 2.4,
                    });
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
                        to: center(&layout, enemy(Strike, 5)),
                        color: palette::PLAYER,
                    });
                }
            }
        }
    }
}
