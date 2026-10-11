//! Turn intake: a `30522` submission is validated against the live
//! snapshot and answered with the `30524` receipt the protocol defines.

mod common;

use common::{create_test_game, seed_runtime_snapshot};
use nc_data::hosted::{self, TurnSubmissionStatus};
use nc_data::{PlanetTurnAction, PlanetTurnBlock, TurnSubmission};
use nc_host::game::effects::GameEffects;
use nc_host::game::worker::GameWorker;
use nc_host::support::runtime::{ensure_hosted_player_initialized, load_runtime_game_data};
use nc_nostr::turn_commands::{TurnCommands, TurnReceipt, TurnReceiptStatus};

const PLAYER: &str = "intake-player-one";

fn seed_game(game_id: &str) -> (tempfile::TempDir, std::path::PathBuf, hosted::HostedStore) {
    let (temp, game_dir, store) = create_test_game(game_id, 4);
    seed_runtime_snapshot(&game_dir, game_id, "Intake", 4, &[], &[]);
    hosted::claim_seat(store.connection(), game_id, 1, PLAYER, 3000).expect("claim");
    ensure_hosted_player_initialized(&game_dir, 1, Some("one")).expect("join");
    (temp, game_dir, store)
}

fn scorch_homeworld_orders(game_dir: &std::path::Path, player: usize) -> String {
    let game_data = load_runtime_game_data(game_dir).expect("game data");
    let homeworld = usize::from(game_data.players[player - 1].homeworld);
    TurnSubmission {
        player_record_index_1_based: player,
        year: 3000,
        tax_rate: None,
        diplomacy: Vec::new(),
        planets: vec![PlanetTurnBlock {
            planet_record_index_1_based: homeworld,
            actions: vec![PlanetTurnAction::Scorch],
        }],
        fleets: Vec::new(),
        messages: Vec::new(),
    }
    .to_kdl_string()
}

fn commands(game_id: &str, submit_id: &str, turn: u32, orders: &str) -> GameEffects {
    GameEffects::HandleTurnCommands {
        commands: TurnCommands {
            submit_id: submit_id.to_string(),
            game_id: game_id.to_string(),
            turn,
            player_pubkey: PLAYER.to_string(),
            commands: orders.to_string(),
            handle: None,
        },
        game_id: game_id.to_string(),
    }
}

fn receipts(store: &hosted::HostedStore, game_id: &str) -> Vec<TurnReceipt> {
    hosted::get_pending(store.connection(), game_id, 50)
        .expect("pending outbox")
        .into_iter()
        .filter(|item| item.kind == 30524 && item.pubkey == PLAYER)
        .map(|item| serde_json::from_str(&item.content).expect("receipt payload"))
        .collect()
}

fn queue_status(store: &hosted::HostedStore, submit_id: &str) -> hosted::TurnSubmission {
    hosted::get_turn_submission(store.connection(), submit_id)
        .expect("queue lookup")
        .expect("queue row")
}

#[tokio::test]
async fn valid_orders_stay_pending_for_maintenance_and_get_an_accepted_receipt() {
    let game_id = "intake-accept";
    let (_temp, game_dir, store) = seed_game(game_id);
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));

    worker
        .handle_effect(commands(
            game_id,
            "submit-ok",
            0,
            &scorch_homeworld_orders(&game_dir, 1),
        ))
        .await;

    let queued = queue_status(&store, "submit-ok");
    assert_eq!(queued.status, TurnSubmissionStatus::Pending);
    let receipts = receipts(&store, game_id);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].submit_id, "submit-ok");
    assert_eq!(receipts[0].status, TurnReceiptStatus::Accepted);
    assert!(receipts[0].errors.is_empty());

    // Validation is a dry run: the snapshot keeps the orders for maintenance.
    let game_data = load_runtime_game_data(&game_dir).expect("game data");
    let homeworld = usize::from(game_data.players[0].homeworld);
    assert!(!game_data.planets[homeworld - 1].scorch_earth);
}

