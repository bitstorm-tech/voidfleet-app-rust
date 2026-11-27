//! Die Szene spiegelt das Gefecht, solange keine Wiedergabe läuft ([`BoardMode::Planning`]):
//!
//! - je Gegner-Slot die Einheit aus [`Battle`] (Modell nach Katalog, Hülle, Schild) und das Flaggschiff in seiner
//!   Spalte, im Kurzsprung fehlt es; fehlt ein Schiff, entsteht es, steht es woanders, wechselt es den Slot, ist es
//!   weg, verschwindet es; zu Beginn des Gefechts springen alle aus dem Hyperraum ([`Arrive`]), das Flaggschiff sofort,
//!   die Gegner nacheinander im Abstand von 0,4 bis 1 Sekunde;
//! - die Vorschau aus [`BoardFocus`] (drohender Verlust und Totenkopf auf den Balken, die Rechnung am Ziel) und über
//!   jedem Schiff Absicht und Zustände ([`ShipCaption`]); das Ziel der Vorschau, also der Gegner unter dem Fadenkreuz,
//!   mit roter Kontur;
//! - der Geist des Flaggschiffs ([`Ghost`]) in der Spalte aus [`BoardFocus`] (unter dem Zeiger, solange die Wahl eine
//!   Spalte braucht);
//! - Zustände mit eigenem Bild: Brand und Aufladung als Partikel ([`ShipAura`]), der Schild im
//!   Balken; Brand, Notfall, Stillstand und Funkstille dazu als Icons in der Beschriftung.
//!
//! Der Controller schaltet nach jeder Aktion auf [`BoardMode::Playback`]; dann gehört die Szene der Wiedergabe, die
//! Vorschau geht. Zurück in der Planung gleicht sich alles dem Zustand an. Dev-Szenen mit [`Scripted`] stellen ihre
//! Schiffe selbst; dann ruht der Abgleich.

use bevy::prelude::*;
use voidfleet_core::{
    admirals::AdmiralId,
    battle::{Battle as CoreBattle, Enemy},
    data::rules,
    enemies::EnemyId,
    events::{Hit, UnitRef, UnitStats},
};

