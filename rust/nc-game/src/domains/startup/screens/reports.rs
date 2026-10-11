use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::domains::messaging::delete_item_prompt;
use crate::domains::messaging::state::{
    INBOX_VISIBLE_ROWS, InboxFocus, InboxPromptMode, InboxTypeFilter,
};
use crate::reports::InboxDisplayItem;
use crate::screen::layout::{
    LEFT_WINDOW_PAD_COL, PLAYFIELD_WIDTH, PromptFeedback, ScreenGeometry, command_line_row_for,
    draw_command_line_default_input_padded, draw_command_line_prompt_text_padded,
    new_playfield_for, wrap_text,
};
use crate::screen::table::{
    TableAlign, TableColumn, TableWidthMode, resolve_table_columns, table_column_start,
    table_render_width, write_table_window_with_states_at,
};
use crate::screen::{COMMAND_LABEL, CommandMenu, PlayfieldBuffer, Screen, ScreenFrame, StyledSpan};
use crate::theme::classic;

pub struct ReportsScreen;

const TABLE_START_ROW: usize = 1;
const STATUS_ROW: usize = 0;
const FEEDBACK_MAX_ROWS: usize = 3;
impl ReportsScreen {
    pub fn new() -> Self {
        Self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_inbox(
        &mut self,
        geometry: ScreenGeometry,
        _menu: CommandMenu,
        items: &[InboxDisplayItem],
        type_filter: InboxTypeFilter,
        year_filter: Option<u16>,
        cursor: usize,
        scroll_offset: usize,
        preview_scroll: usize,
        focus: InboxFocus,
        id_input: &str,
        year_input: &str,
        prompt_mode: InboxPromptMode,
        feedback: Option<&PromptFeedback>,
        current_year: u16,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        buffer.write_spans(
            STATUS_ROW,
            LEFT_WINDOW_PAD_COL,
            &[
                StyledSpan::new("Type: ", classic::status_label_style()),
                StyledSpan::new(
                    type_filter_label(type_filter),
                    classic::status_value_style(),
                ),
                StyledSpan::new(" | ", classic::status_value_style()),
                StyledSpan::new("Year: ", classic::status_label_style()),
                StyledSpan::new(
                    &year_filter
                        .map(|year| year.to_string())
                        .unwrap_or_else(|| "All".to_string()),
                    classic::status_value_style(),
                ),
                StyledSpan::new(" | ", classic::status_value_style()),
                StyledSpan::new("Focus: ", classic::status_label_style()),
                StyledSpan::new(focus_label(focus), classic::status_value_style()),
            ],
        );

        let table_rows = items
            .iter()
            .map(|item| {
                vec![
                    format!("{:02}", item.display_id),
                    item.item.item_type.code().to_string(),
                    item.item.stardate_label(),
                    item.item.subject.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let base_columns = [
            TableColumn::right("ID", 2),
            TableColumn::center("Type", 4),
            TableColumn::right("Stardate", 8),
            TableColumn::left_flex("Subject", 24, 1),
        ];
        let visible_rows = items.len().min(INBOX_VISIBLE_ROWS);
        let columns = fit_inbox_columns(&base_columns, &table_rows, items.len() > visible_rows);
        let table_width = table_render_width(&columns);
        let metrics = write_table_window_with_states_at(
            &mut buffer,
            TABLE_START_ROW,
            0,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::table_header_style(),
            classic::table_body_style(),
            None,
            0,
            None,
        );
        if focus == InboxFocus::Inbox {
            highlight_table_chrome(
                &mut buffer,
                TABLE_START_ROW,
                metrics.bottom_row,
                table_width,
            );
        }
        if let Some(selected_row) = selected_visible_row(cursor, scroll_offset, visible_rows) {
            highlight_selected_id_cell(
                &mut buffer,
                TABLE_START_ROW + 3 + selected_row,
                table_column_start(&columns, 0).unwrap_or(1),
                columns[0],
                &selected_id_label(items, cursor),
            );
        }

        let preview_top_row = metrics.bottom_row + 1;
        let command_line_row = command_line_row_for(geometry);
        let preview_bottom_row = command_line_row.saturating_sub(1);
        draw_preview_border(&mut buffer, preview_top_row, preview_bottom_row, focus);

        let preview_body_row = preview_top_row + 1;
        let preview_body_last_row = preview_bottom_row.saturating_sub(1);
        let feedback_rows = feedback
            .map(|value| preview_feedback_row_count(value, PLAYFIELD_WIDTH.saturating_sub(2)))
            .unwrap_or(0)
            .min(preview_body_last_row.saturating_sub(preview_body_row) + 1);
        let preview_body_rows = preview_body_last_row
            .saturating_sub(preview_body_row)
            .saturating_add(1)
            .saturating_sub(feedback_rows);
        let preview_lines = items
            .get(cursor)
            .map(|item| {
                crate::reports::runtime_inbox_preview_lines(
                    &item.item.body_lines,
                    PLAYFIELD_WIDTH.saturating_sub(2),
                )
            })
            .unwrap_or_else(|| vec!["<no matching items>".to_string()]);
        for (idx, line) in preview_lines
            .iter()
            .skip(preview_scroll)
            .take(preview_body_rows)
            .enumerate()
        {
            buffer.write_text(preview_body_row + idx, 1, line, classic::body_style());
        }

        if let Some(feedback) = feedback {
            let feedback_start_row = preview_body_last_row + 1 - feedback_rows;
            draw_feedback_block_in_preview(
                &mut buffer,
                feedback_start_row,
                PLAYFIELD_WIDTH.saturating_sub(2),
                feedback,
            );
        }

        match prompt_mode {
            InboxPromptMode::Normal => {
                let prompt = format!(
                    "? M R A Y D <TAB> <Q> [{}] -> ",
                    selected_id_label(items, cursor)
                );
                draw_command_line_prompt_text_padded(
                    &mut buffer,
                    command_line_row,
                    COMMAND_LABEL,
                    &prompt,
                );
                if let Some((col, row)) = buffer.cursor() {
                    buffer.write_text(
                        row as usize,
                        col as usize,
                        id_input,
                        classic::prompt_hotkey_style(),
                    );
                    buffer.set_cursor(col + id_input.chars().count() as u16, row);
                }
            }
            InboxPromptMode::YearInput => {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_line_row,
                    COMMAND_LABEL,
                    "Year ",
                    &current_year.to_string(),
                    year_input,
                );
            }
            InboxPromptMode::DeleteConfirm => {
                draw_command_line_prompt_text_padded(
                    &mut buffer,
                    command_line_row,
                    COMMAND_LABEL,
                    &delete_item_prompt(&selected_id_label(items, cursor)),
                );
            }
        }

        Ok(buffer)
    }
}

impl Screen for ReportsScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_inbox(
            ScreenGeometry::local_default(),
            CommandMenu::General,
            &[],
            InboxTypeFilter::All,
            None,
            0,
            0,
            0,
            InboxFocus::Inbox,
            "",
            "",
            InboxPromptMode::Normal,
            None,
            0,
        )
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }
}

fn fit_inbox_columns(
    columns: &[TableColumn<'static>],
    rows: &[Vec<String>],
    scrollbar_visible: bool,
) -> Vec<TableColumn<'static>> {
    resolve_table_columns(
        columns,
        rows,
        PLAYFIELD_WIDTH,
        scrollbar_visible,
        TableWidthMode::Expand,
    )
}

fn highlight_selected_id_cell(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    column: TableColumn<'_>,
    value: &str,
) {
    let text = match column.align {
        TableAlign::Left => format!("{value:<width$}", width = column.width),
        TableAlign::Center => {
            let width = column.width;
            let text_width = value.chars().count();
            let pad = width.saturating_sub(text_width);
            let left = pad / 2;
            let right = pad.saturating_sub(left);
            format!("{}{}{}", " ".repeat(left), value, " ".repeat(right))
        }
        TableAlign::Right => format!("{value:>width$}", width = column.width),
    };
    buffer.write_text(row, col, &text, classic::selected_row_style());
}

fn selected_visible_row(cursor: usize, scroll_offset: usize, visible_rows: usize) -> Option<usize> {
    if cursor < scroll_offset || cursor >= scroll_offset + visible_rows {
        None
    } else {
        Some(cursor - scroll_offset)
    }
}

fn type_filter_label(filter: InboxTypeFilter) -> &'static str {
    match filter {
        InboxTypeFilter::All => "All",
        InboxTypeFilter::Messages => "Messages",
        InboxTypeFilter::Reports => "Reports",
    }
}

