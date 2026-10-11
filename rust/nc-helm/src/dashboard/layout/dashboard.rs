//! Measured dashboard layout sized from actual widget content.

use crate::dashboard::geometry::ScreenGeometry;

use crate::dashboard::app::state::{DashApp, MapViewMode};
use crate::dashboard::layout::geometry::{
    CELL_WIDTH, MAP_LEFT_PADDING, MAP_RIGHT_PADDING, MAP_VERTICAL_PADDING, ROW_LABEL_COLS,
    dashboard_frame_geometry, minimum_projected_map_height, minimum_required_map_height,
    minimum_required_map_width,
};
use crate::dashboard::layout::widgets::{
    DashboardWidgetFrames, MapWidgetFrame, WidgetRect, frame_offset_for, panel_widget_frame,
};
use crate::dashboard::panels::{
    comms, diplomacy, economy, fleets, known_galaxy, planets, sector_detail, war_record,
};

const LEFT_PANEL_WIDTH_CAP: usize = 20;
const RIGHT_PANEL_WIDTH_CAP: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashboardLayout {
    pub frame: ScreenGeometry,
    pub widgets: DashboardWidgetFrames,
    pub left_width: usize,
    pub right_width: usize,
}

pub fn dashboard_layout(app: &DashApp) -> DashboardLayout {
    let measurements = measure_dashboard(app);
    let frame = match app.map_view_mode {
        MapViewMode::Readable => preferred_readable_frame(app.geometry, measurements),
        MapViewMode::Fill => app.geometry,
    };
    let content_height = frame.height().saturating_sub(6);
    let center_width = frame
        .width()
        .saturating_sub(1 + measurements.left_width + 1 + 1 + measurements.right_width + 1);
    let map_block_width = match app.map_view_mode {
        MapViewMode::Readable => measurements.preferred_center_width.min(center_width),
        MapViewMode::Fill => center_width,
    };
    let map_block_height = content_height;
    let left_rows = allocate_body_rows(
        measurements.left_preferred_rows,
        measurements.left_minimum_rows,
        content_height,
        [0, 1, 2, 3],
    );
    let right_rows = allocate_body_rows(
        measurements.right_preferred_rows,
        measurements.right_minimum_rows,
        content_height,
        [3, 2, 0, 1],
    );

    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count) as usize;
    let widgets = build_widget_frames(
        app.geometry,
        frame,
        content_height,
        measurements.left_width,
        measurements.right_width,
        left_rows,
        right_rows,
        map_block_width,
        map_block_height,
        map_size,
    );

    DashboardLayout {
        frame,
        widgets,
        left_width: measurements.left_width,
        right_width: measurements.right_width,
    }
}

pub fn required_dashboard_frame(app: &DashApp) -> ScreenGeometry {
    let measurements = measure_dashboard(app);

    dashboard_frame_geometry(
        measurements.minimum_center_width,
        measurements.left_width,
        measurements.right_width,
        measurements.minimum_content_height,
    )
}

pub fn preferred_dashboard_frame(app: &DashApp) -> ScreenGeometry {
    let measurements = measure_dashboard(app);
    dashboard_frame_geometry(
        measurements.preferred_center_width,
        measurements.left_width,
        measurements.right_width,
        measurements.preferred_content_height,
    )
}

