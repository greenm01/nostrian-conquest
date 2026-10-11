use super::HOTKEYS;
use crate::dashboard::app::state::{ActiveOverlay, DashApp, InboxFocus, InboxPromptMode};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::inbox::{DashInboxItem, matches_filter, project_inbox_items};
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::Rect;
use crate::dashboard::modal::wrap_modal_text_lines;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, overlay_chrome_height,
    overlay_popup_rect_for_body_in_parent, standard_table_body_height, write_clipped,
};
use crate::dashboard::table::{
    TableAlign, TableColumn, TableFooter, TableRowState, draw_scrollbar_at, table_render_width,
    write_table_window_with_theme_at,
};
use crate::dashboard::theme;

const INBOX_LIST_TARGET_TABLE_WIDTH: usize = 38;
const INBOX_PREVIEW_TARGET_TABLE_WIDTH: usize = 80;
const INBOX_TABLE_GAP_WIDTH: usize = 1;
const TABLE_SCROLL_GUTTER_WIDTH: usize = 1;
const INBOX_LIST_MIN_TABLE_WIDTH: usize = 22;
const INBOX_PREVIEW_MIN_TABLE_WIDTH: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxPane {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InboxPaneLayout {
    list_table_width: usize,
    list_scroll_gutter_width: usize,
    preview_offset: usize,
    preview_table_width: usize,
    preview_scroll_gutter_width: usize,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    if app.inbox_overlay.prompt_mode == InboxPromptMode::Outbox {
        draw_outbox(buf, app, _map_frame);
        return;
    }
    // TODO: Routing for Compose stages will go here or in mod.rs
    let items = inbox_items(app);
    let selected = app
        .inbox_overlay
        .selected
        .min(items.len().saturating_sub(1));
    let selected_default = items.get(selected).map(|_| format!("{:02}", selected + 1));
    let footer = inbox_footer(app, selected_default.as_deref());
    let filter_line = format!(
        "Filter:{}  Year:{}  Focus:{}{}",
        app.inbox_overlay.filter.label(),
        if app.inbox_overlay.current_year_only {
            "Current"
        } else {
            "All"
        },
        match app.inbox_overlay.focus {
            InboxFocus::List => "List",
            InboxFocus::Preview => "Preview",
        },
        if app.inbox_overlay.delete_confirm {
            "  Delete this item? Y/[N]"
        } else {
            ""
        }
    );
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = inbox_body_width(&filter_line).min(max_overlay_body_width_for_parent(parent));
    let natural_content_rows = inbox_natural_content_rows(&items, selected, body_width);
    let body_height = 1 + standard_table_body_height(natural_content_rows);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        "INBOX",
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );
    let visible_rows = frame
        .body_height
        .saturating_sub(1 + standard_table_body_height(0));
    let list_rows = inbox_list_rows(&items);
    let list_scrollable = list_rows.len() > visible_rows;
    let pane_layout = inbox_pane_layout_for_scrollbars(frame.body_width, list_scrollable, false);
    let preview_probe_rows = preview_rows(
        items.get(selected),
        preview_content_width(pane_layout.preview_table_width),
    );
    let preview_scrollable = preview_probe_rows.len() > visible_rows;
    let pane_layout =
        inbox_pane_layout_for_scrollbars(frame.body_width, list_scrollable, preview_scrollable);
    let list_columns = inbox_list_columns(pane_layout.list_table_width);
    let preview_columns = (pane_layout.preview_table_width > 0)
        .then(|| preview_columns(pane_layout.preview_table_width));
    let preview_rows = preview_rows(
        items.get(selected),
        preview_content_width(pane_layout.preview_table_width),
    );
    let used_width = preview_columns
        .as_ref()
        .map(|columns| {
            pane_layout.preview_offset
                + table_render_width(columns)
                + pane_layout.preview_scroll_gutter_width
        })
        .unwrap_or_else(|| {
            table_render_width(&list_columns) + pane_layout.list_scroll_gutter_width
        });
    let used_height = 1 + standard_table_body_height(visible_rows);
    assert_overlay_body_write_fits(frame, "INBOX", used_width, used_height);

    let list_focus = matches!(app.inbox_overlay.focus, InboxFocus::List);
    let preview_focus = matches!(app.inbox_overlay.focus, InboxFocus::Preview);
    let mut list_theme = theme::table_theme();
    let mut preview_theme = theme::table_theme();
    if list_focus {
        list_theme.header_style = theme::classic::notice_style();
    }
    if preview_focus {
        preview_theme.header_style = theme::classic::notice_style();
        preview_theme.body_style = theme::value_style();
    } else {
        preview_theme.body_style = theme::label_style();
    }

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        &filter_line,
        theme::section_title_style(),
    );

    let list_row_states = inbox_list_row_states(items.is_empty(), list_rows.len());
    let scroll = if items.is_empty() {
        0
    } else {
        clamp_scroll(
            app.inbox_overlay.scroll,
            selected,
            visible_rows,
            items.len(),
        )
    };
    let _list_metrics = write_table_window_with_theme_at(
        buf,
        frame.body_row + 1,
        frame.body_col,
        &list_columns,
        &list_rows,
        scroll,
        visible_rows,
        list_theme,
        items.get(selected).map(|_| selected),
        0,
        Some(&list_row_states),
    );

    if pane_layout.preview_table_width > 0 {
        let preview_col = frame.body_col + pane_layout.preview_offset;
        let preview_scroll = clamp_offset(
            app.inbox_overlay.preview_scroll,
            visible_rows,
            preview_rows.len(),
        );
        let _preview_metrics = write_table_window_with_theme_at(
            buf,
            frame.body_row + 1,
            preview_col,
            preview_columns.as_ref().expect("preview columns"),
            &preview_rows,
            preview_scroll,
            visible_rows,
            preview_theme,
            None,
            0,
            None,
        );
        if preview_rows.len() > visible_rows {
            draw_scrollbar_at(
                buf,
                frame.body_row + 1,
                preview_col + pane_layout.preview_table_width,
                visible_rows,
                preview_rows.len(),
                preview_scroll,
                preview_theme,
            );
        }
    }
}

