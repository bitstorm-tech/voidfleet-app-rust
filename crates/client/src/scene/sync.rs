//! Die Szene spiegelt das Gefecht, solange keine Wiedergabe läuft ([`BoardMode::Planning`]):
//!
//! - je Gegner-Slot die Einheit aus [`Battle`] (Modell nach Katalog, Hülle, Schild) und das Flaggschiff in seiner
//!   Spalte, im Kurzsprung fehlt es; fehlt ein Schiff, entsteht es, steht es woanders, wechselt es den Slot, ist es
//!   weg, verschwindet es;
//! - die Vorschau aus [`BoardFocus`] (drohender Verlust und Totenkopf auf den Balken, die Rechnung am Ziel), das Visier
//!   unter dem Ziel der Station und über jedem Schiff Absicht und Zustände ([`ShipCaption`]);
//! - der Geist des Flaggschiffs ([`Ghost`]) in der Zielspalte, solange die Manöver-Station besetzt ist;
//! - Zustände mit eigenem Bild: Brand und Aufladung als Partikel ([`ShipAura`]), Ausweichen als Marke, der Schild im
//!   Balken; Brand, Notfall, Stillstand und Funkstille dazu als Icons in der Beschriftung.
//!
//! Der Controller schaltet nach jeder Aktion auf [`BoardMode::Playback`]; dann gehört die Szene der Wiedergabe, die
//! Vorschau geht. Zurück in der Planung gleicht sich alles dem Zustand an. Dev-Szenen mit [`Scripted`] stellen ihre
//! Schiffe selbst; dann ruht der Abgleich.

use bevy::prelude::*;
use voidfleet_core::{
    admirals::{AdmiralId, Station},
    battle::{Battle as CoreBattle, Enemy},
    events::{Hit, UnitRef, UnitStats},
};

use super::{
    aura::ShipAura,
    board::{BoardEntities, OnSlot, place},
    coords::{Side, SlotRef},
    labels::ShipCaption,
    markers::{MarkerKind, MarkerSpec, ShipMarkers},
    ship::{Ghost, Ship, ToonState, spawn_ghost, spawn_ship},
    status::ShipStatus,
};
use crate::{
    assets::AppState,
    controller::{
        Scripted,
        selection::{station_needs_target, unit_uid},
    },
    data::ship_models::{ShipKind, ShipModels},
    game::Battle,
    i18n::{Condition, Locale, Messages},
    palette,
    playback::PlayEvents,
};

/// Wem die Szene gehört: der Planung, die sie aus dem Gefecht stellt, oder der Wiedergabe.
#[derive(SubStates, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::Match)]
pub enum BoardMode {
    #[default]
    Planning,
    Playback,
}

/// Was der Controller unter dem Zeiger zeigt: die Treffer der gewählten Karte bzw. Station auf den Gegner `uid`.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct BoardFocus {
    pub preview: Option<(u32, Vec<Hit>)>,
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
                (sync_ships, sync_ghost).run_if(not(resource_exists::<Scripted>)),
            )
            .add_systems(OnEnter(BoardMode::Playback), (clear_preview, clear_ghost))
            .add_systems(
                Update,
                (sync_ships, sync_ghost)
                    .run_if(board_changed)
                    .in_set(SyncSystems)
                    .run_if(in_state(BoardMode::Planning))
                    // Der Wechsel zur Wiedergabe greift erst im nächsten Bild; so lange bleibt der alte Stand.
                    .run_if(not(on_message::<PlayEvents>))
                    .run_if(not(resource_exists::<Scripted>)),
            );
    }
}

/// Ob sich Gefecht, Vorschau oder Sprache geändert haben, auch ob das Gefecht gekommen oder gegangen ist.
fn board_changed(
    battle: Option<Res<Battle>>,
    focus: Res<BoardFocus>,
    locale: Res<Locale>,
    mut had_battle: Local<bool>,
) -> bool {
    let has_battle = battle.is_some();
    let toggled = std::mem::replace(&mut *had_battle, has_battle) != has_battle;
    toggled
        || battle.is_some_and(|battle| battle.is_changed())
        || focus.is_changed()
        || locale.is_changed()
}

/// Ein Schiff, wie es dastehen soll.
struct Wanted {
    uid: u32,
    kind: ShipKind,
    slot: SlotRef,
    stats: UnitStats,
    max_hull: i32,
    caption: ShipCaption,
    aimed: bool,
    evading: bool,
    aura: ShipAura,
}

/// Zustände einer gegnerischen Einheit als Icons (der Brand kommt mit seiner Zahl eigens).
fn enemy_conditions(enemy: &Enemy) -> Vec<Condition> {
    [
        (enemy.emergency(), Condition::Emergency),
        (enemy.stasis, Condition::Stasis),
        (enemy.silenced, Condition::Silenced),
    ]
    .into_iter()
    .filter_map(|(active, condition)| active.then_some(condition))
    .collect()
}

