//! Ziele der Admiralsfähigkeiten und der Markierung, portiert aus `server/src/powers.test.ts`.

use voidfleet_core::admirals::PowerId;
use voidfleet_core::board::LaneType::{self, Command, Strike};
use voidfleet_core::board::Side::{self, Enemy, Player};
use voidfleet_core::board::{Board, SlotRef, SlotTarget};
use voidfleet_core::cards::ShipId as S;
use voidfleet_core::powers::{
    PlannedArrival, can_mark, mark_targets, power_targets, railgun_fires,
};

fn place(board: &mut Board, side: Side, lane: LaneType, index: usize, id: S) -> SlotRef {
    let at = SlotRef::new(side, lane, index);
    board.place(at, id).expect("Slot ist frei");
    at
}

const fn slot(side: Side, lane: LaneType, index: usize) -> SlotRef {
    SlotRef::new(side, lane, index)
}

const fn at(lane: LaneType, index: usize) -> SlotTarget {
    SlotTarget::new(lane, index)
}

/// Eigene Seite: ein Kreuzer vorne, eine Repair Drone dahinter; Gegner: ein Jäger vor seiner Hive.
fn board() -> Board {
    let mut board = Board::new();
    place(&mut board, Player, Strike, 1, S::HorizonCruiser);
    place(&mut board, Player, Command, 1, S::RepairDrone);
    place(&mut board, Enemy, Strike, 2, S::VoidInterceptor);
    place(&mut board, Enemy, Command, 2, S::Hive);
    board
}

const FLAGSHIP: PlannedArrival = PlannedArrival {
    card: S::Vanguard,
    target: at(Command, 4),
};

mod ziele_der_admiralsfaehigkeiten {
    use super::*;

    #[test]
    fn gegenfeuer_auf_jedes_eigene_schiff_auch_auf_eines_das_in_dieser_runde_landet() {
        assert_eq!(
            power_targets(&board(), Player, PowerId::Counterfire, &[FLAGSHIP], &[]),
            [
                slot(Player, Strike, 1),
                slot(Player, Command, 1),
                slot(Player, Command, 4)
            ]
        );
    }

    #[test]
    fn taktvorsprung_nur_auf_eigene_kampfschiffe_gelegte_eingeschlossen() {
        let booster = PlannedArrival {
            card: S::SignalBooster,
            target: at(Strike, 0),
        };
        assert_eq!(
            power_targets(
                &board(),
                Player,
                PowerId::HeadStart,
                &[FLAGSHIP, booster],
                &[]
            ),
            [slot(Player, Strike, 1), slot(Player, Command, 4)]
        );
    }

    #[test]
    fn fokusfeuer_auf_jedes_gegnerische_schiff_gedeckt_oder_nicht() {
        assert_eq!(
            power_targets(&board(), Player, PowerId::FocusFire, &[], &[]),
            [slot(Enemy, Strike, 2), slot(Enemy, Command, 2)]
        );
    }

    #[test]
    fn schwarmangriff_nur_mit_eigener_drohne_und_nie_auf_das_flaggschiff() {
        let mut with_drone = board();
        assert_eq!(
            power_targets(&with_drone, Player, PowerId::SwarmStrike, &[], &[]),
            []
        );
        place(&mut with_drone, Player, Strike, 5, S::Drone);
        assert_eq!(
            power_targets(&with_drone, Player, PowerId::SwarmStrike, &[], &[]),
            [slot(Enemy, Strike, 2)]
        );
    }

    #[test]
    fn trugbild_in_einen_freien_eigenen_strike_slot_in_dem_nichts_landet_oder_hinzieht() {
        let deploy = PlannedArrival {
            card: S::IronDreadnought,
            target: at(Strike, 0),
        };
        assert_eq!(
            power_targets(
                &board(),
                Player,
                PowerId::Decoy,
                &[deploy, FLAGSHIP],
                &[at(Strike, 5)]
            ),
            [
                slot(Player, Strike, 2),
                slot(Player, Strike, 3),
                slot(Player, Strike, 4)
            ]
        );
    }
}

mod markierung_der_pendulum {
    use super::*;

    #[test]
    fn nur_mit_pendulum_auf_jedes_gegnerische_schiff_nicht_in_der_runde_ihrer_railgun() {
        let mut board = Board::new();
        let target = place(&mut board, Enemy, Strike, 0, S::HorizonCruiser);
        place(&mut board, Enemy, Command, 3, S::Bastion);
        assert!(!can_mark(&board, Player));
        assert_eq!(mark_targets(&board, Player), []);
        place(&mut board, Player, Command, 2, S::Pendulum);
        assert_eq!(
            mark_targets(&board, Player),
            [slot(Enemy, Strike, 0), slot(Enemy, Command, 3)]
        );
        board.ship_mut(target).marked = true;
        assert_eq!(
            [railgun_fires(&board, Player), railgun_fires(&board, Enemy)],
            [true, false]
        );
        assert!(!can_mark(&board, Player));
        assert_eq!(mark_targets(&board, Player), []);
    }
}
