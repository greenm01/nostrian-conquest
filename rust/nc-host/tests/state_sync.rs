mod common;

use common::create_test_game;
use common::seed_runtime_snapshot;
use nc_data::{
    CampaignStore, PlayerMode, QueuedPlayerMail, ReportBlockRow, stored_production_points,
};
use nc_host::game::effects::GameEffects;
use nc_host::game::worker::GameWorker;
use nc_nostr::first_join::{FirstJoinSetupRequest, FirstJoinSetupResult, FirstJoinSetupStatus};
use nc_nostr::state_sync::{StateErrorCode, StateErrorPayload, StateRequest};

#[test]
fn test_state_sync_structs() {
    use nc_nostr::state_sync::{
        GameState, HostedPlayerState, HostedStarmapState, HostedStatePayload, StateDelta,
        StateDeltas,
    };

    let state = GameState {
        game_id: "test".to_string(),
        turn: 5,
        year: 3005,
        player_seat: 1,
        player_name: "Test Empire".to_string(),
        state_hash: "abc123".to_string(),
        state: HostedStatePayload {
            player: HostedPlayerState {
                seat: 1,
                empire_name: "Test Empire".to_string(),
                handle: None,
                mode: "active".to_string(),
                tax_rate: 10,
                planet_count: 1,
                starbase_count: 0,
                homeworld_planet_index: 1,
                last_run_year: 3005,
                diplomacy: Vec::new(),
            },
            roster: Vec::new(),
            starmap: HostedStarmapState {
                map_width: 18,
                map_height: 18,
                viewer_empire_id: 1,
                year: 3005,
                worlds: Vec::new(),
            },
            owned_planets: Vec::new(),
            owned_fleets: Vec::new(),
        },
        queued_mail: vec![],
        report_blocks: vec![],
    };

    assert_eq!(state.turn, 5);
    assert_eq!(state.year, 3005);

    let deltas = StateDeltas {
        planets: Some(vec![]),
        fleets: Some(vec![]),
        ..StateDeltas::default()
    };

    let delta = StateDelta {
        game_id: "test".to_string(),
        turn: 6,
        year: 3006,
        player_name: "Test Empire".to_string(),
        base_hash: "abc123".to_string(),
        state_hash: "def456".to_string(),
        deltas,
    };

    assert_eq!(delta.turn, 6);
    assert_eq!(delta.year, 3006);
    assert_eq!(delta.base_hash, "abc123");
}

#[test]
fn test_game_settings_for_state() {
    let (_temp, _game_dir, store) = create_test_game("state-sync-test", 4);
    let game_id = "state-sync-test";

    let settings = nc_data::hosted::get_settings(store.connection(), game_id).expect("should get");

    assert!(settings.maintenance_enabled);
    assert_eq!(settings.maintenance_interval_minutes, 1440);
    assert_eq!(
        settings.lobby_visibility,
        nc_data::hosted::LobbyVisibility::Public
    );
}

#[test]
fn test_seat_lookup_for_player() {
    let (_temp, _game_dir, store) = create_test_game("state-sync-seat", 4);
    let game_id = "state-sync-seat";

    let player_pubkey = "8a937a446e7061f24f6b4b037c56c671146f50c8754472601527805a35cd4dc4";

    nc_data::hosted::open_seat(store.connection(), game_id, 1, "invite-123").expect("open");
    nc_data::hosted::claim_seat(store.connection(), game_id, 1, player_pubkey, 3000)
        .expect("claim");

    let seat = nc_data::hosted::get_seat_by_pubkey(store.connection(), game_id, player_pubkey)
        .expect("should get")
        .expect("seat should exist");

    assert_eq!(seat.seat_number, 1);
    assert_eq!(seat.player_pubkey, Some(player_pubkey.to_string()));
}

