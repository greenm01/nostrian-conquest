use crate::screen::format_sector_coords_default;
use crate::screen::{PlayfieldBuffer, StyledSpan};
use crate::theme::classic;
use nc_ui::prompt as shared_prompt;

pub const PLAYFIELD_WIDTH: usize = 80;
pub const PLAYFIELD_HEIGHT: usize = 25;
pub const COMMAND_LINE_ROW: usize = PLAYFIELD_HEIGHT - 1;
pub const DOOR_FALLBACK_HEIGHT: usize = 24;
pub const EXPERT_MENU_PROMPT_ROW: usize = 0;
pub const CMD_COL_1: usize = 2;
pub const CMD_COL_2: usize = 26;
pub const PRIMARY_MENU_ROW: usize = 1;
pub const PRIMARY_MENU_TITLE_COL: usize = 1;
pub const LEFT_WINDOW_PAD_COL: usize = 1;

// ScreenGeometry is now defined in nc-ui. Re-export for backward compatibility.
// local_default() and for_door() are also defined in nc-ui's ScreenGeometry impl.
pub use nc_ui::layout::ScreenGeometry;

pub use nc_ui::layout::{command_line_row_for, last_body_row_for};

pub const fn last_body_row() -> usize {
    last_body_row_for(ScreenGeometry::local_default())
}

pub const fn menu_prompt_row_for(geometry: ScreenGeometry, last_content_row: usize) -> usize {
    let desired = last_content_row + 2;
    let command_line_row = command_line_row_for(geometry);
    if desired > command_line_row {
        command_line_row
    } else {
        desired
    }
}

pub const fn menu_prompt_row(last_content_row: usize) -> usize {
    menu_prompt_row_for(ScreenGeometry::local_default(), last_content_row)
}

pub const fn dismiss_prompt_row_for(geometry: ScreenGeometry, last_content_row: usize) -> usize {
    let desired = last_content_row + 2;
    let command_line_row = command_line_row_for(geometry);
    if desired > command_line_row {
        command_line_row
    } else {
        desired
    }
}

pub const fn dismiss_prompt_row(last_content_row: usize) -> usize {
    dismiss_prompt_row_for(ScreenGeometry::local_default(), last_content_row)
}

pub const fn menu_notice_row_for(geometry: ScreenGeometry, command_row: usize) -> usize {
    let desired = command_row + 4;
    let last_body_row = last_body_row_for(geometry);
    if desired > last_body_row {
        last_body_row
    } else {
        desired
    }
}

pub const fn menu_notice_row(command_row: usize) -> usize {
    menu_notice_row_for(ScreenGeometry::local_default(), command_row)
}

pub const fn menu_general_message_row_for(geometry: ScreenGeometry, command_row: usize) -> usize {
    let desired = command_row + 2;
    let last_body_row = last_body_row_for(geometry);
    if desired > last_body_row {
        last_body_row
    } else {
        desired
    }
}

pub const fn menu_general_message_row(command_row: usize) -> usize {
    menu_general_message_row_for(ScreenGeometry::local_default(), command_row)
}

pub const fn table_prompt_row_for(geometry: ScreenGeometry, table_bottom_row: usize) -> usize {
    let desired = table_bottom_row + 1;
    let command_line_row = command_line_row_for(geometry);
    if desired > command_line_row {
        command_line_row
    } else {
        desired
    }
}

pub const fn table_prompt_row(table_bottom_row: usize) -> usize {
    table_prompt_row_for(ScreenGeometry::local_default(), table_bottom_row)
}

pub const fn table_dismiss_prompt_row_for(
    geometry: ScreenGeometry,
    table_bottom_row: usize,
) -> usize {
    let desired = table_bottom_row + 1;
    let command_line_row = command_line_row_for(geometry);
    if desired > command_line_row {
        command_line_row
    } else {
        desired
    }
}

pub const fn table_dismiss_prompt_row(table_bottom_row: usize) -> usize {
    table_dismiss_prompt_row_for(ScreenGeometry::local_default(), table_bottom_row)
}

pub const fn centered_row(first_row: usize, last_row: usize, block_height: usize) -> usize {
    let available_rows = last_row.saturating_sub(first_row) + 1;
    first_row + available_rows.saturating_sub(block_height) / 2
}

pub const fn standard_table_visible_rows_for(geometry: ScreenGeometry, start_row: usize) -> usize {
    command_line_row_for(geometry).saturating_sub(start_row + 4)
}

