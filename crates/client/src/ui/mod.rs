//! Oberfläche in `bevy_ui` (Port des DOM-Overlays `src/ui/` samt `styles.css`).
//!
//! Fundament, das auch die Dialoge tragen:
//! - [`theme`] — Farben der Flächen, Schrift-Helfer, Steuerpult der Dialoge, [`theme::UiOpacity`], [`theme::Modal`].
//! - [`button`] — Tasten mit Tiefe ([`button::button`], [`button::text_button`], [`button::icon_button`]).
//! - [`tooltip`] — [`tooltip::Tooltip`] statt des `title`-Attributs.
//! - [`card`] — Schiffs-, Befehls- und Fähigkeitskarten ([`card::ship_card`], [`card::command_card`],
//!   [`card::power_card`]).
//!
//! Das Match-HUD:
//! - [`hand`] — beide Hände, Stapel, Auswahl, Hover-Zoom, Einflug neu gezogener Karten.
//! - [`hud`] — Runde, Overdrive, Status, Aktionsknopf mit Checkliste, Log, Banner.
//! - [`power_card`] — Admiralsfähigkeit und Zielerfassung über der Hand.
//! - [`opponent_panel`] — was man vom Gegner weiß.
//! - [`round_log`] — Rundenereignisse als Log-Zeilen.
//! - [`radio_subtitle`] — Untertitel des Bordfunks.
//! - [`view_toggle`], [`fullscreen`] — Icon-Knöpfe und Tasten V, F und Pfeiltasten.
//! - [`cursor`] — der gewählte Mauszeiger als Fenster-Cursor.
//! - [`preview`] — Dev-Ansicht mit Beispiel-Hand (`VOIDFLEET_UI_PREVIEW`).
//!
//! Schnittstelle zum Controller: Die Widgets lesen [`Match`](crate::game::Match), [`Locale`] und [`HudState`]
//! (was der Controller festlegt: Status, Aktionsknopf, ob geplant wird, was gewählt ist) und melden Absichten als
//! Messages ([`HandCardPressed`], [`PowerPressed`], [`MarkPressed`], [`ActionPressed`]); was sie bedeuten,
//! entscheidet der Controller. Log und Banner füllt er mit [`AppendLog`], [`ShowBanner`], [`HideBanner`] und
//! [`ShowRoundBanner`].

pub mod button;
pub mod card;
pub mod cursor;
pub mod fullscreen;
pub mod hand;
pub mod hud;
pub mod loading_screen;
pub mod opponent_panel;
pub mod power_card;
pub mod preview;
pub mod radio_subtitle;
pub mod round_log;
pub mod theme;
pub mod tooltip;
pub mod view_toggle;

use bevy::{ecs::system::NonSendMarker, prelude::*, window::PrimaryWindow, winit::WINIT_WINDOWS};
use voidfleet_core::{admirals::AdmiralId, board::Side};

use crate::{
    assets::{AppState, Preload},
    game::ShipPick,
    i18n::Locale,
};

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

/// Die Icons der Oberfläche (`images/ui/<name>.png`, weiß, gerastert von `tools/rasterize`); eingefärbt wird beim
/// Anzeigen.
#[derive(Resource, Clone)]
pub struct UiIcons {
    pub log: Handle<Image>,
    pub fullscreen_enter: Handle<Image>,
    pub fullscreen_exit: Handle<Image>,
    pub view_toggle: Handle<Image>,
    pub crosshair: Handle<Image>,
    pub check: Handle<Image>,
    pub crown: Handle<Image>,
}

/// Was an der Stelle des Aktionsknopfs steht.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HudAction {
    /// „Zug beenden“ (nach dem Abgeben „Warte auf den Gegner …“).
    #[default]
    EndTurn,
    /// „Animation überspringen“, während eine Runde abgespielt wird.
    Skip,
    /// „Gegen Computer spielen“, solange kein Gegner da ist.
    PlayComputer,
    /// „Mitspieler einladen“: macht den Platz des Computergegners frei.
    InvitePlayer,
}

/// Was gerade gewählt ist, laut Controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Selected {
    Ship(ShipPick),
    /// Hand-Index der Befehlskarte.
    Command(usize),
    /// Die Admiralsfähigkeit wartet auf ihr Ziel.
    Power,
    /// Die Zielerfassung der Pendulum wartet auf ihr Ziel.
    Mark,
}

/// Was der Controller für das HUD festlegt; Hand, HUD und Fähigkeitskarte zeigen es zusammen mit dem Spielzustand.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct HudState {
    /// Die Statuszeile oben.
    pub status: String,
    pub action: HudAction,
    /// Der Aktionsknopf ist frei (etwa: geplant werden darf, ein Gegner ist da, das Flaggschiff liegt).
    pub action_enabled: bool,
    /// Es wird geplant: Checkliste sichtbar, Fähigkeit und Zielerfassung wählbar.
    pub planning: bool,
    pub selected: Option<Selected>,
}

