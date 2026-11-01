//! Das Toon-Material der Schiffe und Wracks (`assets/shaders/toon.wesl`), ein Material für alle Arten des
//! Material-Vertrags aus `tools/ships/vf.py`.

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use serde::{Deserialize, Deserializer};

use super::look::{
    GHOST_ALPHA, GHOST_HOLOGRAM_ALPHA, GHOST_LIGHT, GHOST_TONES, GLOW_BOOST, KEY, RIM_FLAT,
    RIM_ROUND,
};

const SHADER: &str = "shaders/toon.wesl";

/// Eine Farbe aus dem Vertrag (sRGB-Hex wie `"#56657a"`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hex(pub Color);

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Srgba::hex(&hex)
            .map(|color| Self(color.into()))
            .map_err(serde::de::Error::custom)
    }
}

/// Der Material-Vertrag (`extras.vf` jedes glTF-Materials aus `tools/ships/`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ToonSpec {
    /// Drei Töne (Licht, Halbschatten, Schatten), harte Stufen; `round` für geglättete Teile, `glint` Glanzpunkt.
    Toon {
        tones: [Hex; 3],
        round: bool,
        glint: f32,
    },
    /// Unbeleuchtet in genau dieser Farbe: Düsen, Fenster, Emitter, Kennzeichen; mit `glow` im Glühen.
    Light { color: Hex, glow: bool },
    /// Durchscheinend und als einzige Art beidseitig.
    Hologram { color: Hex, alpha: f32, glow: bool },
    /// Bruchflächen der Wracks: eckige Toon-Fläche plus Glutmuster (Textur) × `heat`.
    Wreck { tones: [Hex; 3], heat: Hex },
}

/// `extras` eines glTF-Materials.
#[derive(Deserialize)]
struct Extras {
    vf: ToonSpec,
}

impl ToonSpec {
    /// Liest den Vertrag aus den `extras` eines glTF-Materials (JSON); `None` für Materialien ohne `vf`.
    pub fn from_extras(json: &str) -> Option<Self> {
        serde_json::from_str::<Extras>(json)
            .ok()
            .map(|extras| extras.vf)
    }

    pub fn is_hologram(&self) -> bool {
        matches!(self, Self::Hologram { .. })
    }
}

/// Ein Toon-Material. Die Modelle bringen je glTF-Material eine Vorlage mit; jedes Schiff bekommt beim Spawnen eigene
/// Kopien, denn Seite, Glut und Lichter gelten je Schiff ([`crate::scene::ship::ToonState`]).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[uniform(0, ToonUniform)]
#[bind_group_data(ToonKey)]
pub struct ToonMaterial {
    pub spec: ToonSpec,
    /// Geist eines eingeplanten Schiffs: bläuliches Hologramm in drei Tönen, halb durchsichtig, ohne Konturen.
    pub ghost: bool,
    /// +1 eigene Seite (Nase nach +x), −1 Gegner: Das Hauptlicht ist je Seite gespiegelt.
    pub facing: f32,
    /// Glut der Bruchflächen (linear); `None`: die Farbe aus dem Vertrag.
    pub heat: Option<LinearRgba>,
    /// Helligkeit der Lichter (1 an, 0 erloschen).
    pub lights: f32,
    /// Überlagerung in einer Farbe (rgb linear, a Anteil): Treffer-Blitz, Glühen beim Warp, Verdunkeln der Trümmer.
    pub overlay: Vec4,
    /// Faktor auf die Deckkraft (Flackern des Trugbilds).
    pub opacity: f32,
    /// Das Blitzlicht der Explosionen: Lage (xyz) und Stärke (w), siehe [`super::ToonFlash`].
    pub flash: Vec4,
    /// Glutmuster der Bruchflächen (nur `wreck`).
    #[texture(1)]
    #[sampler(2)]
    pub heat_texture: Option<Handle<Image>>,
}

impl ToonMaterial {
    pub fn new(spec: ToonSpec, heat_texture: Option<Handle<Image>>) -> Self {
        Self {
            spec,
            ghost: false,
            facing: 1.0,
            heat: None,
            lights: 1.0,
            overlay: Vec4::ZERO,
            opacity: 1.0,
            flash: Vec4::ZERO,
            heat_texture,
        }
    }
}

