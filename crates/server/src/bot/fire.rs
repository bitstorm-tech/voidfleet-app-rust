//! Die Zielwahl im Kampf ([`aim_fire`]): je Schütze eine Spalte oder ein Schiff der Gegnerseite. Der Bot zielt so
//! selbst und erwartet es so vom Gegner.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use voidfleet_core::admirals::{ABILITY_VALUES, PowerId};
use voidfleet_core::board::{Board, LaneType, SLOT_COUNT, Side, SlotRef};
use voidfleet_core::cards::ShipId;

use super::deploy::guard_target;
use super::threat::{Guard, Lead, Risk};
use super::{
    EVADE, FLAGSHIP_WEIGHT, KILL_BONUS, POWER, UNSURE, card, durability, highest, is_flagship,
    is_token,
};

/// Ein Feuerbefehl im Probekampf: Spaltenfeuer auf eine Spalte der Gegenseite oder Schiffsfeuer auf ein gegnerisches
/// Schiff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Column(usize),
    Ship(SlotRef),
}

/// Was der Abschuss eines Schiffs mit `attack` ATK bringt, dem noch `left` Schaden fehlt, gerechnet wie die Stellung
/// (`position_score`): der Schaden, am Flaggschiff doppelt (sein Fall entscheidet), ein Bonus je Schiff außer Tokens und
/// sein Feuer in den Folgerunden.
pub(super) fn kill_value(board: &Board, at: SlotRef, left: i32, attack: i32) -> f64 {
    let flagship = is_flagship(board, at);
    let value = left * if flagship { FLAGSHIP_WEIGHT } else { 1 }
        + if is_token(board, at) { 0 } else { KILL_BONUS }
        + attack * POWER.future;
    f64::from(value) + if flagship { super::DECISIVE } else { 0.0 }
}

/// Die Schützen für einen Abschuss, dem `need` Schaden fehlt, mit möglichst wenig verschenktem Schaden: der
/// schwächste, der allein reicht, sonst der stärkste und weiter so; `None`, wenn alle zusammen nicht reichen. `able`
/// ist nach Schaden absteigend sortiert.
fn killers(able: &[SlotRef], need: i32, damage: impl Fn(SlotRef) -> i32) -> Option<Vec<SlotRef>> {
    let mut pool = able.to_vec();
    let mut chosen = Vec::new();
    let mut left = need;
    while left > 0 {
        if let Some(&finisher) = pool.iter().rev().find(|&&at| damage(at) >= left) {
            chosen.push(finisher);
            return Some(chosen);
        }
        if pool.is_empty() {
            return None;
        }
        let strongest = pool.remove(0);
        left -= damage(strongest);
        chosen.push(strongest);
    }
    Some(chosen)
}

/// Je Spalte die gegnerischen Schiffe, die stehen, wenn die Salven beginnen (vorne zuerst), und die Schützen, die darauf
/// feuern.
#[derive(Debug, Clone, Default)]
struct Column {
    stack: Vec<SlotRef>,
    volley: Vec<SlotRef>,
}

/// Ein möglicher Schritt der Zielwahl: Schützen auf eine Spalte, was der Spalte danach am wenigsten fehlt (beim
/// Abschuss 0) und was er je Schaden bringt.
struct Step {
    shooters: Vec<SlotRef>,
    rest: f64,
    column: usize,
    density: f64,
}

/// Was die Zielwahl über beide Seiten weiß, wenn die Salven beginnen.
struct Aiming<'a> {
    board: &'a Board,
    risk: &'a dyn Fn(SlotRef) -> Risk,
    /// ATK einmal je Schiff: [`Board::attack`] rechnet Aura und Speerspitze bei jedem Aufruf neu.
    attack: HashMap<SlotRef, i32>,
    /// Was jedem gegnerischen Schiff bis zum Abschuss fehlt, nach Railgun und Schwarmangriff.
    left: HashMap<SlotRef, i32>,
    /// Kampfreihenfolge der eigenen Schützen: Taktvorsprung zuerst, dann nach Slots.
    rank: HashMap<SlotRef, usize>,
}