pub const fn standard_table_visible_rows(start_row: usize) -> usize {
    standard_table_visible_rows_for(ScreenGeometry::local_default(), start_row)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandMessage<'a> {
    General { label: &'a str, value: &'a str },
    Notice(&'a str),
    Error(&'a str),
    Warning(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptFeedback {
    Notice(String),
    Error(String),
    Warning(String),
}

impl PromptFeedback {
    pub fn notice(value: impl Into<String>) -> Self {
        Self::Notice(value.into())
    }

    pub fn error(value: impl Into<String>) -> Self {
        Self::Error(value.into())
    }

    pub fn warning(value: impl Into<String>) -> Self {
        Self::Warning(value.into())
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Notice(value) | Self::Error(value) | Self::Warning(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuEntry<'a> {
    pub col: usize,
    pub hotkey: &'a str,
    pub label: &'a str,
    pub featured: bool,
}

impl<'a> MenuEntry<'a> {
    pub const fn new(col: usize, hotkey: &'a str, label: &'a str) -> Self {
        Self {
            col,
            hotkey,
            label,
            featured: false,
        }
    }

    pub const fn featured(col: usize, hotkey: &'a str, label: &'a str) -> Self {
        Self {
            col,
            hotkey,
            label,
            featured: true,
        }
    }
}

pub const fn stacked_table_visible_rows_for(geometry: ScreenGeometry, start_row: usize) -> usize {
    command_line_row_for(geometry).saturating_sub(start_row + 5)
}

pub const fn stacked_table_visible_rows(start_row: usize) -> usize {
    stacked_table_visible_rows_for(ScreenGeometry::local_default(), start_row)
}

pub fn new_playfield_for(geometry: ScreenGeometry) -> PlayfieldBuffer {
    PlayfieldBuffer::new(geometry.width(), geometry.height(), classic::body_style())
}

pub fn new_playfield() -> PlayfieldBuffer {
    new_playfield_for(ScreenGeometry::local_default())
}

pub fn draw_title_bar(buffer: &mut PlayfieldBuffer, row: usize, title: &str) {
    draw_title_bar_at_col(buffer, row, 0, title);
}

pub fn draw_title_bar_padded(buffer: &mut PlayfieldBuffer, row: usize, title: &str) {
    draw_title_bar_at_col(buffer, row, LEFT_WINDOW_PAD_COL, title);
}

pub fn draw_title_bar_at_col(buffer: &mut PlayfieldBuffer, row: usize, col: usize, title: &str) {
    buffer.fill_row(row, classic::menu_style());
    buffer.write_text(row, col, title, classic::title_style());
}

pub fn draw_menu_row(buffer: &mut PlayfieldBuffer, row: usize, entries: &[MenuEntry<'_>]) {
    buffer.fill_row(row, classic::menu_style());
    for entry in entries {
        draw_menu_entry_item(buffer, row, *entry);
    }
}

pub fn draw_command_center(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    top_row_entries: &[MenuEntry<'_>],
    rows: &[&[MenuEntry<'_>]],
    prompt_label: &str,
    prompt_keys: &str,
) {
    draw_title_bar_at_col(buffer, PRIMARY_MENU_ROW, PRIMARY_MENU_TITLE_COL, title);
    for entry in top_row_entries {
        draw_menu_entry_item(buffer, PRIMARY_MENU_ROW, *entry);
    }
    for (idx, row_entries) in rows.iter().enumerate() {
        draw_menu_row(buffer, PRIMARY_MENU_ROW + idx + 1, row_entries);
    }
    draw_command_prompt_at(
        buffer,
        menu_prompt_row(PRIMARY_MENU_ROW + rows.len()),
        prompt_label,
        prompt_keys,
    );
}

pub fn draw_command_center_padded(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    top_row_entries: &[MenuEntry<'_>],
    rows: &[&[MenuEntry<'_>]],
    prompt_label: &str,
    prompt_keys: &str,
) {
    draw_title_bar_at_col(buffer, PRIMARY_MENU_ROW, PRIMARY_MENU_TITLE_COL, title);
    for entry in top_row_entries {
        draw_menu_entry_item(buffer, PRIMARY_MENU_ROW, *entry);
    }
    for (idx, row_entries) in rows.iter().enumerate() {
        draw_menu_row(buffer, PRIMARY_MENU_ROW + idx + 1, row_entries);
    }
    draw_command_prompt_padded(
        buffer,
        menu_prompt_row(PRIMARY_MENU_ROW + rows.len()),
        prompt_label,
        prompt_keys,
    );
}

pub fn draw_expert_menu(
    buffer: &mut PlayfieldBuffer,
    prompt_label: &str,
    prompt_keys: &str,
    notice: Option<&str>,
) {
    draw_command_prompt_at(buffer, EXPERT_MENU_PROMPT_ROW, prompt_label, prompt_keys);
    if let Some(notice) = notice {
        draw_menu_notice(buffer, EXPERT_MENU_PROMPT_ROW, notice);
    }
}

pub fn draw_expert_menu_padded(
    buffer: &mut PlayfieldBuffer,
    prompt_label: &str,
    prompt_keys: &str,
    notice: Option<&str>,
) {
    draw_command_prompt_padded(buffer, EXPERT_MENU_PROMPT_ROW, prompt_label, prompt_keys);
    if let Some(notice) = notice {
        draw_menu_notice_padded(buffer, EXPERT_MENU_PROMPT_ROW, notice);
    }
}

pub fn draw_menu_entry(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkey: &str,
    label: &str,
) {
    draw_menu_entry_styled(buffer, row, col, hotkey, label, false);
}

pub fn draw_menu_entry_item(buffer: &mut PlayfieldBuffer, row: usize, entry: MenuEntry<'_>) {
    draw_menu_entry_styled(
        buffer,
        row,
        entry.col,
        entry.hotkey,
        entry.label,
        entry.featured,
    );
}

pub fn draw_menu_entry_styled(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkey: &str,
    label: &str,
    featured: bool,
) {
    let hotkey_style = classic::menu_hotkey_style();
    let label_style = if featured {
        classic::menu_featured_label_style()
    } else {
        classic::menu_style()
    };
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new(hotkey, hotkey_style),
            StyledSpan::new(">", classic::menu_style()),
            StyledSpan::new(label, label_style),
        ],
    );
}

/// Draw a menu entry with an inline `[ON] [OFF]` toggle indicator.
///
/// The active state is highlighted with `indicator_on_style()` and the
/// inactive state is dimmed with `indicator_off_style()`.  Brackets stay
/// in the normal `menu_style()`.
pub fn draw_menu_entry_with_toggle(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkey: &str,
    label: &str,
    is_on: bool,
) {
    let menu = classic::menu_style();
    let on_style = if is_on {
        classic::indicator_on_style()
    } else {
        classic::indicator_off_style()
    };
    let off_style = if is_on {
        classic::indicator_off_style()
    } else {
        classic::indicator_on_style()
    };
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new(hotkey, classic::menu_hotkey_style()),
            StyledSpan::new(">", menu),
            StyledSpan::new(label, menu),
            StyledSpan::new("[", menu),
            StyledSpan::new("ON", on_style),
            StyledSpan::new("] [", menu),
            StyledSpan::new("OFF", off_style),
            StyledSpan::new("]", menu),
        ],
    );
}

pub fn draw_status_line(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    draw_status_line_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, value);
}

pub fn draw_status_line_padded(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    draw_status_line_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, value);
}

pub fn draw_status_line_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    value: &str,
) {
    buffer.write_spans_clipped(
        row,
        col,
        &[
            StyledSpan::new(label, classic::status_label_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn aligned_label_width<'a, I>(labels: I) -> usize
where
    I: IntoIterator<Item = &'a str>,
{
    labels
        .into_iter()
        .map(|label| label.trim_end().trim_end_matches(':').chars().count())
        .max()
        .unwrap_or(0)
}

pub fn draw_aligned_status_line_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label_width: usize,
    label: &str,
    value: &str,
) {
    if col >= buffer.width() {
        return;
    }
    let normalized_label = label.trim_end().trim_end_matches(':');
    let label_text = format!("{normalized_label:<label_width$}");
    buffer.write_text_clipped(row, col, &label_text, classic::status_label_style());
    let separator_col = col + label_width;
    buffer.write_text_clipped(row, separator_col, ": ", classic::status_label_style());
    let value_col = separator_col + 2;
    buffer.write_text_clipped(row, value_col, value, classic::status_value_style());
}

pub fn draw_aligned_status_line(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label_width: usize,
    label: &str,
    value: &str,
) {
    draw_aligned_status_line_at(buffer, row, LEFT_WINDOW_PAD_COL, label_width, label, value);
}

pub fn draw_aligned_status_line_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label_width: usize,
    label: &str,
    value: &str,
) {
    draw_aligned_status_line_at(buffer, row, LEFT_WINDOW_PAD_COL, label_width, label, value);
}

pub fn draw_aligned_detail_line_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label_width: usize,
    label: &str,
    separator: &str,
    value: &str,
) {
    if col >= buffer.width() {
        return;
    }
    let normalized_label = label.trim_end().trim_end_matches(':');
    let label_text = format!("{normalized_label:<label_width$}");
    buffer.write_text_clipped(row, col, &label_text, classic::status_label_style());
    let separator_col = col + label_width;
    buffer.write_text_clipped(row, separator_col, separator, classic::status_label_style());
    let value_col = separator_col + separator.chars().count();
    buffer.write_text_clipped(row, value_col, value, classic::status_value_style());
}

pub fn draw_aligned_detail_line(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label_width: usize,
    label: &str,
    separator: &str,
    value: &str,
) {
    draw_aligned_detail_line_at(
        buffer,
        row,
        LEFT_WINDOW_PAD_COL,
        label_width,
        label,
        separator,
        value,
    );
}

pub fn draw_aligned_detail_line_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label_width: usize,
    label: &str,
    separator: &str,
    value: &str,
) {
    draw_aligned_detail_line_at(
        buffer,
        row,
        LEFT_WINDOW_PAD_COL,
        label_width,
        label,
        separator,
        value,
    );
}

pub struct DetailField<'a> {
    pub label_width: usize,
    pub label: &'a str,
    pub separator: &'a str,
    pub value: &'a str,
}

pub fn draw_aligned_detail_pair_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left_col: usize,
    left: DetailField<'_>,
    right_col: usize,
    right: DetailField<'_>,
) {
    draw_aligned_detail_line_at(
        buffer,
        row,
        left_col,
        left.label_width,
        left.label,
        left.separator,
        left.value,
    );
    draw_aligned_detail_line_at(
        buffer,
        row,
        right_col,
        right.label_width,
        right.label,
        right.separator,
        right.value,
    );
}

pub fn draw_notice_line(buffer: &mut PlayfieldBuffer, row: usize, value: &str) {
    draw_notice_line_at_col(buffer, row, 0, value);
}

pub fn draw_notice_line_padded(buffer: &mut PlayfieldBuffer, row: usize, value: &str) {
    draw_notice_line_at_col(buffer, row, LEFT_WINDOW_PAD_COL, value);
}

pub fn draw_notice_line_at_col(buffer: &mut PlayfieldBuffer, row: usize, col: usize, value: &str) {
    buffer.write_spans_clipped(
        row,
        col,
        &[
            StyledSpan::new("Notice: ", classic::notice_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn draw_alert_line(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    draw_alert_line_at_col(buffer, row, 0, label, value);
}

pub fn draw_alert_line_padded(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    draw_alert_line_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, value);
}

pub fn draw_alert_line_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    value: &str,
) {
    buffer.write_spans_clipped(
        row,
        col,
        &[
            StyledSpan::new(label, classic::error_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn draw_message_line(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    draw_message_line_at_col(buffer, row, 0, label, value);
}

pub fn draw_message_line_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    value: &str,
) {
    draw_message_line_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, value);
}

pub fn draw_message_line_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    value: &str,
) {
    buffer.write_spans_clipped(
        row,
        col,
        &[
            StyledSpan::new(label, classic::status_label_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn draw_wrapped_status(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_wrapped_status_at_col(buffer, start_row, 0, max_rows, label, value)
}

pub fn draw_wrapped_status_padded(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_wrapped_status_at_col(
        buffer,
        start_row,
        LEFT_WINDOW_PAD_COL,
        max_rows,
        label,
        value,
    )
}

pub fn draw_wrapped_status_at_col(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    if max_rows == 0 {
        return 0;
    }
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let first_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let continuation_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let lines = wrap_text(value, first_width, continuation_width);
    let rows_to_draw = lines.len().min(max_rows);
    for (idx, line) in lines.into_iter().take(rows_to_draw).enumerate() {
        let current_label = if idx == 0 { label } else { &continuation };
        draw_status_line_at_col(buffer, start_row + idx, col, current_label, &line);
    }
    rows_to_draw
}

pub fn draw_wrapped_notice(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    value: &str,
) -> usize {
    draw_wrapped_notice_at_col(buffer, start_row, 0, max_rows, value)
}

pub fn draw_wrapped_notice_padded(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    value: &str,
) -> usize {
    draw_wrapped_notice_at_col(buffer, start_row, LEFT_WINDOW_PAD_COL, max_rows, value)
}

pub fn draw_wrapped_notice_at_col(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    max_rows: usize,
    value: &str,
) -> usize {
    if max_rows == 0 {
        return 0;
    }
    let label = "Notice: ";
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let first_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let continuation_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let lines = wrap_text(value, first_width, continuation_width);
    let rows_to_draw = lines.len().min(max_rows);
    for (idx, line) in lines.into_iter().take(rows_to_draw).enumerate() {
        if idx == 0 {
            draw_notice_line_at_col(buffer, start_row + idx, col, &line);
        } else {
            buffer.write_spans_clipped(
                start_row + idx,
                col,
                &[
                    StyledSpan::new(&continuation, classic::notice_style()),
                    StyledSpan::new(&line, classic::status_value_style()),
                ],
            );
        }
    }
    rows_to_draw
}

pub fn draw_wrapped_alert(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_wrapped_alert_at_col(buffer, start_row, 0, max_rows, label, value)
}

pub fn draw_wrapped_alert_padded(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_wrapped_alert_at_col(
        buffer,
        start_row,
        LEFT_WINDOW_PAD_COL,
        max_rows,
        label,
        value,
    )
}

pub fn draw_wrapped_alert_at_col(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    if max_rows == 0 {
        return 0;
    }
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let first_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let continuation_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let lines = wrap_text(value, first_width, continuation_width);
    let rows_to_draw = lines.len().min(max_rows);
    for (idx, line) in lines.into_iter().take(rows_to_draw).enumerate() {
        if idx == 0 {
            draw_alert_line_at_col(buffer, start_row + idx, col, label, &line);
        } else {
            buffer.write_spans_clipped(
                start_row + idx,
                col,
                &[
                    StyledSpan::new(&continuation, classic::error_style()),
                    StyledSpan::new(&line, classic::status_value_style()),
                ],
            );
        }
    }
    rows_to_draw
}

pub fn draw_wrapped_message(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_wrapped_message_at_col(buffer, start_row, 0, max_rows, label, value)
}

pub fn draw_wrapped_message_padded(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_wrapped_message_at_col(
        buffer,
        start_row,
        LEFT_WINDOW_PAD_COL,
        max_rows,
        label,
        value,
    )
}

pub fn draw_wrapped_message_at_col(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    if max_rows == 0 {
        return 0;
    }
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let first_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let continuation_width = PLAYFIELD_WIDTH.saturating_sub(col + label_width).max(1);
    let lines = wrap_text(value, first_width, continuation_width);
    let rows_to_draw = lines.len().min(max_rows);
    for (idx, line) in lines.into_iter().take(rows_to_draw).enumerate() {
        if idx == 0 {
            draw_message_line_at_col(buffer, start_row + idx, col, label, &line);
        } else {
            buffer.write_spans_clipped(
                start_row + idx,
                col,
                &[
                    StyledSpan::new(&continuation, classic::status_label_style()),
                    StyledSpan::new(&line, classic::status_value_style()),
                ],
            );
        }
    }
    rows_to_draw
}

pub fn draw_menu_notice(buffer: &mut PlayfieldBuffer, command_row: usize, notice: &str) -> usize {
    draw_command_message_stack_at(
        buffer,
        menu_notice_row(command_row),
        &[CommandMessage::Notice(notice)],
    )
}

pub fn draw_menu_notice_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    notice: &str,
) -> usize {
    draw_command_message_stack_at_col(
        buffer,
        menu_notice_row(command_row),
        LEFT_WINDOW_PAD_COL,
        &[CommandMessage::Notice(notice)],
    )
}

pub fn draw_menu_notice_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    notice: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Notice(notice)])
}

pub fn draw_menu_alert_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    label: &str,
    value: &str,
) -> usize {
    let message = match label {
        "Error: " => CommandMessage::Error(value),
        _ => CommandMessage::Warning(value),
    };
    draw_command_message_stack_after(buffer, previous_end_row, &[message])
}

pub fn draw_prompt_notice_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Notice(value)])
}

pub fn draw_prompt_notice_after_padded(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after_at_col(
        buffer,
        previous_end_row,
        LEFT_WINDOW_PAD_COL,
        &[CommandMessage::Notice(value)],
    )
}

pub fn draw_prompt_warning_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Warning(value)])
}

