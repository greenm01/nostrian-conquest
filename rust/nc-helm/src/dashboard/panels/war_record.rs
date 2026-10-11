//! Left panel: campaign-lifetime war and expansion summary.

use crate::dashboard::app::state::DashApp;
use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::theme;

pub(crate) const TITLE: &str = "WAR RECORD";
pub(crate) const MIN_BODY_ROWS: usize = 5;

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
    let stats = app.player_war_stats;
    vec![
        (
            layout::format_left_column_value("Colonies", &stats.colonies_established.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Taken", &stats.worlds_taken.to_string()),
            if stats.worlds_taken > 0 {
                theme::friendly_style()
            } else {
                theme::dim_style()
            },
        ),
        (
            layout::format_left_column_value("Lost", &stats.worlds_lost.to_string()),
            if stats.worlds_lost > 0 {
                theme::enemy_style()
            } else {
                theme::dim_style()
            },
        ),
        (
            layout::format_left_column_value("Bombards", &stats.bombardments_launched.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value(
                "Invade S|F",
                &format!("{}|{}", stats.invade_successes, stats.invade_failures()),
            ),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value(
                "Blitz S|F",
                &format!("{}|{}", stats.blitz_successes, stats.blitz_failures()),
            ),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Repelled", &stats.attacks_repelled.to_string()),
            if stats.attacks_repelled > 0 {
                theme::friendly_style()
            } else {
                theme::dim_style()
            },
        ),
        (
            layout::format_left_column_value(
                "Ships Dest",
                &stats.total_enemy_units_destroyed().to_string(),
            ),
            if stats.total_enemy_units_destroyed() > 0 {
                theme::friendly_style()
            } else {
                theme::dim_style()
            },
        ),
        (
            layout::format_left_column_value("Ships Lost", &stats.total_units_lost().to_string()),
            if stats.total_units_lost() > 0 {
                theme::enemy_style()
            } else {
                theme::dim_style()
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::body_rows;
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn war_record_uses_ship_loss_labels() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let labels = body_rows(&app)
            .into_iter()
            .map(|(row, _)| row)
            .collect::<Vec<_>>();

        assert!(labels.iter().any(|row| row.contains("Ships Dest")));
        assert!(labels.iter().any(|row| row.contains("Ships Lost")));
    }
}
