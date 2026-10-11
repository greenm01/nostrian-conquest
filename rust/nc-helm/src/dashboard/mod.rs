pub mod app;
pub mod buffer;
pub mod client_settings;
pub mod coords;
pub mod diplomacy_view;
pub mod geometry;
pub mod inbox;
pub mod input;
pub mod launch;
pub mod layout;
pub mod modal;
pub mod native;
pub mod overlays;
pub mod panels;
pub mod planet_view;
pub mod popups;
pub mod prompt;
pub mod table;
pub mod table_filter;
pub mod table_layout;
pub mod table_selection;
pub mod theme;
pub mod ui;

use self::modal::modal_min_width_for_title;
pub use app::state::{DashApp, DashboardExitRequest};
pub use geometry::ScreenGeometry;
pub use launch::DashLaunchState;

use nc_data::TurnSubmission;
use nc_nostr::state_sync::GameState;
use std::time::{Duration, Instant};

use self::native::NativeApp;

pub(crate) const QUIT_CONFIRM_TITLE: &str = "QUIT";
pub(crate) const QUIT_CONFIRM_HEIGHT: usize = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HostedViewRenderStats {
    pub dashboard_render: Duration,
    pub convert: Duration,
    pub dirty_regions: usize,
    pub full_rebuild: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct HostedBufferRenderResult {
    pub hashes: app::render::RegionHashes,
    pub stats: HostedViewRenderStats,
}

pub(crate) fn quit_confirm_popup_width(message: &str) -> usize {
    (message.chars().count() + 4).max(modal_min_width_for_title(QUIT_CONFIRM_TITLE))
}

pub fn build_hosted_dash_app(
    snapshot: &GameState,
    geometry: ScreenGeometry,
) -> Result<DashApp, Box<dyn std::error::Error>> {
    DashLaunchState::from_hosted_snapshot(snapshot)?.into_app(geometry)
}

#[allow(dead_code)]
pub fn replay_hosted_draft(
    dashboard: &mut DashApp,
    draft: &TurnSubmission,
) -> Result<(), nc_data::TurnSubmissionError> {
    draft.apply_to(&mut dashboard.game_data, &mut dashboard.queued_mail)?;
    dashboard.hosted_turn_draft = Some(draft.clone());
    dashboard.game_data_revision += 1;
    *dashboard.panel_cache.borrow_mut() = app::panel_cache::PanelCache::default();
    *dashboard.starmap_projection_cache.borrow_mut() = None;
    *dashboard.sector_detail_cache.borrow_mut() = None;
    Ok(())
}

pub fn resize_hosted_app(dashboard: &mut DashApp, cols: usize, rows: usize) {
    NativeApp::resize_canvas(dashboard, cols as u16, rows as u16);
}

pub fn dispatch_hosted_key(dashboard: &mut DashApp, key: input::KeyEvent) {
    NativeApp::dispatch_key_event(dashboard, key);
}

pub fn dispatch_hosted_mouse(dashboard: &mut DashApp, mouse: input::MouseEvent) -> bool {
    NativeApp::dispatch_mouse_event(dashboard, mouse)
}

pub fn hosted_on_idle(dashboard: &mut DashApp) -> bool {
    NativeApp::on_idle(dashboard)
}

pub fn hosted_next_wakeup(dashboard: &DashApp) -> Option<Instant> {
    NativeApp::next_wakeup(dashboard)
}

pub fn hosted_take_exit_request(dashboard: &mut DashApp) -> Option<DashboardExitRequest> {
    dashboard.take_exit_request()
}

pub fn hosted_wants_text_input(dashboard: &DashApp) -> bool {
    NativeApp::wants_text_input(dashboard)
}

pub fn hosted_wants_window_focus(dashboard: &DashApp) -> bool {
    NativeApp::wants_window_focus(dashboard)
}

pub fn render_hosted_buffer(
    dashboard: &DashApp,
) -> Result<crate::PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut playfield = buffer::PlayfieldBuffer::new(
        0,
        0,
        buffer::CellStyle::new(buffer::GameColor::Black, buffer::GameColor::Black, false),
    );
    render_hosted_playfield_into(dashboard, &mut playfield)?;
    let mut buffer = crate::PlayfieldBuffer::new(
        playfield.width(),
        playfield.height(),
        crate::CellStyle::new(crate::GameColor::Black, crate::GameColor::Black, false),
    );
    render_hosted_buffer_into_playfield(&playfield, &mut buffer);
    Ok(buffer)
}

