//! Die Szene spiegelt den Spielzustand (Port von `syncBoard`, `refreshGhosts`, `refreshMarkers`, `showPreview` und
//! `refreshHoldFire` aus `matchController.ts`), solange keine Runde abgespielt wird ([`BoardMode::Planning`]):
//!
//! - [`sync_ships`]: je Slot das Schiff aus dem Brett von [`Match`] samt HP und Schild; fehlt eines, entsteht es, steht
//!   ein anderes da, wird es ersetzt, ist der Slot leer, verschwindet es.
//! - [`sync_plan`]: was der Zug zeigt — Geister der eingeplanten Schiffe (auch Flaggschiff, verlegte Schiffe samt
//!   Tauschpartner beim Phasentausch, Trugbild und die Drohnen eingeplanter Träger), Bewegungspfeile, Fadenkreuze auf
//!   Zielen der Gegnerseite, Marken (Zielerfassung, Fähigkeit, Bereitschaft und Visier der Feuerbefehle), die
//!   Zielvorschau aus [`preview_round`] (Bögen, drohender Verlust auf den Balken, Totenkopf) und gedämpfte Balken des
//!   manövrierenden Schiffs.
//!
//! Der Controller lenkt die Darstellung über [`BoardFocus`]: Unter dem Zeiger zeigt die Szene statt des eingeplanten
//! Zugs eine Variante davon (`hypothetical`, etwa `game.clone()` mit dem Schiff auf dem Slot unter dem Zeiger), und der
//! gewählte Schütze trägt das Visier, seine Bögen sind hervorgehoben, vor den gegnerischen Spalten liegen die Pfeile.
//! Die Wiedergabe schaltet auf [`BoardMode::Playback`]; dann verschwinden Geister, Pfeile, Bögen und Vorschau, und die
//! Szene gehört ihr. Zurück in der Planung gleicht sich alles dem Zustand an.

use bevy::{platform::collections::HashMap, prelude::*};
use voidfleet_core::{
    admirals::AdmiralId,
    board::{LaneType, Side, SlotRef, SlotTarget},
    cards::ShipId,
    protocol::FireTarget,
};

use super::{
    board::{BoardEntities, ColumnMarks, GHOST_LIFT, OnSlot, all_slots},
    markers::{MarkerKind, MarkerSpec, ShipMarkers},
    overlays::{MoveArrow, PlanOverlays, TargetLine},
    ship::{BOARD_SCALE, Ship, spawn_ship},
    status::ShipStatus,
};
use crate::{
    assets::AppState,
    data::{admirals::power_admiral, ship_models::ShipModels},
    game::{Match, RoundPreview, ShotOutcome, preview_round},
    palette::{self, admiral_color},
};

/// Wem die Szene gehört: der Planung, die sie aus dem Spielzustand stellt, oder der Rundenwiedergabe.
#[derive(SubStates, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::Match)]
pub enum BoardMode {
    #[default]
    Planning,
    Playback,
}

/// Was der Controller beim Planen hervorhebt.
#[derive(Resource, Clone, Debug, Default)]
pub struct BoardFocus {
    /// Der Zug, wie er nach dem Klick unter dem Zeiger aussähe (eine geänderte Kopie von [`Match`]); `None`: der
    /// eingeplante.
    pub hypothetical: Option<Match>,
    /// Das gewählte eigene Schiff, das gerade sein Ziel wählt (Slot beim Planen).
    pub shooter: Option<SlotTarget>,
}

/// Die Systeme des Abgleichs; wer danach die Szene ändert, ordnet sich dahinter.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SyncSystems;

pub(super) struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<BoardMode>()
            .init_resource::<BoardFocus>()
            .add_systems(
                OnEnter(BoardMode::Planning),
                (sync_ships, sync_plan).chain(),
            )
            .add_systems(OnEnter(BoardMode::Playback), clear_plan)
            .add_systems(
                Update,
                (
                    sync_ships.run_if(resource_changed::<Match>),
                    sync_plan
                        .run_if(resource_changed::<Match>.or_eager(resource_changed::<BoardFocus>)),
                )
                    .chain()
                    .in_set(SyncSystems)
                    .run_if(in_state(BoardMode::Planning)),
            );
    }
}

