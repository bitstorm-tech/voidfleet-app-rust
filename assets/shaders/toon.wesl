// Clean Toon: Fragment-Shader der Schiffe und Wracks (`ToonMaterial` in `crates/client/src/scene/toon/material.rs`).
// Werte wie `tools/ships/look.py`. Alles Licht ist gerechnet, nicht beleuchtet; gerechnet wird linear, Bevy schreibt
// sRGB. Was heller als 1 wird (Lichter mit Glühen, heiße Glut), glüht im Bloom der Kamera.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;

struct ToonMaterial {
    tone_lit: vec4<f32>,
    tone_mid: vec4<f32>,
    tone_dark: vec4<f32>,
    // rgb: Glut der Bruchflächen
    heat: vec4<f32>,
    // xyz: Lage des Blitzlichts, w: Stärke
    flash: vec4<f32>,
    // xyz: Richtung zum Hauptlicht (je Seite gespiegelt)
    key: vec4<f32>,
    // xyz: Schwellen des Kantenlichts auf N·R und facing, höchstes N·oben
    rim_style: vec4<f32>,
    // x: gestuft beleuchtet (1) oder genau seine Farbe (0), y: Glanzpunkt, z: Helligkeit, w: Deckkraft
    surface: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: ToonMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var heat_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var heat_sampler: sampler;

// Schwellen auf N·L: Schatten | Halbschatten | Licht.
const SHADOW_STEP: f32 = 0.12;
const LIGHT_STEP: f32 = 0.5;
// Kantenlicht von hinten oben: Blickrichtung der Kamera plus so viel „oben“.
const RIM_UP: f32 = 0.9;
const RIM_COLOR: vec3<f32> = vec3<f32>(0.23840, 0.41254, 0.64448);
const RIM_MIX: f32 = 0.65;
const SEAM_FACING: f32 = 0.86;
const SEAM_MIX: f32 = 0.5;
const GLINT_COLOR: vec3<f32> = vec3<f32>(0.77582, 0.83880, 0.90466);
const GLINT_THRESHOLD: f32 = 0.975;
// Blitzlicht der Explosionen: zwei harte Stufen auf der Bestrahlung, die je so viel aufhellen.
const FLASH_COLOR: vec3<f32> = vec3<f32>(1.0, 0.55201, 0.24620);
const FLASH_LOW: f32 = 1.5;
const FLASH_HIGH: f32 = 6.0;
const FLASH_LIFT: f32 = 0.22;

// Harte Stufe, deren Kante über die Bildableitung geglättet ist: auf flachen Facetten exakt, auf runden Teilen ohne
// Treppen.
fn band(edge: f32, value: f32) -> f32 {
    let width = max(fwidth(value) * 0.7, 1e-4);
    return smoothstep(edge - width, edge + width, value);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var normal = normalize(in.world_normal);
    if !is_front {
        normal = -normal;
    }
    var color = material.tone_lit.rgb * material.surface.z;
    if material.surface.x > 0.5 {
        let to_camera = normalize(view.world_position - in.world_position.xyz);
        let key = material.key.xyz;
        let light = max(dot(normal, key), 0.0);
        color = mix(material.tone_dark.rgb, material.tone_mid.rgb, band(SHADOW_STEP, light));
        color = mix(color, material.tone_lit.rgb, band(LIGHT_STEP, light));

        let forward = -view.world_from_view[2].xyz;
        let rim_direction = normalize(forward + vec3<f32>(0.0, RIM_UP, 0.0));
        let facing = 1.0 - abs(dot(normal, to_camera));
        let rim_style = material.rim_style.xyz;
        let rim = band(rim_style.x, dot(normal, rim_direction)) * band(rim_style.y, facing)
            * (1.0 - band(rim_style.z, normal.y));
        let seam = band(SEAM_FACING, facing) * (1.0 - band(SHADOW_STEP, light));
        color = mix(color, RIM_COLOR, RIM_MIX * max(rim, SEAM_MIX * seam));
        color = mix(color, GLINT_COLOR, material.surface.y * band(GLINT_THRESHOLD, dot(normal, normalize(to_camera + key))));

        let to_flash = material.flash.xyz - in.world_position.xyz;
        let distance2 = dot(to_flash, to_flash);
        let irradiance = material.flash.w * max(dot(normal, to_flash), 0.0) * inverseSqrt(distance2 + 1e-4) / (1.0 + distance2);
        color += FLASH_COLOR * FLASH_LIFT * (band(FLASH_LOW, irradiance) + band(FLASH_HIGH, irradiance));

        @if(VERTEX_UVS_A) {
            color += material.heat.rgb * textureSample(heat_texture, heat_sampler, in.uv).r;
        }
    }
    return vec4<f32>(color, material.surface.w);
}
