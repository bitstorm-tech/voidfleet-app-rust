//! glTF-Erweiterung, die den Schiffen beim Laden ihre Materialien gibt (Port von `loadShipModels` in
//! `scene/ship.ts`): Aus jedem glTF-Material mit `extras.vf` wird ein [`ToonMaterial`] (Label `<material>/toon`) und,
//! außer bei Hologrammen, ein [`OutlineMaterial`] (Label `<material>/outline`); jedes Primitiv bekommt das eine und eine
//! Konturhülle mit dem anderen als Kind (Label `<primitive>/outline`). Jedes
//! Material der Modelle trägt den Vertrag; Bevys `StandardMaterial` für glTF ist abgeschaltet (`PbrPlugin` in
//! `lib.rs`). Die Szenen der Modelle sind damit schon fertig, wenn sie geladen sind; gespawnt wird ohne weiteren Umbau.
//!
//! Die Maske der Verwitterung (`baseColorTexture` der toon-Materialien, ein Bild je Modell) ist Daten, kein sRGB, und
//! braucht eine volle Mip-Kette, die Bevy für PNGs nicht erzeugt: Ohne sie flimmerte die Maske auf dem Brett. Beides
//! richtet [`weather_image`] beim Laden ein, das Bild ersetzt sein Sub-Asset unter demselben Label. Das Wrack bettet
//! die Maske nicht ein, seine Stücke nehmen die ihres Schiffs (`mask` im Vertrag): So liegt sie nur einmal im Speicher.

use bevy::{
    asset::{LoadContext, RenderAssetUsages},
    ecs::world::EntityWorldMut,
    gltf::{
        GltfAssetLabel, GltfMaterial,
        extensions::{ErasedGltfExtensionHandler, GltfExtensionHandler, GltfExtensionHandlers},
        gltf,
    },
    image::{ImageSampler, ImageSamplerDescriptor},
    mesh::MeshVertexAttribute,
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::TextureFormat,
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

/// Richtet die Maske eines Modells einmal ein (das erste Material, das sie nutzt): ersetzt das Bild unter seinem Label
/// durch [`weather_image`]. Alle toon-Materialien teilen die eine Textur (`tools/ships/vf.py` fasst sie zusammen).
fn prepare_weather(load_context: &mut LoadContext<'_>, texture: &Handle<Image>) {
    let Some(label) = texture
        .path()
        .and_then(|path| path.label())
        .map(str::to_owned)
    else {
        return;
    };
    let Some(image) = load_context
        .get_labeled_by_id(texture)
        .and_then(|asset| asset.get::<Image>())
        .filter(|image| image.texture_descriptor.mip_level_count == 1)
    else {
        return;
    };
    let Some(image) = weather_image(image.clone()) else {
        warn!("Maske der Verwitterung {label} ist kein RGBA mit 8 Bit: ohne Mip-Kette");
        return;
    };
    // Ein zweites Sub-Asset unter demselben Label ersetzt das erste samt Handle (`LoadContext::add_labeled_asset`)
    load_context.add_labeled_asset(label, image);
}

/// Die Maske als lineare Daten (die Bytes bleiben, nur das sRGB-Format fällt), mit voller Mip-Kette und einem Sampler,
/// der zwischen Stufen und Texeln mischt (anisotrop für die flach gesehenen Decks). Sie lebt nur auf der Grafikkarte:
/// Die Kopie im Hauptspeicher (bis 48 MB je Schiff) liest niemand mehr. `None` für andere Formate als RGBA mit 8 Bit.
fn weather_image(mut image: Image) -> Option<Image> {
    if !matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
    ) {
        return None;
    }
    let size = image.texture_descriptor.size;
    let (data, levels) = mip_chain(
        image.data.take()?,
        size.width as usize,
        size.height as usize,
    );
    image.data = Some(data);
    image.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    image.texture_descriptor.mip_level_count = levels;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        anisotropy_clamp: 8,
        ..ImageSamplerDescriptor::linear()
    });
    Some(image)
}

