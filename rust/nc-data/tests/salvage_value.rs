//! What a Salvage fleet yields on arrival, against classic ECMAINT
//! (2026-10-10, t069): half each ship's build cost rounded up, and the
//! armies of loaded transports land on the world.

mod common;

use common::move_scenario::*;
use nc_data::{
    Order, Real48, SalvageResolvedEvent, recompute_max_speed, set_armies_aboard,
    set_troop_transports, stored_production_points,
};

#[test]
fn salvage_values_each_hull_as_classic_does() {
    // Ships as (battleships, cruisers, destroyers, scouts, etacs), then
    // transports, how many of them loaded, and the points the fleet yields.
    let cases: [([u16; 5], u16, u16, u32); 7] = [
        ([1, 0, 0, 0, 0], 0, 0, 23),
        ([0, 1, 0, 0, 0], 0, 0, 8),
        ([0, 0, 1, 0, 0], 0, 0, 3),
        ([0, 0, 0, 1, 0], 0, 0, 8),
        ([0, 0, 0, 0, 1], 0, 0, 10),
        ([0, 0, 0, 0, 0], 1, 0, 3),
        ([0, 0, 0, 0, 0], 1, 1, 3),
    ];
    for ([bb, ca, dd, sc, etac], transports, loaded, points) in cases {
        let mut game_data = scenario();
        let fleet = &mut game_data.fleets[0];
        set_ships(fleet, bb, ca, dd, sc, etac);
        set_troop_transports(fleet, transports);
        set_armies_aboard(fleet, loaded);
        recompute_max_speed(fleet);
        place_fleet(
            &mut game_data,
            [15, 13],
            Real48::ZERO,
            Order::Salvage,
            HOME,
            3,
        );

        let events = run(&mut game_data);

        assert!(
            matches!(
                events.salvage_events[..],
                [SalvageResolvedEvent::Succeeded { recovered_points, armies_landed, .. }]
                    if recovered_points == points && armies_landed == loaded
            ),
            "{bb} {ca} {dd} {sc} {etac} {transports} {loaded}: {:?}",
            events.salvage_events
        );
    }
}

#[test]
fn salvage_credits_the_classic_value_per_hull_and_lands_the_armies() {
    // One of each hull and two loaded transports (probe: 61 points, 2
    // armies): 23 + 8 + 3 + 3 + 10 + 8 for the hulls, 3 + 3 for the loaded
    // transports.
    let mut game_data = scenario();
    let armies_before = planet_at(&mut game_data, HOME).armies;
    let stored_before = stored_production_points(planet_at(&mut game_data, HOME));
    let fleet = &mut game_data.fleets[0];
    set_ships(fleet, 1, 1, 1, 1, 1);
    set_troop_transports(fleet, 3);
    set_armies_aboard(fleet, 2);
    recompute_max_speed(fleet);
    place_fleet(
        &mut game_data,
        [15, 13],
        Real48::ZERO,
        Order::Salvage,
        HOME,
        3,
    );

    let events = run(&mut game_data);

    assert!(matches!(
        events.salvage_events[..],
        [SalvageResolvedEvent::Succeeded {
            coords: HOME,
            recovered_points: 61,
            armies_landed: 2,
            ..
        }]
    ));
    assert_eq!(
        stored_production_points(planet_at(&mut game_data, HOME)),
        stored_before + 61
    );
    assert_eq!(planet_at(&mut game_data, HOME).armies, armies_before + 2);
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("will yield 61 production point(s). In addition, we have transfered the 2 armies we were transporting to the planet's surface."),
        "{text}"
    );
}

#[test]
fn salvage_report_says_army_for_one_loaded_transport() {
    let mut game_data = scenario();
    let fleet = &mut game_data.fleets[0];
    set_ships(fleet, 0, 0, 1, 0, 0);
    set_troop_transports(fleet, 2);
    set_armies_aboard(fleet, 1);
    recompute_max_speed(fleet);
    place_fleet(
        &mut game_data,
        [15, 13],
        Real48::ZERO,
        Order::Salvage,
        HOME,
        3,
    );

    let events = run(&mut game_data);

    // A destroyer and two transports, one of them loaded.
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("will yield 9 production point(s). In addition, we have transfered the 1 army we were transporting to the planet's surface."),
        "{text}"
    );
}
