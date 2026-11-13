//! Das Gefecht: ein Flaggschiff gegen die gegnerische Flotte, Runde für Runde aus Spielerzug, Gegnerzug und Auflösung
//! (`docs/gefecht.md`). Im Spielerzug wirkt alles sofort; jeder Schritt liefert seine Ereignisse.
//! Die Vorschau rechnet auf einer Kopie, also genau wie die Ausführung.

use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::admirals::{AdmiralId, AdmiralStation, Station};
use crate::board::{COLUMNS, EnemySlot, MAX_ENEMIES};
use crate::cards::{CardId, CardTarget, CardType, DeckCard};
use crate::data::{Step, rules};
use crate::enemies::{Action, EnemyClass, EnemyId, Unit, action, step_index};
use crate::events::{
    BattleEvent, Breakdown, DestroyCause, Hit, IdleReason, Intent, MissReason, UnitRef, UnitStats,
};
use crate::modules::ModuleId;

/// Was der Run dem Gefecht mitgibt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSetup {
    pub admiral: AdmiralId,
    pub hull: i32,
    pub max_hull: i32,
    pub deck: Vec<DeckCard>,
    pub modules: Vec<ModuleId>,
    /// Das Modul Notaggregat ist da und im Run noch nicht verbraucht.
    pub emergency_generator_available: bool,
    pub encounter: Encounter,
}

/// Ein Verband, wie er zu Beginn auf dem Brett steht: je Reihe die drei Spalten von links nach rechts. So steht
/// er auch an den Knoten der Sternenkarten (`assets/data/maps/`), jede Einheit mit allen ihren Werten.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub front: [Option<Unit>; COLUMNS],
    #[serde(default)]
    pub back: [Option<Unit>; COLUMNS],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reinforcements: Vec<Reinforcement>,
    /// Gegenflotte: Fällt eine Eskorte, rückt die nächste in ihren Slot nach.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replacements: Vec<Unit>,
}

impl Encounter {
    /// Die Einheiten samt Slot in der Reihenfolge von [`EnemySlot::ALL`].
    pub fn units(&self) -> impl Iterator<Item = (&Unit, EnemySlot)> {
        let units = self.front.iter().chain(&self.back);
        units
            .zip(EnemySlot::ALL)
            .filter_map(|(unit, slot)| Some((unit.as_ref()?, slot)))
    }
}

/// Angesagte Verstärkung: trifft im Gegnerzug der Runde `round` ein.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reinforcement {
    pub round: u32,
    pub unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    None,
    Enemy(u32),
    Column(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleResult {
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleError {
    BattleOver,
    NotEnoughPoints,
    /// Das Ziel passt nicht zur Karte oder Station, steht nicht oder ist die eigene Spalte.
    TargetInvalid,
    AdmiralStationInEmergency,
    Jumped,
    /// Diese Station lief in dieser Runde schon.
    StationUsed,
    /// Alle Stationsplätze der Runde sind verbraucht.
    NoStationSlot,
}

/// Das Flaggschiff des Spielers samt dem Zustand dieser Runde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flagship {
    pub admiral: AdmiralId,
    pub hull: i32,
    pub max_hull: i32,
    pub column: usize,
    pub burning: bool,
    /// Schild für den folgenden Gegnerzug; verfällt in der Auflösung.
    pub shield: i32,
    /// Ablativpanzerung (Rest des Gefechts), ohne Panzerplatten.
    pub ablative: i32,
    /// Vorstoß (nur Vance).
    pub momentum: i32,
    pub jumped: bool,
    /// Wie viele Stationen in dieser Runde laufen dürfen (Alarmstufe Rot +1).
    pub station_slots: usize,
    /// Die Stationen, die in dieser Runde liefen, in Reihenfolge (für Nachsetzen und Vances Vorstoß).
    pub stations_used: Vec<Station>,
    /// Energiestoß und Überladung in dieser Runde.
    pub attack_bonus: i32,
    /// Überladung: Schaden zum Zugende.
    pub overcharge: i32,
    /// Feuerleitdoktrin: Zuschlag auf Schüsse und Angriffskarten.
    pub fire_control: i32,
    /// Alarmbereitschaft: Schild zu Beginn jedes Gegnerzugs.
    pub battle_stations: i32,
    /// Gegenmaßnahmen: so viele gegnerische Angriffe im Gegnerzug halbiert.
    pub countermeasures: u32,
    /// Zielstörer: ATK-Abzug aller gegnerischen Schüsse im Gegnerzug.
    pub jammer: i32,
    /// Standhalten: Hülle, die jeder Schütze verliert, der das Flaggschiff trifft.
    pub hold_the_line: i32,
    pub counterfire: bool,
    /// Fokusfeuer: im folgenden Gegnerzug je Treffer 2 mehr.
    pub focus_vulnerable: bool,
}

impl Flagship {
    pub fn emergency(&self) -> bool {
        at_most(self.hull, self.max_hull, rules().battle.emergency_percent)
    }

    pub fn stats(&self) -> UnitStats {
        UnitStats {
            hull: self.hull,
            shield: self.shield,
        }
    }
}

/// Eine gegnerische Einheit. `intents` sind die nächsten beiden Aktionen, `step` der nächste Schritt von `pattern`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enemy {
    pub uid: u32,
    pub id: EnemyId,
    pub slot: EnemySlot,
    pub hull: i32,
    pub max_hull: i32,
    /// ATK vor Gefechtsuhr und Spotter.
    pub attack: i32,
    /// Fester Abzug je Treffer.
    pub armor: i32,
    pub pattern: Vec<Step>,
    pub burning: bool,
    pub shield: i32,
    pub charged: bool,
    pub intents: [Action; 2],
    pub step: u32,
    /// In dieser Runde vom Spieler getroffen (Plünderer weichen dann aus).
    pub hit: bool,
    pub stasis: bool,
    pub silenced: bool,
    /// In dieser Runde eingetroffen: handelt erst in der nächsten.
    pub arrived: bool,
}

