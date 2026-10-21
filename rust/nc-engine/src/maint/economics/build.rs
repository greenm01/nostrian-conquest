use crate::{CoreGameData, ProductionItemKind};
use nc_data::Real48;

pub(super) fn process_build_completion(
    game_data: &mut CoreGameData,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let planet_count = game_data.planets.len();
    let mut planets_with_builds = Vec::new();

    for planet_idx in 0..planet_count {
        // Spending is capped by the treasury and the build capacity; a first
        // starbase may draw its cost from the treasury outside the capacity.
        let mut limits = game_data.planet_spend_limits(planet_idx + 1)?;

        let mut had_builds = false;
        for slot in 0..10 {
            let owed = game_data.planets[planet_idx].build_queue[slot].points_owed;

            if owed == 0 {
                continue;
            }
            had_builds = true;

            let build_kind = game_data.planets[planet_idx].build_queue[slot].kind.code();
            let build_item_kind = ProductionItemKind::from_code(build_kind);
            let decrement = owed.min(limits.points_for(build_item_kind));
            if decrement == 0 {
                continue;
            }
            let new_owed = owed - decrement;
            let completed_qty = completed_unit_count(build_item_kind, owed, new_owed);

            match build_item_kind {
                ProductionItemKind::Army => {
                    let current =
                        u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX);
                    let free_capacity = u32::from(u8::MAX.saturating_sub(current));
                    if completed_qty > free_capacity {
                        continue;
                    }
                }
                ProductionItemKind::GroundBattery => {
                    let current = u8::try_from(game_data.planets[planet_idx].ground_batteries)
                        .unwrap_or(u8::MAX);
                    let free_capacity = u32::from(u8::MAX.saturating_sub(current));
                    if completed_qty > free_capacity {
                        continue;
                    }
                }
                _ => {}
            }

            // Spend whole points; the fraction left in the treasury, and any
            // debt, carry over.
            let planet = &mut game_data.planets[planet_idx];
            planet.stored_goods =
                Real48::from_f64(planet.stored_goods.to_f64() - f64::from(decrement));
            game_data.planets[planet_idx].build_queue[slot].points_owed = new_owed;
            limits.spend(build_item_kind, decrement);

            match build_item_kind {
                ProductionItemKind::Army => {
                    let current =
                        u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX);
                    game_data.planets[planet_idx].armies =
                        u16::from(current.saturating_add(completed_qty as u8));
                }
                ProductionItemKind::GroundBattery => {
                    let current = u8::try_from(game_data.planets[planet_idx].ground_batteries)
                        .unwrap_or(u8::MAX);
                    game_data.planets[planet_idx].ground_batteries =
                        u16::from(current.saturating_add(completed_qty as u8));
                }
                kind if kind.requires_stardock() => {
                    add_to_stardock(
                        &mut game_data.planets[planet_idx],
                        build_kind,
                        completed_qty,
                    );
                }
                _ => {}
            }

            if new_owed == 0 {
                game_data.planets[planet_idx].build_queue[slot].kind =
                    ProductionItemKind::Unknown(0);
            }
        }

        if had_builds {
            planets_with_builds.push(planet_idx);
        }
    }

    Ok(planets_with_builds)
}

fn completed_unit_count(kind: ProductionItemKind, old_remaining: u32, new_remaining: u32) -> u32 {
    let Some(cost) = kind.build_cost() else {
        return 0;
    };
    old_remaining.div_ceil(cost) - new_remaining.div_ceil(cost)
}

/// Completed ships and starbases wait in their type's stardock slot.
fn add_to_stardock(planet: &mut nc_data::Planet, kind: u8, qty: u32) {
    if let Some(slot) = nc_data::stardock_slot_for_kind(kind) {
        let current = planet.stardock[slot];
        planet.stardock[slot] = current.saturating_add(qty);
    }
}
