mod common;

use common::create_test_game;
use nc_data::hosted::{
    GameTier, RecruitingMode, RosterStore, record_player_joined, upsert_player_seen,
};
use nc_data::{CampaignStore, PlayerActivityState, ReportBlockRow};

fn seed_runtime_with_activity(
    game_dir: &std::path::Path,
    game_id: &str,
    year: u16,
    activity_states: &[PlayerActivityState],
) {
    let game_data =
        nc_engine::build_seeded_new_game(4, year, 12345).expect("game state should build");

    let store = CampaignStore::open_default_in_dir(game_dir).expect("campaign store should open");
    let intel_by_viewer = (1..=4)
        .map(|viewer_empire_id| {
            nc_data::merge_player_intel_from_runtime(&game_data, viewer_empire_id, year, None, None)
        })
        .collect::<Vec<_>>();
    store
        .save_runtime_state_structured_with_intel_and_activity(
            game_data.conquest.game_year,
            &game_data,
            &[] as &[ReportBlockRow],
            &[],
            &intel_by_viewer,
            activity_states,
        )
        .expect("runtime state should save");
    store
        .save_campaign_settings(&nc_data::CampaignSettings::new(game_id, game_id))
        .expect("campaign settings should save");
}

fn set_tier(store: &nc_data::hosted::HostedStore, game_id: &str, tier: GameTier) {
    let mut settings =
        nc_data::hosted::get_settings(store.connection(), game_id).expect("settings");
    settings.game_tier = tier;
    settings.recruiting = RecruitingMode::NewPlayers;
    nc_data::hosted::update_settings(store.connection(), game_id, &settings).expect("update");
}

fn seed_joined_roster(game_dir: &std::path::Path, game_id: &str, npub: &str, seat: u32) {
    let roster_db = game_dir.parent().expect("games root").join("roster.db");
    let roster = RosterStore::open(&roster_db).expect("open roster");
    upsert_player_seen(roster.connection(), npub, Some("Pilot"), game_id).expect("seen");
    record_player_joined(roster.connection(), npub, game_id, seat).expect("joined");
}

#[test]
fn sandbox_recycles_claimed_seat_after_ten_turns_without_abandonment() {
    let (_temp, game_dir, store) = create_test_game("sandbox-maint-tenure", 4);
    let game_id = "sandbox-maint-tenure";
    let player_pubkey = "sandbox-tenure-player";

    set_tier(&store, game_id, GameTier::Sandbox);
    seed_runtime_with_activity(
        &game_dir,
        game_id,
        3000,
        &[
            PlayerActivityState {
                player_record_index_1_based: 1,
                last_participation_year: 3008,
                inactivity_autopilot_pending_clear: false,
            },
            PlayerActivityState {
                player_record_index_1_based: 2,
                last_participation_year: 3000,
                inactivity_autopilot_pending_clear: false,
            },
            PlayerActivityState {
                player_record_index_1_based: 3,
                last_participation_year: 3000,
                inactivity_autopilot_pending_clear: false,
            },
            PlayerActivityState {
                player_record_index_1_based: 4,
                last_participation_year: 3000,
                inactivity_autopilot_pending_clear: false,
            },
        ],
    );
    nc_data::hosted::claim_seat(store.connection(), game_id, 1, player_pubkey, 3000)
        .expect("claim");
    seed_joined_roster(&game_dir, game_id, player_pubkey, 1);

    nc_host::commands::maint::run(&[game_dir.to_str().expect("game dir"), "10"]).expect("maint");

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 1)
        .expect("get seat")
        .expect("seat exists");
    assert_eq!(seat.status, nc_data::hosted::SeatStatus::Pending);
    assert_eq!(seat.player_pubkey, None);
    assert_eq!(seat.claimed_year, None);

    let roster_db = game_dir.parent().expect("games root").join("roster.db");
    let roster = RosterStore::open(&roster_db).expect("open roster");
    let entry = nc_data::hosted::get_roster_entry(roster.connection(), player_pubkey)
        .expect("roster entry")
        .expect("entry exists");
    assert_eq!(entry.games_joined, 1);
    assert_eq!(entry.games_abandoned, 0);
    let events = nc_data::hosted::list_roster_events_for_npub(roster.connection(), player_pubkey)
        .expect("events");
    assert!(events.iter().all(|event| event.event_type != "abandoned"));
}

#[test]
fn league_games_do_not_apply_sandbox_turn_cap() {
    let (_temp, game_dir, store) = create_test_game("league-maint-tenure", 4);
    let game_id = "league-maint-tenure";
    let player_pubkey = "league-tenure-player";

    seed_runtime_with_activity(
        &game_dir,
        game_id,
        3000,
        &[
            PlayerActivityState {
                player_record_index_1_based: 1,
                last_participation_year: 3008,
                inactivity_autopilot_pending_clear: false,
            },
            PlayerActivityState {
                player_record_index_1_based: 2,
                last_participation_year: 3000,
                inactivity_autopilot_pending_clear: false,
            },
            PlayerActivityState {
                player_record_index_1_based: 3,
                last_participation_year: 3000,
                inactivity_autopilot_pending_clear: false,
            },
            PlayerActivityState {
                player_record_index_1_based: 4,
                last_participation_year: 3000,
                inactivity_autopilot_pending_clear: false,
            },
        ],
    );
    nc_data::hosted::claim_seat(store.connection(), game_id, 1, player_pubkey, 3000)
        .expect("claim");

    nc_host::commands::maint::run(&[game_dir.to_str().expect("game dir"), "10"]).expect("maint");

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 1)
        .expect("get seat")
        .expect("seat exists");
    assert_eq!(seat.status, nc_data::hosted::SeatStatus::Claimed);
    assert_eq!(seat.player_pubkey.as_deref(), Some(player_pubkey));
    assert_eq!(seat.claimed_year, Some(3000));
}
