pub mod invite_requests;
pub mod outbox;
pub mod player_messages;
pub mod player_roster;
pub mod schema;
pub mod seats;
pub mod settings;
pub mod state_sync;
pub mod store;
pub mod sysop_notifications;
pub mod threads;
pub mod turn_queue;

pub use invite_requests::{
    InviteRequest, InviteRequestStatus, SandboxApprovalOutcome, approve_request,
    approve_request_for_seat, auto_approve_sandbox_request, count_pending_requests,
    count_unpublished_decisions, create_request, delete_request, get_pending_request_count,
    get_request, list_pending_decisions, list_requests, mark_decision_published, reject_request,
};
pub use outbox::{
    OutboxEvent, OutboxItem, OutboxStatus, count_by_status, delete_published_older_than, enqueue,
    get_pending, increment_retry, mark_failed, mark_published,
};
pub use player_messages::{
    PlayerMessage as HostedPlayerMessage, list_messages as list_player_messages,
    store_message as store_player_message,
};
pub use player_roster::{
    HandleOwnership, RosterEntry, RosterEvent, RosterStore, get_roster_entry, list_roster,
    list_roster_events_for_npub, record_player_abandoned, record_player_joined,
    resolve_handle_ownership, upsert_player_seen,
};
pub use schema::INIT_SQL;
pub use seats::{
    Seat, SeatStatus, claim_seat, close_seat, create_seats, find_seat_by_invite_hash,
    get_seat_by_number, get_seat_by_pubkey, list_seats, open_seat, reissue_seat, reset_seat,
    set_claimed_year,
};
pub use settings::{
    CatalogState, GameMetadata, GameSettings, GameTier, LobbyVisibility, RecruitingMode,
    clear_catalog_dirty, get_catalog_dirty_since, get_game_metadata, get_settings,
    mark_catalog_dirty, set_game_clock, update_settings,
};
pub use state_sync::{get_latest_state_snapshot, get_state_snapshot_by_hash, save_state_snapshot};
pub use store::HostedStore;
pub use sysop_notifications::{
    SysopNotification, SysopNotificationStatus, enqueue as enqueue_sysop_notification,
    get_pending as get_pending_sysop_notifications, mark_failed as mark_sysop_notification_failed,
    mark_sent as mark_sysop_notification_sent,
};
pub use threads::{
    ThreadMessage as HostedThreadMessage, list_messages as list_thread_messages,
    list_thread_players, store_message as store_thread_message,
};
pub use turn_queue::{
    TurnSubmission, TurnSubmissionStatus, accept_turn, count_pending_turns, enqueue_turn,
    get_pending_turn, get_turn_submission, list_pending_turns, mark_superseded, reject_turn,
};
