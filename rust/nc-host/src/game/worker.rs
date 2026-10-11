use crate::game::effects::GameEffects;
use crate::game::msg::GameMsg;
use crate::game::outbox::enqueue_encrypted_event;
use crate::game::sync::enqueue_state_sync;
use crate::game::turns::{enqueue_turn_receipt, turn_receipt, validate_turn_submission};
use crate::support::pubkeys::short_pubkey;
use crate::support::runtime::{
    apply_hosted_first_join_names, current_runtime_year, ensure_hosted_player_initialized,
    reset_hosted_player_to_baseline,
};
use nc_data::hosted::{self, GameTier, HandleOwnership, HostedStore};
use nc_nostr::claim::{SeatClaimRequest, SeatClaimResultPayload, SeatClaimStatus};
use nc_nostr::first_join::{
    FIRST_JOIN_NAME_MAX_CHARS, FirstJoinSetupRequest, FirstJoinSetupResult, FirstJoinSetupStatus,
};
use nc_nostr::invite_request::{
    InviteDecision, InviteRequest, InviteRequestReceipt, InviteRequestReceiptStatus,
};
use nc_nostr::sandbox_release::{
    SandboxReleaseRequest, SandboxReleaseResult, SandboxReleaseStatus,
};
use nc_nostr::state_sync::{StateErrorCode, StateErrorPayload, StateRequest};
use nc_nostr::turn_commands::{TurnCommands, TurnReceipt, TurnReceiptError, TurnReceiptStatus};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct GameWorker {
    game_id: String,
    db_path: PathBuf,
}

impl GameWorker {
    pub fn new(game_id: String, db_path: PathBuf) -> Self {
        Self { game_id, db_path }
    }

    pub async fn handle_effect(&self, effect: GameEffects) {
        match effect {
            GameEffects::HandleStateRequest { request } => {
                self.handle_state_request(request).await;
            }
            GameEffects::HandleInviteRequest { request, .. } => {
                self.handle_invite_request(request).await;
            }
            GameEffects::HandleFirstJoinSetup { request, .. } => {
                self.handle_first_join_setup(request).await;
            }
            GameEffects::HandleSeatClaim { request, .. } => {
                self.handle_seat_claim(request).await;
            }
            GameEffects::HandleSandboxRelease { request, .. } => {
                self.handle_sandbox_release(request).await;
            }
            GameEffects::HandleTurnCommands { commands, .. } => {
                self.handle_turn_commands(commands).await;
            }
            GameEffects::HandleThreadMessage { message, .. } => {
                self.handle_thread_message(message).await;
            }
            GameEffects::HandlePlayerMessage { message, .. } => {
                self.handle_player_message(message).await;
            }
            GameEffects::QueueEvent { .. } => {}
            GameEffects::UpdateLobbyCatalog { .. } => {}
            GameEffects::NotifySysop { .. } => {}
            GameEffects::RunMaintenance { .. } => {
                self.run_scheduled_maintenance();
            }
            GameEffects::InvalidEvent { reason } => {
                tracing::warn!("Invalid event for game {}: {}", self.game_id, reason);
            }
        }
    }

    async fn handle_state_request(&self, request: StateRequest) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let seat = match hosted::get_seat_by_pubkey(
            store.connection(),
            &self.game_id,
            &request.player_pubkey,
        ) {
            Ok(Some(seat)) => seat,
            Ok(None) => {
                tracing::warn!(
                    "Rejecting state request for unclaimed player {} in {}",
                    short_pubkey(&request.player_pubkey),
                    self.game_id
                );
                let _ = self.enqueue_state_error(
                    store.connection(),
                    &request.player_pubkey,
                    Some(&request.request_id),
                    StateErrorCode::NotAPlayer,
                    "You no longer have a claimed seat in this game.",
                );
                return;
            }
            Err(e) => {
                tracing::error!("Failed to lookup player seat: {}", e);
                let _ = self.enqueue_state_error(
                    store.connection(),
                    &request.player_pubkey,
                    Some(&request.request_id),
                    StateErrorCode::StateUnavailable,
                    "Failed to lookup your hosted seat.",
                );
                return;
            }
        };