pub(crate) fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    if app.inbox_overlay.prompt_mode == InboxPromptMode::Outbox {
        return outbox_popup_rect(app, _map_frame);
    }
    let items = inbox_items(app);
    let selected = app
        .inbox_overlay
        .selected
        .min(items.len().saturating_sub(1));
    let selected_default = items.get(selected).map(|_| format!("{:02}", selected + 1));
    let footer = inbox_footer(app, selected_default.as_deref());
    let filter_line = format!(
        "Filter:{}  Year:{}  Focus:{}{}",
        app.inbox_overlay.filter.label(),
        if app.inbox_overlay.current_year_only {
            "Current"
        } else {
            "All"
        },
        match app.inbox_overlay.focus {
            InboxFocus::List => "List",
            InboxFocus::Preview => "Preview",
        },
        if app.inbox_overlay.delete_confirm {
            "  Delete this item? Y/[N]"
        } else {
            ""
        }
    );
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = inbox_body_width(&filter_line).min(max_overlay_body_width_for_parent(parent));
    let natural_content_rows = inbox_natural_content_rows(&items, selected, body_width);
    overlay_popup_rect_for_body_in_parent(
        parent,
        "INBOX",
        body_width,
        1 + standard_table_body_height(natural_content_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

fn inbox_footer<'a>(app: &'a DashApp, selected_default: Option<&'a str>) -> TableFooter<'a> {
    match app.inbox_overlay.prompt_mode {
        InboxPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default,
            input: &app.inbox_overlay.jump_input,
        },
        InboxPromptMode::ComposeRecipient => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Recipient empire #",
            default: &app.inbox_overlay.prompt_default,
            input: &app.inbox_overlay.prompt_input,
        },
        InboxPromptMode::ComposeSubject => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Subject -> ",
            input: &app.inbox_overlay.prompt_input,
        },
        InboxPromptMode::ComposeBody => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Body -> ",
            input: &app.inbox_overlay.prompt_input,
        },
        InboxPromptMode::ComposeConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Send message? Y/[N] ->",
        },
        InboxPromptMode::Outbox => unreachable!("outbox uses its own footer"),
    }
}

