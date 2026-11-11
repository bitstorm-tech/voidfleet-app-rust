// Fragment-Shader der Schiffe und Wracks im dreckigen Look (`ToonMaterial` in
// `crates/client/src/scene/toon/material.rs`). Alles Licht ist gerechnet, nicht beleuchtet; gerechnet wird linear, Bevy
// schreibt sRGB. Was heller als 1 wird (Lichter mit Glühen, heiße Glut), glüht im Bloom der Kamera. Reihenfolge auf
// toon-Flächen: drei Töne als matter Verlauf (mit Lackierung), Verwitterung (Plattenton, Fugen, Schmutz, Ruß,
// Kantenabrieb), Kantenlicht und Glanz, Blitzlicht, Glut, Überlagerung. Konturen gibt es nicht; nur das Ziel unter dem
// Fadenkreuz bekommt eine (`toon_outline.wesl`).

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
    // x: beleuchtet (1) oder genau seine Farbe (0), y: Glanz des Materials, z: Helligkeit, w: Deckkraft
    surface: vec4<f32>,
    // rgb: Farbe der Überlagerung (Treffer-Blitz, Warp, Verdunkeln), a: ihr Anteil
    overlay: vec4<f32>,
    // x: Maske der Verwitterung an (1) oder aus (0), y: log2(Dichte der Maske / 32), verschiebt FINE_LOD für Schiffe
    // mit dichterer Maske
    weather: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: ToonMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var heat_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var heat_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var weather_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var weather_sampler: sampler;

// Verlauf über Halb-Lambert (N·L · 0,5 + 0,5): Schatten → Halbschatten, Halbschatten → Licht; danach alles so viel
// dunkler.
const SHADE_MID: vec2<f32> = vec2<f32>(0.3, 0.6);
const SHADE_LIT: vec2<f32> = vec2<f32>(0.62, 0.8);
const EXPOSURE: f32 = 0.8;
// Kantenlicht statt Kontur: Anteil y der Farbe nach facing hoch x.
const RIM_COLOR: vec3<f32> = vec3<f32>(0.23840, 0.41254, 0.64448);
const RIM: vec2<f32> = vec2<f32>(4.0, 0.3);
// Glanz: Exponent x, Stärke y. Ihn tragen Materialien mit Glanz und blankes Metall an abgeschabten Kanten.
const GLINT_COLOR: vec3<f32> = vec3<f32>(0.77582, 0.83880, 0.90466);
const GLOSS: vec2<f32> = vec2<f32>(24.0, 0.5);
// Blitzlicht der Explosionen: zwei harte Stufen auf der Bestrahlung, die je so viel aufhellen.
const FLASH_COLOR: vec3<f32> = vec3<f32>(1.0, 0.55201, 0.24620);
const FLASH_LOW: f32 = 1.5;
const FLASH_HIGH: f32 = 6.0;
const FLASH_LIFT: f32 = 0.22;
// Verwitterung aus der Maske `<id>_weather` (`tools/ships/weather.py`): R Schmutz, G Kantenabrieb, B Plattenton,
// A Fugen; Farben wirken als Faktor. Schmutz und Ruß setzen als Verlauf ein (von x bis y), den ein Korn aufbricht: Die
// Maske hat nur 32 bis 64 Texel je Einheit, ohne Korn wäre der Rand in der Nahaufnahme Nebel.
const PLATE_TONE: f32 = 0.14;
// Fuge: harte Rille unter SEAM_STEP mit Helligkeit SEAM_DARK, darum eine weiche Schulter (Reichweite x auf Kanal A,
// Helligkeit y an der Rille).
const SEAM_STEP: f32 = 0.14;
const SEAM_DARK: f32 = 0.55;
const SEAM_SHOULDER: vec2<f32> = vec2<f32>(0.7, 0.8);
const DIRT: vec2<f32> = vec2<f32>(0.15, 0.5);
const SOOT: vec2<f32> = vec2<f32>(0.7, 0.95);
const DIRT_DESAT: f32 = 0.4;
const DIRT_TINT: vec3<f32> = vec3<f32>(0.55, 0.48, 0.4);
const SOOT_TINT: vec3<f32> = vec3<f32>(0.42, 0.38, 0.34);
const WEAR_STEP: f32 = 0.25;
const WEAR_DESAT: f32 = 0.5;
const WEAR_LIFT: f32 = 1.6;
// Korn in Texeln der Maske: x fein (1 Texel), y grob (4 Texel); so stark verschiebt es Schmutz und Abrieb. MOTTLE: so
// stark fleckt es die offene Fläche.
const GRAIN: vec2<f32> = vec2<f32>(0.3, 0.3);
const MOTTLE: f32 = 0.12;
// Harte Rille und Abrieb nach der Mip-Stufe (log2 der Texel je Pixel bei 32 Texeln je Einheit): von voll bis weg.
const FINE_LOD: vec2<f32> = vec2<f32>(-0.6, 0.2);
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

