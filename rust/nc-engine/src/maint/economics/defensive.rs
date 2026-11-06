use crate::{CoreGameData, ProductionItemKind};
use nc_data::build_queue_unit_counts;

/// The most of either unit a world holds.
const SURFACE_LIMIT: u32 = 255;

/// Defensive mode, before the planet pass spends this year's points: each
/// owned world in the mode queues as many ground batteries as the points its
/// other orders leave will buy, within room for 255, and armies with the
/// rest, within room for 255 (owner, 2026-11-06).
pub(super) fn queue_defensive_builds(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    for planet_idx in 0..game_data.planets.len() {
        let planet = &game_data.planets[planet_idx];
        if !planet.defensive || planet.owner.0 == 0 {
            continue;
        }
        let queued = build_queue_unit_counts(planet);
        let room = |kind: ProductionItemKind, built: u16| {
            let queued = queued.get(&kind.code()).copied().unwrap_or(0);
            SURFACE_LIMIT.saturating_sub(u32::from(built) + queued)
        };
        let battery_room = room(ProductionItemKind::GroundBattery, planet.ground_batteries);
        let army_room = room(ProductionItemKind::Army, planet.armies);

        let mut points = game_data.planet_build_points_left(planet_idx + 1)?;
        for (kind, room) in [
            (ProductionItemKind::GroundBattery, battery_room),
            (ProductionItemKind::Army, army_room),
        ] {
            let cost = kind.build_cost().expect("ground defenses have a cost");
            let units = (points / cost).min(room);
            if units == 0 {
                continue;
            }
            // A full queue (ten other kinds) takes nothing more.
            if game_data
                .append_planet_build_order(planet_idx + 1, units * cost, kind.code())
                .is_ok()
            {
                points -= units * cost;
            }
        }
    }
    Ok(())
}
