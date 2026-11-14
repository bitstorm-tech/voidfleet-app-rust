//! Das Material der Schiffe und Wracks im dreckigen Look (`assets/shaders/toon.wesl`), ein Material für alle Arten des
//! Material-Vertrags aus `tools/ships/vf.py`. Der Vertrag nennt beleuchtete Flächen weiter `toon`.

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{
        render_resource::{
            AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        },
        storage::ShaderBuffer,
    },
    shader::ShaderRef,
};
use serde::{Deserialize, Deserializer};

use super::look::{
    GHOST_ALPHA, GHOST_HOLOGRAM_ALPHA, GHOST_LIGHT, GHOST_TONES, GLOW_BOOST, KEY, LIVERY_ACCENT,
    TONE_SPREAD,
};
use crate::palette;

/// Der eine Puffer des Blitzlichts, den alle Toon-Materialien teilen ([`super::ToonFlash`]): Ihn neu zu schreiben ändert
/// kein Material, sonst bereitete Bevy während jeder Explosion jedes Bild alle Materialien samt Pipelines neu vor.
pub const FLASH: Handle<ShaderBuffer> =
    bevy::asset::uuid_handle!("57cba132-720b-4aed-873b-edd21504cc8f");

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
    /// Drei Töne (Licht, Halbschatten, Schatten) als matter Verlauf; `round` für geglättete Teile (nur für den Bau der
    /// Normalen in Blender), `glint` Glanz.
    Toon {
        tones: [Hex; 3],
        round: bool,
        glint: f32,
        /// Das Schiff (`<id>`), dessen Maske der Verwitterung die Fläche trägt (`<id>_weather.ktx2` neben dem GLB),
        /// an Schiff und Wrackstücken dasselbe; ohne Verwitterung fehlt es.
        #[serde(default)]
        mask: Option<String>,
        /// Das Wesen (`<id>`), dessen Haut die Fläche trägt (`<id>_skin.ktx2` neben dem GLB); nur am Fremden.
        #[serde(default)]
        skin: Option<String>,
    },
    /// Unbeleuchtet in genau dieser Farbe: Düsen, Fenster, Emitter, Kennzeichen; mit `glow` im Glühen. Am Fremden mit
    /// Haut, deren Zeichnung das Leuchten ungleichmäßig macht.
    Light {
        color: Hex,
        glow: bool,
        #[serde(default)]
        skin: Option<String>,
    },
    /// Durchscheinend und als einzige Art beidseitig.
    Hologram { color: Hex, alpha: f32, glow: bool },
    /// Bruchflächen der Wracks: eckige Toon-Fläche plus Glutmuster (`emissiveTexture`) × `heat`.
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

    /// Das Wesen, dessen Haut die Fläche trägt.
    pub fn skin(&self) -> Option<&str> {
        match self {
            Self::Toon { skin, .. } | Self::Light { skin, .. } => skin.as_deref(),
            _ => None,
        }
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
    /// In der Lackierung der Gegner ([`livery`]).
    pub livery: bool,
    /// Das Blitzlicht der Explosionen ([`FLASH`]).
    #[storage(7, read_only)]
    pub flash: Handle<ShaderBuffer>,
    /// Glutmuster der Bruchflächen (nur `wreck`).
    #[texture(1)]
    #[sampler(2)]
    pub heat_texture: Option<Handle<Image>>,
    /// Maske der Verwitterung (nur `toon`): R Schmutz, G Kantenabrieb, B Helligkeit (Plattenton mal Verdeckung),
    /// A Fugen; linear, mit Mip-Kette ([`super::gltf`]).
    #[texture(3)]
    #[sampler(4)]
    pub weather_texture: Option<Handle<Image>>,
    /// Feinstruktur der Rümpfe, kachelbar (mit der Maske): R, G Neigung des Reliefs, B Ton, A Korn.
    #[texture(5)]
    #[sampler(6)]
    pub detail_texture: Option<Handle<Image>>,
    /// Haut der Wesen (`skin` im Vertrag): R Zeichnung, G Leuchten, B Nässe, A Höhe; linear, mit Mip-Kette.
    #[texture(8)]
    #[sampler(9)]
    pub skin_texture: Option<Handle<Image>>,
}

