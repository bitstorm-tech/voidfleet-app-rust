//! Tooltips (Ersatz für das `title`-Attribut des Browsers): Eine Entity mit [`Tooltip`] zeigt ihren Text, sobald der
//! Zeiger eine Weile darüber ruht, in einem kleinen Feld neben dem Zeiger. Es gibt nur ein Feld; es liegt über allem.

use bevy::{
    picking::{hover::HoverMap, pointer::PointerId},
    prelude::*,
    window::PrimaryWindow,
};

use super::{Fonts, cursor, theme};
use crate::{palette, settings::Settings};

/// Der Text, der beim Überfahren erscheint; leer zeigt nichts.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct Tooltip(pub String);

/// So lange (Sekunden) muss der Zeiger ruhen, bis das Feld erscheint, und sein Abstand zum Bild des Zeigers.
const DELAY: f32 = 0.5;
const GAP: f32 = 4.0;
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
    hover_map: Res<HoverMap>,
    parents: Query<&ChildOf>,
    tooltips: Query<&Tooltip>,
    window: Single<&Window, With<PrimaryWindow>>,
    settings: Res<Settings>,
    tooltip_box: Single<(&mut Node, &mut Visibility, &ComputedNode), With<TooltipBox>>,
    mut text: Single<&mut Text, With<TooltipText>>,
) {
    // Der innerste Tooltip mit Text gewinnt: vom vordersten Treffer des Zeigers aus der erste nach außen.
    let hovered = hover_map
        .get(&PointerId::Mouse)
        .into_iter()
        .flatten()
        .filter_map(|(&hit, data)| {
            std::iter::once(hit)
                .chain(parents.iter_ancestors(hit))
                .find_map(|entity| {
                    tooltips
                        .get(entity)
                        .ok()
                        .filter(|tooltip| !tooltip.0.is_empty())
                        .map(|tooltip| (data.depth, entity, tooltip))
                })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let now = time.elapsed_secs();
    let current = hovered.map(|(_, entity, _)| entity);
    if current != hover.entity {
        *hover = TooltipHover {
            entity: current,
            since: now,
        };
    }
    let (mut node, mut visibility, computed) = tooltip_box.into_inner();
    let (Some((.., tooltip)), Some(cursor)) = (hovered, window.cursor_position()) else {
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
    // Das Feld steht unter dem Bild des Zeigers (die Größe ist wählbar), am unteren Rand darüber; am rechten Rand
    // rückt es nach links, ohne den Zeiger zu überdecken, da es senkrecht neben ihm steht.
    let size = computed.size() * computed.inverse_scale_factor();
    let (above, below) = cursor::reach(&settings);
    let mut at = Vec2::new(cursor.x, cursor.y + below.y + GAP);
    if at.y + size.y > window.height() {
        at.y = cursor.y + above.y - size.y - GAP;
    }
    at.x = at.x.min(window.width() - size.x);
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
