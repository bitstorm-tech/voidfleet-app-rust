//! Dialoge und Overlays (Port von `admiralSelect.ts`, `resultScreen.ts`, `iconDialog.ts`, `settingsDialog.ts` und
//! `rulesDialog.ts`).
//!
//! Zwei Arten:
//! - Overlays, die das Gefecht verlangt und die sich selbst aus [`Battle`](crate::game::Battle) ableiten: die
//!   Admiralswahl ([`admiral_select`]) solange kein Gefecht läuft, das Ergebnis ([`result_screen`]) sobald es
//!   entschieden ist.
//! - Dialoge hinter den Icon-Knöpfen oben rechts ([`Dialog`]): ein Steuerpult über abgedunkeltem Hintergrund, als
//!   [`Modal`] markiert. Sie öffnen per Knopf oder [`OpenDialog`]; „Schließen“ oder Escape schließt sie, ein Klick auf
//!   den Hintergrund nicht. Höchstens einer ist offen.
//!
//! Schnittstelle zum Controller:
//! - rein: [`OpenDialog`] (etwa Escape ohne Auswahl → Einstellungen).
//! - raus: [`AdmiralChosen`] (mit Bestätigungsklang; bis das Gefecht steht, bleibt die Wahl gesperrt), [`NextBattle`],
//!   [`NewRun`]. Die Einstellungen schreibt der Dialog direkt in [`Settings`](crate::settings::Settings).
//!
//! Escape schließt einen offenen Dialog in [`DialogSystems`]; wer Escape selbst auswertet, läuft davor und prüft, ob
//! ein [`Modal`] offen ist.

pub mod admiral_select;
pub mod result_screen;
pub mod rules;
pub mod settings_dialog;

use bevy::{
    ecs::spawn::SpawnableList,
    prelude::*,
    ui_widgets::{Activate, observe},
};
use voidfleet_core::admirals::AdmiralId;

use super::{
    Fonts, UiIcons,
    button::{self, Key},
    theme::{self, Modal, UiOpacity},
    tooltip::Tooltip,
};
use crate::{assets::AppState, i18n::Locale, palette};

/// Die Dialoge hinter den Icon-Knöpfen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dialog {
    Settings,
    Rules,
}

/// Öffnet einen Dialog (ein offener anderer schließt dabei).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenDialog(pub Dialog);

/// Der Spieler hat seinen Admiral gewählt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmiralChosen(pub AdmiralId);

/// „Weiter“ nach einem Sieg: das nächste Gefecht.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct NextBattle;

/// „Neuer Run“ nach einer Niederlage: zurück zur Admiralswahl.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct NewRun;

/// Hier schließt Escape einen offenen Dialog.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DialogSystems;

/// Ein offener Dialog hinter einem Icon-Knopf (die Wurzel über dem ganzen Bild).
#[derive(Component, Clone, Copy, Debug)]
struct IconDialog(Dialog);

/// Der Icon-Knopf eines Dialogs.
#[derive(Component, Clone, Copy, Debug)]
struct DialogButton(Dialog);

/// Ebenen über dem Brett: Overlays über dem HUD, das Ergebnis darüber, Dialoge über allem außer Tooltips.
const OVERLAY_LAYER: i32 = 4;
const RESULT_LAYER: i32 = 20;
const DIALOG_LAYER: i32 = 50;

impl Dialog {
    fn title(self, locale: &Locale) -> &'static str {
        match self {
            Self::Settings => locale.settings_title(),
            Self::Rules => locale.rules_title(),
        }
    }

    fn icon(self, icons: &UiIcons) -> Handle<Image> {
        match self {
            Self::Settings => icons.settings.clone(),
            Self::Rules => icons.rules.clone(),
        }
    }

    fn spawn(self, commands: &mut Commands) {
        match self {
            Self::Settings => commands.run_system_cached(settings_dialog::spawn),
            Self::Rules => commands.run_system_cached(rules::spawn),
        }
    }
}

/// Der Icon-Knopf eines Dialogs für die [`IconBar`](super::IconBar).
pub(super) fn dialog_button(dialog: Dialog, icons: &UiIcons, locale: &Locale) -> impl Bundle {
    (
        DialogButton(dialog),
        button::icon_button(dialog.icon(icons), dialog.title(locale)),
        observe(
            move |_: On<Activate>, mut open: MessageWriter<OpenDialog>| {
                open.write(OpenDialog(dialog));
            },
        ),
    )
}