/// Was auf dem Brett stehen soll.
fn wanted_ships(battle: &CoreBattle, locale: &dyn Messages, focus: &BoardFocus) -> Vec<Wanted> {
    let preview_text = |uid: u32| {
        focus
            .preview
            .as_ref()
            .filter(|(focused, _)| *focused == uid)
            .and_then(|(_, hits)| hits.iter().find(|hit| unit_uid(hit.target) == uid))
            .map(|hit| locale.breakdown(&hit.breakdown))
            .unwrap_or_default()
    };
    let aiming = station_needs_target(battle)
        .then_some(battle.flagship.target)
        .flatten();
    // Der Uhrmacher sieht zwei Runden voraus.
    let clockwork = battle.flagship.admiral == AdmiralId::Kerr;
    let mut ships: Vec<Wanted> = battle
        .enemies
        .iter()
        .map(|enemy| {
            let [first, second] = battle.announced(enemy.uid);
            Wanted {
                uid: enemy.uid,
                kind: ShipKind::Enemy(enemy.id),
                slot: SlotRef::enemy(enemy.slot),
                stats: enemy.stats(),
                max_hull: enemy.max_hull,
                caption: ShipCaption {
                    intent: Some(first),
                    next: clockwork.then_some(second),
                    burning: enemy.burning.then(|| battle.burn_damage(enemy.unit())),
                    conditions: enemy_conditions(enemy),
                    preview: preview_text(enemy.uid),
                },
                aimed: aiming == Some(enemy.uid),
                evading: enemy.is_evading(),
                aura: ShipAura {
                    burning: enemy.burning,
                    charged: enemy.charged,
                    ..default()
                },
            }
        })
        .collect();
    let flagship = &battle.flagship;
    if !flagship.jumped {
        ships.push(Wanted {
            uid: 0,
            kind: ShipKind::Flagship(flagship.admiral.flagship()),
            slot: SlotRef::player(flagship.column),
            stats: flagship.stats(),
            max_hull: flagship.max_hull,
            caption: ShipCaption {
                burning: flagship
                    .burning
                    .then(|| battle.burn_damage(UnitRef::Flagship)),
                conditions: flagship
                    .emergency()
                    .then_some(Condition::Emergency)
                    .into_iter()
                    .collect(),
                preview: preview_text(0),
                ..default()
            },
            aimed: false,
            evading: false,
            aura: ShipAura {
                burning: flagship.burning,
                ..default()
            },
        });
    }
    ships
}

/// Hülle und Schild einer Einheit nach den Treffern der Vorschau; `None`, wenn sie nicht getroffen wird.
fn preview_after(hits: &[Hit], uid: u32, now: UnitStats) -> Option<(UnitStats, bool)> {
    let mut after = None;
    for hit in hits.iter().filter(|hit| unit_uid(hit.target) == uid) {
        let (stats, lethal): (UnitStats, bool) = after.unwrap_or((now, false));
        after = Some((
            UnitStats {
                hull: (stats.hull - hit.breakdown.hull).max(0),
                shield: (stats.shield - hit.breakdown.shielded).max(0),
            },
            lethal || hit.breakdown.lethal,
        ));
    }
    after
}

/// Stellt die Schiffe aus dem Gefecht hin, mit Werten, Vorschau, Marken und Beschriftung.
pub fn sync_ships(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    focus: Res<BoardFocus>,
    locale: Res<Locale>,
    models: Res<ShipModels>,
    board: Res<BoardEntities>,
    mut ships: Query<(
        Entity,
        &Ship,
        &OnSlot,
        &mut ShipStatus,
        &mut ShipMarkers,
        Option<&mut ShipCaption>,
        Option<&mut ShipAura>,
    )>,
) {
    let wanted = battle.map_or_else(Vec::new, |battle| wanted_ships(&battle, &**locale, &focus));
    let hits: &[Hit] = focus.preview.as_ref().map_or(&[], |(_, hits)| hits);
    let mut kept = Vec::new();
    for (entity, ship, on, mut status, mut markers, caption, aura) in &mut ships {
        let Some(want) = wanted
            .iter()
            .find(|want| want.uid == ship.uid && want.kind == ship.kind)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        kept.push(want.uid);
        if on.0 != want.slot {
            place(&mut commands, &board, entity, want.slot);
        }
        let preview = preview_after(hits, want.uid, want.stats);
        status.set_if_neq(ShipStatus {
            hull: want.stats.hull,
            max_hull: want.max_hull,
            preview: preview.map(|(stats, _)| stats),
            doomed: preview.is_some_and(|(_, lethal)| lethal),
            ..status.with_shield(want.stats.shield)
        });
        markers.set_if_neq(ship_markers(want));
        match aura {
            Some(mut aura) => {
                aura.set_if_neq(want.aura);
            }
            None => {
                commands.entity(entity).insert(want.aura);
            }
        }
        match caption {
            Some(mut caption) => {
                caption.set_if_neq(want.caption.clone());
            }
            None => {
                commands.entity(entity).insert(want.caption.clone());
            }
        }
    }
    for want in wanted.iter().filter(|want| !kept.contains(&want.uid)) {
        let ship = spawn_ship(&mut commands, &models, want.kind);
        place(&mut commands, &board, ship, want.slot);
        commands.entity(ship).insert((
            Ship {
                kind: want.kind,
                uid: want.uid,
            },
            ShipStatus::new(want.stats.hull, want.max_hull, want.stats.shield),
            ship_markers(want),
            want.caption.clone(),
            want.aura,
        ));
    }
}

