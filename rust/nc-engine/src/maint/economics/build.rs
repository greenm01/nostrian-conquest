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
                ProductionItemKind::Starbase => {
                    let open_slots =
                        open_stardock_slot_count(&game_data.planets.records[planet_idx]);
                    if completed_qty > open_slots as u32 {
                        continue;
                    }
                }
                kind if kind.requires_stardock() => {
                    if completed_qty > 0
                        && !can_store_ship_completion(
                            &game_data.planets.records[planet_idx],
                            build_kind,
                        )
                    {
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
                ProductionItemKind::Starbase => {
                    add_starbases_to_stardock(
                        &mut game_data.planets.records[planet_idx],
                        build_kind,
                        completed_qty,
                    );
                }
                kind if kind.requires_stardock() => {
                    add_ships_to_stardock(
                        &mut game_data.planets.records[planet_idx],
                        build_kind,
                        completed_qty,
                    )?;
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

fn open_stardock_slot_count(planet: &nc_data::PlanetRecord) -> usize {
    (0..crate::STARDOCK_SLOT_COUNT)
        .filter(|&slot| planet.stardock_kind_raw(slot) == 0)
        .count()
}

fn can_store_ship_completion(planet: &nc_data::PlanetRecord, kind_raw: u8) -> bool {
    (0..crate::STARDOCK_SLOT_COUNT).any(|slot| {
        let existing_kind = planet.stardock_kind_raw(slot);
        existing_kind == 0 || existing_kind == kind_raw
    })
}

fn add_ships_to_stardock(
    planet: &mut nc_data::PlanetRecord,
    kind_raw: u8,
    qty: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if qty == 0 {
        return Ok(());
    }
    for slot in 0..crate::STARDOCK_SLOT_COUNT {
        if planet.stardock_kind_raw(slot) == kind_raw {
            let current = planet.stardock_count_raw(slot);
            planet.set_stardock_count_raw(slot, current.saturating_add(qty as u16));
            return Ok(());
        }
    }
    for slot in 0..crate::STARDOCK_SLOT_COUNT {
        if planet.stardock_kind_raw(slot) == 0 {
            planet.set_stardock_kind_raw(slot, kind_raw);
            planet.set_stardock_count_raw(slot, qty as u16);
            return Ok(());
        }
    }
    Err("stardock has no room for completed ships".into())
}

fn add_starbases_to_stardock(planet: &mut nc_data::PlanetRecord, kind_raw: u8, qty: u32) {
    let mut remaining = qty;
    for slot in 0..crate::STARDOCK_SLOT_COUNT {
        if remaining == 0 {
            break;
        }
        if planet.stardock_kind_raw(slot) != 0 {
            continue;
        }
        planet.set_stardock_kind_raw(slot, kind_raw);
        planet.set_stardock_count_raw(slot, 1);
        remaining -= 1;
    }
}
