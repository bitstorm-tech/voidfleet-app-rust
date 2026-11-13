//! Knöpfe als Schaltflächen eines Raumschiff-HUDs: kantige Fläche aus getöntem Glas mit feinem Rahmen, Licht, das
//! von der Unterkante aufsteigt, und vier leuchtende Eckklammern. Hover lässt die Fläche aufleuchten und die Klammern
//! auseinanderfahren, Drücken zieht sie zusammen und blitzt auf, gesperrt ([`InteractionDisabled`]) ist die Taste
//! blass. Varianten setzen nur Farben ([`Key`]).
//!
//! Gebaut wird mit [`button`], [`text_button`] oder [`icon_button`]; die Taste ist ein `bevy_ui_widgets`-[`Button`]
//! und meldet einen Klick (auch Enter/Leertaste bei Fokus) als [`Activate`] an ihre Entity — wer sie baut, hängt einen
//! Observer an (`.observe(|_: On<Activate>| …)`). Den Klick-Klang spielt jede Taste selbst, die Eckklammern hängt sie
//! sich selbst an.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed, UiSystems},
    ui_widgets::{Activate, Button},
};

use super::{
    theme::{self, rgb, rgba},
    tooltip::Tooltip,
};
use crate::{
    audio::{PlayUiSound, UiSound},
    palette,
};

/// Farben einer Taste.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Key {
    /// Fläche (getöntes Glas, leicht durchscheinend).
    pub face: Color,
    /// Leuchtfarbe von Rahmen, Eckklammern und Schein.
    pub glow: Color,
}

impl Key {
    /// Gewöhnliche Taste.
    pub const NORMAL: Self = Self {
        face: rgba(18, 28, 42, 0.86),
        glow: palette::PLAYER,
    };

    /// Hauptaktion („Zug beenden“, „Übernehmen“, „Neues Match“): leuchtet in der Auswahlfarbe.
    pub const PRIMARY: Self = Self {
        face: rgba(12, 56, 88, 0.9),
        glow: palette::SELECT,
    };

    /// Gefahr („Aufgeben“): warnt rot.
    pub const DANGER: Self = Self {
        face: rgba(76, 20, 18, 0.9),
        glow: palette::LOST,
    };

    /// Icon-Taste oben rechts.
    pub const ICON: Self = Self::NORMAL;

    /// Taste im Ton einer Akzentfarbe, etwa die Zielerfassung in der Farbe des Admirals.
    pub fn tinted(accent: Color) -> Self {
        let face = rgb(14, 18, 26).to_srgba().mix(&accent.to_srgba(), 0.22);
        Self {
            face: Color::from(face).with_alpha(0.9),
            glow: accent,
        }
    }
}

/// Eine Taste, die eingeschaltet leuchtet wie beim Überfahren (das aufgeklappte Log, eine gewählte Option).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Lit;

/// Eine Taste, die blass aussieht, aber klickbar bleibt (etwa die Zielerfassung ohne Ziel: Der Controller sagt dann
/// hörbar nein).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Dimmed;

/// Eine Taste, deren Leuchten pulsiert, solange sie auf etwas wartet (Zielwahl).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Pulsing;

/// Ein Icon auf einer Taste: gedämpft, beim Überfahren oder eingeschaltet in der Auswahlfarbe.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct KeyIcon;

/// Eine Eckklammer der Taste; `Vec2` zeigt von der Mitte nach außen (x: −1 links, +1 rechts; y: −1 oben, +1 unten).
#[derive(Component, Clone, Copy, Debug)]
struct KeyCorner(Vec2);

/// Der laufende Zustand der Taste; folgt dem Ziel aus Hover, Druck und Sperre.
#[derive(Component, Clone, Copy, Debug)]
struct KeyMotion {
    /// Wie weit die Eckklammern nach außen gefahren sind (logische Pixel, negativ: nach innen).
    spread: f32,
    /// Unschärfe des Scheins um die Taste.
    halo: f32,
    /// Helligkeit der Fläche.
    bright: f32,
    /// Leuchten von Klammern und Licht (0 Ruhe, 1 aktiv).
    heat: f32,
    /// Noch nie gezeichnet (oder neue Farben): einmal anwenden, auch wenn sich nichts bewegt.
    fresh: bool,
}

/// Kantenlänge und Stärke der Eckklammern (logische Pixel).
const CORNER: f32 = 7.0;
const CORNER_WIDTH: f32 = 2.0;

/// Eine Taste: `node` legt Größe und Innenabstand fest, Rand und Ecken setzt die Taste selbst.
pub fn button(key: Key, node: Node) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        key,
        KeyMotion {
            spread: 0.0,
            halo: 0.0,
            bright: 1.0,
            heat: 0.0,
            fresh: true,
        },
        Node {
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::ZERO,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..node
        },
        BackgroundColor(key.face),
        glass(key.glow, 0.0, 1.0),
        BorderColor::all(key.glow.with_alpha(0.32)),
        BoxShadow::default(),
    )
}

