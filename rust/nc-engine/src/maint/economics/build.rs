use crate::{CoreGameData, ProductionItemKind};

pub(super) fn process_build_completion(
    game_data: &mut CoreGameData,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let planet_count = game_data.planets.records.len();
    let mut planets_with_builds = Vec::new();

    for planet_idx in 0..planet_count {
        // Spending is capped by the planet budget: min(treasury, build capacity).
        let mut remaining_spend_capacity = game_data.planet_build_budget(planet_idx + 1)?;

        let mut had_builds = false;
        for slot in 0..10 {
            let build_count = game_data.planets.records[planet_idx].build_count_raw(slot);

            if build_count == 0 {
                continue;
            }
            had_builds = true;
            if remaining_spend_capacity == 0 {
                continue;
            }

            let build_kind = game_data.planets.records[planet_idx].build_kind_raw(slot);
            let build_item_kind = ProductionItemKind::from_raw(build_kind);
            let decrement = u32::from(build_count).min(remaining_spend_capacity);
            let new_count = u32::from(build_count).saturating_sub(decrement) as u8;
            let completed_qty = completed_unit_count(build_item_kind, build_count, new_count);

            match build_item_kind {
                ProductionItemKind::Army => {
                    let current = game_data.planets.records[planet_idx].army_count_raw();
                    let free_capacity = u32::from(u8::MAX.saturating_sub(current));
                    if completed_qty > free_capacity {
                        continue;
                    }
                }
                ProductionItemKind::GroundBattery => {
                    let current = game_data.planets.records[planet_idx].ground_batteries_raw();
                    let free_capacity = u32::from(u8::MAX.saturating_sub(current));
                    if completed_qty > free_capacity {
                        continue;
                    }
                }
                _ => {}
            }

            let current_stored = game_data.planets.records[planet_idx].stored_production_points();
            game_data.planets.records[planet_idx]
                .set_stored_production_points(current_stored.saturating_sub(decrement));
            game_data.planets.records[planet_idx].set_build_count_raw(slot, new_count);
            remaining_spend_capacity = remaining_spend_capacity.saturating_sub(decrement);

            match build_item_kind {
                ProductionItemKind::Army => {
                    let current = game_data.planets.records[planet_idx].army_count_raw();
                    game_data.planets.records[planet_idx]
                        .set_army_count_raw(current.saturating_add(completed_qty as u8));
                }
                ProductionItemKind::GroundBattery => {
                    let current = game_data.planets.records[planet_idx].ground_batteries_raw();
                    game_data.planets.records[planet_idx]
                        .set_ground_batteries_raw(current.saturating_add(completed_qty as u8));
                }
                kind if kind.requires_stardock() => {
                    add_to_stardock(
                        &mut game_data.planets.records[planet_idx],
                        build_kind,
                        completed_qty,
                    );
                }
                _ => {}
            }

            if new_count == 0 {
                game_data.planets.records[planet_idx].set_build_kind_raw(slot, 0);
            }
        }

        if had_builds {
            planets_with_builds.push(planet_idx);
        }
    }

    Ok(planets_with_builds)
}

fn completed_unit_count(kind: ProductionItemKind, old_remaining: u8, new_remaining: u8) -> u32 {
    let Some(cost) = kind.build_cost() else {
        return 0;
    };
    ceil_div(u32::from(old_remaining), cost)
        .saturating_sub(ceil_div(u32::from(new_remaining), cost))
}

fn ceil_div(value: u32, divisor: u32) -> u32 {
    if value == 0 {
        0
    } else {
        ((value - 1) / divisor) + 1
    }
}

/// Completed ships and starbases wait in their type's stardock slot.
fn add_to_stardock(planet: &mut nc_data::PlanetRecord, kind_raw: u8, qty: u32) {
    if let Some(slot) = nc_data::stardock_slot_for_kind(kind_raw) {
        let current = planet.stardock_count(slot);
        planet.set_stardock_count(slot, current.saturating_add(qty));
    }
}
