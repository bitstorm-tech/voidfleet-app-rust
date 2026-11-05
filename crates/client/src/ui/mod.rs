//! Oberfläche in `bevy_ui` (Port des DOM-Overlays `src/ui/` samt `styles.css`).
//!
//! Fundament, das auch die Dialoge tragen:
//! - [`theme`] — Farben der Flächen, Schrift-Helfer, Steuerpult der Dialoge, [`theme::UiOpacity`], [`theme::Modal`].
//! - [`button`] — HUD-Schaltflächen mit Eckklammern ([`button::button`], [`button::text_button`],
//!   [`button::icon_button`]).
//! - [`tooltip`] — [`tooltip::Tooltip`] statt des `title`-Attributs.
//! - [`card`] — Befehlskarten ([`card::command_card`]) und Flaggschiff-Karten.
//!
//! Das HUD des Gefechts:
//! - [`hand`] — die Hand mit Kosten, Stapeln, Auswahl, Hover-Zoom, Einflug neu gezogener Karten.
//! - [`stations`] — Befehlspunkte, die fünf Stationen mit Ziel bzw. Spalte, „Auslösen“ und „Zug beenden“ mit
//!   Checkliste.
//! - [`hud`] — Kopfzeile (Runde, Gefechtsuhr, Hülle), Log, Banner.
//! - [`round_log`] — je Ereignis des Gefechts eine Log-Zeile.
//! - [`radio_subtitle`] — Untertitel des Bordfunks.
//! - [`view_toggle`], [`fullscreen`] — Icon-Knöpfe und Tasten V, F und Pfeiltasten.
//! - [`dialogs`] — Admiralswahl, Ergebnis und die Dialoge hinter den Icon-Knöpfen (Einstellungen, Regeln);
//!   Schnittstelle dort.
//! - [`cursor`] — der gewählte Mauszeiger in der gewählten Größe als Systemzeiger.
//!
//! Schnittstelle zum Controller: Die Widgets lesen [`Battle`](crate::game::Battle), [`Locale`] und [`HudState`] (was
//! der Controller festlegt: ob gehandelt werden darf, ob eine Wiedergabe läuft, welche Karte gewählt ist) und melden
//! Absichten als Messages ([`HandCardPressed`], [`StationPressed`], [`ActionPressed`]); was sie bedeuten, entscheidet
//! der Controller. Log und Banner füllen Controller und Wiedergabe mit [`AppendLog`], [`ClearLog`], [`ShowBanner`],
//! [`HideBanner`] und [`ShowRoundBanner`].

pub mod button;
pub mod card;
pub mod cursor;
pub mod dialogs;
pub mod fps;
pub mod fullscreen;
pub mod hand;
pub mod hud;
pub mod loading_screen;
pub mod radio_subtitle;
pub mod round_log;
pub mod stations;
pub mod theme;
pub mod tooltip;
pub mod view_toggle;

use bevy::{ecs::system::NonSendMarker, prelude::*, window::PrimaryWindow, winit::WINIT_WINDOWS};
use voidfleet_core::admirals::{AdmiralId, Station};

use crate::{
    assets::{AppState, LoadingPhase, Preload},
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

impl FromWorld for Fonts {
    fn from_world(world: &mut World) -> Self {
        preloaded(world, |server, preload| Self {
            body: preload.track(server.load("fonts/exo2-regular.ttf")),
            display: preload.track(server.load("fonts/rajdhani-600.ttf")),
            display_bold: preload.track(server.load("fonts/rajdhani-700.ttf")),
        })
    }
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
    pub settings: Handle<Image>,
    pub rules: Handle<Image>,
}

impl FromWorld for UiIcons {
    fn from_world(world: &mut World) -> Self {
        preloaded(world, |server, preload| {
            let mut icon = |name: &str| preload.track(server.load(format!("images/ui/{name}.png")));
            Self {
                log: icon("log"),
                fullscreen_enter: icon("fullscreen_enter"),
                fullscreen_exit: icon("fullscreen_exit"),
                view_toggle: icon("view_toggle"),
                crosshair: icon("crosshair"),
                check: icon("check"),
                crown: icon("crown"),
                settings: icon("settings"),
                rules: icon("rules"),
            }
        })
    }
}

/// Legt eine Ressource mit Assets an, auf die der Ladebildschirm wartet: `load` lädt über den [`AssetServer`] und
/// meldet die Handles bei [`Preload`] an.
fn preloaded<T>(world: &mut World, load: impl FnOnce(&AssetServer, &mut Preload) -> T) -> T {
    world.resource_scope(|world, mut preload: Mut<Preload>| {
        load(world.resource::<AssetServer>(), &mut preload)
    })
}

/// Die Aktionsknöpfe neben den Stationen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HudAction {
    /// „Auslösen“: die eingestellte Station jetzt.
    Trigger,
    /// „Zug beenden“ (während einer Wiedergabe „Überspringen“).
    EndTurn,
    Skip,
}

