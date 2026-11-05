//! Text über jedem Schiff, als Knoten der Oberfläche über seinen Balken: die Absicht (bei Kerr darunter kleiner die
//! zweite), die Zustände als kurze Textmarken und die Rechnung der Vorschau. Was dort steht, setzt
//! [`crate::scene::sync`] in [`ShipCaption`]; die erzählte Wiedergabe macht daraus später Icons.

use bevy::{
    picking::Pickable,
    prelude::*,
    transform::TransformSystems,
    ui::{UiSystems, UiTransform},
};

use super::{
    camera::MainCamera,
    ship::Ship,
    status::{ShipParts, ShipStatus},
};
use crate::{palette, ui::Fonts};

/// Abstand über der Mitte der Balken (Welteinheiten).
const LIFT: f32 = 0.7;

/// Was über einem Schiff steht; leere Zeilen fallen weg.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ShipCaption {
    pub intent: String,
    /// Die Absicht danach (Uhrwerk).
    pub next: String,
    pub conditions: String,
    /// Die Rechnung der Vorschau am Ziel unter dem Zeiger.
    pub preview: String,
}

/// Der Knoten der Beschriftung eines Schiffs.
#[derive(Component, Clone, Copy, Debug)]
struct CaptionOf(Entity);

/// Eine Zeile der Beschriftung.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    Intent,
    Next,
    Conditions,
    Preview,
}

impl Line {
    const ALL: [Self; 4] = [Self::Preview, Self::Intent, Self::Next, Self::Conditions];

    fn text(self, caption: &ShipCaption) -> &str {
        match self {
            Self::Intent => &caption.intent,
            Self::Next => &caption.next,
            Self::Conditions => &caption.conditions,
            Self::Preview => &caption.preview,
        }
    }

    fn style(self, fonts: &Fonts) -> (TextFont, Color) {
        match self {
            Self::Intent => (fonts.display_bold(19.0), palette::ENEMY),
            Self::Next => (fonts.display(14.0), palette::ENEMY.with_alpha(0.65)),
            Self::Conditions => (fonts.body(13.0), palette::BUFF),
            Self::Preview => (fonts.display_bold(17.0), palette::DAMAGE),
        }
    }
}

pub(super) struct LabelsPlugin;

impl Plugin for LabelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_caption)
            .add_systems(Update, update_text)
            .add_systems(
                PostUpdate,
                // Nach den Lagen dieses Bilds und vor dem UI-Layout: Die Beschriftung steht im selben Bild dort.
                follow_ships
                    .after(TransformSystems::Propagate)
                    .before(UiSystems::Layout),
            );
    }
}

fn spawn_caption(add: On<Add<ShipCaption>>, fonts: Res<Fonts>, mut commands: Commands) {
    commands
        .spawn((
            Name::new("Beschriftung"),
            CaptionOf(add.entity),
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            UiTransform {
                translation: Val2::percent(-50.0, -100.0),
                ..default()
            },
            GlobalZIndex(-5),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|caption| {
            for line in Line::ALL {
                let (font, color) = line.style(&fonts);
                caption.spawn((
                    line,
                    Text::default(),
                    font,
                    TextColor(color),
                    TextShadow {
                        offset: Vec2::new(0.0, 1.0),
                        color: Color::BLACK.with_alpha(0.9),
                    },
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn update_text(
    captions: Query<&ShipCaption>,
    nodes: Query<(&CaptionOf, &Children)>,
    mut lines: Query<(&Line, &mut Text, &mut Node)>,
) {
    for (caption_of, children) in &nodes {
        let Ok(caption) = captions.get(caption_of.0) else {
            continue;
        };
        for &child in children {
            if let Ok((line, mut text, mut node)) = lines.get_mut(child) {
                let content = line.text(caption);
                if text.0 != content {
                    text.0 = content.to_owned();
                }
                let display = if content.is_empty() {
                    Display::None
                } else {
                    Display::Flex
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
    }
}

/// Stellt jede Beschriftung über die Balken ihres Schiffs; ist das Schiff fort, geht sie mit.
fn follow_ships(
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ships: Query<(&ShipParts, &ShipStatus, &ShipCaption), With<Ship>>,
    overlays: Query<&GlobalTransform>,
    mut nodes: Query<(Entity, &CaptionOf, &mut Node, &mut Visibility)>,
) {
    let (camera, camera_transform) = *camera;
    for (entity, caption_of, mut node, mut visibility) in &mut nodes {
        let Ok((parts, status, caption)) = ships.get(caption_of.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let position = overlays
            .get(parts.overlay)
            .ok()
            .map(|overlay| overlay.translation() + Vec3::Y * LIFT)
            .and_then(|world| camera.world_to_viewport(camera_transform, world).ok());
        let empty = *caption == ShipCaption::default();
        let shown = position.is_some() && !status.hidden && !empty;
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(position) = position {
            node.left = Val::Px(position.x);
            node.top = Val::Px(position.y);
        }
    }
}
