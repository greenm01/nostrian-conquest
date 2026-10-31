use nc_data::{
    Coords, GameStateBuilder, ProductionItemKind, set_armies_aboard, set_troop_transports,
};
use nc_engine::{
    ArmyTransportMode, default_fleet_transport_fleet_number,
    planet_build_max_selectable_unit_number, planet_build_specify_entries, planet_build_view,
    planet_commission_draft_state, production_item_kind_code,
};

#[test]
fn production_item_kind_code_matches_runtime_codes() {
    assert_eq!(production_item_kind_code(ProductionItemKind::Destroyer), 1);
    assert_eq!(production_item_kind_code(ProductionItemKind::Cruiser), 2);
    assert_eq!(production_item_kind_code(ProductionItemKind::Battleship), 3);
    assert_eq!(production_item_kind_code(ProductionItemKind::Scout), 4);
    assert_eq!(production_item_kind_code(ProductionItemKind::Transport), 5);
    assert_eq!(production_item_kind_code(ProductionItemKind::Etac), 6);
    assert_eq!(
        production_item_kind_code(ProductionItemKind::GroundBattery),
        7
    );
    assert_eq!(production_item_kind_code(ProductionItemKind::Army), 8);
    assert_eq!(production_item_kind_code(ProductionItemKind::Starbase), 9);
}

#[test]
fn planet_build_view_counts_queue_and_stardock_units() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let planet = &mut game_data.planets[0];
    planet.build_queue[0].kind = ProductionItemKind::Battleship;
    planet.build_queue[0].points_owed = 90;
    planet.stardock.cruisers = 5;

    let row = game_data.empire_planet_economy_rows(1).remove(0);
    let view = planet_build_view(&game_data, &row).expect("view");

    assert_eq!(view.committed_points, 90);
    assert_eq!(view.building_count, 2);
    assert_eq!(view.docked_count, 5);
}

#[test]
fn planet_build_view_budget_capped_by_treasury_not_revenue() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let planet = &mut game_data.planets[0];
    planet.build_queue[0].kind = ProductionItemKind::Etac;
    planet.build_queue[0].points_owed = 20;

    // treasury=65 < build_capacity=100, so budget = 65 regardless of revenue
    let row = {
        let mut row = game_data.empire_planet_economy_rows(1).remove(0);
        row.stored_production_points = 65;
        row.yearly_tax_revenue = 50;
        row.build_capacity = 100;
        row
    };

    let view = planet_build_view(&game_data, &row).expect("view");

    assert_eq!(view.budget, 65);
    assert_eq!(view.treasury_left, 45);
    assert_eq!(view.points_left, 45);
}

#[test]
fn build_specify_entries_include_all_build_choices_and_blank_unaffordable_rows() {
    let entries = planet_build_specify_entries(
        10,
        10,
        &[nc_engine::PlanetBuildOrderLine {
            kind: ProductionItemKind::GroundBattery,
            points_remaining: 20,
        }],
    );

    assert_eq!(entries.len(), 9);
    assert_eq!(entries[0].number, 1);
    assert!(entries[0].selectable);
    assert_eq!(entries[5].number, 6);
    assert!(!entries[5].selectable);
    assert_eq!(entries[8].number, 10);
    assert_eq!(entries[8].queued_qty, 1);
    assert!(!entries[8].selectable);
    assert_eq!(planet_build_max_selectable_unit_number(&entries), 9);
}

#[test]
fn commission_draft_state_rolls_up_ships_and_preserves_starbase_rows() {
    let state = planet_commission_draft_state(&[
        nc_engine::PlanetCommissionSlotEntry {
            slot_0_based: 0,
            kind: ProductionItemKind::Destroyer,
            qty: 2,
        },
        nc_engine::PlanetCommissionSlotEntry {
            slot_0_based: 1,
            kind: ProductionItemKind::Destroyer,
            qty: 3,
        },
        nc_engine::PlanetCommissionSlotEntry {
            slot_0_based: 2,
            kind: ProductionItemKind::Starbase,
            qty: 1,
        },
    ]);

    assert_eq!(state.draft_slots, vec![0, 1]);
    assert_eq!(state.rows.len(), 2);
    assert_eq!(
        state.rows[0],
        nc_engine::PlanetCommissionDraftEntry {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Destroyer,
            remaining_qty: 5,
            fleet_qty: 0,
        }
    );
    assert_eq!(
        state.rows[1],
        nc_engine::PlanetCommissionDraftEntry {
            direct_slot_0_based: Some(2),
            kind: ProductionItemKind::Starbase,
            remaining_qty: 1,
            fleet_qty: 0,
        }
    );
}

#[test]
fn default_transport_fleet_prefers_highest_eligible_capacity() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let homeworld_coords = game_data.planets[0].sector.to_array();
    let homeworld = &mut game_data.planets[0];
    homeworld.armies = 50;

    for (idx, fleet) in game_data.fleets.iter_mut().enumerate() {
        fleet.owner.0 = 1;
        fleet.fleet_number.0 = (idx + 1) as u16;
        fleet.sector = Coords::from_array(homeworld_coords);
        set_troop_transports(fleet, 0);
        set_armies_aboard(fleet, 0);
    }
    set_troop_transports(&mut game_data.fleets[0], 4);
    set_troop_transports(&mut game_data.fleets[1], 8);

    let rows = game_data.empire_planet_economy_rows(1);
    let fleet_number =
        default_fleet_transport_fleet_number(&game_data, 1, ArmyTransportMode::Load, &rows);

    assert_eq!(fleet_number, Some(2));
}

#[test]
fn a_first_starbase_past_the_build_capacity_is_offered() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    // Barsville in play: no base, capacity 46, treasury 86.
    let row = {
        let mut row = game_data.empire_planet_economy_rows(1).remove(0);
        row.stored_production_points = 86;
        row.build_capacity = 46;
        row.has_friendly_starbase = false;
        row
    };
    assert_eq!(
        game_data.planets[row.planet_record_index_1_based - 1]
            .stardock
            .starbases,
        0
    );

    let view = planet_build_view(&game_data, &row).expect("view");
    let entries = planet_build_specify_entries(view.points_left, view.starbase_points_left, &[]);

    assert_eq!(view.points_left, 46);
    let starbase = entries
        .iter()
        .find(|entry| entry.kind == ProductionItemKind::Starbase)
        .expect("starbase row");
    assert!(starbase.selectable);
}