        if let Err(message) =
            self.validate_player_handle(&request.player_pubkey, request.handle.as_deref())
        {
            let _ = self.enqueue_state_error(
                store.connection(),
                &request.player_pubkey,
                Some(&request.request_id),
                StateErrorCode::HandleTaken,
                &message,
            );
            return;
        }
        self.refresh_player_seen(&request.player_pubkey, request.handle.as_deref());

        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            let _ = self.enqueue_state_error(
                store.connection(),
                &request.player_pubkey,
                Some(&request.request_id),
                StateErrorCode::StateUnavailable,
                "Hosted state is unavailable right now.",
            );
            return;
        };

        if let Err(e) =
            ensure_hosted_player_initialized(game_dir, seat.seat_number, request.handle.as_deref())
        {
            tracing::error!(
                "Failed to initialize hosted player {} in {}: {}",
                seat.seat_number,
                self.game_id,
                e
            );
            let _ = self.enqueue_state_error(
                store.connection(),
                &request.player_pubkey,
                Some(&request.request_id),
                StateErrorCode::StateUnavailable,
                "Failed to initialize hosted player state.",
            );
            return;
        }

        let prior_snapshot = request.last_hash.as_deref().and_then(|state_hash| {
            hosted::get_state_snapshot_by_hash(
                store.connection(),
                &self.game_id,
                seat.seat_number,
                state_hash,
            )
            .ok()
            .flatten()
        });

        match enqueue_state_sync(
            &store,
            game_dir,
            &self.game_id,
            seat.seat_number,
            &request.player_pubkey,
            prior_snapshot.as_ref(),
            Some(&request.request_id),
        ) {
            Ok(_) => tracing::info!(
                "Queued encrypted hosted state sync for {}",
                short_pubkey(&request.player_pubkey)
            ),
            Err(e) => {
                tracing::error!(
                    "Failed to queue hosted state sync for {}: {}",
                    self.game_id,
                    e
                );
                let _ = self.enqueue_state_error(
                    store.connection(),
                    &request.player_pubkey,
                    Some(&request.request_id),
                    StateErrorCode::StateUnavailable,
                    "Failed to build hosted game state.",
                );
            }
        }
    }

    fn enqueue_state_error(
        &self,
        conn: &rusqlite::Connection,
        recipient_pubkey: &str,
        request_id: Option<&str>,
        code: StateErrorCode,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = StateErrorPayload {
            game_id: self.game_id.clone(),
            code,
            message: message.to_string(),
        };
        let content = serde_json::to_string(&payload)?;
        let mut tags = nc_nostr::state_sync::build_state_error_tags(&payload)
            .into_iter()
            .map(|(key, value)| vec![key.to_string(), value])
            .collect::<Vec<_>>();
        if let Some(request_id) = request_id {
            tags.push(vec!["request-id".to_string(), request_id.to_string()]);
        }
        enqueue_encrypted_event(conn, &self.game_id, recipient_pubkey, 30520, &content, tags)?;
        Ok(())
    }

    async fn handle_first_join_setup(&self, request: FirstJoinSetupRequest) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let seat = match hosted::get_seat_by_pubkey(
            store.connection(),
            &self.game_id,
            &request.player_pubkey,
        ) {
            Ok(Some(seat)) => seat,
            Ok(None) => {
                let _ = self.enqueue_first_join_setup_result(
                    store.connection(),
                    &request.player_pubkey,
                    FirstJoinSetupResult {
                        request_id: request.request_id,
                        game_id: self.game_id.clone(),
                        status: FirstJoinSetupStatus::Rejected,
                        message: "You no longer have a claimed seat in this game.".to_string(),
                        state: None,
                    },
                );
                return;
            }
            Err(e) => {
                tracing::error!("Failed to lookup player seat: {}", e);
                return;
            }
        };

        let empire_name = request.empire_name.trim();
        let homeworld_name = request.homeworld_name.trim();
        if empire_name.is_empty() {
            let _ = self.enqueue_first_join_setup_result(
                store.connection(),
                &request.player_pubkey,
                FirstJoinSetupResult {
                    request_id: request.request_id,
                    game_id: self.game_id.clone(),
                    status: FirstJoinSetupStatus::Rejected,
                    message: "Empire names need at least one visible character.".to_string(),
                    state: None,
                },
            );
            return;
        }
        if homeworld_name.is_empty() {
            let _ = self.enqueue_first_join_setup_result(
                store.connection(),
                &request.player_pubkey,
                FirstJoinSetupResult {
                    request_id: request.request_id,
                    game_id: self.game_id.clone(),
                    status: FirstJoinSetupStatus::Rejected,
                    message: "Homeworld names need at least one visible character.".to_string(),
                    state: None,
                },
            );
            return;
        }
        if empire_name.chars().count() > FIRST_JOIN_NAME_MAX_CHARS
            || homeworld_name.chars().count() > FIRST_JOIN_NAME_MAX_CHARS
        {
            let _ = self.enqueue_first_join_setup_result(
                store.connection(),
                &request.player_pubkey,
                FirstJoinSetupResult {
                    request_id: request.request_id,
                    game_id: self.game_id.clone(),
                    status: FirstJoinSetupStatus::Rejected,
                    message: format!(
                        "Empire and homeworld names must be {} characters or less.",
                        FIRST_JOIN_NAME_MAX_CHARS
                    ),
                    state: None,
                },
            );
            return;
        }

        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            return;
        };

        if let Err(e) = ensure_hosted_player_initialized(game_dir, seat.seat_number, None) {
            tracing::error!(
                "Failed to initialize hosted player {} in {} before naming: {}",
                seat.seat_number,
                self.game_id,
                e
            );
            let _ = self.enqueue_first_join_setup_result(
                store.connection(),
                &request.player_pubkey,
                FirstJoinSetupResult {
                    request_id: request.request_id,
                    game_id: self.game_id.clone(),
                    status: FirstJoinSetupStatus::Rejected,
                    message: "Unable to prepare your empire for first-time naming.".to_string(),
                    state: None,
                },
            );
            return;
        }

        if let Err(e) =
            apply_hosted_first_join_names(game_dir, seat.seat_number, empire_name, homeworld_name)
        {
            let _ = self.enqueue_first_join_setup_result(
                store.connection(),
                &request.player_pubkey,
                FirstJoinSetupResult {
                    request_id: request.request_id,
                    game_id: self.game_id.clone(),
                    status: FirstJoinSetupStatus::Rejected,
                    message: e.to_string(),
                    state: None,
                },
            );
            return;
        }

        let state = match crate::game::state::build_game_state_payload(
            game_dir,
            &self.game_id,
            seat.seat_number,
        ) {
            Ok(state) => state,
            Err(e) => {
                tracing::error!(
                    "Failed to build first-join state payload for {}: {}",
                    self.game_id,
                    e
                );
                let _ = self.enqueue_first_join_setup_result(
                    store.connection(),
                    &request.player_pubkey,
                    FirstJoinSetupResult {
                        request_id: request.request_id,
                        game_id: self.game_id.clone(),
                        status: FirstJoinSetupStatus::Rejected,
                        message: "Unable to build the updated hosted state.".to_string(),
                        state: None,
                    },
                );
                return;
            }
        };

        if let Err(e) =
            hosted::save_state_snapshot(store.connection(), &self.game_id, seat.seat_number, &state)
        {
            tracing::error!("Failed to persist first-join hosted state snapshot: {}", e);
        }

        let _ = self.enqueue_first_join_setup_result(
            store.connection(),
            &request.player_pubkey,
            FirstJoinSetupResult {
                request_id: request.request_id,
                game_id: self.game_id.clone(),
                status: FirstJoinSetupStatus::Accepted,
                message: "First-join names saved.".to_string(),
                state: Some(state),
            },
        );
    }

    fn enqueue_first_join_setup_result(
        &self,
        conn: &rusqlite::Connection,
        recipient_pubkey: &str,
        result: FirstJoinSetupResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string(&result)?;
        let tags = nc_nostr::first_join::build_first_join_setup_result_tags(&result)
            .into_iter()
            .map(|(key, value)| vec![key.to_string(), value])
            .collect();
        enqueue_encrypted_event(conn, &self.game_id, recipient_pubkey, 30528, &content, tags)?;
        Ok(())
    }

    async fn handle_invite_request(&self, request: InviteRequest) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let metadata = match hosted::get_game_metadata(store.connection(), &self.game_id) {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to load metadata for {}: {}", self.game_id, e);
                return;
            }
        };
        let settings = match hosted::get_settings(store.connection(), &self.game_id) {
            Ok(settings) => settings,
            Err(e) => {
                tracing::error!("Failed to load settings for {}: {}", self.game_id, e);
                return;
            }
        };

        let receipt = if metadata.status == "finished" {
            InviteRequestReceipt {
                request_id: request.request_id.clone(),
                game_id: self.game_id.clone(),
                status: InviteRequestReceiptStatus::GameClosed,
                message: "This game is closed to new invite requests.".to_string(),
            }
        } else if settings.recruiting == hosted::RecruitingMode::None {
            InviteRequestReceipt {
                request_id: request.request_id.clone(),
                game_id: self.game_id.clone(),
                status: InviteRequestReceiptStatus::NotRecruiting,
                message: "This game is not recruiting right now.".to_string(),
            }
        } else {
            match hosted::get_pending_request_count(
                store.connection(),
                &self.game_id,
                &request.player_pubkey,
            ) {
                Ok(count) if count > 0 => InviteRequestReceipt {
                    request_id: request.request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: InviteRequestReceiptStatus::RateLimited,
                    message: "You already have a pending invite request for this game.".to_string(),
                },
                Ok(_) => {
                    if let Err(message) = self
                        .validate_player_handle(&request.player_pubkey, request.handle.as_deref())
                    {
                        return self.publish_invite_receipt(
                            store.connection(),
                            &request.player_pubkey,
                            &InviteRequestReceipt {
                                request_id: request.request_id.clone(),
                                game_id: self.game_id.clone(),
                                status: InviteRequestReceiptStatus::HandleTaken,
                                message,
                            },
                        );
                    }

                    if let Err(e) = hosted::create_request(
                        store.connection(),
                        &request.request_id,
                        &self.game_id,
                        &request.player_pubkey,
                        &request.message,
                    ) {
                        tracing::error!("Failed to store invite request: {}", e);
                        return;
                    }

                    self.refresh_player_seen(&request.player_pubkey, request.handle.as_deref());

                    if settings.game_tier == GameTier::Sandbox {
                        self.sandbox_auto_approve(&request)
                    } else {
                        if let Err(e) = crate::lobby::notify_sysop::enqueue_invite_request_summary(
                            &store,
                            &self.game_id,
                            &request.request_id,
                            &request.player_pubkey,
                            request.handle.as_deref(),
                        ) {
                            tracing::warn!(
                                "Failed to queue sysop invite notification {} for {}: {}",
                                request.request_id,
                                self.game_id,
                                e
                            );
                        }

                        InviteRequestReceipt {
                            request_id: request.request_id.clone(),
                            game_id: self.game_id.clone(),
                            status: InviteRequestReceiptStatus::Received,
                            message: "Your request has been queued for the sysop.".to_string(),
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to check pending request count: {}", e);
                    return;
                }
            }
        };

        self.publish_invite_receipt(store.connection(), &request.player_pubkey, &receipt);
    }

    fn sandbox_auto_approve(&self, request: &InviteRequest) -> InviteRequestReceipt {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store for sandbox auto-approve: {}", e);
                return self.queued_receipt(request);
            }
        };

        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            return self.queued_receipt(request);
        };
        let claimed_year = match current_runtime_year(game_dir) {
            Ok(year) => year,
            Err(e) => {
                tracing::error!(
                    "Failed to load runtime year for sandbox auto-approve: {}",
                    e
                );
                return self.queued_receipt(request);
            }
        };

        match hosted::auto_approve_sandbox_request(
            store.connection(),
            &request.request_id,
            &self.game_id,
            &request.player_pubkey,
            claimed_year,
            "Auto-approved for sandbox game.",
        ) {
            Ok(hosted::SandboxApprovalOutcome::Claimed { seat }) => {
                if let Err(e) = crate::lobby::invite_decisions::enqueue_invite_decision(
                    &store,
                    &self.game_id,
                    &request.player_pubkey,
                    &request.request_id,
                    InviteDecision::Approved { seat },
                    "Auto-approved for sandbox game.",
                ) {
                    tracing::warn!(
                        "Failed to enqueue sandbox auto-approve decision for {}: {}",
                        request.request_id,
                        e
                    );
                }

                tracing::info!(
                    "Sandbox auto-approved request {} seat {} for {}",
                    request.request_id,
                    seat,
                    short_pubkey(&request.player_pubkey)
                );

                InviteRequestReceipt {
                    request_id: request.request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: InviteRequestReceiptStatus::Received,
                    message: format!("Auto-approved for sandbox seat {}.", seat),
                }
            }
            Ok(hosted::SandboxApprovalOutcome::Full) => {
                if let Err(e) = hosted::delete_request(store.connection(), &request.request_id) {
                    tracing::warn!(
                        "Failed to clear full sandbox request {} for {}: {}",
                        request.request_id,
                        self.game_id,
                        e
                    );
                }
                InviteRequestReceipt {
                    request_id: request.request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: InviteRequestReceiptStatus::GameFull,
                    message: "This sandbox is full right now.".to_string(),
                }
            }
            Err(e) => {
                tracing::error!(
                    "Sandbox auto-approve failed for request {}: {}",
                    request.request_id,
                    e
                );
                self.queued_receipt(request)
            }
        }
    }

    fn roster_db_path(&self) -> Option<std::path::PathBuf> {
        self.db_path.parent()?.parent().map(|p| p.join("roster.db"))
    }

    fn queued_receipt(&self, request: &InviteRequest) -> InviteRequestReceipt {
        InviteRequestReceipt {
            request_id: request.request_id.clone(),
            game_id: self.game_id.clone(),
            status: InviteRequestReceiptStatus::Received,
            message: "Your request has been queued for the sysop.".to_string(),
        }
    }

    async fn handle_seat_claim(&self, request: SeatClaimRequest) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let invite_token = request
            .invite_code
            .split_once('@')
            .map(|(token, _)| token)
            .unwrap_or(request.invite_code.as_str())
            .trim()
            .to_ascii_lowercase();

        let invite_hash = blake3::hash(invite_token.as_bytes()).to_hex().to_string();
        let result =
            match hosted::find_seat_by_invite_hash(store.connection(), &self.game_id, &invite_hash)
            {
                Ok(Some(seat)) if seat.status == hosted::SeatStatus::Pending => {
                    let Some(game_dir) = self.db_path.parent() else {
                        tracing::error!(
                            "Hosted db path has no parent for {}",
                            self.db_path.display()
                        );
                        return;
                    };
                    let claimed_year = match current_runtime_year(game_dir) {
                        Ok(year) => year,
                        Err(e) => {
                            tracing::error!(
                                "Failed to load runtime year for seat claim in {}: {}",
                                self.game_id,
                                e
                            );
                            return;
                        }
                    };
                    match hosted::claim_seat(
                        store.connection(),
                        &self.game_id,
                        seat.seat_number,
                        &request.player_pubkey,
                        claimed_year,
                    ) {
                        Ok(()) => {
                            let _ = hosted::mark_catalog_dirty(store.connection(), &self.game_id);
                            if let Some(roster_db) = self.roster_db_path() {
                                if let Ok(rs) = hosted::RosterStore::open(&roster_db) {
                                    let _ = hosted::record_player_joined(
                                        rs.connection(),
                                        &request.player_pubkey,
                                        &self.game_id,
                                        seat.seat_number,
                                    );
                                }
                            }
                            SeatClaimResultPayload {
                                nonce: request.nonce.clone(),
                                game_id: Some(self.game_id.clone()),
                                status: SeatClaimStatus::Claimed,
                                message: format!("Seat {} claimed.", seat.seat_number),
                                seat: Some(seat.seat_number),
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to claim seat {}: {}", seat.seat_number, e);
                            SeatClaimResultPayload {
                                nonce: request.nonce.clone(),
                                game_id: Some(self.game_id.clone()),
                                status: SeatClaimStatus::AlreadyClaimed,
                                message: "That invite is no longer available.".to_string(),
                                seat: Some(seat.seat_number),
                            }
                        }
                    }
                }
                Ok(Some(seat)) => {
                    let same_player =
                        seat.player_pubkey.as_deref() == Some(request.player_pubkey.as_str());
                    SeatClaimResultPayload {
                        nonce: request.nonce.clone(),
                        game_id: Some(self.game_id.clone()),
                        status: if same_player {
                            SeatClaimStatus::Claimed
                        } else {
                            SeatClaimStatus::AlreadyClaimed
                        },
                        message: if same_player {
                            format!("Seat {} already claimed by this player.", seat.seat_number)
                        } else {
                            "That invite has already been claimed.".to_string()
                        },
                        seat: Some(seat.seat_number),
                    }
                }
                Ok(None) => SeatClaimResultPayload {
                    nonce: request.nonce.clone(),
                    game_id: Some(self.game_id.clone()),
                    status: SeatClaimStatus::InvalidInvite,
                    message: "Unknown invite code.".to_string(),
                    seat: None,
                },
                Err(e) => {
                    tracing::error!("Failed to lookup invite hash: {}", e);
                    return;
                }
            };

        let content = match serde_json::to_string(&result) {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("Failed to serialize seat claim result: {}", e);
                return;
            }
        };

        let tags = nc_nostr::claim::build_seat_claim_result_tags(&result)
            .into_iter()
            .map(|(key, value)| vec![key.to_string(), value])
            .collect();

        if let Err(e) = enqueue_encrypted_event(
            store.connection(),
            &self.game_id,
            &request.player_pubkey,
            30511,
            &content,
            tags,
        ) {
            tracing::error!("Failed to enqueue seat claim result: {}", e);
        }
    }

    async fn handle_sandbox_release(&self, request: SandboxReleaseRequest) {
        let request_id = request.request_id.clone();
        let player_pubkey = request.player_pubkey.clone();
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("Failed to open store: {}", err);
                return;
            }
        };

        let settings = match hosted::get_settings(store.connection(), &self.game_id) {
            Ok(settings) => settings,
            Err(err) => {
                tracing::error!("Failed to load settings for {}: {}", self.game_id, err);
                let _ = self.enqueue_sandbox_release_result(
                    store.connection(),
                    &player_pubkey,
                    SandboxReleaseResult {
                        request_id: request_id.clone(),
                        game_id: self.game_id.clone(),
                        status: SandboxReleaseStatus::Rejected,
                        message: "Unable to load hosted sandbox settings.".to_string(),
                    },
                );
                return;
            }
        };

        if settings.game_tier != GameTier::Sandbox {
            let _ = self.enqueue_sandbox_release_result(
                store.connection(),
                &player_pubkey,
                SandboxReleaseResult {
                    request_id: request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: SandboxReleaseStatus::Rejected,
                    message: "Only sandbox games can be deleted from My Games.".to_string(),
                },
            );
            return;
        }

        let seat =
            match hosted::get_seat_by_pubkey(store.connection(), &self.game_id, &player_pubkey) {
                Ok(Some(seat)) => seat,
                Ok(None) => {
                    let _ = self.enqueue_sandbox_release_result(
                        store.connection(),
                        &player_pubkey,
                        SandboxReleaseResult {
                            request_id: request_id.clone(),
                            game_id: self.game_id.clone(),
                            status: SandboxReleaseStatus::Rejected,
                            message: "You no longer have a claimed sandbox seat in this game."
                                .to_string(),
                        },
                    );
                    return;
                }
                Err(err) => {
                    tracing::error!("Failed to lookup sandbox seat: {}", err);
                    let _ = self.enqueue_sandbox_release_result(
                        store.connection(),
                        &player_pubkey,
                        SandboxReleaseResult {
                            request_id: request_id.clone(),
                            game_id: self.game_id.clone(),
                            status: SandboxReleaseStatus::Rejected,
                            message: "Unable to look up your sandbox seat.".to_string(),
                        },
                    );
                    return;
                }
            };

        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            let _ = self.enqueue_sandbox_release_result(
                store.connection(),
                &player_pubkey,
                SandboxReleaseResult {
                    request_id: request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: SandboxReleaseStatus::Rejected,
                    message: "Hosted sandbox state is unavailable right now.".to_string(),
                },
            );
            return;
        };

        if let Err(err) = reset_hosted_player_to_baseline(game_dir, seat.seat_number) {
            tracing::error!(
                "Failed to reset sandbox runtime seat {} in {}: {}",
                seat.seat_number,
                self.game_id,
                err
            );
            let _ = self.enqueue_sandbox_release_result(
                store.connection(),
                &player_pubkey,
                SandboxReleaseResult {
                    request_id: request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: SandboxReleaseStatus::Rejected,
                    message: "Unable to reset the hosted sandbox seat.".to_string(),
                },
            );
            return;
        }

        if let Err(err) = hosted::reset_seat(store.connection(), &self.game_id, seat.seat_number) {
            tracing::error!(
                "Failed to reset sandbox seat {} in {}: {}",
                seat.seat_number,
                self.game_id,
                err
            );
            let _ = self.enqueue_sandbox_release_result(
                store.connection(),
                &player_pubkey,
                SandboxReleaseResult {
                    request_id: request_id.clone(),
                    game_id: self.game_id.clone(),
                    status: SandboxReleaseStatus::Rejected,
                    message: "Unable to release your sandbox seat.".to_string(),
                },
            );
            return;
        }

        if settings.recruiting == hosted::RecruitingMode::None {
            let mut reopened = settings.clone();
            reopened.recruiting = hosted::RecruitingMode::NewPlayers;
            if let Err(err) = hosted::update_settings(store.connection(), &self.game_id, &reopened)
            {
                tracing::warn!(
                    "Failed to reopen recruiting for released sandbox {}: {}",
                    self.game_id,
                    err
                );
            }
        }

        if let Err(err) = hosted::mark_catalog_dirty(store.connection(), &self.game_id) {
            tracing::warn!(
                "Failed to mark sandbox catalog dirty for {}: {}",
                self.game_id,
                err
            );
        }

        let _ = self.enqueue_sandbox_release_result(
            store.connection(),
            &player_pubkey,
            SandboxReleaseResult {
                request_id,
                game_id: self.game_id.clone(),
                status: SandboxReleaseStatus::Accepted,
                message: "Sandbox seat released.".to_string(),
            },
        );
    }

    async fn handle_turn_commands(&self, commands: TurnCommands) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let _seat = match hosted::get_seat_by_pubkey(
            store.connection(),
            &self.game_id,
            &commands.player_pubkey,
        ) {
            Ok(Some(s)) => s,
            Ok(None) => {
                let receipt = TurnReceipt {
                    submit_id: commands.submit_id.clone(),
                    game_id: self.game_id.clone(),
                    turn: commands.turn,
                    status: TurnReceiptStatus::NotClaimed,
                    message: Some("Player has not claimed a seat in this hosted game.".to_string()),
                    errors: vec![TurnReceiptError {
                        path: "player".to_string(),
                        message: "unclaimed_seat".to_string(),
                    }],
                };
                self.publish_turn_receipt(store.connection(), &commands.player_pubkey, &receipt);
                return;
            }
            Err(e) => {
                tracing::error!("Failed to lookup seat: {}", e);
                return;
            }
        };

        if let Err(message) =
            self.validate_player_handle(&commands.player_pubkey, commands.handle.as_deref())
        {
            let receipt = TurnReceipt {
                submit_id: commands.submit_id.clone(),
                game_id: self.game_id.clone(),
                turn: commands.turn,
                status: TurnReceiptStatus::Rejected,
                message: Some(message),
                errors: vec![TurnReceiptError {
                    path: "handle".to_string(),
                    message: "handle_taken".to_string(),
                }],
            };
            self.publish_turn_receipt(store.connection(), &commands.player_pubkey, &receipt);
            return;
        }
        self.refresh_player_seen(&commands.player_pubkey, commands.handle.as_deref());

        // A newer submission for the same turn replaces the one staged before it.
        match hosted::get_pending_turn(
            store.connection(),
            &self.game_id,
            commands.turn,
            &commands.player_pubkey,
        ) {
            Ok(Some(previous)) => {
                if let Err(e) = hosted::mark_superseded(
                    store.connection(),
                    &self.game_id,
                    commands.turn,
                    &commands.player_pubkey,
                ) {
                    tracing::error!("Failed to supersede turn {}: {}", previous.id, e);
                } else {
                    let receipt = turn_receipt(
                        &previous,
                        TurnReceiptStatus::Superseded,
                        "Replaced by a newer submission for this turn.",
                        vec![],
                    );
                    self.publish_turn_receipt(
                        store.connection(),
                        &previous.player_pubkey,
                        &receipt,
                    );
                }
            }
            Ok(None) => {}
            Err(e) => tracing::error!("Failed to check for an earlier submission: {}", e),
        }

        if let Err(e) = hosted::enqueue_turn(
            store.connection(),
            &commands.submit_id,
            &self.game_id,
            commands.turn,
            &commands.player_pubkey,
            &commands.commands,
        ) {
            tracing::error!("Failed to enqueue turn: {}", e);
            let receipt = TurnReceipt {
                submit_id: commands.submit_id.clone(),
                game_id: self.game_id.clone(),
                turn: commands.turn,
                status: TurnReceiptStatus::Rejected,
                message: Some("Failed to stage turn commands.".to_string()),
                errors: vec![TurnReceiptError {
                    path: "commands".to_string(),
                    message: e.to_string(),
                }],
            };
            self.publish_turn_receipt(store.connection(), &commands.player_pubkey, &receipt);
            return;
        }

        self.process_turn_submission(&commands.submit_id);
    }

    /// Validate a staged submission against the live snapshot and answer
    /// it: accepted ones stay pending for maintenance, the rest are settled
    /// in the queue with the reason the receipt carries.
    pub fn process_turn_submission(&self, submit_id: &str) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };
        let queued = match hosted::get_turn_submission(store.connection(), submit_id) {
            Ok(Some(queued)) if queued.status == hosted::TurnSubmissionStatus::Pending => queued,
            Ok(Some(queued)) => {
                tracing::debug!(
                    "Turn submission {} for {} is already {}",
                    submit_id,
                    self.game_id,
                    queued.status.as_str()
                );
                return;
            }
            Ok(None) => {
                tracing::warn!("Unknown turn submission {} for {}", submit_id, self.game_id);
                return;
            }
            Err(e) => {
                tracing::error!("Failed to load turn submission {}: {}", submit_id, e);
                return;
            }
        };
        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            return;
        };

        let receipt = match validate_turn_submission(&store, game_dir, &self.game_id, &queued) {
            Ok(()) => turn_receipt(
                &queued,
                TurnReceiptStatus::Accepted,
                "Orders staged for the next maintenance run.",
                vec![],
            ),
            Err(rejection) => {
                if let Err(e) =
                    hosted::reject_turn(store.connection(), &queued.id, &rejection.message)
                {
                    tracing::error!("Failed to record rejected turn {}: {}", queued.id, e);
                }
                rejection.into_receipt(&queued)
            }
        };
        self.publish_turn_receipt(store.connection(), &queued.player_pubkey, &receipt);
    }

    fn run_scheduled_maintenance(&self) {
        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            return;
        };
        match crate::game::maint::run_scheduled_maintenance(game_dir, &self.game_id) {
            Ok(report) => tracing::info!(
                "Maintenance for {} advanced {} -> {}: {} submissions applied, {} rejected, {} seats sent their state",
                self.game_id,
                report.start_year,
                report.end_year,
                report.applied_submissions,
                report.rejected_submissions,
                report.published_seats
            ),
            Err(e) => tracing::error!("Maintenance for {} failed: {}", self.game_id, e),
        }
    }

    async fn handle_thread_message(&self, message: nc_nostr::thread_message::SysopThreadMessage) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        if let Err(e) = crate::lobby::threads::store_player_message(&store, &self.game_id, &message)
        {
            tracing::error!(
                "Failed to store thread message {} for {}: {}",
                message.message_id,
                self.game_id,
                e
            );
            return;
        }

        if message.sender_role == nc_nostr::thread_message::SenderRole::Player {
            if let Err(e) = crate::lobby::notify_sysop::enqueue_thread_message_summary(
                &store,
                &self.game_id,
                &message.message_id,
                &message.sender_pubkey,
                message.sender_handle.as_deref(),
            ) {
                tracing::warn!(
                    "Failed to queue sysop thread notification {} for {}: {}",
                    message.message_id,
                    self.game_id,
                    e
                );
            }
        }

        tracing::info!(
            "Stored thread message {} for game {} from {}",
            message.message_id,
            self.game_id,
            short_pubkey(&message.sender_npub)
        );
    }

    async fn handle_player_message(&self, message: nc_nostr::player_message::PlayerMessageRequest) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let sender_seat = match hosted::get_seat_by_pubkey(
            store.connection(),
            &self.game_id,
            &message.sender_pubkey,
        ) {
            Ok(Some(seat)) => seat,
            Ok(None) => {
                tracing::warn!(
                    "Ignoring player message {} from unclaimed player {} in {}",
                    message.message_id,
                    short_pubkey(&message.sender_pubkey),
                    self.game_id
                );
                return;
            }
            Err(err) => {
                tracing::error!("Failed to lookup sender seat: {}", err);
                return;
            }
        };

        if sender_seat.seat_number == u32::from(message.recipient_empire_id) {
            tracing::warn!(
                "Ignoring self-directed player message {}",
                message.message_id
            );
            return;
        }

        let recipient_seat = match hosted::get_seat_by_number(
            store.connection(),
            &self.game_id,
            u32::from(message.recipient_empire_id),
        ) {
            Ok(Some(seat)) if seat.status == hosted::SeatStatus::Claimed => seat,
            Ok(_) => {
                tracing::warn!(
                    "Ignoring player message {} for unavailable empire {}",
                    message.message_id,
                    message.recipient_empire_id
                );
                return;
            }
            Err(err) => {
                tracing::error!("Failed to lookup recipient seat: {}", err);
                return;
            }
        };

        let Some(recipient_pubkey) = recipient_seat.player_pubkey.as_deref() else {
            tracing::warn!(
                "Ignoring player message {} with missing recipient pubkey",
                message.message_id
            );
            return;
        };

        let Some(game_dir) = self.db_path.parent() else {
            tracing::error!(
                "Hosted db path has no parent for {}",
                self.db_path.display()
            );
            return;
        };
        let game_data = match crate::support::runtime::load_runtime_game_data(game_dir) {
            Ok(game_data) => game_data,
            Err(err) => {
                tracing::error!("Failed to load core game data for player message: {}", err);
                return;
            }
        };
        let sender_name = player_empire_name(&game_data, sender_seat.seat_number as u8);
        let recipient_name = player_empire_name(&game_data, message.recipient_empire_id);
        let payload = nc_nostr::player_message::PlayerMessage {
            message_id: message.message_id.clone(),
            game_id: self.game_id.clone(),
            sender_empire_id: sender_seat.seat_number as u8,
            sender_empire_name: sender_name,
            recipient_empire_id: message.recipient_empire_id,
            recipient_empire_name: recipient_name,
            body: message.body.trim().to_string(),
            created_at: message.created_at,
        };

        if let Err(err) = crate::lobby::player_messages::store_message(
            &store,
            &self.game_id,
            &payload,
            &message.sender_pubkey,
            recipient_pubkey,
        ) {
            tracing::error!(
                "Failed to store player message {} for {}: {}",
                payload.message_id,
                self.game_id,
                err
            );
            return;
        }

        if let Err(err) = crate::lobby::player_messages::enqueue_message(
            &store,
            &self.game_id,
            recipient_pubkey,
            &payload,
        ) {
            tracing::error!(
                "Failed to enqueue recipient player message {}: {}",
                payload.message_id,
                err
            );
            return;
        }
        if let Err(err) = crate::lobby::player_messages::enqueue_message(
            &store,
            &self.game_id,
            &message.sender_pubkey,
            &payload,
        ) {
            tracing::error!(
                "Failed to enqueue sender player message copy {}: {}",
                payload.message_id,
                err
            );
            return;
        }

        tracing::info!(
            "Stored player message {} for game {} from empire {} to empire {}",
            payload.message_id,
            self.game_id,
            payload.sender_empire_id,
            payload.recipient_empire_id
        );
    }

    fn publish_turn_receipt(
        &self,
        conn: &rusqlite::Connection,
        player_pubkey: &str,
        receipt: &TurnReceipt,
    ) {
        if let Err(e) = enqueue_turn_receipt(conn, &self.game_id, player_pubkey, receipt) {
            tracing::error!("Failed to enqueue turn receipt: {}", e);
        } else {
            tracing::info!(
                "Queued encrypted turn receipt ({}) for {}",
                receipt.status.as_str(),
                short_pubkey(player_pubkey)
            );
        }
    }

    fn publish_invite_receipt(
        &self,
        conn: &rusqlite::Connection,
        player_pubkey: &str,
        receipt: &InviteRequestReceipt,
    ) {
        let content = match serde_json::to_string(receipt) {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("Failed to serialize invite receipt: {}", e);
                return;
            }
        };

        let tags = nc_nostr::invite_request::build_invite_request_receipt_tags(receipt)
            .into_iter()
            .map(|(key, value)| vec![key.to_string(), value])
            .collect();

        if let Err(e) =
            enqueue_encrypted_event(conn, &self.game_id, player_pubkey, 30514, &content, tags)
        {
            tracing::error!("Failed to enqueue invite receipt: {}", e);
        } else {
            tracing::info!(
                "Queued encrypted invite request receipt for {}",
                short_pubkey(player_pubkey)
            );
        }
    }

    fn enqueue_sandbox_release_result(
        &self,
        conn: &rusqlite::Connection,
        recipient_pubkey: &str,
        result: SandboxReleaseResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string(&result)?;
        let tags = nc_nostr::sandbox_release::build_sandbox_release_result_tags(&result)
            .into_iter()
            .map(|(key, value)| vec![key.to_string(), value])
            .collect();
        enqueue_encrypted_event(conn, &self.game_id, recipient_pubkey, 30530, &content, tags)?;
        Ok(())
    }

    fn validate_player_handle(
        &self,
        player_pubkey: &str,
        handle: Option<&str>,
    ) -> Result<(), String> {
        let Some(handle) = handle.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        let Some(roster_db) = self.roster_db_path() else {
            return Ok(());
        };
        let roster = hosted::RosterStore::open(&roster_db).map_err(|err| err.to_string())?;
        match hosted::resolve_handle_ownership(roster.connection(), player_pubkey, handle)
            .map_err(|err| err.to_string())?
        {
            HandleOwnership::Available | HandleOwnership::OwnedBySelf => Ok(()),
            HandleOwnership::Taken => Err(format!(
                "Handle '{}' is already used on this nc-host. Choose another handle.",
                handle
            )),
        }
    }

    fn refresh_player_seen(&self, player_pubkey: &str, handle: Option<&str>) {
        let Some(roster_db) = self.roster_db_path() else {
            return;
        };
        if let Ok(roster) = hosted::RosterStore::open(&roster_db) {
            let _ = hosted::upsert_player_seen(
                roster.connection(),
                player_pubkey,
                handle,
                &self.game_id,
            );
        }
    }
}

