//! Das Material aller selbstleuchtenden Teile der Szene außer den Schiffen (Port der unbeleuchteten
//! `StandardMaterial`s aus Babylon): Effekte, Partikel, Marken am Boden, Statusbalken, Bögen, Pfeile und Pads.
//! Farbe mal Textur mal Vertexfarbe, ohne Licht (`assets/shaders/fx.wesl`).
//!
//! Glühen: Babylons GlowLayer gibt es nicht; was heller als 1 leuchtet, glüht im Bloom der Kamera. Wer glühen soll,
//! nimmt seine Farbe mal [`GLOW`] ([`glowing`]).
//!
//! Dazu die gemeinsamen Meshes und Sprite-Texturen ([`FxAssets`]) und Billboards ([`Billboard`]), die sich jedes Bild
//! zur Kamera drehen.

use bevy::{
    asset::RenderAssetUsages,
    gltf::GltfAssetLabel,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, Face, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    transform::TransformSystems,
    world_serialization::WorldAsset,
};

use super::{camera::MainCamera, paint::Canvas};
use crate::assets::Preload;

const SHADER: &str = "shaders/fx.wesl";

/// Faktor, um den Leuchtendes heller als 1 wird und damit im Bloom glüht.
pub const GLOW: f32 = 2.2;

/// Wie sich das Material mit dem Bild dahinter mischt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FxBlend {
    /// Additiv, wie Licht (Babylons `ALPHA_ADD`).
    Add,
    /// Deckend nach Alpha (Rauch, Balken, Beschriftungen).
    Alpha,
}

/// Ein selbstleuchtendes Material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[uniform(0, FxUniform)]
#[bind_group_data(FxKey)]
pub struct FxMaterial {
    /// Farbe (linear, darf über 1 gehen) und Deckkraft.
    pub color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    pub blend: FxBlend,
    /// Über allem, ohne Tiefentest (Babylons zweite Rendergruppe): Statusbalken, Blitze, Druckwellen.
    pub on_top: bool,
    /// Beidseitig (flache Scheiben, Billboards); sonst nur die Vorderseiten.
    pub double_sided: bool,
    /// Kantenleuchten (Exponent; 0 aus): Die Mitte bleibt dunkel, der Rand leuchtet (Schildhülle). Negativ umgekehrt:
    /// Die Mitte leuchtet und verläuft zum Rand (Halo eines Bolzens).
    pub rim: f32,
    /// Lässt die Textur fließen (UV je Sekunde); eine zweite, gespiegelte Lage läuft langsamer darüber (Plasma).
    pub flow: Vec2,
}

impl FxMaterial {
    /// Additiv leuchtend, nur Vorderseiten.
    pub fn light(color: LinearRgba) -> Self {
        Self {
            color,
            texture: None,
            blend: FxBlend::Add,
            on_top: false,
            double_sided: false,
            rim: 0.0,
            flow: Vec2::ZERO,
        }
    }

    /// Nach Alpha gemischt, nur Vorderseiten.
    pub fn solid(color: LinearRgba) -> Self {
        Self {
            blend: FxBlend::Alpha,
            ..Self::light(color)
        }
    }

    pub fn with_texture(self, texture: Handle<Image>) -> Self {
        Self {
            texture: Some(texture),
            double_sided: true,
            ..self
        }
    }

    pub fn on_top(self) -> Self {
        Self {
            on_top: true,
            ..self
        }
    }

    pub fn double_sided(self) -> Self {
        Self {
            double_sided: true,
            ..self
        }
    }

    /// Setzt die Deckkraft.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.color.alpha = alpha;
    }
}

/// Eine Palettenfarbe (sRGB) als lineare Farbe mit dieser Deckkraft.
pub fn linear(color: Color, alpha: f32) -> LinearRgba {
    color.to_linear().with_alpha(alpha)
}

/// Eine Palettenfarbe, die im Bloom glüht.
pub fn glowing(color: Color, alpha: f32) -> LinearRgba {
    (color.to_linear() * GLOW).with_alpha(alpha)
}

/// Lineare Mischung zweier Farben in sRGB (wie Babylons `Color3.Lerp` auf Gamma-Werten).
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    Color::srgb(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
    )
}

#[derive(Clone, Default, ShaderType)]
pub struct FxUniform {
    color: Vec4,
    params: Vec4,
}

