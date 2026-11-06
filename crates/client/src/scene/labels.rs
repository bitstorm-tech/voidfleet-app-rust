//! Über jedem Schiff, als Knoten der Oberfläche über seinen Balken: die Rechnung der Vorschau als Text, darunter die
//! Absicht als Icon mit Zahl (bei Kerr darunter kleiner die zweite) und die Zustände als Icons. Ein Tooltip über den
//! Icons erklärt jedes von ihnen, Zeile für Zeile neben demselben Icon. Was dort steht, setzt [`crate::scene::sync`] in [`ShipCaption`], während der
//! Wiedergabe die Absichten auch [`crate::playback`].

use bevy::{
    picking::Pickable,
    prelude::*,
    transform::TransformSystems,
    ui::{UiSystems, UiTransform},
};
use voidfleet_core::{
    enemies::{Action, BOLT_SHIELD},
    events::Intent,
};

use super::{
    camera::MainCamera,
    ship::Ship,
    status::{ShipParts, ShipStatus},
};
use crate::{
    i18n::{Condition, Locale, Messages},
    palette,
    ui::{
        Fonts, UiIcons, theme,
        tooltip::{Glyph, TooltipGlyphs, spawn_glyph},
    },
};

/// Abstand über der Mitte der Balken (Welteinheiten).
const LIFT: f32 = 0.9;

/// Farbe der Zustände.
const CONDITION: Color = palette::BUFF;

/// Was über einem Schiff steht; Leeres fällt weg.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ShipCaption {
    pub intent: Option<Intent>,
    /// Die Absicht danach (Uhrwerk).
    pub next: Option<Intent>,
    /// Was ein Brand je Auflösung kostet, solange das Schiff brennt.
    pub burning: Option<i32>,
    pub conditions: Vec<Condition>,
    /// Die Rechnung der Vorschau am Ziel unter dem Zeiger.
    pub preview: String,
}

/// Der Knoten der Beschriftung eines Schiffs.
#[derive(Component, Clone, Copy, Debug)]
struct CaptionOf(Entity);

/// Eine Zeile Icons in ihrer Größe, je Icon seine Zeile im Tooltip.
struct GlyphRow {
    size: f32,
    glyphs: Vec<(Glyph, String)>,
}

fn action_icon(action: Action, icons: &UiIcons) -> Handle<Image> {
    match action {
        Action::Fire => icons.crosshair.clone(),
        Action::ColumnFire { .. } => icons.column_fire.clone(),
        Action::Ram => icons.ram.clone(),
        Action::Shields(_) | Action::Bolt => icons.shield.clone(),
        Action::Evade => icons.evade.clone(),
        Action::Charge => icons.charge.clone(),
        Action::TargetLock => icons.target_lock.clone(),
        Action::Call(_) => icons.call.clone(),
    }
}

fn intent_glyph(
    intent: Intent,
    color: Color,
    locale: &dyn Messages,
    icons: &UiIcons,
    next: bool,
) -> (Glyph, String) {
    let value = match intent.action {
        Action::Shields(amount) => Some(amount),
        Action::Bolt => Some(BOLT_SHIELD),
        _ => (intent.damage > 0).then_some(intent.damage),
    };
    let columns = match intent.action {
        Action::ColumnFire { columns } => Some(columns),
        _ => None,
    };
    let hint = locale.action_hint(intent.action);
    (
        Glyph {
            icon: action_icon(intent.action, icons),
            color,
            value,
            columns,
        },
        if next {
            format!("{}: {hint}", locale.caption_next())
        } else {
            hint
        },
    )
}

fn condition_glyph(condition: Condition, icons: &UiIcons) -> Glyph {
    let icon = match condition {
        Condition::Burning => icons.flame.clone(),
        Condition::Emergency => icons.emergency.clone(),
        Condition::Stasis => icons.stasis.clone(),
        Condition::Silenced => icons.silenced.clone(),
    };
    Glyph {
        icon,
        color: CONDITION,
        value: None,
        columns: None,
    }
}

