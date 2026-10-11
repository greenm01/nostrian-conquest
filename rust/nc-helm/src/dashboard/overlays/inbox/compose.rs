use crate::dashboard::app::state::{ActiveOverlay, DashApp, InboxPromptMode};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::{Rect, wrap_modal_text_lines};
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, overlay_chrome_height,
    overlay_popup_rect_for_body_in_parent, standard_table_body_height, write_clipped,
};
use crate::dashboard::table::{
    TableColumn, TableFooter, TableWidthMode, draw_scrollbar_at, resolve_table_columns_for_widget,
    table_render_width, write_table_window_with_cursor_at,
};
use crate::dashboard::theme;

const RECIPIENT_COLUMNS: [TableColumn<'static>; 2] = [
    TableColumn::right("Emp#", 4),
    TableColumn::left_flex("Empire", 28, 1),
];

const TITLE: &str = "COMMUNICATE (SEND MESSAGE)";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let mode = app.inbox_overlay.prompt_mode;
    if mode == InboxPromptMode::ComposeRecipient {
        draw_recipient_picker(buf, app);
    } else {
        draw_prompt_overlay(buf, app);
    }
}

pub fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    let mode = app.inbox_overlay.prompt_mode;
    if mode == InboxPromptMode::ComposeRecipient {
        recipient_picker_rect(app)
    } else {
        prompt_overlay_rect(app)
    }
}

fn draw_recipient_picker(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let rows = recipient_rows(app);
    let footer = TableFooter::None;
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let visible_rows = recipient_visible_rows(parent, rows.len(), footer);
    let scrollable = rows.len() > visible_rows;
    let target_width = compose_body_width(parent);

    let columns = resolve_table_columns_for_widget(
        &RECIPIENT_COLUMNS,
        &rows,
        target_width,
        scrollable,
        TableWidthMode::Expand,
        Some(TITLE),
        Some(footer),
    );
    let table_width = table_render_width(&columns);
    let body_width = table_width + usize::from(scrollable);
    let body_height = standard_table_body_height(visible_rows);

    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        TITLE,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );
    assert_overlay_body_write_fits(frame, TITLE, body_width, body_height);

    let _metrics = write_table_window_with_cursor_at(
        buf,
        frame.body_row,
        frame.body_col,
        &columns,
        &rows,
        app.inbox_overlay.compose_recipient_scroll,
        visible_rows,
        theme::value_style(),
        theme::label_style(),
        Some(app.inbox_overlay.compose_recipient_selected),
        0,
    );

    if scrollable {
        draw_scrollbar_at(
            buf,
            frame.body_row,
            frame.body_col + table_width,
            visible_rows,
            rows.len(),
            app.inbox_overlay.compose_recipient_scroll,
            theme::table_theme(),
        );
    }
}

fn draw_prompt_overlay(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let footer = prompt_footer(app);
    let lines = prompt_overlay_lines(app, parent);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1)
        .max(1);
    let body_height = lines.len().max(1);

    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        TITLE,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );

    for (idx, line) in lines.iter().enumerate() {
        let style = if app.inbox_overlay.prompt_mode == InboxPromptMode::ComposeSubject && idx > 0 {
            theme::value_style()
        } else {
            theme::label_style()
        };
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            style,
        );
    }
}

pub fn recipient_rows(app: &DashApp) -> Vec<Vec<String>> {
    app.game_data
        .players
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx + 1 != app.player_record_index_1_based)
        .map(|(idx, player)| {
            let empire_id = idx + 1;
            let name = player.empire_name.trim().to_string();
            let fallback = player.empire_name.trim().to_string();
            let display = if !name.is_empty() { name } else { fallback };
            vec![format!("{:>2}", empire_id), display]
        })
        .collect()
}

fn prompt_footer<'a>(app: &'a DashApp) -> TableFooter<'a> {
    match app.inbox_overlay.prompt_mode {
        InboxPromptMode::ComposeSubject => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Subject -> ",
            input: &app.inbox_overlay.prompt_input,
        },
        InboxPromptMode::ComposeConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Send message? Y/[N] ->",
        },
        _ => TableFooter::None,
    }
}