/// Das Licht auf dem Glas: ein heller Streif oben, ein Schimmer der Leuchtfarbe, der von unten aufsteigt (stärker mit
/// `heat`).
fn glass(glow: Color, heat: f32, opacity: f32) -> BackgroundGradient {
    theme::vertical_gradient(&[
        (Color::WHITE.with_alpha(0.12 * opacity), 0.0),
        (Color::WHITE.with_alpha(0.03 * opacity), 8.0),
        (Color::WHITE.with_alpha(0.0), 50.0),
        (glow.with_alpha((0.1 + 0.18 * heat) * opacity), 100.0),
    ])
}

/// Eine Eckklammer: zwei Kanten eines kleinen Quadrats, bündig auf der Ecke des Rahmens.
fn corner(direction: Vec2, color: Color) -> impl Bundle {
    let at = |outward: bool| if outward { px(-1.0) } else { Val::Auto };
    let edge = |on: bool| if on { px(CORNER_WIDTH) } else { Val::ZERO };
    let (left, top) = (direction.x < 0.0, direction.y < 0.0);
    (
        KeyCorner(direction),
        Node {
            position_type: PositionType::Absolute,
            width: px(CORNER),
            height: px(CORNER),
            left: at(left),
            right: at(!left),
            top: at(top),
            bottom: at(!top),
            border: UiRect {
                left: edge(left),
                right: edge(!left),
                top: edge(top),
                bottom: edge(!top),
            },
            ..default()
        },
        BorderColor::all(color),
        UiTransform::default(),
        Pickable::IGNORE,
    )
}

/// Hängt jeder neuen Taste ihre vier Eckklammern an.
fn add_corners(add: On<Add<Key>>, keys: Query<&Key>, mut commands: Commands) {
    let Ok(key) = keys.get(add.entity) else {
        return;
    };
    for direction in [
        Vec2::new(-1.0, -1.0),
        Vec2::new(1.0, -1.0),
        Vec2::new(-1.0, 1.0),
        Vec2::new(1.0, 1.0),
    ] {
        commands.spawn((corner(direction, key.glow), ChildOf(add.entity)));
    }
}

/// Eine Taste mit Beschriftung in der Display-Schrift (groß geschrieben und gesperrt).
pub fn text_button(font: TextFont, label: &str, key: Key, node: Node) -> impl Bundle {
    (
        button(key, node),
        children![(
            theme::caps(font, label, theme::TRACKING, palette::TEXT),
            theme::text_shadow(0.55),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        )],
    )
}

/// Eine Icon-Taste oben rechts (32 × 32) mit Tooltip.
pub fn icon_button(icon: Handle<Image>, tooltip: impl Into<String>) -> impl Bundle {
    (
        button(
            Key::ICON,
            Node {
                width: px(32),
                height: px(32),
                ..default()
            },
        ),
        Tooltip(tooltip.into()),
        children![icon_image(icon, 18.0)],
    )
}

/// Ein Icon auf einer Taste in der Größe `size`.
pub fn icon_image(icon: Handle<Image>, size: f32) -> impl Bundle {
    (
        KeyIcon,
        Node {
            width: px(size),
            height: px(size),
            ..default()
        },
        ImageNode::new(icon).with_color(palette::TEXT_DIM),
        Pickable::IGNORE,
    )
}

/// Wechselt das Icon einer Icon-Taste (Vollbild an/aus).
pub fn set_icon(
    key: Entity,
    icon: &Handle<Image>,
    children: &Query<&Children>,
    images: &mut Query<&mut ImageNode, With<KeyIcon>>,
) {
    for child in children.iter_descendants(key) {
        if let Ok(mut image) = images.get_mut(child)
            && image.image != *icon
        {
            image.image = icon.clone();
        }
    }
}

/// Gibt eine Taste frei oder sperrt sie.
pub fn set_enabled(commands: &mut Commands, key: Entity, enabled: bool) {
    if enabled {
        commands.entity(key).remove::<InteractionDisabled>();
    } else {
        commands.entity(key).insert(InteractionDisabled);
    }
}

/// Wie schnell die Taste ihrem Ziel folgt (1/s): etwa 0,08 s, gedrückt schneller.
const FOLLOW: f32 = 40.0;
const FOLLOW_PRESSED: f32 = 100.0;
/// Halbe Periode des Pulsierens (Sekunden).
const PULSE: f32 = 1.1;

type KeyState<'a> = (
    Entity,
    Ref<'a, Key>,
    &'a Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    Has<Lit>,
    Has<Dimmed>,
    Has<Pulsing>,
    &'a mut KeyMotion,
    &'a mut BackgroundColor,
    &'a mut BackgroundGradient,
    &'a mut BoxShadow,
);