pub fn draw_prompt_warning_after_padded(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after_at_col(
        buffer,
        previous_end_row,
        LEFT_WINDOW_PAD_COL,
        &[CommandMessage::Warning(value)],
    )
}

pub fn draw_prompt_error_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Error(value)])
}

pub fn draw_prompt_error_after_padded(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after_at_col(
        buffer,
        previous_end_row,
        LEFT_WINDOW_PAD_COL,
        &[CommandMessage::Error(value)],
    )
}

pub fn draw_prompt_feedback_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    feedback: &PromptFeedback,
) -> usize {
    match feedback {
        PromptFeedback::Notice(value) => draw_prompt_notice_after(buffer, previous_end_row, value),
        PromptFeedback::Error(value) => draw_prompt_error_after(buffer, previous_end_row, value),
        PromptFeedback::Warning(value) => {
            draw_prompt_warning_after(buffer, previous_end_row, value)
        }
    }
}

pub fn draw_prompt_feedback_after_padded(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    feedback: &PromptFeedback,
) -> usize {
    match feedback {
        PromptFeedback::Notice(value) => {
            draw_prompt_notice_after_padded(buffer, previous_end_row, value)
        }
        PromptFeedback::Error(value) => {
            draw_prompt_error_after_padded(buffer, previous_end_row, value)
        }
        PromptFeedback::Warning(value) => {
            draw_prompt_warning_after_padded(buffer, previous_end_row, value)
        }
    }
}