fn draw_outbox(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let messages = staged_outbox_messages(app);
    let selected = app
        .inbox_overlay
        .outbox_selected
        .min(messages.len().saturating_sub(1));
    let selected_default = messages
        .get(selected)
        .map(|_| format!("{:02}", selected + 1));
    let footer = TableFooter::CommandBar {
        hotkeys_markup: "? D <ESC>",
        default: selected_default.as_deref(),
        input: &app.inbox_overlay.prompt_input,
    };
    let filter_line = format!(
        "OUTBOX - Focus:{}",
        match app.inbox_overlay.outbox_focus {
            InboxFocus::List => "List",
            InboxFocus::Preview => "Preview",
        }
    );
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = inbox_body_width(&filter_line).min(max_overlay_body_width_for_parent(parent));
    let natural_rows = outbox_natural_content_rows(&messages, selected, &app.game_data, body_width);
    let body_height = 1 + standard_table_body_height(natural_rows);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        "OUTBOX",
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );
    let visible_rows = frame
        .body_height
        .saturating_sub(1 + standard_table_body_height(0));
    let rows = outbox_rows(&app.game_data, &messages);
    let list_scrollable = rows.len() > visible_rows;
    let pane_layout = inbox_pane_layout_for_scrollbars(frame.body_width, list_scrollable, false);
    let preview_probe_rows = outbox_preview_rows(
        messages.get(selected),
        preview_content_width(pane_layout.preview_table_width),
    );
    let preview_scrollable = preview_probe_rows.len() > visible_rows;
    let pane_layout =
        inbox_pane_layout_for_scrollbars(frame.body_width, list_scrollable, preview_scrollable);
    let list_columns = outbox_columns(pane_layout.list_table_width);
    let preview_columns = (pane_layout.preview_table_width > 0)
        .then(|| preview_columns(pane_layout.preview_table_width));
    let preview_rows = outbox_preview_rows(
        messages.get(selected),
        preview_content_width(pane_layout.preview_table_width),
    );

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        &filter_line,
        theme::section_title_style(),
    );

    let list_focus = matches!(app.inbox_overlay.outbox_focus, InboxFocus::List);
    let preview_focus = matches!(app.inbox_overlay.outbox_focus, InboxFocus::Preview);
    let mut list_theme = theme::table_theme();
    let mut preview_theme = theme::table_theme();
    if list_focus {
        list_theme.header_style = theme::classic::notice_style();
    }
    if preview_focus {
        preview_theme.header_style = theme::classic::notice_style();
        preview_theme.body_style = theme::value_style();
    } else {
        preview_theme.body_style = theme::label_style();
    }

    let row_states = inbox_list_row_states(messages.is_empty(), rows.len());
    let scroll = if messages.is_empty() {
        0
    } else {
        clamp_scroll(
            app.inbox_overlay.outbox_scroll,
            selected,
            visible_rows,
            messages.len(),
        )
    };
    let _metrics = write_table_window_with_theme_at(
        buf,
        frame.body_row + 1,
        frame.body_col,
        &list_columns,
        &rows,
        scroll,
        visible_rows,
        list_theme,
        messages.get(selected).map(|_| selected),
        0,
        Some(&row_states),
    );

    if pane_layout.preview_table_width > 0 {
        let preview_col = frame.body_col + pane_layout.preview_offset;
        let preview_scroll = clamp_offset(
            app.inbox_overlay.outbox_preview_scroll,
            visible_rows,
            preview_rows.len(),
        );
        let _preview_metrics = write_table_window_with_theme_at(
            buf,
            frame.body_row + 1,
            preview_col,
            preview_columns.as_ref().expect("preview columns"),
            &preview_rows,
            preview_scroll,
            visible_rows,
            preview_theme,
            None,
            0,
            None,
        );
        if preview_rows.len() > visible_rows {
            draw_scrollbar_at(
                buf,
                frame.body_row + 1,
                preview_col + pane_layout.preview_table_width,
                visible_rows,
                preview_rows.len(),
                preview_scroll,
                preview_theme,
            );
        }
    }
}

