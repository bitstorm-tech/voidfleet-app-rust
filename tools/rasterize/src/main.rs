//! Rastert die Icons der Oberfläche zu PNG, weil Bevy kein SVG zeichnet: Lucide-Stil, Strich in Weiß, damit die
//! Oberfläche sie einfärbt, nach `assets/images/ui/<name>.png` in [`ICON_SIZE`] Pixeln.
//!
//! ```sh
//! cargo run --release --manifest-path tools/rasterize/Cargo.toml
//! ```

use std::{fs, path::Path};

use resvg::{tiny_skia, usvg};

/// Kantenlänge der Icons in Pixeln: doppelt so groß wie die größte Anzeige (24 logische Pixel), scharf auch auf
/// Retina-Bildschirmen.
const ICON_SIZE: u32 = 48;

/// Strich-Icons im Raster 24 × 24 (Lucide); `fill` steht für gefüllte Icons.
const STROKE: &str =
    r#"fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round""#;

const ICONS: &[(&str, &str)] = &[
    // Log aufklappen.
    ("log", r#"<path d="M4 6h16M4 12h16M4 18h10" />"#),
    // Vollbild an und aus (Lucide „maximize“ / „minimize“).
    (
        "fullscreen_enter",
        r#"<path d="M8 3H5a2 2 0 0 0-2 2v3M21 8V5a2 2 0 0 0-2-2h-3M3 16v3a2 2 0 0 0 2 2h3M16 21h3a2 2 0 0 0 2-2v-3" />"#,
    ),
    (
        "fullscreen_exit",
        r#"<path d="M8 3v3a2 2 0 0 1-2 2H3M21 8h-3a2 2 0 0 1-2-2V3M3 16h3a2 2 0 0 1 2 2v3M16 21v-3a2 2 0 0 1 2-2h3" />"#,
    ),
    // Ansicht wechseln (Lucide „switch-camera“).
    (
        "view_toggle",
        r#"<path d="M11 19H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h5" /><path d="M13 5h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-5" /><circle cx="12" cy="12" r="3" /><path d="m18 22-3-3 3-3" /><path d="m6 2 3 3-3 3" />"#,
    ),
    // Zielerfassung und Railgun.
    (
        "crosshair",
        r#"<circle cx="12" cy="12" r="7" /><path d="M12 1v6M12 17v6M1 12h6M17 12h6" />"#,
    ),
    // Pfeil im Aufwerten-Dialog (Lucide „arrow-right“).
    ("arrow_right", r#"<path d="M5 12h14M12 5l7 7-7 7" />"#),
    // Haken der Checkliste und des Gegner-Felds.
    ("check", r#"<path d="M20 6 9 17l-5-5" />"#),
    // Einstellungen (Lucide „settings“).
    (
        "settings",
        r#"<circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />"#,
    ),
    // Spielregeln (Lucide „help-circle“).
    (
        "rules",
        r#"<circle cx="12" cy="12" r="10" /><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" /><line x1="12" y1="17" x2="12.01" y2="17" />"#,
    ),
    // Absichten der Gegner (Zielfeuer trägt das Fadenkreuz oben).
    ("column_fire", r#"<path d="M12 3v13M6 10l6 6 6-6M4 21h16" />"#),
    ("ram", r#"<path d="m6 17 5-5-5-5M13 17l5-5-5-5" />"#),
    (
        "shield",
        r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />"#,
    ),
    (
        "evade",
        r#"<path d="M12.8 19.6A2 2 0 1 0 14 16H2M17.5 8a2.5 2.5 0 1 1 2 4H2M9.8 4.4A2 2 0 1 1 11 8H2" />"#,
    ),
    (
        "charge",
        r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z" />"#,
    ),
    (
        "target_lock",
        r#"<path d="M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2" /><circle cx="12" cy="12" r="3" />"#,
    ),
    (
        "call",
        r#"<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5M19.1 4.9C23 8.8 23 15.1 19.1 19" /><circle cx="12" cy="12" r="2" />"#,
    ),
    // Zustände.
    (
        "flame",
        r#"<path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.07-2.14-.22-4.05 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.15.43-2.29 1-3a2.5 2.5 0 0 0 2.5 2.5z" />"#,
    ),
    (
        "emergency",
        r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3M12 9v4M12 17h.01" />"#,
    ),
    (
        "stasis",
        r#"<path d="M5 22h14M5 2h14M17 22v-4.17a2 2 0 0 0-.59-1.42L12 12l-4.41 4.41A2 2 0 0 0 7 17.83V22M7 2v4.17a2 2 0 0 0 .59 1.42L12 12l4.41-4.41A2 2 0 0 0 17 6.17V2" />"#,
    ),
    (
        "silenced",
        r#"<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9M19.1 4.9C23 8.8 23 15.1 19.1 19M3 3l18 18" /><circle cx="12" cy="12" r="2" />"#,
    ),
    // Stationen Schadenskontrolle (Lucide „wrench“) und Manöver (Lucide „move-horizontal“).
    (
        "repair",
        r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />"#,
    ),
    ("maneuver", r#"<path d="m18 8 4 4-4 4M2 12h20M6 8l-4 4 4 4" />"#),
    // Deckung (Lucide „brick-wall“).
    (
        "cover",
        r#"<rect width="18" height="18" x="3" y="3" rx="2" /><path d="M12 9v6M16 15v6M16 3v6M3 15h18M3 9h18M8 15v6M8 3v6" />"#,
    ),
    // Panzerung: eine Panzerplatte (Lucide „hexagon“) mit Kern.
    (
        "armor",
        r#"<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" /><path d="m12 8 3.5 2v4L12 16l-3.5-2v-4z" />"#,
    ),
];

/// Gefüllte Icons.
const FILLED_ICONS: &[(&str, &str)] = &[
    // Drei Sterne der Flaggschiffe, des Bosses und der Admiralsstation.
    (
        "stars",
        r#"<path d="M4.5 8.4l.88 2.39 2.54.1-1.99 1.57.69 2.45-2.12-1.41-2.12 1.41.69-2.45-1.99-1.57 2.54-.1z" /><path d="M12 8.4l.88 2.39 2.54.1-1.99 1.57.69 2.45-2.12-1.41-2.12 1.41.69-2.45-1.99-1.57 2.54-.1z" /><path d="M19.5 8.4l.88 2.39 2.54.1-1.99 1.57.69 2.45-2.12-1.41-2.12 1.41.69-2.45-1.99-1.57 2.54-.1z" />"#,
    ),
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let icons = root.join("images/ui");
    fs::create_dir_all(&icons).expect("Ordner der Icons");
    let stroked = ICONS
        .iter()
        .map(|(name, body)| (name, format!("<g {STROKE}>{body}</g>")));
    let filled = FILLED_ICONS
        .iter()
        .map(|(name, body)| (name, format!(r#"<g fill="white">{body}</g>"#)));
    for (name, body) in stroked.chain(filled) {
        let svg =
            format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">{body}</svg>"#);
        render(&svg, ICON_SIZE, &icons.join(format!("{name}.png")));
    }
}

/// Rastert `svg` auf `size` Pixel Kantenlänge und schreibt das PNG.
fn render(svg: &str, size: u32, out: &Path) {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("gültiges SVG");
    let natural = tree.size();
    let scale = size as f32 / natural.width().max(natural.height());
    let width = (natural.width() * scale).round() as u32;
    let height = (natural.height() * scale).round() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("Fläche");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(out).expect("PNG geschrieben");
    println!("{}", out.display());
}
