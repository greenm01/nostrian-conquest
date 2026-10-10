use nc_data::{CampaignOutcome, CampaignOutlook, CampaignState, GameStateBuilder};
use nc_engine::run_maintenance_turn;

fn baseline_game() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build")
}

#[test]
fn owned_planets_mean_stable_campaign_state() {
    let game_data = baseline_game();
    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::Stable)
    );
    assert_eq!(game_data.campaign_contenders(), vec![1, 2, 3, 4]);
    assert_eq!(game_data.sole_contender(), None);
}

#[test]
fn no_planets_but_etac_means_marginal_existence() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::MarginalExistence)
    );
}

#[test]
fn no_planets_but_loaded_transports_mean_marginal_existence() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_cruiser_count(0);
            fleet.set_destroyer_count(0);
            fleet.set_battleship_count(0);
            fleet.set_scout_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }
    let fleet = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1)
        .expect("player 1 fleet");
    fleet.set_troop_transport_count(2);
    fleet.set_army_count(2);

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::MarginalExistence)
    );
}

#[test]
fn no_planets_and_no_recovery_path_means_defection_risk() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::DefectionRisk)
    );
}

#[test]
fn no_planets_and_no_fleet_presence_means_defeated() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
            fleet.set_destroyer_count(0);
            fleet.set_cruiser_count(0);
            fleet.set_battleship_count(0);
            fleet.set_scout_count(0);
        }
    }

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::Defeated)
    );
}

#[test]
fn rogue_and_civil_disorder_states_are_preserved() {
    let mut game_data = baseline_game();
    game_data.player.records[0].set_owner_empire_raw(0xff);
    game_data.player.records[1].set_owner_empire_raw(0x00);

    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::Rogue)
    );
    assert_eq!(
        game_data.empire_campaign_state(2),
        Some(CampaignState::CivilDisorder)
    );
}

#[test]
fn empty_rogue_empire_is_not_a_campaign_contender() {
    let mut game_data = baseline_game();
    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
            }
        }
    }
    game_data.player.records[1].set_player_mode_raw(0xff);

    assert_eq!(
        game_data.empire_campaign_state(2),
        Some(CampaignState::Rogue)
    );
    assert_eq!(game_data.campaign_contenders(), vec![1]);
    assert_eq!(
        game_data.campaign_outlook(),
        CampaignOutlook::SoleContender(1)
    );
}

#[test]
fn maintenance_marks_empire_without_recovery_path_defeated() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let player = &game_data.player.records[0];
    assert_eq!(player.owner_mode_raw(), 0x00);
    assert_eq!(player.legacy_status_name_summary(), "");
    assert_eq!(events.civil_disorder_events.len(), 0);
    assert_eq!(events.empire_elimination_events.len(), 1);
    assert_eq!(events.empire_elimination_events[0].defeated_empire_raw, 1);
    assert_eq!(
        game_data.empire_campaign_state(1),
        Some(CampaignState::CivilDisorder)
    );
}

#[test]
fn sole_contender_is_reported_when_only_one_empire_can_still_contest() {
    let mut game_data = baseline_game();
    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }

    assert_eq!(game_data.campaign_contenders(), vec![1]);
    assert_eq!(game_data.sole_contender(), Some(1));
    assert_eq!(
        game_data.campaign_outlook(),
        CampaignOutlook::SoleContender(1)
    );
    assert_eq!(
        game_data.campaign_outcome(),
        CampaignOutcome::RecognizedEmperor(1)
    );
}

#[test]
fn maintenance_emits_campaign_outlook_event_when_one_contender_remains() {
    let mut game_data = baseline_game();
    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(
        game_data.campaign_outlook(),
        CampaignOutlook::SoleContender(1)
    );
    assert_eq!(events.campaign_outlook_events.len(), 0);
}

#[test]
fn multiple_contenders_keep_campaign_outlook_contested() {
    let game_data = baseline_game();
    assert_eq!(game_data.campaign_outlook(), CampaignOutlook::Contested);
    assert_eq!(game_data.campaign_outcome(), CampaignOutcome::Ongoing);
}

#[test]
fn maintenance_emits_campaign_outcome_when_last_stable_contender_remains() {
    let mut game_data = baseline_game();
    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(
        game_data.campaign_outcome(),
        CampaignOutcome::RecognizedEmperor(1)
    );
    assert_eq!(events.campaign_outcome_events.len(), 0);
    assert_eq!(events.game_victory_notice_events.len(), 0);
}

#[test]
fn civil_disorder_empire_loses_one_fleet_to_defection_each_turn() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }

    let first_events =
        run_maintenance_turn(&mut game_data).expect("first maintenance should succeed");
    assert_eq!(first_events.civil_disorder_events.len(), 0);
    assert_eq!(first_events.empire_elimination_events.len(), 1);
    let fleet_count_after_collapse = game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .count();

    let second_events =
        run_maintenance_turn(&mut game_data).expect("second maintenance should succeed");
    let fleet_count_after_defection = game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .count();

    assert_eq!(second_events.fleet_defection_events.len(), 1);
    assert_eq!(fleet_count_after_defection + 1, fleet_count_after_collapse);
}
