use std::collections::BTreeMap;

use crate::reports::InboxItemSource;
use crate::screen::layout::PromptFeedback;

pub const INBOX_VISIBLE_ROWS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxTypeFilter {
    All,
    Messages,
    Reports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxFocus {
    Inbox,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxPromptMode {
    Normal,
    YearInput,
    DeleteConfirm,
}

/// What the queued-message outbox shows: the list, one message to read, or
/// the delete prompt for the selected message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxMode {
    List,
    Read,
    DeleteConfirm,
}

pub struct MessagingState {
    pub compose_recipient_input: String,
    pub compose_recipient_status: Option<String>,
    pub compose_recipient_scroll_offset: usize,
    pub compose_recipient_cursor: usize,
    pub compose_recipient_empire: Option<u8>,
    pub compose_subject: String,
    pub compose_subject_status: Option<String>,
    pub compose_body: String,
    pub compose_body_cursor_row: usize,
    pub compose_body_cursor_col: usize,
    pub compose_body_status: Option<String>,
    pub compose_outbox_input: String,
    pub compose_outbox_status: Option<String>,
    pub compose_outbox_scroll_offset: usize,
    pub compose_outbox_cursor: usize,
    pub compose_outbox_mode: OutboxMode,
    pub compose_sent_status: Option<String>,
    pub delete_reviewables_prompt_active: bool,
    pub inbox_type_filter: InboxTypeFilter,
    pub inbox_year_filter: Option<u16>,
    pub inbox_cursor: usize,
    pub inbox_scroll_offset: usize,
    pub inbox_preview_scroll: usize,
    pub inbox_focus: InboxFocus,
    pub inbox_id_input: String,
    pub inbox_year_input: String,
    pub inbox_prompt_mode: InboxPromptMode,
    pub inbox_feedback: Option<PromptFeedback>,
    pub inbox_display_ids: BTreeMap<InboxItemSource, usize>,
    pub inbox_next_display_id: usize,
}

impl Default for MessagingState {
    fn default() -> Self {
        Self {
            compose_recipient_input: String::new(),
            compose_recipient_status: None,
            compose_recipient_scroll_offset: 0,
            compose_recipient_cursor: 0,
            compose_recipient_empire: None,
            compose_subject: String::new(),
            compose_subject_status: None,
            compose_body: String::new(),
            compose_body_cursor_row: 0,
            compose_body_cursor_col: 0,
            compose_body_status: None,
            compose_outbox_input: String::new(),
            compose_outbox_status: None,
            compose_outbox_scroll_offset: 0,
            compose_outbox_cursor: 0,
            compose_outbox_mode: OutboxMode::List,
            compose_sent_status: None,
            delete_reviewables_prompt_active: false,
            inbox_type_filter: InboxTypeFilter::All,
            inbox_year_filter: None,
            inbox_cursor: 0,
            inbox_scroll_offset: 0,
            inbox_preview_scroll: 0,
            inbox_focus: InboxFocus::Inbox,
            inbox_id_input: String::new(),
            inbox_year_input: String::new(),
            inbox_prompt_mode: InboxPromptMode::Normal,
            inbox_feedback: None,
            inbox_display_ids: BTreeMap::new(),
            inbox_next_display_id: 1,
        }
    }
}