/// Was der Shader je Material liest (`ToonMaterial` in `toon.wesl`).
#[derive(Clone, Default, ShaderType)]
pub struct ToonUniform {
    tone_lit: Vec4,
    tone_mid: Vec4,
    tone_dark: Vec4,
    /// rgb Glut der Bruchflächen.
    heat: Vec4,
    flash: Vec4,
    /// xyz Richtung zum Hauptlicht.
    key: Vec4,
    /// xyz Schwellen des Kantenlichts auf N·R und facing und das höchste N·oben.
    rim_style: Vec4,
    /// x gestuft beleuchtet (1) oder genau seine Farbe (0), y Glanzpunkt, z Helligkeit, w Deckkraft.
    surface: Vec4,
    /// rgb Farbe der Überlagerung, a ihr Anteil.
    overlay: Vec4,
}

fn linear(color: Color) -> Vec4 {
    color.to_linear().to_vec4()
}

impl From<&ToonMaterial> for ToonUniform {
    fn from(material: &ToonMaterial) -> Self {
        let ghost_tones = GHOST_TONES.map(linear);
        let (tones, lit, glint, power, opacity, rim) = match &material.spec {
            ToonSpec::Toon {
                tones,
                round,
                glint,
            } => (
                tones.map(|tone| linear(tone.0)),
                true,
                *glint,
                1.0,
                1.0,
                if *round { RIM_ROUND } else { RIM_FLAT },
            ),
            ToonSpec::Wreck { tones, .. } => (
                tones.map(|tone| linear(tone.0)),
                true,
                0.0,
                1.0,
                1.0,
                RIM_FLAT,
            ),
            ToonSpec::Light { color, glow } => {
                let color = if material.ghost { GHOST_LIGHT } else { color.0 };
                let glow = if *glow { GLOW_BOOST } else { 1.0 };
                (
                    [linear(color); 3],
                    false,
                    0.0,
                    glow * material.lights,
                    1.0,
                    RIM_FLAT,
                )
            }
            ToonSpec::Hologram { color, alpha, glow } => {
                let glow = if *glow { GLOW_BOOST } else { 1.0 };
                let alpha = if material.ghost {
                    alpha * GHOST_HOLOGRAM_ALPHA
                } else {
                    *alpha
                };
                ([linear(color.0); 3], false, 0.0, glow, alpha, RIM_FLAT)
            }
        };
        let tones = if material.ghost && lit {
            ghost_tones
        } else {
            tones
        };
        let opacity = if material.ghost && !material.spec.is_hologram() {
            GHOST_ALPHA
        } else {
            opacity
        };
        let heat = match (&material.spec, material.heat) {
            (_, Some(heat)) => heat.to_vec4(),
            (ToonSpec::Wreck { heat, .. }, None) => linear(heat.0),
            _ => Vec4::ZERO,
        };
        Self {
            tone_lit: tones[0],
            tone_mid: tones[1],
            tone_dark: tones[2],
            heat,
            flash: material.flash,
            key: (KEY * Vec3::new(material.facing, 1.0, 1.0)).extend(0.0),
            rim_style: Vec3::from_array(rim).extend(0.0),
            surface: Vec4::new(
                if lit { 1.0 } else { 0.0 },
                glint,
                power,
                opacity * material.opacity,
            ),
            overlay: material.overlay,
        }
    }
}

/// Teil des Pipeline-Schlüssels: Hologramme zeichnen beidseitig, alles andere nur Vorderseiten (die Hüllen sind
/// geschlossen und nach außen orientiert).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToonKey {
    double_sided: bool,
}

impl From<&ToonMaterial> for ToonKey {
    fn from(material: &ToonMaterial) -> Self {
        Self {
            double_sided: material.spec.is_hologram(),
        }
    }
}

impl Material for ToonMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.ghost || self.spec.is_hologram() {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = (!key.bind_group_data.double_sided).then_some(Face::Back);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_contract() {
        let toon = ToonSpec::from_extras(
            r##"{"vf": {"kind": "toon", "tones": ["#56657a", "#3a4657", "#222a36"], "round": true, "glint": 0.6}}"##,
        )
        .unwrap();
        assert!(matches!(toon, ToonSpec::Toon { round: true, glint, .. } if glint == 0.6));
        let wreck =
            ToonSpec::from_extras(r##"{"vf": {"kind": "wreck", "tones": ["#3b3532", "#2c2624", "#1c1816"], "heat": "#ff9945"}}"##)
                .unwrap();
        assert!(matches!(wreck, ToonSpec::Wreck { .. }));
        assert!(ToonSpec::from_extras(r#"{"other": 1}"#).is_none());
    }

    #[test]
    fn enemy_light_is_mirrored() {
        let spec = ToonSpec::Light {
            color: Hex(Color::WHITE),
            glow: false,
        };
        let mut material = ToonMaterial::new(spec, None);
        material.facing = -1.0;
        let uniform = ToonUniform::from(&material);
        assert_eq!(uniform.key.x, -KEY.x);
        assert_eq!(uniform.surface.z, 1.0);
    }
}
