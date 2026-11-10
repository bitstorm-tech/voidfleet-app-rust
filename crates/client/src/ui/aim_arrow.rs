//! Der Zielpfeil: Wartet eine Karte oder Station auf ein gegnerisches Ziel ([`HudState::aiming`]), zieht sich von
//! ihrer oberen Kante ein holografisches Band mit Pfeilspitze zum Mauszeiger und folgt ihm. Solange der Pfeil steht
//! ([`AimArrowShown`]), ist seine Spitze der Zeiger; den Systemzeiger blendet [`super::cursor`] dann aus.
//!
//! Ein Knoten über das ganze Fenster mit eigenem Material (`assets/shaders/aim_arrow.wesl`); das System setzt nur die
//! drei Punkte der Kurve ([`curve`]). Der Knoten steht schon während der Generalprobe sichtbar und ohne Werte (der
//! Shader zeichnet dann nichts), damit Bevy seine Pipeline vor dem ersten Zielen übersetzt.

use bevy::{
    prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef, window::PrimaryWindow,
};

use super::{HudState, card::Marked, hand::HandStack, stations::StationButton, theme::Modal};
use crate::{
    assets::{AppState, LoadingPhase},
    palette,
};

/// So weit muss der Zeiger über dem Gewählten stehen, damit der Pfeil erscheint (logische Pixel; die Spitze braucht
/// den Platz): Auf der Hand selbst gibt es nichts zu zielen, dort bleibt der Systemzeiger.
const MIN_RISE: f32 = 48.0;

/// Ob der Pfeil gerade steht.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AimArrowShown(pub bool);

/// Die Werte des Shaders, alle Punkte in physischen Pixeln des Fensters.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct AimArrowMaterial {
    /// Anfang (xy) und Leitpunkt (zw) der Kurve.
    #[uniform(0)]
    from_control: Vec4,
    /// Spitze (xy), physische Pixel je logischem (z).
    #[uniform(0)]
    tip_scale: Vec4,
    #[uniform(0)]
    color: LinearRgba,
}

impl UiMaterial for AimArrowMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/aim_arrow.wesl".into()
    }
}

/// Der Leitpunkt der Kurve vom Gewählten (`from`) zum Zeiger: Sie steigt senkrecht auf und biegt dann zum Zeiger ab.
/// `None`, solange der Zeiger nicht über dem Gewählten steht.
fn control(from: Vec2, cursor: Vec2, px: f32) -> Option<Vec2> {
    (cursor.y <= from.y - MIN_RISE * px).then(|| {
        Vec2::new(
            from.x,
            from.y.lerp(cursor.y, 0.75) - 0.15 * (cursor.x - from.x).abs(),
        )
    })
}

fn spawn(mut commands: Commands, mut materials: ResMut<Assets<AimArrowMaterial>>) {
    commands.spawn((
        Name::new("Zielpfeil"),
        MaterialNode(materials.add(AimArrowMaterial::default())),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        // Über Hand und Stationen, unter Bannern und Dialogen.
        GlobalZIndex(3),
        Pickable::IGNORE,
    ));
}

fn aim(
    hud: Res<HudState>,
    window: Single<&Window, With<PrimaryWindow>>,
    cards: Query<(&ComputedNode, &UiGlobalTransform), (With<HandStack>, With<Marked>)>,
    stations: Query<(&StationButton, &ComputedNode, &UiGlobalTransform)>,
    modal: Query<(), With<Modal>>,
    arrow: Single<(&MaterialNode<AimArrowMaterial>, &mut Visibility)>,
    mut materials: ResMut<Assets<AimArrowMaterial>>,
    mut shown: ResMut<AimArrowShown>,
) {
    let (material, mut visibility) = arrow.into_inner();
    let px = window.scale_factor();
    let source = cards.iter().next().or_else(|| {
        stations
            .iter()
            .find(|(button, ..)| hud.station == Some(button.0))
            .map(|(_, node, transform)| (node, transform))
    });
    // Über einem Dialog zeigt wieder der Systemzeiger.
    let points = source
        .filter(|_| hud.aiming && modal.is_empty())
        .zip(window.physical_cursor_position())
        .and_then(|((node, transform), cursor)| {
            let from = transform.transform_point2(Vec2::new(0.0, -node.size().y / 2.0));
            Some((from, control(from, cursor, px)?, cursor))
        });
    shown.set_if_neq(AimArrowShown(points.is_some()));
    let Some((from, control, tip)) = points else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    if let Some(mut material) = materials.get_mut(&material.0) {
        material.from_control = from.extend(control.x).extend(control.y);
        material.tip_scale = tip.extend(px).extend(0.0);
        material.color = palette::SELECT.to_linear();
    }
    visibility.set_if_neq(Visibility::Inherited);
}

pub(super) struct AimArrowPlugin;

impl Plugin for AimArrowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AimArrowShown>()
            .add_plugins(UiMaterialPlugin::<AimArrowMaterial>::default())
            .add_systems(OnEnter(LoadingPhase::Rehearsal), spawn)
            .add_systems(Update, aim.run_if(in_state(AppState::Match)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_rises_from_the_card() {
        let from = Vec2::new(500.0, 800.0);
        // Der Zeiger liegt noch auf der Hand.
        assert_eq!(control(from, Vec2::new(520.0, 790.0), 1.0), None);

        let bend = control(from, Vec2::new(900.0, 300.0), 2.0).expect("Zeiger über der Karte");
        assert_eq!(bend.x, from.x);
        assert!(bend.y < from.y);

        // Senkrecht über der Karte liegt der Leitpunkt vor dem Zeiger: Die Spitze hat eine Richtung.
        let bend = control(from, Vec2::new(500.0, 300.0), 1.0).expect("gerade");
        assert!(bend.y > 300.0);
    }
}
