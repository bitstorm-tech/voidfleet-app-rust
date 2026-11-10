// Der Zielpfeil (`AimArrowMaterial` in `crates/client/src/ui/aim_arrow.rs`): ein holografisches Band entlang einer
// quadratischen Bézierkurve vom Gewählten zum Mauszeiger, mit Pfeilspitze. Im Band laufen Winkel zur Spitze, die
// Kanten leuchten, außen ein Schein, darüber Abtastzeilen. Gezeichnet auf einem Knoten über das ganze Fenster.

import bevy_render::globals::Globals;
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct AimArrow {
    // xy Anfang, zw Leitpunkt der Kurve (physische Pixel)
    from_control: vec4<f32>,
    // xy Spitze (physische Pixel), z physische Pixel je logischem
    tip_scale: vec4<f32>,
    // rgb linear, a Deckkraft
    color: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> arrow: AimArrow;

// Die Kurve als Streckenzug.
const STEPS: i32 = 28;
// Die Spitze in logischen Pixeln: Länge, halbe Breite, Kerbe hinten.
const HEAD: f32 = 34.0;
const HEAD_HALF: f32 = 15.0;
const NOTCH: f32 = 9.0;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let px = arrow.tip_scale.z;
    let p = in.uv * in.size;
    let a = arrow.from_control.xy;
    let c = arrow.from_control.zw;
    let tip = arrow.tip_scale.xy;
    // Außerhalb der Hülle der Kurve (samt Schein) bleibt das Bild leer; ohne Werte (alles 0) also überall.
    let pad = 48.0 * px;
    if any(p <= min(min(a, c), tip) - pad) || any(p >= max(max(a, c), tip) + pad) {
        return vec4<f32>(0.0);
    }

    // Die Kurve endet in der Kerbe der Spitze.
    let forward = normalize(tip - c);
    let b = tip - forward * (HEAD - NOTCH) * px;
    // Nächster Punkt des Streckenzugs: Abstand und Weg bis dorthin.
    var nearest = 1e9;
    var along = 0.0;
    var total = 0.0;
    var previous = a;
    for (var i = 1; i <= STEPS; i++) {
        let t = f32(i) / f32(STEPS);
        let point = mix(mix(a, c, t), mix(c, b, t), t);
        let segment = point - previous;
        let span = max(length(segment), 1e-4);
        let h = clamp(dot(p - previous, segment) / (span * span), 0.0, 1.0);
        let d = distance(p, previous + segment * h);
        if d < nearest {
            nearest = d;
            along = total + h * span;
        }
        total += span;
        previous = point;
    }
    // Ab hier in logischen Pixeln; positiv heißt innen.
    let d = nearest / px;
    let s = along / px;
    let ribbon = mix(3.0, 8.0, along / total) - d;

    let q = (tip - p) / px;
    let x = dot(q, forward);
    let y = abs(dot(q, vec2<f32>(-forward.y, forward.x)));
    let side = (x * HEAD_HALF / HEAD - y) * HEAD / length(vec2<f32>(HEAD, HEAD_HALF));
    let back = HEAD - x - NOTCH * (1.0 - y / HEAD_HALF);
    // Außen der Abstand zu einem Kegel um die Achse, damit der Schein rund bleibt und keine Strahlen wirft.
    let axis = clamp(x, 0.0, HEAD);
    let cone = axis * HEAD_HALF / HEAD - length(vec2<f32>(x - axis, y));
    let head = min(min(side, back), cone);

    let shape = max(ribbon, head);
    let inside = smoothstep(-0.75, 0.75, shape);
    let edge = 1.0 - smoothstep(0.5, 2.0, abs(shape));
    let glow = 0.4 * exp(min(shape, 0.0) / 7.0) * (1.0 - inside);
    // Winkel, die zur Spitze laufen.
    let wave = fract((s + d * 1.3) / 24.0 - globals.time * 1.8);
    let chevron = smoothstep(0.4, 0.55, wave) * (1.0 - smoothstep(0.85, 1.0, wave));
    let fill = mix(
        0.3 + 0.12 * sin(globals.time * 5.0),
        0.12 + 0.5 * chevron,
        smoothstep(0.0, 1.0, ribbon),
    );
    let scan = 0.84 + 0.16 * sin(p.y / px * 1.7 - globals.time * 7.0);
    // Am Anfang blendet das Band ein.
    let fade = smoothstep(0.0, 90.0, s);
    let alpha = clamp((edge + inside * fill) * scan + glow, 0.0, 1.0) * fade * arrow.color.a;
    return vec4<f32>(mix(arrow.color.rgb, vec3<f32>(1.0), edge * 0.6), alpha);
}
