//! Die Statuszeile (Port von `buildStatusText`, `buildPlanningText`, `unitText` und `shotTexts`): Verbindung, Phase
//! des Matches und beim Planen Flotte, Hinweise des letzten Klicks, Warnung vor verpuffenden Schüssen und was die
//! Auswahl gerade erwartet — beim gewählten Schiff auch, wohin sein Schuss laut Zielvorschau geht.

use voidfleet_core::{
    admirals::PowerId,
    board::{MAX_SHIPS_PER_LANE, MAX_SHIPS_PER_SIDE, Side, SlotRef, SlotTarget},
    cards::ShipId,
    protocol::{FireTarget, Miss, ShipRef},
};

use super::planner::{Notice, Selection, hovered_aim, is_maneuvering, is_valid_target};
use crate::{
    game::{Match, RoundPreview, ShotOutcome},
    i18n::Messages,
    net::ConnectionStatus,
    scene::board::{BoardHover, all_slots},
    ui::round_log::slot_label,
};

/// Alles, was die Statuszeile beim Planen braucht.
pub struct StatusView<'a> {
    pub game: &'a Match,
    pub selection: Selection,
    pub notices: &'a [Notice],
    pub hover: BoardHover,
    /// Die Zielvorschau des gezeigten Zugs (beim gewählten Schiff mit dem Ziel unter dem Zeiger).
    pub preview: Option<&'a RoundPreview>,
    pub locale: &'a dyn Messages,
}

/// Die ganze Statuszeile; `match_id` zum Einladen eines Mitspielers.
pub fn status_line(
    connection: ConnectionStatus,
    playing_back: bool,
    match_id: &str,
    view: &StatusView,
) -> String {
    let StatusView { game, locale, .. } = *view;
    match connection {
        ConnectionStatus::Disconnected => return locale.status_disconnected().into(),
        _ if game.round == 0 => return locale.status_connecting().into(),
        _ => {}
    }
    if playing_back {
        return locale.status_resolving().into();
    }
    if game.result.is_some() {
        return locale.status_match_over().into();
    }
    let Some(opponent) = &game.opponent else {
        return match game.admiral {
            None => locale.status_choose_admiral(Some(match_id)),
            Some(_) => locale.status_waiting_for_opponent(match_id),
        };
    };
    if game.admiral.is_none() {
        return locale.status_choose_admiral(None);
    }
    if opponent.admiral.is_none() {
        return locale.status_opponent_choosing().into();
    }
    if game.submitted {
        return locale.status_submitted().into();
    }
    if game.draft.is_some() {
        return locale.status_draft().into();
    }
    let fleet = locale.status_fleet(game.board.ship_count(Side::Player), MAX_SHIPS_PER_SIDE);
    // Solange mehr als ein Schiff erlaubt ist, steht dabei, wie viele noch gelegt werden dürfen.
    let deploys = if game.deploy_limit() > 1 {
        format!(" · {}", locale.status_deploys_left(game.deploys_left()))
    } else {
        String::new()
    };
    let mut parts: Vec<String> = view
        .notices
        .iter()
        .map(|notice| view.notice(*notice))
        .collect();
    // Verpuffende Schüsse sind verschenkt; beim gewählten Schützen sagt es sein eigener Text.
    let fizzles = match (view.selection, view.preview) {
        (Selection::Unit(_), _) | (_, None) => 0,
        (_, Some(preview)) => preview
            .shots
            .iter()
            .filter(|shot| matches!(shot.outcome, ShotOutcome::Miss(_)))
            .count(),
    };
    if fizzles > 0 {
        parts.push(locale.status_fizzles(fizzles));
    }
    parts.push(view.planning_text());
    format!("{fleet}{deploys} — {}", parts.join(" "))
}

