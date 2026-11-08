//! Einstellungen (Port von `ui/settingsDialog.ts`): Zahnrad in der Icon-Leiste, ein Pult mit einer Tastenreihe oben,
//! die eines von vier eingelassenen Feldern wählt: „Spiel“ mit je einer Geschwindigkeitswahl für Angriffe,
//! Schiffsbewegungen und die übrigen Animationen, dem Mauszeiger als Raster aus Vorschaubildern samt Regler für seine
//! Größe und der Sprache; „Video“ mit dem Fenstermodus (Fenster, maximiert, Vollbild), VSync und der FPS-Anzeige;
//! „Sound“ mit den Reglern für Musik, Effekte und Bordfunk; „Dev“ mit dem Weg in die Modellschau
//! ([`crate::dev::viewer`]). Jede Wahl schreibt direkt in [`Settings`] und gilt sofort; die Tasten folgen den
//! Einstellungen, auch wenn sie anderswo umgeschaltet werden (Vollbild per F). Die Felder liegen übereinander, damit der
//! Dialog beim Wechsel der Tabs seine Größe behält.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{
        Activate, Button, Slider, SliderDragState, SliderRange, SliderStep, SliderThumb,
        SliderValue, TrackClick, ValueChange, observe,
    },
};

use super::{
    Dialog, close_dialogs, dialog_console, footer_close, icon_dialog, run_nodes::AbandonPrompt,
};
use crate::{
    assets::AppState,
    audio::{PlayUiSound, UiSound},
    i18n::{LANGUAGES, Locale, SettingsTab},
    palette,
    run::{ActiveRun, RunView},
    settings::{AnimationSpeed, CursorId, Language, Settings, WindowMode},
    ui::{
        Fonts,
        button::{self, Key},
        cursor::{self, CURSORS, CursorImages},
        theme::{self, rgb, rgba},
        tooltip::Tooltip,
    },
};

/// Der gewählte Tab; bleibt über das Schließen hinaus.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ui) struct ActiveTab(pub SettingsTab);

impl Default for ActiveTab {
    fn default() -> Self {
        Self(SettingsTab::Game)
    }
}

/// Die drei Geschwindigkeitsregler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Speed {
    Attack,
    Movement,
    Animation,
}

impl Speed {
    const ALL: [Self; 3] = [Self::Attack, Self::Movement, Self::Animation];

    fn get(self, settings: &Settings) -> AnimationSpeed {
        match self {
            Self::Attack => settings.attack_speed,
            Self::Movement => settings.movement_speed,
            Self::Animation => settings.animation_speed,
        }
    }

    fn get_mut(self, settings: &mut Settings) -> &mut AnimationSpeed {
        match self {
            Self::Attack => &mut settings.attack_speed,
            Self::Movement => &mut settings.movement_speed,
            Self::Animation => &mut settings.animation_speed,
        }
    }

    fn label(self, locale: &Locale) -> &'static str {
        match self {
            Self::Attack => locale.settings_attack_speed(),
            Self::Movement => locale.settings_movement_speed(),
            Self::Animation => locale.settings_animation_speed(),
        }
    }
}

const SPEEDS: [AnimationSpeed; 4] = [
    AnimationSpeed::Slow,
    AnimationSpeed::Normal,
    AnimationSpeed::Fast,
    AnimationSpeed::VeryFast,
];

const WINDOW_MODES: [WindowMode; 3] = [
    WindowMode::Windowed,
    WindowMode::Maximized,
    WindowMode::Fullscreen,
];

/// Eine Taste einer Auswahl: was sie wählt.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Button, Hovered, KeyDepth(REST))]
enum Choice {
    Tab(SettingsTab),
    Speed(Speed, AnimationSpeed),
    Cursor(CursorId),
    Language(Language),
    WindowMode(WindowMode),
    ShowFps(bool),
    Vsync(bool),
}

