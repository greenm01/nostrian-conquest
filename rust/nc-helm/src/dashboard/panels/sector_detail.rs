//! Right panel: condensed detail for the currently selected map sector.

use crate::dashboard::buffer::PlayfieldBuffer;

use crate::dashboard::app::state::DashApp;
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::planet_view::{
    EMPTY_SECTOR_LABEL, MIN_BODY_ROWS as PLANET_VIEW_MIN_BODY_ROWS,
    preferred_sector_detail_body_rows, preferred_sector_detail_body_width, rendered_widget_lines,
    selected_planet_detail,
};
use crate::dashboard::theme;

pub(crate) const TITLE: &str = "SECTOR DETAIL";
pub(crate) const MIN_BODY_ROWS: usize = PLANET_VIEW_MIN_BODY_ROWS;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    let Some(detail) = selected_planet_detail(app) else {
        layout::write_panel_body_line(buf, frame, 0, EMPTY_SECTOR_LABEL, theme::dim_style());
        return;
    };

    for (row_idx, line) in
        rendered_widget_lines(&detail.widget_fields, frame.body.width, frame.body.height)
            .into_iter()
            .enumerate()
    {
        layout::write_panel_body_line(buf, frame, row_idx, &line, theme::value_style());
    }
}

pub(crate) fn preferred_body_width(app: &DashApp) -> usize {
    preferred_sector_detail_body_width(app)
        .max(EMPTY_SECTOR_LABEL.chars().count())
        .min(crate::dashboard::planet_view::PREFERRED_BODY_WIDTH_CAP)
}

pub(crate) fn preferred_body_rows(app: &DashApp) -> usize {
    preferred_sector_detail_body_rows(app)
}

#[cfg(test)]
mod tests {
    use super::{preferred_body_rows, preferred_body_width, rendered_widget_lines};
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::buffer::PlayfieldBuffer;
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::layout::dashboard_layout;
    use crate::dashboard::planet_view::{DetailLine, MAX_BODY_ROWS};
    use crate::dashboard::theme;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn widget_rows_keep_top_priority_lines_when_height_is_tight() {
        let rows = vec![
            DetailLine {
                label: "Planet",
                value: String::from("Foo"),
            },
            DetailLine {
                label: "Owner",
                value: String::from("You"),
            },
            DetailLine {
                label: "State",
                value: String::from("Normal"),
            },
            DetailLine {
                label: "Production",
                value: String::from("9"),
            },
            DetailLine {
                label: "Potential Production",
                value: String::from("10"),
            },
            DetailLine {
                label: "Treasury",
                value: String::from("8"),
            },
            DetailLine {
                label: "Armies",
                value: String::from("1"),
            },
            DetailLine {
                label: "Ground Batteries",
                value: String::from("2"),
            },
            DetailLine {
                label: "Starbases",
                value: String::from("3"),
            },
            DetailLine {
                label: "Orbit",
                value: String::from("Fleet"),
            },
            DetailLine {
                label: "Building",
                value: String::from("Nothing"),
            },
            DetailLine {
                label: "Docked",
                value: String::from("Nothing"),
            },
        ];

        assert_eq!(
            rendered_widget_lines(&rows, 19, 5),
            vec![
                String::from("Planet: Foo"),
                String::from("Owner: You"),
                String::from("State: Normal"),
                String::from("Production: 9"),
                String::from("Pot Prod: 10"),
            ]
        );
    }

    #[test]
    fn empty_sector_uses_compact_copy() {
        let mut app = DashApp::new_for_tests(
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
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        );
        let occupied = app
            .game_data
            .planets
            .iter()
            .map(|planet| planet.coords_raw())
            .collect::<std::collections::BTreeSet<_>>();
        let empty = (1..=18)
            .flat_map(|y| (1..=18).map(move |x| [x, y]))
            .find(|coords| !occupied.contains(coords))
            .expect("empty sector");
        app.crosshair_x = empty[0];
        app.crosshair_y = empty[1];
        let widgets = dashboard_layout(&app).widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        super::draw(&mut buffer, &app, widgets.right_sector_detail);

        assert!(
            buffer
                .plain_line(widgets.right_sector_detail.body.row)
                .contains("empty sector")
        );
    }

    #[test]
    fn preferred_width_covers_empty_and_world_states() {
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
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        );

        assert!(preferred_body_width(&app) >= "empty sector".chars().count());
        assert_eq!(MAX_BODY_ROWS, 16);
        assert!(preferred_body_rows(&app) <= MAX_BODY_ROWS);
    }

    #[test]
    fn visible_sector_rows_do_not_pad_labels_to_align_colons() {
        let rows = vec![
            DetailLine {
                label: "Planet",
                value: String::from("Aurora"),
            },
            DetailLine {
                label: "Owner",
                value: String::from("You"),
            },
            DetailLine {
                label: "State",
                value: String::from("Stable"),
            },
            DetailLine {
                label: "Production",
                value: String::from("98"),
            },
            DetailLine {
                label: "Potential Production",
                value: String::from("120"),
            },
        ];
        let formatted = rendered_widget_lines(&rows, 19, rows.len());

        assert_eq!(formatted[0], "Planet: Aurora");
        assert_eq!(formatted[1], "Owner: You");
        assert_eq!(formatted[2], "State: Stable");
    }

    #[test]
    fn building_and_docked_wrap_on_token_boundaries() {
        let lines = rendered_widget_lines(
            &[DetailLine {
                label: "Building",
                value: String::from("5BB 10CA 4DD 6TT 3ET 2SB"),
            }],
            24,
            MAX_BODY_ROWS,
        );

        assert_eq!(
            lines,
            vec![
                String::from("Building: 5BB 10CA 4DD"),
                String::from("          6TT 3ET 2SB"),
            ]
        );
    }
}
