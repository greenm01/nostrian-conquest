//! Planets change hands during maintenance only through colonization or a
//! successful invasion or blitz, each of which reports an event.

use std::collections::BTreeMap;

use nc_data::ColonizationResolvedEvent;

use super::{TurnObservation, Violation};

pub fn check(observation: &TurnObservation) -> Vec<Violation> {
    let events = observation.events;
    let mut explained = BTreeMap::new();
    for event in &events.ownership_change_events {
        explained.insert(event.planet_idx, event.new_owner_empire_raw);
    }
    for event in &events.colonization_events {
        if let ColonizationResolvedEvent::Succeeded {
            planet_idx,
            colonizer_empire_raw,
            ..
        } = *event
        {
            explained.insert(planet_idx, colonizer_empire_raw);
        }
    }

    let before = &observation.before.planets;
    let after = &observation.after.planets;
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter_map(|(idx, (was, now))| {
            let from = was.owner_empire_slot_raw();
            let to = now.owner_empire_slot_raw();
            (from != to && explained.get(&idx) != Some(&to)).then_some(
                Violation::UnexplainedOwnershipChange {
                    planet_record: idx + 1,
                    from,
                    to,
                },
            )
        })
        .collect()
}