/// Die Zeilen der Icons: Absicht, die nächste, Zustände (Brand zuerst).
fn glyph_rows(caption: &ShipCaption, locale: &dyn Messages, icons: &UiIcons) -> [GlyphRow; 3] {
    let burning = caption.burning.map(|amount| {
        (
            Glyph {
                value: (amount > 0).then_some(amount),
                ..condition_glyph(Condition::Burning, icons)
            },
            locale.burn_hint(amount),
        )
    });
    let conditions = caption.conditions.iter().map(|&condition| {
        (
            condition_glyph(condition, icons),
            locale.condition_hint(condition).to_owned(),
        )
    });
    [
        GlyphRow {
            size: 22.0,
            glyphs: caption
                .intent
                .map(|intent| intent_glyph(intent, palette::ENEMY, locale, icons, false))
                .into_iter()
                .collect(),
        },
        GlyphRow {
            size: 16.0,
            glyphs: caption
                .next
                .map(|intent| {
                    intent_glyph(intent, palette::ENEMY.with_alpha(0.65), locale, icons, true)
                })
                .into_iter()
                .collect(),
        },
        GlyphRow {
            size: 16.0,
            glyphs: burning.into_iter().chain(conditions).collect(),
        },
    ]
}

pub(super) struct LabelsPlugin;

impl Plugin for LabelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_caption)
            .add_systems(Update, rebuild_captions)
            .add_systems(
                PostUpdate,
                // Nach den Lagen dieses Bilds und vor dem UI-Layout: Die Beschriftung steht im selben Bild dort.
                follow_ships
                    .after(TransformSystems::Propagate)
                    .before(UiSystems::Layout),
            );
    }
}

fn spawn_caption(add: On<Add<ShipCaption>>, mut commands: Commands) {
    commands.spawn((
        Name::new("Beschriftung"),
        CaptionOf(add.entity),
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        UiTransform {
            translation: Val2::percent(-50.0, -100.0),
            ..default()
        },
        GlobalZIndex(-5),
        Pickable::IGNORE,
        Visibility::Hidden,
    ));
}

/// Baut eine Beschriftung neu, sobald sich ihr Inhalt oder die Sprache ändert.
fn rebuild_captions(
    mut commands: Commands,
    captions: Query<Ref<ShipCaption>>,
    nodes: Query<(Entity, &CaptionOf)>,
    locale: Res<Locale>,
    icons: Res<UiIcons>,
    fonts: Res<Fonts>,
) {
    for (node, caption_of) in &nodes {
        let Ok(caption) = captions.get(caption_of.0) else {
            continue;
        };
        if !caption.is_changed() && !locale.is_changed() {
            continue;
        }
        let rows = glyph_rows(&caption, &**locale, &icons);
        let tooltip = TooltipGlyphs(rows.iter().flat_map(|row| row.glyphs.clone()).collect());
        let mut entity = commands.entity(node);
        entity.despawn_related::<Children>();
        entity.with_children(|parent| {
            if !caption.preview.is_empty() {
                parent.spawn((
                    Text::new(caption.preview.clone()),
                    fonts.display_bold(17.0),
                    TextColor(palette::DAMAGE),
                    theme::text_shadow(0.9),
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ));
            }
            if tooltip.0.is_empty() {
                return;
            }
            // Nur die Icons tragen den Tooltip; Klicks und das Zielen gehen durch sie hindurch aufs Schiff.
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(2),
                        ..default()
                    },
                    tooltip,
                    Pickable {
                        should_block_lower: false,
                        is_hoverable: true,
                    },
                ))
                .with_children(|block| {
                    for row in rows.iter().filter(|row| !row.glyphs.is_empty()) {
                        block
                            .spawn((
                                Node {
                                    column_gap: px(8),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|line| {
                                for (glyph, _) in &row.glyphs {
                                    spawn_glyph(line, glyph, row.size, &fonts);
                                }
                            });
                    }
                });
        });
    }
}

/// Stellt jede Beschriftung über die Balken ihres Schiffs; ist das Schiff fort, geht sie mit.
fn follow_ships(
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ships: Query<(&ShipParts, &ShipStatus, &ShipCaption), With<Ship>>,
    overlays: Query<&GlobalTransform>,
    mut nodes: Query<(Entity, &CaptionOf, &mut Node, &mut Visibility)>,
) {
    let (camera, camera_transform) = *camera;
    for (entity, caption_of, mut node, mut visibility) in &mut nodes {
        let Ok((parts, status, caption)) = ships.get(caption_of.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let position = overlays
            .get(parts.overlay)
            .ok()
            .map(|overlay| overlay.translation() + Vec3::Y * LIFT)
            .and_then(|world| camera.world_to_viewport(camera_transform, world).ok());
        let empty = *caption == ShipCaption::default();
        let shown = position.is_some() && !status.hidden && !empty;
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(position) = position {
            node.left = Val::Px(position.x);
            node.top = Val::Px(position.y);
        }
    }
}
