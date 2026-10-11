//! Hosted turn submissions: the typed KDL orders a player staged for the
//! current turn. Intake validates a submission against the live snapshot
//! and answers with a `30524` receipt; maintenance applies the pending ones
//! through the same `TurnSubmission` path `nc-game submit-turn` uses, so a
//! hosted game takes orders exactly as a local one does.

use std::path::Path;

use nc_data::TurnSubmission;
use nc_data::hosted::{self, HostedStore, SeatStatus, TurnSubmission as QueuedTurn};
use nc_engine::CampaignMaintenance;
use nc_nostr::turn_commands::{TurnReceipt, TurnReceiptError, TurnReceiptStatus};
use rusqlite::Connection;

use crate::game::outbox::enqueue_encrypted_event;
use crate::support::runtime::{load_latest_runtime_state, open_existing_campaign_store};

/// The hosted turn number for a game year: turn 0 is year 3000.
pub fn turn_for_year(game_year: u16) -> u32 {
    u32::from(game_year.saturating_sub(3000))
}

/// Why a submission was not taken, as the player's receipt will say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRejection {
    pub status: TurnReceiptStatus,
    pub path: &'static str,
    pub message: String,
}

impl TurnRejection {
    fn rejected(path: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: TurnReceiptStatus::Rejected,
            path,
            message: message.into(),
        }
    }

    pub fn into_receipt(self, queued: &QueuedTurn) -> TurnReceipt {
        turn_receipt(
            queued,
            self.status,
            self.message.clone(),
            vec![TurnReceiptError {
                path: self.path.to_string(),
                message: self.message,
            }],
        )
    }
}

pub fn turn_receipt(
    queued: &QueuedTurn,
    status: TurnReceiptStatus,
    message: impl Into<String>,
    errors: Vec<TurnReceiptError>,
) -> TurnReceipt {
    TurnReceipt {
        submit_id: queued.id.clone(),
        game_id: queued.game_id.clone(),
        turn: queued.turn,
        status,
        message: Some(message.into()),
        errors,
    }
}

/// Queue a receipt for the player. The public tags carry only the ids and
/// status; the message and errors travel in the encrypted payload.
pub fn enqueue_turn_receipt(
    conn: &Connection,
    game_id: &str,
    player_pubkey: &str,
    receipt: &TurnReceipt,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string(receipt)?;
    let tags = vec![
        vec!["d".to_string(), receipt.submit_id.clone()],
        vec!["game-id".to_string(), receipt.game_id.clone()],
        vec!["turn".to_string(), receipt.turn.to_string()],
        vec!["status".to_string(), receipt.status.as_str().to_string()],
    ];
    enqueue_encrypted_event(conn, game_id, player_pubkey, 30524, &content, tags)?;
    Ok(())
}

/// What a pending submission did when maintenance applied it.
pub struct AppliedSubmissions {
    pub applied: usize,
    pub rejections: Vec<(QueuedTurn, TurnRejection)>,
}

/// Validate a queued submission against the live snapshot without saving:
/// the seat, the turn, the KDL, and a dry run of the orders and mail.
pub fn validate_turn_submission(
    store: &HostedStore,
    game_dir: &Path,
    game_id: &str,
    queued: &QueuedTurn,
) -> Result<(), TurnRejection> {
    let campaign_store = open_existing_campaign_store(game_dir).map_err(unavailable)?;
    let mut state = load_latest_runtime_state(&campaign_store).map_err(unavailable)?;
    let player_count = state.game_data.conquest.player_count;
    let player_lifecycle_states = campaign_store
        .latest_player_lifecycle_states(player_count)
        .map_err(|err| unavailable(err.into()))?;
    let mut player_activity_states = campaign_store
        .latest_player_activity_states(player_count)
        .map_err(|err| unavailable(err.into()))?;
    let submission = resolve_submission(
        store.connection(),
        game_id,
        queued,
        state.game_data.conquest.game_year,
    )?;
    submission
        .apply_to_runtime_state(
            &mut state.game_data,
            &mut state.queued_mail,
            &player_lifecycle_states,
            state.winner_state,
            &mut player_activity_states,
        )
        .map(|_| ())
        .map_err(|err| TurnRejection::rejected("orders", err.to_string()))
}