fn focus_label(focus: InboxFocus) -> &'static str {
    match focus {
        InboxFocus::Inbox => "Inbox",
        InboxFocus::Preview => "Preview",
    }
}

fn selected_id_label(items: &[InboxDisplayItem], cursor: usize) -> String {
    items
        .get(cursor)
        .map(|item| format!("{:02}", item.display_id))
        .unwrap_or_else(|| "00".to_string())
}

pub(crate) fn draw_preview_border(
    buffer: &mut PlayfieldBuffer,
    top_row: usize,
    bottom_row: usize,
    focus: InboxFocus,
) {
    let border_style = if focus == InboxFocus::Preview {
        classic::notice_style()
    } else {
        classic::table_chrome_style()
    };
    let top = format!("┌{}┐", "─".repeat(PLAYFIELD_WIDTH.saturating_sub(2)));
    let bottom = format!("└{}┘", "─".repeat(PLAYFIELD_WIDTH.saturating_sub(2)));
    buffer.write_text(top_row, 0, &top, border_style);
    for row in top_row + 1..bottom_row {
        buffer.write_text(row, 0, "│", border_style);
        buffer.write_text(row, PLAYFIELD_WIDTH.saturating_sub(1), "│", border_style);
    }
    buffer.write_text(bottom_row, 0, &bottom, border_style);
}

