//! glTF-Erweiterung, die den Schiffen beim Laden ihre Materialien gibt (Port von `loadShipModels` in
//! `scene/ship.ts`): Aus jedem glTF-Material mit `extras.vf` wird ein [`ToonMaterial`] (Label `<material>/toon`) und,
//! außer bei Hologrammen, ein [`OutlineMaterial`] (Label `<material>/outline`); jedes Primitiv bekommt das eine und eine
//! Konturhülle mit dem anderen als Kind (Label `<primitive>/outline`). Jedes
//! Material der Modelle trägt den Vertrag; Bevys `StandardMaterial` für glTF ist abgeschaltet (`PbrPlugin` in
//! `lib.rs`). Die Szenen der Modelle sind damit schon fertig, wenn sie geladen sind; gespawnt wird ohne weiteren Umbau.
//!
//! Die Maske der Verwitterung liegt neben dem GLB (`<id>_weather.ktx2`, `mask` im Vertrag der toon-Materialien von
//! Schiff und Wrack): BC7 mit Mip-Kette, ein Viertel des Grafikspeichers roher Texel, einmal je Schiff. Dazu teilen
//! alle Rümpfe die Feinstruktur `detail.ktx2`. Beide sind Daten, kein sRGB, und leben nur auf der Grafikkarte
//! ([`data_texture`]).

use bevy::{
    asset::{LoadContext, RenderAssetUsages},
    ecs::world::EntityWorldMut,
    gltf::{
        GltfMaterial,
        extensions::{ErasedGltfExtensionHandler, GltfExtensionHandler, GltfExtensionHandlers},
        gltf,
    },
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    mesh::MeshVertexAttribute,
    platform::collections::HashMap,
    prelude::*,
};

use super::{
    material::{ToonMaterial, ToonSpec},
    outline::{OutlineMaterial, ToonOutline, outline_mesh},
};

/// Meldet die Erweiterung beim glTF-Loader an.
pub(super) fn register(app: &mut App) {
    app.world_mut()
        .resource_mut::<GltfExtensionHandlers>()
        .0
        .write_blocking()
        .push(Box::new(ToonGltf));
}

#[derive(Clone, Default)]
struct ToonGltf;

/// Der Vertrag eines glTF-Materials, sofern es einen trägt.
fn spec(material: &gltf::Material) -> Option<ToonSpec> {
    material
        .extras()
        .as_ref()
        .and_then(|extras| ToonSpec::from_extras(extras.get()))
}

/// Feinstruktur aller Rümpfe (`tools/ships/detail.py`).
const DETAIL: &str = "models/ships/detail.ktx2";

/// Eine Textur aus `tools/ships/`, die der Shader als Daten liest: linear statt sRGB, zwischen Mip-Stufen und Texeln
/// gemischt (anisotrop für die flach gesehenen Decks), nur auf der Grafikkarte.
fn data_texture(
    load_context: &mut LoadContext<'_>,
    path: String,
    address_mode: ImageAddressMode,
) -> Handle<Image> {
    load_context
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.asset_usage = RenderAssetUsages::RENDER_WORLD;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: address_mode,
                address_mode_v: address_mode,
                anisotropy_clamp: 8,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(path)
}

fn outline_label(mesh: &gltf::Mesh, primitive: &gltf::Primitive) -> String {
    let primitive = GltfAssetLabel::Primitive {
        mesh: mesh.index(),
        primitive: primitive.index(),
    };
    format!("{primitive}/outline")
}

impl GltfExtensionHandler for ToonGltf {
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler> {
        Box::new(self.clone())
    }