impl StatusView<'_> {
    fn ship_name(&self, slot: SlotRef) -> &'static str {
        let ship = match slot.side {
            Side::Player => self.game.ship_at(slot),
            Side::Enemy => self.game.board.card(slot),
        };
        self.locale.ship(ship.expect("dort steht ein Schiff")).name
    }

    fn power_name(&self, power: PowerId) -> &'static str {
        self.locale.power(power).name
    }

    fn notice(&self, notice: Notice) -> String {
        let locale = self.locale;
        match notice {
            Notice::CommandDropped {
                command,
                ship,
                power,
            } => {
                let lost: Vec<&str> = ship
                    .map(|ship| locale.ship(ship).name)
                    .into_iter()
                    .chain(power.map(|power| self.power_name(power)))
                    .collect();
                locale.status_command_dropped(locale.command(command).name, &lost.join(" + "))
            }
            Notice::FireCleared(ship) => locale.status_fire_cleared(locale.ship(ship).name),
            Notice::ManeuverReplacesPower(power) => {
                locale.status_maneuver_replaces_power(self.power_name(power))
            }
            Notice::PowerReplacesManeuver { power, ship } => locale
                .status_power_replaces_maneuver(self.power_name(power), locale.ship(ship).name),
            Notice::NoPowerTarget(PowerId::SwarmStrike) => {
                locale.status_no_drone(self.power_name(PowerId::SwarmStrike))
            }
            Notice::NoPowerTarget(power) => locale.status_no_power_target(self.power_name(power)),
        }
    }

    /// Wie viele Slots für die Auswahl gültige Ziele sind.
    fn valid_targets(&self) -> usize {
        all_slots()
            .filter(|&slot| is_valid_target(self.game, self.selection, slot))
            .count()
    }

    /// Was die Auswahl erwartet; ohne Auswahl, was eingeplant ist.
    fn planning_text(&self) -> String {
        let StatusView { game, locale, .. } = *self;
        match self.selection {
            Selection::Unit(unit) => return self.unit_text(unit),
            Selection::Ship(pick) => {
                let name = locale.ship(game.card_of(pick)).name;
                return if self.valid_targets() == 0 {
                    locale.status_no_free_slot(name, MAX_SHIPS_PER_LANE, MAX_SHIPS_PER_SIDE)
                } else {
                    locale.status_place_ship(name)
                };
            }
            Selection::Command { index, move_source } => {
                let command = game.command_hand[index];
                let name = locale.command(command).name;
                return if command.targets_enemy_column() {
                    locale.status_choose_enemy_column(name)
                } else if command.targets_enemy_slot() {
                    locale.status_choose_enemy_slot(name)
                } else if command.targets_enemy_ship() {
                    if self.valid_targets() == 0 {
                        locale.status_no_enemy_target(name)
                    } else {
                        locale.status_choose_enemy_ship(name)
                    }
                } else if let Some(source) = move_source {
                    locale.status_choose_destination(name, self.ship_name(source))
                } else if self.valid_targets() == 0 {
                    locale.status_no_command_target(name)
                } else {
                    locale.status_choose_command_target(name)
                };
            }
            Selection::Power => {
                let power = game.power().expect("Fähigkeit nach der Admiralswahl");
                return locale.status_choose_power_target(self.power_name(power));
            }
            Selection::Mark => return locale.status_choose_mark().into(),
            Selection::Idle => {}
        }
        if game.flagship_missing() {
            let flagship = game.flagship().expect("Flaggschiff nach der Admiralswahl");
            return locale.status_flagship_first(locale.ship(flagship).name);
        }
        let mut planned: Vec<String> = game
            .arrivals()
            .into_iter()
            .map(|arrival| locale.ship(game.card_of(arrival.pick)).name.to_owned())
            .chain(
                game.planned_command()
                    .map(|command| locale.command(command).name.to_owned()),
            )
            .collect();
        if let Some(swap) = game.planned_swap() {
            planned.push(locale.status_swap(self.ship_name(swap.from), self.ship_name(swap.to)));
        } else if let Some(maneuver) = game.planned_maneuver() {
            planned.push(locale.status_maneuver(self.ship_name(maneuver.from)));
        }
        if let (Some(power), Some(target)) = (game.power(), game.planned_power_target()) {
            planned.push(locale.status_power(self.power_name(power), &self.target_label(target)));
        }
        if let Some(mark) = game.plan().mark {
            planned.push(locale.status_mark(self.ship_name(mark.on(Side::Enemy))));
        }
        if !game.plan().fire.is_empty() {
            planned.push(locale.status_fire_orders(game.plan().fire.len()));
        }
        if planned.is_empty() {
            locale.status_idle().into()
        } else {
            locale.status_planned(&planned.join(" + "))
        }
    }

    /// Das gewählte Schiff: ob und worauf es feuern kann, ob es manövrieren kann, wohin sein Schuss laut Vorschau geht
    /// und was ein zweiter Klick darauf zurücknimmt.
    fn unit_text(&self, unit: SlotRef) -> String {
        let StatusView { game, locale, .. } = *self;
        let ship = game.ship_at(unit).expect("das gewählte Schiff steht dort");
        let name = locale.ship(ship).name;
        let maneuvering = is_maneuvering(game, unit);
        let can_maneuver = !game.maneuver_destinations(unit).is_empty();
        let mut texts = Vec::new();
        if game.can_fire(unit) {
            texts.push(locale.status_choose_fire_target(name));
            if can_maneuver {
                texts.push(
                    if ship == ShipId::Mirage {
                        locale.status_maneuver_or_swap()
                    } else {
                        locale.status_maneuver_instead()
                    }
                    .into(),
                );
            }
            texts.extend(self.shot_texts(unit));
        } else {
            texts.push(if ship.atk() == 0 {
                locale.status_no_guns(name)
            } else if maneuvering {
                locale.status_maneuvering(name)
            } else {
                let command = game
                    .planned_command()
                    .expect("ein Befehl hält das Schiff vom Feuern ab");
                locale.status_command_holds_fire(name, locale.command(command).name)
            });
            if can_maneuver {
                texts.push(
                    if maneuvering {
                        locale.status_change_maneuver()
                    } else {
                        locale.status_maneuver_hint()
                    }
                    .into(),
                );
            }
        }
        texts.push(
            if game.planned_pick_at(unit).is_some() {
                locale.status_again_unplan()
            } else if maneuvering {
                locale.status_again_maneuver()
            } else {
                locale.status_esc_cancels()
            }
            .into(),
        );
        texts.join(" ")
    }

    /// Der Feuerbefehl des gewählten Schützen: unter dem Zeiger, was ein Klick daraus macht, sonst der jetzige; dazu
    /// laut Vorschau, wen der Schuss trifft oder warum er verpufft. Die Drohne im eigenen Schwarmangriff fliegt dessen
    /// Ziel an.
    fn shot_texts(&self, unit: SlotRef) -> Vec<String> {
        let StatusView { game, locale, .. } = *self;
        let enemy_name = |target: SlotTarget| self.ship_name(target.on(Side::Enemy));
        let mut texts = Vec::new();
        match hovered_aim(game, self.selection, self.hover) {
            Some(aim) => texts.push(match game.next_fire(unit.target(), aim) {
                None => locale.status_tap_clears_fire().into(),
                Some(order) => match order.target {
                    FireTarget::Column(column) => locale.status_tap_column_fire(column + 1),
                    FireTarget::Ship(target) => locale.status_tap_track_fire(enemy_name(target)),
                },
            }),
            None => texts.extend(game.fire_order_at(unit.target()).map(
                |order| match order.target {
                    FireTarget::Column(column) => locale.status_column_fire(column + 1),
                    FireTarget::Ship(target) => locale.status_track_fire(enemy_name(target)),
                },
            )),
        }
        if game.flies_swarm(unit.target()) {
            texts.push(locale.status_swarm_fire(self.power_name(PowerId::SwarmStrike)));
        }
        // Unter dem Zeiger rechnet die Vorschau schon mit dem Feuerbefehl nach dem Klick.
        let Some(shot) = self
            .preview
            .and_then(|preview| preview.shots.iter().find(|shot| shot.shooter == Some(unit)))
        else {
            return texts;
        };
        let name = |ship: ShipRef| locale.ship(ship.card_id).name;
        match shot.outcome {
            ShotOutcome::Hit {
                ship,
                through: true,
            } => {
                texts.push(locale.status_fire_through(name(ship)));
            }
            ShotOutcome::Hit { ship, .. } if !shot.tracking => {
                texts.push(locale.status_fire_hits(name(ship)));
            }
            ShotOutcome::Hit { .. } => {}
            ShotOutcome::Miss(Miss::Column { cleared, .. }) => texts.push(
                if cleared {
                    locale.status_column_cleared()
                } else {
                    locale.status_column_empty()
                }
                .into(),
            ),
            ShotOutcome::Miss(Miss::Target { target: None, .. }) => {
                texts.push(locale.status_fire_gone().into());
            }
            ShotOutcome::Miss(Miss::Target {
                target: Some(target),
                covered,
            }) => texts.push(if covered {
                locale.status_fire_covered(name(target))
            } else {
                locale.status_fire_fizzles(name(target))
            }),
        }
        texts
    }

    /// Das Ziel einer Fähigkeit in Worten: das Schiff dort, auch ein eingeplantes, beim Trugbild der Slot.
    fn target_label(&self, target: SlotRef) -> String {
        let ship = match target.side {
            Side::Player => self.game.ship_at(target),
            Side::Enemy => self.game.board.card(target),
        };
        match ship {
            Some(ship) => self.locale.ship(ship).name.to_owned(),
            None => slot_label(self.locale, target),
        }
    }
}
