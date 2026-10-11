use crate::app::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::domains::messaging::screens::message_compose::{
    COMPOSE_BODY_WRAP_WIDTH, ComposeCursor, compose_cursor_for_index,
    compose_existing_index_for_cursor, compose_row_end_col, materialize_compose_cursor,
};
use crate::domains::messaging::state::OutboxMode;
use crate::model::{MainMenuSummary, ReviewSummary};
use crate::reports::has_visible_runtime_messages;
use crate::screen::{CommandMenu, ScreenId};
use nc_data::{
    CoreGameData, QueuedPlayerMail, delete_report_blocks_for_viewer, validate_queue_message_limit,
};

const COMPOSE_TAB_WIDTH: usize = 4;

impl App {
    fn compose_recipient_visible_rows(&self) -> usize {
        crate::domains::messaging::screens::message_compose::recipient_visible_rows(
            self.screen_geometry,
        )
    }

    fn compose_outbox_visible_rows(&self) -> usize {
        crate::domains::messaging::screens::message_compose::outbox_visible_rows(
            self.screen_geometry,
        )
    }

    pub fn open_delete_reviewables(&mut self) {
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            self.has_active_report_blocks(),
            has_visible_runtime_messages(self.player.record_index_1_based as u8, &self.queued_mail),
        );
        let review_summary = ReviewSummary::from_main_menu(&summary);
        if !review_summary.reviewable_results && !review_summary.reviewable_messages {
            self.show_command_menu_notice(
                CommandMenu::General,
                "No messages or results are currently reviewable.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.messaging.delete_reviewables_prompt_active = true;
        self.planet.info_prompt_active = false;
        self.current_screen = ScreenId::GeneralMenu;
    }

    pub fn close_delete_reviewables_prompt(&mut self) {
        self.messaging.delete_reviewables_prompt_active = false;
    }

    pub fn open_compose_message_recipient(&mut self) {
        self.messaging.delete_reviewables_prompt_active = false;
        self.messaging.compose_recipient_input.clear();
        self.messaging.compose_recipient_status = None;
        self.messaging.compose_recipient_scroll_offset = 0;
        self.messaging.compose_recipient_cursor = 0;
        self.messaging.compose_recipient_empire = None;
        self.messaging.compose_subject.clear();
        self.messaging.compose_subject_status = None;
        self.messaging.compose_body.clear();
        self.messaging.compose_body_cursor_row = 0;
        self.messaging.compose_body_cursor_col = 0;
        self.messaging.compose_body_status = None;
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;
        self.messaging.compose_outbox_scroll_offset = 0;
        self.messaging.compose_sent_status = None;
        self.current_screen = ScreenId::ComposeMessageRecipient;
    }

    pub fn open_compose_message_subject(&mut self) {
        if self.messaging.compose_recipient_empire.is_none() {
            self.open_compose_message_recipient();
            return;
        }
        self.messaging.compose_subject_status = None;
        self.current_screen = ScreenId::ComposeMessageSubject;
    }

    pub fn open_compose_message_body(&mut self) {
        if self.messaging.compose_recipient_empire.is_none() {
            self.open_compose_message_recipient();
            return;
        }
        self.messaging.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageBody;
    }

    pub fn open_compose_message_outbox(&mut self) {
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;
        self.messaging.compose_outbox_scroll_offset = 0;
        self.messaging.compose_outbox_cursor = 0;
        self.messaging.compose_outbox_mode = OutboxMode::List;
        self.current_screen = ScreenId::ComposeMessageOutbox;
    }

    pub fn open_compose_message_discard_confirm(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.current_screen = ScreenId::ComposeMessageDiscardConfirm;
        }
    }