pub fn draw_menu_general_message(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_general_message_after_command(buffer, command_row, label, value)
}

pub fn draw_general_message_after_command(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_command_message_stack(
        buffer,
        command_row,
        &[CommandMessage::General { label, value }],
    )
}

pub fn draw_general_message_after_command_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_command_message_stack_at_col(
        buffer,
        menu_general_message_row(command_row),
        LEFT_WINDOW_PAD_COL,
        &[CommandMessage::General { label, value }],
    )
}

pub fn draw_command_message_stack(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at(buffer, menu_general_message_row(command_row), messages)
}

pub fn draw_command_message_stack_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at_col(
        buffer,
        menu_general_message_row(command_row),
        LEFT_WINDOW_PAD_COL,
        messages,
    )
}

pub fn draw_command_message_stack_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at(
        buffer,
        (previous_end_row + 2).min(last_body_row()),
        messages,
    )
}

pub fn draw_command_message_stack_after_at_col(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    col: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at_col(
        buffer,
        (previous_end_row + 2).min(last_body_row()),
        col,
        messages,
    )
}

fn draw_command_message_stack_at(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at_col(buffer, start_row, 0, messages)
}

fn draw_command_message_stack_at_col(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    if messages.is_empty() {
        return start_row.saturating_sub(1);
    }
    let mut end_row = start_row.saturating_sub(1);
    for (idx, message) in messages.iter().enumerate() {
        let row = if idx == 0 {
            start_row
        } else {
            (end_row + 2).min(last_body_row())
        };
        end_row = draw_command_message_block(buffer, row, col, *message);
    }
    end_row
}

