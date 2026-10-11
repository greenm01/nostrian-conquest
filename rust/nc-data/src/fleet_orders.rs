//! A fleet's standing order: its one-line description and the mission
//! target ECGAME writes with it.

use crate::{Fleet, Order};

/// Writes the target depth and the mission target together, as ECGAME
/// does when it sets an order: depth 1 for a world or fleet target, and the
/// Join host's fleet number or the guarded starbase's id.
pub fn set_mission_target(fleet: &mut Fleet, target_depth: u8, mission_target: u16) {
    fleet.target_depth = target_depth;
    fleet.mission_target = mission_target;
}

/// The standing order as the fleet screens describe it.
pub fn order_summary(fleet: &Fleet) -> String {
    let (x, y) = (fleet.target.x, fleet.target.y);
    match fleet.order {
        Order::HoldPosition => "Hold position".to_string(),
        Order::MoveOnly => format!("Move fleet to Sector ({x},{y})"),
        Order::SeekHome => "Seek home".to_string(),
        Order::PatrolSector => format!("Patrol Sector ({x},{y})"),
        Order::GuardStarbase => format!("Guard starbase at Sector ({x},{y})"),
        Order::GuardBlockadeWorld => format!("Guard/blockade world in System ({x},{y})"),
        Order::BombardWorld => format!("Bombard world in System ({x},{y})"),
        Order::InvadeWorld => format!("Invade world in System ({x},{y})"),
        Order::BlitzWorld => format!("Blitz world in System ({x},{y})"),
        Order::ViewWorld => format!("View world in System ({x},{y})"),
        Order::ScoutSector => format!("Scout Sector ({x},{y})"),
        Order::ScoutSolarSystem => format!("Scout solar system ({x},{y})"),
        Order::ColonizeWorld => format!("Colonize world in System ({x},{y})"),
        Order::JoinAnotherFleet => "Join another fleet".to_string(),
        Order::RendezvousSector => format!("Rendezvous at Sector ({x},{y})"),
        Order::Salvage => format!("Salvage at Sector ({x},{y})"),
        Order::Unknown(code) => format!("Unknown order {code} target ({x},{y})"),
    }
}
