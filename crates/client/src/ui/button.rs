//! Knöpfe als Tasten eines Steuerpults (Port von `#ui button` in `styles.css`): gewölbte Oberseite, sichtbare Flanke
//! unten und Schlagschatten. Hover hebt die Taste an und lässt sie leuchten, Drücken senkt sie fast bis aufs Pult,
//! gesperrt ([`InteractionDisabled`]) ist sie blass. Varianten setzen nur Farben und Ruhehöhe ([`Key`]).
//!
//! Gebaut wird mit [`button`], [`text_button`] oder [`icon_button`]; die Taste ist ein `bevy_ui_widgets`-[`Button`]
//! und meldet einen Klick (auch Enter/Leertaste bei Fokus) als [`Activate`] an ihre Entity — wer sie baut, hängt einen
//! Observer an (`.observe(|_: On<Activate>| …)`). Den Klick-Klang spielt jede Taste selbst.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button},
};

use super::{
    theme::{self, rgb},
    tooltip::Tooltip,
};
use crate::{
    audio::{PlayUiSound, UiSound},
    palette,
};

/// Farben und Ruhehöhe einer Taste (`--face`, `--edge`, `--glow`, `--rest`).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Key {
    /// Oberseite.
    pub face: Color,
    /// Flanke unten.
    pub edge: Color,
    /// Leuchten beim Überfahren.
    pub glow: Color,
    /// Höhe der Flanke in Ruhe (logische Pixel).
    pub rest: f32,
}

impl Key {
    /// Gewöhnliche Taste.
    pub const NORMAL: Self = Self {
        face: rgb(52, 60, 76),
        edge: rgb(20, 24, 32),
        glow: palette::PLAYER,
        rest: 4.0,
    };

    /// Hauptaktion („Zug beenden“, „Übernehmen“, „Neues Match“): leuchtet in der eigenen Farbe.
    pub const PRIMARY: Self = Self {
        face: rgb(34, 96, 150),
        edge: rgb(12, 40, 66),
        glow: palette::SELECT,
        rest: 4.0,
    };

    /// Gefahr („Aufgeben“): warnt rot.
    pub const DANGER: Self = Self {
        face: rgb(132, 40, 34),
        edge: rgb(58, 16, 14),
        glow: palette::LOST,
        rest: 4.0,
    };

    /// Flache Icon-Taste oben rechts.
    pub const ICON: Self = Self {
        rest: 3.0,
        ..Self::NORMAL
    };

    /// Taste im Ton einer Akzentfarbe, etwa die Zielerfassung in der Farbe des Admirals.
    pub fn tinted(accent: Color) -> Self {
        let accent = accent.to_srgba();
        let mix = |base: Srgba, share: f32| Color::from(base.mix(&accent, share));
        Self {
            face: mix(rgb(30, 34, 44).to_srgba(), 0.3),
            edge: mix(rgb(10, 12, 16).to_srgba(), 0.25),
            glow: accent.into(),
            rest: 3.0,
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

/// Der laufende Zustand der Tastenbewegung; folgt dem Ziel aus Hover, Druck und Sperre.
#[derive(Component, Clone, Copy, Debug)]
struct KeyMotion {
    depth: f32,
    halo: f32,
    bright: f32,
}

/// Eine Taste: `node` legt Größe und Innenabstand fest, Rand und Ecken setzt die Taste selbst.
pub fn button(key: Key, node: Node) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        key,
        KeyMotion {
            depth: key.rest,
            halo: 0.0,
            bright: 1.0,
        },
        Node {
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..node
        },
        BackgroundColor(key.face),
        // Wölbung: oben Licht, unten Schatten, über der Farbe der Oberseite.
        theme::vertical_gradient(&[
            (Color::WHITE.with_alpha(0.17), 0.0),
            (Color::WHITE.with_alpha(0.04), 46.0),
            (Color::BLACK.with_alpha(0.04), 54.0),
            (Color::BLACK.with_alpha(0.24), 100.0),
        ]),
        BorderColor::all(Color::BLACK.with_alpha(0.65)),
        BoxShadow::default(),
    )
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

/// Wie schnell die Taste ihrem Ziel folgt (1/s): etwa 0,08 s wie die CSS-Transition, gedrückt schneller.
const FOLLOW: f32 = 40.0;
const FOLLOW_PRESSED: f32 = 100.0;
/// Halbe Periode des Pulsierens (Sekunden).
const PULSE: f32 = 1.1;

type KeyState<'a> = (
    Entity,
    &'a Key,
    &'a Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    Has<Lit>,
    Has<Dimmed>,
    Has<Pulsing>,
    &'a mut KeyMotion,
    &'a mut BackgroundColor,
    &'a mut BoxShadow,
    &'a mut UiTransform,
);

/// Hebt, senkt und beleuchtet die Tasten nach Hover, Druck und Sperre.
fn animate_keys(
    time: Res<Time>,
    mut keys: Query<KeyState>,
    children: Query<&Children>,
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
        mut shadow,
        mut transform,
    ) in &mut keys
    {
        let hovered = hovered.get() && !disabled;
        let pressed = pressed && !disabled;
        let (depth, halo, bright) = if disabled {
            (key.rest, 0.0, 0.8)
        } else if pressed {
            (1.0, 8.0, 0.94)
        } else if hovered || lit {
            (key.rest + 1.0, 16.0, 1.14)
        } else {
            (key.rest, 0.0, 1.0)
        };
        let halo = if pulsing && !pressed { pulse } else { halo };
        let rate = if pressed { FOLLOW_PRESSED } else { FOLLOW };
        let mut next = *motion;
        next.depth.smooth_nudge(&depth, rate, delta);
        next.halo.smooth_nudge(&halo, rate, delta);
        next.bright.smooth_nudge(&bright, rate, delta);
        let opacity = if disabled {
            0.5
        } else if dimmed {
            0.45
        } else {
            1.0
        };
        let settled = (next.depth - motion.depth).abs() < 0.001
            && (next.halo - motion.halo).abs() < 0.001
            && (next.bright - motion.bright).abs() < 0.0005
            && (face.0.alpha() - opacity).abs() < 0.001
            && !shadow.is_empty();
        if settled {
            continue;
        }
        *motion = next;
        let lit_face = key.face.to_srgba() * motion.bright;
        face.0 = Color::from(lit_face).with_alpha(opacity);
        transform.translation = Val2::px(0.0, key.rest - motion.depth);
        // CSS nennt den obersten Schatten zuerst, Bevy zeichnet von hinten nach vorn.
        shadow.0 = vec![
            theme::glow(key.glow.with_alpha(0.45 * opacity), motion.halo),
            theme::drop(
                Color::BLACK.with_alpha(0.55 * opacity),
                0.0,
                motion.depth + 3.0,
                9.0,
            ),
            theme::drop(key.edge.with_alpha(opacity), 0.0, motion.depth, 0.0),
        ];
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
            .add_systems(Update, animate_keys);
    }
}