fn draw_command_message_block(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    message: CommandMessage<'_>,
) -> usize {
    match message {
        CommandMessage::General { label, value } => {
            let max_rows = last_body_row().saturating_sub(row) + 1;
            let drawn = draw_wrapped_message_at_col(buffer, row, col, max_rows, label, value);
            row + drawn.saturating_sub(1)
        }
        CommandMessage::Notice(value) => {
            let max_rows = (last_body_row().saturating_sub(row) + 1).min(3);
            let drawn = draw_wrapped_notice_at_col(buffer, row, col, max_rows, value);
            row + drawn.saturating_sub(1)
        }
        CommandMessage::Error(value) => {
            let max_rows = (last_body_row().saturating_sub(row) + 1).min(3);
            let drawn = draw_wrapped_alert_at_col(buffer, row, col, max_rows, "Error: ", value);
            row + drawn.saturating_sub(1)
        }
        CommandMessage::Warning(value) => {
            let max_rows = (last_body_row().saturating_sub(row) + 1).min(3);
            let drawn = draw_wrapped_alert_at_col(buffer, row, col, max_rows, "Warning: ", value);
            row + drawn.saturating_sub(1)
        }
    }
}

pub fn draw_inline_planet_info_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    default_coords: [u8; 2],
    input: &str,
    error: Option<&str>,
    notice: Option<&str>,
) -> usize {
    draw_command_line_default_input_at(
        buffer,
        command_row,
        "COMMAND",
        "Planet coords ",
        &format_sector_coords_default(default_coords),
        input,
    );
    let mut messages = vec![CommandMessage::General {
        label: "",
        value: "Enter coordinates of the planet to view.",
    }];
    if let Some(error) = error {
        messages.push(CommandMessage::Error(error));
    }
    if let Some(notice) = notice {
        messages.push(CommandMessage::Notice(notice));
    }
    draw_command_message_stack(buffer, command_row, &messages)
}