impl Enemy {
    pub fn emergency(&self) -> bool {
        at_most(self.hull, self.max_hull, rules().battle.emergency_percent)
    }

    /// Ausweichen gilt in der Runde, in der es angesagt ist, schon im Spielerzug; im Notfall weicht ein Gegner nicht
    /// mehr aus.
    pub fn is_evading(&self) -> bool {
        self.intents[0] == Action::Evade && !self.emergency()
    }

    pub fn unit(&self) -> UnitRef {
        UnitRef::Enemy {
            uid: self.uid,
            id: self.id,
            slot: self.slot,
        }
    }

    pub fn stats(&self) -> UnitStats {
        UnitStats {
            hull: self.hull,
            shield: self.shield,
        }
    }
}

/// Ein Schuss oder eine Angriffskarte des Flaggschiffs auf eine gegnerische Einheit.
#[derive(Default)]
struct Shot {
    base: i32,
    ignore_evade: bool,
    ignore_cover: bool,
    ignite: bool,
    rammed: bool,
}

/// Ob `hull` höchstens `percent` Prozent von `max_hull` ist.
fn at_most(hull: i32, max_hull: i32, percent: i32) -> bool {
    hull * 100 <= max_hull * percent
}

#[derive(Debug, Clone)]
pub struct Battle {
    pub round: u32,
    pub flagship: Flagship,
    pub enemies: Vec<Enemy>,
    pub hand: Vec<DeckCard>,
    pub draw_pile: Vec<DeckCard>,
    pub discard_pile: Vec<DeckCard>,
    /// Gespielte Doktrinen, sie wirken bis zum Gefechtsende.
    pub doctrines: Vec<DeckCard>,
    pub points: u32,
    pub kills: u32,
    pub modules: Vec<ModuleId>,
    pub emergency_generator_available: bool,
    pub emergency_generator_used: bool,
    pub reinforcements: Vec<Reinforcement>,
    pub replacements: Vec<Unit>,
    pub result: Option<BattleResult>,
    next_uid: u32,
    rng: StdRng,
}

fn half_up(value: i32) -> i32 {
    (value + 1) / 2
}

impl Battle {
    pub fn new(setup: BattleSetup, seed: u64) -> Self {
        let admiral = setup.admiral;
        let mut battle = Battle {
            round: 1,
            flagship: Flagship {
                admiral,
                hull: setup.hull,
                max_hull: setup.max_hull,
                column: COLUMNS / 2,
                burning: false,
                shield: 0,
                ablative: 0,
                momentum: 0,
                jumped: false,
                station_slots: 1,
                stations_used: Vec::new(),
                attack_bonus: 0,
                overcharge: 0,
                fire_control: 0,
                battle_stations: 0,
                countermeasures: 0,
                jammer: 0,
                hold_the_line: 0,
                counterfire: false,
                focus_vulnerable: false,
            },
            enemies: Vec::new(),
            hand: Vec::new(),
            draw_pile: setup.deck,
            discard_pile: Vec::new(),
            doctrines: Vec::new(),
            points: rules().battle.command_points,
            kills: 0,
            emergency_generator_available: setup.emergency_generator_available,
            emergency_generator_used: false,
            reinforcements: setup.encounter.reinforcements.clone(),
            replacements: setup.encounter.replacements.clone(),
            result: None,
            next_uid: 1,
            rng: StdRng::seed_from_u64(seed),
            modules: setup.modules,
        };
        if battle.has(ModuleId::ReserveReactor) {
            battle.points = rules().modules.reserve_reactor_points;
        }
        let mut events = Vec::new();
        battle.draw_pile.shuffle(&mut battle.rng);
        battle.draw(battle.hand_size(), &mut events);
        for (unit, slot) in setup.encounter.units() {
            battle.spawn(unit, slot, false, &mut events);
        }
        battle
    }

    // --- Abfragen ---

    pub fn has(&self, module: ModuleId) -> bool {
        self.modules.contains(&module)
    }

