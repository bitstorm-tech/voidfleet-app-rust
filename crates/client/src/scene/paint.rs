//! Ein kleiner Zeichengrund für die Texturen, die das Original im Canvas zeichnet (Marken am Boden, Pfeile vor den
//! Spalten, Sterne, Totenkopf, Sprites der Effekte). Gezeichnet wird im Einheitsquadrat wie dort nach `translate` und
//! `scale`: Mitte (0, 0), Rand ±1, y nach unten. Jede Form ist eine Funktion „liegt der Punkt darin?“; der Zeichengrund
//! tastet je Pixel 4×4-fach ab und mischt sRGB-Farben wie der Canvas (source-over).

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

/// Abtastungen je Pixel und Achse.
const SAMPLES: u32 = 4;

/// Quadratischer Zeichengrund, durchsichtig, Farben sRGB mit getrenntem Alpha.
pub struct Canvas {
    size: u32,
    pixels: Vec<Srgba>,
}

impl Canvas {
    pub fn new(size: u32) -> Self {
        Self {
            size,
            pixels: vec![Srgba::NONE; (size * size) as usize],
        }
    }

    /// Punkt im Einheitsquadrat für Pixel (x, y) und Abtastung (sx, sy).
    fn point(&self, x: u32, y: u32, sx: u32, sy: u32) -> Vec2 {
        let step = 1.0 / SAMPLES as f32;
        let at = |pixel: u32, sample: u32| {
            (pixel as f32 + (sample as f32 + 0.5) * step) / self.size as f32 * 2.0 - 1.0
        };
        Vec2::new(at(x, sx), at(y, sy))
    }

    fn blend(&mut self, index: usize, color: Srgba, coverage: f32) {
        let alpha = color.alpha * coverage;
        if alpha <= 0.0 {
            return;
        }
        let below = self.pixels[index];
        let out = alpha + below.alpha * (1.0 - alpha);
        let mix = |top: f32, bottom: f32| {
            (top * alpha + bottom * below.alpha * (1.0 - alpha)) / out.max(1e-6)
        };
        self.pixels[index] = Srgba::new(
            mix(color.red, below.red),
            mix(color.green, below.green),
            mix(color.blue, below.blue),
            out,
        );
    }

    /// Füllt die Form in einer Farbe.
    pub fn fill(&mut self, color: Srgba, shape: impl Fn(Vec2) -> bool) {
        for y in 0..self.size {
            for x in 0..self.size {
                let mut hits = 0;
                for sy in 0..SAMPLES {
                    for sx in 0..SAMPLES {
                        hits += u32::from(shape(self.point(x, y, sx, sy)));
                    }
                }
                let coverage = hits as f32 / (SAMPLES * SAMPLES) as f32;
                self.blend((y * self.size + x) as usize, color, coverage);
            }
        }
    }

    /// Füllt mit einer Farbe je Punkt (Verläufe), in der Pixelmitte bestimmt.
    pub fn fill_with(&mut self, color: impl Fn(Vec2) -> Option<Srgba>) {
        let half = SAMPLES / 2;
        for y in 0..self.size {
            for x in 0..self.size {
                // Die Mitte des Pixels liegt zwischen den mittleren Abtastungen.
                let center =
                    (self.point(x, y, half - 1, half - 1) + self.point(x, y, half, half)) / 2.0;
                if let Some(color) = color(center) {
                    self.blend((y * self.size + x) as usize, color, 1.0);
                }
            }
        }
    }

    pub fn into_image(self) -> Image {
        let data = self
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_u8_array())
            .collect();
        Image::new(
            Extent3d {
                width: self.size,
                height: self.size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

/// Punkt im Raum nach `context.rotate(angle)` (im Uhrzeigersinn, weil y nach unten zeigt).
pub fn unrotate(point: Vec2, angle: f32) -> Vec2 {
    Vec2::from_angle(-angle).rotate(point)
}

/// Kreislinie mit Radius `radius` und Strichbreite `width` um den Ursprung.
pub fn ring(radius: f32, width: f32) -> impl Fn(Vec2) -> bool {
    move |point| (point.length() - radius).abs() <= width / 2.0
}

/// Gestrichelte Kreislinie: Striche und Lücken als Bogenlänge, beginnend bei Winkel 0 (+x).
pub fn dashed_ring(radius: f32, width: f32, dash: f32, gap: f32) -> impl Fn(Vec2) -> bool {
    move |point| {
        let angle = point.y.atan2(point.x).rem_euclid(std::f32::consts::TAU);
        (point.length() - radius).abs() <= width / 2.0 && (angle * radius) % (dash + gap) < dash
    }
}

/// Kreisscheibe.
pub fn disc(center: Vec2, radius: f32) -> impl Fn(Vec2) -> bool {
    move |point| point.distance(center) <= radius
}

/// Rechteck wie `fillRect(x, y, w, h)`.
pub fn rect(x: f32, y: f32, width: f32, height: f32) -> impl Fn(Vec2) -> bool {
    move |point| point.x >= x && point.x <= x + width && point.y >= y && point.y <= y + height
}

/// Gefülltes Vieleck (gerade-ungerade-Regel).
pub fn polygon(points: Vec<Vec2>) -> impl Fn(Vec2) -> bool {
    move |point| {
        let mut inside = false;
        let mut previous = points[points.len() - 1];
        for &current in &points {
            if (current.y > point.y) != (previous.y > point.y)
                && point.x
                    < (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y)
                        + current.x
            {
                inside = !inside;
            }
            previous = current;
        }
        inside
    }
}

/// Linienzug mit Strichbreite `width` (runde Ecken statt Gehrung).
pub fn polyline(points: Vec<Vec2>, width: f32) -> impl Fn(Vec2) -> bool {
    move |point| {
        points.windows(2).any(|segment| {
            let (a, b) = (segment[0], segment[1]);
            let along = (point - a).dot(b - a) / (b - a).length_squared();
            point.distance(a + (b - a) * along.clamp(0.0, 1.0)) <= width / 2.0
        })
    }
}

/// Die Form `count`-mal im Kreis wiederholt (wie eine Schleife mit `context.rotate(index / count · 2π)`).
pub fn around(count: usize, shape: impl Fn(Vec2) -> bool) -> impl Fn(Vec2) -> bool {
    move |point| {
        (0..count).any(|index| {
            shape(unrotate(
                point,
                index as f32 / count as f32 * std::f32::consts::TAU,
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cover_their_area() {
        let mut canvas = Canvas::new(16);
        canvas.fill(Srgba::WHITE, disc(Vec2::ZERO, 0.5));
        assert_eq!(canvas.pixels[(8 * 16 + 8) as usize].alpha, 1.0);
        assert_eq!(canvas.pixels[0].alpha, 0.0);
        let triangle = polygon(vec![
            Vec2::new(0.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ]);
        assert!(triangle(Vec2::new(0.0, 0.5)) && !triangle(Vec2::new(0.9, -0.9)));
        assert!(ring(0.8, 0.1)(Vec2::new(0.0, 0.82)) && !ring(0.8, 0.1)(Vec2::ZERO));
        // Nach einer Vierteldrehung im Uhrzeigersinn liegt +x unten (y nach unten).
        assert!(
            unrotate(Vec2::new(0.0, 1.0), std::f32::consts::FRAC_PI_2).abs_diff_eq(Vec2::X, 1e-6)
        );
    }
}