pub fn draw_inline_planet_info_prompt_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    default_coords: [u8; 2],
    input: &str,
    error: Option<&str>,
    notice: Option<&str>,
) -> usize {
    draw_command_line_default_input_at_col(
        buffer,
        command_row,
        LEFT_WINDOW_PAD_COL,
        "COMMAND",
        "Planet coords ",
        &format_sector_coords_default(default_coords),
        input,
    );
    let mut messages = vec![CommandMessage::General {
        label: "",
        value: "Enter coordinates of the planet to view.",
    }];
    if let Some(error) = error {
        messages.push(CommandMessage::Error(error));
    }
    if let Some(notice) = notice {
        messages.push(CommandMessage::Notice(notice));
    }
    draw_command_message_stack_padded(buffer, command_row, &messages)
}

pub fn draw_inline_delete_reviewables_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    notice: Option<&str>,
) -> usize {
    draw_inline_confirm_prompt(buffer, command_row, "COMMAND");
    draw_inline_confirm_block(
        buffer,
        command_row,
        "DELETE ALL REPORTS:",
        &["Clear all reports?"],
        notice,
    )
}

pub fn draw_inline_delete_reviewables_prompt_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    notice: Option<&str>,
) -> usize {
    draw_inline_confirm_prompt_padded(buffer, command_row, "COMMAND");
    draw_inline_confirm_block_padded(
        buffer,
        command_row,
        "DELETE ALL REPORTS:",
        &["Clear all reports?"],
        notice,
    )
}

pub fn draw_inline_confirm_prompt(buffer: &mut PlayfieldBuffer, command_row: usize, label: &str) {
    draw_command_line_prompt_text_at(buffer, command_row, label, "Y/[N] -> ");
}

pub fn draw_inline_confirm_prompt_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    label: &str,
) {
    draw_command_line_prompt_text_at_col(
        buffer,
        command_row,
        LEFT_WINDOW_PAD_COL,
        label,
        "Y/[N] -> ",
    );
}

pub fn draw_inline_confirm_block(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    title: &str,
    lines: &[&str],
    notice: Option<&str>,
) -> usize {
    draw_inline_confirm_block_at_col(buffer, command_row, 0, title, lines, notice)
}

pub fn draw_inline_confirm_block_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    title: &str,
    lines: &[&str],
    notice: Option<&str>,
) -> usize {
    draw_inline_confirm_block_at_col(
        buffer,
        command_row,
        LEFT_WINDOW_PAD_COL,
        title,
        lines,
        notice,
    )
}