    pub fn open_compose_message_send_confirm(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            let body = self.messaging.compose_body.trim();
            if body.is_empty() {
                self.messaging.compose_body_status =
                    Some("Message body cannot be empty.".to_string());
                return;
            }
            self.current_screen = ScreenId::ComposeMessageSendConfirm;
        }
    }

    pub fn append_compose_recipient_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageRecipient
            && self.messaging.compose_recipient_input.len() < 2
        {
            self.messaging.compose_recipient_input.push(ch);
            self.sync_compose_recipient_cursor_to_input();
            self.messaging.compose_recipient_status = None;
        }
    }

    pub fn scroll_compose_recipients(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageRecipient {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        let max_offset = total.saturating_sub(self.compose_recipient_visible_rows());
        self.messaging.compose_recipient_scroll_offset = self
            .messaging
            .compose_recipient_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_compose_recipient_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageRecipient {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        if total == 0 {
            return;
        }
        let next = self.messaging.compose_recipient_cursor as isize + delta as isize;
        self.messaging.compose_recipient_cursor = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.compose_recipient_visible_rows();
        sync_scroll_to_cursor(
            &mut self.messaging.compose_recipient_scroll_offset,
            self.messaging.compose_recipient_cursor,
            visible_rows,
        );
    }

    pub fn backspace_compose_recipient(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageRecipient {
            self.messaging.compose_recipient_input.pop();
            self.sync_compose_recipient_cursor_to_input();
            self.messaging.compose_recipient_status = None;
        }
    }

    pub fn submit_compose_recipient(&mut self) {
        // If the input box is empty, derive the empire id from the cursor row.
        let empire_id = if self.messaging.compose_recipient_input.trim().is_empty() {
            let ids: Vec<u8> = self
                .game_data
                .player
                .records
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
                .map(|(idx, _)| (idx + 1) as u8)
                .collect();
            match ids.get(self.messaging.compose_recipient_cursor) {
                Some(&id) => id,
                None => {
                    self.messaging.compose_recipient_status =
                        Some("No empire selected.".to_string());
                    return;
                }
            }
        } else {
            match self.messaging.compose_recipient_input.parse::<u8>() {
                Ok(id) => id,
                Err(_) => {
                    self.messaging.compose_recipient_status =
                        Some("Enter an empire number.".to_string());
                    return;
                }
            }
        };
        let max_empire = self.game_data.conquest.player_count();
        if !(1..=max_empire).contains(&empire_id) {
            self.messaging.compose_recipient_status =
                Some(format!("Enter an empire number in 1..={max_empire}."));
            return;
        }
        if empire_id as usize == self.player.record_index_1_based {
            self.messaging.compose_recipient_status =
                Some("You cannot message your own empire.".to_string());
            return;
        }
        self.messaging.compose_recipient_empire = Some(empire_id);
        self.messaging.compose_subject.clear();
        self.messaging.compose_subject_status = None;
        self.messaging.compose_body.clear();
        self.messaging.compose_body_cursor_row = 0;
        self.messaging.compose_body_cursor_col = 0;
        self.messaging.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageSubject;
    }

    pub fn append_compose_subject_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageSubject
            && self.messaging.compose_subject.chars().count() < crate::screen::COMPOSE_SUBJECT_LIMIT
        {
            self.messaging.compose_subject.push(ch);
            self.messaging.compose_subject_status = None;
        }
    }

    pub fn backspace_compose_subject(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageSubject {
            self.messaging.compose_subject.pop();
            self.messaging.compose_subject_status = None;
        }
    }

    pub fn submit_compose_subject(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageSubject {
            return;
        }
        self.sync_compose_body_cursor_to_end();
        self.messaging.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageBody;
    }

    pub fn confirm_discard_composed_message(&mut self) {
        self.open_compose_message_recipient();
    }

    pub fn append_compose_body_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.insert_text_at_compose_cursor(&ch.to_string());
    }

    pub fn insert_compose_tab(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.insert_text_at_compose_cursor(&" ".repeat(COMPOSE_TAB_WIDTH));
    }

    pub fn backspace_compose_body(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        let cursor = self.compose_cursor();
        let Some(cursor_index) =
            compose_existing_index_for_cursor(&self.messaging.compose_body, cursor)
        else {
            self.move_compose_body_cursor_left();
            return;
        };
        if cursor_index == 0 {
            self.messaging.compose_body_status = None;
            return;
        }
        remove_char_before(&mut self.messaging.compose_body, cursor_index);
        self.set_compose_cursor_from_index(cursor_index - 1);
        self.messaging.compose_body_status = None;
    }

    pub fn delete_compose_body_char(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        let cursor = self.compose_cursor();
        let Some(cursor_index) =
            compose_existing_index_for_cursor(&self.messaging.compose_body, cursor)
        else {
            self.messaging.compose_body_status = None;
            return;
        };
        remove_char_at(&mut self.messaging.compose_body, cursor_index);
        self.set_compose_cursor_from_index(cursor_index);
        self.messaging.compose_body_status = None;
    }

    pub fn insert_compose_newline(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.insert_text_at_compose_cursor("\n");
    }

    pub fn move_compose_body_cursor_left(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        if self.messaging.compose_body_cursor_col > 0 {
            self.messaging.compose_body_cursor_col -= 1;
        } else if self.messaging.compose_body_cursor_row > 0 {
            self.messaging.compose_body_cursor_row -= 1;
            self.messaging.compose_body_cursor_col = compose_row_end_col(
                &self.messaging.compose_body,
                self.messaging.compose_body_cursor_row,
            );
        }
        self.messaging.compose_body_status = None;
    }

    pub fn move_compose_body_cursor_right(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        if self.messaging.compose_body_cursor_col < COMPOSE_BODY_WRAP_WIDTH {
            self.messaging.compose_body_cursor_col += 1;
        } else {
            self.messaging.compose_body_cursor_row =
                self.messaging.compose_body_cursor_row.saturating_add(1);
            self.messaging.compose_body_cursor_col = 0;
        }
        self.messaging.compose_body_status = None;
    }

    pub fn move_compose_body_cursor_home(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.messaging.compose_body_cursor_col = 0;
        self.messaging.compose_body_status = None;
    }

    pub fn move_compose_body_cursor_end(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.messaging.compose_body_cursor_col = compose_row_end_col(
            &self.messaging.compose_body,
            self.messaging.compose_body_cursor_row,
        );
        self.messaging.compose_body_status = None;
    }

    pub fn move_compose_body_cursor_up(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.messaging.compose_body_cursor_row =
            self.messaging.compose_body_cursor_row.saturating_sub(1);
        self.messaging.compose_body_status = None;
    }

    pub fn move_compose_body_cursor_down(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageBody {
            return;
        }
        self.messaging.compose_body_cursor_row =
            self.messaging.compose_body_cursor_row.saturating_add(1);
        self.messaging.compose_body_status = None;
    }

    pub fn send_composed_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::ComposeMessageSendConfirm {
            return Ok(());
        }
        let Some(recipient_empire_id) = self.messaging.compose_recipient_empire else {
            self.messaging.compose_body_status = Some("Choose a recipient first.".to_string());
            return Ok(());
        };
        let body = trim_compose_body(&self.messaging.compose_body);
        if body.trim().is_empty() {
            self.messaging.compose_body_status = Some("Message body cannot be empty.".to_string());
            return Ok(());
        }
        if let Err(err) = validate_queue_message_limit(
            &self.queued_mail,
            self.player.record_index_1_based as u8,
            recipient_empire_id,
            self.game_data.conquest.game_year(),
        ) {
            self.messaging.compose_body_status = Some(err);
            self.current_screen = ScreenId::ComposeMessageBody;
            return Ok(());
        }
        self.queued_mail.push(QueuedPlayerMail {
            sender_empire_id: self.player.record_index_1_based as u8,
            recipient_empire_id,
            year: self.game_data.conquest.game_year(),
            subject: self.messaging.compose_subject.trim().to_string(),
            body,
            recipient_deleted: false,
        });
        self.save_game_data()?;
        self.messaging.compose_sent_status = Some(format!(
            "Message queued for Empire {recipient_empire_id}. It will be delivered after turn maintenance."
        ));
        self.current_screen = ScreenId::ComposeMessageSent;
        Ok(())
    }

    pub fn scroll_compose_outbox(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return;
        }
        let total = self.compose_outbox_queue_len();
        let max_offset = total.saturating_sub(self.compose_outbox_visible_rows());
        self.messaging.compose_outbox_scroll_offset = self
            .messaging
            .compose_outbox_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_compose_outbox_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return;
        }
        let total = self.compose_outbox_queue_len();
        if total == 0 {
            return;
        }
        let next = self.messaging.compose_outbox_cursor as isize + delta as isize;
        self.messaging.compose_outbox_cursor = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.compose_outbox_visible_rows();
        sync_scroll_to_cursor(
            &mut self.messaging.compose_outbox_scroll_offset,
            self.messaging.compose_outbox_cursor,
            visible_rows,
        );
    }

    pub fn append_compose_outbox_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageOutbox
            && self.messaging.compose_outbox_input.len() < 2
        {
            self.messaging.compose_outbox_input.push(ch);
            self.sync_compose_outbox_cursor_to_input();
            self.messaging.compose_outbox_status = None;
        }
    }

    pub fn backspace_compose_outbox_input(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageOutbox {
            self.messaging.compose_outbox_input.pop();
            self.sync_compose_outbox_cursor_to_input();
            self.messaging.compose_outbox_status = None;
        }
    }

    pub fn open_compose_outbox_message(&mut self) {
        if self.select_compose_outbox_message().is_some() {
            self.messaging.compose_outbox_mode = OutboxMode::Read;
        }
    }

    pub fn open_compose_outbox_delete_confirm(&mut self) {
        if self.select_compose_outbox_message().is_some() {
            self.messaging.compose_outbox_mode = OutboxMode::DeleteConfirm;
        }
    }

    pub fn return_to_compose_outbox_list(&mut self) {
        self.messaging.compose_outbox_mode = OutboxMode::List;
    }

    pub fn delete_queued_compose_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::ComposeMessageOutbox
            || self.messaging.compose_outbox_mode != OutboxMode::DeleteConfirm
        {
            return Ok(());
        }
        self.messaging.compose_outbox_mode = OutboxMode::List;
        let sender_empire_id = self.player.record_index_1_based as u8;
        let own_indexes = self
            .queued_mail
            .iter()
            .enumerate()
            .filter_map(|(idx, mail)| (mail.sender_empire_id == sender_empire_id).then_some(idx))
            .collect::<Vec<_>>();
        let Some(queue_index) = own_indexes
            .get(self.messaging.compose_outbox_cursor)
            .copied()
        else {
            self.messaging.compose_outbox_status = Some(format!(
                "Enter a queued message number in 1..={}.",
                own_indexes.len()
            ));
            return Ok(());
        };

        self.cancel_queued_mail(queue_index);
        self.save_game_data()?;
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;

        // Clamp cursor and scroll offset to the new (smaller) queue.
        let new_len = own_indexes.len().saturating_sub(1);
        self.messaging.compose_outbox_cursor = self
            .messaging
            .compose_outbox_cursor
            .min(new_len.saturating_sub(1));
        let max_offset = new_len.saturating_sub(self.compose_outbox_visible_rows());
        self.messaging.compose_outbox_scroll_offset =
            self.messaging.compose_outbox_scroll_offset.min(max_offset);
        Ok(())
    }

    /// Removes the player's own queued message outright, as the outbox and the
    /// inbox both do when it is cancelled before maintenance. The caller saves.
    pub(crate) fn cancel_queued_mail(&mut self, queue_index: usize) {
        self.queued_mail.remove(queue_index);
        self.shift_inbox_display_ids_after_mail_removal(queue_index);
    }

    /// Resolves the typed queue number, or the highlighted row when nothing is
    /// typed, to a position in the player's own queue and moves the cursor
    /// there.
    fn select_compose_outbox_message(&mut self) -> Option<usize> {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return None;
        }
        let position = if self.messaging.compose_outbox_input.trim().is_empty() {
            self.messaging.compose_outbox_cursor
        } else {
            match self.messaging.compose_outbox_input.parse::<usize>() {
                Ok(queue_no) if queue_no > 0 => queue_no - 1,
                _ => {
                    self.messaging.compose_outbox_status =
                        Some("Enter a queued message number.".to_string());
                    return None;
                }
            }
        };
        let total = self.compose_outbox_queue_len();
        if position >= total {
            self.messaging.compose_outbox_status =
                Some(format!("Enter a queued message number in 1..={total}."));
            return None;
        }
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;
        self.messaging.compose_outbox_cursor = position;
        let visible_rows = self.compose_outbox_visible_rows();
        sync_scroll_to_cursor(
            &mut self.messaging.compose_outbox_scroll_offset,
            self.messaging.compose_outbox_cursor,
            visible_rows,
        );
        Some(position)
    }

    fn sync_compose_recipient_cursor_to_input(&mut self) {
        let ids = self
            .game_data
            .player
            .records
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
            .map(|(idx, _)| (idx + 1) as u8)
            .collect::<Vec<_>>();
        let rows = ids
            .iter()
            .map(|id| vec![format!("{id:02}")])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &rows,
            0,
            &self.messaging.compose_recipient_input,
        ) else {
            return;
        };
        self.messaging.compose_recipient_cursor = index;
        let visible_rows = self.compose_recipient_visible_rows();
        sync_scroll_to_cursor(
            &mut self.messaging.compose_recipient_scroll_offset,
            self.messaging.compose_recipient_cursor,
            visible_rows,
        );
    }

    fn sync_compose_outbox_cursor_to_input(&mut self) {
        let rows = (1..=self.compose_outbox_queue_len())
            .map(|idx| vec![format!("{idx:02}")])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &rows,
            0,
            &self.messaging.compose_outbox_input,
        ) else {
            return;
        };
        self.messaging.compose_outbox_cursor = index;
        let visible_rows = self.compose_outbox_visible_rows();
        sync_scroll_to_cursor(
            &mut self.messaging.compose_outbox_scroll_offset,
            self.messaging.compose_outbox_cursor,
            visible_rows,
        );
    }

    fn compose_cursor(&self) -> ComposeCursor {
        ComposeCursor {
            row: self.messaging.compose_body_cursor_row,
            col: self.messaging.compose_body_cursor_col,
        }
    }

    fn set_compose_cursor_from_index(&mut self, cursor_index: usize) {
        let cursor = compose_cursor_for_index(&self.messaging.compose_body, cursor_index);
        self.messaging.compose_body_cursor_row = cursor.row;
        self.messaging.compose_body_cursor_col = cursor.col;
    }

    fn sync_compose_body_cursor_to_end(&mut self) {
        self.set_compose_cursor_from_index(self.messaging.compose_body.chars().count());
    }

    fn insert_text_at_compose_cursor(&mut self, text: &str) {
        let insert_len = text.chars().count();
        if self.messaging.compose_body.chars().count() + insert_len
            > crate::screen::COMPOSE_BODY_LIMIT
        {
            self.messaging.compose_body_status = Some(format!(
                "Message length limit is {} characters.",
                crate::screen::COMPOSE_BODY_LIMIT
            ));
            return;
        }

        let cursor = self.compose_cursor();
        let Some(insert_index) =
            materialize_compose_cursor(&mut self.messaging.compose_body, cursor)
        else {
            self.messaging.compose_body_status = Some(format!(
                "Message length limit is {} characters.",
                crate::screen::COMPOSE_BODY_LIMIT
            ));
            return;
        };

        insert_str_at(&mut self.messaging.compose_body, insert_index, text);
        self.set_compose_cursor_from_index(insert_index + insert_len);
        self.messaging.compose_body_status = None;
    }

    pub fn delete_reviewables(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        delete_report_blocks_for_viewer(
            &mut self.report_block_rows,
            &self.game_data.player.records,
            self.player.record_index_1_based as u8,
        );
        for mail in &mut self.queued_mail {
            if mail.is_visible_to_recipient(self.player.record_index_1_based as u8) {
                mail.mark_deleted_by_recipient();
            }
        }
        if let Some(player) = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
        {
            player.set_classic_login_reviewables_present(false);
            player.set_classic_results_chain(None);
        }
        self.save_game_data()?;
        self.messaging.delete_reviewables_prompt_active = false;
        self.show_command_menu_notice(CommandMenu::General, "Messages and results deleted.");
        Ok(())
    }

    pub(crate) fn inline_delete_reviewables_active_on_current_screen(&self) -> bool {
        self.messaging.delete_reviewables_prompt_active
            && self.current_screen == ScreenId::GeneralMenu
    }

    pub(crate) fn handle_delete_reviewables_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => crate::app::Action::Messaging(
                crate::domains::messaging::MessagingAction::ConfirmDeleteReviewables,
            ),
            KeyCode::Enter
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Esc => crate::app::Action::Messaging(
                crate::domains::messaging::MessagingAction::CloseDeleteReviewables,
            ),
            _ => crate::app::Action::Noop,
        }
    }
    fn compose_outbox_queue_len(&self) -> usize {
        self.compose_outbox_queue()
            .map(|queue| queue.len())
            .unwrap_or(0)
    }

    pub(crate) fn compose_outbox_queue(
        &self,
    ) -> Result<Vec<QueuedPlayerMail>, Box<dyn std::error::Error>> {
        let sender_empire_id = self.player.record_index_1_based as u8;
        Ok(self
            .queued_mail
            .clone()
            .into_iter()
            .filter(|mail| mail.sender_empire_id == sender_empire_id)
            .collect())
    }
}

