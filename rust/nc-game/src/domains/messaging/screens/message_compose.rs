use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_data::QueuedPlayerMail;

use crate::app::Action;
use crate::domains::messaging::MessagingAction;
use crate::screen::layout::{
    LEFT_WINDOW_PAD_COL, ScreenGeometry, command_line_row_for, dismiss_prompt_row,
    draw_command_line_default_input_with_cancel_at_col, draw_command_line_prompt_text_at,
    draw_command_prompt_at, draw_dismiss_prompt, draw_prompt_error_after_padded, draw_title_bar,
    draw_title_bar_padded, new_playfield, new_playfield_for, standard_table_visible_rows_for,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, format_empire_id, layout_standard_table_block,
    resolve_table_columns_for_widget, write_table_window_with_cursor_at,
};
use crate::screen::{COMMAND_LABEL, PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;

pub struct MessageComposeScreen;
pub(crate) const COMPOSE_SUBJECT_LIMIT: usize = 60;
pub(crate) const COMPOSE_BODY_LIMIT: usize = 1000;
pub(crate) const COMPOSE_BODY_WRAP_WIDTH: usize = 79;
const COMPOSE_BODY_FIRST_ROW: usize = 4;
const COMPOSE_BODY_LAST_ROW: usize = 21;
const COMPOSE_BODY_STATUS_ROW: usize = 22;
const COMPOSE_BODY_CHARS_ROW: usize = 23;

const RECIPIENT_COLUMNS: [TableColumn<'static>; 2] =
    [TableColumn::right("ID", 3), TableColumn::left("Empire", 28)];

const OUTBOX_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::right("No", 3),
    TableColumn::left("To", 28),
    TableColumn::left("Subject", 34),
];

impl MessageComposeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_recipient(
        &mut self,
        frame: &ScreenFrame<'_>,
        input: &str,
        _status: Option<&str>,
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(frame.geometry);
        let rows = frame
            .game_data
            .player
            .records
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx + 1 != frame.player.record_index_1_based)
            .map(|(idx, player)| {
                let empire_id = idx + 1;
                let name = player.controlled_empire_name_summary();
                let fallback = player.legacy_status_name_summary();
                let display = if !name.is_empty() { name } else { fallback };
                vec![format_empire_id(empire_id as u8), display]
            })
            .collect::<Vec<_>>();
        let visible_rows = recipient_visible_rows(frame.geometry);
        let displayed_rows = rows.len().saturating_sub(scroll_offset).min(visible_rows);
        let scrollable = rows.len() > visible_rows;
        let footer = if rows.is_empty() {
            TableFooter::CommandBar {
                hotkeys_markup: "? D <Q>",
                default: None,
                input: "",
            }
        } else {
            let default_empire = rows
                .get(cursor)
                .and_then(|row| row.first())
                .map(String::as_str)
                .unwrap_or("");
            TableFooter::CommandBar {
                hotkeys_markup: "? D <Q>",
                default: Some(default_empire),
                input,
            }
        };
        let columns = resolve_table_columns_for_widget(
            &RECIPIENT_COLUMNS,
            &rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("COMMUNICATE (SEND MESSAGE):"),
            Some(footer),
        );
        let table_layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some("COMMUNICATE (SEND MESSAGE):"),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        let _ = table_layout.title_row;
        draw_table_title(
            &mut buffer,
            table_layout.table_row,
            table_layout.table_col,
            "COMMUNICATE (SEND MESSAGE):",
        );
        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            table_layout.table_row,
            table_layout.table_col,
            &columns,
            &rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );
        draw_table_footer(
            &mut buffer,
            frame.geometry,
            table_layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn render_subject(
        &mut self,
        recipient_label: &str,
        subject: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text_clipped(
            2,
            LEFT_WINDOW_PAD_COL,
            &format!("To: {recipient_label}"),
            classic::status_value_style(),
        );
        let command_row = 4;
        draw_command_line_default_input_with_cancel_at_col(
            &mut buffer,
            command_row,
            LEFT_WINDOW_PAD_COL,
            "COMMAND",
            "Message subject ",
            "",
            subject,
            "<ESC> -> ",
        );
        if let Some(status) = status {
            draw_prompt_error_after_padded(&mut buffer, command_row, status);
        }
        Ok(buffer)
    }

    pub fn render_body(
        &mut self,
        geometry: ScreenGeometry,
        recipient_label: &str,
        subject: &str,
        body: &str,
        cursor_row: usize,
        cursor_col: usize,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text_clipped(
            1,
            0,
            &format!("To: {recipient_label}"),
            classic::status_value_style(),
        );
        buffer.write_text_clipped(
            2,
            0,
            &format!("Subject: {subject}"),
            classic::status_value_style(),
        );
        buffer.write_text(
            3,
            0,
            "-------------------------------------------------------------------------------",
            classic::menu_style(),
        );

        let wrapped = wrap_body_segments(body, COMPOSE_BODY_WRAP_WIDTH);
        let first_body_row = COMPOSE_BODY_FIRST_ROW;
        let status_row = COMPOSE_BODY_STATUS_ROW;
        let chars_row = COMPOSE_BODY_CHARS_ROW;
        let body_last_row = if status.is_some() {
            status_row.saturating_sub(1)
        } else {
            COMPOSE_BODY_LAST_ROW
        };
        let visible = body_last_row.saturating_sub(first_body_row) + 1;
        let total_rows = wrapped.len().max(cursor_row + 1);
        let start = visible_window_start(total_rows, visible, cursor_row);
        for (idx, segment) in wrapped.iter().skip(start).take(visible).enumerate() {
            buffer.write_text_clipped(
                first_body_row + idx,
                0,
                &segment.text,
                classic::body_style(),
            );
        }
        if let Some(status) = status {
            buffer.write_text_clipped(status_row, 0, status, classic::status_value_style());
        }
        buffer.write_text_clipped(
            chars_row,
            0,
            &format!("Chars: {}/{}", body.chars().count(), COMPOSE_BODY_LIMIT),
            classic::body_style(),
        );
        draw_command_prompt_at(
            &mut buffer,
            command_line_row_for(geometry),
            COMMAND_LABEL,
            "^G ^E ^X",
        );
        let render_row = first_body_row + cursor_row.saturating_sub(start);
        buffer.set_cursor(cursor_col as u16, render_row as u16);
        Ok(buffer)
    }

    pub fn render_send_confirm(
        &mut self,
        geometry: ScreenGeometry,
        recipient_label: &str,
        subject: &str,
        body: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = self.render_body(geometry, recipient_label, subject, body, 0, 0, None)?;
        draw_command_line_prompt_text_at(
            &mut buffer,
            command_line_row_for(geometry),
            COMMAND_LABEL,
            "Send message? Y/[N] ->",
        );
        buffer.clear_cursor();
        Ok(buffer)
    }

    pub fn render_discard_confirm(
        &mut self,
        geometry: ScreenGeometry,
        recipient_label: &str,
        subject: &str,
        body: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = self.render_body(geometry, recipient_label, subject, body, 0, 0, None)?;
        draw_command_line_prompt_text_at(
            &mut buffer,
            command_line_row_for(geometry),
            COMMAND_LABEL,
            "Cancel message? Y/[N] ->",
        );
        buffer.clear_cursor();
        Ok(buffer)
    }

    pub fn render_outbox(
        &mut self,
        geometry: ScreenGeometry,
        queue: &[QueuedPlayerMail],
        input: &str,
        _status: Option<&str>,
        scroll_offset: usize,
        cursor: usize,
        game_data: &nc_data::CoreGameData,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let rows = queue
            .iter()
            .enumerate()
            .map(|(idx, mail)| {
                let recipient = compose_empire_label(game_data, mail.recipient_empire_id);
                let subject = if mail.subject.trim().is_empty() {
                    "<no subject>".to_string()
                } else {
                    mail.subject.clone()
                };
                vec![format!("{:02}", idx + 1), recipient, subject]
            })
            .collect::<Vec<_>>();
        let visible_rows = outbox_visible_rows(geometry);
        let displayed_rows = rows.len().saturating_sub(scroll_offset).min(visible_rows);
        let scrollable = rows.len() > visible_rows;
        let default_queue_no = format!("{:02}", cursor + 1);
        let footer = if rows.is_empty() {
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: None,
                input: "",
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: Some(&default_queue_no),
                input,
            }
        };
        let columns = resolve_table_columns_for_widget(
            &OUTBOX_COLUMNS,
            &rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("COMMUNICATE (SEND MESSAGE):"),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some("COMMUNICATE (SEND MESSAGE):"),
            Some(footer),
            scrollable,
            HorizontalAlign::Left,
            VerticalAlign::Top,
        );
        let _ = layout.title_row;
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "COMMUNICATE (SEND MESSAGE):",
        );
        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );
        draw_table_footer(
            &mut buffer,
            geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn render_sent(
        &mut self,
        status: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text_clipped(3, 0, status, classic::status_value_style());
        draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(3));
        Ok(buffer)
    }

    pub fn handle_recipient_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Messaging(MessagingAction::MoveComposeRecipient(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Messaging(MessagingAction::MoveComposeRecipient(1))
            }
            KeyCode::PageUp => Action::Messaging(MessagingAction::MoveComposeRecipient(-8)),
            KeyCode::PageDown => Action::Messaging(MessagingAction::MoveComposeRecipient(8)),
            KeyCode::Char('d') | KeyCode::Char('D') => {
                Action::Messaging(MessagingAction::OpenComposeOutbox)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Enter => Action::Messaging(MessagingAction::SubmitComposeRecipient),
            KeyCode::Backspace => Action::Messaging(MessagingAction::BackspaceComposeRecipient),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Messaging(MessagingAction::AppendComposeRecipientChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_subject_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Messaging(MessagingAction::OpenComposeRecipient),
            KeyCode::Enter => Action::Messaging(MessagingAction::SubmitComposeSubject),
            KeyCode::Backspace => Action::Messaging(MessagingAction::BackspaceComposeSubject),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::Messaging(MessagingAction::AppendComposeSubjectChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_body_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('e') | KeyCode::Char('E')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                Action::Messaging(MessagingAction::OpenComposeSendConfirm)
            }
            KeyCode::Char('x') | KeyCode::Char('X')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                Action::Messaging(MessagingAction::OpenComposeDiscardConfirm)
            }
            KeyCode::Left => Action::Messaging(MessagingAction::MoveComposeBodyCursorLeft),
            KeyCode::Right => Action::Messaging(MessagingAction::MoveComposeBodyCursorRight),
            KeyCode::Up => Action::Messaging(MessagingAction::MoveComposeBodyCursorUp),
            KeyCode::Down => Action::Messaging(MessagingAction::MoveComposeBodyCursorDown),
            KeyCode::Home => Action::Messaging(MessagingAction::MoveComposeBodyCursorHome),
            KeyCode::End => Action::Messaging(MessagingAction::MoveComposeBodyCursorEnd),
            KeyCode::Backspace => Action::Messaging(MessagingAction::BackspaceComposeBody),
            KeyCode::Delete => Action::Messaging(MessagingAction::DeleteComposeBodyChar),
            KeyCode::Tab => Action::Messaging(MessagingAction::InsertComposeTab),
            KeyCode::Enter => Action::Messaging(MessagingAction::InsertComposeNewline),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::Messaging(MessagingAction::AppendComposeBodyChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_discard_confirm_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                Action::Messaging(MessagingAction::ConfirmDiscardComposedMessage)
            }
            _ => Action::Messaging(MessagingAction::OpenComposeBody),
        }
    }

    pub fn handle_send_confirm_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                Action::Messaging(MessagingAction::ConfirmSendComposedMessage)
            }
            _ => Action::Messaging(MessagingAction::OpenComposeBody),
        }
    }

    pub fn handle_outbox_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Messaging(MessagingAction::MoveComposeOutbox(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Messaging(MessagingAction::MoveComposeOutbox(1))
            }
            KeyCode::PageUp => Action::Messaging(MessagingAction::MoveComposeOutbox(-8)),
            KeyCode::PageDown => Action::Messaging(MessagingAction::MoveComposeOutbox(8)),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Messaging(MessagingAction::OpenComposeRecipient)
            }
            KeyCode::Enter => Action::Messaging(MessagingAction::DeleteQueuedComposeMessage),
            KeyCode::Backspace => Action::Messaging(MessagingAction::BackspaceComposeOutboxInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Messaging(MessagingAction::AppendComposeOutboxChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_sent_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}

pub fn recipient_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 5)
}