pub(crate) fn render_hosted_playfield_into(
    dashboard: &DashApp,
    playfield: &mut buffer::PlayfieldBuffer,
) -> Result<(), Box<dyn std::error::Error>> {
    app::render::render_into(dashboard, playfield)
}

#[allow(dead_code)]
pub(crate) fn render_hosted_buffer_into(
    dashboard: &DashApp,
    dashboard_playfield: &mut buffer::PlayfieldBuffer,
    buffer: &mut crate::PlayfieldBuffer,
) -> Result<(), Box<dyn std::error::Error>> {
    render_hosted_playfield_into(dashboard, dashboard_playfield)?;
    render_hosted_buffer_into_playfield(dashboard_playfield, buffer);
    Ok(())
}

pub(crate) fn render_hosted_buffer_incremental_into(
    dashboard: &DashApp,
    previous_hashes: Option<&app::render::RegionHashes>,
    dashboard_playfield: &mut buffer::PlayfieldBuffer,
    buffer: &mut crate::PlayfieldBuffer,
) -> Result<HostedBufferRenderResult, Box<dyn std::error::Error>> {
    let render_started = Instant::now();
    let outcome =
        app::render::render_incremental_into(dashboard, dashboard_playfield, previous_hashes)?;
    let dashboard_render = render_started.elapsed();

    let convert_started = Instant::now();
    if outcome.full_rebuild
        || buffer.width() != dashboard_playfield.width()
        || buffer.height() != dashboard_playfield.height()
    {
        render_hosted_buffer_into_playfield(dashboard_playfield, buffer);
    } else {
        for rect in &outcome.dirty_rects {
            convert_dashboard_region_into_playfield(dashboard_playfield, buffer, *rect);
        }
        sync_playfield_cursor_and_overlays(dashboard_playfield, buffer);
    }
    let convert = convert_started.elapsed();

    Ok(HostedBufferRenderResult {
        hashes: outcome.hashes,
        stats: HostedViewRenderStats {
            dashboard_render,
            convert,
            dirty_regions: outcome.dirty_rects.len(),
            full_rebuild: outcome.full_rebuild,
        },
    })
}

fn render_hosted_buffer_into_playfield(
    playfield: &buffer::PlayfieldBuffer,
    buffer: &mut crate::PlayfieldBuffer,
) {
    let base_style = if playfield.width() > 0 && playfield.height() > 0 {
        convert_style(playfield.row(0)[0].style)
    } else {
        crate::CellStyle::new(crate::GameColor::Black, crate::GameColor::Black, false)
    };
    buffer.reset(playfield.width(), playfield.height(), base_style);
    for row in 0..playfield.height() {
        for (col, cell) in playfield.row(row).iter().copied().enumerate() {
            buffer.set_cell(row, col, cell.ch, convert_style(cell.style));
        }
    }
    sync_playfield_cursor_and_overlays(playfield, buffer);
}

fn convert_dashboard_region_into_playfield(
    playfield: &buffer::PlayfieldBuffer,
    buffer: &mut crate::PlayfieldBuffer,
    rect: app::render::DirtyRect,
) {
    let row_end = rect.row.saturating_add(rect.height).min(playfield.height());
    let col_end = rect.col.saturating_add(rect.width).min(playfield.width());
    for row in rect.row..row_end {
        for col in rect.col..col_end {
            let cell = playfield.row(row)[col];
            buffer.set_cell(row, col, cell.ch, convert_style(cell.style));
        }
    }
}

fn sync_playfield_cursor_and_overlays(
    playfield: &buffer::PlayfieldBuffer,
    buffer: &mut crate::PlayfieldBuffer,
) {
    buffer.clear_cursor();
    if let Some((column, row)) = playfield.cursor() {
        buffer.set_cursor(crate::Point::from_usize(column as usize, row as usize));
    }
    buffer.clear_overlay_logos();
    buffer.clear_overlay_selection();
    if let Some(overlay) = playfield.overlay_selection() {
        buffer.set_overlay_selection(crate::grid::OverlaySelection {
            fg: convert_color(overlay.fg),
            left_col: overlay.left_col,
            right_col: overlay.right_col,
            top_row: overlay.top_row,
            bottom_row: overlay.bottom_row,
        });
    }
}

fn convert_style(style: buffer::CellStyle) -> crate::CellStyle {
    crate::CellStyle::new(convert_color(style.fg), convert_color(style.bg), style.bold)
}