/// Lässt die Tasten nach Hover, Druck und Sperre aufleuchten und ihre Eckklammern fahren.
fn animate_keys(
    time: Res<Time>,
    mut keys: Query<KeyState>,
    children: Query<&Children>,
    mut corners: Query<(&KeyCorner, &mut BorderColor, &mut UiTransform)>,
    mut labels: Query<&mut TextColor>,
    mut icons: Query<&mut ImageNode, With<KeyIcon>>,
) {
    let pulse = 15.0 + 9.0 * (time.elapsed_secs() / PULSE * std::f32::consts::PI).sin();
    let delta = time.delta_secs();
    for (
        entity,
        key,
        hovered,
        pressed,
        disabled,
        lit,
        dimmed,
        pulsing,
        mut motion,
        mut face,
        mut gradient,
        mut shadow,
    ) in &mut keys
    {
        let hovered = hovered.get() && !disabled;
        let pressed = pressed && !disabled;
        let (spread, halo, bright, heat) = if disabled {
            (0.0, 0.0, 0.8, 0.0)
        } else if pressed {
            (-1.0, 10.0, 1.45, 1.0)
        } else if hovered || lit {
            (2.5, 14.0, 1.3, 1.0)
        } else {
            (0.0, 0.0, 1.0, 0.0)
        };
        let halo = if pulsing && !pressed { pulse } else { halo };
        let rate = if pressed { FOLLOW_PRESSED } else { FOLLOW };
        // Eine neue Taste steht sofort in ihrem Zustand: Dialoge bauen ihre Tasten nach jedem Klick neu, sonst blendet
        // jede leuchtende Taste dabei wieder auf.
        let next = if motion.fresh {
            KeyMotion {
                spread,
                halo,
                bright,
                heat,
                fresh: false,
            }
        } else {
            let mut next = KeyMotion {
                fresh: false,
                ..*motion
            };
            next.spread.smooth_nudge(&spread, rate, delta);
            next.halo.smooth_nudge(&halo, rate, delta);
            next.bright.smooth_nudge(&bright, rate, delta);
            next.heat.smooth_nudge(&heat, rate, delta);
            next
        };
        let opacity = if disabled {
            0.5
        } else if dimmed {
            0.45
        } else {
            1.0
        };
        let alpha = key.face.alpha() * opacity;
        let settled = !motion.fresh
            && !key.is_changed()
            && (next.spread - motion.spread).abs() < 0.001
            && (next.halo - motion.halo).abs() < 0.001
            && (next.bright - motion.bright).abs() < 0.0005
            && (next.heat - motion.heat).abs() < 0.0005
            && (face.0.alpha() - alpha).abs() < 0.001;
        if settled {
            continue;
        }
        *motion = next;
        let lit_face = key.face.to_srgba() * motion.bright;
        face.0 = Color::from(lit_face).with_alpha(alpha);
        *gradient = glass(key.glow, motion.heat, opacity);
        // Bevy zeichnet die Schatten von hinten nach vorn.
        shadow.0 = vec![
            theme::glow(key.glow.with_alpha(0.5 * opacity), motion.halo),
            theme::drop(Color::BLACK.with_alpha(0.5 * opacity), 0.0, 2.0, 6.0),
        ];
        let bracket = key.glow.with_alpha((0.55 + 0.45 * motion.heat) * opacity);
        let mut found = false;
        if let Ok(list) = children.get(entity) {
            for child in list.iter() {
                if let Ok((corner, mut border, mut transform)) = corners.get_mut(child) {
                    found = true;
                    border.set_all(bracket);
                    let offset = corner.0 * motion.spread;
                    transform.translation = Val2::px(offset.x, offset.y);
                }
            }
        }
        // Die Klammern entstehen erst mit den Befehlen nach dem Bau: bis dahin noch einmal anwenden.
        motion.fresh = !found;
        for child in children.iter_descendants(entity) {
            if let Ok(mut label) = labels.get_mut(child) {
                label.0.set_alpha(opacity);
            }
            if let Ok(mut icon) = icons.get_mut(child) {
                let tint = if hovered || lit {
                    palette::SELECT
                } else {
                    palette::TEXT_DIM
                };
                icon.color = tint.with_alpha(tint.alpha() * opacity);
            }
        }
    }
}

/// Jede Taste klickt hörbar.
fn click_sound(
    activate: On<Activate>,
    keys: Query<(), With<Key>>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if keys.contains(activate.entity) {
        sounds.write(PlayUiSound(UiSound::Click));
    }
}

pub(super) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(click_sound)
            .add_observer(add_corners)
            // Nach `Update`: Tasten, die dort neu gebaut werden, zeigen schon im ersten Bild ihren Zustand.
            .add_systems(PostUpdate, animate_keys.before(UiSystems::Prepare));
    }
}