fn outbox_popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    let messages = staged_outbox_messages(app);
    let selected = app
        .inbox_overlay
        .outbox_selected
        .min(messages.len().saturating_sub(1));
    let filter_line = format!(
        "OUTBOX - Focus:{}",
        match app.inbox_overlay.outbox_focus {
            InboxFocus::List => "List",
            InboxFocus::Preview => "Preview",
        }
    );
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = inbox_body_width(&filter_line).min(max_overlay_body_width_for_parent(parent));
    let natural_rows = outbox_natural_content_rows(&messages, selected, &app.game_data, body_width);
    let body_height = 1 + standard_table_body_height(natural_rows);
    overlay_popup_rect_for_body_in_parent(
        parent,
        "OUTBOX",
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        TableFooter::CommandBar {
            hotkeys_markup: "? D <ESC>",
            default: None,
            input: &app.inbox_overlay.prompt_input,
        },
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

pub fn staged_outbox_messages(app: &DashApp) -> Vec<nc_data::TurnMessage> {
    app.hosted_turn_draft
        .as_ref()
        .map(|draft| draft.messages.clone())
        .unwrap_or_default()
}

pub fn hit_test_inbox_pane(
    app: &DashApp,
    _map_frame: MapWidgetFrame,
    col: usize,
    row: usize,
) -> Option<InboxPane> {
    let items = inbox_items(app);
    let selected = app
        .inbox_overlay
        .selected
        .min(items.len().saturating_sub(1));
    let selected_default = items.get(selected).map(|_| format!("{:02}", selected + 1));
    let footer = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.inbox_overlay.jump_input,
    };
    let filter_line = format!(
        "Filter:{}  Year:{}  Focus:{}{}",
        app.inbox_overlay.filter.label(),
        if app.inbox_overlay.current_year_only {
            "Current"
        } else {
            "All"
        },
        match app.inbox_overlay.focus {
            InboxFocus::List => "List",
            InboxFocus::Preview => "Preview",
        },
        if app.inbox_overlay.delete_confirm {
            "  Delete this item? Y/[N]"
        } else {
            ""
        }
    );
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = inbox_body_width(&filter_line).min(max_overlay_body_width_for_parent(parent));
    let natural_content_rows = inbox_natural_content_rows(&items, selected, body_width);
    let popup = overlay_popup_rect_for_body_in_parent(
        parent,
        "INBOX",
        body_width,
        1 + standard_table_body_height(natural_content_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );
    let body_col = popup.x as usize + 2;
    let body_row = popup.y as usize + 1;
    let body_height = popup
        .height
        .saturating_sub(overlay_chrome_height(footer) as u16) as usize;
    if row < body_row || row >= body_row + body_height || col < body_col {
        return None;
    }
    let visible_rows = body_height.saturating_sub(1 + standard_table_body_height(0));
    let list_rows = inbox_list_rows(&items);
    let list_scrollable = list_rows.len() > visible_rows;
    let pane = inbox_pane_layout_for_scrollbars(body_width, list_scrollable, false);
    let preview_rows = preview_rows(
        items.get(selected),
        preview_content_width(pane.preview_table_width),
    );
    let pane = inbox_pane_layout_for_scrollbars(
        body_width,
        list_scrollable,
        preview_rows.len() > visible_rows,
    );
    if pane.preview_table_width == 0 || col < body_col + pane.preview_offset {
        Some(InboxPane::List)
    } else {
        Some(InboxPane::Preview)
    }
}

pub fn inbox_items(app: &DashApp) -> Vec<DashInboxItem> {
    let viewer = app.player_record_index_1_based as u8;
    let current_year = app.game_data.conquest.game_year();
    project_inbox_items(
        &app.game_data,
        viewer,
        &app.report_block_rows,
        &app.queued_mail,
    )
    .into_iter()
    .filter(|item| {
        matches_filter(
            item,
            app.inbox_overlay.filter,
            app.inbox_overlay.current_year_only,
            current_year,
        )
    })
    .collect()
}

pub fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    inbox_items(app)
        .into_iter()
        .enumerate()
        .map(|(idx, _)| vec![format!("{:02}", idx + 1)])
        .collect()
}

fn clamp_scroll(scroll: usize, selected: usize, max_rows: usize, total_rows: usize) -> usize {
    if max_rows == 0 || total_rows <= max_rows {
        return 0;
    }
    if selected < scroll {
        return selected;
    }
    if selected >= scroll + max_rows {
        return selected + 1 - max_rows;
    }
    scroll.min(total_rows.saturating_sub(max_rows))
}

fn inbox_body_width(filter_line: &str) -> usize {
    filter_line.chars().count().max(
        INBOX_LIST_TARGET_TABLE_WIDTH
            + TABLE_SCROLL_GUTTER_WIDTH
            + INBOX_TABLE_GAP_WIDTH
            + INBOX_PREVIEW_TARGET_TABLE_WIDTH
            + TABLE_SCROLL_GUTTER_WIDTH,
    )
}

fn max_overlay_body_width_for_parent(parent: Rect) -> usize {
    parent.width.saturating_sub(6).max(1) as usize
}