impl Choice {
    fn is_selected(self, settings: &Settings, locale: &Locale, tab: ActiveTab) -> bool {
        match self {
            Self::Tab(choice) => tab.0 == choice,
            Self::Speed(speed, choice) => speed.get(settings) == choice,
            Self::Cursor(cursor) => settings.cursor == cursor,
            Self::Language(language) => locale.language == language,
            Self::WindowMode(mode) => settings.window_mode == mode,
            Self::ShowFps(show) => settings.show_fps == show,
            Self::Vsync(on) => settings.vsync == on,
        }
    }

    fn apply(self, settings: &mut Settings, tab: &mut ActiveTab) {
        match self {
            Self::Tab(choice) => tab.0 = choice,
            Self::Speed(speed, choice) => *speed.get_mut(settings) = choice,
            Self::Cursor(cursor) => settings.cursor = cursor,
            Self::Language(language) => settings.language = Some(language),
            Self::WindowMode(mode) => settings.window_mode = mode,
            Self::ShowFps(show) => settings.show_fps = show,
            Self::Vsync(on) => settings.vsync = on,
        }
    }
}

/// Eines der drei Felder.
#[derive(Component, Clone, Copy, Debug)]
struct TabPanel(SettingsTab);

/// Die Lampe oben in einer Taste der Auswahl.
#[derive(Component)]
struct Lamp;

/// Die Überschrift des Mauszeiger-Rasters mit dem Namen des gewählten.
#[derive(Component)]
struct CursorLegend;

/// Was ein Regler einstellt: die drei Lautstärken und die Größe des Mauszeigers.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Music,
    Effects,
    Voice,
    CursorSize,
}

impl Level {
    const VOLUMES: [Self; 3] = [Self::Music, Self::Effects, Self::Voice];

    fn get(self, settings: &Settings) -> f32 {
        match self {
            Self::Music => settings.music_volume,
            Self::Effects => settings.effects_volume,
            Self::Voice => settings.voice_volume,
            Self::CursorSize => settings.cursor_size,
        }
    }

    fn get_mut(self, settings: &mut Settings) -> &mut f32 {
        match self {
            Self::Music => &mut settings.music_volume,
            Self::Effects => &mut settings.effects_volume,
            Self::Voice => &mut settings.voice_volume,
            Self::CursorSize => &mut settings.cursor_size,
        }
    }

    fn label(self, locale: &Locale) -> &'static str {
        match self {
            Self::Music => locale.settings_music_volume(),
            Self::Effects => locale.settings_effects_volume(),
            Self::Voice => locale.settings_voice_volume(),
            Self::CursorSize => locale.settings_cursor_size(),
        }
    }

    /// Kleinster und größter Wert und die Schrittweite, auf der der Regler einrastet: Lautstärken in 5 %, die
    /// Zeigergröße in logischen Pixeln.
    const fn range(self) -> (f32, f32, f32) {
        match self {
            Self::Music | Self::Effects | Self::Voice => (0.0, 1.0, 0.05),
            Self::CursorSize => (cursor::MIN_SIZE, cursor::MAX_SIZE, cursor::SIZE_STEP),
        }
    }

    /// Der Wert auf den nächsten Schritt gerastet.
    fn snap(self, value: f32) -> f32 {
        let (min, max, step) = self.range();
        (min + ((value - min) / step).round() * step).clamp(min, max)
    }

    /// Stand des Griffs zwischen 0 (links) und 1 (rechts).
    fn fraction(self, value: f32) -> f32 {
        let (min, max, _) = self.range();
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }

    /// Die Anzeige neben der Überschrift: Prozent bzw. Pixel.
    fn readout(self, value: f32) -> String {
        match self {
            Self::Music | Self::Effects | Self::Voice => format!("{}", (value * 100.0).round()),
            Self::CursorSize => format!("{}", value.round()),
        }
    }
}

/// Die leuchtende Füllung eines Reglers bis zum Griff.
#[derive(Component)]
struct LevelFill;