/// Stellt je Slot das Schiff aus dem Brett hin, mit seinen Werten.
pub fn sync_ships(
    mut commands: Commands,
    game: Res<Match>,
    models: Res<ShipModels>,
    board: Res<BoardEntities>,
    mut ships: Query<(Entity, &Ship, &OnSlot, &mut ShipStatus)>,
) {
    let mut standing: HashMap<SlotRef, (Entity, ShipId)> = HashMap::default();
    for (entity, ship, on, _) in &ships {
        if !ship.ghost {
            standing.insert(on.0, (entity, ship.id));
        }
    }
    for slot in all_slots() {
        let wanted = game.board.state(slot);
        let here = standing.get(&slot).copied();
        match (wanted, here) {
            (Some(state), Some((entity, id))) if id == state.card_id => {
                if let Ok((_, _, _, mut status)) = ships.get_mut(entity)
                    && (status.hp, status.shield) != (state.hp, state.shield)
                {
                    status.hp = state.hp;
                    status.shield = state.shield;
                }
            }
            (wanted, here) => {
                if let Some((entity, _)) = here {
                    commands.entity(entity).despawn();
                }
                if let Some(state) = wanted {
                    let ship = spawn_ship(&mut commands, &models, state.card_id, false);
                    commands.entity(ship).insert((
                        OnSlot(slot),
                        ChildOf(board.anchor(slot)),
                        Transform::from_scale(Vec3::splat(BOARD_SCALE)),
                        ShipStatus {
                            hp: state.hp,
                            shield: state.shield,
                            ..ShipStatus::new(state.card_id)
                        },
                    ));
                }
            }
        }
    }
}

/// Wo nach dem Zug Geister stehen: zuerst die Drohnen eingeplanter Träger (ein späterer Geist auf ihrem Slot verdrängt
/// sie), dann die eingeplanten Schiffe, die verlegten (beim Phasentausch auch der Tauschpartner) und das Trugbild.
fn ghost_targets(shown: &Match, drones: &[SlotRef]) -> Vec<(SlotRef, ShipId)> {
    let mut ghosts: Vec<(SlotRef, ShipId)> =
        drones.iter().map(|slot| (*slot, ShipId::Drone)).collect();
    for arrival in shown.arrivals() {
        ghosts.push((arrival.target.on(Side::Player), shown.card_of(arrival.pick)));
    }
    for planned in [shown.planned_command_move(), shown.planned_maneuver()]
        .into_iter()
        .flatten()
    {
        if let Some(ship) = shown.ship_at(planned.from) {
            ghosts.push((planned.to, ship));
        }
        if let Some(partner) = shown.board.card(planned.to) {
            ghosts.push((planned.from, partner));
        }
    }
    if let Some(decoy) = shown.planned_decoy() {
        ghosts.push((decoy.on(Side::Player), ShipId::Decoy));
    }
    // Ein späterer Geist auf demselben Slot ersetzt den früheren.
    let mut unique: Vec<(SlotRef, ShipId)> = Vec::new();
    for (slot, ship) in ghosts {
        unique.retain(|(other, _)| *other != slot);
        unique.push((slot, ship));
    }
    unique
}

