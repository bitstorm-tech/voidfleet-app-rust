//! Konturen der Schiffe und Wracks als „Inverted Hull“: Zu jedem Primitiv gehört ein Zwilling mit geglätteten
//! Normalen, den `assets/shaders/toon_outline.wesl` in fester Pixelbreite nach außen schiebt und nur mit seinen
//! Rückseiten in der Konturfarbe zeichnet. Hinter dem Schiff bleibt so ein Rand stehen, an Außenkontur und
//! Silhouetten, auch dort, wo ein Teil vor einem anderen liegt; was davor liegt, verdeckt ihn. Hologramme und Geister
//! haben keine Konturen.
//!
//! Das Original (Babylon) rechnet die Linien aus einem Kantenpuffer im Material selbst und zeichnet so auch Knicke im
//! Inneren; die Hülle hier zeigt nur Umriss und Silhouetten.

use bevy::{
    asset::{RenderAssetUsages, uuid_handle},
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    window::PrimaryWindow,
};

use super::look::{CONTOUR_WIDTH, LINE_COLOR};

const SHADER: &str = "shaders/toon_outline.wesl";

/// Das eine Kontur-Material aller Schiffe; die glTF-Erweiterung hängt es beim Laden an.
pub const OUTLINE_MATERIAL: Handle<OutlineMaterial> =
    uuid_handle!("6c1f0d0e-9a57-4b52-a7a3-6cfd0f2b8e41");

/// Markiert die Konturhülle eines Primitivs (Kind des Primitivs).
#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
#[reflect(Component, Default)]
pub struct ToonOutline;

/// Material der Konturhüllen.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct OutlineMaterial {
    /// Konturfarbe (linear).
    #[uniform(0)]
    pub color: LinearRgba,
    /// Breite in physischen Pixeln (logische Breite × Skalierung des Fensters).
    #[uniform(0)]
    pub width: f32,
}

impl Default for OutlineMaterial {
    fn default() -> Self {
        Self {
            color: LINE_COLOR.into(),
            width: CONTOUR_WIDTH,
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

pub(super) fn setup(mut materials: ResMut<Assets<OutlineMaterial>>) {
    materials
        .insert(&OUTLINE_MATERIAL, OutlineMaterial::default())
        .expect("Kontur-Material");
}

/// Hält die Breite der Kontur in logischen Pixeln, auch wenn das Fenster auf einen Bildschirm anderer Dichte wandert.
pub(super) fn sync_width(
    window: Single<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    mut materials: ResMut<Assets<OutlineMaterial>>,
) {
    let width = CONTOUR_WIDTH * window.scale_factor();
    if let Some(mut material) = materials.get_mut(&OUTLINE_MATERIAL)
        && material.width != width
    {
        material.width = width;
    }
}

#[cfg(test)]
mod tests {
    use bevy::mesh::VertexAttributeValues;

    use super::*;

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
