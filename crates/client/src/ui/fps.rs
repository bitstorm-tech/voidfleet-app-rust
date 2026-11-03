//! FPS-Anzeige oben links, über den Einstellungen (Tab „Video“) ein- und ausschaltbar.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
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
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    {
        text.0 = format!("{fps:.0} FPS");
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
                    update.run_if(|settings: Res<Settings>| settings.show_fps),
                ),
            );
    }
}
