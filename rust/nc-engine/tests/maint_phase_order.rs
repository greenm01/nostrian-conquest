//! The yearly phase order (`docs/dev/ec-turn-cycle-spec.md`): ECMAINT's
//! planet pass (`1852:0000`) runs before the year driver (`0824:1004`).

#[allow(dead_code)]
#[path = "encounter/support.rs"]
mod support;

use nc_data::{
    CompactUnitSummaryStyle, Mission, MissionOutcome, Order, PlanetIntelSource, Real48,
    encode_real48, format_stardock_summary,
};
use support::{Fleet, Ships, WORLD, board_with_world, put, years};

/// A ship the world finishes this year is in its stardock before the year's
/// first tick: a scout that reaches orbit in tick 1 reports it.
#[test]
fn ships_built_this_year_are_in_the_stardock_before_the_first_tick() {
    let mut scenario = board_with_world();
    let planet_idx = 13;
    let planet = &mut scenario.planets.records[planet_idx];
    assert_eq!(planet.coords_raw(), WORLD);
    assert_eq!(format_stardock_summary(planet, CompactUnitSummaryStyle::Words), "Nothing");
    // Two destroyers (5 points each), well inside the world's budget.
    planet.set_build_count_raw(0, 10);
    planet.set_build_kind_raw(0, 1);
    planet.set_stored_production_points(200);

    // Empire 1's scout sits at the sector centre, a tick's dive short of
    // orbit, so its Scout Solar System order resolves in tick 1.
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, WORLD, true, Order::ScoutSolarSystem, Ships { sc: 1, ..Ships::default() })
            .max_speed(5)
            .heading(Order::ScoutSolarSystem, WORLD, true, 5),
    );
    scenario.fleets.records[0].set_depth(Real48::from_bytes(
        encode_real48(0.95).expect("depth should encode"),
    ));

    let (game_data, events) = years(&scenario, &[], 0..1).next().expect("one year");

    let dock_after = format_stardock_summary(
        &game_data.planets.records[planet_idx],
        CompactUnitSummaryStyle::Words,
    );
    assert_eq!(dock_after, "2 destroyers");

    assert!(
        events.mission_events.iter().any(|event| {
            event.owner_empire_raw == 1
                && event.fleet_idx == 0
                && event.kind == Mission::ScoutSolarSystem
                && event.outcome == MissionOutcome::Succeeded
        }),
        "the scout should reach orbit this year"
    );

    // The snapshot is taken in the tick the scout arrives (its report week
    // follows the timing model, not the tick), so it shows the dock as the
    // ticks saw it.
    let report = events
        .planet_intel_events
        .iter()
        .find(|event| {
            event.viewer_empire_raw == 1
                && event.planet_idx == planet_idx
                && event.source == PlanetIntelSource::ScoutSolarSystem
        })
        .expect("the scout should report the world");
    let snapshot = report
        .observed_snapshot
        .as_ref()
        .expect("a scout report carries a snapshot");
    assert_eq!(snapshot.known_docked_summary.as_deref(), Some(dock_after.as_str()));
}
