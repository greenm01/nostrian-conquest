use crate::dashboard::app::state::{ActivePopup, DashApp};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::{self, MapWidgetFrame, dashboard};
use crate::dashboard::modal::{Rect, compact_content_width, wrap_modal_text_lines};
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::dashboard::table::{TableFooter, with_command_line_toast};
use crate::dashboard::theme;

const TITLE: &str = "TAX RATE";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let lines = popup_lines(app, compact_content_width(parent));
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        TITLE,
        max_line_width(&lines),
        lines.len(),
        OverlaySizePolicy::default(),
        footer(app),
        app.popup_position_for(ActivePopup::TaxPrompt),
    );
    for (idx, line) in lines.into_iter().enumerate().take(frame.body_height) {
        layout::write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            &line.content,
            line.style,
        );
    }
}

pub fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let lines = popup_lines(app, compact_content_width(parent));
    overlay_popup_rect_for_body_in_parent(
        parent,
        TITLE,
        max_line_width(&lines),
        lines.len(),
        OverlaySizePolicy::default(),
        footer(app),
        app.popup_position_for(ActivePopup::TaxPrompt),
    )
}

fn footer<'a>(app: &'a DashApp) -> TableFooter<'a> {
    with_command_line_toast(
        TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Tax rate -> ",
            input: &app.tax_prompt_input,
        },
        app.active_command_line_toast(),
    )
}

fn current_tax(app: &DashApp) -> u8 {
    app.game_data
        .players
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|player| player.tax_rate())
        .unwrap_or(0)
}

struct PopupLine {
    content: String,
    style: crate::dashboard::buffer::CellStyle,
}

fn popup_lines(app: &DashApp, max_width: usize) -> Vec<PopupLine> {
    let tax = current_tax(app);
    let mut raw = vec![
        (
            format!("Current empire tax rate: {tax}%"),
            theme::value_style(),
        ),
        (
            String::from("Enter 0-100. Rates above 65% can damage production."),
            theme::value_style(),
        ),
    ];
    if let Some(status) = app
        .tax_prompt_status
        .as_deref()
        .filter(|status| !status.is_empty())
    {
        raw.push((status.to_string(), theme::alert_style()));
    }
    let mut wrapped = Vec::new();
    for (line, style) in raw {
        for content in wrap_modal_text_lines(&[line], max_width.max(1)) {
            wrapped.push(PopupLine { content, style });
        }
    }
    if wrapped.is_empty() {
        vec![PopupLine {
            content: String::new(),
            style: theme::value_style(),
        }]
    } else {
        wrapped
    }
}

fn max_line_width(lines: &[PopupLine]) -> usize {
    lines
        .iter()
        .map(|line| line.content.chars().count())
        .max()
        .unwrap_or(1)
        .max(1)
}
