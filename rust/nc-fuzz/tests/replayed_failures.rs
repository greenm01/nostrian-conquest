//! Fuzz failures replayed from their written scenarios, so each one hits
//! its path on every run.

use std::path::Path;

use nc_fuzz::{Violation, replay_scenario};
use nc_harness::CombatScenarioSpec;

fn replay(scenario: &str) -> Vec<Violation> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/scenarios")
        .join(scenario)
        .join("scenario.kdl");
    let spec = CombatScenarioSpec::load_kdl(&path).expect("scenario should parse");
    replay_scenario(&spec).expect("scenario should replay")
}

/// t049: a Bombard fleet reports its arrival in the target sector in week
/// 16, then a re-check of the guard force aborts the mission without a
/// report and the fleet heads home, ending the year a sector away. The
/// event names the right fleet at the right tick.
#[test]
fn a_fleet_may_move_on_after_a_mid_year_event() {
    assert_eq!(replay("bombard-arrival-then-roe-abort"), Vec::new());
}