/// Stellt Geister, Pfeile, Fadenkreuze, Marken und die Zielvorschau des gezeigten Zugs.
pub fn sync_plan(
    mut commands: Commands,
    game: Res<Match>,
    focus: Res<BoardFocus>,
    models: Res<ShipModels>,
    board: Res<BoardEntities>,
    mut ships: Query<(
        Entity,
        &Ship,
        &OnSlot,
        &mut ShipStatus,
        Option<&mut ShipMarkers>,
    )>,
    mut overlays: ResMut<PlanOverlays>,
    mut columns: ResMut<ColumnMarks>,
) {
    let shown = focus.hypothetical.as_ref().unwrap_or(&game);
    let preview = game
        .round_running()
        .then(|| preview_round(shown, shown.plan()));
    let shooter = if game.turn_open() {
        focus.shooter
    } else {
        None
    };
    let losses: &[SlotRef] = preview.as_ref().map_or(&[], |preview| &preview.losses);

    // Geister: bleiben stehen, wenn sie schon richtig stehen.
    let wanted = if game.round_running() {
        ghost_targets(
            shown,
            preview.as_ref().map_or(&[][..], |preview| &preview.drones),
        )
    } else {
        Vec::new()
    };
    let mut standing: Vec<(SlotRef, ShipId, Entity)> = ships
        .iter()
        .filter(|(_, ship, ..)| ship.ghost)
        .map(|(entity, ship, on, ..)| (on.0, ship.id, entity))
        .collect();
    let same = standing.len() == wanted.len()
        && wanted.iter().all(|(slot, id)| {
            standing
                .iter()
                .any(|(on, ghost, _)| on == slot && ghost == id)
        });
    let mut ghost_at: HashMap<SlotRef, Entity> = HashMap::default();
    if same {
        ghost_at.extend(standing.iter().map(|(slot, _, entity)| (*slot, *entity)));
    } else {
        for (_, _, entity) in standing.drain(..) {
            commands.entity(entity).despawn();
        }
        for (slot, id) in &wanted {
            let ghost = spawn_ship(&mut commands, &models, *id, true);
            // Steht schon ein Schiff dort (Phasentausch), schwebt der Geist darüber.
            let lift = if game.board.is_occupied(*slot) {
                GHOST_LIFT
            } else {
                0.0
            };
            commands.entity(ghost).insert((
                OnSlot(*slot),
                ChildOf(board.anchor(*slot)),
                Transform::from_xyz(0.0, lift, 0.0).with_scale(Vec3::splat(BOARD_SCALE)),
                ShipStatus {
                    doomed: losses.contains(slot),
                    ..ShipStatus::new(*id)
                },
            ));
            ghost_at.insert(*slot, ghost);
        }
    }
    let real_at: HashMap<SlotRef, Entity> = ships
        .iter()
        .filter(|(_, ship, ..)| !ship.ghost)
        .map(|(entity, _, on, ..)| (on.0, entity))
        .collect();

    let mut markers = plan_markers(&game, shown, shooter, &real_at, &ghost_at);

    // Zielvorschau und gedämpfte Balken des manövrierenden Schiffs (nicht beim Phasentausch).
    let maneuvering = shown
        .planned_maneuver()
        .filter(|planned| !game.board.is_occupied(planned.to))
        .map(|planned| planned.from);
    for (entity, ship, on, mut status, ship_markers) in &mut ships {
        let wanted_markers = markers.remove(&entity).unwrap_or_default();
        match ship_markers {
            Some(mut ship_markers) => {
                ship_markers.set_if_neq(ShipMarkers(wanted_markers));
            }
            None => {
                commands.entity(entity).insert(ShipMarkers(wanted_markers));
            }
        }
        let next = if ship.ghost {
            ShipStatus {
                doomed: losses.contains(&on.0),
                ..*status
            }
        } else {
            ShipStatus {
                preview: preview
                    .as_ref()
                    .and_then(|preview| preview.outcomes.get(&on.0))
                    .map(|after| after.unwrap_or_default()),
                doomed: losses.contains(&on.0) && !ghost_at.contains_key(&on.0),
                holding_fire: maneuvering == Some(on.0),
                ..*status
            }
        };
        status.set_if_neq(next);
    }
    // Neue Geister sind noch nicht in der Abfrage.
    if !same {
        for entity in ghost_at.values() {
            let specs = markers.remove(entity).unwrap_or_default();
            commands.entity(*entity).insert(ShipMarkers(specs));
        }
    }

    overlays.set_if_neq(plan_overlays(&game, shown, preview.as_ref(), shooter));

    // Die Pfeile vor den Spalten, solange ein eigenes Schiff zielt; gefüllt der seines Spaltenfeuers.
    columns.set_if_neq(ColumnMarks {
        shown: shooter.is_some(),
        chosen: shooter
            .and_then(|shooter| shown.fire_order_at(shooter))
            .and_then(|order| match order.target {
                FireTarget::Column(column) => Some(column),
                FireTarget::Ship(_) => None,
            }),
    });
}

/// Marken je Schiff oder Geist: die Zielerfassung auf dem Brett (beide Seiten), die eigene Fähigkeit (auf dem Schiff,
/// das nach den Bewegungen dort steht), die eingeplante Zielerfassung, Visier und Bereitschaft der Feuerbefehle.
fn plan_markers(
    game: &Match,
    shown: &Match,
    shooter: Option<SlotTarget>,
    real_at: &HashMap<SlotRef, Entity>,
    ghost_at: &HashMap<SlotRef, Entity>,
) -> HashMap<Entity, Vec<MarkerSpec>> {
    let mut markers: HashMap<Entity, Vec<MarkerSpec>> = HashMap::default();
    let mut add = |entity: Option<&Entity>, spec: MarkerSpec| {
        if let Some(&entity) = entity {
            markers.entry(entity).or_default().push(spec);
        }
    };
    let kerr = admiral_color(AdmiralId::Kerr);
    for slot in all_slots() {
        if game.board.state(slot).is_some_and(|state| state.marked) {
            add(real_at.get(&slot), MarkerSpec::new(MarkerKind::Lock, kerr));
        }
    }
    if game.round_running() {
        if let Some(power) = shown.power()
            && let Some(kind) = MarkerKind::of_power(power)
            && let Some(target) = shown.planned_power_target()
        {
            let holder = match shown.planned_command_move() {
                Some(planned) if planned.from == target => ghost_at.get(&planned.to),
                _ => ghost_at.get(&target).or(real_at.get(&target)),
            };
            add(
                holder,
                MarkerSpec::new(kind, admiral_color(power_admiral(power))),
            );
        }
        if let Some(mark) = shown.plan().mark {
            add(
                real_at.get(&mark.on(Side::Enemy)),
                MarkerSpec {
                    planned: true,
                    ..MarkerSpec::new(MarkerKind::Lock, kerr)
                },
            );
        }
    }
    if game.turn_open() {
        for slot in shown.shooters() {
            let at = slot.on(Side::Player);
            let holder = real_at.get(&at).or(ghost_at.get(&at));
            if Some(slot) == shooter {
                add(holder, MarkerSpec::new(MarkerKind::Aim, palette::SELECT));
            } else if !shown.is_armed(slot) {
                add(holder, MarkerSpec::new(MarkerKind::Ready, palette::PLAYER));
            }
        }
    }
    markers
}