fn draw_inline_confirm_block_at_col(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    col: usize,
    title: &str,
    lines: &[&str],
    notice: Option<&str>,
) -> usize {
    let title_row = menu_general_message_row(command_row);
    buffer.write_text(title_row, col, title, classic::notice_style());
    let mut end_row = title_row;
    for line in lines {
        end_row = (end_row + 1).min(last_body_row());
        buffer.write_text(end_row, col, line, classic::status_value_style());
    }
    if let Some(notice) = notice {
        draw_command_message_stack_after_at_col(
            buffer,
            end_row,
            col,
            &[CommandMessage::Notice(notice)],
        )
    } else {
        end_row
    }
}

pub fn draw_inline_tax_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    current_tax: &str,
    input: &str,
    error: Option<&str>,
    notice: Option<&str>,
) -> usize {
    draw_command_line_default_input_at(
        buffer,
        command_row,
        "PLANET COMMAND",
        "Empire tax rate (0 - 100) ",
        current_tax,
        input,
    );
    let mut messages = vec![
        CommandMessage::General {
            label: "PLANET TAX: ",
            value: "Set empire tax rate.",
        },
        CommandMessage::Warning(
            "Taxes in excess of 65% may actually REDUCE your planets' productivity!",
        ),
    ];
    if let Some(error) = error {
        messages.push(CommandMessage::Error(error));
    }
    if let Some(notice) = notice {
        messages.push(CommandMessage::Notice(notice));
    }
    draw_command_message_stack(buffer, command_row, &messages)
}

pub fn draw_inline_tax_prompt_padded(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    current_tax: &str,
    input: &str,
    error: Option<&str>,
    notice: Option<&str>,
) -> usize {
    draw_command_line_default_input_at_col(
        buffer,
        command_row,
        LEFT_WINDOW_PAD_COL,
        "PLANET COMMAND",
        "Empire tax rate (0 - 100) ",
        current_tax,
        input,
    );
    let mut messages = vec![
        CommandMessage::General {
            label: "PLANET TAX: ",
            value: "Set empire tax rate.",
        },
        CommandMessage::Warning(
            "Taxes in excess of 65% may actually REDUCE your planets' productivity!",
        ),
    ];
    if let Some(error) = error {
        messages.push(CommandMessage::Error(error));
    }
    if let Some(notice) = notice {
        messages.push(CommandMessage::Notice(notice));
    }
    draw_command_message_stack_padded(buffer, command_row, &messages)
}

pub fn draw_centered_text(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    text: &str,
    style: crate::screen::CellStyle,
) {
    let col = PLAYFIELD_WIDTH.saturating_sub(text.chars().count()) / 2;
    buffer.write_text(row, col, text, style);
}

pub fn draw_command_prompt_at(buffer: &mut PlayfieldBuffer, row: usize, label: &str, keys: &str) {
    draw_command_prompt_at_col(buffer, row, 0, label, keys);
}

pub fn draw_command_prompt_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    keys: &str,
) {
    draw_command_prompt_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, keys);
}

pub fn draw_command_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    keys: &str,
) {
    shared_prompt::draw_command_prompt_at_col(buffer, row, col, label, keys);
}

pub fn draw_command_line_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    text: &str,
) {
    draw_command_line_text_at_col(buffer, row, 0, label, text);
}

pub fn draw_command_line_text_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    text: &str,
) {
    draw_command_line_text_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, text);
}

pub fn draw_command_line_text_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    text: &str,
) {
    shared_prompt::draw_command_line_text_at_col(buffer, row, col, label, text);
}

pub fn draw_command_line_prompt_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
) {
    draw_command_line_prompt_text_at_col(buffer, row, 0, label, prompt);
}

pub fn draw_command_line_prompt_text_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
) {
    draw_command_line_prompt_text_at_col(buffer, row, LEFT_WINDOW_PAD_COL, label, prompt);
}

pub fn draw_command_line_prompt_text_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
) {
    shared_prompt::draw_command_line_prompt_text_at_col(buffer, row, col, label, prompt);
}

pub fn draw_command_line_prompt_input_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    input: &str,
) {
    shared_prompt::draw_command_line_prompt_input_at_col(buffer, row, col, label, prompt, input);
}

pub fn draw_command_line_default_input_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    draw_command_line_default_input_at_col(buffer, row, 0, label, prompt, default, input)
}

pub fn draw_command_line_default_input_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    draw_command_line_default_input_at_col(
        buffer,
        row,
        LEFT_WINDOW_PAD_COL,
        label,
        prompt,
        default,
        input,
    )
}

pub fn draw_command_line_default_input_with_cancel_padded(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
    cancel_markup: &str,
) {
    draw_command_line_default_input_with_cancel_at_col(
        buffer,
        row,
        LEFT_WINDOW_PAD_COL,
        label,
        prompt,
        default,
        input,
        cancel_markup,
    )
}

pub fn draw_command_line_default_input_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    shared_prompt::draw_command_line_default_input_at_col(
        buffer, row, col, label, prompt, default, input,
    );
}