    pub fn enemy(&self, uid: u32) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.uid == uid)
    }

    /// Eine Back-Einheit ist gedeckt, wenn in ihrer Spalte vorn eine Einheit steht.
    pub fn covered(&self, uid: u32) -> bool {
        self.enemy(uid).is_some_and(|e| self.front_of(e).is_some())
    }

    /// Gefechtsuhr: ATK-Zuschlag der Gegner in dieser Runde.
    pub fn clock(&self) -> i32 {
        self.round.saturating_sub(rules().battle.clock_start - 1) as i32
    }

    /// Panzerung des Flaggschiffs: Ablativpanzerung und Panzerplatten.
    pub fn flagship_armor(&self) -> i32 {
        self.flagship.ablative
            + self.has(ModuleId::ArmorPlates) as i32 * rules().modules.armor_plates_armor
    }

    /// ATK der Waffen-Station samt allen Boni.
    pub fn flagship_attack(&self) -> i32 {
        self.shot_attack(false)
    }

    /// Effektive ATK einer gegnerischen Einheit: Grundwert, Gefechtsuhr, Spotter, Zielstörer, Notfall.
    pub fn enemy_attack(&self, enemy: &Enemy) -> i32 {
        let spotters = self
            .enemies
            .iter()
            .filter(|e| e.id == EnemyId::Spotter && !e.silenced);
        let attack = enemy.attack + self.clock() + spotters.count() as i32 - self.flagship.jammer;
        let attack = attack.max(0);
        if enemy.emergency() {
            half_up(attack)
        } else {
            attack
        }
    }

    /// Die beiden kommenden Absichten einer Einheit, der Schaden live aus dem aktuellen Zustand.
    pub fn announced(&self, uid: u32) -> [Intent; 2] {
        let enemy = self.enemy(uid).expect("Einheit steht");
        let [first, second] = enemy.intents;
        // Funkstille und Stillstand lassen die erste Absicht ausfallen.
        let idle = enemy.silenced || enemy.stasis;
        let charged_after =
            enemy.charged && (idle || !fires(first)) || !idle && first == Action::Charge;
        [
            Intent {
                action: first,
                damage: if idle {
                    0
                } else {
                    self.action_damage(enemy, first, enemy.charged)
                },
            },
            Intent {
                action: second,
                damage: self.action_damage(enemy, second, charged_after),
            },
        ]
    }

    fn hand_size(&self) -> usize {
        if self.has(ModuleId::SupplyPod) {
            rules().modules.supply_pod_hand
        } else {
            rules().battle.hand_size
        }
    }

    fn index(&self, uid: u32) -> Option<usize> {
        self.enemies.iter().position(|e| e.uid == uid)
    }

    fn front_of(&self, enemy: &Enemy) -> Option<u32> {
        let front = enemy.slot.front()?;
        self.enemies.iter().find(|e| e.slot == front).map(|e| e.uid)
    }

    fn shot_attack(&self, focus: bool) -> i32 {
        let f = &self.flagship;
        let momentum = if focus { 2 * f.momentum } else { f.momentum };
        let attack = f.admiral.attack()
            + momentum
            + f.attack_bonus
            + f.fire_control
            + self.has(ModuleId::FireControlComputer) as i32
                * rules().modules.fire_control_computer_attack;
        if f.emergency() {
            half_up(attack)
        } else {
            attack
        }
    }

    fn action_damage(&self, enemy: &Enemy, action: Action, charged: bool) -> i32 {
        let attack = self.enemy_attack(enemy);
        match action {
            Action::Fire => attack * (1 + charged as i32),
            Action::ColumnFire { .. } => {
                attack * (1 + (charged || enemy.id == EnemyId::TorpedoWing) as i32)
            }
            Action::Ram => rules().enemies.ram_damage,
            _ => 0,
        }
    }

    // --- Spielerzug ---

    /// Welches Ziel die Station beim Auslösen braucht.
    pub fn station_target(&self, station: Station) -> CardTarget {
        match station {
            Station::Weapons => CardTarget::Enemy,
            Station::Maneuver => CardTarget::Column,
            Station::Admiral
                if matches!(
                    self.flagship.admiral.station(),
                    AdmiralStation::FocusFire | AdmiralStation::Stasis
                ) =>
            {
                CardTarget::Enemy
            }
            _ => CardTarget::None,
        }
    }

    /// Ob die Station in dieser Runde noch laufen darf, abgesehen vom Ziel.
    pub fn station_open(&self, station: Station) -> Result<(), RuleError> {
        let f = &self.flagship;
        if self.result.is_some() {
            return Err(RuleError::BattleOver);
        }
        if f.jumped {
            return Err(RuleError::Jumped);
        }
        if f.stations_used.contains(&station) {
            return Err(RuleError::StationUsed);
        }
        if f.stations_used.len() >= f.station_slots {
            return Err(RuleError::NoStationSlot);
        }
        if station == Station::Admiral && f.emergency() {
            return Err(RuleError::AdmiralStationInEmergency);
        }
        Ok(())
    }

    pub fn play_card(
        &mut self,
        hand_index: usize,
        target: Target,
    ) -> Result<Vec<BattleEvent>, RuleError> {
        self.check_card(hand_index, target)?;
        let card = self.hand.remove(hand_index);
        self.points -= card.cost();
        if card.id.card_type() == CardType::Doctrine {
            self.doctrines.push(card);
        } else {
            self.discard_pile.push(card);
        }
        let mut events = vec![BattleEvent::CardPlayed { card, target }];
        self.apply_card(card, target, &mut events);
        Ok(events)
    }

    /// Führt die Station sofort aus, mit dem Ziel aus `station_target`.
    pub fn run_station(
        &mut self,
        station: Station,
        target: Target,
    ) -> Result<Vec<BattleEvent>, RuleError> {
        self.station_open(station)?;
        self.check_target(self.station_target(station), target)?;
        let mut events = Vec::new();
        self.execute_station(station, target, &mut events);
        Ok(events)
    }

    /// Zugende (Überladung), Gegnerzug, Auflösung.
    pub fn end_turn(&mut self) -> Vec<BattleEvent> {
        let mut events = Vec::new();
        if self.result.is_some() {
            return events;
        }
        if self.flagship.overcharge > 0 {
            let was = self.flagship.emergency();
            let amount = self.flagship.overcharge.min(self.flagship.hull);
            self.flagship.hull -= amount;
            events.push(BattleEvent::SelfDamage {
                amount,
                after: self.flagship.stats(),
            });
            self.flagship_changed(was, &mut events);
        }
        if self.result.is_none() {
            self.enemy_turn(&mut events);
        }
        if self.result.is_none() {
            self.resolution(&mut events);
        }
        events
    }

    // --- Vorschau ---

    pub fn preview_station(&self, station: Station, target: Target) -> Result<Vec<Hit>, RuleError> {
        let mut copy = self.clone();
        copy.run_station(station, target).map(hits)
    }

    pub fn preview_card(&self, hand_index: usize, target: Target) -> Result<Vec<Hit>, RuleError> {
        let mut copy = self.clone();
        copy.play_card(hand_index, target).map(hits)
    }

    // --- Stationen und Karten ---

    fn execute_station(&mut self, station: Station, target: Target, events: &mut Vec<BattleEvent>) {
        let uid = match target {
            Target::Enemy(uid) => uid,
            _ => 0,
        };
        self.flagship.stations_used.push(station);
        events.push(BattleEvent::StationTriggered { station });
        match station {
            Station::Weapons => self.fire(uid, false, events),
            Station::Shields => {
                self.shield_flagship(self.flagship.admiral.shield_station(), events)
            }
            Station::DamageControl => self.repair(rules().stations.damage_control_repair, events),
            Station::Maneuver => {
                let Target::Column(column) = target else {
                    unreachable!("geprüft")
                };
                self.maneuver(column, events);
            }
            Station::Admiral => match self.flagship.admiral.station() {
                AdmiralStation::Counterfire => {
                    self.flagship.counterfire = true;
                    self.shield_flagship(rules().stations.counterfire_shield, events);
                }
                AdmiralStation::FocusFire => {
                    self.fire(uid, true, events);
                    self.flagship.focus_vulnerable = true;
                }
                AdmiralStation::Stasis => {
                    let i = self.index(uid).expect("geprüft");
                    self.enemies[i].stasis = true;
                }
                AdmiralStation::FollowUp => {
                    let amount = 2 * self.enemy_burn_damage();
                    for uid in self.uids() {
                        let Some(i) = self.index(uid) else { continue };
                        if !self.enemies[i].burning {
                            continue;
                        }
                        self.enemies[i].hit = true;
                        self.drain(
                            i,
                            amount,
                            DestroyCause::Shot,
                            events,
                            |target, lost, after| BattleEvent::Hit {
                                source: UnitRef::Flagship,
                                target,
                                breakdown: Breakdown {
                                    base: amount,
                                    hull: lost,
                                    lethal: after.hull == 0,
                                    ..Breakdown::default()
                                },
                                after,
                            },
                        );
                    }
                }
            },
        }
    }

    /// Waffen-Station oder Fokusfeuer auf `uid`.
    fn fire(&mut self, uid: u32, focus: bool, events: &mut Vec<BattleEvent>) {
        let admiral = self.flagship.admiral;
        let shot = Shot {
            base: self.shot_attack(focus),
            ignore_evade: focus || admiral == AdmiralId::Kerr,
            ignore_cover: admiral == AdmiralId::Kerr,
            ignite: admiral == AdmiralId::Reyes,
            rammed: false,
        };
        self.shoot(uid, shot, events);
    }

    fn check_card(&self, hand_index: usize, target: Target) -> Result<(), RuleError> {
        if self.result.is_some() {
            return Err(RuleError::BattleOver);
        }
        let card = self.hand[hand_index];
        if card.cost() > self.points {
            return Err(RuleError::NotEnoughPoints);
        }
        if self.flagship.jumped
            && matches!(
                card.id,
                CardId::MicroJump | CardId::EvasiveAction | CardId::CollisionCourse
            )
        {
            return Err(RuleError::Jumped);
        }
        self.check_target(card.id.target(), target)?;
        if card.id == CardId::TractorBeam
            && let Target::Enemy(uid) = target
            && let Some(enemy) = self.enemy(uid)
            && (enemy.id.class() == EnemyClass::Flagship || self.tractor_slot(enemy).is_none())
        {
            return Err(RuleError::TargetInvalid);
        }
        Ok(())
    }

    /// Ob das Ziel zur Art passt: ein stehender Gegner oder eine andere Spalte als die eigene.
    fn check_target(&self, kind: CardTarget, target: Target) -> Result<(), RuleError> {
        let valid = match (kind, target) {
            (CardTarget::None, Target::None) => true,
            (CardTarget::Enemy, Target::Enemy(uid)) => self.enemy(uid).is_some(),
            (CardTarget::Column, Target::Column(column)) => {
                column < COLUMNS && column != self.flagship.column
            }
            _ => false,
        };
        valid.then_some(()).ok_or(RuleError::TargetInvalid)
    }

    fn apply_card(&mut self, card: DeckCard, target: Target, events: &mut Vec<BattleEvent>) {
        let value = card.value();
        let uid = match target {
            Target::Enemy(uid) => uid,
            _ => 0,
        };
        let target_enemy = self.enemy(uid).cloned();
        let plain = |base| Shot {
            base,
            ..Shot::default()
        };
        match card.id {
            CardId::Torpedo => {
                let base = self.card_damage(value, uid);
                self.shoot(
                    uid,
                    Shot {
                        ignore_evade: true,
                        ..plain(base)
                    },
                    events,
                );
            }
            CardId::OrbitalStrike => {
                let base = self.card_damage(value, uid);
                self.shoot(
                    uid,
                    Shot {
                        ignore_cover: true,
                        ..plain(base)
                    },
                    events,
                );
            }
            CardId::HeatSeeker => {
                let evading = target_enemy.expect("geprüft").is_evading();
                let base = self.card_damage(
                    value
                        + if evading {
                            rules().cards.heat_seeker_evading_bonus
                        } else {
                            0
                        },
                    uid,
                );
                let shot = Shot {
                    ignore_evade: true,
                    ignore_cover: true,
                    ..plain(base)
                };
                self.shoot(uid, shot, events);
            }
            CardId::CoupDeGrace => {
                let emergency = target_enemy.expect("geprüft").emergency();
                let base = self.card_damage(
                    value
                        + if emergency {
                            rules().cards.coup_de_grace_emergency_bonus
                        } else {
                            0
                        },
                    uid,
                );
                self.shoot(uid, plain(base), events);
            }
            CardId::Firebomb => {
                let base = self.card_damage(value, uid);
                self.shoot(
                    uid,
                    Shot {
                        ignite: true,
                        ..plain(base)
                    },
                    events,
                );
            }
            CardId::VolleyFire | CardId::Barrage | CardId::Bombardment => {
                for uid in self.uids() {
                    let Some(enemy) = self.enemy(uid) else {
                        continue;
                    };
                    let shot = match card.id {
                        CardId::VolleyFire => Shot {
                            ignore_cover: true,
                            ..Shot::default()
                        },
                        CardId::Barrage if enemy.is_evading() => Shot {
                            ignore_evade: true,
                            ..Shot::default()
                        },
                        CardId::Bombardment if enemy.slot.front().is_some() => Shot {
                            ignore_cover: true,
                            ..Shot::default()
                        },
                        _ => continue,
                    };
                    let base = self.card_damage(value, uid);
                    self.shoot(uid, Shot { base, ..shot }, events);
                }
            }
            CardId::CollisionCourse => {
                let enemy = target_enemy.expect("geprüft");
                let enemy_attack = self.enemy_attack(&enemy);
                events.push(BattleEvent::Rammed {
                    target: enemy.unit(),
                });
                let base = 2 * self.flagship_attack() + value + self.follow_up_bonus(&enemy);
                let shot = Shot {
                    ignore_evade: true,
                    ignore_cover: true,
                    rammed: true,
                    base,
                    ..Shot::default()
                };
                self.shoot(uid, shot, events);
                self.hit_flagship(enemy.unit(), enemy_attack, false, false, events);
            }
            CardId::EmergencyShields => self.shield_flagship(value, events),
            CardId::HoldTheLine => {
                self.flagship.hold_the_line += rules().cards.hold_the_line_retaliation;
                self.shield_flagship(value, events);
            }
            CardId::AblativeArmor => self.flagship.ablative += value,
            CardId::Overcharge => {
                self.flagship.attack_bonus += value;
                self.flagship.overcharge += rules().cards.overcharge_self_damage;
            }
            CardId::PowerSurge => self.flagship.attack_bonus += value,
            CardId::RepairCrew => self.repair(value, events),
            CardId::EmpBlast => {
                let i = self.index(uid).expect("geprüft");
                self.enemies[i].shield = 0;
                self.enemies[i].charged = false;
            }
            CardId::TargetingJammer => self.flagship.jammer += 1,
            CardId::RadioSilence => {
                let i = self.index(uid).expect("geprüft");
                self.enemies[i].silenced = true;
                self.enemies[i].charged = false;
            }
            CardId::TractorBeam => {
                let i = self.index(uid).expect("geprüft");
                let from = self.enemies[i].slot;
                self.enemies[i].slot = self.tractor_slot(&self.enemies[i]).expect("geprüft");
                events.push(BattleEvent::Tractored {
                    unit: self.enemies[i].unit(),
                    from,
                });
            }
            CardId::MicroJump => {
                self.flagship.jumped = true;
                events.push(BattleEvent::Jumped);
            }
            CardId::EvasiveAction => {
                let Target::Column(column) = target else {
                    unreachable!("geprüft")
                };
                self.maneuver(column, events);
            }
            CardId::Countermeasures => self.flagship.countermeasures += 1,
            CardId::Scavenge => self.draw(rules().cards.scavenge_draw, events),
            CardId::RedAlert => self.flagship.station_slots += 1,
            CardId::FireControlDoctrine => self.flagship.fire_control += value,
            CardId::BattleStations => self.flagship.battle_stations += value,
        }
    }

    /// Schaden einer Angriffskarte samt Feuerleitdoktrin und Nachsetzen.
    fn card_damage(&self, value: i32, uid: u32) -> i32 {
        let enemy = self.enemy(uid).expect("Ziel steht");
        value + self.flagship.fire_control + self.follow_up_bonus(enemy)
    }

    /// Zuschlag der Feuerleitdoktrin auf den Kartenwert, wie ihn die Hand zeigt (Rammkurs rechnet über die ATK).
    pub fn card_bonus(&self, card: CardId) -> i32 {
        if card.card_type() == CardType::Attack && card != CardId::CollisionCourse {
            self.flagship.fire_control
        } else {
            0
        }
    }

    /// Nachsetzen gibt Angriffskarten auf brennende Ziele +2, sobald es in dieser Runde lief.
    fn follow_up_bonus(&self, enemy: &Enemy) -> i32 {
        let ran = self.flagship.admiral.station() == AdmiralStation::FollowUp
            && self.flagship.stations_used.contains(&Station::Admiral);
        if ran && enemy.burning {
            rules().stations.follow_up_card_bonus
        } else {
            0
        }
    }

    /// Der nächstgelegene freie Slot in der Reihe der Einheit (bei Gleichstand links).
    fn tractor_slot(&self, enemy: &Enemy) -> Option<EnemySlot> {
        (0..COLUMNS)
            .map(|column| EnemySlot::new(enemy.slot.row, column))
            .filter(|slot| self.enemies.iter().all(|e| e.slot != *slot))
            .min_by_key(|slot| slot.column.abs_diff(enemy.slot.column))
    }

    fn shield_flagship(&mut self, amount: i32, events: &mut Vec<BattleEvent>) {
        self.flagship.shield += amount;
        let after = self.flagship.stats();
        events.push(BattleEvent::Shielded {
            target: UnitRef::Flagship,
            amount,
            after,
        });
    }

    fn repair(&mut self, amount: i32, events: &mut Vec<BattleEvent>) {
        let f = &mut self.flagship;
        if f.burning {
            f.burning = false;
            events.push(BattleEvent::Extinguished {
                target: UnitRef::Flagship,
            });
        }
        let was = f.emergency();
        let amount = amount.min(f.max_hull - f.hull);
        f.hull += amount;
        events.push(BattleEvent::Healed {
            target: UnitRef::Flagship,
            amount,
            after: f.stats(),
        });
        self.flagship_changed(was, events);
    }

    fn maneuver(&mut self, to: usize, events: &mut Vec<BattleEvent>) {
        let from = self.flagship.column;
        self.flagship.column = to;
        events.push(BattleEvent::Maneuvered { from, to });
    }

    // --- Schaden ---

    /// Schuss auf eine gegnerische Einheit: Ausweichen, Deckung, Schild, Panzerung (`docs/gefecht.md`).
    fn shoot(&mut self, uid: u32, shot: Shot, events: &mut Vec<BattleEvent>) {
        let Some(i) = self.index(uid) else { return };
        let enemy = &self.enemies[i];
        let mut rest = shot.base;
        let evaded = if !shot.ignore_evade && enemy.is_evading() {
            rest / 2
        } else {
            0
        };
        rest -= evaded;
        let front = if shot.ignore_cover {
            None
        } else {
            self.front_of(enemy)
        };
        let covered = front.map(|front| (front, half_up(rest)));
        if let Some((_, part)) = covered {
            rest -= part;
        }
        let cause = if shot.rammed {
            DestroyCause::Rammed
        } else {
            DestroyCause::Shot
        };
        let breakdown = Breakdown {
            base: shot.base,
            evaded,
            covered,
            ..Breakdown::default()
        };
        self.land(uid, breakdown, rest, shot.ignite, cause, events);
        if let Some((front, part)) = covered {
            let breakdown = Breakdown {
                base: part,
                ..Breakdown::default()
            };
            self.land(front, breakdown, part, false, cause, events);
        }
    }

    fn land(
        &mut self,
        uid: u32,
        mut breakdown: Breakdown,
        rest: i32,
        ignite: bool,
        cause: DestroyCause,
        events: &mut Vec<BattleEvent>,
    ) {
        let Some(i) = self.index(uid) else { return };
        let enemy = &mut self.enemies[i];
        let was = enemy.emergency();
        breakdown.shielded = enemy.shield.min(rest);
        breakdown.armor = enemy.armor.min(rest - breakdown.shielded);
        breakdown.hull = (rest - breakdown.shielded - breakdown.armor).min(enemy.hull);
        enemy.shield -= breakdown.shielded;
        enemy.hull -= breakdown.hull;
        enemy.hit = true;
        breakdown.lethal = enemy.hull == 0;
        let ignite = ignite
            || (breakdown.hull > 0
                && at_most(enemy.hull, enemy.max_hull, rules().battle.burn_percent));
        events.push(BattleEvent::Hit {
            source: UnitRef::Flagship,
            target: enemy.unit(),
            breakdown,
            after: enemy.stats(),
        });
        self.aftermath(i, was, ignite, cause, events);
    }

    /// Hülle ohne Abzüge (Brand, Gegenfeuer, Standhalten, Nachsetzen); `event` meldet den Verlust.
    fn drain(
        &mut self,
        i: usize,
        amount: i32,
        cause: DestroyCause,
        events: &mut Vec<BattleEvent>,
        event: impl FnOnce(UnitRef, i32, UnitStats) -> BattleEvent,
    ) {
        let enemy = &mut self.enemies[i];
        let was = enemy.emergency();
        let lost = amount.min(enemy.hull);
        enemy.hull -= lost;
        events.push(event(enemy.unit(), lost, enemy.stats()));
        self.aftermath(i, was, false, cause, events);
    }

    fn aftermath(
        &mut self,
        i: usize,
        was_emergency: bool,
        ignite: bool,
        cause: DestroyCause,
        events: &mut Vec<BattleEvent>,
    ) {
        let enemy = &mut self.enemies[i];
        if enemy.hull == 0 {
            return self.destroy(i, cause, events);
        }
        if ignite && !enemy.burning {
            enemy.burning = true;
            events.push(BattleEvent::Ignited {
                target: enemy.unit(),
            });
        }
        if enemy.emergency() != was_emergency {
            events.push(BattleEvent::Emergency {
                target: enemy.unit(),
                active: !was_emergency,
            });
        }
    }

    fn destroy(&mut self, i: usize, cause: DestroyCause, events: &mut Vec<BattleEvent>) {
        let enemy = self.enemies.remove(i);
        events.push(BattleEvent::Destroyed {
            target: enemy.unit(),
            cause,
        });
        self.kills += 1;
        if enemy.id == EnemyId::Seal {
            return self.end(BattleResult::Won, events);
        }
        for king in self
            .enemies
            .iter_mut()
            .filter(|e| e.id == EnemyId::ScrapKing)
        {
            let was = king.emergency();
            let amount = rules()
                .enemies
                .cannibalize_heal
                .min(king.max_hull - king.hull);
            king.hull += amount;
            events.push(BattleEvent::Cannibalized {
                unit: king.unit(),
                amount,
                after: king.stats(),
            });
            if king.emergency() != was {
                events.push(BattleEvent::Emergency {
                    target: king.unit(),
                    active: !was,
                });
            }
        }
        if !self.replacements.is_empty() {
            let unit = self.replacements.remove(0);
            self.spawn(&unit, enemy.slot, true, events);
        }
        self.check_won(events);
    }

    /// Treffer am Flaggschiff. `shields`: Gegenmaßnahmen und Schild zählen (nur im Gegnerzug).
    fn hit_flagship(
        &mut self,
        source: UnitRef,
        base: i32,
        plasma: bool,
        shields: bool,
        events: &mut Vec<BattleEvent>,
    ) {
        let armor_value = self.flagship_armor();
        let f = &mut self.flagship;
        let was = f.emergency();
        let mut rest = base;
        let mut breakdown = Breakdown {
            base,
            ..Breakdown::default()
        };
        if shields && f.countermeasures > 0 {
            f.countermeasures -= 1;
            breakdown.countermeasures = rest / 2;
            rest -= rest / 2;
        }
        if shields {
            breakdown.shielded = f.shield.min(rest);
            rest -= breakdown.shielded;
            f.shield -= breakdown.shielded;
        }
        breakdown.armor = armor_value.min(rest);
        rest -= breakdown.armor;
        breakdown.hull = rest.min(f.hull);
        f.hull -= breakdown.hull;
        breakdown.lethal = f.hull == 0;
        events.push(BattleEvent::Hit {
            source,
            target: UnitRef::Flagship,
            breakdown,
            after: f.stats(),
        });
        let percent = if plasma {
            rules().enemies.plasma_burn_percent
        } else {
            rules().battle.burn_percent
        };
        let threshold = at_most(f.hull, f.max_hull, percent);
        if breakdown.hull > 0 && f.hull > 0 && threshold && !f.burning {
            f.burning = true;
            events.push(BattleEvent::Ignited {
                target: UnitRef::Flagship,
            });
        }
        self.flagship_changed(was, events);
    }

    /// Nach einer Änderung der Hülle: Notaggregat oder Niederlage bei 0, sonst Notfall ein oder aus.
    fn flagship_changed(&mut self, was_emergency: bool, events: &mut Vec<BattleEvent>) {
        if self.flagship.hull == 0 {
            if !self.emergency_generator_available || self.emergency_generator_used {
                return self.end(BattleResult::Lost, events);
            }
            self.emergency_generator_used = true;
            self.flagship.hull = 1;
            events.push(BattleEvent::EmergencyGenerator {
                after: self.flagship.stats(),
            });
        }
        if self.flagship.emergency() != was_emergency {
            events.push(BattleEvent::Emergency {
                target: UnitRef::Flagship,
                active: !was_emergency,
            });
        }
    }

    fn check_won(&mut self, events: &mut Vec<BattleEvent>) {
        // Bleiben nur Unterstützer, die keinen Schaden machen, ist das Gefecht ebenso gewonnen.
        if self.enemies.iter().all(|e| e.id.support())
            && self.reinforcements.is_empty()
            && self.replacements.is_empty()
        {
            self.end(BattleResult::Won, events);
        }
    }

    fn end(&mut self, result: BattleResult, events: &mut Vec<BattleEvent>) {
        self.result = Some(result);
        events.push(BattleEvent::BattleEnded { result });
    }

    // --- Gegnerzug ---

    fn enemy_turn(&mut self, events: &mut Vec<BattleEvent>) {
        for enemy in &mut self.enemies {
            enemy.shield = 0;
        }
        if self.flagship.battle_stations > 0 {
            self.shield_flagship(self.flagship.battle_stations, events);
        }
        let round = self.round;
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.reinforcements)
            .into_iter()
            .partition(|r| r.round <= round);
        self.reinforcements = later;
        for Reinforcement { unit, .. } in due {
            self.reinforce(&unit, events);
        }
        for uid in self.uids() {
            let Some(i) = self.index(uid) else { continue };
            let enemy = &self.enemies[i];
            let unit = enemy.unit();
            if enemy.arrived {
                continue;
            }
            if enemy.stasis || enemy.silenced {
                let reason = if enemy.stasis {
                    IdleReason::Stasis
                } else {
                    IdleReason::Silenced
                };
                events.push(BattleEvent::Idle { unit, reason });
                continue;
            }
            let action = enemy.intents[0];
            events.push(BattleEvent::EnemyAction { unit, action });
            match action {
                Action::Fire | Action::ColumnFire { .. } | Action::Ram => {
                    let focus = self.flagship.focus_vulnerable as i32
                        * rules().stations.focus_fire_vulnerability;
                    let damage = self.action_damage(enemy, action, enemy.charged) + focus;
                    let plasma = enemy.id == EnemyId::PlasmaCruiser;
                    self.enemies[i].charged = false;
                    let missed = match action {
                        _ if self.flagship.jumped => Some(MissReason::Jumped),
                        Action::ColumnFire { columns } if !columns[self.flagship.column] => {
                            Some(MissReason::Maneuvered)
                        }
                        _ => None,
                    };
                    match missed {
                        Some(reason) => events.push(BattleEvent::Missed {
                            source: unit,
                            reason,
                        }),
                        None => self.hit_flagship(unit, damage, plasma, true, events),
                    }
                    if self.result.is_some() {
                        return;
                    }
                    if action == Action::Ram {
                        self.enemies.remove(i);
                        events.push(BattleEvent::FireshipSpent { unit });
                        self.check_won(events);
                    } else if missed.is_none() {
                        self.retaliate(uid, events);
                    }
                }
                Action::Shields(amount) => {
                    let enemy = &mut self.enemies[i];
                    enemy.shield += amount;
                    events.push(BattleEvent::Shielded {
                        target: unit,
                        amount,
                        after: enemy.stats(),
                    });
                }
                Action::Charge => self.enemies[i].charged = true,
                Action::Evade | Action::TargetLock => {}
                // Trifft sofort ein und steht damit in der nächsten Runde auf dem Brett. Die Ansage ist der Schritt
                // `step - 2` des Musters; dort steht die gerufene Einheit mit ihren Werten.
                Action::Call(_) => {
                    let enemy = &self.enemies[i];
                    let step = &enemy.pattern[step_index(&enemy.pattern, enemy.step - 2)];
                    if let Step::Call(unit) = step {
                        let unit = unit.clone();
                        self.reinforce(&unit, events);
                    }
                }
                Action::Bolt => {
                    for enemy in &mut self.enemies {
                        enemy.shield += rules().enemies.bolt_shield;
                        let (target, after) = (enemy.unit(), enemy.stats());
                        events.push(BattleEvent::Shielded {
                            target,
                            amount: rules().enemies.bolt_shield,
                            after,
                        });
                    }
                }
            }
            if self.result.is_some() {
                return;
            }
        }
    }

    /// Gegenfeuer und Standhalten am Schützen, der das Flaggschiff getroffen hat.
    fn retaliate(&mut self, uid: u32, events: &mut Vec<BattleEvent>) {
        let cause = DestroyCause::Retaliation;
        if self.flagship.counterfire {
            let i = self.index(uid).expect("Schütze steht");
            self.drain(
                i,
                rules().stations.counterfire_damage,
                cause,
                events,
                |target, amount, after| BattleEvent::Counterfire {
                    target,
                    amount,
                    after,
                },
            );
        }
        let amount = self.flagship.hold_the_line;
        if let (Some(i), true) = (self.index(uid), amount > 0) {
            self.drain(i, amount, cause, events, |target, amount, after| {
                BattleEvent::HoldTheLine {
                    target,
                    amount,
                    after,
                }
            });
        }
    }

    // --- Auflösung ---

    fn resolution(&mut self, events: &mut Vec<BattleEvent>) {
        let burn = self.burn_damage(UnitRef::Flagship);
        let f = &mut self.flagship;
        if f.burning && burn > 0 {
            let was = f.emergency();
            let amount = burn.min(f.hull);
            f.hull -= amount;
            let target = UnitRef::Flagship;
            events.push(BattleEvent::BurnTick {
                target,
                amount,
                after: f.stats(),
            });
            self.flagship_changed(was, events);
            if self.result.is_some() {
                return;
            }
        }
        let amount = self.enemy_burn_damage();
        for uid in self.uids() {
            let Some(i) = self.index(uid).filter(|&i| self.enemies[i].burning) else {
                continue;
            };
            self.drain(
                i,
                amount,
                DestroyCause::BurnedOut,
                events,
                |target, amount, after| BattleEvent::BurnTick {
                    target,
                    amount,
                    after,
                },
            );
            if self.result.is_some() {
                return;
            }
        }

        let f = &mut self.flagship;
        f.shield = 0;
        if f.jumped {
            f.jumped = false;
            events.push(BattleEvent::Returned);
        }
        if f.admiral == AdmiralId::Vance {
            let pressed = !f.stations_used.is_empty()
                && f.stations_used
                    .iter()
                    .all(|s| matches!(s, Station::Weapons | Station::Admiral));
            f.momentum = if pressed {
                (f.momentum + 1).min(rules().stations.max_momentum)
            } else {
                0
            };
        }
        f.stations_used.clear();
        f.station_slots = 1;
        f.attack_bonus = 0;
        f.overcharge = 0;
        f.countermeasures = 0;
        f.jammer = 0;
        f.hold_the_line = 0;
        f.counterfire = false;
        f.focus_vulnerable = false;

        self.round += 1;
        events.push(BattleEvent::RoundStarted { round: self.round });
        if self.clock() > 0 {
            events.push(BattleEvent::Clock {
                bonus: self.clock(),
            });
        }

        let column = self.flagship.column;
        for enemy in &mut self.enemies {
            if !enemy.stasis && !enemy.arrived {
                enemy.intents = [enemy.intents[1], action(&enemy.pattern, enemy.step, column)];
                enemy.step += 1;
                if enemy.id == EnemyId::Raiders && enemy.hit {
                    enemy.intents[0] = Action::Evade;
                }
            }
            enemy.hit = false;
            enemy.stasis = false;
            enemy.silenced = false;
            enemy.arrived = false;
        }
        for uid in self.uids() {
            self.push_intents(uid, events);
        }

        self.discard_pile.append(&mut self.hand);
        self.draw(self.hand_size(), events);
        self.points = rules().battle.command_points;
    }

    /// Was ein Brand der Einheit je Auflösung kostet; 0, wenn er nicht tickt (Brandschutz).
    pub fn burn_damage(&self, unit: UnitRef) -> i32 {
        match unit {
            UnitRef::Flagship if self.modules.contains(&ModuleId::FireSuppression) => 0,
            UnitRef::Flagship => rules().battle.burn_damage,
            UnitRef::Enemy { .. } => self.enemy_burn_damage(),
        }
    }

    fn enemy_burn_damage(&self) -> i32 {
        if self.flagship.admiral == AdmiralId::Reyes {
            rules().battle.ember_burn_damage
        } else {
            rules().battle.burn_damage
        }
    }

    // --- Hilfen ---

    /// Die Kennungen aller Einheiten in Handlungsreihenfolge: Front vor Back, links nach rechts.
    fn uids(&self) -> Vec<u32> {
        let mut units: Vec<_> = self
            .enemies
            .iter()
            .map(|e| (e.slot.order(), e.uid))
            .collect();
        units.sort();
        units.into_iter().map(|(_, uid)| uid).collect()
    }

    /// Stellt eine Einheit auf und lässt sie ihre ersten beiden Absichten ansagen.
    /// Stellt Verstärkung in den ersten freien Slot, solange die Grenze nicht erreicht ist.
    fn reinforce(&mut self, unit: &Unit, events: &mut Vec<BattleEvent>) {
        let free = EnemySlot::ALL
            .into_iter()
            .find(|s| self.enemies.iter().all(|e| e.slot != *s));
        if let (Some(slot), true) = (free, self.enemies.len() < MAX_ENEMIES) {
            self.spawn(unit, slot, true, events);
        }
    }

    fn spawn(
        &mut self,
        unit: &Unit,
        slot: EnemySlot,
        arrived: bool,
        events: &mut Vec<BattleEvent>,
    ) {
        let pattern = unit.pattern.clone();
        let column = self.flagship.column;
        let uid = self.next_uid;
        self.next_uid += 1;
        self.enemies.push(Enemy {
            uid,
            id: unit.id,
            slot,
            hull: unit.hull,
            max_hull: unit.hull,
            attack: unit.attack,
            armor: unit.armor,
            burning: false,
            shield: 0,
            charged: false,
            intents: [action(&pattern, 0, column), action(&pattern, 1, column)],
            pattern,
            step: 2,
            hit: false,
            stasis: false,
            silenced: false,
            arrived,
        });
        if arrived {
            let unit = self.enemies.last().expect("eben gestellt").unit();
            events.push(BattleEvent::Arrived { unit });
            self.push_intents(uid, events);
        }
    }

    fn push_intents(&self, uid: u32, events: &mut Vec<BattleEvent>) {
        let unit = self.enemy(uid).expect("steht").unit();
        events.push(BattleEvent::Intents {
            unit,
            intents: self.announced(uid),
        });
    }

    /// Zieht Karten; ist der Nachziehstapel leer, wird der Ablagestapel gemischt.
    fn draw(&mut self, count: usize, events: &mut Vec<BattleEvent>) {
        let mut cards = Vec::new();
        for _ in 0..count {
            if self.draw_pile.is_empty() {
                if self.discard_pile.is_empty() {
                    break;
                }
                self.draw_pile.append(&mut self.discard_pile);
                self.draw_pile.shuffle(&mut self.rng);
                events.push(BattleEvent::Shuffled);
            }
            cards.push(self.draw_pile.pop().expect("nicht leer"));
        }
        self.hand.extend_from_slice(&cards);
        events.push(BattleEvent::Drawn { cards });
    }
}

fn fires(action: Action) -> bool {
    matches!(action, Action::Fire | Action::ColumnFire { .. })
}

fn hits(events: Vec<BattleEvent>) -> Vec<Hit> {
    events
        .into_iter()
        .filter_map(|event| match event {
            BattleEvent::Hit {
                target, breakdown, ..
            } => Some(Hit { target, breakdown }),
            _ => None,
        })
        .collect()
}
