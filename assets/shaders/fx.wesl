// Selbstleuchtende Effekte, Marken, Balken und Bögen (`FxMaterial` in `crates/client/src/scene/fx.rs`): Farbe mal
// Textur mal Vertexfarbe, unbeleuchtet. Ausgegeben wird vormultipliziert: additiv (Licht) mit Alpha 0, sonst mit Alpha.
// Was heller als 1 leuchtet, glüht im Bloom der Kamera.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;

struct FxMaterial {
    // rgb linear (darf über 1 gehen), a Deckkraft
    color: vec4<f32>,
    // x: Kantenleuchten (Exponent, 0 aus; negativ leuchtet die Mitte), y: additiv (1) oder Alpha-Mischung (0)
    params: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: FxMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var fx_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var fx_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = material.color;
    @if(VERTEX_UVS_A) {
        color *= textureSample(fx_texture, fx_sampler, in.uv);
    }
    @if(VERTEX_COLORS) {
        color *= in.color;
    }
    if material.params.x != 0.0 {
        // Kantenleuchten wie Babylons Fresnel: am Rand hell, zur Mitte hin dunkel; negativ umgekehrt (weicher Halo).
        let to_camera = normalize(view.world_position - in.world_position.xyz);
        let facing = abs(dot(normalize(in.world_normal), to_camera));
        let glow = select(facing, 1.0 - facing, material.params.x > 0.0);
        color = vec4<f32>(color.rgb * pow(glow, abs(material.params.x)), color.a);
    }
    let alpha = clamp(color.a, 0.0, 1.0);
    return vec4<f32>(color.rgb * alpha, alpha * (1.0 - material.params.y));
}
