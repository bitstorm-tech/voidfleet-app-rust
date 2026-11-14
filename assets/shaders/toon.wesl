// Fragment-Shader der Schiffe und Wracks im dreckigen Look (`ToonMaterial` in
// `crates/client/src/scene/toon/material.rs`). Alles Licht ist gerechnet, nicht beleuchtet; gerechnet wird linear, Bevy
// schreibt sRGB. Was heller als 1 wird (Lichter mit Glühen, heiße Glut), glüht im Bloom der Kamera. Reihenfolge auf
// toon-Flächen: Relief (Fasen der Plattenkanten, Feinstruktur bzw. die Höhe der Haut), drei Töne als matter Verlauf
// (mit Lackierung), Verwitterung (Helligkeit aus Plattenton und Verdeckung, Fugen, Schmutz, Ruß, Kantenabrieb),
// Zeichnung der Haut, Kantenlicht und Glanz, Leuchten der Haut, Blitzlicht, Glut, Überlagerung. Konturen gibt es nicht;
// nur das Ziel unter dem Fadenkreuz bekommt eine (`toon_outline.wesl`).

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_functions::get_local_from_world;
import bevy_pbr::render::mesh_view_bindings::view;

struct ToonMaterial {
    tone_lit: vec4<f32>,
    tone_mid: vec4<f32>,
    tone_dark: vec4<f32>,
    // rgb: Glut der Bruchflächen
    heat: vec4<f32>,
    // xyz: Richtung zum Hauptlicht (je Seite gespiegelt)
    key: vec4<f32>,
    // x: beleuchtet (1) oder genau seine Farbe (0), y: Glanz des Materials, z: Helligkeit, w: Deckkraft
    surface: vec4<f32>,
    // rgb: Farbe der Überlagerung (Treffer-Blitz, Warp, Verdunkeln), a: ihr Anteil
    overlay: vec4<f32>,
    // x: Maske der Verwitterung an (1) oder aus (0)
    weather: vec4<f32>,
    // rgb: Leuchten der Haut (erlischt mit den Lichtern), a: Haut an (1) oder aus (0)
    skin: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: ToonMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var heat_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var heat_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var weather_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var weather_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var detail_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var detail_sampler: sampler;
// xyz: Lage des Blitzlichts, w: Stärke; ein Puffer für alle Materialien
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var<storage, read> flash: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(8) var skin_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(9) var skin_sampler: sampler;

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
// Verwitterung aus der Maske `<id>_weather` (`tools/ships/weather.py`): R Schmutz, G Kantenabrieb, B Helligkeit
// (Plattenton mal Verdeckung, geteilt durch SHADE), A Fugen; Farben wirken als Faktor. Schmutz und Ruß setzen als
// Verlauf ein (von x bis y), den das Korn der Feinstruktur aufbricht.
const SHADE: f32 = 1.14;
// Fuge: harte Rille unter SEAM_STEP mit Helligkeit SEAM_DARK, darum eine weiche Schulter (Reichweite x auf Kanal A,
// Helligkeit y an der Rille).
const SEAM_STEP: f32 = 0.14;
const SEAM_DARK: f32 = 0.5;
const SEAM_SHOULDER: vec2<f32> = vec2<f32>(0.6, 0.9);
// Fase der Plattenkante: Zwischen der Rille und x auf Kanal A neigt sich die Normale um y (Tangens) zur Fuge hin; so
// fängt eine Kante Licht, die andere liegt im Schatten, und die Platten lesen als Lagen statt als Linien.
const BEVEL: vec2<f32> = vec2<f32>(0.5, 0.9);
const DIRT: vec2<f32> = vec2<f32>(0.15, 0.5);
const SOOT: vec2<f32> = vec2<f32>(0.7, 0.95);
const DIRT_DESAT: f32 = 0.4;
const DIRT_TINT: vec3<f32> = vec3<f32>(0.55, 0.48, 0.4);
const SOOT_TINT: vec3<f32> = vec3<f32>(0.42, 0.38, 0.34);
const WEAR_STEP: f32 = 0.25;
const WEAR_DESAT: f32 = 0.5;
const WEAR_LIFT: f32 = 1.6;
// Feinstruktur `detail.ktx2` (`tools/ships/detail.py`), aus den drei Achsen des Schiffs projiziert: Kacheln je
// Blender-Einheit, fein und grob (das Verhältnis ist krumm, damit sich nichts sichtbar wiederholt). Sie ist feiner als
// die Maske und trägt die Nahaufnahme; auf dem Brett mittelt ihre Mip-Kette sie weg. RELIEF: Neigung der Normale je
// Lage, MOTTLE: so stark fleckt der Ton die Fläche, GRAIN: so stark verschiebt das Korn Schmutz und Abrieb.
const DETAIL: vec2<f32> = vec2<f32>(1.0, 0.27);
const RELIEF: vec2<f32> = vec2<f32>(0.35, 0.22);
const MOTTLE: vec2<f32> = vec2<f32>(0.16, 0.1);
const GRAIN: vec2<f32> = vec2<f32>(0.2, 0.14);
// Haut der Wesen `<id>_skin` (`tools/ships/skin.py`), statt der Feinstruktur: R Zeichnung, die Helligkeit als Faktor
// 2^(SKIN_SPAN · (2R − 1)), wobei dunkle Stellen um bis zu SKIN_DEEP zum Farbton des Schattentons ziehen (Adern in
// Fleisch werden violett); G Leuchten, mal SKIN_GLOW in der Farbe aus `skin`; B Nässe, der Anteil am Glanz des
// Materials, der auf der Haut schärfer und heller ist als auf Metall (SKIN_WET wie GLOSS); A Höhe, ±SKIN_DEPTH
// Einheiten um 0,5, deren Steigung mal SKIN_RELIEF die Normale neigt. SKIN_SPAN und SKIN_DEPTH stehen ebenso in
// skin.py.
const SKIN_SPAN: f32 = 1.25;
const SKIN_DEPTH: f32 = 0.02;
const SKIN_DEEP: f32 = 0.8;
const SKIN_GLOW: f32 = 1.6;
const SKIN_RELIEF: f32 = 1.0;
const SKIN_WET: vec2<f32> = vec2<f32>(56.0, 0.9);
// Ohne Haut: neutrale Zeichnung, kein Leuchten, voller Glanz, flach.
const NO_SKIN: vec4<f32> = vec4<f32>(0.5, 0.0, 1.0, 0.5);
// Harte Rille, Fase und Abrieb nach dem Maßstab (log2 der Bezugstexel je Pixel bei 64 Texeln je Einheit): von voll bis
// weg.
const FINE_LOD: vec2<f32> = vec2<f32>(0.4, 1.2);
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

// Harte Stufe, deren Kante über die Bildableitung geglättet ist: auf flachen Facetten exakt, auf runden Teilen ohne
// Treppen.
fn band(edge: f32, value: f32) -> f32 {
    let width = max(fwidth(value) * 0.7, 1e-4);
    return smoothstep(edge - width, edge + width, value);
}

// Feinstruktur an einem Punkt des Schiffs: xyz Neigung des Reliefs (Steigung der Höhe in Schiffskoordinaten), w Ton;
// dazu das Korn. Drei Projektionen entlang der Achsen, gewichtet nach der Normale.
struct Detail {
    slope: vec3<f32>,
    tone: f32,
    grain: f32,
}

fn detail_layer(position: vec3<f32>, weight: vec3<f32>) -> Detail {
    let x = textureSample(detail_texture, detail_sampler, position.yz) - 0.5;
    let y = textureSample(detail_texture, detail_sampler, position.zx) - 0.5;
    let z = textureSample(detail_texture, detail_sampler, position.xy) - 0.5;
    let slope = weight.x * vec3<f32>(0.0, x.r, x.g) + weight.y * vec3<f32>(y.g, 0.0, y.r)
        + weight.z * vec3<f32>(z.r, z.g, 0.0);
    let rest = weight.x * x.ba + weight.y * y.ba + weight.z * z.ba;
    return Detail(2.0 * slope, 2.0 * rest.x, 2.0 * rest.y);
}

fn detail(position: vec3<f32>, normal: vec3<f32>) -> Detail {
    var weight = pow(abs(normal), vec3<f32>(4.0));
    weight /= weight.x + weight.y + weight.z;
    let fine = detail_layer(position * DETAIL.x, weight);
    let coarse = detail_layer(position * DETAIL.y + 0.37, weight);
    return Detail(
        RELIEF.x * fine.slope + RELIEF.y * coarse.slope,
        MOTTLE.x * fine.tone + MOTTLE.y * coarse.tone,
        GRAIN.x * fine.grain + GRAIN.y * coarse.grain,
    );
}

// Steigung eines Kanals der Abwicklung in der Fläche, je Längeneinheit des Raums von `position` und `normal`: `across`
// sind seine Differenzen über die Nachbartexel in u und v (zwei Texel Abstand), `texels` die Bildgröße. Die Steigung
// kommt aus Nachbartexeln statt aus der Bildableitung: Die wäre in der Vergrößerung je Texel konstant, also stufig.
fn uphill(across: vec2<f32>, texels: vec2<f32>, uv: vec2<f32>, normal: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    // Steigung je Pixel, dann in die Fläche gelegt (Mikkelsen, „Bump Mapping Unparametrized Surfaces on the GPU“)
    let rise = 0.5 * vec2<f32>(dot(across, dpdx(uv) * texels), dot(across, dpdy(uv) * texels));
    let along_x = dpdx(position);
    let along_y = dpdy(position);
    let r1 = cross(along_y, normal);
    let r2 = cross(normal, along_x);
    let det = dot(along_x, r1);
    return (rise.x * r1 + rise.y * r2) * sign(det) / max(abs(det), 1e-20);
}

// Richtung in der Fläche, in der Kanal A der Maske wächst (von der Fuge weg), in der Welt und normiert.
fn away_from_seam(uv: vec2<f32>, normal: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let texels = vec2<f32>(textureDimensions(weather_texture));
    let step = 1.0 / texels;
    let across = vec2<f32>(
        textureSample(weather_texture, weather_sampler, uv + vec2<f32>(step.x, 0.0)).a
            - textureSample(weather_texture, weather_sampler, uv - vec2<f32>(step.x, 0.0)).a,
        textureSample(weather_texture, weather_sampler, uv + vec2<f32>(0.0, step.y)).a
            - textureSample(weather_texture, weather_sampler, uv - vec2<f32>(0.0, step.y)).a,
    );
    let gradient = uphill(across, texels, uv, normal, position);
    return gradient * inverseSqrt(max(dot(gradient, gradient), 1e-20));
}

// Neigung durch die Höhe der Haut (Kanal A) in Schiffskoordinaten, wie die Neigung der Feinstruktur; höchstens 45°.
fn skin_slope(uv: vec2<f32>, normal: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let texels = vec2<f32>(textureDimensions(skin_texture));
    let step = 1.0 / texels;
    let across = vec2<f32>(
        textureSample(skin_texture, skin_sampler, uv + vec2<f32>(step.x, 0.0)).a
            - textureSample(skin_texture, skin_sampler, uv - vec2<f32>(step.x, 0.0)).a,
        textureSample(skin_texture, skin_sampler, uv + vec2<f32>(0.0, step.y)).a
            - textureSample(skin_texture, skin_sampler, uv - vec2<f32>(0.0, step.y)).a,
    );
    let slope = 2.0 * SKIN_DEPTH * SKIN_RELIEF * uphill(across, texels, uv, normal, position);
    return slope * min(1.0, inverseSqrt(max(dot(slope, slope), 1e-20)));
}

// Zeichnung der Haut: heller oder dunkler, und je dunkler, desto mehr im Farbton des Schattentons.
fn markings(color: vec3<f32>, skin: vec4<f32>) -> vec3<f32> {
    let shade = exp2(SKIN_SPAN * (2.0 * skin.r - 1.0));
    let deep = material.tone_dark.rgb / max(material.tone_lit.rgb, vec3<f32>(1e-4));
    let hue = deep / max(dot(deep, LUMA), 1e-4);
    return color * shade * mix(vec3<f32>(1.0), hue, SKIN_DEEP * saturate(1.0 - shade));
}

// Verwitterung über dem Verlauf: Helligkeit, Fugen, Schmutz, Ruß, Kantenabrieb (nicht im Schatten). `fine` blendet die
// harte Rille und den Abrieb aus, wenn sie zu fein für den Bildschirm würden.
fn weathered(color: vec3<f32>, mask: vec4<f32>, light: f32, fine: f32, structure: Detail) -> vec3<f32> {
    var c = color * mask.b * SHADE * (1.0 + structure.tone);
    c *= mix(SEAM_SHOULDER.y, 1.0, smoothstep(0.0, SEAM_SHOULDER.x, mask.a));
    c *= 1.0 - (1.0 - SEAM_DARK) * fine * (1.0 - band(SEAM_STEP, mask.a));
    let dirt = mask.r + structure.grain;
    let grime = mix(c, vec3<f32>(dot(c, LUMA)), DIRT_DESAT) * DIRT_TINT;
    c = mix(c, grime, smoothstep(DIRT.x, DIRT.y, dirt));
    c = mix(c, grime * SOOT_TINT, smoothstep(SOOT.x, SOOT.y, dirt));
    let bare = mix(c, vec3<f32>(dot(c, LUMA)), WEAR_DESAT) * WEAR_LIFT;
    return mix(c, bare, fine * band(WEAR_STEP, mask.g + structure.grain) * smoothstep(0.0, 0.3, light));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var normal = normalize(in.world_normal);
    if !is_front {
        normal = -normal;
    }
    var skin = NO_SKIN;
    @if(VERTEX_UVS_A) {
        if material.skin.a > 0.5 {
            skin = textureSample(skin_texture, skin_sampler, in.uv);
        }
    }
    // Lichter in genau ihrer Farbe, die eines Wesens nach der Zeichnung seiner Haut heller und dunkler
    var color = material.tone_lit.rgb * material.surface.z * exp2(SKIN_SPAN * (2.0 * skin.r - 1.0));
    if material.surface.x > 0.5 {
        let to_camera = normalize(view.world_position - in.world_position.xyz);
        let key = material.key.xyz;
        // `facing` fällt durch Rundung knapp unter 0: pow gäbe NaN, also ein schwarzes Pixel. Das Kantenlicht folgt
        // der Form, nicht dem Relief.
        let facing = max(1.0 - abs(dot(normal, to_camera)), 0.0);
        // Glanz: der des Materials (auf der Haut nur, wo sie nass ist), dazu blankes Metall an abgeschabten Kanten;
        // Schmutz nimmt ihn weg
        var gloss = material.surface.y * skin.b;
        var mask = vec4<f32>(0.0);
        var structure = Detail(vec3<f32>(0.0), 0.0, 0.0);
        var fine = 0.0;
        @if(VERTEX_UVS_A) {
            if material.weather.x > 0.5 {
                // Schiffskoordinaten (Blender-Einheiten): Der Maßstab der Nahaufnahme und die Feinstruktur hängen an
                // ihnen, nicht an der Abwicklung
                let ship_from_world = get_local_from_world(in.instance_index);
                let turn = mat3x3<f32>(ship_from_world[0].xyz, ship_from_world[1].xyz, ship_from_world[2].xyz);
                let position = (ship_from_world * in.world_position).xyz;
                let footprint = 64.0 * max(length(dpdx(position)), length(dpdy(position)));
                fine = 1.0 - smoothstep(FINE_LOD.x, FINE_LOD.y, log2(footprint));
                mask = textureSample(weather_texture, weather_sampler, in.uv);
                let inside = normalize(turn * normal);
                if material.skin.a > 0.5 {
                    structure.slope = skin_slope(in.uv, inside, position);
                } else {
                    structure = detail(position, inside);
                }
                let chamfer = fine * BEVEL.y * band(SEAM_STEP, mask.a) * (1.0 - smoothstep(SEAM_STEP, BEVEL.x, mask.a));
                // Zeilenvektor mal Matrix: zurück in die Welt (`turn` ist Drehung durch Maßstab, daher der Faktor
                // unten)
                let slope = structure.slope * turn;
                normal = normalize(normal - chamfer * away_from_seam(in.uv, normal, in.world_position.xyz)
                    - (slope - normal * dot(slope, normal)) * inverseSqrt(max(dot(turn[0], turn[0]), 1e-20)));
            }
        }
        let light = max(dot(normal, key), 0.0);
        let wrap = dot(normal, key) * 0.5 + 0.5;
        color = mix(material.tone_dark.rgb, material.tone_mid.rgb, smoothstep(SHADE_MID.x, SHADE_MID.y, wrap));
        color = mix(color, material.tone_lit.rgb, smoothstep(SHADE_LIT.x, SHADE_LIT.y, wrap));
        if material.weather.x > 0.5 {
            color = weathered(color, mask, light, fine, structure);
            gloss = max(gloss, fine * band(WEAR_STEP, mask.g)) * (1.0 - smoothstep(DIRT.x, DIRT.y, mask.r));
        }
        color = markings(color, skin);

        let half_way = max(dot(normal, normalize(to_camera + key)), 0.0);
        color = mix(color * EXPOSURE, RIM_COLOR, RIM.y * pow(facing, RIM.x));
        let shine = select(GLOSS, SKIN_WET, material.skin.a > 0.5);
        color += GLINT_COLOR * pow(half_way, shine.x) * shine.y * gloss;
        color += material.skin.rgb * SKIN_GLOW * skin.g;

        let to_flash = flash.xyz - in.world_position.xyz;
        let distance2 = dot(to_flash, to_flash);
        let irradiance = flash.w * max(dot(normal, to_flash), 0.0) * inverseSqrt(distance2 + 1e-4) / (1.0 + distance2);
        color += FLASH_COLOR * FLASH_LIFT * (band(FLASH_LOW, irradiance) + band(FLASH_HIGH, irradiance));

        // Die Glut ist außer auf Bruchflächen 0 (die Art entscheidet, nicht die UVs: auch toon-Flächen tragen welche)
        @if(VERTEX_UVS_A) {
            color += material.heat.rgb * textureSample(heat_texture, heat_sampler, in.uv).r;
        }
    }
    color = mix(color, material.overlay.rgb, material.overlay.a);
    return vec4<f32>(color, material.surface.w);
}