/// Ein Dialog hinter einem Icon-Knopf: abgedunkelter Hintergrund (fängt Klicks, schließt aber nicht) und das Pult
/// darauf, das kurz aufspringt.
fn icon_dialog(dialog: Dialog, console: impl Bundle) -> impl Bundle {
    (
        Name::new("Dialog"),
        IconDialog(dialog),
        Modal,
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..theme::overlay()
        },
        GlobalZIndex(DIALOG_LAYER),
        children![
            (
                theme::overlay(),
                BackgroundGradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::FarthestCorner,
                    vec![
                        ColorStop::percent(palette::BACKGROUND.with_alpha(0.45), 0.0),
                        ColorStop::percent(palette::BACKGROUND.with_alpha(0.8), 100.0),
                    ],
                )),
                FadeIn::new(0.25),
            ),
            (console, PopIn::new(0.96, 0.32, EaseFunction::CubicOut)),
        ],
    )
}

/// Das Pult eines Dialogs: Kopfleiste mit dem Titel, Inhalt, darunter der Fuß (meist [`close_button`]).
fn dialog_console<C: SpawnableList<ChildOf> + Send + Sync + 'static>(
    fonts: &Fonts,
    title: &str,
    node: Node,
    vents: bool,
    content: C,
) -> impl Bundle {
    theme::console(
        Node {
            min_width: px(360),
            ..node
        },
        vents,
        (
            Spawn(theme::console_header(fonts.display_bold(28.0), title)),
            content,
        ),
    )
}

/// „Schließen“ in der Größe `node`.
fn close_button(fonts: &Fonts, locale: &Locale, node: Node) -> impl Bundle {
    (
        button::text_button(
            fonts.display_bold(18.0),
            locale.dialog_close(),
            Key::NORMAL,
            node,
        ),
        observe(close_dialogs),
    )
}

/// „Schließen“ unten rechts im Pult (`.dialog-close`), `top` unter dem Inhalt.
fn footer_close(fonts: &Fonts, locale: &Locale, top: f32) -> impl Bundle {
    close_button(
        fonts,
        locale,
        Node {
            align_self: AlignSelf::FlexEnd,
            margin: UiRect::top(px(top)),
            padding: UiRect::axes(px(26), px(9)),
            ..default()
        },
    )
}

fn close_dialogs(
    _: On<Activate>,
    mut commands: Commands,
    dialogs: Query<Entity, With<IconDialog>>,
) {
    for dialog in &dialogs {
        commands.entity(dialog).despawn();
    }
}

/// Öffnet den zuletzt verlangten Dialog; ist er schon offen, bleibt er es.
fn open_dialogs(
    mut commands: Commands,
    mut requests: MessageReader<OpenDialog>,
    open: Query<(Entity, &IconDialog)>,
) {
    let Some(&OpenDialog(dialog)) = requests.read().last() else {
        return;
    };
    if open.iter().any(|(_, open)| open.0 == dialog) {
        return;
    }
    for (entity, _) in &open {
        commands.entity(entity).despawn();
    }
    dialog.spawn(&mut commands);
}

fn close_on_escape(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    dialogs: Query<Entity, With<IconDialog>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for dialog in &dialogs {
            commands.entity(dialog).despawn();
        }
    }
}

/// Eine andere Sprache baut den offenen Dialog neu auf und beschriftet die Icon-Knöpfe neu.
fn follow_locale(
    mut commands: Commands,
    locale: Res<Locale>,
    dialogs: Query<(Entity, &IconDialog)>,
    mut buttons: Query<(&DialogButton, &mut Tooltip)>,
) {
    for (entity, &IconDialog(dialog)) in &dialogs {
        commands.entity(entity).despawn();
        dialog.spawn(&mut commands);
    }
    for (&DialogButton(dialog), mut tooltip) in &mut buttons {
        tooltip.0 = dialog.title(&locale).into();
    }
}

// --- Gemeinsames der Overlays über dem Brett ---

/// Wie viel Höhe das Fenster lässt (`@media (max-height: …)`): Die Admiralswahl muss zwischen Kopfzeile und Hand
/// passen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Density {
    Regular,
    /// Bis 900 Pixel hoch.
    Compact,
    /// Bis 820 Pixel hoch.
    Tight,
}

impl Density {
    /// Die Dichte nach der Höhe des Fensters.
    fn of(window: &Window) -> Self {
        match window.height() {
            height if height <= 820.0 => Self::Tight,
            height if height <= 900.0 => Self::Compact,
            _ => Self::Regular,
        }
    }
}