#[test]
fn test_build_game_state_payload_uses_runtime_snapshot() {
    let (_temp, game_dir, store) = create_test_game("state-sync-runtime", 4);
    let game_id = "state-sync-runtime";
    let player_pubkey = "test-player-runtime";

    nc_data::hosted::claim_seat(store.connection(), game_id, 1, player_pubkey, 3000)
        .expect("claim");
    seed_runtime_snapshot(
        &game_dir,
        game_id,
        "Runtime Sync Test",
        4,
        &[QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3000,
            subject: "Scout Note".to_string(),
            body: "Hostile contact at the rim.".to_string(),
            recipient_deleted: false,
        }],
        &[ReportBlockRow {
            viewer_empire_id: 1,
            block_index: 1,
            decoded_text: "First report block".to_string(),
            raw_bytes: None,
            recipient_deleted: false,
        }],
    );

    let payload = nc_host::game::state::build_game_state_payload(&game_dir, game_id, 1)
        .expect("payload should build");

    assert_eq!(payload.game_id, game_id);
    assert_eq!(payload.year, 3000);
    assert_eq!(payload.player_seat, 1);
    assert!(!payload.state_hash.is_empty());
    assert_eq!(payload.queued_mail.len(), 1);
    assert_eq!(payload.report_blocks.len(), 1);

    assert!(!payload.state.owned_planets.is_empty());
    assert!(!payload.state.owned_fleets.is_empty());
    assert_eq!(payload.state.player.seat, 1);
    assert_eq!(payload.queued_mail[0].subject, "Scout Note");
    assert_eq!(payload.report_blocks[0].decoded_text, "First report block");
}

#[tokio::test]
async fn unclaimed_state_request_enqueues_not_a_player_error() {
    let (_temp, game_dir, store) = create_test_game("state-sync-unclaimed", 4);
    let worker = GameWorker::new(
        "state-sync-unclaimed".to_string(),
        game_dir.join("hosted.db"),
    );

    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: StateRequest {
                request_id: "state-001".to_string(),
                game_id: "state-sync-unclaimed".to_string(),
                player_pubkey: "player-pubkey-001".to_string(),
                last_turn: Some(0),
                last_hash: None,
                handle: Some("pilot".to_string()),
            },
        })
        .await;

    let pending = nc_data::hosted::get_pending(store.connection(), "state-sync-unclaimed", 10)
        .expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30520);
    assert_eq!(pending[0].pubkey, "player-pubkey-001");

    let payload: StateErrorPayload =
        serde_json::from_str(&pending[0].content).expect("state error payload");
    assert_eq!(payload.code, StateErrorCode::NotAPlayer);
    assert_eq!(payload.game_id, "state-sync-unclaimed");
}

#[tokio::test]
async fn claimed_state_request_enqueues_game_state_payload() {
    let (_temp, game_dir, store) = create_test_game("state-sync-worker", 4);
    nc_data::hosted::claim_seat(
        store.connection(),
        "state-sync-worker",
        1,
        "player-pubkey-claimed",
        3000,
    )
    .expect("claim seat");
    seed_runtime_snapshot(
        &game_dir,
        "state-sync-worker",
        "State Sync Worker",
        4,
        &[],
        &[],
    );

    let worker = GameWorker::new("state-sync-worker".to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: StateRequest {
                request_id: "state-002".to_string(),
                game_id: "state-sync-worker".to_string(),
                player_pubkey: "player-pubkey-claimed".to_string(),
                last_turn: Some(0),
                last_hash: None,
                handle: Some("pilot".to_string()),
            },
        })
        .await;

    let pending =
        nc_data::hosted::get_pending(store.connection(), "state-sync-worker", 10).expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30520);

    let payload: nc_nostr::state_sync::GameState =
        serde_json::from_str(&pending[0].content).expect("game state payload");
    assert_eq!(payload.game_id, "state-sync-worker");
    assert_eq!(payload.player_seat, 1);
}