/// Die Flanke einer Taste der Auswahl in Pixeln (`--depth`): Ruhe 3, Hover 4, gedrückt oder gewählt 1.
#[derive(Component, Clone, Copy, Debug)]
struct KeyDepth(f32);

const REST: f32 = 3.0;
/// Oberseite und Flanke der Tasten der Auswahl.
const FACE: Color = rgb(46, 53, 68);
const EDGE: Color = rgb(18, 21, 28);

/// Oberseite einer gewählten Taste: 22 % der eigenen Farbe in dunklem Blech.
fn checked_face() -> Color {
    Color::from(
        rgb(30, 36, 48)
            .to_srgba()
            .mix(&palette::PLAYER.to_srgba(), 0.22),
    )
}

/// Wölbung bzw. eingerastete Mulde über der Oberseite.
fn key_gradient(checked: bool) -> BackgroundGradient {
    if checked {
        theme::vertical_gradient(&[
            (Color::BLACK.with_alpha(0.18), 0.0),
            (Color::WHITE.with_alpha(0.03), 60.0),
        ])
    } else {
        theme::vertical_gradient(&[
            (Color::WHITE.with_alpha(0.14), 0.0),
            (Color::WHITE.with_alpha(0.03), 46.0),
            (Color::BLACK.with_alpha(0.04), 54.0),
            (Color::BLACK.with_alpha(0.22), 100.0),
        ])
    }
}

/// Eine Taste der Auswahl; `node` legt Größe und Innenabstand fest.
fn choice_key(choice: Choice, node: Node) -> impl Bundle {
    (
        choice,
        Node {
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..node
        },
        BackgroundColor(FACE),
        key_gradient(false),
        BorderColor::all(Color::BLACK.with_alpha(0.65)),
        BoxShadow::default(),
        observe(press_choice),
    )
}

/// Eine Taste mit Lampe und Beschriftung.
fn labeled_key(fonts: &Fonts, choice: Choice, label: &str, size: f32) -> impl Bundle + use<> {
    (
        choice_key(
            choice,
            Node {
                flex_grow: 1.0,
                flex_basis: Val::ZERO,
                padding: UiRect::new(px(4), px(4), px(11), px(7)),
                ..default()
            },
        ),
        children![
            (
                Lamp,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(4),
                    width: px(18),
                    height: px(3),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(rgb(6, 8, 11)),
                BoxShadow::default(),
                Pickable::IGNORE,
            ),
            (
                theme::text(fonts.body(size), label.to_owned(), palette::TEXT_DIM),
                theme::text_shadow(0.55),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Überschrift einer Auswahl (`legend`).
fn legend(fonts: &Fonts, label: &str, size: f32) -> impl Bundle + use<> {
    (
        Node {
            margin: UiRect::bottom(px(10)),
            ..default()
        },
        theme::caps(
            fonts.display_bold(size),
            label,
            theme::TRACKING,
            palette::TEXT_DIM,
        ),
        Pickable::IGNORE,
    )
}

/// Eine Reihe von Tasten, eine je Option (`.settings-choice`).
fn key_row(fonts: &Fonts, options: Vec<(Choice, String)>, size: f32) -> impl Bundle + use<> {
    let keys: Vec<_> = options
        .into_iter()
        .map(|(choice, label)| labeled_key(fonts, choice, &label, size))
        .collect();
    (
        Node {
            column_gap: px(6),
            margin: UiRect::bottom(px(18)),
            ..default()
        },
        Children::spawn(SpawnIter(keys.into_iter())),
    )
}

/// Eine Auswahl mit Überschrift.
fn choice(fonts: &Fonts, label: &str, options: Vec<(Choice, String)>) -> impl Bundle + use<> {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![legend(fonts, label, 15.0), key_row(fonts, options, 14.0)],
    )
}

/// Der Mauszeiger als Raster aus Vorschaubildern; die Überschrift nennt den gewählten.
fn cursor_grid(
    fonts: &Fonts,
    locale: &Locale,
    images: &CursorImages,
    current: CursorId,
) -> impl Bundle {
    let tiles: Vec<_> = CURSORS
        .into_iter()
        .map(|cursor| {
            (
                choice_key(
                    Choice::Cursor(cursor),
                    Node {
                        aspect_ratio: Some(1.4),
                        padding: UiRect::all(px(7)),
                        ..default()
                    },
                ),
                Tooltip(locale.cursor(cursor).into()),
                children![(
                    // Feste Größe: Sonst nähme die Kachel die Pixelgröße des Bildes als Mindestbreite.
                    Node {
                        width: px(cursor::PREVIEW_SIZE),
                        height: px(cursor::PREVIEW_SIZE),
                        ..default()
                    },
                    ImageNode::new(images.get(cursor)),
                    Pickable::IGNORE,
                )],
            )
        })
        .collect();
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                CursorLegend,
                legend(fonts, &cursor_legend(locale, current), 14.0)
            ),
            (
                Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::fr(5, 1.0),
                    row_gap: px(8),
                    column_gap: px(6),
                    margin: UiRect::bottom(px(18)),
                    ..default()
                },
                Children::spawn(SpawnIter(tiles.into_iter())),
            ),
        ],
    )
}