    fn on_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_material: &gltf::Material,
        _material: Handle<GltfMaterial>,
        material_asset: &GltfMaterial,
        material_label: &str,
    ) {
        let Some(spec) = spec(gltf_material) else {
            return;
        };
        let heat_texture = matches!(spec, ToonSpec::Wreck { .. })
            .then(|| material_asset.emissive_texture.clone())
            .flatten();
        let weather = match &spec {
            ToonSpec::Toon {
                mask: Some(ship), ..
            } => Some((
                data_texture(
                    load_context,
                    format!("models/ships/{ship}_weather.ktx2"),
                    ImageAddressMode::ClampToEdge,
                ),
                data_texture(load_context, DETAIL.into(), ImageAddressMode::Repeat),
            )),
            _ => None,
        };
        if !spec.is_hologram() {
            load_context.add_labeled_asset(
                format!("{material_label}/outline"),
                OutlineMaterial::default(),
            );
        }
        load_context.add_labeled_asset(
            format!("{material_label}/toon"),
            ToonMaterial::new(spec, heat_texture, weather),
        );
    }

    async fn on_gltf_primitive(
        &mut self,
        load_context: &mut LoadContext<'_>,
        _gltf_document: &gltf::Gltf,
        gltf_mesh: &gltf::Mesh<'_>,
        gltf_primitive: &gltf::Primitive<'_>,
        buffer_data: &[Vec<u8>],
        _custom_vertex_attributes: &HashMap<Box<str>, MeshVertexAttribute>,
        _gltf_mesh_on_skinned_nodes: bool,
        _gltf_mesh_on_non_skinned_nodes: bool,
        _user_mesh: &mut Option<Mesh>,
    ) {
        if spec(&gltf_primitive.material()).is_none_or(|spec| spec.is_hologram()) {
            return;
        }
        let reader =
            gltf_primitive.reader(|buffer| buffer_data.get(buffer.index()).map(Vec::as_slice));
        let (Some(positions), Some(normals), Some(indices)) = (
            reader.read_positions(),
            reader.read_normals(),
            reader.read_indices(),
        ) else {
            warn!("Primitiv ohne Positionen, Normalen oder Indizes: keine Kontur");
            return;
        };
        let normals: Vec<[f32; 3]> = normals.collect();
        let mesh = outline_mesh(positions.collect(), &normals, indices.into_u32().collect());
        load_context.add_labeled_asset(outline_label(gltf_mesh, gltf_primitive), mesh);
    }

    fn on_spawn_mesh_and_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        primitive: &gltf::Primitive,
        mesh: &gltf::Mesh,
        material: &gltf::Material,
        entity: &mut EntityWorldMut,
        material_label: &str,
    ) {
        let Some(spec) = spec(material) else {
            return;
        };
        let toon = load_context.get_label_handle::<ToonMaterial>(format!("{material_label}/toon"));
        entity.insert(MeshMaterial3d(toon));
        if !spec.is_hologram() {
            let outline = load_context.get_label_handle::<Mesh>(outline_label(mesh, primitive));
            let color = load_context
                .get_label_handle::<OutlineMaterial>(format!("{material_label}/outline"));
            entity.with_child((
                Name::new("Kontur"),
                ToonOutline,
                // Sichtbar erst am Ziel unter dem Fadenkreuz (`mark_target` in `scene/sync.rs`)
                Visibility::Hidden,
                Mesh3d(outline),
                MeshMaterial3d(color),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Der Material-Vertrag (`tools/ships/vf.py`) an den gebauten Modellen: Alle toon-Materialien eines Schiffs und
    /// seines Wracks nennen dieselbe Maske, die neben dem GLB liegt, oder keins hat eine; ein Bild bettet nur das
    /// Glutmuster der Bruchflächen ein. UVs tragen genau die Primitive mit Maske oder Glutmuster, Lichter und
    /// Hologramme nicht.
    #[test]
    fn models_follow_the_contract() {
        let dir = std::path::Path::new(crate::REPO_ASSETS).join("models/ships");
        assert!(dir.join("detail.ktx2").exists());
        let mut weathered = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|extension| extension != "glb") {
                continue;
            }
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let ship = name.trim_end_matches("_wreck");
            let gltf = gltf::Gltf::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let mut masks = Vec::new();
            for material in gltf.materials() {
                assert!(
                    material
                        .pbr_metallic_roughness()
                        .base_color_texture()
                        .is_none(),
                    "{name}"
                );
                match spec(&material).unwrap() {
                    ToonSpec::Toon { mask, .. } => masks.push(mask),
                    ToonSpec::Wreck { .. } => {
                        assert!(material.emissive_texture().is_some(), "{name}")
                    }
                    _ => {}
                }
            }
            let masked = masks.first().is_some_and(Option::is_some);
            let expected = masked.then(|| ship.to_owned());
            assert!(masks.iter().all(|mask| *mask == expected), "{name}");
            assert_eq!(
                dir.join(format!("{ship}_weather.ktx2")).exists(),
                masked,
                "{name}"
            );
            weathered += usize::from(masked);
            for primitive in gltf
                .meshes()
                .flat_map(|mesh| mesh.primitives().collect::<Vec<_>>())
            {
                let material = primitive.material();
                let textured = material.emissive_texture().is_some()
                    || matches!(spec(&material), Some(ToonSpec::Toon { mask: Some(_), .. }));
                let uvs = primitive.get(&gltf::Semantic::TexCoords(0)).is_some();
                assert_eq!(uvs, textured, "{name}: {:?}", material.name());
            }
        }
        assert!(weathered > 0, "kein Modell mit Verwitterung");
    }
}