fn player_empire_name(game_data: &nc_data::CoreGameData, empire_id: u8) -> String {
    let Some(player) = game_data.players.get(empire_id.saturating_sub(1) as usize) else {
        return format!("Seat {}", empire_id);
    };
    let empire_name = player.empire_name.trim().to_string();
    if empire_name.is_empty() {
        format!("Seat {}", empire_id)
    } else {
        empire_name
    }
}

#[derive(Clone)]
pub struct GameWorkerHandle {
    pub game_id: String,
    sender: mpsc::Sender<GameMsg>,
}

impl fmt::Debug for GameWorkerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameWorkerHandle")
            .field("game_id", &self.game_id)
            .finish()
    }
}

impl GameWorkerHandle {
    pub async fn send(&self, msg: GameMsg) -> Result<(), mpsc::error::SendError<GameMsg>> {
        self.sender.send(msg).await
    }
}

pub fn spawn_worker(game_id: String, db_path: PathBuf) -> GameWorkerHandle {
    let (tx, mut rx) = mpsc::channel::<GameMsg>(100);

    let worker = GameWorker::new(game_id.clone(), db_path);
    let worker = Arc::new(worker);

    let game_id_clone = game_id.clone();

    tokio::spawn(async move {
        tracing::debug!("Game worker started for {}", game_id_clone);

        while let Some(msg) = rx.recv().await {
            match msg {
                GameMsg::Tick => {}
                GameMsg::PublishLobbyCatalog => {}
                GameMsg::ProcessInviteRequest { request_id } => {
                    tracing::debug!(
                        "Processing invite request {} for {}",
                        request_id,
                        game_id_clone
                    );
                }
                GameMsg::ProcessTurnSubmission { submit_id } => {
                    worker.process_turn_submission(&submit_id);
                }
                GameMsg::HandleEffect(effect) => {
                    worker.handle_effect(effect).await;
                }
            }
        }

        tracing::debug!("Game worker stopped for {}", game_id_clone);
    });

    GameWorkerHandle {
        game_id,
        sender: tx,
    }
}