/// Ein Overlay über dem Brett zwischen Kopfzeile und Hand (`.admiral-overlay`); als erstes Kind
/// gehört [`overlay_backdrop`] hinein.
fn board_overlay() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            top: px(60),
            left: Val::ZERO,
            right: Val::ZERO,
            bottom: px(290),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(4),
            padding: UiRect::horizontal(px(24)),
            ..default()
        },
        GlobalZIndex(OVERLAY_LAYER),
    )
}

/// Der abgedunkelte Grund eines Overlays, der einblendet (`result-fade`). Eigener Knoten, damit das Einblenden die
/// Karten darüber nicht berührt: Die tragen ihre eigene [`UiOpacity`].
fn overlay_backdrop() -> impl Bundle {
    (
        theme::overlay(),
        BackgroundGradient::from(RadialGradient::new(
            UiPosition::CENTER,
            RadialGradientShape::FarthestSide,
            vec![
                ColorStop::percent(palette::BACKGROUND.with_alpha(0.82), 0.0),
                ColorStop::percent(palette::BACKGROUND.with_alpha(0.55), 75.0),
            ],
        )),
        FadeIn::new(0.4),
    )
}

/// Titel eines Overlays (`.admiral-title`) in der Akzentfarbe.
fn overlay_title(fonts: &Fonts, title: &str, density: Density) -> impl Bundle + use<> {
    let size = if density == Density::Regular {
        38.0
    } else {
        30.0
    };
    (
        theme::caps(fonts.display_bold(size), title, 0.04, theme::ACCENT),
        Pickable::IGNORE,
    )
}

/// Hinweis unter dem Titel (`.admiral-hint`).
fn overlay_hint(fonts: &Fonts, hint: &str, density: Density) -> impl Bundle + use<> {
    let size = if density == Density::Regular {
        17.0
    } else {
        15.0
    };
    (
        Node {
            max_width: px(980),
            margin: UiRect::bottom(px(8)),
            ..default()
        },
        theme::text(fonts.body(size), hint.to_owned(), palette::TEXT_DIM),
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
    )
}

// --- Einblenden ---

/// Springt von `from` auf volle Größe auf (`console-boot`, `result-pop`).
#[derive(Component, Clone, Debug)]
#[require(UiTransform)]
struct PopIn {
    from: f32,
    ease: EaseFunction,
    timer: Timer,
}

impl PopIn {
    fn new(from: f32, duration: f32, ease: EaseFunction) -> Self {
        Self {
            from,
            ease,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// Blendet einen Teilbaum ohne eigene Farbanimationen ein (`result-fade`).
#[derive(Component, Clone, Debug)]
#[require(UiOpacity(0.0))]
struct FadeIn(Timer);

impl FadeIn {
    fn new(duration: f32) -> Self {
        Self(Timer::from_seconds(duration, TimerMode::Once))
    }
}

fn pop_in(
    mut commands: Commands,
    time: Res<Time>,
    mut pops: Query<(Entity, &mut PopIn, &mut UiTransform)>,
    mut fades: Query<(Entity, &mut FadeIn, &mut UiOpacity)>,
) {
    for (entity, mut pop, mut transform) in &mut pops {
        let progress = pop.timer.tick(time.delta()).fraction();
        let scale = pop.from + (1.0 - pop.from) * pop.ease.sample_clamped(progress);
        transform.scale = Vec2::splat(scale);
        if pop.timer.is_finished() {
            commands.entity(entity).remove::<PopIn>();
        }
    }
    for (entity, mut fade, mut opacity) in &mut fades {
        let progress = fade.0.tick(time.delta()).fraction();
        opacity.0 = EaseFunction::QuadraticOut.sample_clamped(progress);
        if fade.0.is_finished() {
            commands.entity(entity).remove::<FadeIn>();
        }
    }
}

pub(super) struct DialogsPlugin;

impl Plugin for DialogsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenDialog>()
            .add_message::<AdmiralChosen>()
            .add_message::<NextBattle>()
            .add_message::<NewRun>()
            .add_plugins((
                admiral_select::AdmiralSelectPlugin,
                result_screen::ResultScreenPlugin,
                settings_dialog::SettingsDialogPlugin,
                rules::RulesPlugin,
            ))
            .add_systems(
                Update,
                (
                    (close_on_escape, open_dialogs)
                        .chain()
                        .in_set(DialogSystems),
                    follow_locale.run_if(resource_changed::<Locale>),
                    pop_in,
                )
                    .run_if(in_state(AppState::Match)),
            );
    }
}