impl Aiming<'_> {
    fn damage(&self, shooter: SlotRef, hit: SlotRef) -> i32 {
        let focus = self.board.ship(hit).powers.contains(&PowerId::FocusFire);
        self.attack[&shooter] + if focus { ABILITY_VALUES.focus_fire } else { 0 }
    }

    /// Was den Schiffen von `stack` (vorne zuerst) nach den Schüssen `shots` in Kampfreihenfolge noch fehlt; `hits`
    /// erfährt jeden Treffer.
    fn rest(
        &self,
        stack: &[SlotRef],
        shots: &[SlotRef],
        mut hits: Option<&mut Vec<(SlotRef, SlotRef)>>,
    ) -> Vec<i32> {
        let mut remaining: Vec<i32> = stack.iter().map(|at| self.left[at]).collect();
        let mut order = shots.to_vec();
        // Steht nur ein Schiff in der Spalte, spielt die Reihenfolge keine Rolle.
        if stack.len() > 1 || hits.is_some() {
            order.sort_by_key(|at| self.rank[at]);
        }
        for shooter in order {
            let standing =
                |remaining: &[i32], index: usize| index < stack.len() && remaining[index] > 0;
            // Wie in der Auflösung: das vorderste Schiff, das noch steht; Flaggschiffe schießen an einem Trugbild
            // vorbei.
            let past = standing(&remaining, 1)
                && is_flagship(self.board, shooter)
                && card(self.board, stack[0]) == ShipId::Decoy;
            let index = if standing(&remaining, 0) && !past {
                0
            } else if standing(&remaining, 1) {
                1
            } else {
                continue;
            };
            remaining[index] -= self.damage(shooter, stack[index]);
            if let Some(hits) = hits.as_deref_mut() {
                hits.push((shooter, stack[index]));
            }
        }
        remaining
    }

    /// Was die Treffer in `stack` bringen: je Abschuss [`kill_value`], sonst der Schaden, am Flaggschiff doppelt;
    /// unsichere Ziele halb.
    fn worth(&self, stack: &[SlotRef], after: &[i32]) -> f64 {
        stack
            .iter()
            .zip(after)
            .map(|(&at, &after)| {
                let need = self.left[&at];
                let gain = if after <= 0 {
                    kill_value(self.board, at, need, self.attack[&at])
                } else {
                    let weight = if is_flagship(self.board, at) {
                        FLAGSHIP_WEIGHT
                    } else {
                        1
                    };
                    f64::from((need - after) * weight)
                };
                gain * if (self.risk)(at).unsure { UNSURE } else { 1.0 }
            })
            .sum()
    }

    /// Die Schützen aus `pool`, die zusammen mit `volley` das Schiff `stack[target]` fällen, auch durch das Schiff
    /// davor, mit möglichst wenig verschenktem Schaden: erst wie [`killers`] nach dem Schaden, der bis dahin fehlt,
    /// dann nachgelegt, falls Overkill vorne einen Schuss mehr kostet, zuletzt ohne die Entbehrlichen, die stärksten
    /// zuerst; `None`, wenn alle zusammen nicht reichen. `now`: was der Spalte nach `volley` noch fehlt.
    fn killing(
        &self,
        stack: &[SlotRef],
        volley: &[SlotRef],
        target: usize,
        now: &[i32],
        pool: &[SlotRef],
    ) -> Option<Vec<SlotRef>> {
        // Steht nur ein Schiff in der Spalte, reicht der Schaden, der ihm fehlt.
        if stack.len() == 1 {
            return killers(pool, now[0], |at| self.damage(at, stack[0]));
        }
        let falls =
            |shots: &[SlotRef]| self.rest(stack, &[volley, shots].concat(), None)[target] <= 0;
        let need = now[..=target].iter().map(|&value| value.max(0)).sum();
        let first = killers(pool, need, |at| self.attack[&at])?;
        let mut chosen = first.clone();
        for &extra in pool.iter().filter(|at| !first.contains(at)) {
            if falls(&chosen) {
                break;
            }
            chosen.push(extra);
        }
        if !falls(&chosen) {
            return None;
        }
        for shooter in chosen.clone() {
            let without: Vec<SlotRef> =
                chosen.iter().copied().filter(|&at| at != shooter).collect();
            if !without.is_empty() && falls(&without) {
                chosen = without;
            }
        }
        Some(chosen)
    }
}

