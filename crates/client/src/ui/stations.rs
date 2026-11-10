//! Das Stationen-Feld rechts in der Hand: die fünf Stationen des Flaggschiffs (die Admiralsstation mit ihrem Namen je
//! Admiral); ein Klick wirkt wie auf eine Karte der Hand (ohne Ziel sofort, sonst ist die Station gewählt und leuchtet,
//! bis ein Ziel auf dem Brett sie auslöst). Gesperrt ist, was in dieser Runde nicht mehr laufen darf, also auch die
//! gelaufenen (eine, mit Alarmstufe Rot zwei). Daneben „Zug beenden“ (während einer Wiedergabe „Überspringen“) mit der
//! Checkliste des Zugs: gelaufene Stationen, freie Stationen und offene Befehlspunkte als Hinweis. Klicks melden
//! [`StationPressed`] und [`ActionPressed`].

use bevy::{
    prelude::*,
    text::LineHeight,
    ui::InteractionDisabled,
    ui_widgets::{Activate, observe},
};
use voidfleet_core::{admirals::Station, battle::Battle as CoreBattle};

use super::{
    ActionPressed, Fonts, HudAction, HudState, StationPressed, UiIcons,
    button::{self, Key, Lit},
    theme,
    tooltip::Tooltip,
};
use crate::{assets::AppState, game::Battle, i18n::Locale, palette};

/// Eine Station im Feld.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct StationButton(pub(super) Station);

/// Ein Aktionsknopf.
#[derive(Component, Clone, Copy, Debug)]
struct ActionButton(HudAction);

#[derive(Component)]
struct Checklist;

/// Wie weit ein Punkt der Checkliste ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChecklistState {
    Done,
    Open,
    /// Nur ein Hinweis (offene Befehlspunkte, Spaltenwahl in Runde 1).
    Hint,
}

/// Das Feld; steht rechts in der Hand ([`super::hand`]).
pub(super) fn stations_panel(fonts: &Fonts, icons: &UiIcons) -> impl Bundle {
    let station = |station: Station| {
        // Die drei Sterne füllen nur ein Drittel der Höhe ihres Icons.
        let (icon, size) = match station {
            Station::Weapons => (&icons.crosshair, 16.0),
            Station::Shields => (&icons.shield, 16.0),
            Station::DamageControl => (&icons.repair, 16.0),
            Station::Maneuver => (&icons.maneuver, 16.0),
            Station::Admiral => (&icons.stars, 26.0),
        };
        (
            StationButton(station),
            button::button(
                Key::NORMAL,
                Node {
                    width: px(190),
                    height: px(30),
                    column_gap: px(8),
                    ..default()
                },
            ),
            children![
                button::icon_image(icon.clone(), size),
                (
                    theme::caps(fonts.display_bold(16.0), "", theme::TRACKING, palette::TEXT),
                    theme::text_shadow(0.55),
                    Pickable::IGNORE,
                ),
            ],
            Tooltip::default(),
            observe(
                move |_: On<Activate>, mut pressed: MessageWriter<StationPressed>| {
                    pressed.write(StationPressed(station));
                },
            ),
        )
    };
    let action = |action: HudAction, key: Key, height: f32| {
        (
            ActionButton(action),
            button::text_button(
                fonts.display_bold(22.0),
                "",
                key,
                Node {
                    width: px(220),
                    height: px(height),
                    ..default()
                },
            ),
            observe(
                |activate: On<Activate>,
                 buttons: Query<&ActionButton>,
                 mut pressed: MessageWriter<ActionPressed>| {
                    if let Ok(&ActionButton(action)) = buttons.get(activate.entity) {
                        pressed.write(ActionPressed(action));
                    }
                },
            ),
        )
    };
    (
        Name::new("Stationen"),
        Node {
            column_gap: px(14),
            align_self: AlignSelf::Center,
            flex_shrink: 0.0,
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(theme::PANEL),
        Pickable::IGNORE,
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    station(Station::Weapons),
                    station(Station::Shields),
                    station(Station::DamageControl),
                    station(Station::Maneuver),
                    station(Station::Admiral),
                ],
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    action(HudAction::EndTurn, Key::PRIMARY, 56.0),
                    (
                        Checklist,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(3),
                            ..default()
                        },
                        Visibility::Hidden,
                        Pickable::IGNORE,
                    ),
                ],
            ),
        ],
    )
}