fn highlight_table_chrome(
    buffer: &mut PlayfieldBuffer,
    top_row: usize,
    bottom_row: usize,
    table_width: usize,
) {
    let border_style = classic::notice_style();
    for row in top_row..=bottom_row {
        for col in 0..table_width {
            let ch = buffer.row(row)[col].ch;
            if is_table_chrome_char(ch) {
                buffer.write_text(row, col, &ch.to_string(), border_style);
            }
        }
    }
}

fn is_table_chrome_char(ch: char) -> bool {
    matches!(
        ch,
        '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' | '─' | '│'
    )
}

fn truncate_to_width(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out = text.chars().take(max_chars).collect::<String>();
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

fn preview_feedback_row_count(feedback: &PromptFeedback, width: usize) -> usize {
    let (label, value) = match feedback {
        PromptFeedback::Notice(value) => ("Notice: ", value.as_str()),
        PromptFeedback::Error(value) => ("Error: ", value.as_str()),
        PromptFeedback::Warning(value) => ("Warning: ", value.as_str()),
    };
    let label_width = label.chars().count();
    wrap_text(
        value,
        width.saturating_sub(label_width).max(1),
        width.saturating_sub(label_width).max(1),
    )
    .len()
    .min(FEEDBACK_MAX_ROWS)
}

fn draw_feedback_block_in_preview(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    width: usize,
    feedback: &PromptFeedback,
) {
    let (label, label_style, value) = match feedback {
        PromptFeedback::Notice(value) => ("Notice: ", classic::notice_style(), value.as_str()),
        PromptFeedback::Error(value) => ("Error: ", classic::error_style(), value.as_str()),
        PromptFeedback::Warning(value) => ("Warning: ", classic::error_style(), value.as_str()),
    };
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let wrapped = wrap_text(
        value,
        width.saturating_sub(label_width).max(1),
        width.saturating_sub(label_width).max(1),
    );
    for (idx, line) in wrapped.into_iter().take(FEEDBACK_MAX_ROWS).enumerate() {
        let current_label = if idx == 0 { label } else { &continuation };
        buffer.write_spans(
            start_row + idx,
            1,
            &[
                StyledSpan::new(current_label, label_style),
                StyledSpan::new(
                    &truncate_to_width(&line, width.saturating_sub(label_width)),
                    classic::status_value_style(),
                ),
            ],
        );
    }
}