fn inbox_pane_layout(body_width: usize) -> InboxPaneLayout {
    inbox_pane_layout_for_scrollbars(body_width, true, true)
}

fn inbox_pane_layout_for_scrollbars(
    body_width: usize,
    list_scrollable: bool,
    preview_scrollable: bool,
) -> InboxPaneLayout {
    let list_gutter = usize::from(list_scrollable) * TABLE_SCROLL_GUTTER_WIDTH;
    let preview_gutter = usize::from(preview_scrollable) * TABLE_SCROLL_GUTTER_WIDTH;
    let minimum_split_width = INBOX_LIST_MIN_TABLE_WIDTH
        + list_gutter
        + INBOX_TABLE_GAP_WIDTH
        + INBOX_PREVIEW_MIN_TABLE_WIDTH
        + preview_gutter;
    if body_width < minimum_split_width {
        return InboxPaneLayout {
            list_table_width: body_width.saturating_sub(list_gutter),
            list_scroll_gutter_width: list_gutter.min(body_width),
            preview_offset: body_width,
            preview_table_width: 0,
            preview_scroll_gutter_width: 0,
        };
    }

    let available_table_width =
        body_width.saturating_sub(list_gutter + INBOX_TABLE_GAP_WIDTH + preview_gutter);
    let (list_table_width, preview_table_width) = if available_table_width
        >= INBOX_LIST_TARGET_TABLE_WIDTH + INBOX_PREVIEW_TARGET_TABLE_WIDTH
    {
        (
            INBOX_LIST_TARGET_TABLE_WIDTH,
            available_table_width - INBOX_LIST_TARGET_TABLE_WIDTH,
        )
    } else if available_table_width >= INBOX_LIST_MIN_TABLE_WIDTH + INBOX_PREVIEW_TARGET_TABLE_WIDTH
    {
        (
            available_table_width - INBOX_PREVIEW_TARGET_TABLE_WIDTH,
            INBOX_PREVIEW_TARGET_TABLE_WIDTH,
        )
    } else {
        let list_width = INBOX_LIST_MIN_TABLE_WIDTH
            .min(available_table_width.saturating_sub(INBOX_PREVIEW_MIN_TABLE_WIDTH));
        (list_width, available_table_width.saturating_sub(list_width))
    };
    let preview_offset = list_table_width + list_gutter + INBOX_TABLE_GAP_WIDTH;
    InboxPaneLayout {
        list_table_width,
        list_scroll_gutter_width: list_gutter,
        preview_offset,
        preview_table_width,
        preview_scroll_gutter_width: preview_gutter,
    }
}

fn preview_content_width(preview_table_width: usize) -> usize {
    preview_table_width.saturating_sub(2).max(1)
}

fn inbox_natural_content_rows(
    items: &[DashInboxItem],
    selected: usize,
    body_width: usize,
) -> usize {
    let layout = inbox_pane_layout(body_width);
    let preview_rows = if layout.preview_table_width > 0 {
        preview_rows(
            items.get(selected),
            preview_content_width(layout.preview_table_width),
        )
        .len()
    } else {
        0
    };
    items.len().max(1).max(preview_rows.max(1))
}

fn outbox_natural_content_rows(
    messages: &[nc_data::TurnMessage],
    selected: usize,
    game_data: &nc_data::CoreGameData,
    body_width: usize,
) -> usize {
    let layout = inbox_pane_layout(body_width);
    let preview_rows = if layout.preview_table_width > 0 {
        outbox_preview_rows(
            messages.get(selected),
            preview_content_width(layout.preview_table_width),
        )
        .len()
    } else {
        0
    };
    outbox_rows(game_data, messages)
        .len()
        .max(1)
        .max(preview_rows.max(1))
}

fn inbox_list_columns(table_width: usize) -> Vec<TableColumn<'static>> {
    columns_for_width(
        table_width,
        &[
            ("ID", 2, TableAlign::Right),
            ("Type", 4, TableAlign::Left),
            ("Year", 4, TableAlign::Right),
            ("Subject", usize::MAX, TableAlign::Left),
        ],
    )
}

fn preview_columns(table_width: usize) -> Vec<TableColumn<'static>> {
    vec![TableColumn {
        header: "Preview",
        width: table_width.saturating_sub(2),
        align: TableAlign::Left,
        flex: 0,
    }]
}