/// Die Feuerbefehle, die die Zielwahl bisher vergeben hat, je Spalte die Salven darauf und die freien Schützen.
struct Orders {
    orders: HashMap<SlotRef, Target>,
    columns: Vec<Column>,
    free: Vec<SlotRef>,
}

impl Orders {
    fn assign(&mut self, shooters: &[SlotRef], column: usize) {
        for &shooter in shooters {
            self.orders.insert(shooter, Target::Column(column));
        }
        self.columns[column].volley.extend_from_slice(shooters);
        self.free.retain(|at| !shooters.contains(at));
    }
}

/// Die Zielwahl von `side` im Kampf auf `board`, der Stellung zu Kampfbeginn, wie die Seite sie erwartet: je Schütze
/// eine Spalte oder ein Schiff der Gegnerseite. Schützen sind ihre Kampfschiffe, die feuern können, Ziele für
/// Schiffsfeuer die gegnerischen Schiffe, beide nur, wenn sie beim Planen standen (`planned`). Gezielt wird erst wie mit
/// Spaltenfeuer, das in dieser Stellung nie weniger bringt: Jeder Schuss trifft das vorderste Schiff der Spalte, das
/// dann noch steht (Flaggschiffe schießen am Trugbild vorbei), in Kampfreihenfolge (Taktvorsprung zuerst, dann nach
/// Slots), sodass Overkill auf das vordere Schiff auf das dahinter durchgeht; vorher feuert die Railgun, und Drohnen
/// fliegen den Schwarmangriff ohnehin (ihr Feuerbefehl gilt der Spalte ihres Ziels, falls Funkstille ihn aufhebt). So:
///
/// 1. Fällt das gegnerische Flaggschiff, feuern so viele auf seine Spalte, wie es dafür braucht, auch durch das Schiff
///    davor: Das entscheidet das Match. Mit `lead` (Vorhalten) fällen es die übrigen, soweit sie reichen, auch in den
///    gedeckten Spalten, in die es wohl ausweicht, per Spaltenfeuer durch die Deckung, die wahrscheinlichste zuerst;
///    mit `lead.gamble` geht sie der Spalte vor, in der es steht. Ungedeckt auszuweichen rettet es nur vor
///    Spaltenfeuer, und kann es nur das, folgt ihm schon der Abschuss per Schiffsfeuer (siehe 4.).
/// 2. Ein Schiff mit Taktvorsprung schießt einen gegnerischen Schützen vorne in einer Spalte ab, bevor der feuert.
/// 3. Dann Schritt für Schritt, was je Schaden am meisten bringt: ein Abschuss ([`kill_value`]), auch des Schiffs
///    hinter einem, das dabei fällt, mit möglichst wenig verschenktem Schaden (`killing`), oder ein Treffer ohne
///    Abschuss, am Flaggschiff doppelt; bei Gleichstand der Abschuss, sonst das Ziel, das danach am wenigsten übrig
///    hat. Ein Ziel, das wohl jedem Schuss entgeht ([`Risk::unsure`]), zählt halb.
/// 4. Schiffsfeuer: Was dabei fällt oder schwer getroffen wird ([`EVADE`]), weicht wohl aus. Kann es das nur ungedeckt
///    ([`Risk::open`]) und legt der Gegner nichts davor ([`guard_target`]: vor das ungedeckte Command-Schiff, auf das
///    dieser Plan am meisten feuert, zuerst vor das Flaggschiff; kann davor nur Moreaus Trugbild landen, setzt er beim
///    Flaggschiff mit `lead.gamble` auf das Ausweichen), zielt jeder Schuss, der es trifft, auf das Schiff statt auf
///    die Spalte: Er trifft dasselbe, folgt ihm aber in jede Spalte. So bricht Spaltenfeuer die Deckung, und
///    Schiffsfeuer trifft das Schiff dahinter.
/// 5. Wer dann noch übrig ist, weil alles in Reichweite schon fällt, zielt zur Sicherheit auf das wertvollste Ziel, am
///    liebsten das Flaggschiff, wie in 4. per Schiffs- oder Spaltenfeuer (der Schuss verpufft, wenn es fällt, und
///    trifft, wenn ihn ein Schild rettet oder ein frisch gelegtes Schiff davor die ersten Schüsse nimmt).
///
/// Gibt je Schütze das Ziel seines Feuerbefehls zurück.
pub(super) fn aim_fire(
    board: &Board,
    side: Side,
    planned: &dyn Fn(SlotRef) -> bool,
    risk: &dyn Fn(SlotRef) -> Risk,
    lead: Option<&Lead>,
) -> HashMap<SlotRef, Target> {
    let enemy = side.opposite();
    let own: Vec<SlotRef> = board.ships(side).collect();
    let ships: Vec<SlotRef> = board.ships(enemy).collect();
    let attack: HashMap<SlotRef, i32> = own
        .iter()
        .chain(&ships)
        .map(|&at| (at, board.attack(at)))
        .collect();
    let mut left: HashMap<SlotRef, i32> = ships
        .iter()
        .map(|&at| (at, durability(board, at)))
        .collect();
    let marked = ships.iter().copied().find(|&at| board.ship(at).marked);
    if let Some(marked) = marked.filter(|&at| board.is_exposed(at, true)) {
        *left.get_mut(&marked).expect("gegnerisches Schiff") -= ABILITY_VALUES.railgun;
    }
    let mut orders = HashMap::new();
    let shooters: Vec<SlotRef> = own
        .iter()
        .copied()
        .filter(|&at| board.can_attack(at))
        .collect();
    let swarmed = ships
        .iter()
        .copied()
        .find(|&at| board.ship(at).powers.contains(&PowerId::SwarmStrike));
    let drones: Vec<SlotRef> = match swarmed {
        Some(_) => shooters
            .iter()
            .copied()
            .filter(|&at| card(board, at) == ShipId::Drone)
            .collect(),
        None => Vec::new(),
    };
    if let Some(swarmed) = swarmed {
        let focus = board.ship(swarmed).powers.contains(&PowerId::FocusFire);
        for &drone in &drones {
            let damage = attack[&drone] + if focus { ABILITY_VALUES.focus_fire } else { 0 };
            *left.get_mut(&swarmed).expect("gegnerisches Schiff") -=
                damage + ABILITY_VALUES.swarm_strike;
            if planned(drone) {
                orders.insert(drone, Target::Column(swarmed.slot_index));
            }
        }
    }
    let rank = own
        .iter()
        .enumerate()
        .map(|(index, &at)| {
            let head_start = board.ship(at).powers.contains(&PowerId::HeadStart);
            (at, if head_start { 0 } else { own.len() } + index)
        })
        .collect();
    let mut free: Vec<SlotRef> = shooters
        .iter()
        .copied()
        .filter(|&at| planned(at) && !drones.contains(&at))
        .collect();
    free.sort_by_key(|at| Reverse(attack[at]));
    let columns = (0..SLOT_COUNT)
        .map(|index| Column {
            stack: LaneType::ALL
                .into_iter()
                .map(|lane| SlotRef::new(enemy, lane, index))
                .filter(|at| board.is_occupied(*at) && left[at] > 0)
                .collect(),
            volley: Vec::new(),
        })
        .collect();
    let aiming = Aiming {
        board,
        risk,
        attack,
        left,
        rank,
    };
    let mut plan = Orders {
        orders,
        columns,
        free,
    };
    // Schützen, die vorhalten: Sie zielen auf das Ausweichen des Flaggschiffs und behalten ihren Modus.
    let mut leading = HashSet::new();

    // 1. Das Flaggschiff, wenn es fällt, auch durch das Schiff davor; mit Vorhalten auch hinter der Deckung, hinter die
    //    es wohl ausweicht. Ungedeckt auszuweichen rettet es nur vor Spaltenfeuer: Kann es nur das, folgt ihm der
    //    Abschuss per Schiffsfeuer (siehe 4.), sonst weicht es in Deckung aus.
    let flagship = ships.iter().copied().find(|&at| is_flagship(board, at));
    if let Some(flagship) = flagship.filter(|at| plan.columns[at.slot_index].stack.contains(at)) {
        let strike = |column: usize| SlotRef::new(enemy, LaneType::Strike, column);
        let stay = flagship.slot_index;
        let away: Vec<usize> = lead
            .map(|lead| lead.escapes.as_slice())
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|&column| {
                column != stay
                    && board.is_occupied(strike(column))
                    && aiming.left[&strike(column)] > 0
            })
            .collect();
        let order: Vec<usize> = if lead.is_some_and(|lead| lead.gamble) {
            away.iter()
                .take(1)
                .copied()
                .chain([stay])
                .chain(away.iter().skip(1).copied())
                .collect()
        } else {
            [stay].into_iter().chain(away).collect()
        };
        for column in order {
            let stack = if column == stay {
                plan.columns[column].stack.clone()
            } else {
                vec![strike(column), flagship]
            };
            let target = stack
                .iter()
                .position(|&at| at == flagship)
                .expect("im Stapel");
            let now = aiming.rest(&stack, &[], None);
            let Some(shooters) = aiming.killing(&stack, &[], target, &now, &plan.free) else {
                continue;
            };
            plan.assign(&shooters, column);
            if column != stay {
                leading.extend(shooters);
            }
        }
    }

    // 2. Taktvorsprung: ein gegnerischer Schütze vorne in einer Spalte, auf die noch niemand feuert, den er allein
    //    fällt.
    let head_starts: Vec<SlotRef> = plan
        .free
        .iter()
        .copied()
        .filter(|&at| board.ship(at).powers.contains(&PowerId::HeadStart))
        .collect();
    for shooter in head_starts {
        let prey = plan
            .columns
            .iter()
            .enumerate()
            .filter_map(|(column, entry)| {
                let hit = *entry.stack.first()?;
                (entry.volley.is_empty()
                    && board.can_attack(hit)
                    && aiming.damage(shooter, hit) >= aiming.left[&hit])
                    .then_some((hit, column))
            });
        let target = highest(prey, |&(hit, _)| {
            let attack = aiming.attack[&hit];
            kill_value(board, hit, aiming.left[&hit], attack) + f64::from(attack)
        });
        if let Some((_, column)) = target {
            plan.assign(&[shooter], column);
        }
    }

    // 3. Der nächste Schritt: Abschuss oder Treffer mit dem meisten Wert je Schaden; bei Gleichstand der mit weniger
    //    Rest.
    while let Some(step) = best_step(&aiming, &plan) {
        plan.assign(&step.shooters, step.column);
    }

    // 4. Schiffsfeuer auf das, was dabei fällt oder schwer getroffen wird und nur ungedeckt ausweichen kann. Vor das
    //    ungedeckte Command-Schiff, auf das dieser Plan am meisten feuert, legt der Gegner wohl sein nächstes Schiff.
    let mut after = HashMap::new();
    let mut hits = Vec::new();
    for column in &plan.columns {
        let remaining = aiming.rest(&column.stack, &column.volley, Some(&mut hits));
        after.extend(column.stack.iter().copied().zip(remaining));
    }
    let rest_of = |at: SlotRef| after.get(&at).copied().unwrap_or(aiming.left[&at]);
    let dealt = |at: SlotRef| durability(board, at) - rest_of(at);
    let guardable: Vec<SlotRef> = ships
        .iter()
        .copied()
        .filter(|&at| risk(at).guard.is_some())
        .collect();
    let guarded = guard_target(board, &guardable, dealt);
    let gamble = lead.is_some_and(|lead| lead.gamble);
    let follows = |at: SlotRef| {
        let risk = risk(at);
        let gambled = risk.guard == Some(Guard::Decoy) && is_flagship(board, at) && gamble;
        planned(at)
            && risk.open
            && (Some(at) != guarded || gambled)
            && (rest_of(at) <= 0 || dealt(at) >= EVADE.hit)
    };
    for (shooter, ship) in hits {
        if !leading.contains(&shooter) && follows(ship) {
            plan.orders.insert(shooter, Target::Ship(ship));
        }
    }

    // 5. Zur Sicherheit.
    let spare = highest(ships.iter().copied(), |&at| {
        kill_value(board, at, 0, aiming.attack[&at])
    });
    if let Some(spare) = spare {
        let target = if follows(spare) {
            Target::Ship(spare)
        } else {
            Target::Column(spare.slot_index)
        };
        for &shooter in &plan.free {
            plan.orders.insert(shooter, target);
        }
    }
    plan.orders
}

