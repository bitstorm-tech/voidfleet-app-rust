//! Konturen der Schiffe und Wracks als „Inverted Hull“: Zu jedem Primitiv gehört ein Zwilling mit geglätteten
//! Normalen, den `assets/shaders/toon_outline.wesl` nach außen schiebt und nur mit seinen Rückseiten in der Konturfarbe
//! zeichnet. Hinter dem Schiff bleibt so ein Rand stehen, an Außenkontur und Silhouetten, auch dort, wo ein Teil vor
//! einem anderen liegt; was davor liegt, verdeckt ihn. Die Breite ist in Welt-Einheiten gemessen und auf ein Band in
//! Pixeln geklemmt: klein auf dem Brett schmal, damit kleine Teile nicht in Schwarz verschwimmen, herangezoomt breiter.
//! Die Farbe ist je Material ein abgedunkelter Ton der Fläche selbst (Schatten-Ton, bei Lichtern die Lichtfarbe), in
//! der Lackierung der Gegner deren Ton. Hologramme und Geister haben keine Konturen. Das Ziel unter dem Fadenkreuz
//! ([`OutlineMaterial::targeted`]) zeichnet seine Kontur rot.
//!
//! Das Original (Babylon) rechnet die Linien aus einem Kantenpuffer im Material selbst und zeichnet so auch Knicke im
//! Inneren; die Hülle hier zeigt nur Umriss und Silhouetten.

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    window::PrimaryWindow,
};

use super::{
    look::{CONTOUR_DARKEN, CONTOUR_MAX, CONTOUR_MIN, CONTOUR_WORLD},
    material::{ToonSpec, livery},
};
use crate::palette;

const SHADER: &str = "shaders/toon_outline.wesl";

/// Markiert die Konturhülle eines Primitivs (Kind des Primitivs).
#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
#[reflect(Component, Default)]
pub struct ToonOutline;

/// Material der Konturhüllen, je glTF-Material eine Vorlage (Label `<material>/outline`); jedes Schiff bekommt beim
/// Spawnen eigene Kopien, denn die Lackierung gilt je Schiff ([`crate::scene::ship`]).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[uniform(0, OutlineUniform)]
pub struct OutlineMaterial {
    /// Der Vertrag der Fläche, deren Kontur das ist.
    pub spec: ToonSpec,
    /// In der Lackierung der Gegner ([`livery`]).
    pub livery: bool,
    /// Skalierung des Fensters (logische → physische Pixel).
    pub scale: f32,
    /// Das Schiff liegt als gültiges Ziel unter dem Fadenkreuz.
    pub targeted: bool,
}

impl OutlineMaterial {
    pub fn new(spec: ToonSpec) -> Self {
        Self {
            spec,
            livery: false,
            scale: 1.0,
            targeted: false,
        }
    }

    /// Der Schatten-Ton der Fläche (bei Lichtern ihre Farbe), in Oklch um [`CONTOUR_DARKEN`] abgedunkelt.
    pub fn color(&self) -> Color {
        if self.targeted {
            return palette::darkened(palette::HP_EMPTY, 0.3);
        }
        let tone = match &self.spec {
            ToonSpec::Light { color, .. } if self.livery => livery(color.0),
            ToonSpec::Toon { tones, .. } | ToonSpec::Wreck { tones, .. } => tones[2].0,
            ToonSpec::Light { color, .. } | ToonSpec::Hologram { color, .. } => color.0,
        };
        let tone = Oklcha::from(tone);
        tone.with_lightness(tone.lightness * CONTOUR_DARKEN).into()
    }
}

/// Was der Shader je Material liest (`OutlineMaterial` in `toon_outline.wesl`).
#[derive(Clone, Default, ShaderType)]
pub struct OutlineUniform {
    color: Vec4,
    /// x Breite in Welt-Einheiten, y und z kleinste und größte Breite in physischen Pixeln.
    width: Vec4,
}

impl From<&OutlineMaterial> for OutlineUniform {
    fn from(material: &OutlineMaterial) -> Self {
        Self {
            color: material.color().to_linear().to_vec4(),
            width: Vec4::new(
                CONTOUR_WORLD,
                CONTOUR_MIN * material.scale,
                CONTOUR_MAX * material.scale,
                0.0,
            ),
        }
    }
}

impl Material for OutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
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
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

/// Die Konturhülle eines Primitivs: dieselben Dreiecke, aber jede Ecke mit der gemittelten Normale aller Ecken an
/// derselben Stelle, damit die Hülle an harten Kanten nicht aufreißt.
pub fn outline_mesh(positions: Vec<[f32; 3]>, normals: &[[f32; 3]], indices: Vec<u32>) -> Mesh {
    let key = |p: &[f32; 3]| p.map(|v| (v * 1e4).round() as i32);
    let mut sums: HashMap<[i32; 3], Vec3> = HashMap::default();
    for (position, normal) in positions.iter().zip(normals) {
        *sums.entry(key(position)).or_default() += Vec3::from_array(*normal);
    }
    let smoothed: Vec<[f32; 3]> = positions
        .iter()
        .zip(normals)
        .map(|(position, normal)| {
            sums[&key(position)]
                .try_normalize()
                .unwrap_or(Vec3::from_array(*normal))
                .to_array()
        })
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, smoothed)
    .with_inserted_indices(Indices::U32(indices))
}

/// Hält die Breite der Kontur in logischen Pixeln, auch wenn das Fenster auf einen Bildschirm anderer Dichte wandert;
/// erfasst auch Materialien, die später geladen oder kopiert werden.
pub(super) fn sync_width(
    window: Single<&Window, With<PrimaryWindow>>,
    mut materials: ResMut<Assets<OutlineMaterial>>,
) {
    let scale = window.scale_factor();
    let stale: Vec<_> = materials
        .iter()
        .filter(|(_, material)| material.scale != scale)
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut material) = materials.get_mut(id) {
            material.scale = scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::mesh::VertexAttributeValues;

    use super::*;
    use crate::scene::toon::material::Hex;

    /// Die Kontur ist dunkler als der Schatten-Ton.
    #[test]
    fn contour_darkens_the_shadow_tone() {
        let shadow = Color::srgb(0.3, 0.35, 0.45);
        let tone = Hex(Color::WHITE);
        let material = OutlineMaterial::new(ToonSpec::Toon {
            tones: [tone, tone, Hex(shadow)],
            round: false,
            glint: 0.0,
        });
        let plain = material.color();
        assert!(Oklcha::from(plain).lightness < Oklcha::from(shadow).lightness * 0.7);
    }

    /// Zwei Flächen eines Würfelecks: Die Ecke an derselben Stelle bekommt in beiden die gemittelte Normale.
    #[test]
    fn normals_are_smoothed_per_position() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let normals = [
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ];
        let mesh = outline_mesh(positions, &normals, vec![0, 1, 2, 2, 3, 0]);
        let Some(VertexAttributeValues::Float32x3(smoothed)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("Normalen fehlen");
        };
        let corner = Vec3::new(1.0, 1.0, 0.0).normalize().to_array();
        assert_eq!(smoothed[0], corner);
        assert_eq!(smoothed[2], corner);
        assert_eq!(smoothed[1], [0.0, 1.0, 0.0]);
    }
}
