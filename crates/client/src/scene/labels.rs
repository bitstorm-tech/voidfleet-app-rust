//! Unter jedem Schiff, als Knoten der Oberfläche direkt unter seinen Balken: eine Reihe Icons — erst Passiven und was
//! hilft (Flaggschiff/Brutverbindung, Kannibalisieren, Lebende Hülle, Panzerung, Unbeweglich, Plasmageschütze,
//! Ausweichreflex, Deckung), dann was schadet (Brand, Notfall, Stillstand, Funkstille, Markiert), dann die Absicht mit
//! Zahl (bei Kerr kleiner die zweite) —, darunter die Rechnung der Vorschau als Text (Schaden rot, Kannibalisieren-Heilung
//! grün). Ein Tooltip über den Icons — und beim freien Erkunden auch über dem 3D-Schiff selbst — zeigt den Schiffsnamen
//! und erklärt jedes Icon Zeile für Zeile. Was dort steht, setzt [`crate::scene::sync`] in [`ShipCaption`], während der
//! Wiedergabe die Absichten auch [`crate::playback`].

use bevy::{
    picking::Pickable,
    prelude::*,
    transform::TransformSystems,
    ui::{UiSystems, UiTransform},
};
use voidfleet_core::{data::rules, enemies::Action, events::Intent};

use super::{
    board::{BoardHover, OnSlot},
    camera::MainCamera,
    coords::Side,
    ship::Ship,
    status::{ShipParts, ShipStatus},
};
use crate::{
    i18n::{Condition, Locale, Messages},
    palette,
    ui::{
        Fonts, HudState, UiIcons,
        theme::{self, Modal},
        tooltip::{BoardTooltip, Glyph, Tooltip, TooltipGlyphs, spawn_glyph},
    },
};

/// Abstand unter der Unterkante der Balken (logische Pixel).
const GAP: f32 = 2.0;
/// Größe der Icons: Absicht, alle übrigen.
const INTENT_SIZE: f32 = 22.0;
const ICON_SIZE: f32 = 16.0;

/// Dauerhafte passive Sonderfähigkeit einer gegnerischen Einheit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Passive {
    /// Schrottkönig: heilt sich bei jedem zerstörten Schiff um diesen Wert.
    Cannibalize(i32),
    /// Wächter, Siegel, Unbekannter Kontakt: immun gegen Traktorstrahl.
    Immovable,
    /// Siegel: fällt das Siegel, fällt die gesamte Gegenflotte.
    Flagship,
    /// Unbekannter Kontakt: fällt er, vergeht seine Brut.
    BroodLink,
    /// Plasmakreuzer: Treffer zünden Brand schon ab 60 % Hülle.
    PlasmaBurn,
    /// Plünderer-Staffel: sagt nach einem Treffer für die nächste Runde Ausweichen an.
    EvadeReflex,
}

/// Was unter einem Schiff steht; Leeres fällt weg.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ShipCaption {
    /// Name der Einheit als Überschrift im Tooltip.
    pub name: String,
    pub intent: Option<Intent>,
    /// Die Absicht danach (Uhrwerk).
    pub next: Option<Intent>,
    /// Was ein Brand je Auflösung kostet, solange das Schiff brennt.
    pub burning: Option<i32>,
    /// Panzerung: Abzug je Treffer, 0 zeigt nichts.
    pub armor: i32,
    /// Lebende Hülle: was je Auflösung nachwächst, 0 zeigt nichts.
    pub regeneration: i32,
    /// Dauerhafte passive Sonderfähigkeiten dieser Einheit.
    pub passives: Vec<Passive>,
    pub conditions: Vec<Condition>,
    /// Die Rechnung der Vorschau am Ziel unter dem Zeiger (Schaden).
    pub preview: String,
    /// Grüne Vorschau einer ausgelösten Heilung (z. B. Kannibalisieren am Schrottkönig).
    pub heal_preview: String,
}

impl ShipCaption {
    /// Ob es unter dem Schiff überhaupt etwas zu zeigen gibt.
    fn is_empty(&self) -> bool {
        self.intent.is_none()
            && self.next.is_none()
            && self.burning.is_none()
            && self.armor <= 0
            && self.regeneration <= 0
            && self.passives.is_empty()
            && self.conditions.is_empty()
            && self.preview.is_empty()
            && self.heal_preview.is_empty()
    }
}

