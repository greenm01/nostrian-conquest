//! Hosted maintenance applies the staged turn submissions and runs the same
//! pipeline as `nc-sysop maint`, so a hosted game and a local game given the
//! same orders end the year with the same snapshot, reports and mail.

mod common;

use common::{create_test_game, seed_runtime_snapshot};
use nc_data::hosted::{self, TurnSubmissionStatus};
use nc_data::{
    CampaignRuntimeState, CampaignStore, DEFAULT_CAMPAIGN_DB_NAME, PlanetTurnAction,
    PlanetTurnBlock, TurnMessage, TurnSubmission,
};
use nc_engine::CampaignMaintenance;
use nc_host::game::state::build_game_state_payload;
use nc_host::support::runtime::ensure_hosted_player_initialized;
use nc_nostr::state_sync::{GameState, StateDelta};
use nc_nostr::turn_commands::{TurnReceipt, TurnReceiptStatus};
use std::path::Path;

const SEAT_ONE: &str = "hosted-maint-player-one";
const SEAT_TWO: &str = "hosted-maint-player-two";

/// A hosted game at year 3000 with seats 1 and 2 claimed and joined.
fn seed_two_player_game(
    game_id: &str,
) -> (tempfile::TempDir, std::path::PathBuf, hosted::HostedStore) {
    let (temp, game_dir, store) = create_test_game(game_id, 4);
    seed_runtime_snapshot(&game_dir, game_id, "Hosted Maint", 4, &[], &[]);
    for (seat, pubkey, handle) in [(1, SEAT_ONE, "one"), (2, SEAT_TWO, "two")] {
        hosted::claim_seat(store.connection(), game_id, seat, pubkey, 3000).expect("claim");
        ensure_hosted_player_initialized(&game_dir, seat, Some(handle)).expect("join");
    }
    (temp, game_dir, store)
}

fn load_runtime(dir: &Path) -> CampaignRuntimeState {
    CampaignStore::open_default_in_dir(dir)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist")
}

/// Player 1's orders for year 3000: a tax change, Scorch-Earth on the
/// homeworld, and a message to player 2.
fn player_one_orders(runtime: &CampaignRuntimeState) -> TurnSubmission {
    let homeworld = usize::from(runtime.game_data.players[0].homeworld);
    TurnSubmission {
        player_record_index_1_based: 1,
        year: 3000,
        tax_rate: Some(45),
        diplomacy: Vec::new(),
        planets: vec![PlanetTurnBlock {
            planet_record_index_1_based: homeworld,
            actions: vec![PlanetTurnAction::Scorch],
        }],
        fleets: Vec::new(),
        messages: vec![TurnMessage {
            recipient_empire: 2,
            subject: "Border".to_string(),
            body: "Stay clear of the rim this year.".to_string(),
        }],
    }
}

fn pending_outbox(store: &hosted::HostedStore, game_id: &str) -> Vec<hosted::OutboxItem> {
    hosted::get_pending(store.connection(), game_id, 50).expect("pending outbox")
}

#[test]
fn hosted_maintenance_matches_the_local_path_for_the_same_orders() {
    let game_id = "hosted-maint-parity";
    let (_temp, hosted_dir, store) = seed_two_player_game(game_id);
    let orders = player_one_orders(&load_runtime(&hosted_dir));

    // The local path: the same campaign store, orders applied the way
    // `nc-game submit-turn` does, then the pipeline `nc-sysop maint` runs.
    let local_dir = hosted_dir.parent().expect("games root").join("local-copy");
    std::fs::create_dir_all(&local_dir).expect("local dir");
    std::fs::copy(
        hosted_dir.join(DEFAULT_CAMPAIGN_DB_NAME),
        local_dir.join(DEFAULT_CAMPAIGN_DB_NAME),
    )
    .expect("copy campaign store");
    let local_store = CampaignStore::open_default_in_dir(&local_dir).expect("open local store");
    let state = local_store
        .load_latest_runtime_state()
        .expect("load")
        .expect("snapshot");
    orders
        .submit_to_runtime_state(&local_store, state)
        .expect("local submit");
    let mut maintenance = CampaignMaintenance::load(&local_store).expect("load maintenance");
    maintenance.run_turn().expect("local turn");
    maintenance.save(&local_store).expect("local save");

    // The hosted path: the orders staged as a turn submission, then maint.
    hosted::enqueue_turn(
        store.connection(),
        "submit-one",
        game_id,
        0,
        SEAT_ONE,
        &orders.to_kdl_string(),
    )
    .expect("enqueue");
    nc_host::commands::maint::run(&[hosted_dir.to_str().expect("dir"), "1"]).expect("maint");

    let hosted = load_runtime(&hosted_dir);
    let local = load_runtime(&local_dir);
    assert_eq!(hosted.game_year, 3001);
    assert_eq!(hosted.game_data, local.game_data);
    assert_eq!(hosted.report_block_rows, local.report_block_rows);
    assert_eq!(hosted.queued_mail, local.queued_mail);
    assert_eq!(hosted.winner_state, local.winner_state);

    // The orders took: the tax rate, the scorch order, and the delivered mail.
    assert_eq!(hosted.game_data.players[0].tax_rate, 45);
    let homeworld = usize::from(hosted.game_data.players[0].homeworld);
    assert!(hosted.game_data.planets[homeworld - 1].scorch_earth);
    let queued = hosted::get_turn_submission(store.connection(), "submit-one")
        .expect("queue row")
        .expect("row exists");
    assert_eq!(queued.status, TurnSubmissionStatus::Accepted);
    let seat_two = build_game_state_payload(&hosted_dir, game_id, 2).expect("seat 2 state");
    assert_eq!(seat_two.queued_mail.len(), 1);
    assert_eq!(seat_two.queued_mail[0].subject, "Border");
    assert_eq!(seat_two.queued_mail[0].sender_empire_id, 1);
    let seat_one = build_game_state_payload(&hosted_dir, game_id, 1).expect("seat 1 state");
    assert!(
        !seat_one.report_blocks.is_empty(),
        "maintenance should leave player 1 a report"
    );
}

