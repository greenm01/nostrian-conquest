use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::app::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::domains::messaging::MessagingAction;
use crate::domains::messaging::state::{
    INBOX_VISIBLE_ROWS, InboxFocus, InboxPromptMode, InboxTypeFilter,
};
use crate::reports::{
    InboxDisplayItem, InboxItem, InboxItemSource, InboxItemType, runtime_inbox_items,
};
use crate::screen::ScreenId;
use crate::screen::layout::{PromptFeedback, command_line_row_for};
use nc_data::delete_report_block_for_viewer;

const INBOX_PREVIEW_PAGE_DELTA: i8 = INBOX_VISIBLE_ROWS as i8;

impl App {
    pub fn open_reports_inbox(&mut self) {
        self.command_return_menu = self.origin_command_menu();
        self.messaging.inbox_type_filter = InboxTypeFilter::All;
        self.messaging.inbox_year_filter = None;
        self.messaging.inbox_cursor = 0;
        self.messaging.inbox_scroll_offset = 0;
        self.messaging.inbox_preview_scroll = 0;
        self.messaging.inbox_focus = InboxFocus::Inbox;
        self.messaging.inbox_id_input.clear();
        self.messaging.inbox_year_input.clear();
        self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
        self.messaging.inbox_feedback = None;
        self.current_screen = ScreenId::Reports;
        self.normalize_inbox_selection();
    }

