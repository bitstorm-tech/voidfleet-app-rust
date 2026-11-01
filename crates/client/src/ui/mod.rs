//! Oberfläche: Schriften ([`Fonts`]), Fenster-Icon und Ladebildschirm ([`loading_screen`]). Die übrigen Teile der
//! DOM-Oberfläche (Hand, HUD, Dialoge) kommen später als Bevy-UI dazu.

pub mod loading_screen;

use bevy::{ecs::system::NonSendMarker, prelude::*, window::PrimaryWindow, winit::WINIT_WINDOWS};

use crate::assets::Preload;

/// Die Schriften (OFL, latin-Untermenge von Google Fonts): Exo 2 für Fließtext, Rajdhani (600 und 700) für Titel,
/// Knöpfe, Labels, Kartennamen und Schwebezahlen, dort groß geschrieben und gesperrt.
#[derive(Resource, Clone)]
pub struct Fonts {
    pub body: Handle<Font>,
    pub display: Handle<Font>,
    pub display_bold: Handle<Font>,
}

impl Fonts {
    /// Fließtext in dieser Größe (logische Pixel).
    pub fn body(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size)).with_font(self.body.clone())
    }

    /// Display-Schrift (Rajdhani 600).
    pub fn display(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size)).with_font(self.display.clone())
    }

    /// Display-Schrift fett (Rajdhani 700).
    pub fn display_bold(&self, size: f32) -> TextFont {
        TextFont::from_font_size(FontSize::Px(size)).with_font(self.display_bold.clone())
    }
}

/// Icon der App (`assets/app/`), für das Fenster unter Windows und Linux; unter macOS kommt das Icon aus dem Bundle.
#[derive(Resource)]
struct AppIcon(Handle<Image>);

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load)
            .add_plugins(loading_screen::LoadingScreenPlugin)
            .add_systems(Update, set_window_icon);
    }
}

fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    commands.insert_resource(Fonts {
        body: preload.track(server.load("fonts/exo2-regular.ttf")),
        display: preload.track(server.load("fonts/rajdhani-600.ttf")),
        display_bold: preload.track(server.load("fonts/rajdhani-700.ttf")),
    });
    commands.insert_resource(AppIcon(server.load("app/icon-192.png")));
}

/// Setzt das Icon, sobald es geladen ist (einmal; `winit` erlaubt das nur im Haupt-Thread).
fn set_window_icon(
    mut done: Local<bool>,
    icon: Res<AppIcon>,
    images: Res<Assets<Image>>,
    window: Single<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    if *done {
        return;
    }
    let Some(image) = images.get(&icon.0) else {
        return;
    };
    *done = true;
    let size = image.size();
    let Some(rgba) = image
        .clone()
        .try_into_dynamic()
        .ok()
        .map(|image| image.into_rgba8().into_raw())
    else {
        warn!("App-Icon nicht lesbar");
        return;
    };
    match winit::window::Icon::from_rgba(rgba, size.x, size.y) {
        Ok(icon) => WINIT_WINDOWS.with_borrow(|windows| {
            if let Some(window) = windows.get_window(*window) {
                window.set_window_icon(Some(icon));
            }
        }),
        Err(error) => warn!("App-Icon ungültig: {error}"),
    }
}
