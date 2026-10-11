//! Left panel: Treasury, production, revenue, and generated PP growth.

use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use nc_data::yearly_growth_delta;

use crate::dashboard::app::state::DashApp;
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::theme;

pub(crate) const TITLE: &str = "ECONOMY";
pub(crate) const MIN_BODY_ROWS: usize = 6;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    for (row_idx, (text, style)) in body_rows(app).into_iter().enumerate() {
        if row_idx >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, row_idx, &text, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.players.get(player_idx) else {
        return Vec::new();
    };
    let tax = player.tax_rate;
    let owner_slot = app.player_record_index_1_based as u8;

    let mut total_treasury: u32 = 0;
    let mut total_present: u32 = 0;
    let mut total_potential: u32 = 0;
    for planet in &app.game_data.planets {
        if planet.owner.0 != owner_slot {
            continue;
        }
        total_treasury += nc_data::stored_production_points(&planet);
        total_present += nc_data::present_production_points(&planet) as u32;
        total_potential += u16::from(planet.potential_production) as u32;
    }

    let economy = app
        .game_data
        .empire_economy_summary(app.player_record_index_1_based);

    let growth: i32 = if total_present < total_potential {
        yearly_growth_delta(total_present as u16, total_potential as u16, tax, false) as i32
    } else {
        0
    };
    let growth_percent = if total_present == 0 {
        String::from("?")
    } else {
        format!(
            "{:.1}%",
            (f64::from(growth.max(0) as u16) / f64::from(total_present as u16)) * 100.0
        )
    };
    let gs = if growth > 0 {
        theme::friendly_style()
    } else if growth < 0 {
        theme::enemy_style()
    } else {
        theme::dim_style()
    };

    vec![
        (
            layout::format_left_column_value("Treasury", &total_treasury.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value(
                "Emp Rev",
                &economy.total_available_points.to_string(),
            ),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Prod", &total_present.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Pot Prod", &total_potential.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Tax Rate", &format!("{tax}%")),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("PP Gen", &format!("{growth:+}")),
            gs,
        ),
        (
            layout::format_left_column_value("% Growth", &growth_percent),
            gs,
        ),
        (
            layout::format_left_column_value(
                "Efficiency",
                &format!("{:.1}%", economy.efficiency_percent),
            ),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value(
                "Prod Rank",
                &format!("#{}", economy.rank_by_present_production),
            ),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Plnt Rank", &format!("#{}", economy.rank_by_planets)),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value(
                "Cmd Limit",
                &format_cmd_limit(
                    economy.current_fleets_and_bases,
                    economy.max_fleets_and_bases,
                ),
            ),
            theme::value_style(),
        ),
    ]
}

fn format_cmd_limit(current: usize, max: usize) -> String {
    format!("{current:03}|{max:03}")
}

#[cfg(test)]
mod tests {
    use super::{body_rows, format_cmd_limit};
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn command_limit_uses_zero_padded_pipe_format() {
        assert_eq!(format_cmd_limit(4, 500), "004|500");
    }

    #[test]
    fn economy_rows_render_compact_cmd_limit() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        assert!(
            body_rows(&app)
                .iter()
                .any(|(row, _)| row.contains("Cmd Limit") && row.contains('|'))
        );
    }
}