pub fn outbox_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 4)
}

#[derive(Debug, Clone)]
pub(crate) struct WrappedSegment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ComposeCursor {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

pub(crate) fn wrap_body_segments(body: &str, width: usize) -> Vec<WrappedSegment> {
    if body.is_empty() {
        return vec![WrappedSegment {
            start: 0,
            end: 0,
            text: String::new(),
        }];
    }

    let mut out = Vec::new();
    let chars = body.chars().collect::<Vec<_>>();
    let mut line_start = 0usize;
    let mut idx = 0usize;
    while idx <= chars.len() {
        let line_end = if idx == chars.len() {
            idx
        } else if chars[idx] == '\n' {
            idx
        } else {
            idx += 1;
            continue;
        };

        if line_start == line_end {
            out.push(WrappedSegment {
                start: line_start,
                end: line_end,
                text: String::new(),
            });
        } else {
            let mut seg_start = line_start;
            while seg_start < line_end {
                let hard_end = usize::min(seg_start + width, line_end);
                let seg_end = if hard_end == line_end {
                    line_end
                } else {
                    chars[seg_start..hard_end]
                        .iter()
                        .rposition(|ch| ch.is_whitespace())
                        .map(|idx| seg_start + idx + 1)
                        .filter(|&end| end > seg_start)
                        .unwrap_or(hard_end)
                };
                out.push(WrappedSegment {
                    start: seg_start,
                    end: seg_end,
                    text: chars[seg_start..seg_end].iter().collect(),
                });
                seg_start = seg_end;
            }
        }

        if idx == chars.len() {
            break;
        }
        idx += 1;
        line_start = idx;
    }

    if out.is_empty() {
        out.push(WrappedSegment {
            start: 0,
            end: 0,
            text: String::new(),
        });
    }
    out
}

pub(crate) fn compose_cursor_for_index(body: &str, cursor_index: usize) -> ComposeCursor {
    let cursor = cursor_index.min(body.chars().count());
    let segments = wrap_body_segments(body, COMPOSE_BODY_WRAP_WIDTH);
    for (idx, segment) in segments.iter().enumerate() {
        if cursor >= segment.start && cursor <= segment.end {
            return ComposeCursor {
                row: idx,
                col: cursor.saturating_sub(segment.start),
            };
        }
    }
    ComposeCursor::default()
}

pub(crate) fn compose_row_end_col(body: &str, row: usize) -> usize {
    wrap_body_segments(body, COMPOSE_BODY_WRAP_WIDTH)
        .get(row)
        .map(|segment| segment.end.saturating_sub(segment.start))
        .unwrap_or(0)
}

pub(crate) fn compose_existing_index_for_cursor(
    body: &str,
    cursor: ComposeCursor,
) -> Option<usize> {
    let segments = wrap_body_segments(body, COMPOSE_BODY_WRAP_WIDTH);
    let segment = segments.get(cursor.row)?;
    let row_len = segment.end.saturating_sub(segment.start);
    (cursor.col <= row_len).then_some(segment.start + cursor.col)
}

pub(crate) fn materialize_compose_cursor(
    body: &mut String,
    cursor: ComposeCursor,
) -> Option<usize> {
    let required_rows = cursor.row + 1;
    let current_rows = wrap_body_segments(body, COMPOSE_BODY_WRAP_WIDTH).len();
    if required_rows > current_rows {
        let extra_rows = required_rows - current_rows;
        if body.chars().count() + extra_rows > COMPOSE_BODY_LIMIT {
            return None;
        }
        for _ in 0..extra_rows {
            body.push('\n');
        }
    }

    let segments = wrap_body_segments(body, COMPOSE_BODY_WRAP_WIDTH);
    let segment = segments.get(cursor.row)?;
    let row_len = segment.end.saturating_sub(segment.start);
    if cursor.col > row_len {
        let extra = cursor.col - row_len;
        if body.chars().count() + extra > COMPOSE_BODY_LIMIT {
            return None;
        }
        let byte_index = char_to_byte_index(body, segment.end);
        body.insert_str(byte_index, &" ".repeat(extra));
    }
    Some(segment.start + cursor.col)
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

fn visible_window_start(total: usize, visible: usize, cursor_row: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let max_start = total - visible;
    cursor_row
        .saturating_sub(visible.saturating_sub(1))
        .min(max_start)
}

fn compose_empire_label(game_data: &nc_data::CoreGameData, empire_id: u8) -> String {
    let Some(player) = game_data
        .player
        .records
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id:02}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    let display = if !name.is_empty() { name } else { fallback };
    format!("{} {}", format_empire_id(empire_id), display)
}