pub(crate) fn compose_recipient_label(game_data: &CoreGameData, empire_id: Option<u8>) -> String {
    let Some(empire_id) = empire_id else {
        return "<unknown>".to_string();
    };
    let Some(player) = game_data
        .player
        .records
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    let display = if !name.is_empty() { name } else { fallback };
    format!("Empire {empire_id} ({display})")
}

fn char_to_byte_index(body: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    body.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(body.len())
}

fn insert_str_at(body: &mut String, cursor_index: usize, text: &str) {
    let byte_index = char_to_byte_index(body, cursor_index);
    body.insert_str(byte_index, text);
}

fn remove_char_before(body: &mut String, cursor_index: usize) {
    if cursor_index == 0 {
        return;
    }
    let start = char_to_byte_index(body, cursor_index - 1);
    let end = char_to_byte_index(body, cursor_index);
    body.replace_range(start..end, "");
}

fn remove_char_at(body: &mut String, cursor_index: usize) {
    let char_count = body.chars().count();
    if cursor_index >= char_count {
        return;
    }
    let start = char_to_byte_index(body, cursor_index);
    let end = char_to_byte_index(body, cursor_index + 1);
    body.replace_range(start..end, "");
}

fn trim_compose_body(body: &str) -> String {
    body.trim_end_matches([' ', '\n', '\r', '\t']).to_string()
}
