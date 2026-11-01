//! Die Illustrationen des Regel-Dialogs (Port der Geometrie aus `ui/rulesFigures.ts`): das Brett mit der
//! Deckungsregel, die Feuerbefehle in vier Fällen und die beiden Symbole im Bild eines Zugs (Manöver, Feuerbefehle).
//! Ohne Text: Die Beschriftung setzt der Client in seiner Sprache an dieselben Koordinaten des `viewBox`
//! (`crates/client/src/ui/dialogs/rules_figures.rs`). Farben wie die Palette des Clients.

use std::fmt::Write;

/// Gerastert in doppelter Größe des `viewBox`, scharf auch auf Retina-Bildschirmen.
const SCALE: f32 = 2.0;

/// Farbkanäle der Palette (`palette.rs` im Client).
const PLAYER: &str = "120, 194, 255";
const ENEMY: &str = "255, 128, 107";
const SELECT: &str = "115, 235, 255";
const DAMAGE: &str = "255, 112, 89";
const HEAL: &str = "120, 237, 140";

/// Alle Illustrationen als `(Dateiname, SVG)`.
pub fn all() -> [(&'static str, String); 4] {
    [
        ("board", board()),
        ("fire_orders", fire_orders()),
        ("turn_maneuver", turn_maneuver()),
        ("turn_fire", turn_fire()),
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Player,
    Enemy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Highlight {
    Covered,
    Exposed,
}

/// Ein SVG im Aufbau.
struct Svg {
    width: f32,
    height: f32,
    body: String,
}

impl Svg {
    /// Mit den Pfeilspitzen für Schüsse (Schadensfarbe) und Manöver (Auswahlfarbe).
    fn new(width: f32, height: f32) -> Self {
        let mut body = String::from("<defs>");
        for (id, color) in [("shot", DAMAGE), ("move", SELECT)] {
            write!(
                body,
                r#"<marker id="head-{id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 Z" fill="rgb({color})" /></marker>"#
            )
            .unwrap();
        }
        body.push_str("</defs>");
        Self {
            width,
            height,
            body,
        }
    }

    fn push(&mut self, element: String) {
        self.body.push_str(&element);
    }

    fn finish(self) -> String {
        let Self {
            width,
            height,
            body,
        } = self;
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{}" height="{}">{body}</svg>"#,
            width * SCALE,
            height * SCALE,
        )
    }
}

/// Schiffssymbol mit der Nase nach oben (eigene Schiffe) oder unten (Gegner).
fn ship(cx: f32, cy: f32, side: Side, muted: bool) -> String {
    let dir = if side == Side::Player { -1.0 } else { 1.0 };
    let points = [
        (cx, cy + 14.0 * dir),
        (cx + 10.0, cy - 9.0 * dir),
        (cx, cy - 4.0 * dir),
        (cx - 10.0, cy - 9.0 * dir),
    ];
    let path = points
        .iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join("L");
    let color = if side == Side::Player { PLAYER } else { ENEMY };
    let opacity = if muted { 0.35 } else { 1.0 };
    format!(r#"<path d="M{path}Z" fill="rgb({color})" opacity="{opacity}" />"#)
}

fn slot(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    side: Side,
    highlight: Option<Highlight>,
) -> String {
    let (fill, stroke) = match (highlight, side) {
        (Some(Highlight::Covered), _) => {
            (format!("rgba({HEAL}, 0.16)"), format!("rgba({HEAL}, 0.8)"))
        }
        (Some(Highlight::Exposed), _) => (
            format!("rgba({DAMAGE}, 0.16)"),
            format!("rgba({DAMAGE}, 0.8)"),
        ),
        (None, Side::Player) => (
            "rgba(255, 255, 255, 0.03)".into(),
            format!("rgba({PLAYER}, 0.5)"),
        ),
        (None, Side::Enemy) => (
            "rgba(255, 255, 255, 0.03)".into(),
            format!("rgba({ENEMY}, 0.45)"),
        ),
    };
    format!(
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="3" fill="{fill}" stroke="{stroke}" />"#
    )
}

/// Ein Schuss als gestrichelte Linie mit Spitze.
fn shot(from: Point, to: Point) -> String {
    format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="rgb({DAMAGE})" stroke-width="1.5" stroke-dasharray="5 4" marker-end="url(#head-shot)" />"#,
        from.0, from.1, to.0, to.1
    )
}

/// Ein Manöver als Pfad mit Spitze in der Auswahlfarbe.
fn movement(d: &str, width: f32) -> String {
    format!(
        r#"<path d="{d}" fill="none" stroke="rgb({SELECT})" stroke-width="{width}" marker-end="url(#head-move)" />"#
    )
}

type Point = (f32, f32);

/// Das Brett: oben der Gegner (Command, Strike), unten die eigene Seite (Strike, Command). Eigene Command-Schiffe hinter
/// einem Strike-Schiff sind gedeckt, die anderen ungedeckt; zwei gegnerische Schüsse fallen in eigene Spalten.
fn board() -> String {
    let mut svg = Svg::new(640.0, 316.0);
    let (slot_width, step, slot_height, x0) = (80.0, 86.0, 46.0, 100.0);
    let center = |index: usize| x0 + index as f32 * step + slot_width / 2.0;
    let rows = [
        (10.0, Side::Enemy, false),
        (64.0, Side::Enemy, true),
        (150.0, Side::Player, true),
        (204.0, Side::Player, false),
    ];
    let middle = |y: f32| y + slot_height / 2.0;
    // Schiffe je Seite: (Strike, Command).
    let ships = |side: Side| -> (&[usize], &[usize]) {
        match side {
            Side::Player => (&[1, 3], &[1, 4]),
            Side::Enemy => (&[2], &[2]),
        }
    };
    svg.push(
        r#"<line x1="0" y1="130" x2="640" y2="130" stroke="rgba(255, 255, 255, 0.12)" stroke-dasharray="4 4" />"#
            .into(),
    );
    for (y, side, strike) in rows {
        let (strike_ships, command_ships) = ships(side);
        let lane = if strike { strike_ships } else { command_ships };
        for index in 0..6 {
            let covered = (!strike && lane.contains(&index)).then(|| {
                if strike_ships.contains(&index) {
                    Highlight::Covered
                } else {
                    Highlight::Exposed
                }
            });
            svg.push(slot(
                x0 + index as f32 * step,
                y,
                slot_width,
                slot_height,
                side,
                covered,
            ));
            if lane.contains(&index) {
                svg.push(ship(center(index), middle(y), side, false));
            }
        }
    }
    let [
        (command_row, ..),
        (strike_row, ..),
        (player_strike, ..),
        (player_command, ..),
    ] = rows;
    svg.push(shot(
        (center(2) + 8.0, middle(strike_row) + 12.0),
        (center(4) - 4.0, player_command - 2.0),
    ));
    svg.push(shot(
        (center(2) - 8.0, middle(command_row) + 12.0),
        (center(1) + 4.0, player_strike - 2.0),
    ));
    svg.finish()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Row {
    Command,
    Strike,
    Own,
}

/// Ein Slot in einer Spalte der Feuerbefehl-Figur: Reihe und Nummer.
type Spot = (Row, usize);

/// Feuerbefehle im Kampf, vier Fälle im Raster: oben Spaltenfeuer (trifft das vordere Schiff, nach dessen Fall das
/// dahinter; verpufft, wenn das Ziel die Spalte verlässt), unten Schiffsfeuer (folgt seinem Schiff; verpufft, solange
/// es gedeckt ist).
fn fire_orders() -> String {
    let mut svg = Svg::new(640.0, 334.0);
    let (slot_width, step, slot_height) = (58.0, 64.0, 30.0);
    let row_y = |row: Row| match row {
        Row::Command => 32.0,
        Row::Strike => 66.0,
        Row::Own => 126.0,
    };
    // Wo die Slots eines Falls liegen (`edge` −1 oben, 1 unten, 0 Mitte).
    let at = move |index: usize| {
        let cx = 160.0 + (index % 2) as f32 * 320.0;
        let oy = (index / 2) as f32 * 176.0;
        let ox = cx - step - slot_width / 2.0;
        move |(row, slot): Spot, edge: f32| -> Point {
            (
                ox + slot as f32 * step + slot_width / 2.0,
                oy + row_y(row) + (1.0 + edge) * slot_height / 2.0,
            )
        }
    };
    let mut panel = |index: usize, ships: &[Spot], hit: &[Spot], covered: Option<Spot>| {
        let place = at(index);
        for row in [Row::Command, Row::Strike, Row::Own] {
            for slot_index in 0..3 {
                let spot = (row, slot_index);
                let highlight = if hit.contains(&spot) {
                    Some(Highlight::Exposed)
                } else {
                    (covered == Some(spot)).then_some(Highlight::Covered)
                };
                let (x, _) = place(spot, 0.0);
                let (_, y) = place(spot, -1.0);
                let side = if row == Row::Own {
                    Side::Player
                } else {
                    Side::Enemy
                };
                svg.push(slot(
                    x - slot_width / 2.0,
                    y,
                    slot_width,
                    slot_height,
                    side,
                    highlight,
                ));
            }
        }
        for &spot in ships {
            let (x, y) = place(spot, 0.0);
            let side = if spot.0 == Row::Own {
                Side::Player
            } else {
                Side::Enemy
            };
            svg.push(ship(x, y, side, false));
        }
    };
    // Spaltenfeuer, zwei Schüsse in dieselbe Spalte: Der erste zerstört das Schiff vorn, der zweite trifft das dahinter.
    panel(
        0,
        &[(Row::Command, 1), (Row::Own, 0), (Row::Own, 2)],
        &[(Row::Strike, 1), (Row::Command, 1)],
        None,
    );
    // Spaltenfeuer, das Ziel manövriert aus der Spalte.
    panel(1, &[(Row::Strike, 2), (Row::Own, 1)], &[], None);
    // Schiffsfeuer folgt dem Schiff in den neuen Slot.
    panel(
        2,
        &[(Row::Strike, 2), (Row::Own, 1)],
        &[(Row::Strike, 2)],
        None,
    );
    // Schiffsfeuer auf ein gedecktes Schiff verpufft.
    panel(
        3,
        &[(Row::Command, 1), (Row::Strike, 1), (Row::Own, 0)],
        &[],
        Some((Row::Command, 1)),
    );

    let shift = |(x, y): Point, dx: f32, dy: f32| (x + dx, y + dy);
    // Spaltenfeuer: gepunkteter Rahmen um beide Slots der Spalte.
    let column = |(x, top): Point, (_, bottom): Point| {
        format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="5" {ORDER} />"#,
            x - slot_width / 2.0 - 3.0,
            top - 3.0,
            slot_width + 6.0,
            bottom - top + 6.0
        )
    };
    // Schiffsfeuer: gepunkteter Ring um das verfolgte Schiff.
    let ring = |(x, y): Point| format!(r#"<circle cx="{x}" cy="{}" r="16" {ORDER} />"#, y + 2.0);
    // Ein Schuss, der verpufft: blass, mit Kreuz statt Spitze, `stop` vor dem Ziel.
    let fizzle = |(from_x, from_y): Point, (to_x, to_y): Point, stop: f32| {
        let ratio = 1.0 - stop / (to_x - from_x).hypot(to_y - from_y);
        let (x, y) = (
            from_x + (to_x - from_x) * ratio,
            from_y + (to_y - from_y) * ratio,
        );
        format!(
            r#"<g opacity="0.45" stroke="rgb({DAMAGE})" fill="none"><line x1="{from_x}" y1="{from_y}" x2="{x}" y2="{y}" stroke-width="1.5" stroke-dasharray="5 4" /><path d="M{},{}L{},{}M{},{}L{},{}" stroke-width="2" /></g>"#,
            x - 5.0,
            y - 5.0,
            x + 5.0,
            y + 5.0,
            x + 5.0,
            y - 5.0,
            x - 5.0,
            y + 5.0
        )
    };
    // Ein in diesem Kampf zerstörtes Schiff: matt und durchgestrichen.
    let wreck = |(x, y): Point| {
        let mut out = ship(x, y, Side::Enemy, true);
        for dx in [-9.0, 9.0] {
            write!(
                out,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="rgb({DAMAGE})" stroke-width="2" stroke-linecap="round" />"#,
                x + dx,
                y - 9.0,
                x - dx,
                y + 9.0
            )
            .unwrap();
        }
        out
    };
    // Das Manöver eines gegnerischen Schiffs innerhalb der Lane, als flacher Bogen über den Slots.
    let lane_move = |(from_x, y): Point, (to_x, _): Point| {
        movement(
            &format!(
                "M{},{} Q{},{} {},{}",
                from_x + 8.0,
                y - 6.0,
                (from_x + to_x) / 2.0,
                y - 22.0,
                to_x - 14.0,
                y - 6.0
            ),
            1.8,
        )
    };

    let through = at(0);
    svg.push(column(
        through((Row::Command, 1), -1.0),
        through((Row::Strike, 1), 1.0),
    ));
    svg.push(wreck(through((Row::Strike, 1), 0.0)));
    svg.push(shot(
        through((Row::Own, 0), -1.0),
        shift(through((Row::Strike, 1), 1.0), -10.0, 0.0),
    ));
    svg.push(shot(
        through((Row::Own, 2), -1.0),
        shift(through((Row::Command, 1), 1.0), 8.0, 0.0),
    ));

    let dodge = at(1);
    svg.push(column(
        dodge((Row::Command, 1), -1.0),
        dodge((Row::Strike, 1), 1.0),
    ));
    svg.push(lane_move(
        dodge((Row::Strike, 1), 0.0),
        dodge((Row::Strike, 2), 0.0),
    ));
    svg.push(fizzle(
        dodge((Row::Own, 1), -1.0),
        dodge((Row::Strike, 1), 0.3),
        0.0,
    ));

    let follows = at(2);
    svg.push(lane_move(
        follows((Row::Strike, 1), 0.0),
        follows((Row::Strike, 2), 0.0),
    ));
    svg.push(ring(follows((Row::Strike, 2), 0.0)));
    svg.push(shot(
        follows((Row::Own, 1), -1.0),
        follows((Row::Strike, 2), 1.0),
    ));

    let covered = at(3);
    // Das Kreuz bleibt knapp vor dem Ring, damit beide zu sehen sind.
    svg.push(ring(covered((Row::Command, 1), 0.0)));
    svg.push(fizzle(
        covered((Row::Own, 0), -1.0),
        shift(covered((Row::Command, 1), 0.0), 0.0, 2.0),
        24.0,
    ));
    svg.finish()
}

/// Strich des Feuerbefehls: dünn gepunktet in der eigenen Farbe.
const ORDER: &str = r#"fill="none" stroke="rgba(120, 194, 255, 0.8)" stroke-width="1.5" stroke-dasharray="1.5 3.5" stroke-linecap="round""#;

/// Das Grundmanöver im Bild eines Zugs: ein Schiff wechselt aus der Command- in die Strike-Lane.
fn turn_maneuver() -> String {
    let mut svg = Svg::new(140.0, 196.0);
    svg.push(slot(20.0, 120.0, 100.0, 50.0, Side::Player, None));
    svg.push(slot(20.0, 26.0, 100.0, 50.0, Side::Player, None));
    svg.push(ship(70.0, 145.0, Side::Player, true));
    svg.push(ship(70.0, 51.0, Side::Player, false));
    svg.push(movement("M100,118 C130,90 130,56 112,52", 2.5));
    svg.finish()
}

/// Die Feuerbefehle im Bild eines Zugs: zwei eigene Schiffe feuern über Kreuz auf zwei gegnerische.
fn turn_fire() -> String {
    let mut svg = Svg::new(140.0, 196.0);
    for x in [8.0, 76.0] {
        svg.push(slot(x, 26.0, 56.0, 50.0, Side::Enemy, None));
        svg.push(slot(x, 120.0, 56.0, 50.0, Side::Player, None));
        svg.push(ship(x + 28.0, 51.0, Side::Enemy, false));
        svg.push(ship(x + 28.0, 145.0, Side::Player, false));
    }
    svg.push(shot((40.0, 116.0), (100.0, 80.0)));
    svg.push(shot((100.0, 116.0), (40.0, 80.0)));
    svg.finish()
}