/// Beschriftung, Tooltip, Leuchten (gewählt) und Sperre (darf nicht mehr laufen) der Stationen.
fn update_stations(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    hud: Res<HudState>,
    locale: Res<Locale>,
    mut buttons: Query<(
        Entity,
        &StationButton,
        &Children,
        &mut Tooltip,
        Has<Lit>,
        Has<InteractionDisabled>,
    )>,
    mut labels: Query<&mut Text>,
) {
    let Some(battle) = battle else {
        return;
    };
    if !battle.is_changed() && !hud.is_changed() && !locale.is_changed() {
        return;
    }
    let admiral = battle.flagship.admiral;
    for (entity, &StationButton(station), children, mut tooltip, lit, disabled) in &mut buttons {
        let label = locale.station(station, admiral).to_uppercase();
        for &child in children {
            if let Ok(mut text) = labels.get_mut(child)
                && text.0 != label
            {
                text.0.clone_from(&label);
            }
        }
        tooltip.0 = locale.station_text(station, admiral, battle.flagship_attack());
        let chosen = hud.station == Some(station);
        if chosen && !lit {
            commands.entity(entity).insert(Lit);
        } else if !chosen && lit {
            commands.entity(entity).remove::<Lit>();
        }
        let enabled = hud.planning && battle.station_open(station).is_ok();
        if disabled == enabled {
            button::set_enabled(&mut commands, entity, enabled);
        }
    }
}

/// Beschriftung und Sperre der Aktionsknöpfe; während einer Wiedergabe wird „Zug beenden“ zu „Überspringen“.
fn update_actions(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    hud: Res<HudState>,
    locale: Res<Locale>,
    mut buttons: Query<(
        Entity,
        &mut ActionButton,
        &Children,
        Has<InteractionDisabled>,
    )>,
    mut labels: Query<&mut Text>,
) {
    let changed = battle.as_ref().is_none_or(|battle| battle.is_changed());
    if !changed && !hud.is_changed() && !locale.is_changed() {
        return;
    }
    for (entity, mut button, children, disabled) in &mut buttons {
        button.0 = if hud.playing_back {
            HudAction::Skip
        } else {
            HudAction::EndTurn
        };
        let (label, enabled) = match button.0 {
            HudAction::EndTurn => (locale.hud_end_turn(), hud.planning),
            HudAction::Skip => (locale.hud_skip(), true),
        };
        let label = label.to_uppercase();
        for &child in children {
            if let Ok(mut text) = labels.get_mut(child)
                && text.0 != label
            {
                text.0.clone_from(&label);
            }
        }
        if disabled == enabled {
            button::set_enabled(&mut commands, entity, enabled);
        }
    }
}

/// Die Zeilen der Checkliste: je gelaufene Station eine (erledigt), freie Stationsplätze (offen, solange noch eine
/// laufen darf) und offene Befehlspunkte.
pub fn checklist_lines(locale: &Locale, battle: &CoreBattle) -> Vec<(String, ChecklistState)> {
    let flagship = &battle.flagship;
    let mut lines: Vec<_> = flagship
        .stations_used
        .iter()
        .map(|&station| {
            let name = locale.station(station, flagship.admiral);
            (locale.checklist_station(name), ChecklistState::Done)
        })
        .collect();
    let free = flagship
        .station_slots
        .saturating_sub(flagship.stations_used.len());
    if free > 0 && !flagship.jumped && battle.result.is_none() {
        lines.push((locale.checklist_free_stations(free), ChecklistState::Open));
    }
    let affordable = battle.hand.iter().any(|card| card.cost() <= battle.points);
    if battle.points > 0 && affordable {
        lines.push((locale.checklist_points(battle.points), ChecklistState::Hint));
    }
    lines
}

