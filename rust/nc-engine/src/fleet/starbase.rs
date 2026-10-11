use nc_data::{CoreGameData, Order};

pub fn guard_fleet_numbers_for_starbase(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
    base_id: u16,
) -> Vec<u16> {
    let mut fleets = game_data
        .fleets
        .iter()
        .filter(|fleet| {
            fleet.owner_empire_raw() as usize == player_record_index_1_based
                && fleet.standing_order_kind() == Order::GuardStarbase
                && fleet.guard_starbase_index_raw() == base_id
        })
        .map(|fleet| fleet.local_slot_word_raw())
        .collect::<Vec<_>>();
    fleets.sort_unstable();
    fleets.dedup();
    fleets
}

pub fn format_guard_fleet_clause(fleet_numbers: &[u16]) -> Option<String> {
    match fleet_numbers {
        [] => None,
        [fleet] => Some(format!("Guard Fleet {} will follow it.", fleet)),
        [first, second] => Some(format!(
            "Guard Fleets {} and {} will follow it.",
            first, second
        )),
        many => Some(format!("{} guard fleets will follow it.", many.len())),
    }
}

pub fn format_starbase_list_guard_label(fleet_numbers: &[u16]) -> String {
    match fleet_numbers {
        [] => "N/A".to_string(),
        [fleet] => format!("The {} Fleet", ordinal_number(*fleet as usize)),
        many => format!("{} guards", many.len()),
    }
}

pub fn format_starbase_review_guard_label(fleet_numbers: &[u16]) -> String {
    match fleet_numbers {
        [] => "N/A".to_string(),
        [fleet] => format!("The {} Fleet", ordinal_number(*fleet as usize)),
        [first, second] => format!("Guard Fleets {} and {}", first, second),
        many => {
            let mut label = String::from("Guard Fleets ");
            for (idx, fleet) in many.iter().enumerate() {
                if idx > 0 {
                    if idx + 1 == many.len() {
                        label.push_str(", and ");
                    } else {
                        label.push_str(", ");
                    }
                }
                label.push_str(&fleet.to_string());
            }
            label
        }
    }
}

pub fn starbase_eta_label(coords: [u8; 2], destination_coords: [u8; 2]) -> String {
    crate::estimate_direct_eta(coords, destination_coords, 1, true).to_string()
}

pub fn starbase_operation_label(coords: [u8; 2], destination_coords: [u8; 2]) -> String {
    if destination_coords == coords {
        "Protection & Enhancement".to_string()
    } else {
        "Starbase in transit".to_string()
    }
}

pub(super) fn ordinal_number(value: usize) -> String {
    let suffix = match value % 100 {
        11..=13 => "th",
        _ => match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{value}{suffix}")
}