use super::{
    animation::{Animation, AnimationFinished, AnimationScope, random_between},
    aura::ShipAura,
    board::{BoardEntities, OnSlot, place},
    coords::{Side, SlotRef},
    labels::{Passive, ShipCaption},
    motion::Arrive,
    ship::{Ghost, Ship, ToonState, spawn_ghost, spawn_ship},
    status::ShipStatus,
    toon::ToonOutline,
};
use crate::{
    assets::AppState,
    controller::{
        Scripted,
        selection::{TargetPreview, unit_uid},
    },
    data::ship_models::{ShipKind, ShipModels},
    game::Battle,
    i18n::{Condition, Locale, Messages},
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

/// Was der Controller unter dem Zeiger zeigt: Treffer und Kannibalisieren-Heilungen der gewählten Karte bzw. Station
/// sowie die Spalte, in der der Geist des Flaggschiffs steht.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct BoardFocus {
    pub preview: Option<TargetPreview>,
    pub ghost: Option<SlotRef>,
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
            .add_systems(Update, mark_target.run_if(resource_changed::<BoardFocus>))
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

/// Gefechtsbeginn: Abstand in Sekunden zwischen zwei Gegnern, die aus dem Hyperraum springen.
const ENEMY_ARRIVAL_GAP: (f32, f32) = (0.4, 1.0);

/// Ein Schiff, wie es dastehen soll.
struct Wanted {
    uid: u32,
    kind: ShipKind,
    slot: SlotRef,
    stats: UnitStats,
    max_hull: i32,
    caption: ShipCaption,
    aura: ShipAura,
}

/// Dauerhafte passive Sonderfähigkeiten einer gegnerischen Einheit.
fn enemy_passives(enemy: &Enemy) -> Vec<Passive> {
    let mut passives = Vec::new();
    match enemy.id {
        EnemyId::ScrapKing => passives.push(Passive::Cannibalize(rules().enemies.cannibalize_heal)),
        EnemyId::Seal => passives.push(Passive::Flagship),
        EnemyId::UnknownContact => passives.push(Passive::BroodLink),
        EnemyId::PlasmaCruiser => passives.push(Passive::PlasmaBurn),
        EnemyId::Raiders => passives.push(Passive::EvadeReflex),
        _ => {}
    }
    if enemy.id.immovable() {
        passives.push(Passive::Immovable);
    }
    passives
}

/// Zustände einer gegnerischen Einheit als Icons (der Brand kommt mit seiner Zahl eigens).
fn enemy_conditions(battle: &CoreBattle, enemy: &Enemy) -> Vec<Condition> {
    [
        (battle.covered(enemy.uid), Condition::Covered),
        (enemy.emergency(), Condition::Emergency),
        (enemy.stasis, Condition::Stasis),
        (enemy.silenced, Condition::Silenced),
        (enemy.marked, Condition::Marked),
    ]
    .into_iter()
    .filter_map(|(active, condition)| active.then_some(condition))
    .collect()
}

/// Was auf dem Brett stehen soll.
fn wanted_ships(battle: &CoreBattle, locale: &Messages, focus: &BoardFocus) -> Vec<Wanted> {
    let preview_text = |uid: u32| {
        let Some(preview) = focus.preview.as_ref().filter(|p| p.target == uid) else {
            return String::new();
        };
        let mut groups: Vec<(String, usize)> = Vec::new();
        for line in preview
            .hits
            .iter()
            .filter(|hit| unit_uid(hit.target) == uid)
            .map(|hit| locale.breakdown(&hit.breakdown))
        {
            if let Some((_last, count)) = groups.last_mut().filter(|(last, _)| *last == line) {
                *count += 1;
            } else {
                groups.push((line, 1));
            }
        }
        groups
            .into_iter()
            .map(|(line, count)| {
                if count > 1 {
                    format!("{count}× {line}")
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let heal_preview_text = |uid: u32| {
        focus
            .preview
            .as_ref()
            .and_then(|preview| {
                preview
                    .cannibalize
                    .iter()
                    .find(|(id, amount)| *id == uid && *amount > 0)
            })
            .map(|(_, amount)| locale.cannibalize_preview(*amount))
            .unwrap_or_default()
    };
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
                    name: locale.enemy(enemy.id).to_owned(),
                    intent: Some(first),
                    next: clockwork.then_some(second),
                    burning: enemy.burning.then(|| battle.burn_damage(enemy.unit())),
                    armor: enemy.armor,
                    regeneration: enemy.regeneration(),
                    passives: enemy_passives(enemy),
                    conditions: enemy_conditions(battle, enemy),
                    preview: preview_text(enemy.uid),
                    heal_preview: heal_preview_text(enemy.uid),
                },
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
                name: locale.flagship(flagship.admiral.flagship()).to_owned(),
                burning: flagship
                    .burning
                    .then(|| battle.burn_damage(UnitRef::Flagship)),
                armor: battle.flagship_armor(),
                conditions: flagship
                    .emergency()
                    .then_some(Condition::Emergency)
                    .into_iter()
                    .collect(),
                preview: preview_text(0),
                ..default()
            },
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
        Option<&mut ShipCaption>,
        Option<&mut ShipAura>,
    )>,
) {
    let wanted = battle
        .as_ref()
        .map_or_else(Vec::new, |battle| wanted_ships(battle, &locale, &focus));
    let hits: &[Hit] = focus
        .preview
        .as_ref()
        .map_or(&[], |preview| preview.hits.as_slice());
    let mut kept = Vec::new();
    for (entity, ship, on, mut status, caption, aura) in &mut ships {
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
    // Zu Beginn des Gefechts springen alle aus dem Hyperraum, die Gegner nacheinander.
    let arriving = battle.is_some_and(|battle| battle.is_added());
    let mut delay = 0.0;
    for want in wanted.iter().filter(|want| !kept.contains(&want.uid)) {
        let ship = spawn_ship(&mut commands, &models, want.kind);
        place(&mut commands, &board, ship, want.slot);
        let mut status = ShipStatus::new(want.stats.hull, want.max_hull, want.stats.shield);
        status.hidden = arriving;
        commands.entity(ship).insert((
            Ship {
                kind: want.kind,
                uid: want.uid,
            },
            status,
            want.caption.clone(),
            want.aura,
        ));
        if !arriving {
            continue;
        } else if matches!(want.kind, ShipKind::Flagship(_)) {
            commands.spawn(Arrive { ship });
            continue;
        }
        delay += random_between(ENEMY_ARRIVAL_GAP);
        commands.entity(ship).insert(Visibility::Hidden);
        commands
            .spawn(Animation::new(delay).in_scope(AnimationScope::Movement))
            .observe(
                move |_: On<AnimationFinished>,
                      mut commands: Commands,
                      ships: Query<(), With<Ship>>| {
                    // Das Gefecht kann schon vorbei sein.
                    if ships.contains(ship) {
                        commands.spawn(Arrive { ship });
                    }
                },
            );
    }
}

/// Stellt den Geist des Flaggschiffs in die Spalte aus [`BoardFocus`] oder nimmt ihn weg.
fn sync_ghost(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    focus: Res<BoardFocus>,
    models: Res<ShipModels>,
    board: Res<BoardEntities>,
    ghosts: Query<(Entity, &Ghost, &ChildOf)>,
) {
    let wanted = battle.and_then(|battle| {
        let kind = ShipKind::Flagship(battle.flagship.admiral.flagship());
        focus.ghost.map(|slot| (kind, board.anchor(slot)))
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

/// Die Wiedergabe übernimmt: Die Vorschau geht.
/// Das Schiff, auf das die Vorschau zielt (gültiges Ziel unter dem Fadenkreuz), bekommt die rote Kontur.
fn mark_target(
    focus: Res<BoardFocus>,
    ships: Query<(Entity, &Ship, &OnSlot)>,
    children: Query<&Children>,
    mut contours: Query<&mut Visibility, With<ToonOutline>>,
) {
    let target = focus.preview.as_ref().map(|preview| preview.target);
    for (entity, ship, on) in &ships {
        let targeted = on.0.side == Side::Enemy && target == Some(ship.uid);
        let visibility = if targeted {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for part in children.iter_descendants(entity) {
            if let Ok(mut contour) = contours.get_mut(part) {
                contour.set_if_neq(visibility);
            }
        }
    }
}

fn clear_preview(mut ships: Query<(&mut ShipStatus, &mut ShipCaption)>) {
    for (mut status, mut caption) in &mut ships {
        status.preview = None;
        status.doomed = false;
        if !caption.preview.is_empty() {
            caption.preview.clear();
        }
        if !caption.heal_preview.is_empty() {
            caption.heal_preview.clear();
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
        let ships = wanted_ships(&battle, &locale, &BoardFocus::default());
        assert_eq!(ships.len(), 4);
        assert!(ships[..3].iter().all(|ship| ship.caption.next.is_some()));
        assert_eq!(ships[3].slot, SlotRef::player(1));
        battle.flagship.jumped = true;
        assert_eq!(
            wanted_ships(&battle, &locale, &BoardFocus::default()).len(),
            3
        );
    }

    /// Nur das Ziel unter dem Fadenkreuz zeigt seine Kontur; ohne Vorschau ist sie wieder weg.
    #[test]
    fn target_shows_its_contour() {
        use bevy::ecs::system::RunSystemOnce;
        use voidfleet_core::{
            board::{EnemySlot, Row},
            enemies::EnemyId,
        };

        use crate::data::ship_models::ShipKind;

        let mut world = World::new();
        let mut contour_of = |uid, column| {
            let contour = world.spawn((ToonOutline, Visibility::Hidden)).id();
            let ship = Ship {
                kind: ShipKind::Enemy(EnemyId::Raiders),
                uid,
            };
            let slot = SlotRef::enemy(EnemySlot::new(Row::Front, column));
            world.spawn((ship, OnSlot(slot))).add_child(contour);
            contour
        };
        let (aimed, other) = (contour_of(1, 0), contour_of(2, 1));
        world.insert_resource(BoardFocus {
            preview: Some(TargetPreview {
                target: 1,
                ..default()
            }),
            ghost: None,
        });
        world.run_system_once(mark_target).unwrap();
        assert_eq!(world.get::<Visibility>(aimed), Some(&Visibility::Inherited));
        assert_eq!(world.get::<Visibility>(other), Some(&Visibility::Hidden));
        world.insert_resource(BoardFocus::default());
        world.run_system_once(mark_target).unwrap();
        assert_eq!(world.get::<Visibility>(aimed), Some(&Visibility::Hidden));
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

    #[test]
    fn marked_condition_and_multi_hit_preview_show_in_captions() {
        let locale = Locale::new(Language::En);
        let mut battle = fixture_battle(AdmiralId::Kerr);
        battle.enemies[0].marked = true;
        let uid = battle.enemies[0].uid;
        let repeated = Hit {
            target: UnitRef::Enemy {
                uid,
                id: battle.enemies[0].id,
                slot: battle.enemies[0].slot,
            },
            breakdown: Breakdown {
                base: 2,
                marked: 1,
                hull: 3,
                ..Breakdown::default()
            },
        };
        let distinct = Hit {
            breakdown: Breakdown {
                base: 4,
                armor: 1,
                hull: 3,
                ..Breakdown::default()
            },
            ..repeated
        };
        let focus = BoardFocus {
            preview: Some(TargetPreview {
                target: uid,
                hits: vec![repeated, repeated, distinct],
                cannibalize: Vec::new(),
            }),
            ghost: None,
        };
        let ships = wanted_ships(&battle, &locale, &focus);
        let target = ships.iter().find(|ship| ship.uid == uid).unwrap();
        assert!(target.caption.conditions.contains(&Condition::Marked));
        assert_eq!(
            target.caption.preview,
            "2× 2 · marked +1 → 3\n4 · armor −1 → 3"
        );
    }

    #[test]
    fn enemy_passives_and_cannibalize_preview_appear_in_captions() {
        use voidfleet_core::battle::{BattleSetup, Encounter};

        let locale = Locale::new(Language::De);
        let encounter = Encounter {
            front: [
                Some(EnemyId::Raiders.into()),
                Some(EnemyId::PlasmaCruiser.into()),
                Some(EnemyId::Warden.into()),
            ],
            back: [Some(EnemyId::ScrapKing.into()), None, None],
            ..Encounter::default()
        };
        let battle = CoreBattle::new(
            BattleSetup {
                admiral: AdmiralId::Vance,
                hull: AdmiralId::Vance.hull(),
                max_hull: AdmiralId::Vance.hull(),
                deck: voidfleet_core::cards::starter_deck(AdmiralId::Vance),
                modules: Vec::new(),
                emergency_generator_available: false,
                encounter,
            },
            1,
        );
        let king_uid = battle
            .enemies
            .iter()
            .find(|e| e.id == EnemyId::ScrapKing)
            .unwrap()
            .uid;
        let raiders_uid = battle
            .enemies
            .iter()
            .find(|e| e.id == EnemyId::Raiders)
            .unwrap()
            .uid;
        let focus = BoardFocus {
            preview: Some(TargetPreview {
                target: raiders_uid,
                hits: Vec::new(),
                cannibalize: vec![(king_uid, 8)],
            }),
            ghost: None,
        };
        let ships = wanted_ships(&battle, &locale, &focus);
        let king = ships.iter().find(|s| s.uid == king_uid).unwrap();
        assert_eq!(king.caption.name, "Schrottkönig");
        assert!(
            king.caption
                .passives
                .contains(&Passive::Cannibalize(rules().enemies.cannibalize_heal))
        );
        assert_eq!(king.caption.heal_preview, "Kannibalisieren +8");

        let warden = ships
            .iter()
            .find(|s| s.kind == ShipKind::Enemy(EnemyId::Warden))
            .unwrap();
        assert!(warden.caption.passives.contains(&Passive::Immovable));

        let plasma = ships
            .iter()
            .find(|s| s.kind == ShipKind::Enemy(EnemyId::PlasmaCruiser))
            .unwrap();
        assert!(plasma.caption.passives.contains(&Passive::PlasmaBurn));

        let raiders = ships.iter().find(|s| s.uid == raiders_uid).unwrap();
        assert!(raiders.caption.passives.contains(&Passive::EvadeReflex));
    }
}
