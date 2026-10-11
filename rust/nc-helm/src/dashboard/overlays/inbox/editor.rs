use crate::dashboard::app::state::{ActiveOverlay, DashApp};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::Rect;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent, write_clipped,
};
use crate::dashboard::table::TableFooter;
use crate::dashboard::theme;

pub(crate) const COMPOSE_SUBJECT_LIMIT: usize = 60;
pub(crate) const COMPOSE_BODY_LIMIT: usize = 1000;
pub(crate) const COMPOSE_BODY_WRAP_WIDTH: usize = 79;
const TITLE: &str = "COMMUNICATE (SEND MESSAGE)";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let footer = TableFooter::CommandBar {
        hotkeys_markup: "? Alt-S Alt-X",
        default: None,
        input: "",
    };

    let body_width = COMPOSE_BODY_WRAP_WIDTH;
    let body_height = (parent.height as usize).saturating_sub(10).max(5).min(20);

    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        TITLE,
        body_width,
        body_height + 4, // Space for headers
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Inbox),
    );

    let recipient_name = app
        .inbox_overlay
        .compose_recipient_empire
        .and_then(|id| {
            app.game_data
                .players
                .get(id as usize - 1)
                .map(|p| p.empire_name.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        &format!("To: {recipient_name}"),
        theme::label_style(),
    );
    write_clipped(
        buf,
        frame.body_row + 1,
        frame.body_col,
        frame.body_width,
        &format!("Subject: {}", app.inbox_overlay.compose_subject),
        theme::label_style(),
    );
    write_clipped(
        buf,
        frame.body_row + 2,
        frame.body_col,
        frame.body_width,
        "-------------------------------------------------------------------------------",
        theme::border_style(),
    );

    let wrapped = wrap_body_segments(&app.inbox_overlay.compose_body, body_width);
    let first_editor_row = frame.body_row + 3;
    let total_rows = wrapped
        .len()
        .max(app.inbox_overlay.compose_body_cursor_row + 1);
    let start = visible_window_start(
        total_rows,
        body_height,
        app.inbox_overlay.compose_body_cursor_row,
    );

    for (idx, segment) in wrapped.iter().skip(start).take(body_height).enumerate() {
        write_clipped(
            buf,
            first_editor_row + idx,
            frame.body_col,
            frame.body_width,
            &segment.text,
            theme::body_style(),
        );
    }

    let char_count = app.inbox_overlay.compose_body.chars().count();
    write_clipped(
        buf,
        frame.body_row + 3 + body_height,
        frame.body_col,
        frame.body_width,
        &format!("Chars: {}/{}", char_count, COMPOSE_BODY_LIMIT),
        theme::dim_style(),
    );

    let render_row = first_editor_row
        + app
            .inbox_overlay
            .compose_body_cursor_row
            .saturating_sub(start);
    buf.set_cursor(
        (frame.body_col + app.inbox_overlay.compose_body_cursor_col) as u16,
        render_row as u16,
    );
}

pub fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_height = (parent.height as usize).saturating_sub(10).max(5).min(20);
    overlay_popup_rect_for_body_in_parent(
        parent,
        TITLE,
        COMPOSE_BODY_WRAP_WIDTH,
        body_height + 4,
        OverlaySizePolicy::default(),
        TableFooter::CommandBar {
            hotkeys_markup: "? Alt-S Alt-X",
            default: None,
            input: "",
        },
        app.overlay_position_for(ActiveOverlay::Inbox),
    )
}

#[derive(Debug, Clone)]
pub(crate) struct WrappedSegment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
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

fn visible_window_start(total: usize, visible: usize, cursor_row: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let max_start = total - visible;
    cursor_row
        .saturating_sub(visible.saturating_sub(1))
        .min(max_start)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ComposeCursor {
    pub(crate) row: usize,
    pub(crate) col: usize,
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

pub(crate) fn char_to_byte_index(body: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    body.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(body.len())
}