fn cursor_legend(locale: &Locale, cursor: CursorId) -> String {
    format!("{}: {}", locale.settings_cursor(), locale.cursor(cursor)).to_uppercase()
}

/// Ein Regler: Schiene mit Leuchtsegmenten bis zum Griff, darüber die Überschrift und die Anzeige des Werts.
fn level_slider(fonts: &Fonts, locale: &Locale, level: Level, value: f32) -> impl Bundle {
    let (min, max, step) = level.range();
    let fraction = level.fraction(value);
    (
        Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::fr(1.0), GridTrack::auto()],
            align_items: AlignItems::Center,
            row_gap: px(6),
            column_gap: px(10),
            margin: UiRect::bottom(px(16)),
            ..default()
        },
        children![
            (
                theme::caps(
                    fonts.display_bold(15.0),
                    level.label(locale),
                    theme::TRACKING,
                    palette::TEXT_DIM
                ),
                Pickable::IGNORE,
            ),
            (
                Node {
                    min_width: px(42),
                    padding: UiRect::axes(px(6), px(1)),
                    justify_content: JustifyContent::FlexEnd,
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(theme::INK),
                children![(
                    level,
                    theme::text(
                        fonts.display_bold(15.0),
                        level.readout(value),
                        palette::SELECT
                    ),
                    TextShadow {
                        offset: Vec2::ZERO,
                        color: palette::SELECT.with_alpha(0.5),
                    },
                    Pickable::IGNORE,
                )],
            ),
            (
                level,
                Slider {
                    track_click: TrackClick::Snap,
                    ..default()
                },
                SliderValue(value),
                SliderRange::new(min, max),
                SliderStep(step),
                Hovered::default(),
                Node {
                    grid_column: GridPlacement::span(2),
                    height: px(24),
                    ..default()
                },
                observe(change_level),
                children![
                    // Schiene.
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::ZERO,
                            right: Val::ZERO,
                            top: px(7),
                            height: px(10),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(3)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(rgb(5, 7, 10)),
                        BorderColor::all(Color::BLACK),
                        BoxShadow(vec![theme::glow(palette::SELECT.with_alpha(0.15), 10.0)]),
                        Pickable::IGNORE,
                        children![
                            (
                                LevelFill,
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::ZERO,
                                    top: Val::ZERO,
                                    bottom: Val::ZERO,
                                    width: percent(fraction * 100.0),
                                    ..default()
                                },
                                theme::horizontal_gradient(&[
                                    (palette::PLAYER, 0.0),
                                    (palette::SELECT, 100.0)
                                ]),
                                Pickable::IGNORE,
                            ),
                            // Fugen zwischen den Leuchtsegmenten.
                            (
                                Node {
                                    column_gap: px(7),
                                    padding: UiRect::left(px(7)),
                                    ..theme::overlay()
                                },
                                Pickable::IGNORE,
                                Children::spawn(SpawnIter((0..60).map(|_| {
                                    (
                                        Node {
                                            width: px(2),
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                        BackgroundColor(rgba(4, 6, 9, 0.85)),
                                        Pickable::IGNORE,
                                    )
                                }))),
                            ),
                        ],
                    ),
                    // Weg des Griffs: so breit wie die Schiene ohne den Griff.
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::ZERO,
                            right: px(20),
                            top: Val::ZERO,
                            bottom: Val::ZERO,
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![thumb(fraction)],
                    ),
                ],
            ),
        ],
    )
}

