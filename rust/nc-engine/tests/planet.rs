use nc_data::{GameStateBuilder, ProductionItemKind};
use nc_engine::{
    ArmyTransportMode, default_fleet_transport_fleet_number,
    planet_build_max_selectable_unit_number, planet_build_specify_entries, planet_build_view,
    planet_commission_draft_state, production_item_kind_raw,
};

#[test]
fn production_item_kind_raw_matches_runtime_codes() {
    assert_eq!(production_item_kind_raw(ProductionItemKind::Destroyer), 1);
    assert_eq!(production_item_kind_raw(ProductionItemKind::Cruiser), 2);
    assert_eq!(production_item_kind_raw(ProductionItemKind::Battleship), 3);
    assert_eq!(production_item_kind_raw(ProductionItemKind::Scout), 4);
    assert_eq!(production_item_kind_raw(ProductionItemKind::Transport), 5);
    assert_eq!(production_item_kind_raw(ProductionItemKind::Etac), 6);
    assert_eq!(
        production_item_kind_raw(ProductionItemKind::GroundBattery),
        7
    );
    assert_eq!(production_item_kind_raw(ProductionItemKind::Army), 8);
    assert_eq!(production_item_kind_raw(ProductionItemKind::Starbase), 9);
}

#[test]
fn planet_build_view_counts_queue_and_stardock_units() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let planet = &mut game_data.planets.records[0];
    planet.set_build_kind_raw(0, 3);
    planet.set_build_count_raw(0, 90);
    planet.set_stardock_count_raw(1, 5);

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
    let planet = &mut game_data.planets.records[0];
    planet.set_build_kind_raw(0, 6);
    planet.set_build_count_raw(0, 20);

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
    let homeworld_coords = game_data.planets.records[0].coords_raw();
    let homeworld = &mut game_data.planets.records[0];
    homeworld.set_army_count_raw(50);

    for (idx, fleet) in game_data.fleets.records.iter_mut().enumerate() {
        fleet.set_owner_empire_raw(1);
        fleet.set_local_slot_word_raw((idx + 1) as u16);
        fleet.set_current_location_coords_raw(homeworld_coords);
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
    }
    game_data.fleets.records[0].set_troop_transport_count(4);
    game_data.fleets.records[1].set_troop_transport_count(8);

    let rows = game_data.empire_planet_economy_rows(1);
    let fleet_number =
        default_fleet_transport_fleet_number(&game_data, 1, ArmyTransportMode::Load, &rows);

    assert_eq!(fleet_number, Some(2));
}
