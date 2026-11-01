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
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, Face, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    transform::TransformSystems,
};

use super::{camera::MainCamera, paint::Canvas};

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
    /// Kantenleuchten (Exponent; 0 aus): Die Mitte bleibt dunkel, der Rand leuchtet (Schildblase).
    pub rim: f32,
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
                0.0,
                0.0,
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
}

/// Kantenlänge der berechneten Sprite-Texturen.
const SPRITE_SIZE: u32 = 64;

pub(super) struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FxMaterial>::default())
            .add_systems(PreStartup, create_assets)
            .add_systems(PostUpdate, face_camera.before(TransformSystems::Propagate));
    }
}

fn create_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let sprite = |alpha_at: fn(f32) -> f32| {
        let mut canvas = Canvas::new(SPRITE_SIZE);
        canvas.fill_with(|point| Some(Srgba::WHITE.with_alpha(alpha_at(point.length()))));
        canvas.into_image()
    };
    commands.insert_resource(FxAssets {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        floor_quad: meshes.add(floor_quad()),
        sphere: meshes.add(Sphere::new(0.5).mesh().uv(16, 12)),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        beam: meshes.add(
            Cylinder::new(0.5, 1.0)
                .mesh()
                .resolution(10)
                .build()
                .rotated_by(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ),
        dot: images.add(sprite(|distance| (1.0 - distance).max(0.0).powi(2))),
        ring: images.add(sprite(|distance| {
            (1.0 - (distance - 0.72).abs() / 0.22).max(0.0).powf(1.5)
        })),
    });
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
/// Transforms dieses Bilds (die Maßstäbe sind gleichmäßig), damit nichts nachhinkt, wenn ein Schiff in der Kurve dreht.
#[allow(clippy::type_complexity)]
fn face_camera(
    camera: Single<&Transform, With<MainCamera>>,
    ancestors: Query<(&Transform, Option<&ChildOf>), Without<Billboard>>,
    mut billboards: Query<
        (&mut Transform, Option<&ChildOf>),
        (With<Billboard>, Without<MainCamera>),
    >,
) {
    let rotation = camera.rotation;
    for (mut transform, child_of) in &mut billboards {
        let mut parent = Quat::IDENTITY;
        let mut next = child_of.map(ChildOf::parent);
        while let Some(entity) = next {
            let Ok((ancestor, child_of)) = ancestors.get(entity) else {
                break;
            };
            parent = ancestor.rotation * parent;
            next = child_of.map(ChildOf::parent);
        }
        transform.rotation = parent.inverse() * rotation;
    }
}