impl From<&FxMaterial> for FxUniform {
    fn from(material: &FxMaterial) -> Self {
        Self {
            color: material.color.to_vec4(),
            params: Vec4::new(
                material.rim,
                if material.blend == FxBlend::Add {
                    1.0
                } else {
                    0.0
                },
                material.flow.x,
                material.flow.y,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FxKey {
    on_top: bool,
    double_sided: bool,
}

impl From<&FxMaterial> for FxKey {
    fn from(material: &FxMaterial) -> Self {
        Self {
            on_top: material.on_top,
            double_sided: material.double_sided,
        }
    }
}

impl Material for FxMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    /// Vormultipliziert: Der Shader entscheidet zwischen additiv und Alpha-Mischung.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = (!key.bind_group_data.double_sided).then_some(Face::Back);
        if key.bind_group_data.on_top
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            depth.depth_compare = Some(CompareFunction::Always);
        }
        Ok(())
    }
}

/// Dreht sich jedes Bild zur Kamera (Babylons `BILLBOARDMODE_ALL`): Die lokale +Z-Achse zeigt zur Kamera, die Kinder
/// stehen in ihrem Raum nebeneinander im Bild.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Billboard;

/// Gemeinsame Meshes und Sprite-Texturen der Effekte.
#[derive(Resource, Clone)]
pub struct FxAssets {
    /// Quadrat 1×1 in der xy-Ebene, Vorderseite +Z (Billboards, Balken).
    pub quad: Handle<Mesh>,
    /// Quadrat 1×1 flach in der xz-Ebene, Vorderseite oben (Marken am Boden); u entlang +x, v entlang +z.
    pub floor_quad: Handle<Mesh>,
    /// Kugel mit Durchmesser 1.
    pub sphere: Handle<Mesh>,
    /// Würfel mit Kantenlänge 1.
    pub cube: Handle<Mesh>,
    /// Zylinder mit Durchmesser 1 und Länge 1 entlang z (Strahlen).
    pub beam: Handle<Mesh>,
    /// Weiche Scheibe: Partikel und Blitze.
    pub dot: Handle<Image>,
    /// Ring mit weichen Kanten: Druckwellen, Zielmarken, Portal.
    pub ring: Handle<Image>,
    /// Kachelbare Plasmafäden (Helligkeit im Alpha): die brodelnde Hülle eines Bolzens.
    pub plasma: Handle<Image>,
    /// Kugel mit Durchmesser 1, die Pole auf der z-Achse (u läuft rundum, v von Pol zu Pol).
    pub spindle: Handle<Mesh>,
    /// Modell des Torpedos (`tools/ships/fleet/torpedo.py`), Spitze nach −Z, Mitte im Ursprung.
    pub torpedo: Handle<WorldAsset>,
}

/// Kantenlänge der berechneten Sprite-Texturen.
const SPRITE_SIZE: u32 = 64;

pub(super) struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FxMaterial>::default())
            .init_resource::<FxAssets>()
            .add_systems(PostUpdate, face_camera.before(TransformSystems::Propagate));
    }
}