fn outbox_columns(table_width: usize) -> Vec<TableColumn<'static>> {
    columns_for_width(
        table_width,
        &[
            ("ID", 2, TableAlign::Right),
            ("Emp#", 4, TableAlign::Right),
            ("Empire", 12, TableAlign::Left),
            ("Year", 4, TableAlign::Right),
            ("Subject", usize::MAX, TableAlign::Left),
        ],
    )
}

fn columns_for_width(
    table_width: usize,
    specs: &[(&'static str, usize, TableAlign)],
) -> Vec<TableColumn<'static>> {
    let separators = specs.len() + 1;
    let mut remaining = table_width.saturating_sub(separators);
    specs
        .iter()
        .enumerate()
        .map(|(idx, (header, preferred_width, align))| {
            let last = idx + 1 == specs.len();
            let width = if last {
                remaining
            } else {
                let width = remaining.min(*preferred_width);
                remaining -= width;
                width
            };
            TableColumn {
                header,
                width,
                align: *align,
                flex: 0,
            }
        })
        .collect()
}

fn inbox_list_rows(items: &[DashInboxItem]) -> Vec<Vec<String>> {
    if items.is_empty() {
        return vec![vec![
            String::new(),
            String::new(),
            String::new(),
            "No inbox messages.".to_string(),
        ]];
    }
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            vec![
                format!("{:02}", idx + 1),
                item.item_type.code().to_string(),
                item.year.to_string(),
                item.subject.clone(),
            ]
        })
        .collect()
}

fn preview_rows(item: Option<&DashInboxItem>, width: usize) -> Vec<Vec<String>> {
    let lines = item
        .map(|item| {
            if item.body_lines.is_empty() {
                vec!["(no body)".to_string()]
            } else {
                item.body_lines.clone()
            }
        })
        .unwrap_or_else(|| vec!["No message selected.".to_string()]);
    wrap_preview_lines(lines, width)
}

fn outbox_preview_rows(message: Option<&nc_data::TurnMessage>, width: usize) -> Vec<Vec<String>> {
    let lines = message
        .map(|message| {
            if message.body.is_empty() {
                vec!["(no body)".to_string()]
            } else {
                message.body.lines().map(str::to_string).collect::<Vec<_>>()
            }
        })
        .unwrap_or_else(|| vec!["No message selected.".to_string()]);
    wrap_preview_lines(lines, width)
}

fn wrap_preview_lines(lines: Vec<String>, width: usize) -> Vec<Vec<String>> {
    wrap_modal_text_lines(&lines, width)
        .into_iter()
        .map(|line| vec![line])
        .collect()
}

fn outbox_rows(
    game_data: &nc_data::CoreGameData,
    messages: &[nc_data::TurnMessage],
) -> Vec<Vec<String>> {
    let current_year = game_data.conquest.game_year();
    if messages.is_empty() {
        return vec![vec![
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "No staged outgoing messages.".to_string(),
        ]];
    }
    messages
        .iter()
        .enumerate()
        .map(|(idx, message)| {
            let emp_id = message.recipient_empire_raw;
            let emp_name = app_compose_empire_name(game_data, emp_id);
            vec![
                format!("{:02}", idx + 1),
                format!("{:02}", emp_id),
                emp_name,
                current_year.to_string(),
                message.subject.clone(),
            ]
        })
        .collect()
}

fn app_compose_empire_name(game_data: &nc_data::CoreGameData, empire_id: u8) -> String {
    let Some(player) = game_data
        .player
        .records
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id:02}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    if !name.is_empty() {
        name
    } else if !fallback.is_empty() {
        fallback
    } else {
        format!("Empire {empire_id:02}")
    }
}

fn inbox_list_row_states(empty: bool, len: usize) -> Vec<TableRowState> {
    vec![
        if empty {
            TableRowState::Disabled
        } else {
            TableRowState::Normal
        };
        len
    ]
}

fn clamp_offset(scroll: usize, visible_rows: usize, total_rows: usize) -> usize {
    if visible_rows == 0 || total_rows <= visible_rows {
        0
    } else {
        scroll.min(total_rows.saturating_sub(visible_rows))
    }
}

