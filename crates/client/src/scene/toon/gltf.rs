//! glTF-Erweiterung, die den Schiffen beim Laden ihre Toon-Materialien gibt (Port von `loadShipModels` in
//! `scene/ship.ts`): Aus jedem glTF-Material mit `extras.vf` wird ein [`ToonMaterial`] (Label `<material>/toon`), jedes
//! Primitiv bekommt es und, außer bei Hologrammen, eine Konturhülle als Kind (Label `<primitive>/outline`). Jedes
//! Material der Modelle trägt den Vertrag; Bevys `StandardMaterial` für glTF ist abgeschaltet (`PbrPlugin` in
//! `lib.rs`). Die Szenen der Modelle sind damit schon fertig, wenn sie geladen sind; gespawnt wird ohne weiteren Umbau.

use bevy::{
    asset::LoadContext,
    ecs::world::EntityWorldMut,
    gltf::{
        GltfAssetLabel, GltfMaterial,
        extensions::{ErasedGltfExtensionHandler, GltfExtensionHandler, GltfExtensionHandlers},
        gltf,
    },
    mesh::MeshVertexAttribute,
    platform::collections::HashMap,
    prelude::*,
};

use super::{
    material::{ToonMaterial, ToonSpec},
    outline::{OUTLINE_MATERIAL, ToonOutline, outline_mesh},
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
        load_context.add_labeled_asset(
            format!("{material_label}/toon"),
            ToonMaterial::new(spec, heat_texture),
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
            entity.with_child((
                Name::new("Kontur"),
                ToonOutline,
                Mesh3d(outline),
                MeshMaterial3d(OUTLINE_MATERIAL),
            ));
        }
    }
}