impl FromWorld for FxAssets {
    fn from_world(world: &mut World) -> Self {
        let sprite = |alpha_at: fn(f32) -> f32| {
            let mut canvas = Canvas::new(SPRITE_SIZE);
            canvas.fill_with(|point| Some(Srgba::WHITE.with_alpha(alpha_at(point.length()))));
            canvas.into_image()
        };
        let mut images = world.resource_mut::<Assets<Image>>();
        let dot = images.add(sprite(|distance| (1.0 - distance).max(0.0).powi(2)));
        let ring = images.add(sprite(|distance| {
            (1.0 - (distance - 0.72).abs() / 0.22).max(0.0).powf(1.5)
        }));
        let plasma = images.add(plasma());
        let torpedo = world
            .resource::<AssetServer>()
            .load(GltfAssetLabel::Scene(0).from_asset("models/ships/torpedo.glb"));
        world.resource_mut::<Preload>().track(torpedo.clone());
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self {
            quad: meshes.add(Rectangle::new(1.0, 1.0)),
            floor_quad: meshes.add(floor_quad()),
            sphere: meshes.add(Sphere::new(0.5).mesh().uv(16, 12)),
            spindle: meshes.add(
                Sphere::new(0.5)
                    .mesh()
                    .uv(16, 12)
                    .rotated_by(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ),
            cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            beam: meshes.add(
                Cylinder::new(0.5, 1.0)
                    .mesh()
                    .resolution(10)
                    .build()
                    .rotated_by(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ),
            dot,
            ring,
            plasma,
            torpedo,
        }
    }
}

/// Kantenlänge der Plasmatextur.
const PLASMA_SIZE: u32 = 128;

/// Plasmafäden: verwirbeltes, gefaltetes Rauschen, dessen Grate als helle Fäden stehen, grobe Knoten vor feinen Fäden,
/// damit sie auch auf einem kleinen Bolzen lesbar bleiben; kachelt in beiden Richtungen.
fn plasma() -> Image {
    let mut canvas = Canvas::new(PLASMA_SIZE);
    canvas.fill_with(|point| {
        let p = (point + 1.0) / 2.0;
        let warp = Vec2::new(fbm(p + 0.37, 4), fbm(p + 0.71, 4)) - 0.5;
        let p = p + warp * 0.35;
        let ridges = (0..4)
            .map(|octave| {
                let ridge = 1.0 - (2.0 * value_noise(p, 2 << octave) - 1.0).abs();
                ridge.powi(6) * 0.4f32.powi(octave)
            })
            .sum::<f32>()
            / 1.624;
        Some(Srgba::WHITE.with_alpha((ridges * 1.8).min(1.0)))
    });
    let mut image = canvas.into_image();
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    image
}

/// Fraktales Rauschen aus `octaves` Lagen, Mittel 0,5, kachelt auf dem Einheitsquadrat.
fn fbm(p: Vec2, octaves: i32) -> f32 {
    (0..octaves)
        .map(|octave| value_noise(p, 4 << octave) * 0.5f32.powi(octave))
        .sum::<f32>()
        / (2.0 - 0.5f32.powi(octaves - 1))
}

/// Wertrauschen in [0, 1] auf einem Gitter aus `cells` Zellen je Einheit, glatt interpoliert; kachelt auf dem
/// Einheitsquadrat.
fn value_noise(p: Vec2, cells: i32) -> f32 {
    let hash = |x: i32, y: i32| {
        let (x, y) = (x.rem_euclid(cells) as u32, y.rem_euclid(cells) as u32);
        let h = (x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ cells as u32)
            .wrapping_mul(1_274_126_177);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let p = p * cells as f32;
    let (cell, f) = (p.floor(), p - p.floor());
    let s = f * f * (3.0 - 2.0 * f);
    let (x, y) = (cell.x as i32, cell.y as i32);
    let bottom = hash(x, y) + (hash(x + 1, y) - hash(x, y)) * s.x;
    let top = hash(x, y + 1) + (hash(x + 1, y + 1) - hash(x, y + 1)) * s.x;
    bottom + (top - bottom) * s.y
}

/// Quadrat in der xz-Ebene mit nach oben zeigender Vorderseite.
fn floor_quad() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, 0.0, -0.5],
            [0.5, 0.0, -0.5],
            [0.5, 0.0, 0.5],
            [-0.5, 0.0, 0.5],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]))
}

/// Dreht Billboards zur Kamera, bevor die Lagen weitergereicht werden. Die Drehung der Vorfahren kommt aus ihren
/// Transforms dieses Bilds (die Maßstäbe sind gleichmäßig), damit nichts nachhinkt, wenn ein Schiff in der Kurve dreht;
/// über einem Billboard als Vorfahren geht es nicht weiter.
fn face_camera(
    camera: Single<&Transform, With<MainCamera>>,
    parents: Query<&ChildOf>,
    ancestors: Query<&Transform, Without<Billboard>>,
    mut billboards: Query<(Entity, &mut Transform), (With<Billboard>, Without<MainCamera>)>,
) {
    let rotation = camera.rotation;
    for (entity, mut transform) in &mut billboards {
        let parent = parents
            .iter_ancestors(entity)
            .map_while(|ancestor| ancestors.get(ancestor).ok())
            .fold(Quat::IDENTITY, |parent, ancestor| {
                ancestor.rotation * parent
            });
        transform.rotation = parent.inverse() * rotation;
    }
}
