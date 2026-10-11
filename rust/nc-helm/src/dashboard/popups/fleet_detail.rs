//! Fleet info popup rendered over the fleet list caller.

use crate::dashboard::app::state::{ActivePopup, DashApp, FleetOverlayRowKey};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::coords::format_sector_coords_table;
use crate::dashboard::layout::{self, MapWidgetFrame, dashboard};
use crate::dashboard::modal::{Rect, max_content_width};
use crate::dashboard::overlays::fleet_list;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::dashboard::planet_view::DetailLine;
use crate::dashboard::popups::planet_detail;
use crate::dashboard::table::TableFooter;
use crate::dashboard::theme;

pub fn draw(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    _map_frame: MapWidgetFrame,
    fleet_record_index_1_based: usize,
) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let max_body_width = max_content_width(parent);
    let title = popup_title(app, fleet_record_index_1_based);
    let lines = popup_lines(app, fleet_record_index_1_based, max_body_width);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let popup = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        &title,
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::None,
        app.popup_position_for(ActivePopup::FleetDetail {
            fleet_record_index_1_based,
        }),
    );
    for (idx, line) in lines.into_iter().enumerate().take(popup.body_height) {
        layout::write_clipped(
            buf,
            popup.body_row + idx,
            popup.body_col,
            popup.body_width,
            &line,
            theme::value_style(),
        );
    }
}

pub fn popup_rect(
    app: &DashApp,
    _map_frame: MapWidgetFrame,
    fleet_record_index_1_based: usize,
) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let max_body_width = max_content_width(parent);
    let title = popup_title(app, fleet_record_index_1_based);
    let lines = popup_lines(app, fleet_record_index_1_based, max_body_width);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    overlay_popup_rect_for_body_in_parent(
        parent,
        &title,
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::None,
        app.popup_position_for(ActivePopup::FleetDetail {
            fleet_record_index_1_based,
        }),
    )
}

fn popup_lines(
    app: &DashApp,
    fleet_record_index_1_based: usize,
    max_body_width: usize,
) -> Vec<String> {
    let Some(lines) = detail_lines(app, fleet_record_index_1_based) else {
        return vec!["Selected fleet is no longer available.".to_string()];
    };
    planet_detail::popup_lines(&lines, max_body_width)
}

fn detail_lines(app: &DashApp, fleet_record_index_1_based: usize) -> Option<Vec<DetailLine>> {
    let fleet = app
        .game_data
        .fleets
        .get(fleet_record_index_1_based.checked_sub(1)?)?;
    let row = fleet_list::table_rows(app)
        .into_iter()
        .find(|row| row.key == FleetOverlayRowKey::Fleet(fleet_record_index_1_based))?;
    let mut lines = vec![
        DetailLine {
            label: "Location",
            value: format_sector_coords_table(row.coords),
        },
        DetailLine {
            label: "Speed",
            value: row.current_speed.to_string(),
        },
        DetailLine {
            label: "Max Speed",
            value: fleet.max_speed.to_string(),
        },
        DetailLine {
            label: "ROE",
            value: row.roe.to_string(),
        },
        DetailLine {
            label: "Orders",
            value: fleet_list::fleet_table_order_label(row.order).to_string(),
        },
        DetailLine {
            label: "Target",
            value: format_sector_coords_table(row.target_coords),
        },
        DetailLine {
            label: "Ships",
            value: nc_data::ship_composition_table_summary(&fleet),
        },
        DetailLine {
            label: "Armies",
            value: row.loaded_armies.to_string(),
        },
    ];
    if !fleet.crippled.is_empty() {
        // Crippled ships by class, after the composition they are part of.
        lines.insert(
            7,
            DetailLine {
                label: "Damaged",
                value: nc_data::damaged_ships_table_summary(&fleet),
            },
        );
    }
    Some(lines)
}

fn popup_title(app: &DashApp, fleet_record_index_1_based: usize) -> String {
    let Some(row) = fleet_list::table_rows(app)
        .into_iter()
        .find(|row| row.key == FleetOverlayRowKey::Fleet(fleet_record_index_1_based))
    else {
        return "REVIEW FLEET".to_string();
    };
    match row.id_label.is_empty() {
        true => "REVIEW FLEET".to_string(),
        false => format!("REVIEW FLEET {}", row.id_label),
    }
}