pub fn layout_canvas_requirement(layout: &DashboardLayout) -> ScreenGeometry {
    let widgets = layout.widgets;
    let max_col = [
        widgets.left_divider_col,
        widgets.right_divider_col,
        widgets.left_economy.outer.last_col(),
        widgets.left_planets.outer.last_col(),
        widgets.left_fleets.outer.last_col(),
        widgets.left_war_record.outer.last_col(),
        widgets.center_map.outer.last_col(),
        widgets.center_map.map_block.last_col(),
        widgets.center_map.grid.last_col(),
        widgets.right_comms.outer.last_col(),
        widgets.right_galaxy.outer.last_col(),
        widgets.right_diplomacy.outer.last_col(),
        widgets.right_sector_detail.outer.last_col(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    let max_row = [
        widgets.outer_bottom,
        widgets.header_bar_row,
        widgets.header_divider_row,
        widgets.footer_divider_row,
        widgets.footer_bar_row,
        widgets.left_economy.outer.last_row(),
        widgets.left_planets.outer.last_row(),
        widgets.left_fleets.outer.last_row(),
        widgets.left_war_record.outer.last_row(),
        widgets.center_map.outer.last_row(),
        widgets.center_map.map_block.last_row(),
        widgets.center_map.grid.last_row(),
        widgets.right_comms.outer.last_row(),
        widgets.right_galaxy.outer.last_row(),
        widgets.right_diplomacy.outer.last_row(),
        widgets.right_sector_detail.outer.last_row(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    ScreenGeometry::new(max_col.saturating_add(1), max_row.saturating_add(1))
}

pub fn dashboard_fits_canvas(canvas: ScreenGeometry, layout: &DashboardLayout) -> bool {
    let required = layout_canvas_requirement(layout);
    required.width() <= canvas.width() && required.height() <= canvas.height()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DashboardMeasurements {
    left_width: usize,
    right_width: usize,
    left_preferred_rows: [usize; 4],
    left_minimum_rows: [usize; 4],
    right_preferred_rows: [usize; 4],
    right_minimum_rows: [usize; 4],
    minimum_center_width: usize,
    preferred_center_width: usize,
    minimum_content_height: usize,
    preferred_content_height: usize,
}

fn measure_dashboard(app: &DashApp) -> DashboardMeasurements {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count) as usize;

    let economy_rows = economy::body_rows(app);
    let planet_rows = planets::body_rows(app);
    let fleet_rows = fleets::body_rows(app);
    let war_rows = war_record::body_rows(app);
    let comms_rows = comms::body_rows(app);
    let galaxy_rows = known_galaxy::body_rows(app);
    let diplomacy_rows = diplomacy::body_rows(app);
    let diplomacy_body_rows = diplomacy_rows.len().max(diplomacy::MIN_BODY_ROWS);
    let sector_detail_rows = sector_detail::preferred_body_rows(app);
    let sector_detail_width = sector_detail::preferred_body_width(app);

    let left_width = cap_panel_width(
        panel_outer_width(economy::TITLE, styled_row_width(&economy_rows)),
        panel_outer_width(planets::TITLE, styled_row_width(&planet_rows)),
        panel_outer_width(fleets::TITLE, styled_row_width(&fleet_rows)),
        panel_outer_width(war_record::TITLE, styled_row_width(&war_rows)),
        LEFT_PANEL_WIDTH_CAP,
    );
    let right_width = cap_panel_width(
        panel_outer_width(comms::TITLE, styled_row_width(&comms_rows)),
        panel_outer_width(known_galaxy::TITLE, styled_row_width(&galaxy_rows)),
        panel_outer_width(diplomacy::TITLE, diplomacy::preferred_body_width(app)),
        panel_outer_width(sector_detail::TITLE, sector_detail_width),
        RIGHT_PANEL_WIDTH_CAP,
    );
    let left_preferred_rows = [
        economy_rows.len(),
        planet_rows.len(),
        fleet_rows.len(),
        war_rows.len(),
    ];
    let left_minimum_rows = [
        economy::MIN_BODY_ROWS.min(left_preferred_rows[0]),
        planets::MIN_BODY_ROWS.min(left_preferred_rows[1]),
        fleets::MIN_BODY_ROWS.min(left_preferred_rows[2]),
        war_record::MIN_BODY_ROWS.min(left_preferred_rows[3]),
    ];
    let right_preferred_rows = [
        comms_rows.len(),
        galaxy_rows.len(),
        diplomacy_body_rows,
        sector_detail_rows,
    ];
    let right_minimum_rows = [
        comms::MIN_BODY_ROWS.min(right_preferred_rows[0]),
        known_galaxy::MIN_BODY_ROWS.min(right_preferred_rows[1]),
        diplomacy::MIN_BODY_ROWS.min(right_preferred_rows[2]),
        sector_detail::MIN_BODY_ROWS.min(right_preferred_rows[3]),
    ];
    let minimum_map_height = minimum_required_map_height(map_size);
    let preferred_map_height = minimum_projected_map_height(map_size);
    let left_min_stack = stack_height_4(left_minimum_rows);
    let left_stack = stack_height_4(left_preferred_rows);
    let right_min_stack = stack_height_4(right_minimum_rows);

    let minimum_content_height = minimum_map_height.max(left_min_stack).max(right_min_stack);
    let preferred_content_height = preferred_map_height
        .max(left_stack)
        .max(stack_height_4(right_preferred_rows));

    // Dynamic snapping logic for "Readable" mode that fills well without uneven gaps.
    let preferred_center_width = ROW_LABEL_COLS + map_size * CELL_WIDTH + 1 + MAP_RIGHT_PADDING;

    DashboardMeasurements {
        left_width,
        right_width,
        left_preferred_rows,
        left_minimum_rows,
        right_preferred_rows,
        right_minimum_rows,
        minimum_center_width: minimum_required_map_width(map_size),
        preferred_center_width,
        minimum_content_height,
        preferred_content_height,
    }
}

fn preferred_readable_frame(
    canvas: ScreenGeometry,
    measurements: DashboardMeasurements,
) -> ScreenGeometry {
    let preferred = dashboard_frame_geometry(
        measurements.preferred_center_width,
        measurements.left_width,
        measurements.right_width,
        measurements.preferred_content_height,
    );
    ScreenGeometry::new(
        preferred.width().min(canvas.width()),
        preferred.height().min(canvas.height()),
    )
}

fn panel_outer_width(title: &str, body_width: usize) -> usize {
    title.chars().count().max(body_width) + 1
}

fn cap_panel_width(a: usize, b: usize, c: usize, d: usize, cap: usize) -> usize {
    [a, b, c, d].into_iter().max().unwrap_or(1).min(cap)
}

fn panel_outer_height(body_rows: usize) -> usize {
    1 + body_rows
}

fn stack_height_4(body_rows: [usize; 4]) -> usize {
    body_rows.into_iter().map(panel_outer_height).sum::<usize>() + 3
}

/// Snap the map block to the fixed ASCII lattice pitch used by the live map.
///
/// Returns `(snapped_block_width, snapped_grid_height)`.  The block width
/// accounts for the row-label gutter on the left, the trailing right-label
/// gutter, and a lattice body whose visible sector count is `N * CELL_WIDTH`
/// columns plus one closing wall. The grid height is trimmed to the largest
/// odd number that fits after reserving the top and bottom axis rows so the
/// lattice rows can alternate separator/content and end with a closing border.
fn snap_map_block_to_map_size(
    raw_width: usize,
    raw_height: usize,
    _map_size: usize,
) -> (usize, usize) {
    debug_assert_eq!(MAP_LEFT_PADDING, 0);
    debug_assert_eq!(MAP_VERTICAL_PADDING, 0);
    if raw_width == 0 || raw_height == 0 {
        return (raw_width, raw_height);
    }
    let gutters = ROW_LABEL_COLS + MAP_RIGHT_PADDING;
    let lattice_width_budget = raw_width.saturating_sub(gutters);
    let snapped_lattice_width = if lattice_width_budget == 0 {
        0
    } else {
        1 + lattice_width_budget.saturating_sub(1) / CELL_WIDTH * CELL_WIDTH
    };
    let snapped_width = ROW_LABEL_COLS + snapped_lattice_width + MAP_RIGHT_PADDING;
    let grid_height_budget = raw_height.saturating_sub(2); // top + bottom axis rows
    let snapped_grid_height = if grid_height_budget == 0 {
        0
    } else if grid_height_budget % 2 == 0 {
        grid_height_budget.saturating_sub(1)
    } else {
        grid_height_budget
    };
    (snapped_width, snapped_grid_height)
}

fn styled_row_width(rows: &[(String, crate::dashboard::buffer::CellStyle)]) -> usize {
    rows.iter()
        .map(|(row, _)| row.chars().count())
        .max()
        .unwrap_or(0)
}

fn allocate_body_rows<const N: usize>(
    preferred_rows: [usize; N],
    minimum_rows: [usize; N],
    content_height: usize,
    priority: [usize; N],
) -> [usize; N] {
    let chrome_rows = N + N.saturating_sub(1);
    let minimum_total = minimum_rows.iter().sum::<usize>() + chrome_rows;
    let mut actual = minimum_rows;
    let mut extra = content_height.saturating_sub(minimum_total);

    while extra > 0 {
        let mut advanced = false;
        for idx in priority {
            if actual[idx] < preferred_rows[idx] {
                actual[idx] += 1;
                extra -= 1;
                advanced = true;
                if extra == 0 {
                    break;
                }
            }
        }
        if !advanced {
            break;
        }
    }

    actual
}

fn build_widget_frames(
    canvas: ScreenGeometry,
    frame: ScreenGeometry,
    content_height: usize,
    left_width: usize,
    right_width: usize,
    left_rows: [usize; 4],
    right_rows: [usize; 4],
    map_block_width: usize,
    map_block_height: usize,
    map_size: usize,
) -> DashboardWidgetFrames {
    let (ox, oy) = frame_offset_for(canvas, frame);

    let outer_top = oy;
    let outer_bottom = oy + frame.height().saturating_sub(1);
    let header_bar_row = outer_top + 1;
    let header_divider_row = outer_top + 2;
    let footer_divider_row = outer_bottom.saturating_sub(2);
    let footer_bar_row = outer_bottom.saturating_sub(1);
    let content_top = header_divider_row + 1;
    let left_col = ox + 1;
    let left_divider_col = left_col + left_width;
    let center_col = left_divider_col + 1;
    let center_width = frame
        .width()
        .saturating_sub(2 + left_width + right_width + 2);
    let right_divider_col = center_col + center_width;
    let right_col = right_divider_col + 1;
    let left_heights = left_rows.map(panel_outer_height);
    let right_heights = right_rows.map(panel_outer_height);

    let left_planets_top = content_top + left_heights[0] + 1;
    let left_fleets_top = left_planets_top + left_heights[1] + 1;
    let left_war_record_top = left_fleets_top + left_heights[2] + 1;

    let right_galaxy_top = content_top + right_heights[0] + 1;
    let right_diplomacy_top = right_galaxy_top + right_heights[1] + 1;
    let right_sector_top = right_diplomacy_top + right_heights[2] + 1;

    let center_outer = WidgetRect {
        col: center_col,
        row: content_top,
        width: center_width,
        height: content_height,
    };
    // Snap the map block's width so its sector grid divides evenly into
    // `map_size` columns, and compute the snapped grid height separately so
    // any vertical slack can be split symmetrically above the axis row and
    // below the last sector (rather than piling up at the bottom).
    let (snapped_block_width, snapped_grid_height) =
        snap_map_block_to_map_size(map_block_width, map_block_height, map_size);
    let map_block = WidgetRect {
        col: center_col + center_width.saturating_sub(snapped_block_width) / 2,
        row: content_top,
        width: snapped_block_width,
        height: map_block_height,
    };
    // Vertical layout inside map_block:
    //   [top_gutter][top_axis][grid (odd lattice rows)][bottom_axis][bottom_gutter]
    // where top_gutter and bottom_gutter together absorb any leftover rows
    // split as evenly as possible.
    let axis_and_grid = 2 + snapped_grid_height;
    let vertical_slack = map_block.height.saturating_sub(axis_and_grid);
    let top_gutter = vertical_slack / 2;
    let axis_row = map_block.row + MAP_VERTICAL_PADDING + top_gutter;
    let grid = WidgetRect {
        col: map_block.col + MAP_LEFT_PADDING,
        row: axis_row + 1,
        width: map_block
            .width
            .saturating_sub(MAP_LEFT_PADDING + MAP_RIGHT_PADDING),
        height: snapped_grid_height,
    };
    let center_map = MapWidgetFrame {
        outer: center_outer,
        map_block,
        axis_row,
        grid,
        bottom_pad_row: grid.last_row().saturating_add(1),
        row_label_cols: ROW_LABEL_COLS,
        cell_width: CELL_WIDTH,
    };

    DashboardWidgetFrames {
        outer_top,
        outer_bottom,
        header_bar_row,
        header_divider_row,
        footer_divider_row,
        footer_bar_row,
        left_divider_col,
        right_divider_col,
        left_economy: panel_widget_frame(
            left_col,
            left_width,
            content_top,
            content_top + left_heights[0].saturating_sub(1),
        ),
        left_planets: panel_widget_frame(
            left_col,
            left_width,
            left_planets_top,
            left_planets_top + left_heights[1].saturating_sub(1),
        ),
        left_fleets: panel_widget_frame(
            left_col,
            left_width,
            left_fleets_top,
            left_fleets_top + left_heights[2].saturating_sub(1),
        ),
        left_war_record: panel_widget_frame(
            left_col,
            left_width,
            left_war_record_top,
            left_war_record_top + left_heights[3].saturating_sub(1),
        ),
        center_map,
        right_comms: panel_widget_frame(
            right_col,
            right_width,
            content_top,
            content_top + right_heights[0].saturating_sub(1),
        ),
        right_galaxy: panel_widget_frame(
            right_col,
            right_width,
            right_galaxy_top,
            right_galaxy_top + right_heights[1].saturating_sub(1),
        ),
        right_diplomacy: panel_widget_frame(
            right_col,
            right_width,
            right_diplomacy_top,
            right_diplomacy_top + right_heights[2].saturating_sub(1),
        ),
        right_sector_detail: panel_widget_frame(
            right_col,
            right_width,
            right_sector_top,
            right_sector_top + right_heights[3].saturating_sub(1),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dashboard_fits_canvas, dashboard_layout, layout_canvas_requirement,
        preferred_dashboard_frame, required_dashboard_frame,
    };
    use crate::dashboard::app::state::{DashApp, MapViewMode};
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::panels::{diplomacy, war_record};
    use nc_data::GameStateBuilder;
    use nc_engine::build_seeded_initialized_game;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn side_columns_are_measured_from_widget_content() {
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
        let layout = dashboard_layout(&app);

        assert_eq!(layout.left_width, layout.widgets.left_economy.outer.width);
        assert!(layout.right_width >= "SECTOR DETAIL".chars().count() + 1);
        assert_eq!(
            layout.widgets.center_map.outer.height,
            layout.frame.height().saturating_sub(6)
        );
        // Snapping the map block to a multiple of `map_size` may shrink it
        // inside `outer`.  It must still fit within the widget.
        let map_block = layout.widgets.center_map.map_block;
        let outer = layout.widgets.center_map.outer;
        assert!(map_block.col >= outer.col);
        assert!(map_block.last_col() <= outer.last_col());
        assert!(map_block.row >= outer.row);
        assert!(map_block.last_row() <= outer.last_row());
        assert!(layout.frame.width() < app.geometry.width());
        assert!(
            layout.widgets.left_fleets.outer.last_row() < layout.widgets.left_war_record.outer.row
        );
    }

    #[test]
    fn fill_mode_uses_full_terminal_canvas() {
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.map_view_mode = MapViewMode::Fill;
        let layout = dashboard_layout(&app);

        assert_eq!(layout.frame, app.geometry);
        let map_block = layout.widgets.center_map.map_block;
        let outer = layout.widgets.center_map.outer;
        assert!(map_block.last_col() <= outer.last_col());
        assert!(map_block.last_row() <= outer.last_row());
    }

    #[test]
    fn required_frame_stays_based_on_minimum_projected_map() {
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
        let required = required_dashboard_frame(&app);
        let readable = dashboard_layout(&app).frame;

        assert!(required.width() < readable.width());
    }

    #[test]
    fn diplomacy_widget_uses_actual_visible_relation_rows() {
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let required = required_dashboard_frame(&app);
        app.geometry = required;
        let layout = dashboard_layout(&app);

        assert_eq!(layout.frame, required);
        assert_eq!(
            layout.widgets.right_diplomacy.body.height,
            diplomacy::body_rows(&app).len()
        );
        assert!(layout.widgets.left_war_record.body.height >= war_record::MIN_BODY_ROWS);
    }

    #[test]
    fn layout_fit_helper_detects_widget_overrun() {
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
        let mut layout = dashboard_layout(&app);

        assert!(dashboard_fits_canvas(app.geometry, &layout));

        let overflow = app
            .geometry
            .width()
            .saturating_sub(layout_canvas_requirement(&layout).width())
            + 1;
        layout.widgets.right_sector_detail.outer.width += overflow;
        layout.widgets.right_sector_detail.body.width += overflow;

        assert!(!dashboard_fits_canvas(app.geometry, &layout));
        assert!(layout_canvas_requirement(&layout).width() > app.geometry.width());
    }

    #[test]
    fn benchmark_laptop_geometry_supports_large_map_dashboard() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let required = required_dashboard_frame(&app);
        let layout = dashboard_layout(&app);

        assert!(required.width() <= 187);
        assert!(required.height() <= 45);
        assert!(dashboard_fits_canvas(app.geometry, &layout));
        assert!(layout_canvas_requirement(&layout).height() <= app.geometry.height());
    }

    #[test]
    fn preferred_frame_expands_to_full_large_map_dashboard() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(300, 140),
            ScreenGeometry::new(0, 0),
            1,
        );
        let required = required_dashboard_frame(&app);
        let preferred = preferred_dashboard_frame(&app);

        assert!(preferred.width() > required.width());
        assert!(preferred.height() > required.height());

        app.geometry = preferred;
        assert_eq!(dashboard_layout(&app).frame, preferred);
    }
}