#[cfg(test)]
mod tests {
    use super::{HOTKEYS, inbox_pane_layout, inbox_pane_layout_for_scrollbars};
    use crate::dashboard::app::render;
    use crate::dashboard::app::state::{ActiveOverlay, DashApp, InboxPromptMode};
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::{GameStateBuilder, QueuedPlayerMail, ReportBlockRow, TurnMessage};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_inbox_commands() {
        assert_eq!(HOTKEYS, "? M I A Y D C O <ESC>");
    }

    #[test]
    fn inbox_pane_layout_keeps_target_widths_when_space_allows() {
        let layout = inbox_pane_layout(121);

        assert_eq!(layout.list_table_width, 38);
        assert_eq!(layout.list_scroll_gutter_width, 1);
        assert_eq!(layout.preview_offset, 40);
        assert_eq!(layout.preview_table_width, 80);
        assert_eq!(layout.preview_scroll_gutter_width, 1);
    }

    #[test]
    fn inbox_pane_layout_shrinks_list_to_preserve_preview_table() {
        let layout = inbox_pane_layout(114);

        assert_eq!(layout.list_table_width, 31);
        assert_eq!(layout.list_scroll_gutter_width, 1);
        assert_eq!(layout.preview_offset, 33);
        assert_eq!(layout.preview_table_width, 80);
        assert_eq!(layout.preview_scroll_gutter_width, 1);
    }

    #[test]
    fn inbox_pane_layout_reclaims_unused_scroll_gutters() {
        let layout = inbox_pane_layout_for_scrollbars(119, false, false);

        assert_eq!(layout.list_table_width, 38);
        assert_eq!(layout.list_scroll_gutter_width, 0);
        assert_eq!(layout.preview_offset, 39);
        assert_eq!(layout.preview_table_width, 80);
        assert_eq!(layout.preview_scroll_gutter_width, 0);
    }

    #[test]
    fn inbox_pane_layout_shrinks_preview_after_list_minimum() {
        let layout = inbox_pane_layout(40);

        assert_eq!(layout.list_table_width, 22);
        assert_eq!(layout.list_scroll_gutter_width, 1);
        assert_eq!(layout.preview_offset, 24);
        assert_eq!(layout.preview_table_width, 15);
        assert_eq!(layout.preview_scroll_gutter_width, 1);
    }

    #[test]
    fn inbox_pane_layout_drops_preview_when_split_would_not_fit() {
        let layout = inbox_pane_layout(20);

        assert_eq!(layout.list_table_width, 19);
        assert_eq!(layout.list_scroll_gutter_width, 1);
        assert_eq!(layout.preview_offset, 20);
        assert_eq!(layout.preview_table_width, 0);
        assert_eq!(layout.preview_scroll_gutter_width, 0);
    }