/// Der beste Schritt über alle Spalten (siehe [`aim_fire`], 3.); `None`, wenn keiner etwas bringt.
fn best_step(aiming: &Aiming, plan: &Orders) -> Option<Step> {
    let mut best: Option<Step> = None;
    for step in (0..SLOT_COUNT).flat_map(|column| steps_on(aiming, plan, column)) {
        let better = best.as_ref().is_none_or(|best| {
            step.density > best.density || (step.density == best.density && step.rest < best.rest)
        });
        if step.density > 0.0 && better {
            best = Some(step);
        }
    }
    best
}

/// Die möglichen Schritte auf eine Spalte: je stehendem Schiff ein Abschuss, dazu ein Treffer ohne Abschuss.
fn steps_on(aiming: &Aiming, plan: &Orders, column: usize) -> Vec<Step> {
    let Column { stack, volley } = &plan.columns[column];
    if stack.is_empty() || plan.free.is_empty() {
        return Vec::new();
    }
    let with = |shooters: &[SlotRef]| [volley.as_slice(), shooters].concat();
    let now = aiming.rest(stack, volley, None);
    let base = aiming.worth(stack, &now);
    let mut steps: Vec<(Vec<SlotRef>, f64)> = now
        .iter()
        .enumerate()
        .filter(|&(_, &value)| value > 0)
        .filter_map(|(target, _)| aiming.killing(stack, volley, target, &now, &plan.free))
        .map(|shooters| (shooters, 0.0))
        .collect();
    // Ein Treffer ohne Abschuss: der stärkste freie Schütze, dessen Schuss nichts fällt.
    let spares = |at: SlotRef| {
        if stack.len() == 1 {
            aiming.damage(at, stack[0]) < now[0]
        } else {
            aiming
                .rest(stack, &with(&[at]), None)
                .iter()
                .zip(&now)
                .all(|(&after, &before)| after > 0 || before <= 0)
        }
    };
    if let Some(chip) = plan.free.iter().copied().find(|&at| spares(at)) {
        let after = aiming.rest(stack, &with(&[chip]), None);
        let rest = after
            .iter()
            .zip(&now)
            .filter(|(after, before)| after < before)
            .map(|(&after, _)| f64::from(after))
            .fold(f64::INFINITY, f64::min);
        steps.push((vec![chip], rest));
    }
    steps
        .into_iter()
        .map(|(shooters, rest)| {
            let gain = aiming.worth(stack, &aiming.rest(stack, &with(&shooters), None)) - base;
            let damage: i32 = shooters.iter().map(|at| aiming.attack[at]).sum();
            Step {
                shooters,
                rest,
                column,
                density: gain / f64::from(damage),
            }
        })
        .collect()
}