// Harte Stufe, deren Kante über die Bildableitung geglättet ist: auf flachen Facetten exakt, auf runden Teilen ohne
// Treppen.
fn band(edge: f32, value: f32) -> f32 {
    let width = max(fwidth(value) * 0.7, 1e-4);
    return smoothstep(edge - width, edge + width, value);
}

// Rauschen ohne Textur (Hash nach Hoskins), geglättet zwischen den Gitterpunkten; 0 bis 1.
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(p.xyx * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(cell), hash(cell + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(cell + vec2<f32>(0.0, 1.0)), hash(cell + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

// Verwitterung über dem Verlauf: Plattenton, Fugen, Schmutz, Ruß, Kantenabrieb (nicht im Schatten). `fine` blendet die
// harte Rille und den Abrieb aus, wenn sie zu fein für den Bildschirm würden. `texel` ist die Lage in Texeln der Maske,
// `footprint` Texel je Pixel: Das Korn blendet aus, bevor es feiner als das Bild wird (sonst flimmerte es am bewegten
// Schiff).
fn weathered(color: vec3<f32>, mask: vec4<f32>, light: f32, fine: f32, texel: vec2<f32>, footprint: f32) -> vec3<f32> {
    let grain = GRAIN.x * (1.0 - smoothstep(0.35, 0.7, footprint)) * (noise(texel) - 0.5)
        + GRAIN.y * (1.0 - smoothstep(1.4, 2.8, footprint)) * (noise(texel * 0.25 + 17.0) - 0.5);
    var c = color * (1.0 + PLATE_TONE * (mask.b - 0.5) / 0.3) * (1.0 + MOTTLE * grain / GRAIN.x);
    c *= mix(SEAM_SHOULDER.y, 1.0, smoothstep(0.0, SEAM_SHOULDER.x, mask.a));
    c *= 1.0 - (1.0 - SEAM_DARK) * fine * (1.0 - band(SEAM_STEP, mask.a));
    let dirt = mask.r + grain;
    let grime = mix(c, vec3<f32>(dot(c, LUMA)), DIRT_DESAT) * DIRT_TINT;
    c = mix(c, grime, smoothstep(DIRT.x, DIRT.y, dirt));
    c = mix(c, grime * SOOT_TINT, smoothstep(SOOT.x, SOOT.y, dirt));
    let bare = mix(c, vec3<f32>(dot(c, LUMA)), WEAR_DESAT) * WEAR_LIFT;
    return mix(c, bare, fine * band(WEAR_STEP, mask.g + grain) * smoothstep(0.0, 0.3, light));
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
        let wrap = dot(normal, key) * 0.5 + 0.5;
        color = mix(material.tone_dark.rgb, material.tone_mid.rgb, smoothstep(SHADE_MID.x, SHADE_MID.y, wrap));
        color = mix(color, material.tone_lit.rgb, smoothstep(SHADE_LIT.x, SHADE_LIT.y, wrap));
        // Glanz: der des Materials, dazu blankes Metall an abgeschabten Kanten; Schmutz nimmt ihn weg
        var gloss = material.surface.y;
        @if(VERTEX_UVS_A) {
            if material.weather.x > 0.5 {
                let mask = textureSample(weather_texture, weather_sampler, in.uv);
                let texels = vec2<f32>(textureDimensions(weather_texture));
                let footprint = max(length(dpdx(in.uv) * texels), length(dpdy(in.uv) * texels));
                let fine = 1.0 - smoothstep(FINE_LOD.x, FINE_LOD.y, log2(footprint) - material.weather.y);
                color = weathered(color, mask, light, fine, in.uv * texels, footprint);
                gloss = max(gloss, fine * band(WEAR_STEP, mask.g)) * (1.0 - smoothstep(DIRT.x, DIRT.y, mask.r));
            }
        }

        // `facing` fällt durch Rundung knapp unter 0: pow gäbe NaN, also ein schwarzes Pixel
        let facing = max(1.0 - abs(dot(normal, to_camera)), 0.0);
        let half_way = max(dot(normal, normalize(to_camera + key)), 0.0);
        color = mix(color * EXPOSURE, RIM_COLOR, RIM.y * pow(facing, RIM.x));
        color += GLINT_COLOR * pow(half_way, GLOSS.x) * GLOSS.y * gloss;

        let to_flash = material.flash.xyz - in.world_position.xyz;
        let distance2 = dot(to_flash, to_flash);
        let irradiance = material.flash.w * max(dot(normal, to_flash), 0.0) * inverseSqrt(distance2 + 1e-4) / (1.0 + distance2);
        color += FLASH_COLOR * FLASH_LIFT * (band(FLASH_LOW, irradiance) + band(FLASH_HIGH, irradiance));

        // Die Glut ist außer auf Bruchflächen 0 (die Art entscheidet, nicht die UVs: auch toon-Flächen tragen welche)
        @if(VERTEX_UVS_A) {
            color += material.heat.rgb * textureSample(heat_texture, heat_sampler, in.uv).r;
        }
    }
    color = mix(color, material.overlay.rgb, material.overlay.a);
    return vec4<f32>(color, material.surface.w);
}