#[test]
fn hosted_maintenance_rejects_a_stale_submission_and_receipts_it() {
    let game_id = "hosted-maint-reject";
    let (_temp, game_dir, store) = seed_two_player_game(game_id);
    let runtime = load_runtime(&game_dir);
    let not_mine = usize::from(runtime.game_data.players[1].homeworld);
    let orders = TurnSubmission {
        player_record_index_1_based: 1,
        year: 3000,
        tax_rate: None,
        diplomacy: Vec::new(),
        planets: vec![PlanetTurnBlock {
            planet_record_index_1_based: not_mine,
            actions: vec![PlanetTurnAction::Scorch],
        }],
        fleets: Vec::new(),
        messages: Vec::new(),
    };
    hosted::enqueue_turn(
        store.connection(),
        "submit-bad",
        game_id,
        0,
        SEAT_ONE,
        &orders.to_kdl_string(),
    )
    .expect("enqueue");

    let report =
        nc_host::game::maint::run_hosted_maintenance(&game_dir, game_id, 1).expect("maint");
    assert_eq!(report.applied_submissions, 0);
    assert_eq!(report.rejected_submissions, 1);

    let queued = hosted::get_turn_submission(store.connection(), "submit-bad")
        .expect("queue row")
        .expect("row exists");
    assert_eq!(queued.status, TurnSubmissionStatus::Rejected);
    assert!(
        queued
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("not owned by player 1"))
    );
    let receipts = pending_outbox(&store, game_id)
        .into_iter()
        .filter(|item| item.kind == 30524)
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].pubkey, SEAT_ONE);
    let receipt: TurnReceipt = serde_json::from_str(&receipts[0].content).expect("receipt");
    assert_eq!(receipt.submit_id, "submit-bad");
    assert_eq!(receipt.status, TurnReceiptStatus::Rejected);
    assert_eq!(receipt.errors[0].path, "orders");

    // The year still ran, and a seat without a player got nothing.
    assert_eq!(load_runtime(&game_dir).game_year, 3001);
    assert!(
        hosted::get_latest_state_snapshot(store.connection(), game_id, 3)
            .expect("snapshot lookup")
            .is_none()
    );
}

#[test]
fn hosted_maintenance_sends_each_claimed_seat_its_state() {
    let game_id = "hosted-maint-sync";
    let (_temp, game_dir, store) = seed_two_player_game(game_id);

    let first = nc_host::game::maint::run_hosted_maintenance(&game_dir, game_id, 1).expect("maint");
    assert_eq!(first.published_seats, 2);
    let snapshots = pending_outbox(&store, game_id)
        .into_iter()
        .filter(|item| item.kind == 30520)
        .collect::<Vec<_>>();
    assert_eq!(snapshots.len(), 2);
    let mut recipients = snapshots
        .iter()
        .map(|item| item.pubkey.as_str())
        .collect::<Vec<_>>();
    recipients.sort();
    assert_eq!(recipients, vec![SEAT_ONE, SEAT_TWO]);
    let seat_one_state: GameState = serde_json::from_str(
        &snapshots
            .iter()
            .find(|item| item.pubkey == SEAT_ONE)
            .expect("seat 1 snapshot")
            .content,
    )
    .expect("state payload");
    assert_eq!(seat_one_state.year, 3001);
    assert_eq!(seat_one_state.turn, 1);
    let remembered = hosted::get_latest_state_snapshot(store.connection(), game_id, 1)
        .expect("snapshot lookup")
        .expect("seat 1 snapshot saved");
    assert_eq!(remembered.state_hash, seat_one_state.state_hash);
    let metadata = hosted::get_game_metadata(store.connection(), game_id).expect("metadata");
    assert_eq!(metadata.current_year, 3001);
    assert_eq!(metadata.current_turn, 1);

    // The next year goes out as a delta against the snapshot each seat has.
    for item in pending_outbox(&store, game_id) {
        hosted::mark_published(store.connection(), &item.id, "test").expect("mark published");
    }
    nc_host::game::maint::run_hosted_maintenance(&game_dir, game_id, 1).expect("second maint");
    let deltas = pending_outbox(&store, game_id)
        .into_iter()
        .filter(|item| item.kind == 30521)
        .collect::<Vec<_>>();
    assert_eq!(deltas.len(), 2);
    let delta: StateDelta = serde_json::from_str(
        &deltas
            .iter()
            .find(|item| item.pubkey == SEAT_ONE)
            .expect("seat 1 delta")
            .content,
    )
    .expect("delta payload");
    assert_eq!(delta.base_hash, seat_one_state.state_hash);
    assert_eq!(delta.year, 3002);
    assert_eq!(delta.turn, 2);
}