/// Was der Controller für das HUD festlegt; Hand und Stationen zeigen es zusammen mit dem Gefecht.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct HudState {
    /// Der Spieler darf handeln: Checkliste sichtbar, Karten und Stationen wählbar.
    pub planning: bool,
    pub playing_back: bool,
    /// Hand-Index der Karte, die auf ihr Ziel wartet.
    pub selected: Option<usize>,
}

/// Eine Karte der Hand (Hand-Index) wurde angeklickt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandCardPressed(pub usize);

/// Eine Station im Stationen-Feld wurde angeklickt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationPressed(pub Station);

/// Ein Aktionsknopf wurde angeklickt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionPressed(pub HudAction);

/// Zeilen ans Log anhängen.
#[derive(Message, Clone, Debug, Default)]
pub struct AppendLog(pub Vec<String>);

/// Das Log leeren (neues Gefecht).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct ClearLog;

/// Eine kurze Ansage oben in der Mitte; mit `admiral` das Porträt in seiner Farbe. Blendet sich selbst wieder aus.
#[derive(Message, Clone, Debug)]
pub struct ShowBanner {
    pub title: String,
    pub subtitle: Option<String>,
    pub admiral: Option<AdmiralId>,
}

/// Nimmt das Banner sofort weg (etwa beim Überspringen).
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct HideBanner;

/// Kündigt groß an, dass Runde `0` begonnen hat.
#[derive(Message, Clone, Copy, Debug)]
pub struct ShowRoundBanner(pub u32);

/// Die Leiste der Icon-Knöpfe oben rechts, von rechts nach links: Log, Vollbild, Ansicht, Einstellungen, Regeln.
#[derive(Component, Debug)]
pub struct IconBar;

/// Icon der App (`assets/app/`), für das Fenster unter Windows und Linux; unter macOS setzt [`set_dock_icon`] das Icon
/// im Dock.
#[derive(Resource)]
struct AppIcon(Handle<Image>);

impl FromWorld for AppIcon {
    fn from_world(world: &mut World) -> Self {
        preloaded(world, |server, preload| {
            Self(preload.track(server.load("app/icon-192.png")))
        })
    }
}

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<Fonts>()
            .init_resource::<UiIcons>()
            .init_resource::<AppIcon>()
            .add_message::<HandCardPressed>()
            .add_message::<StationPressed>()
            .add_message::<ActionPressed>()
            .add_message::<AppendLog>()
            .add_message::<ClearLog>()
            .add_message::<ShowBanner>()
            .add_message::<HideBanner>()
            .add_message::<ShowRoundBanner>()
            .add_plugins((
                loading_screen::LoadingScreenPlugin,
                theme::ThemePlugin,
                button::ButtonPlugin,
                tooltip::TooltipPlugin,
                card::CardPlugin,
                hand::HandPlugin,
                hud::HudPlugin,
                stations::StationsPlugin,
                radio_subtitle::RadioSubtitlePlugin,
                view_toggle::ViewTogglePlugin,
                fullscreen::FullscreenPlugin,
                cursor::CursorPlugin,
                dialogs::DialogsPlugin,
            ))
            .add_plugins(fps::FpsPlugin)
            .add_systems(OnEnter(AppState::Match), spawn_icon_bar)
            .add_systems(OnEnter(LoadingPhase::Rehearsal), set_window_icon);
        #[cfg(target_os = "macos")]
        app.add_systems(Startup, set_dock_icon);
    }
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
            dialogs::dialog_button(dialogs::Dialog::Settings, &icons, &locale),
            dialogs::dialog_button(dialogs::Dialog::Rules, &icons, &locale),
        ],
    ));
}

/// Setzt das Icon, sobald alle Dateien geladen sind (`winit` erlaubt das nur im Haupt-Thread).
fn set_window_icon(
    icon: Res<AppIcon>,
    images: Res<Assets<Image>>,
    window: Single<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    let image = images.get(&icon.0).expect("App-Icon geladen");
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

/// Zeigt das Icon der App im Dock. Ohne Bundle (`cargo run`) hätte das Dock sonst das generische Programm-Icon, und das
/// Fenster-Icon von `winit` wirkt unter macOS nicht. Das Bild steckt im Binary, damit es vor den Assets bereitsteht.
#[cfg(target_os = "macos")]
fn set_dock_icon(_main_thread: NonSendMarker) {
    use objc2::{AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSApplication, NSImage, NSImageScaling, NSImageView};
    use objc2_foundation::{NSData, NSPoint, NSRect};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let png = NSData::with_bytes(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/app/icon-512.png"
    )));
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &png) else {
        warn!("Dock-Icon nicht lesbar");
        return;
    };
    let tile = NSApplication::sharedApplication(mtm).dockTile();
    let view = NSImageView::initWithFrame(
        NSImageView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), tile.size()),
    );
    view.setImage(Some(&image));
    view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
    tile.setContentView(Some(&view));
    tile.display();
}