/// Der Griff wie ein Schieber am Mischpult: gewölbt, mit Riffelung.
fn thumb(fraction: f32) -> impl Bundle {
    (
        SliderThumb,
        Node {
            position_type: PositionType::Absolute,
            left: percent(fraction * 100.0),
            top: Val::ZERO,
            width: px(20),
            height: px(22),
            column_gap: px(2),
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        theme::vertical_gradient(&[
            (rgb(168, 178, 194), 0.0),
            (rgb(98, 108, 124), 48.0),
            (rgb(72, 80, 94), 52.0),
            (rgb(110, 120, 136), 100.0),
        ]),
        BorderColor::all(Color::BLACK.with_alpha(0.75)),
        BoxShadow::default(),
        Pickable::IGNORE,
        Children::spawn(SpawnIter((0..3).map(|_| {
            (
                Node {
                    width: px(1),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::BLACK.with_alpha(0.45)),
                Pickable::IGNORE,
            )
        }))),
    )
}

pub(super) fn spawn(
    mut commands: Commands,
    fonts: Res<Fonts>,
    locale: Res<Locale>,
    settings: Res<Settings>,
    cursors: Res<CursorImages>,
    run: Option<Res<ActiveRun>>,
) {
    let fonts = &*fonts;
    let locale = &*locale;
    let options = |choices: Vec<(Choice, &str)>| -> Vec<(Choice, String)> {
        choices
            .into_iter()
            .map(|(choice, label)| (choice, label.to_owned()))
            .collect()
    };
    let speeds: Vec<_> = Speed::ALL
        .into_iter()
        .map(|speed| {
            choice(
                fonts,
                speed.label(locale),
                options(
                    SPEEDS
                        .into_iter()
                        .map(|value| (Choice::Speed(speed, value), locale.speed(value)))
                        .collect(),
                ),
            )
        })
        .collect();
    let languages = LANGUAGES
        .into_iter()
        .map(|language| {
            (
                Choice::Language(language),
                Locale::new(language).language_name(),
            )
        })
        .collect();
    let window_modes = WINDOW_MODES
        .into_iter()
        .map(|mode| (Choice::WindowMode(mode), locale.window_mode(mode)))
        .collect();
    let volumes: Vec<_> = Level::VOLUMES
        .into_iter()
        .map(|level| level_slider(fonts, locale, level, level.get(&settings)))
        .collect();
    let tabs = SettingsTab::ALL
        .into_iter()
        .map(|tab| (Choice::Tab(tab), locale.settings_tab(tab)))
        .collect();
    let panel = |tab: SettingsTab| {
        (
            TabPanel(tab),
            theme::console_inset(Node {
                grid_row: GridPlacement::start(1),
                grid_column: GridPlacement::start(1),
                ..default()
            }),
        )
    };
    commands.spawn(icon_dialog(
        Dialog::Settings,
        dialog_console(
            fonts,
            locale.settings_title(),
            Node::default(),
            true,
            (
                Spawn((
                    Node {
                        width: px(420),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    children![
                        key_row(fonts, options(tabs), 15.0),
                        (
                            Node {
                                display: Display::Grid,
                                grid_template_columns: RepeatedGridTrack::fr(1, 1.0),
                                ..default()
                            },
                            children![
                                (
                                    panel(SettingsTab::Game),
                                    Children::spawn((
                                        SpawnIter(speeds.into_iter()),
                                        Spawn(cursor_grid(
                                            fonts,
                                            locale,
                                            &cursors,
                                            settings.cursor
                                        )),
                                        Spawn(level_slider(
                                            fonts,
                                            locale,
                                            Level::CursorSize,
                                            settings.cursor_size
                                        )),
                                        Spawn(choice(
                                            fonts,
                                            locale.settings_language(),
                                            options(languages)
                                        )),
                                    )),
                                ),
                                (
                                    panel(SettingsTab::Video),
                                    children![
                                        choice(
                                            fonts,
                                            locale.settings_window_mode(),
                                            options(window_modes)
                                        ),
                                        choice(
                                            fonts,
                                            "VSync",
                                            options(
                                                [false, true]
                                                    .map(|on| (
                                                        Choice::Vsync(on),
                                                        locale.on_off(on)
                                                    ))
                                                    .into()
                                            )
                                        ),
                                        choice(
                                            fonts,
                                            locale.settings_show_fps(),
                                            options(
                                                [false, true]
                                                    .map(|show| (
                                                        Choice::ShowFps(show),
                                                        locale.on_off(show)
                                                    ))
                                                    .into()
                                            )
                                        ),
                                    ],
                                ),
                                (
                                    panel(SettingsTab::Sound),
                                    Children::spawn(SpawnIter(volumes.into_iter())),
                                ),
                                (
                                    panel(SettingsTab::Dev),
                                    children![models_button(fonts, locale)]
                                ),
                            ],
                        ),
                    ],
                )),
                Spawn((
                    Node {
                        margin: UiRect::top(px(22)),
                        column_gap: px(12),
                        justify_content: JustifyContent::FlexEnd,
                        ..default()
                    },
                    Children::spawn((
                        SpawnIter(
                            run.is_some()
                                .then(|| abandon_button(fonts, locale))
                                .into_iter(),
                        ),
                        Spawn(footer_close(fonts, locale, 0.0)),
                    )),
                )),
            ),
        ),
    ));
}

/// „Run abbrechen“: schließt die Einstellungen und fragt nach.
fn abandon_button(fonts: &Fonts, locale: &Locale) -> impl Bundle {
    (
        button::text_button(
            fonts.display_bold(18.0),
            locale.run_abandon(),
            Key::DANGER,
            Node {
                padding: UiRect::axes(px(22), px(9)),
                ..default()
            },
        ),
        observe(|_: On<Activate>, mut commands: Commands| {
            commands.insert_resource(AbandonPrompt);
        }),
        observe(close_dialogs),
    )
}

/// „3D-Modelle“: schließt die Einstellungen und öffnet die Modellschau.
fn models_button(fonts: &Fonts, locale: &Locale) -> impl Bundle {
    (
        button::text_button(
            fonts.display_bold(18.0),
            locale.settings_models(),
            Key::PRIMARY,
            Node {
                padding: UiRect::axes(px(22), px(9)),
                ..default()
            },
        ),
        observe(|_: On<Activate>, mut view: ResMut<RunView>| view.models = true),
        observe(close_dialogs),
    )
}

/// Eine Taste der Auswahl schreibt ihre Wahl.
fn press_choice(
    activate: On<Activate>,
    choices: Query<&Choice>,
    mut settings: ResMut<Settings>,
    mut tab: ResMut<ActiveTab>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    if let Ok(&choice) = choices.get(activate.entity) {
        choice.apply(&mut settings, &mut tab);
        sounds.write(PlayUiSound(UiSound::Click));
    }
}

/// Der Regler schreibt seinen Wert und rastet auf dem nächsten Schritt ein.
fn change_level(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    levels: Query<&Level>,
    mut settings: ResMut<Settings>,
    mut sounds: MessageWriter<PlayUiSound>,
) {
    let Ok(&level) = levels.get(change.source) else {
        return;
    };
    let value = level.snap(change.value);
    commands.entity(change.source).insert(SliderValue(value));
    if level.get(&settings) != value {
        *level.get_mut(&mut settings) = value;
    }
    if change.is_final {
        sounds.write(PlayUiSound(UiSound::Click));
    }
}

/// Füllung, Griff und Anzeige folgen dem Wert des Reglers.
fn show_level(
    sliders: Query<(Entity, &Level, &SliderValue), Changed<SliderValue>>,
    children: Query<&Children>,
    mut thumbs: Query<&mut Node, With<SliderThumb>>,
    mut fills: Query<&mut Node, (With<LevelFill>, Without<SliderThumb>)>,
    mut readouts: Query<(&Level, &mut Text)>,
) {
    for (slider, &level, &SliderValue(value)) in &sliders {
        let fraction = level.fraction(value);
        for entity in children.iter_descendants(slider) {
            if let Ok(mut thumb) = thumbs.get_mut(entity) {
                thumb.left = percent(fraction * 100.0);
            }
            if let Ok(mut fill) = fills.get_mut(entity) {
                fill.width = percent(fraction * 100.0);
            }
        }
        for (&readout, mut text) in &mut readouts {
            if readout == level {
                text.0 = level.readout(value);
            }
        }
    }
}

/// Die Felder nach dem gewählten Tab und die Überschrift des Mauszeigers; läuft, wenn sich Tab, Einstellungen oder
/// Sprache ändern oder der Dialog neu entsteht.
fn show_tab(
    tab: Res<ActiveTab>,
    settings: Res<Settings>,
    locale: Res<Locale>,
    mut panels: Query<(&TabPanel, &mut Visibility)>,
    mut legend: Query<&mut Text, With<CursorLegend>>,
) {
    for (&TabPanel(panel), mut visibility) in &mut panels {
        visibility.set_if_neq(if panel == tab.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for mut text in &mut legend {
        let label = cursor_legend(&locale, settings.cursor);
        if text.0 != label {
            text.0 = label;
        }
    }
}

/// Wie schnell die Tasten ihrem Ziel folgen (1/s), etwa die 0,08 s der CSS-Transition.
const FOLLOW: f32 = 40.0;

type ChoiceLook<'a> = (
    &'a Choice,
    &'a Hovered,
    Has<Pressed>,
    &'a Children,
    &'a mut KeyDepth,
    &'a mut BackgroundColor,
    &'a mut BackgroundGradient,
    &'a mut BoxShadow,
    &'a mut UiTransform,
);

/// Hover hebt die Taste, Drücken und Wählen senken sie; die gewählte hat eine leuchtende Lampe und helle Schrift.
fn animate_choices(
    time: Res<Time>,
    settings: Res<Settings>,
    locale: Res<Locale>,
    tab: Res<ActiveTab>,
    mut keys: Query<ChoiceLook, Without<Lamp>>,
    mut lamps: Query<(&mut BackgroundColor, &mut BoxShadow), With<Lamp>>,
    mut labels: Query<&mut TextColor>,
) {
    for (
        &choice,
        hovered,
        pressed,
        children,
        mut depth,
        mut face,
        mut gradient,
        mut shadow,
        mut transform,
    ) in &mut keys
    {
        let checked = choice.is_selected(&settings, &locale, *tab);
        let hovered = hovered.get();
        let target = if checked || pressed {
            1.0
        } else if hovered {
            REST + 1.0
        } else {
            REST
        };
        depth.0.smooth_nudge(&target, FOLLOW, time.delta_secs());
        if (depth.0 - target).abs() < 0.01 {
            depth.0 = target;
        }
        let bright = if hovered && !checked { 1.15 } else { 1.0 };
        let base = if checked { checked_face() } else { FACE };
        face.set_if_neq(BackgroundColor(
            Color::from(base.to_srgba() * bright).with_alpha(1.0),
        ));
        let wanted = key_gradient(checked);
        if *gradient != wanted {
            *gradient = wanted;
        }
        let mut shadows = Vec::new();
        if checked {
            shadows.push(theme::glow(palette::PLAYER.with_alpha(0.25), 12.0));
        }
        shadows.push(theme::drop(
            Color::BLACK.with_alpha(0.5),
            0.0,
            depth.0 + 2.0,
            6.0,
        ));
        shadows.push(theme::drop(EDGE, 0.0, depth.0, 0.0));
        if shadow.0 != shadows {
            shadow.0 = shadows;
        }
        let translation = Val2::px(0.0, REST - depth.0);
        if transform.translation != translation {
            transform.translation = translation;
        }
        for &child in children {
            if let Ok((mut lamp, mut glow)) = lamps.get_mut(child) {
                lamp.set_if_neq(BackgroundColor(if checked {
                    palette::SELECT
                } else {
                    rgb(6, 8, 11)
                }));
                let wanted = if checked {
                    vec![theme::glow(palette::SELECT, 8.0)]
                } else {
                    Vec::new()
                };
                if glow.0 != wanted {
                    glow.0 = wanted;
                }
            } else if let Ok(mut label) = labels.get_mut(child) {
                label.set_if_neq(TextColor(if checked {
                    palette::TEXT
                } else {
                    palette::TEXT_DIM
                }));
            }
        }
    }
}

/// Ein Regler, dessen Griff sich anders zeigen muss: neu, überfahren oder verlassen, Ziehen begonnen oder beendet,
/// gedrückt (das Loslassen meldet [`RemovedComponents`]).
type ThumbChanged = (
    With<Slider>,
    Or<(Changed<Hovered>, Changed<SliderDragState>, Added<Pressed>)>,
);

/// Der Griff hebt sich beim Überfahren, senkt sich beim Ziehen und leuchtet dabei.
fn animate_thumbs(
    changed: Query<Entity, ThumbChanged>,
    mut released: RemovedComponents<Pressed>,
    sliders: Query<(&Hovered, &SliderDragState, Has<Pressed>), With<Slider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut UiTransform, &mut BoxShadow), With<SliderThumb>>,
) {
    for slider in changed.iter().chain(released.read()) {
        let Ok((hovered, drag, pressed)) = sliders.get(slider) else {
            continue;
        };
        let active = drag.dragging || pressed;
        let (lift, depth, blur, glow) = if active {
            (2.0, 1.0, 4.0, Some((0.7, 18.0)))
        } else if hovered.get() {
            (-1.0, 4.0, 8.0, Some((0.5, 14.0)))
        } else {
            (0.0, 3.0, 7.0, None)
        };
        let mut shadows: Vec<_> = glow
            .map(|(alpha, blur)| theme::glow(palette::SELECT.with_alpha(alpha), blur))
            .into_iter()
            .collect();
        shadows.push(theme::drop(
            Color::BLACK.with_alpha(0.6),
            0.0,
            depth + 2.0,
            blur,
        ));
        shadows.push(theme::drop(rgb(30, 34, 42), 0.0, depth, 0.0));
        for entity in children.iter_descendants(slider) {
            if let Ok((mut transform, mut shadow)) = thumbs.get_mut(entity) {
                let translation = Val2::px(0.0, lift);
                if transform.translation != translation {
                    transform.translation = translation;
                }
                if shadow.0 != shadows {
                    shadow.0.clone_from(&shadows);
                }
            }
        }
    }
}

pub(super) struct SettingsDialogPlugin;

impl Plugin for SettingsDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveTab>().add_systems(
            Update,
            (
                show_tab.run_if(
                    resource_changed::<ActiveTab>
                        .or_eager(resource_changed::<Settings>)
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(any_match_filter::<Added<TabPanel>>),
                ),
                show_level,
                animate_choices,
                animate_thumbs,
            )
                .run_if(in_state(AppState::Match)),
        );
    }
}