    pub fn handle_reports_key(&self, key: KeyEvent) -> Action {
        match self.messaging.inbox_prompt_mode {
            InboxPromptMode::YearInput => match key.code {
                KeyCode::Enter => Action::Messaging(MessagingAction::SubmitInboxYearInput),
                KeyCode::Backspace => Action::Messaging(MessagingAction::BackspaceInboxYearInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Messaging(MessagingAction::CancelInboxPrompt)
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    Action::Messaging(MessagingAction::AppendInboxYearChar(ch))
                }
                _ => Action::Noop,
            },
            InboxPromptMode::DeleteConfirm => match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    Action::Messaging(MessagingAction::ConfirmDeleteInboxItem)
                }
                KeyCode::Char('n')
                | KeyCode::Char('N')
                | KeyCode::Char('q')
                | KeyCode::Char('Q')
                | KeyCode::Esc => Action::Messaging(MessagingAction::CancelInboxPrompt),
                _ => Action::Noop,
            },
            InboxPromptMode::Normal => match key.code {
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    Action::Messaging(MessagingAction::SetInboxTypeFilterMessages)
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    Action::Messaging(MessagingAction::SetInboxTypeFilterReports)
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    Action::Messaging(MessagingAction::SetInboxTypeFilterAll)
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if self.messaging.inbox_year_filter.is_some() {
                        Action::Messaging(MessagingAction::ClearInboxYearFilter)
                    } else {
                        Action::Messaging(MessagingAction::OpenInboxYearPrompt)
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    Action::Messaging(MessagingAction::OpenInboxDeleteConfirm)
                }
                KeyCode::Tab => Action::Messaging(MessagingAction::ToggleInboxFocus),
                KeyCode::Backspace => Action::Messaging(MessagingAction::BackspaceInboxIdInput),
                KeyCode::Enter => match self.messaging.inbox_focus {
                    InboxFocus::Inbox if self.messaging.inbox_id_input.trim().is_empty() => {
                        Action::Messaging(MessagingAction::ToggleInboxFocus)
                    }
                    InboxFocus::Inbox => Action::Messaging(MessagingAction::SubmitInboxIdInput),
                    InboxFocus::Preview => Action::Noop,
                },
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    Action::ReturnToCommandMenu
                }
                KeyCode::PageUp => match self.messaging.inbox_focus {
                    InboxFocus::Inbox => Action::Messaging(MessagingAction::PageInboxCursor(-1)),
                    InboxFocus::Preview => Action::Messaging(MessagingAction::PageInboxPreview(-1)),
                },
                KeyCode::PageDown => match self.messaging.inbox_focus {
                    InboxFocus::Inbox => Action::Messaging(MessagingAction::PageInboxCursor(1)),
                    InboxFocus::Preview => Action::Messaging(MessagingAction::PageInboxPreview(1)),
                },
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => match self
                    .messaging
                    .inbox_focus
                {
                    InboxFocus::Inbox => Action::Messaging(MessagingAction::MoveInboxCursor(-1)),
                    InboxFocus::Preview => {
                        Action::Messaging(MessagingAction::ScrollInboxPreview(-1))
                    }
                },
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    match self.messaging.inbox_focus {
                        InboxFocus::Inbox => Action::Messaging(MessagingAction::MoveInboxCursor(1)),
                        InboxFocus::Preview => {
                            Action::Messaging(MessagingAction::ScrollInboxPreview(1))
                        }
                    }
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => match self
                    .messaging
                    .inbox_focus
                {
                    InboxFocus::Inbox => Action::Messaging(MessagingAction::MoveInboxCursor(-1)),
                    InboxFocus::Preview => {
                        Action::Messaging(MessagingAction::ScrollInboxPreview(-1))
                    }
                },
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                    match self.messaging.inbox_focus {
                        InboxFocus::Inbox => Action::Messaging(MessagingAction::MoveInboxCursor(1)),
                        InboxFocus::Preview => {
                            Action::Messaging(MessagingAction::ScrollInboxPreview(1))
                        }
                    }
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    Action::Messaging(MessagingAction::AppendInboxIdChar(ch))
                }
                _ => Action::Noop,
            },
        }
    }

    pub fn set_inbox_type_filter_all(&mut self) {
        self.messaging.inbox_type_filter = InboxTypeFilter::All;
        self.clear_inbox_filter_feedback();
        self.reset_inbox_jump_and_preview();
        self.normalize_inbox_selection();
    }

    pub fn set_inbox_type_filter_messages(&mut self) {
        self.messaging.inbox_type_filter = InboxTypeFilter::Messages;
        self.clear_inbox_filter_feedback();
        self.reset_inbox_jump_and_preview();
        self.normalize_inbox_selection();
    }

    pub fn set_inbox_type_filter_reports(&mut self) {
        self.messaging.inbox_type_filter = InboxTypeFilter::Reports;
        self.clear_inbox_filter_feedback();
        self.reset_inbox_jump_and_preview();
        self.normalize_inbox_selection();
    }

    pub fn open_inbox_year_prompt(&mut self) {
        self.messaging.inbox_prompt_mode = InboxPromptMode::YearInput;
        self.messaging.inbox_year_input.clear();
        self.messaging.inbox_feedback = None;
    }

    pub fn clear_inbox_year_filter(&mut self) {
        self.messaging.inbox_year_filter = None;
        self.clear_inbox_filter_feedback();
        self.reset_inbox_jump_and_preview();
        self.normalize_inbox_selection();
    }

    pub fn append_inbox_year_char(&mut self, ch: char) {
        if self.messaging.inbox_prompt_mode == InboxPromptMode::YearInput
            && self.messaging.inbox_year_input.len() < 4
        {
            self.messaging.inbox_year_input.push(ch);
            self.messaging.inbox_feedback = None;
        }
    }

    pub fn backspace_inbox_year_input(&mut self) {
        if self.messaging.inbox_prompt_mode == InboxPromptMode::YearInput {
            self.messaging.inbox_year_input.pop();
            self.messaging.inbox_feedback = None;
        }
    }

    pub fn submit_inbox_year_input(&mut self) {
        if self.messaging.inbox_prompt_mode != InboxPromptMode::YearInput {
            return;
        }
        let year = if self.messaging.inbox_year_input.trim().is_empty() {
            self.game_data.conquest.game_year()
        } else {
            let Ok(year) = self.messaging.inbox_year_input.trim().parse::<u16>() else {
                self.messaging.inbox_feedback =
                    Some(PromptFeedback::error("Enter a 4-digit year."));
                return;
            };
            year
        };
        if self
            .inbox_items_for_filters(self.messaging.inbox_type_filter, Some(year))
            .is_empty()
        {
            self.messaging.inbox_year_input.clear();
            self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
            self.messaging.inbox_feedback = None;
            return;
        }
        self.messaging.inbox_year_filter = Some(year);
        self.messaging.inbox_year_input.clear();
        self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
        self.clear_inbox_filter_feedback();
        self.reset_inbox_jump_and_preview();
        self.normalize_inbox_selection();
    }

    pub fn move_inbox_cursor(&mut self, delta: i8) {
        let total = self.filtered_inbox_items().len();
        if total == 0 {
            return;
        }
        let next = self.messaging.inbox_cursor as isize + delta as isize;
        self.messaging.inbox_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.messaging.inbox_scroll_offset,
            self.messaging.inbox_cursor,
            INBOX_VISIBLE_ROWS,
        );
        self.messaging.inbox_preview_scroll = 0;
        self.messaging.inbox_id_input.clear();
        self.messaging.inbox_feedback = None;
    }

    pub fn page_inbox_cursor(&mut self, delta: i8) {
        self.move_inbox_cursor(delta.saturating_mul(INBOX_VISIBLE_ROWS as i8));
    }

    pub fn scroll_inbox_preview(&mut self, delta: i8) {
        let visible_rows = self.inbox_preview_visible_rows();
        let total = self.current_inbox_preview_lines().len();
        let max_scroll = total.saturating_sub(visible_rows);
        self.messaging.inbox_preview_scroll = self
            .messaging
            .inbox_preview_scroll
            .saturating_add_signed(delta as isize)
            .min(max_scroll);
        self.messaging.inbox_feedback = None;
    }

    pub fn page_inbox_preview(&mut self, delta: i8) {
        self.scroll_inbox_preview(delta.saturating_mul(INBOX_PREVIEW_PAGE_DELTA));
    }

    pub fn toggle_inbox_focus(&mut self) {
        self.messaging.inbox_focus = match self.messaging.inbox_focus {
            InboxFocus::Inbox => InboxFocus::Preview,
            InboxFocus::Preview => InboxFocus::Inbox,
        };
        self.messaging.inbox_feedback = None;
    }

    pub fn append_inbox_id_char(&mut self, ch: char) {
        if self.messaging.inbox_prompt_mode != InboxPromptMode::Normal
            || self.messaging.inbox_id_input.len() >= 4
        {
            return;
        }
        self.messaging.inbox_id_input.push(ch);
        if self.sync_inbox_cursor_to_id_input() {
            self.messaging.inbox_id_input.clear();
        }
        self.messaging.inbox_feedback = None;
    }

    pub fn backspace_inbox_id_input(&mut self) {
        if self.messaging.inbox_prompt_mode != InboxPromptMode::Normal {
            return;
        }
        self.messaging.inbox_id_input.pop();
        let _ = self.sync_inbox_cursor_to_id_input();
        self.messaging.inbox_feedback = None;
    }

    pub fn submit_inbox_id_input(&mut self) {
        if self.messaging.inbox_prompt_mode != InboxPromptMode::Normal {
            return;
        }
        if self.messaging.inbox_id_input.trim().is_empty() {
            return;
        }
        let Ok(id) = self.messaging.inbox_id_input.trim().parse::<usize>() else {
            self.messaging.inbox_feedback =
                Some(PromptFeedback::error("Enter a valid inbox item ID."));
            return;
        };
        let Some(index) = self
            .filtered_inbox_display_items()
            .iter()
            .position(|item| item.display_id == id)
        else {
            self.messaging.inbox_feedback = Some(PromptFeedback::error(format!(
                "Enter a visible inbox item ID."
            )));
            return;
        };
        self.messaging.inbox_cursor = index;
        sync_scroll_to_cursor(
            &mut self.messaging.inbox_scroll_offset,
            self.messaging.inbox_cursor,
            INBOX_VISIBLE_ROWS,
        );
        self.messaging.inbox_preview_scroll = 0;
        self.messaging.inbox_feedback = None;
    }

    pub fn open_inbox_delete_confirm(&mut self) {
        if self.filtered_inbox_items().is_empty() {
            self.messaging.inbox_feedback =
                Some(PromptFeedback::error("No inbox item is selected."));
            return;
        }
        self.messaging.inbox_prompt_mode = InboxPromptMode::DeleteConfirm;
        self.messaging.inbox_feedback = None;
    }

    pub fn cancel_inbox_prompt(&mut self) {
        self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
        self.messaging.inbox_year_input.clear();
        self.messaging.inbox_feedback = None;
    }

    pub fn confirm_delete_inbox_item(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.messaging.inbox_prompt_mode != InboxPromptMode::DeleteConfirm {
            return Ok(());
        }
        let Some(item) = self.current_inbox_item() else {
            self.messaging.inbox_feedback =
                Some(PromptFeedback::error("No inbox item is selected."));
            self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
            return Ok(());
        };
        let deleted_id = self.current_inbox_display_id();
        match item.source {
            InboxItemSource::QueuedMail(index) => {
                if let Some(mail) = self.queued_mail.get_mut(index) {
                    mail.mark_deleted_by_recipient();
                }
            }
            InboxItemSource::ReportBlock(index) => delete_report_block_for_viewer(
                &mut self.report_block_rows,
                &self.game_data.player.records,
                index,
                self.player.record_index_1_based as u8,
            ),
        }
        self.save_game_data()?;
        self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
        self.messaging.inbox_feedback = Some(PromptFeedback::notice(format!(
            "Deleted item {deleted_id}."
        )));
        self.messaging.inbox_id_input.clear();
        self.messaging.inbox_preview_scroll = 0;
        self.normalize_inbox_selection();
        Ok(())
    }

    pub(crate) fn inbox_items_for_filters(
        &self,
        type_filter: InboxTypeFilter,
        year_filter: Option<u16>,
    ) -> Vec<InboxItem> {
        runtime_inbox_items(
            &self.game_data,
            self.player.record_index_1_based as u8,
            &self.report_block_rows,
            &self.queued_mail,
        )
        .into_iter()
        .filter(|item| match type_filter {
            InboxTypeFilter::All => true,
            InboxTypeFilter::Messages => item.item_type == InboxItemType::Message,
            InboxTypeFilter::Reports => item.item_type == InboxItemType::Report,
        })
        .filter(|item| year_filter.is_none_or(|year| item.year == year))
        .collect()
    }

    pub fn filtered_inbox_items(&self) -> Vec<InboxItem> {
        self.inbox_items_for_filters(
            self.messaging.inbox_type_filter,
            self.messaging.inbox_year_filter,
        )
    }

    pub fn filtered_inbox_display_items(&self) -> Vec<InboxDisplayItem> {
        self.filtered_inbox_items()
            .into_iter()
            .filter_map(|item| {
                self.messaging
                    .inbox_display_ids
                    .get(&item.source)
                    .copied()
                    .map(|display_id| InboxDisplayItem { display_id, item })
            })
            .collect()
    }

    pub fn current_inbox_item(&self) -> Option<InboxItem> {
        self.filtered_inbox_items()
            .get(self.messaging.inbox_cursor)
            .cloned()
    }

    pub fn current_inbox_display_id(&self) -> String {
        self.filtered_inbox_display_items()
            .get(self.messaging.inbox_cursor)
            .map(|item| format!("{:02}", item.display_id))
            .unwrap_or_else(|| "00".to_string())
    }

    fn current_inbox_preview_lines(&self) -> Vec<String> {
        let Some(item) = self.current_inbox_item() else {
            return vec!["<no matching items>".to_string()];
        };
        crate::reports::runtime_inbox_preview_lines(
            &item.body_lines,
            crate::screen::PLAYFIELD_WIDTH.saturating_sub(2),
        )
    }

    fn inbox_preview_visible_rows(&self) -> usize {
        self.inbox_preview_body_rows()
    }

    fn normalize_inbox_selection(&mut self) {
        self.ensure_inbox_display_ids_for_current_filter();
        let total = self.filtered_inbox_items().len();
        if total == 0 {
            self.messaging.inbox_cursor = 0;
            self.messaging.inbox_scroll_offset = 0;
            self.messaging.inbox_preview_scroll = 0;
            return;
        }
        self.messaging.inbox_cursor = self.messaging.inbox_cursor.min(total - 1);
        let max_offset = total.saturating_sub(INBOX_VISIBLE_ROWS);
        self.messaging.inbox_scroll_offset = self.messaging.inbox_scroll_offset.min(max_offset);
        sync_scroll_to_cursor(
            &mut self.messaging.inbox_scroll_offset,
            self.messaging.inbox_cursor,
            INBOX_VISIBLE_ROWS,
        );
        let preview_total = self.current_inbox_preview_lines().len();
        let preview_visible = self.inbox_preview_visible_rows();
        self.messaging.inbox_preview_scroll = self
            .messaging
            .inbox_preview_scroll
            .min(preview_total.saturating_sub(preview_visible));
    }

    fn sync_inbox_cursor_to_id_input(&mut self) -> bool {
        let rows = self
            .filtered_inbox_display_items()
            .iter()
            .map(|item| vec![format!("{:02}", item.display_id)])
            .collect::<Vec<_>>();
        let Some(matched) = crate::screen::table_selection::find_typed_jump(
            &rows,
            0,
            &self.messaging.inbox_id_input,
        ) else {
            return false;
        };
        self.messaging.inbox_cursor = matched.index;
        sync_scroll_to_cursor(
            &mut self.messaging.inbox_scroll_offset,
            self.messaging.inbox_cursor,
            INBOX_VISIBLE_ROWS,
        );
        self.messaging.inbox_preview_scroll = 0;
        matched.is_terminal_exact_match
    }

    fn clear_inbox_filter_feedback(&mut self) {
        self.messaging.inbox_prompt_mode = InboxPromptMode::Normal;
        self.messaging.inbox_year_input.clear();
        self.messaging.inbox_feedback = None;
    }

    fn ensure_inbox_display_ids_for_current_filter(&mut self) {
        let items = self.filtered_inbox_items();
        for item in items {
            if self.messaging.inbox_display_ids.contains_key(&item.source) {
                continue;
            }
            let next = self.messaging.inbox_next_display_id;
            self.messaging.inbox_display_ids.insert(item.source, next);
            self.messaging.inbox_next_display_id += 1;
        }
    }

    fn reset_inbox_jump_and_preview(&mut self) {
        self.messaging.inbox_id_input.clear();
        self.messaging.inbox_preview_scroll = 0;
        self.messaging.inbox_cursor = 0;
        self.messaging.inbox_scroll_offset = 0;
    }

    pub fn inbox_visible_table_rows(&self) -> usize {
        self.filtered_inbox_items().len().min(INBOX_VISIBLE_ROWS)
    }

    pub fn inbox_preview_start_row(&self) -> usize {
        1 + 4 + self.inbox_visible_table_rows()
    }

    pub fn inbox_preview_body_rows(&self) -> usize {
        command_line_row_for(self.screen_geometry)
            .saturating_sub(self.inbox_preview_start_row() + 1)
            .saturating_sub(1)
    }

    pub fn inbox_preview_scroll_offset(&self) -> usize {
        self.messaging.inbox_preview_scroll
    }
}