/// Bewegungspfeile, Fadenkreuze auf Zielen der Gegnerseite und die Bögen der Zielvorschau.
fn plan_overlays(
    game: &Match,
    shown: &Match,
    preview: Option<&RoundPreview>,
    shooter: Option<SlotTarget>,
) -> PlanOverlays {
    let mut next = PlanOverlays::default();
    if !game.round_running() {
        return next;
    }
    let maneuver = shown.planned_maneuver();
    for planned in [shown.planned_command_move(), maneuver]
        .into_iter()
        .flatten()
    {
        next.arrows.push(MoveArrow {
            from: planned.from,
            to: planned.to,
            lifted: game.board.is_occupied(planned.to),
        });
    }
    // Phasentausch: der Rückweg des Tauschpartners.
    if let Some(swap) = maneuver.filter(|planned| game.board.is_occupied(planned.to)) {
        next.arrows.push(MoveArrow {
            from: swap.to,
            to: swap.from,
            lifted: true,
        });
    }
    if let (Some(card), Some(target)) = (shown.planned_command(), shown.planned_enemy_target()) {
        next.reticles = if card.targets_enemy_column() {
            LaneType::ALL
                .map(|lane| SlotRef::new(Side::Enemy, lane, target.slot_index))
                .to_vec()
        } else {
            vec![target.on(Side::Enemy)]
        };
    }
    if let Some(preview) = preview {
        next.lines = preview
            .shots
            .iter()
            .map(|shot| TargetLine {
                from: shot.attacker,
                to: shot.target,
                side: shot.attacker.side,
                fizzle: matches!(shot.outcome, ShotOutcome::Miss(_)),
                tracking: shot.tracking,
                selected: shooter.is_some() && shot.shooter.map(|slot| slot.target()) == shooter,
            })
            .collect();
    }
    next
}

/// Die Wiedergabe übernimmt:Geister, Pfeile, Bögen, Vorschau und die Marken der Feuerbefehle gehen.
fn clear_plan(
    mut commands: Commands,
    mut ships: Query<(Entity, &Ship, &mut ShipStatus, Option<&mut ShipMarkers>)>,
    mut overlays: ResMut<PlanOverlays>,
    mut columns: ResMut<ColumnMarks>,
) {
    for (entity, ship, mut status, markers) in &mut ships {
        if ship.ghost {
            commands.entity(entity).despawn();
            continue;
        }
        status.preview = None;
        status.doomed = false;
        status.holding_fire = false;
        if let Some(mut markers) = markers {
            markers.remove(MarkerKind::Ready);
            markers.remove(MarkerKind::Aim);
        }
    }
    *overlays = PlanOverlays::default();
    *columns = ColumnMarks::default();
}

#[cfg(test)]
mod tests {
    use voidfleet_core::protocol::Maneuver;

    use super::*;
    use crate::game::ShipPick;

    /// Ein eingeplantes Schiff steht als Geist auf seinem Slot; beim Phasentausch stehen beide Schiffe als Geister im
    /// Slot des jeweils anderen.
    #[test]
    fn ghosts_follow_the_plan() {
        let mut game = Match::default();
        game.round = 2;
        game.admiral = Some(AdmiralId::Moreau);
        game.ship_hand = vec![ShipId::VoidInterceptor];
        let mirage = SlotRef::new(Side::Player, LaneType::Strike, 1);
        let partner = SlotRef::new(Side::Player, LaneType::Command, 1);
        game.board.put(mirage, ShipId::Mirage);
        game.board.put(partner, ShipId::RepairDrone);
        game.plan_ship(ShipPick::Hand(0), SlotTarget::new(LaneType::Strike, 3));
        game.set_maneuver(Some(Maneuver {
            ship: mirage.target(),
            destination: partner.target(),
        }));
        let ghosts = ghost_targets(&game, &[]);
        assert!(ghosts.contains(&(
            SlotRef::new(Side::Player, LaneType::Strike, 3),
            ShipId::VoidInterceptor
        )));
        assert!(ghosts.contains(&(partner, ShipId::Mirage)));
        assert!(ghosts.contains(&(mirage, ShipId::RepairDrone)));
    }
}
