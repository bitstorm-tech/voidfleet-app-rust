//! Alle Farben, die eine Bedeutung tragen (Spieler, Gegner, Schaden, Schild …), für Szene und UI gleichermaßen; dazu
//! die Akzentfarbe je Admiral, der Zierton der Flaggschiffe und die Kartenfarben nach Schiffsklasse bzw. Befehlstyp
//! (Port von `data/palette.ts` und `data/cardColors.ts`). Die Werte sind sRGB wie im Original.

use bevy::prelude::*;

use voidfleet_core::{
    admirals::AdmiralId,
    cards::{CommandType, ShipClass},
};

/// Farbe aus einer Hex-Angabe wie `0xE8A33A`.
const fn hex(rgb: u32) -> Color {
    Color::srgb(
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
    )
}

/// Hintergrund hinter allem.
pub const BACKGROUND: Color = Color::srgb(0.016, 0.02, 0.04);
/// Eigene Seite: Schüsse, Banner, Akzente im UI.
pub const PLAYER: Color = Color::srgb(0.47, 0.76, 1.0);
pub const ENEMY: Color = Color::srgb(1.0, 0.5, 0.42);
/// Auswahl und gültige Ziele: Kartenrahmen, leuchtende Pads.
pub const SELECT: Color = Color::srgb(0.45, 0.92, 1.0);
/// Wirkung eines Ereignisses: Fließtexte, Pulse, Balken.
pub const DAMAGE: Color = Color::srgb(1.0, 0.44, 0.35);
pub const HEAL: Color = Color::srgb(0.47, 0.93, 0.55);
pub const SHIELD: Color = Color::srgb(0.47, 0.76, 1.0);
pub const BUFF: Color = Color::srgb(1.0, 0.84, 0.4);
pub const DEBUFF: Color = Color::srgb(0.78, 0.59, 1.0);
/// Hüllenbalken, je nach Füllstand.
pub const HP_EMPTY: Color = Color::srgb(0.93, 0.22, 0.18);
pub const HP_HALF: Color = Color::srgb(0.95, 0.74, 0.2);
pub const HP_FULL: Color = Color::srgb(0.3, 0.86, 0.38);
/// Spielausgang.
pub const WON: Color = Color::srgb(1.0, 0.84, 0.4);
pub const LOST: Color = Color::srgb(1.0, 0.38, 0.31);
pub const DRAW: Color = Color::srgb(0.69, 0.74, 0.81);
/// Gemeinsamer Zierton aller Flaggschiffe: Rahmen der Flaggschiff-Karte, Krone neben dem Hüllenbalken.
pub const FLAGSHIP_TRIM: Color = hex(0xF4D68C);
/// Text der Oberfläche und gedämpfter Text (`--text`, `--text-dim` in `styles.css`).
pub const TEXT: Color = Color::srgb(242.0 / 255.0, 247.0 / 255.0, 1.0);
pub const TEXT_DIM: Color = Color::srgba(199.0 / 255.0, 219.0 / 255.0, 250.0 / 255.0, 0.92);

/// Akzentfarbe eines Admirals: Auswahlkarte, Fähigkeitskarte, Banner, die Marken seiner Fähigkeit und der Ring unter
/// seinem Flaggschiff. Die Modelle der Flaggschiffe tragen dieselben Töne.
pub const fn admiral_color(admiral: AdmiralId) -> Color {
    match admiral {
        AdmiralId::Osei => hex(0xE8A33A),
        AdmiralId::Vance => hex(0xE2483D),
        AdmiralId::Halvorsen => hex(0x9D5CF2),
        AdmiralId::Moreau => hex(0x33D1C6),
        AdmiralId::Kerr => hex(0x5B8DFF),
    }
}

/// Kartenfarbe einer Schiffsklasse.
pub const fn ship_class_color(class: ShipClass) -> Color {
    match class {
        ShipClass::Fighter => Color::srgb(0.9, 0.3, 0.2),
        ShipClass::Cruiser => Color::srgb(0.3, 0.5, 0.9),
        ShipClass::Battleship => Color::srgb(0.5, 0.5, 0.6),
        ShipClass::Support => Color::srgb(0.2, 0.8, 0.5),
    }
}

/// Kartenfarbe eines Befehlstyps.
pub const fn command_type_color(command: CommandType) -> Color {
    match command {
        CommandType::Attack => Color::srgb(0.9, 0.2, 0.2),
        CommandType::Buff => Color::srgb(0.2, 0.7, 0.3),
        CommandType::Debuff => Color::srgb(0.6, 0.2, 0.8),
        CommandType::Tactic => Color::srgb(0.2, 0.6, 0.9),
        CommandType::Reposition => Color::srgb(0.9, 0.7, 0.1),
        CommandType::Special => Color::srgb(0.9, 0.5, 0.1),
    }
}

/// Dunkler um den Anteil `amount`, in sRGB gerechnet (wie Godots `Color.darkened`).
pub fn darkened(color: Color, amount: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(
        srgba.red * (1.0 - amount),
        srgba.green * (1.0 - amount),
        srgba.blue * (1.0 - amount),
        srgba.alpha,
    )
}

/// Heller um den Anteil `amount`, in sRGB gerechnet (wie Godots `Color.lightened`).
pub fn lightened(color: Color, amount: f32) -> Color {
    let srgba = color.to_srgba();
    let lift = |channel: f32| channel + (1.0 - channel) * amount;
    Color::srgba(
        lift(srgba.red),
        lift(srgba.green),
        lift(srgba.blue),
        srgba.alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_matches_css() {
        let osei = admiral_color(AdmiralId::Osei).to_srgba();
        assert_eq!(osei.to_hex(), "#E8A33A");
        assert_eq!(FLAGSHIP_TRIM.to_srgba().to_hex(), "#F4D68C");
    }

    #[test]
    fn darken_and_lighten() {
        let gray = Color::srgb(0.5, 0.5, 0.5);
        assert_eq!(darkened(gray, 0.5).to_srgba().red, 0.25);
        assert_eq!(lightened(gray, 0.5).to_srgba().red, 0.75);
    }
}