fn convert_color(color: buffer::GameColor) -> crate::GameColor {
    match color {
        buffer::GameColor::Black => crate::GameColor::Black,
        buffer::GameColor::Red => crate::GameColor::Red,
        buffer::GameColor::Green => crate::GameColor::Green,
        buffer::GameColor::Yellow => crate::GameColor::Yellow,
        buffer::GameColor::Blue => crate::GameColor::Blue,
        buffer::GameColor::Magenta => crate::GameColor::Magenta,
        buffer::GameColor::Cyan => crate::GameColor::Cyan,
        buffer::GameColor::White => crate::GameColor::White,
        buffer::GameColor::BrightBlack => crate::GameColor::BrightBlack,
        buffer::GameColor::BrightRed => crate::GameColor::BrightRed,
        buffer::GameColor::BrightGreen => crate::GameColor::BrightGreen,
        buffer::GameColor::BrightYellow => crate::GameColor::BrightYellow,
        buffer::GameColor::BrightBlue => crate::GameColor::BrightBlue,
        buffer::GameColor::BrightMagenta => crate::GameColor::BrightMagenta,
        buffer::GameColor::BrightCyan => crate::GameColor::BrightCyan,
        buffer::GameColor::BrightWhite => crate::GameColor::BrightWhite,
        buffer::GameColor::Indexed(value) => crate::GameColor::Indexed(value),
        buffer::GameColor::Rgb(red, green, blue) => crate::GameColor::Rgb(red, green, blue),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HostedBufferRenderResult, buffer, render_hosted_buffer,
        render_hosted_buffer_incremental_into,
    };
    use crate::dashboard::DashApp;
    use crate::dashboard::app::state::ActiveOverlay;
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::overlays::frame::RelativePopupOrigin;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    fn test_dash_app() -> DashApp {
        DashApp::new_for_tests(
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
        )
    }

    fn dashboard_buffer() -> buffer::PlayfieldBuffer {
        buffer::PlayfieldBuffer::new(
            0,
            0,
            buffer::CellStyle::new(buffer::GameColor::Black, buffer::GameColor::Black, false),
        )
    }

    fn grid_buffer() -> crate::PlayfieldBuffer {
        crate::PlayfieldBuffer::new(
            0,
            0,
            crate::CellStyle::new(crate::GameColor::Black, crate::GameColor::Black, false),
        )
    }

    fn assert_playfields_match(actual: &crate::PlayfieldBuffer, expected: &crate::PlayfieldBuffer) {
        assert_eq!(actual.width(), expected.width());
        assert_eq!(actual.height(), expected.height());
        assert_eq!(actual.get_all_cells(), expected.get_all_cells());
        assert_eq!(actual.cursor(), expected.cursor());
    }

    fn render_incrementally(
        app: &DashApp,
        previous: Option<&crate::dashboard::app::render::RegionHashes>,
        dashboard_playfield: &mut buffer::PlayfieldBuffer,
        playfield: &mut crate::PlayfieldBuffer,
    ) -> HostedBufferRenderResult {
        render_hosted_buffer_incremental_into(app, previous, dashboard_playfield, playfield)
            .expect("incremental hosted render")
    }

    fn place_overlay_over_dashboard_divider(app: &mut DashApp) {
        let widgets = crate::dashboard::layout::dashboard_layout(app).widgets;
        let parent = crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(widgets);
        app.overlay_position = Some(RelativePopupOrigin {
            col_offset: widgets
                .left_divider_col
                .saturating_sub(parent.x as usize)
                .saturating_sub(2),
            row_offset: widgets
                .left_planets
                .outer
                .row
                .saturating_sub(parent.y as usize)
                .saturating_sub(1),
        });
    }

    fn place_overlay_over_sector_detail_lower_rows(app: &mut DashApp) {
        let widgets = crate::dashboard::layout::dashboard_layout(app).widgets;
        let parent = crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(widgets);
        app.overlay_position = Some(RelativePopupOrigin {
            col_offset: widgets
                .right_sector_detail
                .body
                .col
                .saturating_sub(parent.x as usize)
                .saturating_sub(2),
            row_offset: widgets
                .right_sector_detail
                .body
                .row
                .saturating_add(9)
                .saturating_sub(parent.y as usize),
        });
    }

    fn move_crosshair_to_empty_sector(app: &mut DashApp) {
        let occupied = app
            .game_data
            .planets
            .iter()
            .map(|planet| planet.sector.to_array())
            .collect::<BTreeSet<_>>();
        let empty = (1..=18)
            .flat_map(|y| (1..=18).map(move |x| [x, y]))
            .find(|coords| !occupied.contains(coords))
            .expect("empty sector");
        app.crosshair_x = empty[0];
        app.crosshair_y = empty[1];
    }

    #[test]
    fn incremental_hosted_crosshair_update_matches_full_render() {
        let mut app = test_dash_app();
        app.crosshair_x = 2;
        app.crosshair_y = 3;

        let mut dashboard_playfield = dashboard_buffer();
        let mut playfield = grid_buffer();
        let first = render_incrementally(&app, None, &mut dashboard_playfield, &mut playfield);

        assert!(first.stats.full_rebuild);

        app.crosshair_x = 4;
        app.crosshair_y = 5;

        let second = render_incrementally(
            &app,
            Some(&first.hashes),
            &mut dashboard_playfield,
            &mut playfield,
        );
        assert!(!second.stats.full_rebuild);
        assert!(second.stats.dirty_regions > 0);
        assert_ne!(first.hashes.starmap, second.hashes.starmap);
        assert_ne!(first.hashes.sector_detail, second.hashes.sector_detail);

        let expected = render_hosted_buffer(&app).expect("full hosted render");
        assert_playfields_match(&playfield, &expected);
    }

    #[test]
    fn incremental_hosted_moved_overlay_restores_dashboard_chrome() {
        let mut app = test_dash_app();
        app.overlay = ActiveOverlay::Inbox;
        place_overlay_over_dashboard_divider(&mut app);

        let mut dashboard_playfield = dashboard_buffer();
        let mut playfield = grid_buffer();
        let first = render_incrementally(&app, None, &mut dashboard_playfield, &mut playfield);

        let origin = app.overlay_position.expect("overlay origin");
        app.overlay_position = Some(RelativePopupOrigin {
            col_offset: origin.col_offset.saturating_add(12),
            row_offset: origin.row_offset.saturating_add(3),
        });

        let second = render_incrementally(
            &app,
            Some(&first.hashes),
            &mut dashboard_playfield,
            &mut playfield,
        );
        assert!(!second.stats.full_rebuild);
        assert!(second.stats.dirty_regions > 0);

        let expected = render_hosted_buffer(&app).expect("full hosted render");
        assert_playfields_match(&playfield, &expected);
    }

    #[test]
    fn incremental_hosted_closed_overlay_restores_dashboard_chrome() {
        let mut app = test_dash_app();
        app.overlay = ActiveOverlay::Inbox;
        place_overlay_over_dashboard_divider(&mut app);

        let mut dashboard_playfield = dashboard_buffer();
        let mut playfield = grid_buffer();
        let first = render_incrementally(&app, None, &mut dashboard_playfield, &mut playfield);

        app.overlay = ActiveOverlay::None;
        app.overlay_position = None;

        let second = render_incrementally(
            &app,
            Some(&first.hashes),
            &mut dashboard_playfield,
            &mut playfield,
        );
        assert!(!second.stats.full_rebuild);
        assert!(second.stats.dirty_regions > 0);

        let expected = render_hosted_buffer(&app).expect("full hosted render");
        assert_playfields_match(&playfield, &expected);
    }

    #[test]
    fn incremental_hosted_sector_detail_cache_miss_does_not_preserve_modal_bleed() {
        let mut app = test_dash_app();
        app.overlay = ActiveOverlay::Inbox;
        place_overlay_over_sector_detail_lower_rows(&mut app);

        let mut dashboard_playfield = dashboard_buffer();
        let mut playfield = grid_buffer();
        let first = render_incrementally(&app, None, &mut dashboard_playfield, &mut playfield);

        move_crosshair_to_empty_sector(&mut app);
        let second = render_incrementally(
            &app,
            Some(&first.hashes),
            &mut dashboard_playfield,
            &mut playfield,
        );
        assert!(!second.stats.full_rebuild);
        assert!(second.stats.dirty_regions > 0);

        app.overlay = ActiveOverlay::None;
        app.overlay_position = None;
        let third = render_incrementally(
            &app,
            Some(&second.hashes),
            &mut dashboard_playfield,
            &mut playfield,
        );
        assert!(!third.stats.full_rebuild);
        assert!(third.stats.dirty_regions > 0);

        let expected = render_hosted_buffer(&app).expect("full hosted render");
        assert_playfields_match(&playfield, &expected);
    }
}