#[tokio::test]
async fn claimed_state_request_returns_delta_when_last_hash_matches_cached_snapshot() {
    let (_temp, game_dir, store) = create_test_game("state-sync-delta", 4);
    nc_data::hosted::claim_seat(
        store.connection(),
        "state-sync-delta",
        1,
        "player-pubkey-delta",
        3000,
    )
    .expect("claim seat");
    seed_runtime_snapshot(
        &game_dir,
        "state-sync-delta",
        "State Sync Delta",
        4,
        &[],
        &[],
    );

    let worker = GameWorker::new("state-sync-delta".to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: StateRequest {
                request_id: "state-delta-001".to_string(),
                game_id: "state-sync-delta".to_string(),
                player_pubkey: "player-pubkey-delta".to_string(),
                last_turn: None,
                last_hash: None,
                handle: Some("pilot".to_string()),
            },
        })
        .await;

    let initial = nc_data::hosted::get_pending(store.connection(), "state-sync-delta", 10)
        .expect("pending initial");
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].kind, 30520);
    let initial_payload: nc_nostr::state_sync::GameState =
        serde_json::from_str(&initial[0].content).expect("initial snapshot");

    seed_runtime_snapshot(
        &game_dir,
        "state-sync-delta",
        "State Sync Delta",
        4,
        &[QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3000,
            subject: "Updated".to_string(),
            body: "Fresh report.".to_string(),
            recipient_deleted: false,
        }],
        &[],
    );

    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: StateRequest {
                request_id: "state-delta-002".to_string(),
                game_id: "state-sync-delta".to_string(),
                player_pubkey: "player-pubkey-delta".to_string(),
                last_turn: Some(initial_payload.turn),
                last_hash: Some(initial_payload.state_hash.clone()),
                handle: Some("pilot".to_string()),
            },
        })
        .await;

    let pending =
        nc_data::hosted::get_pending(store.connection(), "state-sync-delta", 10).expect("pending");
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[1].kind, 30521);

    let tags: Vec<Vec<String>> = serde_json::from_str(&pending[1].tags).expect("tags");
    assert!(
        tags.iter()
            .any(|tag| tag == &["request-id".to_string(), "state-delta-002".to_string()])
    );

    let delta: nc_nostr::state_sync::StateDelta =
        serde_json::from_str(&pending[1].content).expect("delta");
    assert_eq!(delta.base_hash, initial_payload.state_hash);
    assert_eq!(delta.game_id, "state-sync-delta");
    assert!(delta.deltas.queued_mail.is_some());
}

#[tokio::test]
async fn first_hosted_state_request_initializes_claimed_seat_once_without_leaking_full_state() {
    let (_temp, game_dir, store) = create_test_game("state-sync-first-join", 4);
    nc_data::hosted::claim_seat(
        store.connection(),
        "state-sync-first-join",
        1,
        "player-pubkey-first-join",
        3000,
    )
    .expect("claim seat");
    seed_runtime_snapshot(
        &game_dir,
        "state-sync-first-join",
        "State Sync First Join",
        4,
        &[],
        &[],
    );

    let worker = GameWorker::new(
        "state-sync-first-join".to_string(),
        game_dir.join("hosted.db"),
    );
    let request = StateRequest {
        request_id: "state-003".to_string(),
        game_id: "state-sync-first-join".to_string(),
        player_pubkey: "player-pubkey-first-join".to_string(),
        last_turn: Some(0),
        last_hash: None,
        handle: Some("pilot".to_string()),
    };

    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: request.clone(),
        })
        .await;

    let pending = nc_data::hosted::get_pending(store.connection(), "state-sync-first-join", 10)
        .expect("pending");
    let payload: nc_nostr::state_sync::GameState =
        serde_json::from_str(&pending[0].content).expect("game state payload");
    assert_eq!(payload.state.player.mode, "active");
    assert_eq!(payload.state.player.handle.as_deref(), Some("pilot"));
    assert_eq!(payload.state.player.tax_rate, 50);
    assert_eq!(payload.state.player.last_run_year, 3000);
    assert_eq!(payload.state.player.homeworld_planet_index, 1);
    assert_eq!(payload.state.player.planet_count, 1);
    assert_eq!(payload.state.player.starbase_count, 0);
    assert_eq!(payload.state.player.empire_name, "In Civil Disorder");
    let homeworld = payload
        .state
        .owned_planets
        .iter()
        .find(|planet| planet.planet_index == 1)
        .expect("homeworld");
    assert_eq!(homeworld.current_production, 100);
    assert_eq!(homeworld.stored_points, 50);
    assert_eq!(homeworld.armies, 10);
    assert_eq!(homeworld.ground_batteries, 4);
    assert_eq!(payload.state.owned_planets.len(), 1);
    assert_eq!(payload.state.owned_fleets.len(), 4);
    assert!(payload.state.roster.iter().any(|entry| entry.is_self));

    let foreign_world = payload
        .state
        .starmap
        .worlds
        .iter()
        .find(|world| world.planet_index != homeworld.planet_index)
        .expect("foreign world should exist");
    assert_ne!(foreign_world.intel_tier, "owned");
    assert_eq!(foreign_world.known_stored_points, None);
    assert_eq!(foreign_world.known_current_production, None);

    let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("campaign store");
    let state = campaign_store
        .load_latest_runtime_state()
        .expect("runtime state")
        .expect("seeded runtime");
    assert_eq!(state.game_data.players[0].mode, PlayerMode::Human);
    assert_eq!(
        state.game_data.players[0].handle.trim().to_string(),
        "pilot"
    );
    assert_eq!(
        state.game_data.players[0].empire_name.trim().to_string(),
        "In Civil Disorder"
    );
    assert_eq!(state.game_data.players[0].tax_rate, 50);
    assert_eq!(state.game_data.players[0].last_year_on, 3000);
    assert_eq!(stored_production_points(&state.game_data.planets[0]), 50);
    assert!(
        state.game_data.players[1].mode != PlayerMode::Human,
        "other seats should remain unjoined"
    );
    assert_eq!(state.game_data.players[1].handle.trim().to_string(), "");

    worker
        .handle_effect(GameEffects::HandleStateRequest { request })
        .await;

    let state = campaign_store
        .load_latest_runtime_state()
        .expect("runtime state")
        .expect("seeded runtime");
    assert_eq!(stored_production_points(&state.game_data.planets[0]), 50);
    assert_eq!(
        state.game_data.players[0].handle.trim().to_string(),
        "pilot"
    );
}