#[tokio::test]
async fn orders_that_fail_typed_validation_are_rejected_with_the_reason() {
    let game_id = "intake-reject";
    let (_temp, game_dir, store) = seed_game(game_id);
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));

    // Player 2's homeworld is not player 1's to scorch.
    let orders = scorch_homeworld_orders(&game_dir, 2).replace("player=2", "player=1");
    worker
        .handle_effect(commands(game_id, "submit-wrong-planet", 0, &orders))
        .await;
    worker
        .handle_effect(commands(game_id, "submit-garbage", 0, "fleet { order"))
        .await;

    let wrong_planet = queue_status(&store, "submit-wrong-planet");
    assert_eq!(wrong_planet.status, TurnSubmissionStatus::Rejected);
    assert!(
        wrong_planet
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("not owned by player 1"))
    );
    assert_eq!(
        queue_status(&store, "submit-garbage").status,
        TurnSubmissionStatus::Rejected
    );

    let receipts = receipts(&store, game_id);
    assert_eq!(receipts.len(), 2);
    assert_eq!(receipts[0].submit_id, "submit-wrong-planet");
    assert_eq!(receipts[0].status, TurnReceiptStatus::Rejected);
    assert_eq!(receipts[0].errors[0].path, "orders");
    assert!(
        receipts[0].errors[0]
            .message
            .contains("not owned by player 1")
    );
    assert_eq!(receipts[1].submit_id, "submit-garbage");
    assert_eq!(receipts[1].status, TurnReceiptStatus::Rejected);
    assert_eq!(receipts[1].errors[0].path, "commands");
}

#[tokio::test]
async fn orders_for_another_turn_get_a_wrong_turn_receipt() {
    let game_id = "intake-wrong-turn";
    let (_temp, game_dir, store) = seed_game(game_id);
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));

    worker
        .handle_effect(commands(
            game_id,
            "submit-late",
            3,
            &scorch_homeworld_orders(&game_dir, 1),
        ))
        .await;

    assert_eq!(
        queue_status(&store, "submit-late").status,
        TurnSubmissionStatus::Rejected
    );
    let receipts = receipts(&store, game_id);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].status, TurnReceiptStatus::WrongTurn);
    assert_eq!(receipts[0].turn, 3);
    assert!(
        receipts[0]
            .message
            .as_deref()
            .is_some_and(|m| m.contains("turn 0"))
    );
}

#[tokio::test]
async fn a_newer_submission_supersedes_the_one_staged_before_it() {
    let game_id = "intake-supersede";
    let (_temp, game_dir, store) = seed_game(game_id);
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));
    let orders = scorch_homeworld_orders(&game_dir, 1);

    worker
        .handle_effect(commands(game_id, "submit-first", 0, &orders))
        .await;
    worker
        .handle_effect(commands(game_id, "submit-second", 0, &orders))
        .await;

    assert_eq!(
        queue_status(&store, "submit-first").status,
        TurnSubmissionStatus::Superseded
    );
    assert_eq!(
        queue_status(&store, "submit-second").status,
        TurnSubmissionStatus::Pending
    );
    let statuses = receipts(&store, game_id)
        .into_iter()
        .map(|receipt| (receipt.submit_id, receipt.status))
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        vec![
            ("submit-first".to_string(), TurnReceiptStatus::Accepted),
            ("submit-first".to_string(), TurnReceiptStatus::Superseded),
            ("submit-second".to_string(), TurnReceiptStatus::Accepted),
        ]
    );
    let pending = hosted::list_pending_turns(store.connection(), game_id, 0).expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, "submit-second");
}

#[tokio::test]
async fn an_unclaimed_player_is_told_so_and_nothing_is_staged() {
    let game_id = "intake-unclaimed";
    let (_temp, game_dir, store) = create_test_game(game_id, 4);
    seed_runtime_snapshot(&game_dir, game_id, "Intake", 4, &[], &[]);
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));

    worker
        .handle_effect(commands(game_id, "submit-nobody", 0, "player=1"))
        .await;

    assert!(
        hosted::get_turn_submission(store.connection(), "submit-nobody")
            .expect("queue lookup")
            .is_none()
    );
    let receipts = receipts(&store, game_id);
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].status, TurnReceiptStatus::NotClaimed);
}

#[tokio::test]
async fn process_turn_submission_settles_a_staged_row() {
    let game_id = "intake-process";
    let (_temp, game_dir, store) = seed_game(game_id);
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));
    hosted::enqueue_turn(
        store.connection(),
        "submit-staged",
        game_id,
        0,
        PLAYER,
        &scorch_homeworld_orders(&game_dir, 1),
    )
    .expect("enqueue");

    worker.process_turn_submission("submit-staged");

    let sent = receipts(&store, game_id);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].submit_id, "submit-staged");
    assert_eq!(sent[0].status, TurnReceiptStatus::Accepted);

    // Processing a settled row again sends nothing.
    hosted::accept_turn(store.connection(), "submit-staged").expect("accept");
    worker.process_turn_submission("submit-staged");
    assert_eq!(receipts(&store, game_id).len(), 1);
}
