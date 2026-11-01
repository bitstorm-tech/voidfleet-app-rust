//! Englisch, die Ausgangssprache: Die Funksprüche hier sind zugleich der gesprochene Text der Aufnahmen
//! (`tools/generateAudio.ts`).

use std::borrow::Cow;

use voidfleet_core::{
    admirals::{ABILITY_VALUES as V, AdmiralId, PowerId, PowerTarget},
    board::{LaneType, Side},
    cards::{CommandId, CommandType, ShipClass, ShipId, TargetMode},
    protocol::{ErrorCode, MatchResult},
};

use super::{
    AdmiralText, CardText, Messages, Phase, PhaseText, RulesChapter, RulesSection, SettingsTab,
    amount_notes, float_signed, signed, stats,
};
use crate::{
    data::radio::RadioCue,
    settings::{AnimationSpeed, CursorId, WindowMode},
};

pub struct En;

fn section(
    heading: &'static str,
    paragraphs: Vec<Cow<'static, str>>,
    caption: Option<&'static str>,
) -> RulesSection {
    RulesSection {
        heading,
        paragraphs,
        caption,
    }
}

impl Messages for En {
    fn language_name(&self) -> &'static str {
        "English"
    }

    // --- hud ---
    fn hud_round(&self, round: u32) -> String {
        format!("Round {round}")
    }
    fn hud_new_round(&self) -> &'static str {
        "New round"
    }
    fn hud_end_turn(&self) -> &'static str {
        "End turn"
    }
    fn hud_waiting(&self) -> &'static str {
        "Waiting for opponent"
    }
    fn hud_skip(&self) -> &'static str {
        "Skip animation"
    }
    fn hud_play_computer(&self) -> &'static str {
        "Play against computer"
    }
    fn hud_invite_player(&self) -> &'static str {
        "Invite a player"
    }
    fn hud_show_log(&self) -> &'static str {
        "Show log"
    }
    fn hud_hide_log(&self) -> &'static str {
        "Hide log"
    }
    fn hud_overdrive(&self, bonus: i32) -> String {
        format!("Overdrive +{bonus}")
    }
    fn hud_overdrive_hint(&self, bonus: i32) -> String {
        format!("Overdrive: all combat ships +{bonus} ATK")
    }
    fn hud_checklist_flagship(&self) -> &'static str {
        "Deploy your flagship"
    }
    fn hud_checklist_ships(&self, planned: usize, max: usize) -> String {
        if max > 1 {
            format!("Deploy ships ({planned}/{max})")
        } else {
            "Deploy a ship".into()
        }
    }
    fn hud_checklist_command(&self) -> &'static str {
        "Play a command"
    }
    fn hud_checklist_maneuver(&self) -> &'static str {
        "Maneuver or ability"
    }
    fn hud_checklist_maneuver_only(&self) -> &'static str {
        "Maneuver"
    }
    fn hud_checklist_power(&self) -> &'static str {
        "Admiral ability"
    }
    fn hud_checklist_fire(&self, assigned: usize, shooters: usize) -> String {
        if shooters > 0 {
            format!("Assign targets ({assigned}/{shooters})")
        } else {
            "Assign targets".into()
        }
    }
    fn hud_checklist_mark(&self) -> &'static str {
        "Target lock"
    }
    fn hud_power(&self) -> &'static str {
        "Admiral ability"
    }
    fn hud_power_hint(&self) -> &'static str {
        "instead of the maneuver"
    }
    fn hud_mark_button(&self) -> &'static str {
        "Target lock"
    }
    fn hud_mark_hint(&self) -> &'static str {
        "Pendulum: lock onto an enemy ship. Next round the railgun hits it at the start of combat, if it's exposed."
    }
    fn hud_marked(&self, ship: &str) -> String {
        format!("Locked: {ship}")
    }
    fn hud_railgun(&self, ship: &str) -> String {
        format!("Railgun → {ship}")
    }
    fn hud_railgun_hint(&self, amount: i32) -> String {
        format!(
            "The railgun hits the locked ship for {amount} at the start of combat, if it's exposed."
        )
    }
    fn hud_shot_lost(&self) -> &'static str {
        "The Pendulum moves — the shot is lost."
    }
    fn hud_lock_lost(&self) -> &'static str {
        "The Pendulum moves — the lock is lost."
    }
    fn hud_under_fire(&self, ship: &str) -> String {
        format!("Enemy railgun on {ship}")
    }

    // --- admiralSelect ---
    fn admiral_select_title(&self) -> &'static str {
        "Choose your admiral"
    }
    fn admiral_select_hint(&self) -> &'static str {
        "Your choice stays hidden until both have chosen. The admiral ability replaces the maneuver, every round. If your flagship falls, you lose."
    }
    fn admiral_select_flagship(&self) -> &'static str {
        "Flagship"
    }
    fn admiral_select_power(&self) -> &'static str {
        "Admiral ability"
    }
    fn admiral_select_choose(&self) -> &'static str {
        "Choose"
    }

    // --- draft ---
    fn draft_title(&self) -> &'static str {
        "Supplies"
    }
    fn draft_hint(&self) -> &'static str {
        "Take one ship and one command into your hand; the rest goes back into the deck."
    }
    fn draft_ships(&self) -> &'static str {
        "Ships"
    }
    fn draft_commands(&self) -> &'static str {
        "Commands"
    }
    fn draft_none(&self) -> &'static str {
        "Nothing to choose — hand full or deck empty."
    }
    fn draft_confirm(&self) -> &'static str {
        "Take"
    }
    fn draft_peek(&self) -> &'static str {
        "View battlefield"
    }
    fn draft_show(&self) -> &'static str {
        "Show supplies"
    }

    // --- settings ---
    fn settings_title(&self) -> &'static str {
        "Settings"
    }
    fn settings_tab(&self, tab: SettingsTab) -> &'static str {
        match tab {
            SettingsTab::Game => "Game",
            SettingsTab::Video => "Video",
            SettingsTab::Sound => "Sound",
        }
    }
    fn settings_window_mode(&self) -> &'static str {
        "Display"
    }
    fn window_mode(&self, mode: WindowMode) -> &'static str {
        match mode {
            WindowMode::Windowed => "Window",
            WindowMode::Maximized => "Maximized",
            WindowMode::Fullscreen => "Fullscreen",
        }
    }
    fn settings_music_volume(&self) -> &'static str {
        "Music"
    }
    fn settings_effects_volume(&self) -> &'static str {
        "Sound effects"
    }
    fn settings_voice_volume(&self) -> &'static str {
        "Radio chatter"
    }
    fn settings_attack_speed(&self) -> &'static str {
        "Attack speed"
    }
    fn settings_movement_speed(&self) -> &'static str {
        "Movement speed"
    }
    fn settings_animation_speed(&self) -> &'static str {
        "Other animations"
    }
    fn settings_language(&self) -> &'static str {
        "Language"
    }
    fn settings_install(&self) -> &'static str {
        "App"
    }
    fn settings_install_button(&self) -> &'static str {
        "Install Voidfleet as an app"
    }
    fn speed(&self, speed: AnimationSpeed) -> &'static str {
        match speed {
            AnimationSpeed::Slow => "Slow",
            AnimationSpeed::Normal => "Normal",
            AnimationSpeed::Fast => "Fast",
            AnimationSpeed::VeryFast => "Very fast",
        }
    }
    fn settings_cursor(&self) -> &'static str {
        "Mouse pointer"
    }
    fn settings_cursor_size(&self) -> &'static str {
        "Pointer size"
    }
    fn cursor(&self, cursor: CursorId) -> &'static str {
        match cursor {
            CursorId::Arrow => "Toon arrow",
            CursorId::Fighter => "Fighter",
            CursorId::Delta => "Delta glider",
            CursorId::Flagship => "Flagship",
            CursorId::Needle => "Navigation needle",
            CursorId::Shard => "Crystal shard",
            CursorId::Formation => "Formation",
            CursorId::Holo => "Holo arrow",
            CursorId::Thruster => "Thruster",
            CursorId::Crosshair => "Crosshair",
        }
    }

    // --- rules ---
    fn rules_title(&self) -> &'static str {
        "Rules"
    }
    fn rules_section(&self, chapter: RulesChapter) -> RulesSection {
        match chapter {
            RulesChapter::Goal => section(
                "Goal of the game",
                vec![
                    "Voidfleet is a duel between two fleets, each led by an admiral and their flagship. You win by destroying the enemy flagship: whoever loses their flagship loses the match on the spot, and the rest of the round is skipped. If both flagships fall at the same time, for example in the same combat, the match is a draw.".into(),
                    "Before round 1 each player secretly picks one of five admirals. The admiral brings a flagship — a ship outside the decks with an ability of its own — and an admiral ability you can use every round instead of the maneuver (see “Admirals and flagships”).".into(),
                    "Both players plan their moves at the same time, without seeing each other's choice, and there is no time limit: a turn may take as long as you like. There is no energy or mana — every card in your hand is playable right away.".into(),
                    "So that no match drags on forever, Overdrive kicks in from round 8: every combat ship on both sides gets +1 ATK, and the bonus grows by one every 4 rounds (round 12, 16, …). Repairs can't keep up with that forever.".into(),
                ],
                Some("Overdrive adds attack to every combat ship on both sides, the longer the match lasts."),
            ),
            RulesChapter::Turn => section(
                "Your turn",
                vec![
                    "Each round you plan up to three things and then click “End turn”: one ship from your hand onto a free slot of your side, one command from your hand, and either one maneuver of a ship already on the board to another free slot or your admiral ability. All three are optional — you may also pass with an empty turn. On top of that you give your combat ships their fire orders: what each of them fires at in combat, an enemy column or one particular enemy ship (see “Combat”). Admiral Kerr may also lock a target with the Pendulum.".into(),
                    "In round 1 you set up your fleet: your flagship, plus up to two ships instead of one, plus a command. The flagship is mandatory — “End turn” only works once it is planned. From round 2 on it's one ship per round. Two commands change the turn itself while they are planned: Rapid Deployment lets you deploy one extra ship, Carte Blanche gives you the maneuver and your admiral ability together. Take the command back and the extra ship or the ability drops out again.".into(),
                    "A command may target a ship you deploy in the same turn — your flagship too —, for example to overcharge it or shield it immediately. Planned ships appear on the board as translucent ghosts; a command on a ghost is resolved as if the ship already stood there, and a ghost can get a fire order too.".into(),
                    "Nothing is final until you end your turn: everything you planned can be taken back or changed (see “Controls”). Your opponent only ever sees your turn once both have submitted.".into(),
                ],
                Some("Placing a ship: tap the card, then one of the glowing slots on your side."),
            ),
            RulesChapter::Admirals => section(
                "Admirals and flagships",
                vec![
                    "Before round 1 both players choose their admiral at the same time and in secret; you only learn your opponent's choice once both have picked, and then round 1 begins. Each admiral stands for a style of play and brings a flagship and an admiral ability.".into(),
                    "The flagship isn't part of any deck. In round 1 you deploy it in addition to your up to two ships, following the normal placement rules; it counts toward the lane and fleet limit. Otherwise it is a regular combat ship: it fires at the target you give it, maneuvers and can receive your commands. If it is destroyed, you lose. On the board flagships stand out: a little bigger, with a ring in their admiral's color beneath them and a crown next to the hull bar.".into(),
                    "The admiral ability is available every round, but it takes the place of the maneuver: each turn you either maneuver a ship or use the ability, never both — planning one drops the other. Only the command Carte Blanche allows both in the same turn. Tap the ability card next to your commands, then one of the glowing targets; tap the card again or press Escape to take it back. Abilities are revealed right after the deploys. Their mark follows a ship that moves; an ability on one of your ships may also target a ship you deploy this turn.".into(),
                ],
                None,
            ),
            RulesChapter::Roster => section(
                "The five admirals",
                vec![
                    format!("Admiral Osei, The Wall, commands the Bastion (Battleship, {}). Guardian Shield: after the maneuvers, the Bastion gains +{} shield for the round, the ship in the Strike slot directly in front of her +{}. Counterfire: choose one of your ships; every enemy ship except a flagship that hits it in combat takes {} damage per hit afterwards.", stats(ShipId::Bastion), V.guardian_shield, V.guardian_shield_front, V.counterfire).into(),
                    format!("Admiral Vance, The Hammer, commands the Vanguard (Cruiser, {}). Spearhead: the combat ship in the Strike slot directly in front of her has +{} ATK. Focus Fire: choose an enemy ship, even a covered one; this round every hit on it in combat deals +{} damage. It doesn't steer any shot — your fire orders decide who fires at it.", stats(ShipId::Vanguard), V.spearhead, V.focus_fire).into(),
                    format!("Admiral Halvorsen, The Swarm, commands the Hive (Battleship, {}). Brood Chamber: at the end of every round the Hive launches a Drone into the nearest free Strike slot — at most {} of your Drones at a time and {} per match. Swarm Strike needs one of your Drones on the board: choose an enemy ship other than the flagship, covered or not; all your Drones fire at it instead of their own targets — through cover, without fire orders, even those launched this round —, each hit dealing +{} damage.", stats(ShipId::Hive), V.max_drones, V.hive_budget, V.swarm_strike).into(),
                    format!("Admiral Moreau, The Phantom, commands the Mirage (Fighter, {}). Phase Shift: the Mirage's maneuver may also target one of your occupied slots — then both ships swap places and both still fire. Decoy: choose a free Strike slot of yours; a Decoy (1 HP) stands there for this round. It never attacks, but to the enemy it is a ship at the front of its column: column fire hits the Decoy first, and tracking fire at the ship behind it fizzles as long as the Decoy stands. Flagships see through it: it doesn't cover against them, and their column fire passes it and hits the ship behind — only with no ship behind it do they hit the Decoy. At the end of the round it dissolves.", stats(ShipId::Mirage)).into(),
                    format!("Admiral Kerr, The Clockmaker, commands the Pendulum (Battleship, {}). Railgun: in a round in which her railgun doesn't fire, Kerr may lock onto an enemy ship. Next round the railgun hits it for {} at the start of combat if it is exposed; if it is covered, the shot is blocked. If the Pendulum moves in either round, the shot is lost. Locked ships carry a crosshair on both sides of the board. Head Start: choose one of your combat ships; it fires before everyone else, and whatever it destroys no longer fires.", stats(ShipId::Pendulum), V.railgun).into(),
                ],
                Some("The admirals with their flagship, its ability and the admiral ability."),
            ),
            RulesChapter::Board => section(
                "The board",
                vec![
                    "Each side has two lanes with 6 slots each: the Strike lane in front, facing the enemy, and the Command lane behind it. A lane holds at most 4 ships and a side at most 6 (Drones count toward neither), so at least two slots per lane stay empty.".into(),
                    "The two slots with the same number form a column: the Strike slot in front, the Command slot behind it. The Command lane offers cover: a ship there is covered as long as a ship stands in front of it in its column. Gunfire only reaches a covered ship once the ship in front of it has fallen (see “Combat”). Without a ship in front, a Command ship is just as exposed as one in the Strike lane.".into(),
                    "Cover counts at the moment of each shot: once the ship in front is destroyed — even by an earlier shot of the same combat —, the ship behind it is exposed to every shot that follows. Damage from commands ignores cover: Orbital Strike, Torpedo, Heat Seeker, Bombardment and every other damaging command hit covered ships too. Only Collision Course needs an exposed ship opposite.".into(),
                    "Ships in either lane fire with full strength; the lane only matters for cover, for the order of the shots in combat (Strike lane first), for lane-specific commands (Power Surge, Targeting Jammer, Intercept, Recall, Collision Course, Bombardment) and for the Signal Booster, which boosts the ships in its own lane.".into(),
                ],
                Some("Each side: Strike in front, Command behind; slots with the same number form a column. Enemy fire into a column hits the front ship first: the Command ship behind a Strike ship is covered, the one without is exposed."),
            ),
            RulesChapter::Movement => section(
                "Moving ships",
                vec![
                    "Once per round — unless you use your admiral ability instead — you may maneuver one of your ships already on the board to any free slot of your side, in the other lane or the same one: tap the ship, then a glowing slot. Within the lane that is how you tuck a Repair Drone in behind a Strike ship, for example. The price: a maneuvering ship doesn't fire this round. It can still be shot at and covers or is covered from its new position, and a command played on it follows it to the new slot. Enemy gunfire depends on how it was aimed: tracking fire follows the ship, column fire stays on its column — leaving the column dodges it, moving into a column walks into it. Only the Mirage may maneuver onto one of your occupied slots: both ships swap places and keep firing.".into(),
                    "Commands move ships as well: Flanking Maneuver switches a ship to the other lane, Intercept pulls a Command ship forward into the Strike lane with +2 ATK, Recall retreats a Strike ship into the Command lane and repairs 2 HP, Evasive Action shifts a ship to a free slot of the same lane with Shield 2. Ships moved by a command keep firing. Micro-Jump takes one of your ships out of the battle after all moves until the end of the round: nothing hits it — tracking fire at it fizzles, column fire finds its slot empty —, but it neither fires nor covers. Tractor Beam moves an enemy ship instead: after all moves it pulls it into the nearest free slot of its other lane.".into(),
                    "Evasive Action stays within the lane, the other movement commands switch to the other lane. Maneuver and movement command can't move the same ship and can't land on the same slot. The slot a ship leaves stays blocked for the rest of the round — it only frees up next round, also for ships you deploy. Deployed ships and both destinations together count toward the limit of 4 ships per lane.".into(),
                    "Moving carries a risk: Barrage deals 3 damage to every enemy ship that changed slots this round, no matter how, and Heat Seeker follows its target and hits a ship that changed slots for 5 instead of 3. A Torpedo targets a slot, not a ship — Piercing Shot a column: it hits whoever stands there after all moves, much like column fire, so dodging it — or walking into it — is part of the game.".into(),
                ],
                Some("A planned maneuver: the Scrap Raider will retreat from the Strike lane into the Command lane and hold fire this round. A maneuver may also stay within the lane."),
            ),
            RulesChapter::Resolution => section(
                "Resolving a round",
                vec![
                    "Once both players have ended their turn, both turns are revealed and resolved in fixed phases. Within a phase both sides act at the same time; no side ever goes first.".into(),
                ],
                Some("Commands in the Effects phase run by type, not by player: repositioning first, then tactics, buffs, special effects, debuffs, and damage last."),
            ),
            RulesChapter::Combat => section(
                "Combat",
                vec![
                    "Each round you give your combat ships their fire orders: for every ship with attack above 0 you pick what it fires at, and there are two ways to aim. Column fire aims at a column of the enemy side, any of the six, even an empty one. Tracking fire aims at one particular enemy ship that is on the board while you plan, covered ones and the enemy flagship included. A ship without a fire order doesn't fire — there is no automatic targeting, and every round starts without fire orders. Ships you deploy this round can get a fire order too; a ship that maneuvers holds fire (the Mirage's Phase Shift is the exception: both ships keep firing), and one disabled by EMP Blast can't fire.".into(),
                    "Column fire stays on its column: the shot hits the frontmost ship still standing there when it is fired — the Strike ship, or the Command ship behind it if the Strike slot is empty or its ship has already fallen in this combat. If no ship stands there (any more), the shot fizzles. So a ship that maneuvers out of the column dodges it, and one that moves in or is deployed there walks into it. Tracking fire follows its ship through maneuvers, commands, Tractor Beam and Phase Shift and hits it wherever it stands — but it fizzles if the ship is covered at the moment of the shot, already destroyed (it never passes on to another ship) or no longer on the board (destroyed before combat, Micro-Jump).".into(),
                    "Within a side the shots are fired one after another: first the Strike lane from slot 1 to 6, then the Command lane. The two sides fire at the same time, so a ship destroyed in this combat still fires — the order only matters among the shots of one side. Because cover counts at the moment of each shot, column fire from your Strike lane can break the cover first, and tracking fire from your Command lane then hits the ship behind it. Nothing prevents overkill: while you plan, the preview shows which ship each shot hits and how much damage every enemy ship takes, so spread your fire.".into(),
                    "Because a column can be targeted even when it is empty, there is gunfire from round 1 on: blind into the columns where you expect the enemy to land. A ship deployed this round can be hit by column fire (or draw shots with Draw Fire), but not by tracking fire, which needs a ship while you plan. Drones launched during the round don't fire before the next one, unless Swarm Strike sends them.".into(),
                    "Damage goes to the shield first and only then to the hull. A ship's own shield (Iron Dreadnought: 2) doesn't regenerate; extra shield from commands like Emergency Shields lasts for the round only, except for Ablative Armor, which stays until it is shot away. A ship with 0 HP is destroyed and removed — after the Effects phase, after combat, and after self-damage in Cleanup.".into(),
                    "The attack value already includes everything: round bonuses from commands, the Scrap Raider's bonus, the Signal Booster aura, the Vanguard's Spearhead and Overdrive. Support ships never attack, no matter the bonus.".into(),
                    format!("Admirals bend these rules: the Pendulum's railgun fires before anyone else, Head Start lets one ship shoot before the rest, Focus Fire adds +{} damage to every hit on its target, Swarm Strike sends all your Drones at its target — through cover and without fire orders —, and Counterfire strikes back once all shots are fired. Admiral Moreau's Decoy counts as a ship at the front of its column, but flagships see through it. The command Draw Fire pulls the enemy shots of both kinds onto your chosen ship as long as it stands and is exposed to the shooter; only Swarm Strike and the railgun keep their targets.", V.focus_fire).into(),
                ],
                Some("Column fire (dotted frame) hits the front ship of its column, tracking fire (dotted ring) follows its ship. Red: the shot; pale with a cross: a shot that fizzles; the numbers give the order of the shots."),
            ),
            RulesChapter::Ships => section(
                "Ships",
                vec![
                    "Ships have hull points (HP), attack (ATK) and sometimes a shield. There are four classes: Fighters, Cruisers and Battleships are combat ships, Support ships never attack (0 ATK). Your ship deck holds two copies of each ship.".into(),
                    "Some ships have an ability. On deploy, once both sides have placed their ships: a Titan Carrier launches its Drone into the nearest free slot of its lane (Drones count toward neither the lane nor the fleet limit; there is no Drone only if every slot of the lane is taken), then every Plasma Frigate deals 3 damage to a random enemy ship in the lane of the same name (Strike against Strike, Command against Command), ignoring cover — a fresh Drone can be hit too. A ship destroyed that way neither maneuvers nor receives your command.".into(),
                    "The Drone (1 HP, 1 ATK, Fighter) isn't in any deck; it counts as a combat ship, but toward neither the lane nor the fleet limit. Titan Carriers launch it on deploy, Drone Squadron launches two in the Effects phase, the Hive at the end of the round. It only launches if there is room — also for the destinations of your maneuver and movement command this round. A Drone launched during the round only fires from the next round on, with Swarm Strike right away. The Scrap Raider permanently gains +1 ATK whenever an enemy ship in the lane of the same name is destroyed, however it falls; it keeps the bonus when it moves. The Repair Drone repairs in Cleanup, the Signal Booster boosts its lane.".into(),
                    "The gallery also shows the five flagships and the Decoy of Admiral Moreau; neither is part of a deck.".into(),
                ],
                None,
            ),
            RulesChapter::Commands => section(
                "Commands",
                vec![
                    "Commands are played once and resolved in the Effects phase, before combat. Some pick a target when you plan them — one of your ships, an enemy ship (Heat Seeker, Tractor Beam), a slot on the enemy side (Torpedo) or an enemy column (Piercing Shot) —, the others choose their target automatically. Round effects like ATK bonuses, extra shield and EMP end in Cleanup.".into(),
                    "Commands resolve by type, in this order: repositioning, tactics, buffs, special, debuffs, attacks. Countermeasures (special) makes the opponent's attack or debuff command of the same round fizzle before it acts. Your command deck holds one copy of each, except Torpedo and Barrage with two.".into(),
                ],
                None,
            ),
            RulesChapter::Hand => section(
                "Hand and decks",
                vec![
                    "You start with 4 ships — always 1 support ship and 3 combat ships — and 3 commands. After every round you get supplies: 3 ships and 3 commands from your decks, of which you take one ship and one command into your hand; the rest is shuffled back into the deck. Only then do you plan your turn. Each hand holds at most 10 cards: a full hand gets no supplies, so play your cards. If one of your Scrap Raiders salvaged scrap in the round and one is still on the board, one of the three commands is Scrap Salvage: you play it like any command, and the chosen Scrap Raider gets +1 ATK permanently. If you don't take it, it's gone. Identical cards form a stack, but each still counts. Scavenge draws 1 extra ship and 1 extra command right away, Countermeasures 1 command if nothing was countered, again within the hand limit.".into(),
                    "Your ship deck has 16 cards (each ship twice), your command deck 34 (Torpedo and Barrage twice, every other command once). When a deck runs low, the supplies show only what is left.".into(),
                ],
                None,
            ),
            RulesChapter::Controls => section(
                "Controls",
                vec![
                    "Tap a card in your hand, then a glowing slot or ship on the board; the status line at the top tells you what the selected card needs. Tapping the card again takes it back. In round 1 your flagship stands at the front of your ship hand with a golden frame. Without a card selected, tapping one of your ships — or a ghost ship — selects it: an arrow appears in front of every enemy column, and the enemy ships light up. Tapping an arrow gives your ship column fire on that column, tapping an enemy ship gives it tracking fire on exactly that ship; tapping the same target again cancels the fire order. A ship already on the board also shows the slots it can maneuver to; a maneuver cancels its fire order. Tapping the selected ship once more takes back its deployment or maneuver; Escape, a right-click or a tap next to the board ends the selection. Your admiral ability is the card to the left of your commands, Admiral Kerr's target lock the button above it. The checklist below “End turn” shows which parts of your turn are planned, still open or impossible.".into(),
                    "While you plan, the board previews the round as if your opponent did nothing: for each fire order an arc points at the ship the shot would hit, tracking fire with a ring around the tip; a shot that would fizzle is pale and dashed, with a cross instead of the tip. With a ship selected, the preview already shows what a tap on the arrow or ship under the pointer would do. Hull bars blink with the damage a ship would take, and a skull marks every ship that would be destroyed. Chance, like the Plasma Frigate's shot, isn't included. The panel at the top right shows what you know about your opponent: whether they have ended their turn and how many cards they hold and have left in their decks.".into(),
                    "“End turn” submits your plan. While the round is resolved you can skip the animation; the log button at the top right lists every event of the match. Scroll or hold ↑/↓ to zoom the board, and drag to pan while zoomed in. The camera button at the top right or V switches between the side view and the view from behind; hold ← or → or drag with the middle mouse button to turn the camera freely in between. Angle and zoom carry over to your next match. Sound, radio chatter, animation speed and language live in the settings next to this dialog; the flag next to it surrenders the match after a confirmation, and your opponent wins immediately.".into(),
                ],
                None,
            ),
        }
    }
    fn phase(&self, phase: Phase) -> PhaseText {
        let (name, text) = match phase {
            Phase::Deploy => (
                "Deploy",
                "New ships land, in round 1 the flagships too. Admiral abilities are revealed (a Decoy appears), Titan Carriers launch Drones, Plasma Frigates fire, the Pendulum locks on.",
            ),
            Phase::Maneuver => (
                "Maneuvers",
                "Maneuvering ships switch slots and hold fire, the Mirage swaps places. Then the Bastion raises its Guardian Shield.",
            ),
            Phase::Effects => (
                "Effects",
                "Commands take effect, by type; damage last. Destroyed ships are removed.",
            ),
            Phase::Combat => (
                "Combat",
                "Railgun first, then Head Start, then the other ships with a fire order: Strike lane before Command lane, by slot. Column fire hits the front ship of its column, tracking fire its target if exposed — otherwise the shot fizzles. Counterfire strikes back last. Destroyed ships are removed.",
            ),
            Phase::Cleanup => (
                "Cleanup",
                "Micro-jumped ships return, self-damage from Overcharge, Repair Drones repair, round effects expire, the Decoy dissolves, the Hive launches a Drone.",
            ),
            Phase::Draw => (
                "Supplies",
                "Everyone picks a ship and a command out of three each (hand limit 10).",
            ),
            Phase::Check => (
                "Check",
                "Whoever has lost their flagship loses; if both fell, it's a draw.",
            ),
        };
        PhaseText { name, text }
    }
    fn figures_you(&self) -> &'static str {
        "You"
    }
    fn figures_enemy(&self) -> &'static str {
        "Enemy"
    }
    fn figures_slot(&self, index: usize) -> String {
        format!("Slot {index}")
    }
    fn figures_covered(&self) -> &'static str {
        "covered"
    }
    fn figures_exposed(&self) -> &'static str {
        "exposed"
    }
    fn figures_lane_limit(&self) -> &'static str {
        "max. 4 ships per lane, 6 per side"
    }
    fn figures_column_fire(&self) -> &'static str {
        "Column fire"
    }
    fn figures_tracking_fire(&self) -> &'static str {
        "Tracking fire"
    }
    fn figures_column_through(&self) -> &'static str {
        "hits the front ship, once it has fallen the one behind"
    }
    fn figures_column_dodge(&self) -> &'static str {
        "fizzles if the ship leaves the column"
    }
    fn figures_tracking_follows(&self) -> &'static str {
        "follows its ship into another slot"
    }
    fn figures_tracking_covered(&self) -> &'static str {
        "fizzles while its ship is covered"
    }
    fn figures_round(&self, round: u32) -> String {
        format!("Round {round}")
    }
    fn figures_overdrive(&self, bonus: i32) -> String {
        if bonus == 0 {
            "no Overdrive".into()
        } else {
            format!("+{bonus} ATK")
        }
    }
    fn figures_rounds(&self, from: u32, to: Option<u32>) -> String {
        match to {
            None => format!("from round {from}"),
            Some(to) => format!("rounds {from}–{to}"),
        }
    }
    fn figures_maneuver(&self) -> &'static str {
        "Maneuver"
    }
    fn figures_turn_parts(&self) -> [&'static str; 4] {
        ["1 ship", "1 command", "maneuver or ability", "fire orders"]
    }
    fn figures_or(&self) -> &'static str {
        "or"
    }
    fn figures_round_one(&self) -> &'static str {
        "Round 1: flagship + 2 ships"
    }
    fn command_type(&self, command_type: CommandType) -> &'static str {
        match command_type {
            CommandType::Reposition => "Repositioning",
            CommandType::Tactic => "Tactics",
            CommandType::Buff => "Buffs",
            CommandType::Special => "Special",
            CommandType::Debuff => "Debuffs",
            CommandType::Attack => "Attacks",
        }
    }
    fn rules_copies(&self, count: usize) -> String {
        format!("{count}× in deck")
    }
    fn rules_flagship(&self, admiral: &str) -> String {
        format!("Flagship of {admiral}")
    }
    fn rules_token(&self) -> &'static str {
        "Token, not in any deck"
    }

    // --- Dialoge und Knöpfe ---
    fn dialog_close(&self) -> &'static str {
        "Close"
    }
    fn fullscreen_enter(&self) -> &'static str {
        "Fullscreen (F)"
    }
    fn fullscreen_exit(&self) -> &'static str {
        "Exit fullscreen (F)"
    }
    fn camera_view_toggle(&self) -> &'static str {
        "Switch view (V); turn freely with ← → or the middle mouse button"
    }
    fn surrender_title(&self) -> &'static str {
        "Surrender"
    }
    fn surrender_question(&self) -> &'static str {
        "Give up this match? Your opponent wins immediately."
    }
    fn surrender_confirm(&self) -> &'static str {
        "Surrender"
    }

    // --- loading ---
    fn loading_assets(&self, percent: u32) -> String {
        format!("Loading assets … {percent} %")
    }
    fn loading_failed(&self) -> &'static str {
        "Loading failed — restart the game."
    }

    // --- status ---
    fn status_disconnected(&self) -> &'static str {
        "Lost connection to the server — restart the game."
    }
    fn status_connecting(&self) -> &'static str {
        "Connecting to the server …"
    }
    fn status_submitted(&self) -> &'static str {
        "Turn ended — waiting for the opponent."
    }
    fn status_waiting_for_opponent(&self, match_id: &str) -> String {
        format!("Waiting for the opponent — share this match ID: {match_id}")
    }
    fn status_choose_admiral(&self, match_id: Option<&str>) -> String {
        match match_id {
            Some(id) => format!("Choose your admiral. Invite an opponent with this match ID: {id}"),
            None => "Choose your admiral.".into(),
        }
    }
    fn status_opponent_choosing(&self) -> &'static str {
        "The opponent is choosing their admiral …"
    }
    fn status_draft(&self) -> &'static str {
        "Supplies: choose one ship and one command for your hand."
    }
    fn status_flagship_first(&self, flagship: &str) -> String {
        format!(
            "Round 1: deploy your flagship {flagship} — tap its card at the front of your hand, then a glowing slot. “End turn” only works once it is planned."
        )
    }
    fn status_fleet(&self, count: usize, max: usize) -> String {
        format!("Fleet {count}/{max}")
    }
    fn status_no_free_slot(&self, ship: &str, per_lane: usize, per_side: usize) -> String {
        format!(
            "no free slot for {ship} (max. {per_lane} per lane, {per_side} per side, planned ships included)."
        )
    }
    fn status_deploys_left(&self, count: usize) -> String {
        match count {
            0 => "all ships planned".into(),
            1 => "1 ship left to deploy".into(),
            count => format!("{count} ships left to deploy"),
        }
    }
    fn status_place_ship(&self, ship: &str) -> String {
        format!("{ship}: tap a glowing slot. Click the card again to take it back.")
    }
    fn status_no_command_target(&self, command: &str) -> String {
        format!("no own ship to target with {command}.")
    }
    fn status_choose_command_target(&self, command: &str) -> String {
        format!("{command}: tap one of your ships. Click the card again to take it back.")
    }
    fn status_choose_enemy_slot(&self, command: &str) -> String {
        format!(
            "{command}: tap a slot on the enemy side, even an empty one — it hits whichever ship stands there after all moves. Click the card again to take it back."
        )
    }
    fn status_choose_enemy_ship(&self, command: &str) -> String {
        format!(
            "{command}: tap an enemy ship — the command follows it if it moves. Click the card again to take it back."
        )
    }
    fn status_choose_enemy_column(&self, command: &str) -> String {
        format!(
            "{command}: tap a slot on the enemy side — it hits that column, front ship first, after all moves. Click the card again to take it back."
        )
    }
    fn status_no_enemy_target(&self, command: &str) -> String {
        format!("no enemy ship to target with {command}.")
    }
    fn status_command_dropped(&self, command: &str, dropped: &str) -> String {
        format!("Without {command}: {dropped} dropped.")
    }
    fn status_choose_destination(&self, command: &str, ship: &str) -> String {
        format!("{command}: tap a glowing slot for {ship}. Click the card again to take it back.")
    }
    fn status_choose_fire_target(&self, ship: &str) -> String {
        format!(
            "{ship}: tap an arrow in front of an enemy column for column fire — it hits the front ship there; or tap an enemy ship to track it."
        )
    }
    fn status_maneuver_instead(&self) -> &'static str {
        "Or tap a glowing slot of yours to maneuver — then it won't fire."
    }
    fn status_maneuver_or_swap(&self) -> &'static str {
        "Or tap a glowing slot of yours to maneuver (then it won't fire), or one of your ships to swap places with it (Phase Shift, both keep firing)."
    }
    fn status_no_guns(&self, ship: &str) -> String {
        format!("{ship} has no guns.")
    }
    fn status_maneuvering(&self, ship: &str) -> String {
        format!("{ship} maneuvers and won't fire.")
    }
    fn status_command_holds_fire(&self, ship: &str, command: &str) -> String {
        format!("{ship} won't fire this round ({command}).")
    }
    fn status_maneuver_hint(&self) -> &'static str {
        "Tap a glowing slot to maneuver it."
    }
    fn status_change_maneuver(&self) -> &'static str {
        "Tap another glowing slot to change the maneuver."
    }
    fn status_column_fire(&self, column: usize) -> String {
        format!("Column fire on column {column}.")
    }
    fn status_track_fire(&self, target: &str) -> String {
        format!("Tracking {target}.")
    }
    fn status_tap_column_fire(&self, column: usize) -> String {
        format!("Tap for column fire on column {column}.")
    }
    fn status_tap_track_fire(&self, target: &str) -> String {
        format!("Tap to track {target}.")
    }
    fn status_tap_clears_fire(&self) -> &'static str {
        "Tap to cancel the fire order."
    }
    fn status_fire_hits(&self, target: &str) -> String {
        format!("It hits {target}.")
    }
    fn status_fire_through(&self, target: &str) -> String {
        format!("It hits {target} behind — the ship in front is destroyed by then.")
    }
    fn status_column_empty(&self) -> &'static str {
        "The shot fizzles: no ship stands in that column."
    }
    fn status_column_cleared(&self) -> &'static str {
        "The shot fizzles: the ships of that column are destroyed by then."
    }
    fn status_fire_covered(&self, target: &str) -> String {
        format!("The shot fizzles: {target} is covered by then.")
    }
    fn status_fire_fizzles(&self, target: &str) -> String {
        format!("The shot fizzles: {target} is already destroyed by then.")
    }
    fn status_fire_gone(&self) -> &'static str {
        "The shot fizzles: its target is no longer on the board by then."
    }
    fn status_swarm_fire(&self, power: &str) -> String {
        format!("It flies the {power}; its own target only counts if the swarm target is gone.")
    }
    fn status_again_unplan(&self) -> &'static str {
        "Tap the ship again to take it back."
    }
    fn status_again_maneuver(&self) -> &'static str {
        "Tap the ship again to cancel the maneuver."
    }
    fn status_esc_cancels(&self) -> &'static str {
        "Esc cancels."
    }
    fn status_fire_cleared(&self, ship: &str) -> String {
        format!("{ship} won't fire this round — its fire order is cleared.")
    }
    fn status_fire_orders(&self, count: usize) -> String {
        if count == 1 {
            "1 target".into()
        } else {
            format!("{count} targets")
        }
    }
    fn status_fizzles(&self, count: usize) -> String {
        if count == 1 {
            "1 shot fizzles (dashed arc).".into()
        } else {
            format!("{count} shots fizzle (dashed arcs).")
        }
    }
    fn status_maneuver(&self, ship: &str) -> String {
        format!("maneuver of {ship}")
    }
    fn status_swap(&self, ship: &str, partner: &str) -> String {
        format!("Phase Shift of {ship} and {partner}")
    }
    fn status_choose_power_target(&self, power: &str) -> String {
        format!("{power}: tap a glowing target. Tap the ability again or press Esc to cancel.")
    }
    fn status_no_power_target(&self, power: &str) -> String {
        format!("no target for {power} right now.")
    }
    fn status_no_drone(&self, power: &str) -> String {
        format!("{power} needs one of your Drones on the board.")
    }
    fn status_power(&self, power: &str, target: &str) -> String {
        format!("{power} on {target}")
    }
    fn status_power_replaces_maneuver(&self, power: &str, ship: &str) -> String {
        format!("{power} replaces the maneuver of {ship}.")
    }
    fn status_maneuver_replaces_power(&self, power: &str) -> String {
        format!("The maneuver replaces {power}.")
    }
    fn status_choose_mark(&self) -> &'static str {
        "Target lock: tap an enemy ship — next round the Pendulum's railgun hits it, if it's exposed. Tap the button again or press Esc to cancel."
    }
    fn status_mark(&self, ship: &str) -> String {
        format!("target lock on {ship}")
    }
    fn status_planned(&self, cards: &str) -> String {
        format!(
            "{cards} ready. Click “End turn”, or tap a card to take it back — or a ship on the board to change its target or move."
        )
    }
    fn status_idle(&self) -> &'static str {
        "Tap a ship or command card, or one of your ships on the board to give it a target or maneuver it. Click “End turn” without a card to pass."
    }
    fn status_resolving(&self) -> &'static str {
        "Both turns are revealed — resolving the round …"
    }
    fn status_match_over(&self) -> &'static str {
        "The match is over."
    }

    // --- opponent ---
    fn opponent_title(&self) -> &'static str {
        "Opponent"
    }
    fn opponent_choosing(&self) -> &'static str {
        "choosing an admiral …"
    }
    fn opponent_hidden(&self) -> &'static str {
        "choice hidden"
    }
    fn opponent_planning(&self) -> &'static str {
        "planning …"
    }
    fn opponent_ready(&self) -> &'static str {
        "turn ended"
    }
    fn opponent_ships(&self) -> &'static str {
        "Ships"
    }
    fn opponent_commands(&self) -> &'static str {
        "Commands"
    }
    fn opponent_hand(&self) -> &'static str {
        "Hand"
    }
    fn opponent_deck(&self) -> &'static str {
        "Deck"
    }

    // --- radio ---
    fn radio_speaker(&self) -> &'static str {
        "Fleet Comms"
    }
    fn radio_lines(&self, cue: RadioCue) -> &'static [&'static str] {
        match cue {
            RadioCue::BattleStart => &[
                "All ships at battle stations. Awaiting your orders, Admiral.",
                "Fleet in position. Ready when you are, Admiral.",
            ],
            RadioCue::OrdersSent => &[
                "Orders confirmed. Executing.",
                "Copy that, Admiral.",
                "Understood. All ships, execute.",
            ],
            RadioCue::EnemyReady => &[
                "The enemy fleet has locked in their moves.",
                "The enemy is ready, Admiral. Your call.",
            ],
            RadioCue::Overdrive => &[
                "Reactors at overdrive. All weapons free!",
                "Overdrive engaged! Every gun at full power!",
            ],
            RadioCue::OverdriveUp => &[
                "Overdrive rising! The reactors are running hot!",
                "Pushing the reactors further. Expect heavier fire!",
            ],
            RadioCue::LastShip => &[
                "That's our last warship, Admiral!",
                "We're down to our last warship!",
            ],
            RadioCue::Victory => &[
                "Enemy fleet destroyed. The sector is ours, Admiral!",
                "That's the last of them. Victory, Admiral!",
            ],
            RadioCue::Defeat => &[
                "We've lost the fleet. All hands, abandon ship.",
                "Our fleet is gone. All survivors, fall back.",
            ],
            RadioCue::Draw => &["Both fleets are gone. No one wins today."],
            RadioCue::EnemySurrender => &["The enemy is withdrawing. They've surrendered!"],
            RadioCue::Retreat => &["Signal the retreat. All ships, disengage."],
            RadioCue::TorpedoAway => &["Torpedo away!", "Fish in the water!"],
            RadioCue::TorpedoHit => &["Direct hit!", "Torpedo impact confirmed!"],
            RadioCue::TorpedoMissed => &[
                "Torpedo missed! They moved out of the way.",
                "No impact. The target slipped away.",
            ],
            RadioCue::TorpedoIncoming => {
                &["Incoming torpedo!", "Torpedo inbound! Brace for impact!"]
            }
            RadioCue::TorpedoDodged => {
                &["Their torpedo missed!", "Torpedo went wide. Nice flying!"]
            }
            RadioCue::BarrageHit => &[
                "Caught them moving!",
                "Barrage on target! They flew right into it!",
            ],
            RadioCue::BarrageMissed => &["Barrage hit nothing. They held their ground."],
            RadioCue::BarrageCaught => &[
                "Barrage! We flew right into it!",
                "They were waiting for us to move!",
            ],
            RadioCue::OrbitalStrike => &["Orbital strike inbound on target!"],
            RadioCue::OrbitalIncoming => &["Orbital strike incoming! Brace!"],
            RadioCue::Volley => &["All batteries, fire!"],
            RadioCue::VolleyIncoming => &["Missiles incoming!"],
            RadioCue::EmpHit => &["EMP hit! Their weapons are down."],
            RadioCue::EmpIncoming => &["EMP! Our weapons are offline!"],
            RadioCue::Jammer => &["Jamming their targeting systems."],
            RadioCue::Jammed => &["They're jamming our targeting!"],
            RadioCue::ShieldsUp => &["Shields up!", "Diverting power to shields!"],
            RadioCue::Overcharge => &["Overcharging weapons. She won't hold together for long!"],
            RadioCue::PowerSurge => &["Power surge to the strike line!"],
            RadioCue::Scavenge => &["Salvage teams report new supplies."],
            RadioCue::Intercept => &["Moving to intercept!"],
            RadioCue::Recall => &["Pull her back and patch her up!"],
            RadioCue::Flanking => &["Flanking maneuver, go!"],
            RadioCue::DroneLaunch => &["Drone away!"],
            RadioCue::FrigateFire => &["Frigate firing on arrival!"],
            RadioCue::Repair => &["Repair drones at work."],
            RadioCue::Scrap => &["The raider is stripping the wreckage!"],
            RadioCue::EnemyDestroyed => &[
                "Enemy ship destroyed!",
                "Target down!",
                "Splash one!",
                "Kill confirmed!",
            ],
            RadioCue::ShipLost => &[
                "We lost a ship!",
                "Ship down! Ship down!",
                "We've lost one of ours!",
            ],
            RadioCue::HullCritical => &["Hull integrity critical!", "She's badly hurt, Admiral!"],
        }
    }

    // --- result ---
    fn result_title(&self, result: MatchResult) -> &'static str {
        match result {
            MatchResult::Won => "Victory",
            MatchResult::Lost => "Defeat",
            MatchResult::Draw => "Draw",
        }
    }
    fn result_won_subtitle(&self, flagship: &str) -> String {
        format!("The enemy flagship {flagship} is destroyed.")
    }
    fn result_lost_subtitle(&self, flagship: &str) -> String {
        format!("Your flagship {flagship} is destroyed.")
    }
    fn result_draw_subtitle(&self, own: &str, enemy: &str) -> String {
        format!("Both flagships are destroyed: your {own} and the enemy {enemy}.")
    }
    fn result_surrendered(&self, won: bool) -> &'static str {
        if won {
            "The opponent has surrendered."
        } else {
            "You surrendered."
        }
    }
    fn result_rounds(&self, round: u32) -> String {
        format!("Decided in round {round}")
    }
    fn result_view_board(&self) -> &'static str {
        "View board"
    }
    fn result_new_match(&self) -> &'static str {
        "New match"
    }

    // --- log ---
    fn log_round_header(&self, round: u32) -> String {
        format!("— Round {round} —")
    }
    fn log_who(&self, side: Side) -> &'static str {
        match side {
            Side::Player => "You",
            Side::Enemy => "Enemy",
        }
    }
    fn log_slot(&self, side: Side, lane: &str, slot: usize) -> String {
        let owner = match side {
            Side::Player => "your",
            Side::Enemy => "enemy",
        };
        format!("{owner} {lane} slot {slot}")
    }
    fn log_column(&self, side: Side, column: usize) -> String {
        let owner = match side {
            Side::Player => "your",
            Side::Enemy => "enemy",
        };
        format!("{owner} column {column}")
    }
    fn log_ship(&self, ship: &str, slot: &str) -> String {
        format!("{ship} ({slot})")
    }
    fn log_deploy(&self, who: &str, ship: &str, slot: &str) -> String {
        format!("Deploy: {who} — {ship} on {slot}")
    }
    fn log_deploy_flagship(&self, who: &str, ship: &str, slot: &str) -> String {
        format!("Deploy: {who} — flagship {ship} on {slot}")
    }
    fn log_power(&self, who: &str, power: &str, target: &str) -> String {
        format!("Ability: {who} — {power} on {target}")
    }
    fn log_decoy(&self, ship: &str) -> String {
        format!("Ability: {ship} appears")
    }
    fn log_mark(&self, who: &str, ship: &str) -> String {
        format!("Target lock: {who} — the Pendulum locks onto {ship}")
    }
    fn log_swap(&self, who: &str, ship: &str, partner: &str) -> String {
        format!("Phase Shift: {who} — {ship} and {partner} swap places and keep firing")
    }
    fn log_pass(&self, who: &str) -> String {
        format!("Deploy: {who} — no ship")
    }
    fn log_maneuver(&self, who: &str, ship: &str, slot: &str) -> String {
        format!("Maneuver: {who} — {ship} moves to {slot} and holds fire this round")
    }
    fn log_command(&self, who: &str, command: &str) -> String {
        format!("Command: {who} — {command}")
    }
    fn log_command_on(&self, who: &str, command: &str, slot: &str) -> String {
        format!("Command: {who} — {command} on {slot}")
    }
    fn log_damage(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Effect: {command} hits {ship} for {amount}")
    }
    fn log_attack_bonus(&self, command: &str, ship: &str, delta: i32) -> String {
        format!(
            "Effect: {command} — {ship} {} ATK this round",
            signed(delta)
        )
    }
    fn log_shield(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Effect: {command} — {ship} +{amount} shield this round")
    }
    fn log_disable(&self, command: &str, ship: &str) -> String {
        format!("Effect: {command} — {ship} cannot attack this round")
    }
    fn log_heal(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Effect: {command} heals {ship} for {amount}")
    }
    fn log_move(&self, command: &str, ship: &str, slot: &str) -> String {
        format!("Effect: {command} — {ship} moves to {slot}")
    }
    fn log_shield_permanent(&self, command: &str, ship: &str, amount: i32) -> String {
        format!("Effect: {command} — {ship} +{amount} shield, permanently")
    }
    fn log_tow(&self, command: &str, ship: &str, slot: &str) -> String {
        format!("Effect: {command} pulls {ship} to {slot}")
    }
    fn log_jump(&self, command: &str, ship: &str) -> String {
        format!("Effect: {command} — {ship} jumps off the board until cleanup")
    }
    fn log_taunt(&self, command: &str, ship: &str) -> String {
        format!("Effect: {command} — {ship} draws enemy fire this round")
    }
    fn log_launch(&self, command: &str, ship: &str) -> String {
        format!("Effect: {command} launches {ship}")
    }
    fn log_ram(&self, command: &str, ship: &str, target: &str, dealt: i32, taken: i32) -> String {
        format!("Effect: {command} — {ship} rams {target}: {dealt} dealt, {taken} taken")
    }
    fn log_jam(&self, command: &str, ship: &str, power: &str) -> String {
        format!("Effect: {command} — {power} on {ship} is cut off")
    }
    fn log_jam_decoy(&self, command: &str, ship: &str) -> String {
        format!("Effect: {command} — {ship} dissolves")
    }
    fn log_countered(&self, command: &str, who: &str, countered: &str) -> String {
        format!("Effect: {command} ({who}) intercepts {countered}")
    }
    fn log_rule(&self, command: &str, who: &str, detail: &str) -> String {
        format!("Effect: {command} ({who}) — {detail} this round")
    }
    fn log_rejoin(&self, ship: &str) -> String {
        format!("Cleanup: {ship} returns from its jump")
    }
    fn log_repair(&self, source: &str, ship: &str, amount: i32) -> String {
        format!("Cleanup: {source} repairs {ship} for {amount}")
    }
    fn log_ability_damage(&self, source: &str, ship: &str, amount: i32) -> String {
        format!("Deploy: {source} hits {ship} for {amount}")
    }
    fn log_guardian_shield(&self, source: &str, ship: &str, amount: i32) -> String {
        format!("Guardian Shield: {source} gives {ship} +{amount} shield this round")
    }
    fn log_spawn(&self, source: &str, ship: &str) -> String {
        format!("Deploy: {source} launches {ship}")
    }
    fn log_brood(&self, source: &str, ship: &str) -> String {
        format!("Brood Chamber: {source} launches {ship}")
    }
    fn log_salvage(&self, ship: &str) -> String {
        format!("Salvage: {ship} recovers scrap — Scrap Salvage comes with the next supplies")
    }
    fn log_draw(&self, command: &str, who: &str, ships: u32, commands: u32) -> String {
        format!(
            "Effect: {command} — {who} {}",
            self.banner_draw(ships, commands)
        )
    }
    fn log_no_effect(&self, command: &str, who: &str) -> String {
        format!("Effect: {command} ({who}) has no effect")
    }
    fn log_miss(&self, command: &str, slot: &str) -> String {
        format!("Effect: {command} fizzles in {slot}")
    }
    fn log_railgun(&self, source: &str, target: &str, amount: i32) -> String {
        format!("Railgun: {source} hits {target} for {amount}")
    }
    fn log_railgun_blocked(&self, source: &str, target: &str) -> String {
        format!("Railgun: the shot of {source} at {target} is blocked — the target is covered")
    }
    fn log_attack(&self, attacker: &str, target: &str, amount: i32, notes: &[&str]) -> String {
        format!(
            "Combat: {attacker} fires at {target} ({})",
            amount_notes(amount, notes)
        )
    }
    fn log_through(&self, attacker: &str, target: &str, amount: i32, notes: &[&str]) -> String {
        format!(
            "Combat: {attacker} fires past the fallen front ship at {target} ({})",
            amount_notes(amount, notes)
        )
    }
    fn log_fizzle_empty(&self, attacker: &str, column: &str) -> String {
        format!("Combat: the shot of {attacker} fizzles — no ship in {column}")
    }
    fn log_fizzle_cleared(&self, attacker: &str, column: &str) -> String {
        format!(
            "Combat: the shot of {attacker} fizzles — the ships in {column} are already destroyed"
        )
    }
    fn log_fizzle(&self, attacker: &str, target: &str) -> String {
        format!("Combat: the shot of {attacker} fizzles — {target} is already destroyed")
    }
    fn log_fizzle_covered(&self, attacker: &str, target: &str) -> String {
        format!("Combat: the shot of {attacker} fizzles — {target} is covered")
    }
    fn log_fizzle_gone(&self, attacker: &str) -> String {
        format!("Combat: the shot of {attacker} fizzles — its target is no longer on the board")
    }
    fn log_silenced(&self, ship: &str) -> String {
        format!("Head Start: {ship} is destroyed before it can fire")
    }
    fn log_counterfire(&self, ship: &str, source: &str, amount: i32) -> String {
        format!("Counterfire: {ship} takes {amount} for hitting {source}")
    }
    fn log_self_damage(&self, ship: &str, amount: i32) -> String {
        format!("Cleanup: {ship} takes {amount} self-damage")
    }
    fn log_destroyed(&self, ship: &str) -> String {
        format!("Destroyed: {ship}")
    }
    fn log_flagship_destroyed(&self, ship: &str) -> String {
        format!("Destroyed: flagship {ship}!")
    }
    fn log_vanish(&self, ship: &str) -> String {
        format!("Cleanup: {ship} dissolves")
    }

    // --- banner ---
    fn banner_command(&self, who: &str, command: &str) -> String {
        format!("{who}: {command}")
    }
    fn banner_mark_target(&self, ship: &str) -> String {
        format!("locks onto {ship}")
    }
    fn banner_blocked(&self) -> &'static str {
        "blocked — the target is covered"
    }
    fn banner_no_effect(&self) -> &'static str {
        "no effect"
    }
    fn banner_miss(&self, slot: &str) -> String {
        format!("fizzles in {slot}")
    }
    fn banner_draw(&self, ships: u32, commands: u32) -> String {
        if ships > 0 {
            format!("+{ships} ship, +{commands} command")
        } else {
            format!("+{commands} command")
        }
    }
    fn banner_countered(&self, command: &str) -> String {
        format!("intercepts {command}")
    }
    fn banner_extra_ship(&self) -> &'static str {
        "+1 ship"
    }
    fn banner_maneuver_and_power(&self) -> &'static str {
        "maneuver and ability"
    }

    // --- float ---
    fn float_damage(&self, amount: i32) -> String {
        format!("−{amount}")
    }
    fn float_heal(&self, amount: i32) -> String {
        format!("+{amount} HP")
    }
    fn float_shield(&self, amount: i32) -> String {
        format!("+{amount} Shield")
    }
    fn float_attack(&self, delta: i32) -> String {
        format!("{} ATK", float_signed(delta))
    }
    fn float_salvage(&self) -> &'static str {
        "Scrap"
    }
    fn float_disabled(&self) -> &'static str {
        "Disabled"
    }
    fn float_hold_fire(&self) -> &'static str {
        "Holds fire"
    }
    fn float_miss(&self) -> &'static str {
        "Miss"
    }
    fn float_silenced(&self) -> &'static str {
        "Silenced"
    }
    fn float_locked(&self) -> &'static str {
        "Locked"
    }
    fn float_blocked(&self) -> &'static str {
        "Blocked"
    }
    fn float_decoy(&self) -> &'static str {
        "Decoy"
    }
    fn float_shield_permanent(&self, amount: i32) -> String {
        format!("+{amount} Shield (permanent)")
    }
    fn float_taunt(&self) -> &'static str {
        "Draws fire"
    }
    fn float_jammed(&self) -> &'static str {
        "Radio silence"
    }
    fn float_jump(&self) -> &'static str {
        "Jump"
    }
    fn float_fizzle(&self) -> &'static str {
        "Fizzled"
    }
    fn float_covered(&self) -> &'static str {
        "Covered"
    }

    // --- Ids aus dem Regelwerk ---
    fn error(&self, code: ErrorCode) -> &'static str {
        use ErrorCode as E;
        match code {
            E::MatchFull => "This match is already full.",
            E::MatchOver => "The match is over.",
            E::MatchStarted => "The match has already started.",
            E::OpponentMissing => "The opponent hasn't joined yet.",
            E::AdmiralRequired => "Choose your admiral first.",
            E::AdmiralAlreadyChosen => "You have already chosen your admiral.",
            E::OpponentChoosing => "The opponent is still choosing their admiral.",
            E::DraftPending => "Choose your supplies first.",
            E::DraftUnavailable => "There are no supplies to choose right now.",
            E::DraftInvalid => "Choose one ship and one command from the supplies.",
            E::FlagshipRequired => "Deploy your flagship in round 1.",
            E::FlagshipNotAllowed => "The flagship is only deployed in round 1.",
            E::PowerAndManeuver => "Use either the maneuver or your admiral ability, not both.",
            E::PowerTargetInvalid => "Your admiral ability can't target that.",
            E::MarkUnavailable => "The Pendulum can't lock onto a target this round.",
            E::MarkTargetInvalid => "The Pendulum can only lock onto an enemy ship.",
            E::SwapNotAllowed => "Only the Mirage can swap places with one of your ships.",
            E::AlreadySubmitted => "You have already ended your turn this round.",
            E::ShipNotInHand => "This ship card isn't in your hand.",
            E::TooManyShips => "You can't deploy that many ships this round.",
            E::SlotUnavailable => "No ship can land on this slot.",
            E::CommandNotInHand => "This command card isn't in your hand.",
            E::TargetRequired => "This command needs a target.",
            E::TargetNotAllowed => "This command takes no target.",
            E::TargetNotOwnShip => "The command target isn't one of your ships.",
            E::TargetWrongLane => "This command can't target a ship in that lane.",
            E::TargetNotEnemySlot => "The command target isn't a slot on the enemy side.",
            E::TargetNotEnemyShip => "The command target isn't an enemy ship.",
            E::TargetWrongShip => "This command can't target this ship.",
            E::DestinationRequired => "This command needs a slot to move the ship to.",
            E::DestinationNotAllowed => "This command doesn't move a ship.",
            E::DestinationUnavailable => "The ship can't move to this slot.",
            E::ManeuverNotOwnShip => "Only one of your ships already on the board can maneuver.",
            E::ManeuverDestinationUnavailable => "The ship can't maneuver to this slot.",
            E::ManeuverConflict => "A command can't move the ship that maneuvers this round.",
            E::FireShipInvalid => {
                "Only your combat ships can get a fire order, one each — not the ship that maneuvers this round."
            }
            E::FireTargetInvalid => {
                "A fire order needs an enemy column or an enemy ship on the board as its target."
            }
        }
    }
    fn lane(&self, lane: LaneType) -> &'static str {
        match lane {
            LaneType::Strike => "Strike",
            LaneType::Command => "Command",
        }
    }
    fn stats_hp(&self) -> &'static str {
        "HP"
    }
    fn stats_atk(&self) -> &'static str {
        "ATK"
    }
    fn stats_shield(&self) -> &'static str {
        "Shield"
    }
    fn ship_class(&self, class: ShipClass) -> &'static str {
        match class {
            ShipClass::Fighter => "Fighter",
            ShipClass::Cruiser => "Cruiser",
            ShipClass::Battleship => "Battleship",
            ShipClass::Support => "Support",
        }
    }
    fn target_mode(&self, mode: TargetMode) -> &'static str {
        use TargetMode as M;
        match mode {
            M::None => "",
            M::AutoHighestHp => "Highest-HP Enemy",
            M::AutoLowestHp => "Lowest-HP Enemy",
            M::AutoHighestAtk => "Highest-ATK Enemy",
            M::AutoAll => "All Targets",
            M::AutoMoved => "Moved Enemies",
            M::AutoArrived => "New Enemies",
            M::AutoUnpaired => "Unpaired Enemies",
            M::AllFriendly => "All Allies",
            M::AllFriendlyStrike => "Allies in Strike",
            M::AllEnemyStrike => "Enemies in Strike",
            M::AllEnemyCommand => "Enemies in Command",
            M::EnemyPower => "Enemy Power",
            M::ChooseFriendly => "Choose Ally",
            M::ChooseEnemy => "Choose Enemy",
            M::ChooseEnemySlot => "Choose Enemy Slot",
            M::ChooseEnemyColumn => "Choose Enemy Column",
        }
    }
    fn ship(&self, ship: ShipId) -> CardText {
        use ShipId as S;
        match ship {
            S::HorizonCruiser => CardText::new("Horizon Cruiser", "Solid stats, reliable."),
            S::IronDreadnought => CardText::new(
                "Iron Dreadnought",
                "Shield 2: Absorbs the first 2 damage before the hull.",
            ),
            S::PlasmaFrigate => CardText::new(
                "Plasma Frigate",
                "On Deploy: Deal 3 damage to a random enemy ship in the same lane.",
            ),
            S::RepairDrone => CardText::new(
                "Repair Drone",
                "End of each round: Repairs 2 HP on the damaged allied combat ship with the lowest HP.",
            ),
            S::ScrapRaider => CardText::new(
                "Scrap Raider",
                "When an enemy ship in the same lane is destroyed, your next supplies include Scrap Salvage: +1 ATK permanently for a Scrap Raider.",
            ),
            S::SignalBooster => CardText::new(
                "Signal Booster",
                "All other allied ships in the same lane have +1 ATK, as long as the Signal Booster is there.",
            ),
            S::TitanCarrier => CardText::new(
                "Titan Carrier",
                "On Deploy: Spawn a Drone (1 HP, 1 ATK) in the same lane.",
            ),
            S::Drone => CardText::new(
                "Drone",
                "Launched by the Titan Carrier on deploy, by Drone Squadron and by the Hive at the end of the round; not part of any deck.",
            ),
            S::VoidInterceptor => CardText::new("Void Interceptor", "Pure damage, no tricks."),
            S::Decoy => CardText::new(
                "Decoy",
                "Admiral Moreau's hologram: stands for one round and never attacks. Column fire hits it first, tracking fire at the ship behind it fizzles while it stands. Flagships see through it; destroyed, it leaves no scrap.",
            ),
            S::Bastion => CardText::formatted(
                "Bastion",
                format!(
                    "After the maneuvers, she gains +{} shield for the round, the ship in the Strike slot directly in front of her +{}.",
                    V.guardian_shield, V.guardian_shield_front
                ),
            ),
            S::Vanguard => CardText::formatted(
                "Vanguard",
                format!(
                    "The combat ship in the Strike slot directly in front of her has +{} ATK.",
                    V.spearhead
                ),
            ),
            S::Hive => CardText::formatted(
                "Hive",
                format!(
                    "End of each round: launches a Drone into the nearest free Strike slot — at most {} Drones at a time and {} per match.",
                    V.max_drones, V.hive_budget
                ),
            ),
            S::Mirage => CardText::new(
                "Mirage",
                "Her maneuver may also target one of your occupied slots: both ships swap places and both still fire.",
            ),
            S::Pendulum => CardText::formatted(
                "Pendulum",
                format!(
                    "In a round her railgun doesn't fire, she may lock onto an enemy ship. Next round the railgun hits it for {} if it's exposed. If she moves in either round, the shot is lost.",
                    V.railgun
                ),
            ),
        }
    }
    fn admiral(&self, admiral: AdmiralId) -> AdmiralText {
        let (name, epithet, ability) = match admiral {
            AdmiralId::Osei => ("Admiral Osei", "The Wall", "Guardian Shield"),
            AdmiralId::Vance => ("Admiral Vance", "The Hammer", "Spearhead"),
            AdmiralId::Halvorsen => ("Admiral Halvorsen", "The Swarm", "Brood Chamber"),
            AdmiralId::Moreau => ("Admiral Moreau", "The Phantom", "Phase Shift"),
            AdmiralId::Kerr => ("Admiral Kerr", "The Clockmaker", "Railgun"),
        };
        AdmiralText {
            name,
            epithet,
            ability,
        }
    }
    fn power(&self, power: PowerId) -> CardText {
        match power {
            PowerId::Counterfire => CardText::formatted(
                "Counterfire",
                format!(
                    "Choose one of your ships. Every enemy ship except a flagship that hits it in combat takes {} damage per hit afterwards.",
                    V.counterfire
                ),
            ),
            PowerId::FocusFire => CardText::formatted(
                "Focus Fire",
                format!(
                    "Choose an enemy ship, even a covered one. This round every hit on it in combat deals +{} damage.",
                    V.focus_fire
                ),
            ),
            PowerId::SwarmStrike => CardText::formatted(
                "Swarm Strike",
                format!(
                    "Choose an enemy ship other than the flagship, even a covered one. All your Drones fire at it without fire orders, each hit +{} damage.",
                    V.swarm_strike
                ),
            ),
            PowerId::Decoy => CardText::new(
                "Decoy",
                "Choose a free Strike slot. A Decoy (1 HP) stands there this round: column fire hits it first, tracking fire at the ship behind it fizzles. Flagships see through it.",
            ),
            PowerId::HeadStart => CardText::new(
                "Head Start",
                "Choose one of your combat ships. It fires before everyone else, and whatever it destroys no longer fires.",
            ),
        }
    }
    fn power_target(&self, target: PowerTarget) -> &'static str {
        match target {
            PowerTarget::OwnShip => "Choose Ally",
            PowerTarget::OwnCombatShip => "Choose Combat Ally",
            PowerTarget::EnemyShip | PowerTarget::EnemyShipNotFlagship => "Choose Enemy",
            PowerTarget::FreeOwnStrikeSlot => "Choose Strike Slot",
        }
    }
    fn command(&self, command: CommandId) -> CardText {
        use CommandId as C;
        let (name, effect) = match command {
            C::EmergencyShields => (
                "Emergency Shields",
                "Chosen allied ship gains Shield 3 this round.",
            ),
            C::EmpBlast => ("EMP Blast", "Highest-ATK enemy can't fire this round."),
            C::FlankingManeuver => ("Flanking Maneuver", "Ship switches lanes into a free slot."),
            C::Intercept => (
                "Intercept",
                "Command ship moves forward, +2 ATK this round.",
            ),
            C::OrbitalStrike => ("Orbital Strike", "Deal 4 damage to the enemy with most HP."),
            C::Overcharge => ("Overcharge", "+3 ATK this round, then 2 self-damage."),
            C::PowerSurge => ("Power Surge", "Allied Strike ships gain +1 ATK this round."),
            C::Recall => ("Recall", "Strike ship moves back, heals 2 HP."),
            C::Scavenge => ("Scavenge", "Instantly draw 1 ship and 1 command."),
            C::TargetingJammer => (
                "Targeting Jammer",
                "Enemy Strike ships get -1 ATK this round.",
            ),
            C::VolleyFire => ("Volley Fire", "Deal 1 damage to ALL enemy ships."),
            C::Torpedo => ("Torpedo", "After moves: 5 damage to slot, ignoring cover."),
            C::Barrage => ("Barrage", "3 damage to every enemy that changed slots."),
            C::EvasiveAction => (
                "Evasive Action",
                "Ship moves within its lane, gains Shield 2.",
            ),
            C::MicroJump => (
                "Micro-Jump",
                "Leaves battle until round end: untouchable, but inactive.",
            ),
            C::RapidDeployment => (
                "Rapid Deployment",
                "You may deploy one extra ship this round.",
            ),
            C::DrawFire => ("Draw Fire", "Draws all enemy fire orders while exposed."),
            C::ShieldGrid => ("Shield Grid", "All allied ships gain Shield 1 this round."),
            C::AblativeArmor => ("Ablative Armor", "Ship gains Shield 2 permanently."),
            C::CoveringFire => (
                "Covering Fire",
                "Ship and its column partner: +1 ATK, Shield 1.",
            ),
            C::DroneSquadron => (
                "Drone Squadron",
                "Two Drones launch, instantly covering exposed Command ships.",
            ),
            C::CollisionCourse => (
                "Collision Course",
                "Rams the enemy opposite: each takes the other's HP as damage. Doesn't fire.",
            ),
            C::Countermeasures => (
                "Countermeasures",
                "Enemy attack/debuff command fizzles; otherwise draw a command.",
            ),
            C::CarteBlanche => (
                "Carte Blanche",
                "Use both maneuver and admiral ability this round.",
            ),
            C::RadioSilence => (
                "Radio Silence",
                "Enemy admiral ability fizzles; a Decoy dissolves.",
            ),
            C::Minefield => ("Minefield", "3 damage to every newly arrived enemy."),
            C::TractorBeam => (
                "Tractor Beam",
                "Pulls enemy into its other lane; counts as moved.",
            ),
            C::HeatSeeker => ("Heat Seeker", "3 damage through cover, 5 if it moved."),
            C::CoupDeGrace => (
                "Coup de Grâce",
                "4 damage through cover to lowest-HP enemy (no Drones/Decoys).",
            ),
            C::PiercingShot => ("Piercing Shot", "After moves: 3 damage front, 2 behind."),
            C::Bombardment => ("Bombardment", "2 damage to every enemy Command ship."),
            C::Crossfire => ("Crossfire", "3 damage to every enemy alone in its column."),
            C::ScrapSalvage => {
                return CardText {
                    target: Some("Allied Scrap Raider"),
                    ..CardText::new(
                        "Scrap Salvage",
                        "The chosen Scrap Raider gets +1 ATK permanently.",
                    )
                };
            }
        };
        CardText::new(name, effect)
    }
}