/// Der Knoten der Beschriftung eines Schiffs.
#[derive(Component, Clone, Copy, Debug)]
struct CaptionOf(Entity);

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
    locale: &Messages,
    icons: &UiIcons,
    next: bool,
) -> (Glyph, String) {
    let value = match intent.action {
        Action::Shields(amount) => Some(amount),
        Action::Bolt => Some(rules().enemies.bolt_shield),
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

/// Deckung hilft, alle übrigen Zustände schaden.
fn condition_glyph(condition: Condition, icons: &UiIcons) -> Glyph {
    let icon = match condition {
        Condition::Burning => icons.flame.clone(),
        Condition::Emergency => icons.emergency.clone(),
        Condition::Stasis => icons.stasis.clone(),
        Condition::Silenced => icons.silenced.clone(),
        Condition::Marked => icons.marked.clone(),
        Condition::Covered => icons.cover.clone(),
    };
    let color = if condition == Condition::Covered {
        palette::BUFF
    } else {
        palette::DEBUFF
    };
    Glyph {
        icon,
        color,
        value: None,
        columns: None,
    }
}

/// Eine passive Sonderfähigkeit als Icon samt Tooltip-Zeile.
fn passive_glyph(passive: Passive, locale: &Messages, icons: &UiIcons) -> (Glyph, String, f32) {
    let (icon, color, value, hint) = match passive {
        Passive::Cannibalize(amount) => (
            icons.cannibalize.clone(),
            palette::HEAL,
            Some(amount),
            locale.cannibalize_hint(amount),
        ),
        Passive::Immovable => (
            icons.immovable.clone(),
            palette::BUFF,
            None,
            locale.passive_immovable_hint().to_owned(),
        ),
        Passive::Flagship => (
            icons.stars.clone(),
            palette::FLAGSHIP_TRIM,
            None,
            locale.passive_flagship_hint().to_owned(),
        ),
        Passive::BroodLink => (
            icons.stars.clone(),
            palette::ALIEN,
            None,
            locale.passive_brood_link_hint().to_owned(),
        ),
        Passive::PlasmaBurn => (
            icons.plasma_burn.clone(),
            palette::BUFF,
            None,
            locale.passive_plasma_hint().to_owned(),
        ),
        Passive::EvadeReflex => (
            icons.evade_reflex.clone(),
            palette::BUFF,
            None,
            locale.passive_evade_reflex_hint().to_owned(),
        ),
    };
    (
        Glyph {
            icon,
            color,
            value,
            columns: None,
        },
        hint,
        ICON_SIZE,
    )
}

/// Die Icons in einer Reihe, je mit Größe und Zeile im Tooltip: Passiven und was hilft (Panzerung, Lebende Hülle,
/// Deckung), was schadet (Brand, dann die übrigen Zustände), die Absicht, die nächste.
fn glyphs(caption: &ShipCaption, locale: &Messages, icons: &UiIcons) -> Vec<(Glyph, String, f32)> {
    let passives = caption
        .passives
        .iter()
        .map(|&passive| passive_glyph(passive, locale, icons));
    let armor = (caption.armor > 0).then(|| {
        (
            Glyph {
                icon: icons.armor.clone(),
                color: palette::BUFF,
                value: Some(caption.armor),
                columns: None,
            },
            locale.armor_hint(caption.armor),
            ICON_SIZE,
        )
    });
    let regeneration = (caption.regeneration > 0).then(|| {
        (
            Glyph {
                icon: icons.repair.clone(),
                color: palette::HEAL,
                value: Some(caption.regeneration),
                columns: None,
            },
            locale.regeneration_hint(caption.regeneration),
            ICON_SIZE,
        )
    });
    let burning = caption.burning.map(|amount| {
        (
            Glyph {
                value: (amount > 0).then_some(amount),
                ..condition_glyph(Condition::Burning, icons)
            },
            locale.burn_hint(amount),
            ICON_SIZE,
        )
    });
    let condition = |&condition: &Condition| {
        (
            condition_glyph(condition, icons),
            locale.condition_hint(condition).to_owned(),
            ICON_SIZE,
        )
    };
    let (buffs, debuffs): (Vec<_>, Vec<_>) = caption
        .conditions
        .iter()
        .partition(|&&condition| condition == Condition::Covered);
    let intent = caption.intent.map(|intent| {
        let (glyph, hint) = intent_glyph(intent, palette::ENEMY, locale, icons, false);
        (glyph, hint, INTENT_SIZE)
    });
    let next = caption.next.map(|intent| {
        let (glyph, hint) =
            intent_glyph(intent, palette::ENEMY.with_alpha(0.65), locale, icons, true);
        (glyph, hint, ICON_SIZE)
    });
    passives
        .chain(armor)
        .chain(regeneration)
        .chain(buffs.into_iter().map(condition))
        .chain(burning)
        .chain(debuffs.into_iter().map(condition))
        .chain(intent)
        .chain(next)
        .collect()
}

pub(super) struct LabelsPlugin;

impl Plugin for LabelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_caption)
            .add_systems(Update, (rebuild_captions, update_board_tooltip))
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
            translation: Val2::percent(-50.0, 0.0),
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
        let glyphs = glyphs(&caption, &locale, &icons);
        let mut entity = commands.entity(node);
        entity.despawn_related::<Children>();
        entity.with_children(|parent| {
            if !glyphs.is_empty() {
                let tooltip = TooltipGlyphs(
                    glyphs
                        .iter()
                        .map(|(glyph, hint, _)| (glyph.clone(), hint.clone()))
                        .collect(),
                );
                // Nur die Icons tragen den Tooltip; Klicks und das Zielen gehen durch sie hindurch aufs Schiff.
                parent
                    .spawn((
                        Node {
                            column_gap: px(8),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Tooltip(caption.name.clone()),
                        tooltip,
                        Pickable {
                            should_block_lower: false,
                            is_hoverable: true,
                        },
                    ))
                    .with_children(|line| {
                        for (glyph, _, size) in &glyphs {
                            spawn_glyph(line, glyph, *size, &fonts);
                        }
                    });
            }
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
            if !caption.heal_preview.is_empty() {
                parent.spawn((
                    Text::new(caption.heal_preview.clone()),
                    fonts.display_bold(17.0),
                    TextColor(palette::HEAL),
                    theme::text_shadow(0.9),
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ));
            }
        });
    }
}