/// Eine Zeile der Checkliste: Kästchen (erledigt mit Haken) und Text; Hinweise ohne Kästchen.
fn spawn_checklist_item(
    list: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    check: &Handle<Image>,
    text: &str,
    state: ChecklistState,
) {
    let color = match state {
        ChecklistState::Done => theme::ACCENT,
        ChecklistState::Open => palette::TEXT,
        ChecklistState::Hint => palette::TEXT_DIM.with_alpha(0.6),
    };
    let tick = (state == ChecklistState::Done).then(|| {
        (
            Node {
                width: px(10),
                height: px(10),
                ..default()
            },
            ImageNode::new(check.clone()).with_color(color),
            Pickable::IGNORE,
        )
    });
    list.spawn((
        Node {
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|row| {
        row.spawn((
            Node {
                width: px(12),
                height: px(12),
                border: UiRect::all(px(1.5)),
                border_radius: BorderRadius::all(px(3)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(if state == ChecklistState::Hint {
                Color::NONE
            } else {
                color
            }),
            Pickable::IGNORE,
            Children::spawn(SpawnIter(tick.into_iter())),
        ));
        row.spawn((
            theme::text(fonts.display(16.0), text, color),
            LineHeight::RelativeToFont(1.1),
            Pickable::IGNORE,
        ));
    });
}

/// Füllt die Checkliste; außerhalb des Zugs bleibt sie unsichtbar stehen, damit die Knöpfe nicht springen.
fn update_checklist(
    mut commands: Commands,
    mut shown: Local<Vec<(String, ChecklistState)>>,
    battle: Option<Res<Battle>>,
    hud: Res<HudState>,
    locale: Res<Locale>,
    fonts: Res<Fonts>,
    icons: Res<UiIcons>,
    checklist: Single<(Entity, &mut Visibility), With<Checklist>>,
) {
    let (entity, mut visibility) = checklist.into_inner();
    let lines = match battle.as_deref().filter(|_| hud.planning) {
        Some(battle) => checklist_lines(&locale, battle),
        None => Vec::new(),
    };
    visibility.set_if_neq(if lines.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    });
    if *shown == lines {
        return;
    }
    commands
        .entity(entity)
        .despawn_related::<Children>()
        .with_children(|list| {
            for (text, state) in &lines {
                spawn_checklist_item(list, &fonts, &icons.check, text, *state);
            }
        });
    *shown = lines;
}

pub(super) struct StationsPlugin;

impl Plugin for StationsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_stations, update_actions, update_checklist).run_if(in_state(AppState::Match)),
        );
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::admirals::AdmiralId;

    use super::*;
    use crate::{game::fixture_battle, settings::Language};

    #[test]
    fn checklist_follows_stations() {
        let locale = Locale::new(Language::De);
        let mut battle = fixture_battle(AdmiralId::Osei);
        assert_eq!(
            checklist_lines(&locale, &battle),
            [
                ("1 Station frei".to_owned(), ChecklistState::Open),
                ("3 Befehlspunkte offen".to_owned(), ChecklistState::Hint)
            ]
        );
        battle.points = 0;
        battle.flagship.station_slots = 2;
        battle
            .run_station(Station::Shields, voidfleet_core::battle::Target::None)
            .unwrap();
        assert_eq!(
            checklist_lines(&locale, &battle),
            [
                ("Station: Schilde".to_owned(), ChecklistState::Done),
                ("1 Station frei".to_owned(), ChecklistState::Open)
            ]
        );
        battle.flagship.jumped = true;
        assert_eq!(
            checklist_lines(&locale, &battle),
            [("Station: Schilde".to_owned(), ChecklistState::Done)]
        );
    }
}