/// Wohin das Flaggschiff manövrieren wird: die Zielspalte, solange die Manöver-Station besetzt ist und sie woanders
/// liegt.
fn ghost_slot(battle: &CoreBattle) -> Option<SlotRef> {
    let flagship = &battle.flagship;
    (!flagship.jumped
        && flagship.stations.contains(&Station::Maneuver)
        && flagship.maneuver_column != flagship.column)
        .then_some(SlotRef::player(flagship.maneuver_column))
}

/// Stellt den Geist des Flaggschiffs in die Zielspalte des Manövers oder nimmt ihn weg.
fn sync_ghost(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    models: Res<ShipModels>,
    board: Res<BoardEntities>,
    ghosts: Query<(Entity, &Ghost, &ChildOf)>,
) {
    let wanted = battle.and_then(|battle| {
        let kind = ShipKind::Flagship(battle.flagship.admiral.flagship());
        ghost_slot(&battle).map(|slot| (kind, board.anchor(slot)))
    });
    let mut kept = false;
    for (entity, ghost, child_of) in &ghosts {
        match wanted {
            Some((kind, anchor)) if ghost.0 == kind && !kept => {
                kept = true;
                if child_of.parent() != anchor {
                    commands.entity(entity).insert(ChildOf(anchor));
                }
            }
            _ => commands.entity(entity).despawn(),
        }
    }
    if let Some((kind, anchor)) = wanted.filter(|_| !kept) {
        let ghost = spawn_ghost(&mut commands, &models, kind);
        commands
            .entity(ghost)
            .insert((ChildOf(anchor), ToonState::facing(Side::Player)));
    }
}

/// Die Wiedergabe übernimmt: Der Geist geht, das Flaggschiff fliegt selbst hin.
fn clear_ghost(mut commands: Commands, ghosts: Query<Entity, With<Ghost>>) {
    for ghost in &ghosts {
        commands.entity(ghost).despawn();
    }
}

/// Das Visier unter dem Ziel der Station, die Marke einer ausweichenden Einheit.
fn ship_markers(want: &Wanted) -> ShipMarkers {
    ShipMarkers(
        [
            (want.aimed, MarkerKind::Aim, palette::SELECT),
            (want.evading, MarkerKind::Evading, palette::ENEMY),
        ]
        .into_iter()
        .filter_map(|(on, kind, color)| on.then_some(MarkerSpec::new(kind, color)))
        .collect(),
    )
}

/// Die Wiedergabe übernimmt: Die Vorschau geht.
fn clear_preview(mut ships: Query<(&mut ShipStatus, &mut ShipCaption)>) {
    for (mut status, mut caption) in &mut ships {
        status.preview = None;
        status.doomed = false;
        if !caption.preview.is_empty() {
            caption.preview.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::events::{Breakdown, UnitRef};

    use super::*;
    use crate::{game::fixture_battle, settings::Language};

    /// Drei Gegner und das Flaggschiff; im Kurzsprung fehlt es. Der Uhrmacher sieht zwei Absichten.
    #[test]
    fn wanted_follows_battle() {
        let locale = Locale::new(Language::De);
        let mut battle = fixture_battle(AdmiralId::Kerr);
        let ships = wanted_ships(&battle, &*locale, &BoardFocus::default());
        assert_eq!(ships.len(), 4);
        assert!(ships[..3].iter().all(|ship| ship.caption.next.is_some()));
        assert_eq!(ships[3].slot, SlotRef::player(1));
        battle.flagship.jumped = true;
        assert_eq!(
            wanted_ships(&battle, &*locale, &BoardFocus::default()).len(),
            3
        );
    }

    /// Der Geist steht nur bei besetzter Manöver-Station und nur in einer anderen Spalte.
    #[test]
    fn ghost_marks_the_maneuver() {
        let mut battle = fixture_battle(AdmiralId::Kerr);
        battle.flagship.maneuver_column = 2;
        assert_eq!(ghost_slot(&battle), None);
        battle.flagship.stations = vec![Station::Maneuver];
        assert_eq!(ghost_slot(&battle), Some(SlotRef::player(2)));
        battle.flagship.maneuver_column = battle.flagship.column;
        assert_eq!(ghost_slot(&battle), None);
    }

    #[test]
    fn preview_sums_hits() {
        let hit = |hull, shielded, lethal| Hit {
            target: UnitRef::Flagship,
            breakdown: Breakdown {
                hull,
                shielded,
                lethal,
                ..Breakdown::default()
            },
        };
        let now = UnitStats {
            hull: 10,
            shield: 4,
        };
        assert_eq!(preview_after(&[], 0, now), None);
        assert_eq!(
            preview_after(&[hit(3, 4, false), hit(7, 0, true)], 0, now),
            Some((UnitStats { hull: 0, shield: 0 }, true))
        );
    }
}