/// Füllt den Brett-Tooltip beim freien Erkunden (keine Karte/Station gewählt) über einem gegnerischen 3D-Schiff.
fn update_board_tooltip(
    hover: Res<BoardHover>,
    hud: Res<HudState>,
    modal: Query<(), With<Modal>>,
    ships: Query<(&OnSlot, &ShipStatus, &ShipCaption), With<Ship>>,
    locale: Res<Locale>,
    icons: Res<UiIcons>,
    mut board_tooltip: ResMut<BoardTooltip>,
) {
    let free_inspect = hud.planning
        && !hud.aiming
        && hud.selected.is_none()
        && hud.station.is_none()
        && modal.is_empty();
    let next = hover
        .slot
        .filter(|slot| free_inspect && slot.side == Side::Enemy)
        .and_then(|slot| {
            ships
                .iter()
                .find(|(on, status, _)| on.0 == slot && !status.hidden)
                .map(|(_, _, caption)| {
                    let rows = glyphs(caption, &locale, &icons)
                        .into_iter()
                        .map(|(glyph, hint, _)| (glyph, hint))
                        .collect();
                    BoardTooltip {
                        title: caption.name.clone(),
                        glyphs: TooltipGlyphs(rows),
                    }
                })
        })
        .unwrap_or_default();
    board_tooltip.set_if_neq(next);
}

/// Stellt jede Beschriftung unter die Balken ihres Schiffs; ist das Schiff fort, geht sie mit.
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
            .map(|overlay| overlay.transform_point(Vec3::NEG_Y * status.bars_bottom()))
            .and_then(|world| camera.world_to_viewport(camera_transform, world).ok());
        let shown = position.is_some() && !status.hidden && !caption.is_empty();
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(position) = position {
            node.left = Val::Px(position.x);
            node.top = Val::Px(position.y + GAP);
        }
    }
}
