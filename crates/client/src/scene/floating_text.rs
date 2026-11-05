//! Kurze Texte, die über einem Punkt der Szene aufsteigen und verblassen: Schaden, Heilung, Schild, Buffs (Port von
//! `ui/floatingText.ts` samt der CSS-Animation `float-up`). Ein Text ist ein Auftrag ([`FloatingText`]) und ein Knoten der
//! Oberfläche, der jedes Bild über seinem Weltpunkt steht; er läuft im Takt der Animationsgeschwindigkeit und
//! verschwindet danach selbst.

use bevy::{
    picking::Pickable,
    prelude::*,
    transform::TransformSystems,
    ui::{UiSystems, UiTransform},
};

use super::{animation::Animation, camera::MainCamera};
use crate::{palette, ui::Fonts};

/// Dauer der Animation, Schriftgröße und Weg nach oben (logische Pixel).
const DURATION: f32 = 1.4;
const FONT_SIZE: f32 = 30.0;
const RISE: f32 = 56.0;
const DETAIL_FONT_SIZE: f32 = 15.0;

/// Wie ein Text wirkt: seine Farbe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FloatTone {
    #[default]
    Damage,
    Heal,
    Shield,
    Buff,
    Debuff,
}

impl FloatTone {
    pub fn color(self) -> Color {
        match self {
            Self::Damage => palette::DAMAGE,
            Self::Heal => palette::HEAL,
            Self::Shield => palette::SHIELD,
            Self::Buff => palette::BUFF,
            Self::Debuff => palette::DEBUFF,
        }
    }
}

/// Auftrag: `text` steigt über dem Weltpunkt `at` auf (zentriert) und verblasst; `detail` steht klein darunter (die
/// Rechnung eines Treffers: „Panzerung −2 · Schild schluckt 8“).
#[derive(Component, Clone, Debug, Default)]
pub struct FloatingText {
    pub at: Vec3,
    pub text: String,
    pub detail: String,
    pub tone: FloatTone,
}

impl FloatingText {
    pub fn new(at: Vec3, text: impl Into<String>, tone: FloatTone) -> Self {
        Self {
            at,
            text: text.into(),
            detail: String::new(),
            tone,
        }
    }
}

pub(super) struct FloatingTextPlugin;

impl Plugin for FloatingTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(show).add_systems(
            PostUpdate,
            // Nach den Lagen dieses Bilds und vor dem UI-Layout: Der Text steht im selben Bild dort.
            float
                .after(TransformSystems::Propagate)
                .before(UiSystems::Layout),
        );
    }
}

fn show(
    add: On<Add<FloatingText>>,
    texts: Query<&FloatingText>,
    fonts: Res<Fonts>,
    mut commands: Commands,
) {
    let text = texts.get(add.entity).expect("Schwebetext");
    commands.entity(add.entity).insert((
        Name::new("Schwebetext"),
        Text::new(text.text.clone()),
        fonts.display_bold(FONT_SIZE),
        TextColor(text.tone.color().with_alpha(0.0)),
        TextShadow {
            offset: Vec2::new(0.0, 1.0),
            color: Color::BLACK.with_alpha(0.0),
        },
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        UiTransform {
            translation: Val2::percent(-50.0, -50.0),
            ..default()
        },
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
        Animation::new(DURATION),
    ));
    if !text.detail.is_empty() {
        commands.entity(add.entity).with_child((
            TextSpan::new(format!("\n{}", text.detail)),
            fonts.body(DETAIL_FONT_SIZE),
            TextColor(palette::TEXT.with_alpha(0.0)),
        ));
    }
}

/// `float-up`: erscheint von unten wachsend, federt kurz und steigt dann verblassend auf.
fn keyframe(progress: f32) -> (f32, f32, f32) {
    // Ease-out zwischen zwei Schlüsselbildern.
    let segment = |from: f32, to: f32| {
        EaseFunction::QuadraticOut.sample_clamped((progress - from) / (to - from))
    };
    let opacity = if progress < 0.15 {
        segment(0.0, 0.15)
    } else {
        1.0 - segment(0.15, 1.0)
    };
    let (offset, scale) = if progress < 0.15 {
        let t = segment(0.0, 0.15);
        (10.0 * (1.0 - t), 0.8 + 0.3 * t)
    } else if progress < 0.3 {
        let t = segment(0.15, 0.3);
        (-6.0 * t, 1.1 - 0.1 * t)
    } else {
        let t = segment(0.3, 1.0);
        (-6.0 - (RISE - 6.0) * t, 1.0)
    };
    (opacity, offset, scale)
}

fn float(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut texts: Query<(
        &FloatingText,
        &Animation,
        &mut Node,
        &mut UiTransform,
        &mut TextColor,
        &mut TextShadow,
        Option<&Children>,
    )>,
    mut details: Query<&mut TextColor, (With<TextSpan>, Without<FloatingText>)>,
) {
    let (camera, camera_transform) = *camera;
    for (text, animation, mut node, mut transform, mut color, mut shadow, children) in &mut texts {
        let (opacity, offset, scale) = keyframe(animation.progress());
        if let Ok(position) = camera.world_to_viewport(camera_transform, text.at) {
            node.left = Val::Px(position.x);
            node.top = Val::Px(position.y + offset);
        }
        transform.scale = Vec2::splat(scale);
        color.0 = text.tone.color().with_alpha(opacity);
        for &child in children.into_iter().flatten() {
            if let Ok(mut detail) = details.get_mut(child) {
                detail.0 = palette::TEXT.with_alpha(opacity);
            }
        }
        shadow.color = Color::BLACK.with_alpha(0.9 * opacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rises_and_fades() {
        assert_eq!(keyframe(0.0), (0.0, 10.0, 0.8));
        let (opacity, _, scale) = keyframe(0.15);
        assert!((opacity - 1.0).abs() < 1e-5 && (scale - 1.1).abs() < 1e-5);
        let (opacity, offset, _) = keyframe(1.0);
        assert!(opacity.abs() < 1e-5 && (offset + RISE).abs() < 1e-5);
    }
}