    #[test]
    fn inbox_overlay_clamps_to_map_body_width_without_panicking() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(25)
                .with_year(3013)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            vec![ReportBlockRow {
                viewer_empire_id: 0,
                block_index: 0,
                decoded_text: "Stardate: 03/3012\nThis is an intentionally very long inbox preview line that should exceed the modal natural width on an X13-sized terminal but still render safely inside the centered split-pane popup.".to_string(),
                raw_bytes: None,
                recipient_deleted: false,
            }],
            vec![QueuedPlayerMail {
                sender_empire_id: 2,
                recipient_empire_id: 1,
                year: 3012,
                subject: "Long diplomatic subject that also stretches the inbox list width".to_string(),
                body: "Second very long preview line to keep the selected preview pane wide enough to trigger clamping instead of a panic.".to_string(),
                recipient_deleted: false,
            }],
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(108, 26),
            1,
        );
        app.overlay = ActiveOverlay::Inbox;

        let buffer = render::render(&app).expect("render inbox overlay");
        let lines = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>();

        assert!(lines.iter().any(|line| line.contains("INBOX")));
        assert!(lines.iter().any(|line| line.contains("Preview")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("│ID│Type│Year│Subject"))
        );
    }

    #[test]
    fn inbox_overlay_renders_empty_state_inside_table() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(25)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(108, 26),
            1,
        );
        app.overlay = ActiveOverlay::Inbox;

        let buffer = render::render(&app).expect("render empty inbox overlay");
        let lines = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>();

        assert!(
            lines
                .iter()
                .any(|line| line.contains("│ID│Type│Year│Subject"))
        );
        assert!(lines.iter().any(|line| line.contains("No inbox messages.")));
        assert!(!lines.iter().any(|line| line.contains("(empty)")));
    }

    #[test]
    fn outbox_overlay_renders_bordered_table() {
        let mut app = inbox_test_app();
        app.initialize_hosted_turn_draft();
        app.hosted_turn_draft
            .as_mut()
            .expect("draft")
            .messages
            .push(TurnMessage {
                recipient_empire_raw: 2,
                subject: "Move".to_string(),
                body: "Fleet moves".to_string(),
            });
        app.overlay = ActiveOverlay::Inbox;
        app.inbox_overlay.prompt_mode = InboxPromptMode::Outbox;

        let buffer = render::render(&app).expect("render outbox overlay");
        let lines = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>();

        assert!(lines.iter().any(|line| line.contains("OUTBOX")));
        assert!(lines.iter().any(|line| line.contains("│ID│Emp#│Empire")));
        assert!(lines.iter().any(|line| line.contains("Move")));
    }

    #[test]
    fn outbox_overlay_renders_recipient_empire_name() {
        let mut app = inbox_test_app();
        app.game_data.player.records[1].set_controlled_empire_name_raw("Rigel Empire");
        app.initialize_hosted_turn_draft();
        app.hosted_turn_draft
            .as_mut()
            .expect("draft")
            .messages
            .push(TurnMessage {
                recipient_empire_raw: 2,
                subject: "Move".to_string(),
                body: "Fleet moves".to_string(),
            });
        app.overlay = ActiveOverlay::Inbox;
        app.inbox_overlay.prompt_mode = InboxPromptMode::Outbox;

        let buffer = render::render(&app).expect("render outbox overlay");
        let lines = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>();

        assert!(lines.iter().any(|line| line.contains("Rigel Empire")));
    }

    #[test]
    fn inbox_preview_wraps_long_body_lines() {
        let mut app = inbox_test_app();
        app.queued_mail = vec![QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3012,
            subject: "Wrap".to_string(),
            body: "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega".to_string(),
            recipient_deleted: false,
        }];
        app.report_block_rows.clear();
        app.overlay = ActiveOverlay::Inbox;

        let buffer = render::render(&app).expect("render inbox overlay");
        let lines = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>();

        assert!(lines.iter().any(|line| line.contains("alpha beta gamma")));
        assert!(lines.iter().any(|line| line.contains("omega")));
    }

    #[test]
    fn inbox_modal_width_stays_stable_across_selected_messages() {
        let app = inbox_test_app();

        let mut short_selected = app;
        short_selected.overlay = ActiveOverlay::Inbox;
        short_selected.inbox_overlay.selected = 0;

        let mut long_selected = inbox_test_app();
        long_selected.overlay = ActiveOverlay::Inbox;
        long_selected.inbox_overlay.selected = 1;

        let short_buffer = render::render(&short_selected).expect("render short inbox");
        let long_buffer = render::render(&long_selected).expect("render long inbox");

        let short_title_row = find_line(&short_buffer, "INBOX");
        let long_title_row = find_line(&long_buffer, "INBOX");
        assert_eq!(
            short_buffer.plain_line(short_title_row),
            long_buffer.plain_line(long_title_row)
        );
    }

    fn inbox_test_app() -> DashApp {
        DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(25)
                .with_year(3013)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            vec![
                ReportBlockRow {
                    viewer_empire_id: 0,
                    block_index: 0,
                    decoded_text: "Stardate: 03/3012\nShort line.".to_string(),
                    raw_bytes: None,
                    recipient_deleted: false,
                },
                ReportBlockRow {
                    viewer_empire_id: 0,
                    block_index: 1,
                    decoded_text: "Stardate: 04/3012\nThis is an intentionally very long inbox preview line that should exceed the fixed preview width target but must not change the centered modal width when selected.".to_string(),
                    raw_bytes: None,
                    recipient_deleted: false,
                },
            ],
            vec![QueuedPlayerMail {
                sender_empire_id: 2,
                recipient_empire_id: 1,
                year: 3012,
                subject: "Long diplomatic subject that also stretches the inbox list width".to_string(),
                body: "Second very long preview line to keep the selected preview pane wide enough to trigger clipping without resizing.".to_string(),
                recipient_deleted: false,
            }],
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn find_line(buffer: &crate::dashboard::buffer::PlayfieldBuffer, needle: &str) -> usize {
        (0..buffer.height())
            .find(|row| buffer.plain_line(*row).contains(needle))
            .expect("line containing needle")
    }
}
