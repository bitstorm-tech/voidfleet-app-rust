//! Rastert Vektorgrafiken der Oberfläche zu PNG, weil Bevy kein SVG zeichnet:
//!
//! - die Mauszeiger `assets/cursors/cursor_<id>.svg` (sofern noch vorhanden) in ihrer eigenen Größe (32 × 32) zu
//!   `cursor_<id>.png` daneben; die SVGs sind danach entbehrlich und gelöscht,
//! - die Icons unten (Lucide-Stil, Strich in Weiß, damit die Oberfläche sie einfärbt) zu `assets/images/ui/<name>.png`
//!   in [`ICON_SIZE`] Pixeln.
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
const STROKE: &str = r#"fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round""#;

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
    // Haken der Checkliste und des Gegner-Felds.
    ("check", r#"<path d="M20 6 9 17l-5-5" />"#),
];

/// Gefüllte Icons.
const FILLED_ICONS: &[(&str, &str)] = &[
    // Krone der Flaggschiffe.
    (
        "crown",
        r#"<path d="M3 18 2 7l5.5 4L12 4l4.5 7L22 7l-1 11Z" /><rect x="3" y="19.5" width="18" height="2.5" rx="0.6" />"#,
    ),
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    rasterize_cursors(&root.join("cursors"));
    let icons = root.join("images/ui");
    fs::create_dir_all(&icons).expect("Ordner der Icons");
    let stroked = ICONS.iter().map(|(name, body)| (name, format!("<g {STROKE}>{body}</g>")));
    let filled = FILLED_ICONS
        .iter()
        .map(|(name, body)| (name, format!(r#"<g fill="white">{body}</g>"#)));
    for (name, body) in stroked.chain(filled) {
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">{body}</svg>"#);
        render(&svg, ICON_SIZE, &icons.join(format!("{name}.png")));
    }
}

/// Jede `cursor_<id>.svg` wird zu `cursor_<id>.png` in ihrer eigenen Größe.
fn rasterize_cursors(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Ordner der Mauszeiger") {
        let path = entry.expect("Eintrag").path();
        if path.extension().is_none_or(|extension| extension != "svg") {
            continue;
        }
        let svg = fs::read_to_string(&path).expect("SVG lesbar");
        render(&svg, 0, &path.with_extension("png"));
    }
}

/// Rastert `svg` auf `size` Pixel Kantenlänge (0: eigene Größe) und schreibt das PNG.
fn render(svg: &str, size: u32, out: &Path) {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("gültiges SVG");
    let natural = tree.size();
    let scale = if size == 0 {
        1.0
    } else {
        size as f32 / natural.width().max(natural.height())
    };
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