pub fn draw_command_line_default_input_with_cancel_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
    cancel_markup: &str,
) {
    shared_prompt::draw_command_line_default_input_with_cancel_at_col(
        buffer,
        row,
        col,
        label,
        prompt,
        default,
        input,
        cancel_markup,
    );
}

pub fn draw_table_command_bar(
    buffer: &mut PlayfieldBuffer,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    draw_table_command_bar_at(buffer, COMMAND_LINE_ROW, hotkeys_markup, default, input)
}

pub fn draw_table_command_bar_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    draw_table_command_bar_at_col(buffer, row, 0, hotkeys_markup, default, input)
}

pub fn draw_table_command_bar_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    shared_prompt::draw_table_command_bar_at_col(buffer, row, col, hotkeys_markup, default, input)
}

pub fn draw_labeled_table_command_bar_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    shared_prompt::draw_labeled_table_command_bar_at_col(
        buffer,
        row,
        col,
        label,
        hotkeys_markup,
        default,
        input,
    )
}

pub fn draw_table_command_prompt(buffer: &mut PlayfieldBuffer, prompt: &str) -> usize {
    draw_table_command_prompt_at(buffer, COMMAND_LINE_ROW, prompt)
}

pub fn draw_table_command_prompt_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    prompt: &str,
) -> usize {
    draw_table_command_prompt_at_col(buffer, row, 0, prompt)
}

pub fn draw_table_command_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    prompt: &str,
) -> usize {
    shared_prompt::draw_table_command_prompt_at_col(buffer, row, col, prompt)
}

pub fn draw_plain_prompt(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) -> usize {
    draw_plain_prompt_at_col(buffer, row, 0, prompt)
}

pub fn draw_plain_prompt_padded(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) -> usize {
    draw_plain_prompt_at_col(buffer, row, LEFT_WINDOW_PAD_COL, prompt)
}

pub fn draw_plain_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    prompt: &str,
) -> usize {
    shared_prompt::draw_plain_prompt_at_col(buffer, row, col, prompt)
}

pub fn draw_dismiss_prompt(buffer: &mut PlayfieldBuffer, row: usize) -> usize {
    draw_dismiss_prompt_at_col(buffer, row, 0)
}

pub fn draw_dismiss_prompt_padded(buffer: &mut PlayfieldBuffer, row: usize) -> usize {
    draw_dismiss_prompt_at_col(buffer, row, LEFT_WINDOW_PAD_COL)
}

pub fn draw_dismiss_prompt_at_col(buffer: &mut PlayfieldBuffer, row: usize, col: usize) -> usize {
    draw_plain_prompt_at_col(buffer, row, col, "(slap a key)")
}

pub fn draw_help_panel(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    header: &str,
    lines: &[&str],
    prompt_label: &str,
) {
    draw_title_bar_padded(buffer, 0, title);
    buffer.fill_row(2, classic::help_header_style());
    buffer.write_text(2, LEFT_WINDOW_PAD_COL, header, classic::help_header_style());
    for row in 3..COMMAND_LINE_ROW {
        buffer.fill_row(row, classic::help_panel_style());
    }
    let mut last_content_row = 2;
    for (idx, line) in lines.iter().enumerate() {
        let row = 3 + idx;
        if row >= COMMAND_LINE_ROW - 1 {
            break;
        }
        buffer.write_text(row, LEFT_WINDOW_PAD_COL, line, classic::help_panel_style());
        last_content_row = row;
    }
    let _ = prompt_label;
    draw_dismiss_prompt_padded(buffer, dismiss_prompt_row(last_content_row));
}

pub fn draw_bottom_aligned_transcript_rows<F>(
    buffer: &mut PlayfieldBuffer,
    rows: &[String],
    revealed_rows: usize,
    first_content_row: usize,
    last_content_row: usize,
    mut draw_row: F,
) where
    F: FnMut(&mut PlayfieldBuffer, usize, &str),
{
    if first_content_row > last_content_row {
        return;
    }
    let page_height = last_content_row - first_content_row + 1;
    let revealed_end = revealed_rows.min(rows.len());
    let visible_start = revealed_end.saturating_sub(page_height);
    let visible_rows = &rows[visible_start..revealed_end];
    let first_row = last_content_row
        .saturating_add(1)
        .saturating_sub(visible_rows.len());
    for (idx, line) in visible_rows.iter().enumerate() {
        draw_row(buffer, first_row + idx, line);
    }
}

pub fn wrap_text(value: &str, first_width: usize, continuation_width: usize) -> Vec<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>();
    if normalized.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut limit = first_width;
    for word in normalized {
        let separator = if current.is_empty() { 0 } else { 1 };
        if current.len() + separator + word.len() > limit && !current.is_empty() {
            lines.push(current);
            current = String::new();
            limit = continuation_width;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        if word.len() > limit && current.is_empty() {
            let mut remaining = word;
            while remaining.len() > limit {
                lines.push(remaining[..limit].to_string());
                remaining = &remaining[limit..];
                limit = continuation_width;
            }
            current.push_str(remaining);
        } else {
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
