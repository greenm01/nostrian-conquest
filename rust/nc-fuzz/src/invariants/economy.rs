//! Spending stays within each planet's build budget and no treasury wraps.
//!
//! A planet's build budget is `min(treasury, build capacity)`. Submitted
//! build orders may only grow the queue up to the budget, and one
//! maintenance turn may only spend up to the budget it started with.

use std::collections::BTreeSet;

use nc_data::{CoreGameData, InvalidPlayerStateEvent, PlayerMode, stored_production_points};

use super::{TurnObservation, Violation};

/// A stored-production value this large means a subtraction wrapped.
const WRAPPED_TREASURY: u32 = 1 << 31;

/// Builds submitted this turn stayed within each planet's budget.
pub fn check_submitted_spending(
    turn_start: &CoreGameData,
    submitted: &CoreGameData,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for record in 1..=submitted.planets.len() {
        let committed_start = committed_points(turn_start, record);
        let committed = committed_points(submitted, record);
        if committed <= committed_start {
            continue;
        }
        let budget = budget(turn_start, record).max(budget(submitted, record));
        if committed > budget {
            violations.push(Violation::OverBudget {
                planet_record: record,
                spent: committed,
                budget,
            });
        }
    }
    violations
}

/// Maintenance spent no more than each planet's budget and left every
/// treasury non-negative.
pub fn check_maintenance_spending(observation: &TurnObservation) -> Vec<Violation> {
    let before = observation.before;
    let after = observation.after;
    let events = observation.events;
    // Planets whose queue maintenance changes for reasons other than spending.
    let mut excluded = BTreeSet::new();
    excluded.extend(events.bombard_events.iter().map(|event| event.planet_idx));
    excluded.extend(
        events
            .assault_report_events
            .iter()
            .map(|event| event.planet_idx),
    );
    excluded.extend(
        events
            .invalid_player_state_events
            .iter()
            .filter_map(|event| match event {
                InvalidPlayerStateEvent::PlanetInput { planet_idx, .. } => Some(*planet_idx),
                _ => None,
            }),
    );

    let mut violations = Vec::new();
    for (idx, planet) in after.planets.iter().enumerate() {
        let record = idx + 1;
        let stored = stored_production_points(planet);
        if stored >= WRAPPED_TREASURY {
            violations.push(Violation::NegativeTreasury {
                planet_record: record,
                stored,
            });
        }
        let owner = before.planets[idx].owner.0;
        if excluded.contains(&idx)
            || owner == 0
            || owner != planet.owner.0
            || runs_ai(before, owner)
        {
            continue;
        }
        let spent =
            committed_points(before, record).saturating_sub(committed_points(after, record));
        let budget = budget(before, record);
        if spent > budget {
            violations.push(Violation::OverBudget {
                planet_record: record,
                spent,
                budget,
            });
        }
    }
    violations
}

/// The autopilot and rogue AI queue builds of their own during maintenance.
fn runs_ai(state: &CoreGameData, empire: u8) -> bool {
    state
        .players
        .get(usize::from(empire) - 1)
        .is_some_and(|player| player.autopilot || player.mode == PlayerMode::Rogue)
}

fn committed_points(state: &CoreGameData, record: usize) -> u32 {
    let planet = &state.planets[record - 1];
    (0..10)
        .map(|slot| u32::from(planet.build_queue[slot].points_owed))
        .sum()
}

fn budget(state: &CoreGameData, record: usize) -> u32 {
    state.planet_build_budget(record).unwrap_or(0)
}