fn recipient_picker_rect(app: &DashApp) -> Rect {
    let rows = recipient_rows(app);
    let footer = TableFooter::None;
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let visible_rows = recipient_visible_rows(parent, rows.len(), footer);
    let scrollable = rows.len() > visible_rows;
    let target_width = compose_body_width(parent);
    let columns = resolve_table_columns_for_widget(
        &RECIPIENT_COLUMNS,
        &rows,
        target_width,
        scrollable,
        TableWidthMode::Expand,
        Some(TITLE),
        Some(footer),
    );
    overlay_popup_rect_for_body_in_parent(
        parent,
        TITLE,
        table_render_width(&columns) + usize::from(scrollable),
        standard_table_body_height(visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

fn prompt_overlay_rect(app: &DashApp) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let lines = prompt_overlay_lines(app, parent);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1)
        .max(1);
    overlay_popup_rect_for_body_in_parent(
        parent,
        TITLE,
        body_width,
        lines.len().max(1),
        OverlaySizePolicy::default(),
        prompt_footer(app),
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

fn recipient_visible_rows(parent: Rect, row_count: usize, footer: TableFooter<'_>) -> usize {
    let max_popup_height = (parent.height as usize).saturating_sub(2).max(1);
    let max_body_height = max_popup_height.saturating_sub(overlay_chrome_height(footer));
    let max_table_rows = max_body_height.saturating_sub(4);
    row_count.min(max_table_rows)
}

fn compose_body_width(parent: Rect) -> usize {
    parent.width.saturating_sub(6).max(1).min(80) as usize
}

fn prompt_overlay_lines(app: &DashApp, parent: Rect) -> Vec<String> {
    let recipient_name = app
        .inbox_overlay
        .compose_recipient_empire
        .map(|id| compose_empire_name(app, id))
        .unwrap_or_else(|| "Unknown".to_string());
    let subject = if app.inbox_overlay.prompt_mode == InboxPromptMode::ComposeSubject {
        &app.inbox_overlay.prompt_input
    } else {
        &app.inbox_overlay.compose_subject
    };
    wrap_modal_text_lines(
        &[
            format!("To: {recipient_name}"),
            format!("Subject: {subject}"),
        ],
        compose_body_width(parent),
    )
}

fn compose_empire_name(app: &DashApp, empire_id: u8) -> String {
    let Some(player) = app
        .game_data
        .players
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id:02}");
    };
    let name = player.empire_name.trim().to_string();
    let fallback = player.empire_name.trim().to_string();
    if !name.is_empty() {
        name
    } else if !fallback.is_empty() {
        fallback
    } else {
        format!("Empire {empire_id:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::app::state::{DashApp, InboxPromptMode};
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn recipient_picker_popup_reserves_full_table_body_height() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(108, 26),
            1,
        );
        app.inbox_overlay.prompt_mode = InboxPromptMode::ComposeRecipient;

        let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(&app).widgets);
        let popup = recipient_picker_rect(&app);
        let visible_rows =
            recipient_visible_rows(parent, recipient_rows(&app).len(), TableFooter::None);
        let body_height = popup.height as usize - overlay_chrome_height(TableFooter::None);

        assert!(body_height >= standard_table_body_height(visible_rows));
    }

    #[test]
    fn recipient_picker_rows_show_empire_names() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(108, 26),
            1,
        );
        nc_data::set_empire_name(&mut app.game_data.players[1], "Rigel Empire");
        app.inbox_overlay.prompt_mode = InboxPromptMode::ComposeRecipient;

        assert!(
            recipient_rows(&app)
                .iter()
                .any(|row| row.get(1).is_some_and(|name| name == "Rigel Empire"))
        );
    }

    #[test]
    fn subject_prompt_wraps_long_content_into_taller_popup() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(80, 26),
            ScreenGeometry::new(108, 26),
            1,
        );
        app.inbox_overlay.prompt_mode = InboxPromptMode::ComposeSubject;
        app.inbox_overlay.compose_recipient_empire = Some(2);
        app.inbox_overlay.prompt_input =
            "A deliberately long diplomatic subject that needs wrapping before the confirmation popup can display it cleanly".to_string();

        let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(&app).widgets);
        let lines = prompt_overlay_lines(&app, parent);
        let popup = prompt_overlay_rect(&app);

        assert!(lines.len() > 2);
        assert!(popup.height as usize > overlay_chrome_height(prompt_footer(&app)) + 2);
    }
}
