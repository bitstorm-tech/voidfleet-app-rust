//! Tooltips (Ersatz für das `title`-Attribut des Browsers): Eine Entity mit [`Tooltip`] zeigt ihren Text, sobald der
//! Zeiger eine Weile darüber ruht, in einem kleinen Feld neben dem Zeiger. Es gibt nur ein Feld; es liegt über allem.

use bevy::{picking::hover::Hovered, prelude::*, window::PrimaryWindow};

use super::{Fonts, theme};
use crate::palette;

/// Der Text, der beim Überfahren erscheint; leer zeigt nichts.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[require(Hovered)]
pub struct Tooltip(pub String);

/// So lange (Sekunden) muss der Zeiger ruhen, bis das Feld erscheint, und sein Abstand zum Zeiger.
const DELAY: f32 = 0.5;
const OFFSET: Vec2 = Vec2::new(12.0, 20.0);
const MAX_WIDTH: f32 = 300.0;

/// Das eine Feld.
#[derive(Component)]
struct TooltipBox;

#[derive(Component)]
struct TooltipText;

/// Welche Entity das Feld zuletzt meinte und seit wann.
#[derive(Resource, Default)]
struct TooltipHover {
    entity: Option<Entity>,
    since: f32,
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>) {
    commands.spawn((
        Name::new("Tooltip"),
        TooltipBox,
        Node {
            position_type: PositionType::Absolute,
            max_width: px(MAX_WIDTH),
            padding: UiRect::axes(px(8), px(5)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme::PANEL),
        BorderColor::all(Color::WHITE.with_alpha(0.12)),
        BoxShadow(vec![theme::drop(
            Color::BLACK.with_alpha(0.5),
            0.0,
            4.0,
            12.0,
        )]),
        GlobalZIndex(200),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![(
            TooltipText,
            theme::text(fonts.body(13.0), "", palette::TEXT),
            Pickable::IGNORE,
        )],
    ));
}

/// Zeigt den Tooltip der überfahrenen Entity nach der Wartezeit neben dem Zeiger.
fn follow_pointer(
    time: Res<Time>,
    mut hover: ResMut<TooltipHover>,
    tooltips: Query<(Entity, &Tooltip, &Hovered)>,
    window: Single<&Window, With<PrimaryWindow>>,
    tooltip_box: Single<(&mut Node, &mut Visibility, &ComputedNode), With<TooltipBox>>,
    mut text: Single<&mut Text, With<TooltipText>>,
) {
    // Der innerste überfahrene Tooltip gewinnt: Er ist der zuletzt entstandene unter den überfahrenen.
    let hovered = tooltips
        .iter()
        .filter(|(_, tooltip, hovered)| hovered.get() && !tooltip.0.is_empty())
        .max_by_key(|(entity, ..)| *entity);
    let now = time.elapsed_secs();
    let current = hovered.map(|(entity, ..)| entity);
    if current != hover.entity {
        *hover = TooltipHover {
            entity: current,
            since: now,
        };
    }
    let (mut node, mut visibility, computed) = tooltip_box.into_inner();
    let (Some((_, tooltip, _)), Some(cursor)) = (hovered, window.cursor_position()) else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    if now - hover.since < DELAY {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    }
    if text.0 != tooltip.0 {
        text.0.clone_from(&tooltip.0);
    }
    // Am rechten und unteren Rand klappt das Feld auf die andere Seite des Zeigers.
    let size = computed.size() * computed.inverse_scale_factor();
    let mut at = cursor + OFFSET;
    if at.x + size.x > window.width() {
        at.x = cursor.x - size.x - 4.0;
    }
    if at.y + size.y > window.height() {
        at.y = cursor.y - size.y - 8.0;
    }
    node.left = px(at.x.max(0.0));
    node.top = px(at.y.max(0.0));
    visibility.set_if_neq(Visibility::Inherited);
}

pub(super) struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipHover>()
            .add_systems(Startup, spawn)
            .add_systems(Update, follow_pointer);
    }
}