/// Hängt an die Texel eines RGBA-Bilds (4 Bytes je Texel) alle Mip-Stufen bis 1 × 1: je Stufe das gerundete Mittel
/// über 2 × 2 Texel; bei ungerader Größe fällt die letzte Zeile bzw. Spalte weg. Dieselbe Rechnung wie `downsample` in
/// `tools/ships/weather.py`, damit die Vorschau zeigt, was das Spiel zeigt. Liefert die Daten und die Zahl der Stufen.
fn mip_chain(mut data: Vec<u8>, mut width: usize, mut height: usize) -> (Vec<u8>, u32) {
    let mut start = 0;
    let mut levels = 1;
    // Die Kette ist ein Drittel länger; ohne das verdoppelte `Vec` beim Anhängen seinen Platz
    data.reserve_exact(data.len() / 3 + 4);
    while width > 1 || height > 1 {
        let (w, h) = ((width / 2).max(1), (height / 2).max(1));
        let mut next = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                for channel in 0..4 {
                    let texel = |dx: usize, dy: usize| {
                        let (tx, ty) = ((2 * x + dx).min(width - 1), (2 * y + dy).min(height - 1));
                        u32::from(data[start + (ty * width + tx) * 4 + channel])
                    };
                    let sum = texel(0, 0) + texel(1, 0) + texel(0, 1) + texel(1, 1);
                    next.push(((sum + 2) / 4) as u8);
                }
            }
        }
        start = data.len();
        data.extend_from_slice(&next);
        (width, height) = (w, h);
        levels += 1;
    }
    (data, levels)
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
        let weather_texture =
            match &spec {
                // Im GLB des Schiffs ist die Maske die einzige Textur
                ToonSpec::Toon {
                    mask: Some(ship), ..
                } => Some(load_context.load(
                    GltfAssetLabel::Texture(0).from_asset(format!("models/ships/{ship}.glb")),
                )),
                ToonSpec::Toon { .. } => material_asset.base_color_texture.clone(),
                _ => None,
            };
        if let Some(texture) = &weather_texture {
            prepare_weather(load_context, texture);
        }
        if !spec.is_hologram() {
            load_context.add_labeled_asset(
                format!("{material_label}/outline"),
                OutlineMaterial::default(),
            );
        }
        load_context.add_labeled_asset(
            format!("{material_label}/toon"),
            ToonMaterial::new(spec, heat_texture, weather_texture),
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

    /// Je Stufe das gerundete Mittel über 2 × 2 Texel bis 1 × 1, ungerade Ränder fallen weg.
    #[test]
    fn mip_chain_averages_down_to_one_texel() {
        let gray = |values: &[u8]| values.iter().flat_map(|&v| [v; 4]).collect::<Vec<u8>>();
        let (data, levels) = mip_chain(gray(&[0, 4, 8, 12, 100, 104, 108, 112]), 4, 2);
        assert_eq!(levels, 3);
        assert_eq!(data[8 * 4..], gray(&[52, 60, 56])[..]);
        let (data, levels) = mip_chain(gray(&[10, 20, 255, 30, 41, 255, 255, 255, 255]), 3, 3);
        assert_eq!((levels, &data[9 * 4..]), (2, &gray(&[25])[..]));
    }

    /// Der Material-Vertrag (`tools/ships/vf.py`) an den gebauten Modellen: Alle toon-Materialien eines Schiffs teilen
    /// eine Maske (die Textur 0 auf ein PNG `<id>_weather`) oder keins hat eine; die eines Wracks nennen stattdessen ihr
    /// Schiff und betten kein Bild ein. UVs tragen genau die Primitive mit Maske oder Glutmuster, Lichter und Hologramme
    /// nicht.
    #[test]
    fn models_follow_the_contract() {
        let dir = std::path::Path::new(crate::REPO_ASSETS).join("models/ships");
        let mut weathered = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let ship = name.trim_end_matches("_wreck");
            let gltf = gltf::Gltf::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let mut masks = Vec::new();
            for material in gltf.materials() {
                let texture = material.pbr_metallic_roughness().base_color_texture();
                match spec(&material).unwrap() {
                    ToonSpec::Toon { mask: Some(of), .. } => {
                        assert!(name.ends_with("_wreck") && of == ship, "{name}");
                        assert!(texture.is_none(), "{name}");
                    }
                    ToonSpec::Toon { .. } => masks.push(texture.map(|info| info.texture())),
                    ToonSpec::Wreck { .. } => {
                        assert!(material.emissive_texture().is_some(), "{name}")
                    }
                    _ => assert!(texture.is_none(), "{name}"),
                }
            }
            let first = masks.first().cloned().flatten();
            assert!(
                masks
                    .iter()
                    .all(|mask| mask.as_ref().map(|t| t.index())
                        == first.as_ref().map(|t| t.index())),
                "{name}"
            );
            if let Some(texture) = first {
                assert_eq!(texture.index(), 0, "{name}");
                assert_eq!(
                    texture.source().name(),
                    Some(format!("{ship}_weather").as_str()),
                    "{name}"
                );
                weathered += 1;
            }
            for primitive in gltf
                .meshes()
                .flat_map(|mesh| mesh.primitives().collect::<Vec<_>>())
            {
                let material = primitive.material();
                let textured = material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .is_some()
                    || material.emissive_texture().is_some()
                    || matches!(spec(&material), Some(ToonSpec::Toon { mask: Some(_), .. }));
                let uvs = primitive.get(&gltf::Semantic::TexCoords(0)).is_some();
                assert_eq!(uvs, textured, "{name}: {:?}", material.name());
            }
        }
        assert!(weathered > 0, "kein Modell mit Verwitterung");
    }
}
