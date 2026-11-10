//! FPS-Anzeige oben links, über den Einstellungen (Tab „Video“) ein- und ausschaltbar.

use std::time::Duration;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    time::common_conditions::on_timer,
};

use super::{Fonts, theme};
use crate::{palette, settings::Settings};

#[derive(Component)]
struct FpsText;

fn spawn(mut commands: Commands, fonts: Res<Fonts>) {
    commands.spawn((
        Name::new("FPS"),
        FpsText,
        theme::text(fonts.body(12.0), "", palette::TEXT_DIM),
        Node {
            position_type: PositionType::Absolute,
            top: px(4),
            left: px(8),
            ..default()
        },
        GlobalZIndex(300),
        Pickable::IGNORE,
    ));
}

fn show(settings: Res<Settings>, mut node: Single<&mut Node, With<FpsText>>) {
    node.display = if settings.show_fps {
        Display::Flex
    } else {
        Display::None
    };
}

fn update(diagnostics: Res<DiagnosticsStore>, mut text: Single<&mut Text, With<FpsText>>) {
    // Die Bildzeit mitteln, nicht Bevys `FPS` (Mittel über 1/Δt): Das schießt bei schwankenden Bildabständen über die
    // Bildwiederholrate hinaus, obwohl im Schnitt nicht mehr Bilder kommen.
    if let Some(ms) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|frame_time| frame_time.smoothed())
    {
        text.0 = format!("{:.0} FPS", 1000.0 / ms);
    }
}

pub(super) struct FpsPlugin;

impl Plugin for FpsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, spawn)
            .add_systems(
                Update,
                (
                    show.run_if(resource_changed::<Settings>),
                    update
                        .run_if(on_timer(Duration::from_secs(1)))
                        .run_if(|settings: Res<Settings>| settings.show_fps),
                ),
            );
    }
}