/// Eine Karte der Hand: Schiff (Hand-Index oder Flaggschiff) bzw. Befehl (Hand-Index). Bei Stapeln ist es das
/// Exemplar, das ein Klick meint: das gewählte, sonst das nächste freie, sonst das zuletzt eingeplante.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandCard {
    Ship(ShipPick),
    Command(usize),
}

/// Eine Karte der Hand wurde angeklickt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandCardPressed(pub HandCard);

/// Die Karte der Admiralsfähigkeit wurde angeklickt.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct PowerPressed;

/// Der Knopf der Zielerfassung (Pendulum) wurde angeklickt.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct MarkPressed;

/// Der Aktionsknopf wurde angeklickt; `HudAction` ist, was er dabei zeigte.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionPressed(pub HudAction);

/// Zeilen ans Log anhängen.
#[derive(Message, Clone, Debug, Default)]
pub struct AppendLog(pub Vec<String>);

/// Kündigt einen aufgedeckten Befehl oder eine Fähigkeit an; mit `admiral` das Porträt in seiner Farbe. Blendet sich
/// selbst wieder aus.
#[derive(Message, Clone, Debug)]
pub struct ShowBanner {
    pub title: String,
    pub subtitle: Option<String>,
    pub side: Side,
    pub admiral: Option<AdmiralId>,
}

/// Nimmt das Banner sofort weg (etwa beim Überspringen).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct HideBanner;

/// Kündigt groß an, dass Runde `0` begonnen hat (samt steigendem Overdrive).
#[derive(Message, Clone, Copy, Debug)]
pub struct ShowRoundBanner(pub u32);

/// Die Leiste der Icon-Knöpfe oben rechts, von rechts nach links: Log, Vollbild, Ansicht; Dialoge hängen ihre Knöpfe
/// als weitere Kinder an.
#[derive(Component, Debug)]
pub struct IconBar;

/// Icon der App (`assets/app/`), für das Fenster unter Windows und Linux; unter macOS kommt das Icon aus dem Bundle.
#[derive(Resource)]
struct AppIcon(Handle<Image>);

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .add_message::<HandCardPressed>()
            .add_message::<PowerPressed>()
            .add_message::<MarkPressed>()
            .add_message::<ActionPressed>()
            .add_message::<AppendLog>()
            .add_message::<ShowBanner>()
            .add_message::<HideBanner>()
            .add_message::<ShowRoundBanner>()
            .add_systems(PreStartup, load)
            .add_plugins((
                loading_screen::LoadingScreenPlugin,
                theme::ThemePlugin,
                button::ButtonPlugin,
                tooltip::TooltipPlugin,
                card::CardPlugin,
                hand::HandPlugin,
                hud::HudPlugin,
                power_card::PowerCardPlugin,
                opponent_panel::OpponentPanelPlugin,
                radio_subtitle::RadioSubtitlePlugin,
                view_toggle::ViewTogglePlugin,
                fullscreen::FullscreenPlugin,
                cursor::CursorPlugin,
                preview::PreviewPlugin,
            ))
            .add_systems(OnEnter(AppState::Match), spawn_icon_bar)
            .add_systems(Update, set_window_icon);
    }
}

fn load(mut commands: Commands, server: Res<AssetServer>, mut preload: ResMut<Preload>) {
    commands.insert_resource(Fonts {
        body: preload.track(server.load("fonts/exo2-regular.ttf")),
        display: preload.track(server.load("fonts/rajdhani-600.ttf")),
        display_bold: preload.track(server.load("fonts/rajdhani-700.ttf")),
    });
    let mut icon = |name: &str| preload.track(server.load(format!("images/ui/{name}.png")));
    commands.insert_resource(UiIcons {
        log: icon("log"),
        fullscreen_enter: icon("fullscreen_enter"),
        fullscreen_exit: icon("fullscreen_exit"),
        view_toggle: icon("view_toggle"),
        crosshair: icon("crosshair"),
        check: icon("check"),
        crown: icon("crown"),
    });
    commands.insert_resource(AppIcon(server.load("app/icon-192.png")));
}

/// Die Icon-Knöpfe oben rechts.
fn spawn_icon_bar(mut commands: Commands, icons: Res<UiIcons>, locale: Res<Locale>) {
    commands.spawn((
        Name::new("Icon-Knöpfe"),
        IconBar,
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            right: px(8),
            flex_direction: FlexDirection::RowReverse,
            column_gap: px(4),
            ..default()
        },
        GlobalZIndex(10),
        Pickable::IGNORE,
        children![
            hud::log_button(&icons, &locale),
            fullscreen::fullscreen_button(&icons, &locale),
            view_toggle::view_button(&icons, &locale),
        ],
    ));
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