#[tokio::test]
async fn first_join_setup_renames_empire_and_homeworld_and_returns_updated_state() {
    let (_temp, game_dir, store) = create_test_game("state-sync-name-setup", 4);
    nc_data::hosted::claim_seat(
        store.connection(),
        "state-sync-name-setup",
        1,
        "player-pubkey-name-setup",
        3000,
    )
    .expect("claim seat");
    seed_runtime_snapshot(
        &game_dir,
        "state-sync-name-setup",
        "State Sync Name Setup",
        4,
        &[],
        &[],
    );

    let worker = GameWorker::new(
        "state-sync-name-setup".to_string(),
        game_dir.join("hosted.db"),
    );
    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: StateRequest {
                request_id: "state-004".to_string(),
                game_id: "state-sync-name-setup".to_string(),
                player_pubkey: "player-pubkey-name-setup".to_string(),
                last_turn: Some(0),
                last_hash: None,
                handle: Some("pilot".to_string()),
            },
        })
        .await;

    worker
        .handle_effect(GameEffects::HandleFirstJoinSetup {
            request: FirstJoinSetupRequest {
                request_id: "first-join-001".to_string(),
                game_id: "state-sync-name-setup".to_string(),
                player_pubkey: "player-pubkey-name-setup".to_string(),
                empire_name: "Terran Union".to_string(),
                homeworld_name: "Sol".to_string(),
            },
            game_id: "state-sync-name-setup".to_string(),
        })
        .await;

    let pending = nc_data::hosted::get_pending(store.connection(), "state-sync-name-setup", 10)
        .expect("pending");
    let result: FirstJoinSetupResult =
        serde_json::from_str(&pending.last().expect("setup result").content)
            .expect("first join result");
    assert_eq!(result.status, FirstJoinSetupStatus::Accepted);
    let state = result.state.expect("updated state");
    assert_eq!(state.state.player.empire_name, "Terran Union");
    assert_eq!(state.state.roster[0].empire_name, "Terran Union");
    assert_eq!(state.state.owned_planets[0].name, "Sol");
    let homeworld = state
        .state
        .starmap
        .worlds
        .iter()
        .find(|world| world.planet_index == 1)
        .expect("homeworld world");
    assert_eq!(homeworld.known_name.as_deref(), Some("Sol"));

    let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("campaign store");
    let state = campaign_store
        .load_latest_runtime_state()
        .expect("runtime state")
        .expect("seeded runtime");
    assert_eq!(
        state.game_data.players[0].empire_name.trim().to_string(),
        "Terran Union"
    );
    assert_eq!(state.game_data.planets[0].name, "Sol");
}
