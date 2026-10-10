use nc_data::{
    CampaignOutcomeEvent, CampaignOutlookEvent, GameStateBuilder, GameVictoryNoticeEvent,
    InvalidPlayerStateEvent, MaintenanceEvents, PlanetPlayerInputValidationError,
};
use nc_engine::{apply_results_reviewable_flags, build_results_report_blocks};

fn joined_game_data() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3059)
        .build_initialized_baseline()
        .expect("baseline should build")
}

fn outlook_events() -> MaintenanceEvents {
    let mut events = MaintenanceEvents::default();
    events.campaign_outlook_events.push(CampaignOutlookEvent {
        empire_raw: 2,
        stardate_week: Some(4),
    });
    events
}

fn has_results(game_data: &nc_data::CoreGameData, empire_raw: u8) -> bool {
    game_data.player.records[empire_raw as usize - 1].has_classic_results_chain_state()
}

#[test]
fn campaign_outlook_reaches_every_joined_empire() {
    let mut game_data = joined_game_data();
    let rows = build_results_report_blocks(&game_data, &outlook_events());
    assert!(!rows.is_empty());

    apply_results_reviewable_flags(&mut game_data, &rows);
    for empire_raw in 1..=4 {
        assert!(
            has_results(&game_data, empire_raw),
            "empire {empire_raw} should have the outlook report"
        );
    }
}

#[test]
fn campaign_outlook_claims_the_throne_only_for_the_leader() {
    let game_data = joined_game_data();
    let rows = build_results_report_blocks(&game_data, &outlook_events());
    let text_for = |empire_raw: u8| {
        rows.iter()
            .filter(|row| row.is_visible_to_viewer(empire_raw))
            .map(|row| row.decoded_text.replace('\n', " "))
            .collect::<Vec<_>>()
            .join(" ")
    };

    let leader = text_for(2);
    assert!(leader.contains("our claim"), "{leader}");
    for empire_raw in [1, 3, 4] {
        let other = text_for(empire_raw);
        assert!(other.contains("Empire #2"), "{other}");
        assert!(!other.contains("our claim"), "{other}");
    }
}

#[test]
fn campaign_outlook_skips_unjoined_empires() {
    let mut game_data = joined_game_data();
    game_data.player.records[0].set_player_mode_raw(0x00);
    game_data.player.records[3].set_player_mode_raw(0xff);

    let rows = build_results_report_blocks(&game_data, &outlook_events());
    assert!(!rows.is_empty());

    apply_results_reviewable_flags(&mut game_data, &rows);
    assert!(!has_results(&game_data, 1));
    assert!(has_results(&game_data, 2));
    assert!(has_results(&game_data, 3));
    assert!(!has_results(&game_data, 4));
}

#[test]
fn emperor_outcome_is_reported_only_through_targeted_victory_notices() {
    let game_data = joined_game_data();
    let mut events = MaintenanceEvents::default();
    events.campaign_outcome_events.push(CampaignOutcomeEvent {
        emperor_empire_raw: 1,
        stardate_week: Some(4),
    });
    for recipient_empire_raw in [1, 2] {
        events
            .game_victory_notice_events
            .push(GameVictoryNoticeEvent {
                recipient_empire_raw,
                winner_empire_raw: 1,
                stardate_week: Some(4),
            });
    }

    let rows = build_results_report_blocks(&game_data, &events);
    assert!(
        rows.iter().all(|row| row.viewer_empire_id != 0),
        "emperor outcome must not be broadcast: {rows:?}"
    );
    let visible = |empire_raw: u8| {
        rows.iter()
            .filter(|row| row.is_visible_to_viewer(empire_raw))
            .count()
    };
    assert_eq!(visible(1), 1);
    assert_eq!(visible(2), 1);
    // Empire 3 was already defeated and got no targeted notice.
    assert_eq!(visible(3), 0);
}

#[test]
fn invalid_input_on_unowned_planet_is_not_reported_to_empire_one() {
    let game_data = joined_game_data();
    let mut events = MaintenanceEvents::default();
    events
        .invalid_player_state_events
        .push(InvalidPlayerStateEvent::PlanetInput {
            planet_idx: 0,
            owner_empire_raw: 0,
            coords: [7, 7],
            reason: PlanetPlayerInputValidationError::MissingBuildKindForCount,
        });

    let rows = build_results_report_blocks(&game_data, &events);
    assert!(rows.is_empty(), "unexpected rows: {rows:?}");
}