impl ToonMaterial {
    /// `weather`: Maske der Verwitterung und Feinstruktur; `skin_texture`: Haut der Wesen.
    pub fn new(
        spec: ToonSpec,
        heat_texture: Option<Handle<Image>>,
        weather: Option<(Handle<Image>, Handle<Image>)>,
        skin_texture: Option<Handle<Image>>,
    ) -> Self {
        let (weather_texture, detail_texture) = weather.unzip();
        Self {
            spec,
            ghost: false,
            facing: 1.0,
            heat: None,
            lights: 1.0,
            overlay: Vec4::ZERO,
            opacity: 1.0,
            livery: false,
            flash: FLASH,
            heat_texture,
            weather_texture,
            detail_texture,
            skin_texture,
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
    /// xyz Richtung zum Hauptlicht.
    key: Vec4,
    /// x beleuchtet (1) oder genau seine Farbe (0), y Glanz, z Helligkeit, w Deckkraft.
    surface: Vec4,
    /// rgb Farbe der Überlagerung, a ihr Anteil.
    overlay: Vec4,
    /// x Verwitterung an (1) oder aus (0).
    weather: Vec4,
    /// rgb Leuchten der Haut (linear, erlischt mit den Lichtern), a Haut an (1) oder aus (0).
    skin: Vec4,
}

/// Linear und auf ≥ 0 begrenzt: Spreizung und Lackierung der Lichter behalten das Chroma und können aus dem sRGB-Raum fallen.
fn linear(color: Color) -> Vec4 {
    color.to_linear().to_vec4().max(Vec4::ZERO)
}

/// Rückt Halbschatten und Schatten in Oklch-Lightness um [`TONE_SPREAD`] vom Licht-Ton weg; Farbton und Chroma bleiben.
fn spread(tones: [Color; 3]) -> [Color; 3] {
    let lit = Oklcha::from(tones[0]).lightness;
    tones.map(|tone| {
        let tone = Oklcha::from(tone);
        tone.with_lightness((lit - (lit - tone.lightness) * TONE_SPREAD).max(0.0))
            .into()
    })
}

/// Ein Licht in der Lackierung der Gegner: im Farbton der Gegnerfarbe; die Rumpftöne bleiben, wie sie sind.
pub fn livery(color: Color) -> Color {
    let tone = Oklcha::from(color);
    let enemy = Oklcha::from(palette::ENEMY);
    tone.with_hue(enemy.hue)
        .with_chroma(tone.chroma + (enemy.chroma - tone.chroma) * LIVERY_ACCENT)
        .into()
}

impl From<&ToonMaterial> for ToonUniform {
    fn from(material: &ToonMaterial) -> Self {
        let ghost_tones = GHOST_TONES.map(linear);
        let (tones, lit, glint, power, opacity) = match &material.spec {
            ToonSpec::Toon { tones, glint, .. } => (
                spread(tones.map(|tone| tone.0)).map(linear),
                true,
                *glint,
                1.0,
                1.0,
            ),
            ToonSpec::Wreck { tones, .. } => (
                spread(tones.map(|tone| tone.0)).map(linear),
                true,
                0.0,
                1.0,
                1.0,
            ),
            ToonSpec::Light { color, glow, .. } => {
                let color = match (material.ghost, material.livery) {
                    (true, _) => GHOST_LIGHT,
                    (false, true) => livery(color.0),
                    (false, false) => color.0,
                };
                let glow = if *glow { GLOW_BOOST } else { 1.0 };
                ([linear(color); 3], false, 0.0, glow * material.lights, 1.0)
            }
            ToonSpec::Hologram { color, alpha, glow } => {
                let glow = if *glow { GLOW_BOOST } else { 1.0 };
                let alpha = if material.ghost {
                    alpha * GHOST_HOLOGRAM_ALPHA
                } else {
                    *alpha
                };
                ([linear(color.0); 3], false, 0.0, glow, alpha)
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
        // Glut nur auf Bruchflächen, Verwitterung nur auf toon-Flächen: Beide tragen UVs, die Art entscheidet
        let heat = match &material.spec {
            ToonSpec::Wreck { heat, .. } => {
                material.heat.map_or(linear(heat.0), LinearRgba::to_vec4)
            }
            _ => Vec4::ZERO,
        };
        let weathered = matches!(material.spec, ToonSpec::Toon { .. })
            && material.weather_texture.is_some()
            && !material.ghost;
        let grown = material.skin_texture.is_some() && !material.ghost;
        let skin_glow = linear(palette::ALIEN).truncate() * material.lights;
        Self {
            tone_lit: tones[0],
            tone_mid: tones[1],
            tone_dark: tones[2],
            heat,
            key: (KEY * Vec3::new(material.facing, 1.0, 1.0)).extend(0.0),
            surface: Vec4::new(
                if lit { 1.0 } else { 0.0 },
                glint,
                power,
                opacity * material.opacity,
            ),
            overlay: material.overlay,
            weather: Vec4::new(if weathered { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0),
            skin: skin_glow.extend(if grown { 1.0 } else { 0.0 }),
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

    /// Die Haut der Wesen: Der Vertrag nennt sie an Flächen und Lichtern; ihr Leuchten erlischt mit den Lichtern,
    /// Geister tragen sie nicht.
    #[test]
    fn skin_glows_with_the_lights() {
        let organ = ToonSpec::from_extras(
            r##"{"vf": {"kind": "light", "color": "#45e0c8", "glow": true, "skin": "brood"}}"##,
        )
        .unwrap();
        assert_eq!(organ.skin(), Some("brood"));
        let mut material = ToonMaterial::new(organ, None, None, Some(Handle::default()));
        assert_eq!(ToonUniform::from(&material).skin.w, 1.0);
        material.lights = 0.0;
        assert_eq!(ToonUniform::from(&material).skin.truncate(), Vec3::ZERO);
        material.ghost = true;
        assert_eq!(ToonUniform::from(&material).skin.w, 0.0);
    }

    /// Die Lackierung färbt Lichter in der Gegnerfarbe.
    #[test]
    fn livery_tints_lights() {
        let light = Oklcha::from(livery(Color::srgb(0.3, 0.6, 1.0)));
        let enemy = Oklcha::from(palette::ENEMY);
        assert!((light.hue - enemy.hue).abs() < 1.0);
    }

    /// Die Spreizung lässt den Licht-Ton stehen und dunkelt Halbschatten und Schatten ab.
    #[test]
    fn spread_darkens_mid_and_dark() {
        let tones =
            ["#56657a", "#3a4657", "#222a36"].map(|hex| Color::from(Srgba::hex(hex).unwrap()));
        let spread = spread(tones).map(Oklcha::from);
        let tones = tones.map(Oklcha::from);
        assert!((spread[0].lightness - tones[0].lightness).abs() < 1e-5);
        assert!(
            spread[1].lightness < tones[1].lightness && spread[2].lightness < tones[2].lightness
        );
    }

    #[test]
    fn enemy_light_is_mirrored() {
        let spec = ToonSpec::Light {
            color: Hex(Color::WHITE),
            glow: false,
            skin: None,
        };
        let mut material = ToonMaterial::new(spec, None, None, None);
        material.facing = -1.0;
        let uniform = ToonUniform::from(&material);
        assert_eq!(uniform.key.x, -KEY.x);
        assert_eq!(uniform.surface.z, 1.0);
    }

    /// Toon-Flächen und Bruchflächen tragen beide UVs: Die Art entscheidet, wer Glut und wer Verwitterung bekommt,
    /// auch wenn die Zerstörung die Glut für alle Materialien eines Schiffs setzt. Geister bleiben sauber.
    #[test]
    fn heat_and_weather_follow_the_kind() {
        let toon = ToonSpec::Toon {
            tones: [Hex(Color::WHITE); 3],
            round: false,
            glint: 0.0,
            mask: None,
            skin: None,
        };
        let wreck = ToonSpec::Wreck {
            tones: [Hex(Color::WHITE); 3],
            heat: Hex(Color::WHITE),
        };
        let mask = Some((Handle::<Image>::default(), Handle::default()));
        let mut hull = ToonMaterial::new(toon, None, mask, None);
        hull.heat = Some(LinearRgba::RED);
        let uniform = ToonUniform::from(&hull);
        assert_eq!((uniform.heat, uniform.weather.x), (Vec4::ZERO, 1.0));
        hull.ghost = true;
        assert_eq!(ToonUniform::from(&hull).weather.x, 0.0);
        let mut fracture = ToonMaterial::new(wreck, Some(Handle::default()), None, None);
        assert_eq!(ToonUniform::from(&fracture).heat, Vec4::ONE);
        fracture.heat = Some(LinearRgba::RED);
        let uniform = ToonUniform::from(&fracture);
        assert_eq!(
            (uniform.heat, uniform.weather.x),
            (LinearRgba::RED.to_vec4(), 0.0)
        );
    }
}