/// Apply every pending submission for the current turn in the order it
/// arrived, each all-or-nothing, and settle its queue row. The rejections
/// come back so the caller can send their receipts.
pub fn apply_pending_turn_submissions(
    store: &HostedStore,
    game_id: &str,
    maintenance: &mut CampaignMaintenance,
) -> Result<AppliedSubmissions, Box<dyn std::error::Error>> {
    let conn = store.connection();
    let game_year = maintenance.game_data.conquest.game_year;
    let pending = hosted::list_pending_turns(conn, game_id, turn_for_year(game_year))?;
    let mut outcome = AppliedSubmissions {
        applied: 0,
        rejections: Vec::new(),
    };
    for queued in pending {
        let mut game_data = maintenance.game_data.clone();
        let mut queued_mail = maintenance.queued_mail.clone();
        let mut player_activity_states = maintenance.player_activity_states.clone();
        let applied =
            resolve_submission(conn, game_id, &queued, game_year).and_then(|submission| {
                submission
                    .apply_to_runtime_state(
                        &mut game_data,
                        &mut queued_mail,
                        &maintenance.player_lifecycle_states,
                        maintenance.winner_state,
                        &mut player_activity_states,
                    )
                    .map(|_| ())
                    .map_err(|err| TurnRejection::rejected("orders", err.to_string()))
            });
        match applied {
            Ok(()) => {
                maintenance.game_data = game_data;
                maintenance.queued_mail = queued_mail;
                maintenance.player_activity_states = player_activity_states;
                hosted::accept_turn(conn, &queued.id)?;
                outcome.applied += 1;
            }
            Err(rejection) => {
                hosted::reject_turn(conn, &queued.id, &rejection.message)?;
                outcome.rejections.push((queued, rejection));
            }
        }
    }
    Ok(outcome)
}

/// The typed submission in `queued`, if its sender holds the seat it names
/// and it is for the game's current turn.
fn resolve_submission(
    conn: &Connection,
    game_id: &str,
    queued: &QueuedTurn,
    game_year: u16,
) -> Result<TurnSubmission, TurnRejection> {
    let seat = match hosted::get_seat_by_pubkey(conn, game_id, &queued.player_pubkey) {
        Ok(Some(seat)) if seat.status == SeatStatus::Claimed => seat,
        Ok(_) => {
            return Err(TurnRejection {
                status: TurnReceiptStatus::NotClaimed,
                path: "player",
                message: "Player has not claimed a seat in this hosted game.".to_string(),
            });
        }
        Err(err) => {
            return Err(TurnRejection::rejected(
                "player",
                format!("Failed to look up your hosted seat: {err}"),
            ));
        }
    };
    let turn = turn_for_year(game_year);
    if queued.turn != turn {
        return Err(TurnRejection {
            status: TurnReceiptStatus::WrongTurn,
            path: "turn",
            message: format!(
                "Orders are for turn {}; the game is on turn {}.",
                queued.turn, turn
            ),
        });
    }
    let submission = TurnSubmission::parse_kdl_str(&queued.commands)
        .map_err(|err| TurnRejection::rejected("commands", err.to_string()))?;
    if submission.player_record_index_1_based != seat.seat_number as usize {
        return Err(TurnRejection::rejected(
            "player",
            format!(
                "Orders are for player {}, but this seat is {}.",
                submission.player_record_index_1_based, seat.seat_number
            ),
        ));
    }
    Ok(submission)
}

fn unavailable(err: Box<dyn std::error::Error>) -> TurnRejection {
    TurnRejection::rejected("game", format!("Hosted game state is unavailable: {err}"))
}
