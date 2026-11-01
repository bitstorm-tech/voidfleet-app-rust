// Konturen der Schiffe als „Inverted Hull“ (`OutlineMaterial` in `crates/client/src/scene/toon/outline.rs`): Die Hülle
// mit geglätteten Normalen wird in fester Pixelbreite nach außen geschoben, nur ihre Rückseiten werden gezeichnet.

import bevy_pbr::render::mesh_functions::{get_world_from_local, mesh_normal_local_to_world, mesh_position_local_to_clip};
import bevy_pbr::render::mesh_view_bindings::view;

struct OutlineMaterial {
    color: vec4<f32>,
    // Breite in physischen Pixeln
    width: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> outline: OutlineMaterial;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> @builtin(position) vec4<f32> {
    let world_from_local = get_world_from_local(vertex.instance_index);
    var clip = mesh_position_local_to_clip(world_from_local, vec4<f32>(vertex.position, 1.0));
    let world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    // Richtung der Normale im Bild, in Pixeln gemessen, damit die Linie in jeder Richtung gleich breit ist.
    let screen = (view.clip_from_world * vec4<f32>(world_normal, 0.0)).xy * view.viewport.zw;
    let screen_length = length(screen);
    if screen_length > 1e-6 {
        let offset = screen / screen_length * outline.width * 2.0 / view.viewport.zw;
        clip = vec4<f32>(clip.xy + offset * clip.w, clip.zw);
    }
    return clip;
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return outline.color;
}
