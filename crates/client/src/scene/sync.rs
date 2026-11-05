//! Die Szene spiegelt das Gefecht, solange keine Wiedergabe läuft ([`BoardMode::Planning`]):
//!
//! - je Gegner-Slot die Einheit aus [`Battle`] (Modell nach Katalog, Hülle, Schild) und das Flaggschiff in seiner
//!   Spalte, im Kurzsprung fehlt es; fehlt ein Schiff, entsteht es, steht es woanders, wechselt es den Slot, ist es
//!   weg, verschwindet es;
//! - die Vorschau aus [`BoardFocus`] (drohender Verlust und Totenkopf auf den Balken, die Rechnung am Ziel), das Visier
//!   unter dem Ziel der Station und über jedem Schiff Absicht und Zustände ([`ShipCaption`]);
//! - Zustände mit eigenem Bild: Brand und Aufladung als Partikel ([`ShipAura`]), Ausweichen als Marke, der Schild im
//!   Balken; die übrigen bleiben Textmarken.
//!
//! Der Controller schaltet nach jeder Aktion auf [`BoardMode::Playback`]; dann gehört die Szene der Wiedergabe, die
//! Vorschau geht. Zurück in der Planung gleicht sich alles dem Zustand an. Dev-Szenen mit [`Scripted`] stellen ihre
//! Schiffe selbst; dann ruht der Abgleich.

use bevy::prelude::*;
use voidfleet_core::{
    admirals::AdmiralId,
    battle::{Battle as CoreBattle, Enemy},
    events::{Hit, UnitStats},
};

use super::{
    aura::ShipAura,
    board::{BoardEntities, OnSlot, place},
    coords::SlotRef,
    labels::ShipCaption,
    markers::{MarkerKind, MarkerSpec, ShipMarkers},
    ship::{Ship, spawn_ship},
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
                sync_ships.run_if(not(resource_exists::<Scripted>)),
            )
            .add_systems(OnEnter(BoardMode::Playback), clear_preview)
            .add_systems(
                Update,
                sync_ships
                    .run_if(board_changed)
                    .in_set(SyncSystems)
                    .run_if(in_state(BoardMode::Planning))
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

/// Zustände einer gegnerischen Einheit als Textmarken (ohne eigenes Bild).
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

fn conditions_text(locale: &dyn Messages, conditions: &[Condition]) -> String {
    conditions
        .iter()
        .map(|&condition| locale.condition(condition))
        .collect::<Vec<_>>()
        .join(" · ")
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
                    intent: locale.intent(first),
                    next: if clockwork {
                        locale.intent(second)
                    } else {
                        String::new()
                    },
                    conditions: conditions_text(locale, &enemy_conditions(enemy)),
                    preview: preview_text(enemy.uid),
                },
                aimed: aiming == Some(enemy.uid),
                evading: enemy.is_evading(),
                aura: ShipAura {
                    burning: enemy.burning,
                    charged: enemy.charged,
                },
            }
        })
        .collect();
    let flagship = &battle.flagship;
    if !flagship.jumped {
        let conditions = [(flagship.emergency(), Condition::Emergency)]
            .into_iter()
            .filter_map(|(active, condition)| active.then_some(condition))
            .collect::<Vec<_>>();
        ships.push(Wanted {
            uid: 0,
            kind: ShipKind::Flagship(flagship.admiral.flagship()),
            slot: SlotRef::player(flagship.column),
            stats: flagship.stats(),
            max_hull: flagship.max_hull,
            caption: ShipCaption {
                conditions: conditions_text(locale, &conditions),
                preview: preview_text(0),
                ..default()
            },
            aimed: false,
            evading: false,
            aura: ShipAura {
                burning: flagship.burning,
                charged: false,
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
            shield: want.stats.shield,
            preview: preview.map(|(stats, _)| stats),
            doomed: preview.is_some_and(|(_, lethal)| lethal),
            ..*status
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
        assert!(ships[..3].iter().all(|ship| !ship.caption.next.is_empty()));
        assert_eq!(ships[3].slot, SlotRef::player(1));
        battle.flagship.jumped = true;
        assert_eq!(
            wanted_ships(&battle, &*locale, &BoardFocus::default()).len(),
            3
        );
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
