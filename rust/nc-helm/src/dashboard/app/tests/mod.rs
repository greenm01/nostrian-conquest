use super::{map_coord_rows, parse_table_coord};
use crate::dashboard::app::state::{
    ActiveOverlay, ActivePopup, DashApp, DashboardExitRequest, FleetOrderScope, FleetOverlayFilter,
    FleetOverlayPromptMode, FleetOverlayRowKey, FleetOverlaySort, HelpContext,
    InboxMessageConfirmAction, InboxPromptMode, IntelOverlayFilter, IntelOverlayPromptMode,
    IntelOverlaySort, OwnedPlanetPopupMode, PlanetOverlayFilter, PlanetOverlayPromptMode,
    PlanetOverlaySort, SortDirection,
};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::input::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crate::dashboard::layout::dashboard::dashboard_layout;
use crate::dashboard::native::NativeApp;
use crate::dashboard::overlays::{diplomacy, fleet_list, intel_database, planet_list};
use crate::dashboard::panels::starmap;
use crate::dashboard::planet_view;
use crate::dashboard::table_selection::{wrap_next_index, wrap_prev_index};
use nc_data::{
    CampaignStore, DiplomaticRelation, GameStateBuilder, IntelTier, Order, PlanetIntelSnapshot,
    PlanetTurnAction, PlanetTurnBlock, QueuedPlayerMail, ReportBlockRow, TurnSubmission,
};
use nc_engine::{
    build_seeded_initialized_game, fleet_target_input_kind, recommended_coordinate_target,
    recommended_coordinate_target_y_for_entered_x,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod confirm_defaults;
mod inbox_outgoing;
mod inbox_year_filter;
mod stale_session;

#[test]
fn wrap_prev_goes_from_first_to_last() {
    assert_eq!(wrap_prev_index(0, 4), 3);
}

#[test]
fn wrap_next_goes_from_last_to_first() {
    assert_eq!(wrap_next_index(3, 4), 0);
}

#[test]
fn parse_table_coord_reads_table_style_coords() {
    assert_eq!(parse_table_coord("(02,03)"), Some([2, 3]));
    assert_eq!(parse_table_coord("[02,03]"), Some([2, 3]));
    assert_eq!(parse_table_coord("bogus"), None);
}

#[test]
fn map_coord_rows_cover_entire_map_in_numeric_coordinate_order() {
    let app = dash_app();
    let rows = map_coord_rows(&app);
    assert_eq!(
        rows.first().and_then(|row| row.first()),
        Some(&"(01,01)".to_string())
    );
    assert_eq!(
        rows.get(1).and_then(|row| row.first()),
        Some(&"(01,02)".to_string())
    );
    assert_eq!(
        rows.get(18).and_then(|row| row.first()),
        Some(&"(02,01)".to_string())
    );
}

#[test]
fn typed_map_coords_move_crosshair_and_clear_on_exact_match() {
    let mut app = dash_app();
    app.handle_key(KeyEvent::new(
        KeyCode::Char('0'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('2'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char(','),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('0'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('3'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));

    assert_eq!([app.crosshair_x, app.crosshair_y], [2, 3]);
    assert!(app.map_coord_input.is_empty());
}

#[test]
fn startup_review_opens_automatically_if_pending() {
    let viewer_id = 1;
    let report_block_rows = vec![ReportBlockRow {
        viewer_empire_id: viewer_id as u8,
        block_index: 0,
        decoded_text: "Empire Status Report Body Line 1\nLine 2".to_string(),
        raw_bytes: Some(Vec::new()),
        recipient_deleted: false,
    }];

    let geom = ScreenGeometry::new(100, 40);
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        GameStateBuilder::new()
            .with_player_count(4)
            .build_initialized_baseline()
            .unwrap(),
        BTreeMap::new(),
        report_block_rows.clone(),
        Vec::new(),
        Vec::new(),
        geom,
        geom,
        viewer_id,
    );

    // Manually trigger what into_app does for the test
    let reports_pending = nc_data::has_visible_runtime_reports(viewer_id as u8, &report_block_rows);
    if reports_pending {
        app.popup = ActivePopup::StartupReview;
        app.startup_review.reports = nc_data::ReportsPreview::from_block_rows(
            &app.game_data,
            viewer_id as u8,
            &report_block_rows,
            &[],
        );
    }

    assert_eq!(app.popup, ActivePopup::StartupReview);
    assert_eq!(app.startup_review.reports.result_blocks.len(), 1);
}

#[test]
fn startup_review_advances_on_slap() {
    let viewer_id = 1;
    let report_block_rows = vec![
        ReportBlockRow {
            viewer_empire_id: viewer_id as u8,
            block_index: 0,
            decoded_text: "Block 1".to_string(),
            raw_bytes: Some(Vec::new()),
            recipient_deleted: false,
        },
        ReportBlockRow {
            viewer_empire_id: viewer_id as u8,
            block_index: 1,
            decoded_text: "Block 2".to_string(),
            raw_bytes: Some(Vec::new()),
            recipient_deleted: false,
        },
    ];

    let geom = ScreenGeometry::new(100, 40);
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        GameStateBuilder::new()
            .with_player_count(4)
            .build_initialized_baseline()
            .unwrap(),
        BTreeMap::new(),
        report_block_rows.clone(),
        Vec::new(),
        Vec::new(),
        geom,
        geom,
        viewer_id,
    );

    app.popup = ActivePopup::StartupReview;
    app.startup_review.reports = nc_data::ReportsPreview::from_block_rows(
        &app.game_data,
        viewer_id as u8,
        &report_block_rows,
        &[],
    );

    assert_eq!(app.popup, ActivePopup::StartupReview);
    assert_eq!(app.startup_review.results_block, 0);

    // Slap a key (Space)
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert_eq!(app.startup_review.results_block, 1);

    // Slap again
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn startup_review_nonstop_advances_on_idle() {
    let viewer_id = 1;
    let report_block_rows = vec![
        ReportBlockRow {
            viewer_empire_id: viewer_id as u8,
            block_index: 0,
            decoded_text: "Block 1".to_string(),
            raw_bytes: Some(Vec::new()),
            recipient_deleted: false,
        },
        ReportBlockRow {
            viewer_empire_id: viewer_id as u8,
            block_index: 1,
            decoded_text: "Block 2".to_string(),
            raw_bytes: Some(Vec::new()),
            recipient_deleted: false,
        },
    ];

    let geom = ScreenGeometry::new(100, 40);
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        GameStateBuilder::new()
            .with_player_count(4)
            .build_initialized_baseline()
            .unwrap(),
        BTreeMap::new(),
        report_block_rows.clone(),
        Vec::new(),
        Vec::new(),
        geom,
        geom,
        viewer_id,
    );

    app.popup = ActivePopup::StartupReview;
    app.startup_review.reports = nc_data::ReportsPreview::from_block_rows(
        &app.game_data,
        viewer_id as u8,
        &report_block_rows,
        &[],
    );
    app.startup_review.results_nonstop = true;

    assert_eq!(app.startup_review.results_block, 0);

    // Call on_idle (via NativeApp trait)
    app.on_idle();
    assert_eq!(app.startup_review.results_block, 1);

    // Call on_idle again
    app.on_idle();
    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn typed_map_coords_keep_partial_input_visible() {
    let mut app = dash_app();
    app.handle_key(KeyEvent::new(
        KeyCode::Char('0'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('2'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));

    assert_eq!([app.crosshair_x, app.crosshair_y], [2, 1]);
    assert_eq!(app.map_coord_input, "02");
}

#[test]
fn typed_map_coords_do_not_enter_readable_void_rows() {
    let mut app = dash_app();
    app.handle_key(KeyEvent::new(
        KeyCode::Char('0'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('1'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char(','),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('2'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('3'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));

    assert!(app.crosshair_x <= 18);
    assert!(app.crosshair_y <= 18);
}

#[test]
fn dashboard_actions_clear_partial_map_coord_input() {
    let mut app = dash_app();
    app.handle_key(KeyEvent::new(
        KeyCode::Char('0'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char('2'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));
    app.handle_key(KeyEvent::new(
        KeyCode::Char(']'),
        crate::dashboard::input::KeyModifiers::NONE,
    ));

    assert!(app.map_coord_input.is_empty());
}

#[test]
fn empty_planet_filter_reverts_to_all_rows() {
    let mut app = dash_app();
    let all_rows = planet_list::table_rows(&app).len();

    app.apply_planet_overlay_filter(PlanetOverlayFilter::Range {
        anchor: [18, 18],
        radius: 0,
    });

    assert_eq!(app.planet_overlay.filter, PlanetOverlayFilter::All);
    assert_eq!(planet_list::table_rows(&app).len(), all_rows);
}

#[test]
fn empty_fleet_filter_reverts_to_all_rows() {
    let mut app = dash_app();
    app.game_data.fleets.clear();
    app.game_data.bases.clear();
    let all_rows = fleet_list::table_rows(&app).len();

    app.apply_fleet_overlay_filter(FleetOverlayFilter::Combat);

    assert_eq!(app.fleet_overlay.filter, FleetOverlayFilter::All);
    assert_eq!(fleet_list::table_rows(&app).len(), all_rows);
}

#[test]
fn empty_intel_filter_reverts_to_all_rows() {
    let mut app = dash_app();
    let all_rows = intel_database::table_rows(&app).len();

    app.apply_intel_overlay_filter(IntelOverlayFilter::Empire(99));

    assert_eq!(app.intel_overlay.filter, IntelOverlayFilter::All);
    assert_eq!(intel_database::table_rows(&app).len(), all_rows);
}

#[test]
fn closing_planet_build_modal_returns_to_planet_list_overlay() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(50));
    app.open_planet_build_specify();
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
}

#[test]
fn opening_build_specify_with_no_budget_keeps_build_table_open() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(1));

    app.open_planet_build_specify();

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(app.planet_overlay.footer_notice, None);
    assert_eq!(
        app.planet_overlay.build_planet_record_index_1_based,
        Some(1)
    );
}

#[test]
fn successful_build_that_exhausts_budget_stays_in_build_table() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(2));

    app.open_planet_build_specify();
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );

    app.handle_key(key(KeyCode::Char('+')));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.planet_overlay.footer_notice, None);
    assert_eq!(
        app.planet_overlay.build_planet_record_index_1_based,
        Some(1)
    );
}

#[test]
fn equals_key_queues_selected_build_unit() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Char('=')));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    let orders = nc_engine::planet_build_orders(&app.game_data.planets[0]);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].kind, nc_data::ProductionItemKind::Destroyer);
    assert_eq!(orders[0].points_remaining, 5);
}

#[test]
fn build_command_refreshes_cached_planet_status_without_store() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    app.crosshair_x = owned_coords[0];
    app.crosshair_y = owned_coords[1];
    app.game_data.planets[expected_record - 1].stored_goods =
        nc_data::Real48::from_f64(f64::from(80));

    let before = planet_view::selected_planet_detail(&app).expect("selected planet");
    assert_eq!(
        detail_value(&before.widget_fields, "Building"),
        Some("Nothing")
    );
    let initial_revision = app.game_data_revision;

    app.open_planet_build_specify_for_record(expected_record);
    app.handle_key(key(KeyCode::Char('+')));

    assert!(app.game_data_revision > initial_revision);
    let after = planet_view::selected_planet_detail(&app).expect("selected planet");
    assert_eq!(detail_value(&after.widget_fields, "Building"), Some("DD1"));
}

#[test]
fn empty_build_browse_enter_opens_quantity_for_highlighted_unit() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildQuantity
    );
}

#[test]
fn zero_build_browse_enter_exits_overlay() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Char('0')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
}

#[test]
fn numeric_build_browse_enter_selects_unit_and_opens_quantity() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Char('2')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildQuantity
    );
    assert_eq!(
        app.planet_overlay.build_selected_kind,
        Some(nc_data::ProductionItemKind::Cruiser)
    );
}

#[test]
fn numeric_build_browse_input_highlights_matching_unit_before_enter() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Char('2')));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(
        app.planet_overlay.build_selected_kind,
        Some(nc_data::ProductionItemKind::Cruiser)
    );
}

#[test]
fn planet_overlay_delete_clears_selected_kind_queue_in_place() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data
        .append_planet_build_order(1, 10, 1)
        .expect("queue build order");
    app.open_planet_build_specify();

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(
        app.planet_overlay.build_selected_kind,
        Some(nc_data::ProductionItemKind::Destroyer)
    );

    app.handle_key(key(KeyCode::Char('d')));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert!(
        nc_engine::planet_build_orders(&app.game_data.planets[0]).is_empty(),
        "build queue should be cleared for the selected unit"
    );
}

#[test]
fn planet_overlay_minus_removes_one_selected_unit_in_place() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::PlanetList;
    app.game_data
        .append_planet_build_order(1, 15, 1)
        .expect("queue build order");
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Char('-')));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    let orders = nc_engine::planet_build_orders(&app.game_data.planets[0]);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].points_remaining, 10);
}

#[test]
fn planet_overlay_delete_reports_empty_selected_kind_queue_inline() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.open_planet_build_specify();

    app.handle_key(key(KeyCode::Char('d')));

    assert_eq!(
        app.planet_overlay.build_unit_status.as_deref(),
        Some("No queued units of this type.")
    );
}

#[test]
fn planet_overlay_footer_notice_clears_after_navigation() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay.footer_notice = Some("No build budget remains.".to_string());

    app.handle_key(key(KeyCode::Down));

    assert_eq!(app.planet_overlay.footer_notice, None);
}

#[test]
fn nested_planet_filter_modals_unwind_one_level_at_a_time() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;

    app.handle_key(key(KeyCode::Char('f')));
    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Char('o')));
    app.handle_key(key(KeyCode::Char('o')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::FilterValueInput
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
}

#[test]
fn planet_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;

    app.handle_key(key(KeyCode::Char('f')));
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::FilterMenu
    );
    assert_eq!(
        app.planet_overlay.prompt_status.as_deref(),
        Some(" Ambiguous: sbs/sta")
    );
    assert!(
        render_planet_footer_line(&app, "Ambiguous: sbs/sta")
            .contains("COMMAND <-  Ambiguous: sbs/sta")
    );

    app.handle_key(key(KeyCode::Backspace));
    assert_eq!(app.planet_overlay.prompt_status, None);
    app.handle_key(key(KeyCode::Char('d')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::FilterValueInput
    );
    assert_eq!(
        app.planet_overlay
            .pending_filter_column
            .expect("pending column")
            .code,
        "sta"
    );
}

#[test]
fn closing_fleet_order_modal_returns_to_fleet_list_overlay() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.open_selected_fleet_order_flow();

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
}

#[test]
fn nested_fleet_order_modals_unwind_one_level_at_a_time() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::MoveOnly.code().to_string();
    app.submit_fleet_mission_picker();
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderTargetX
    );

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderTargetY
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderTargetX
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MissionPicker
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
}

#[test]
fn empty_coordinate_submissions_accept_defaults() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.code());
    app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;

    let expected_x = app.fleet_order_target_x_default_value();
    assert!(!expected_x.is_empty());

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderTargetY
    );
    assert_eq!(app.fleet_overlay.order_target_x_input, expected_x);

    let expected_y = app.fleet_order_target_y_default_value();
    assert!(!expected_y.is_empty());

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderConfirm
    );
    assert_eq!(app.fleet_overlay.order_target_y_input, expected_y);
}

#[test]
fn fleet_missions_route_to_expected_target_prompt_modes() {
    let mut app = audit_ready_dash_app();
    app.overlay = ActiveOverlay::FleetList;

    for mission_code in 0..=15 {
        select_first_fleet_row(&mut app);
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = mission_code.to_string();
        let record = app
            .selected_fleet_order_row()
            .expect("selected fleet row")
            .fleet_record_index_1_based;
        let automatic = nc_engine::automatic_order_targets(&app.game_data, mission_code, &[record]);

        app.submit_fleet_mission_picker();

        let question =
            nc_engine::seek_next_closest_question(&app.game_data, mission_code, &[record]);

        // Seek Home and a single-world Salvage skip the coordinates, as ECGAME does.
        let expected_prompt_mode = match (automatic, fleet_target_input_kind(Some(mission_code))) {
            (Err(_), _) => FleetOverlayPromptMode::MissionPicker,
            (Ok(Some(_)), _) if question.is_some() => FleetOverlayPromptMode::SeekNextClosest,
            (Ok(Some(_)), _) => FleetOverlayPromptMode::OrderConfirm,
            (
                Ok(None),
                nc_engine::FleetTargetInputKind::StarbaseId
                | nc_engine::FleetTargetInputKind::FleetId
                | nc_engine::FleetTargetInputKind::None,
            ) => FleetOverlayPromptMode::OrderTarget,
            (Ok(None), nc_engine::FleetTargetInputKind::Coordinates) => {
                FleetOverlayPromptMode::OrderTargetX
            }
        };
        assert_eq!(
            app.fleet_overlay.prompt_mode, expected_prompt_mode,
            "mission {mission_code} routed to wrong prompt"
        );
    }
}

#[test]
fn seek_home_order_targets_the_nearest_owned_world_without_coordinates() {
    let mut app = audit_ready_dash_app();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    app.open_selected_fleet_order_flow();
    let record = app
        .selected_fleet_order_row()
        .expect("selected fleet row")
        .fleet_record_index_1_based;
    let fleet = &mut app.game_data.fleets[record - 1];
    fleet.sector = nc_data::Coords::from_array([1, 1]);
    nc_data::set_fleet_at_rest(fleet);

    app.fleet_overlay.mission_picker_input = "2".to_string();
    app.submit_fleet_mission_picker();
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderConfirm
    );
    app.handle_key(key(KeyCode::Enter));

    let fleet = &app.game_data.fleets[record - 1];
    assert_eq!(fleet.order, nc_data::Order::SeekHome);
    assert_eq!(fleet.target.to_array(), [16, 13]);
}

/// Order the first fleet, at rest in its home system at (16,13), to Seek
/// Home; with `second_world` player 1 also owns (8,8).
fn seek_home_from_the_home_system(second_world: bool) -> (DashApp, usize) {
    let mut app = audit_ready_dash_app();
    app.overlay = ActiveOverlay::FleetList;
    if second_world {
        app.game_data
            .planets
            .iter_mut()
            .find(|planet| planet.sector.to_array() == [8, 8])
            .expect("seeded world")
            .owner
            .0 = 1;
    }
    select_first_fleet_row(&mut app);
    app.open_selected_fleet_order_flow();
    let record = app
        .selected_fleet_order_row()
        .expect("selected fleet row")
        .fleet_record_index_1_based;
    let fleet = &mut app.game_data.fleets[record - 1];
    fleet.sector = nc_data::Coords::from_array([16, 13]);
    nc_data::set_fleet_at_rest(fleet);
    app.fleet_overlay.mission_picker_input = "2".to_string();
    app.submit_fleet_mission_picker();
    (app, record)
}

#[test]
fn seek_home_in_the_home_system_asks_and_defaults_to_the_next_closest() {
    let (mut app, record) = seek_home_from_the_home_system(true);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::SeekNextClosest
    );
    let mut buffer = crate::dashboard::buffer::PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    crate::dashboard::overlays::fleet_list::draw(
        &mut buffer,
        &app,
        dashboard_layout(&app).widgets.center_map,
    );
    let lines = (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .collect::<Vec<_>>();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("fleet is already in a star system you control."))
    );
    assert!(lines.iter().any(|line| line.contains(
        "COMMAND <- Do you wish this fleet to seek the next closest system? [Y]/N <ESC> ->"
    )));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderConfirm
    );
    app.handle_key(key(KeyCode::Enter));

    let fleet = &app.game_data.fleets[record - 1];
    assert_eq!(fleet.order, nc_data::Order::SeekHome);
    assert_eq!(fleet.target.to_array(), [8, 8]);
}

#[test]
fn seek_home_in_the_home_system_stays_on_no() {
    let (mut app, record) = seek_home_from_the_home_system(true);
    app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderConfirm
    );
    app.handle_key(key(KeyCode::Enter));

    let fleet = &app.game_data.fleets[record - 1];
    assert_eq!(fleet.order, nc_data::Order::SeekHome);
    assert_eq!(fleet.target.to_array(), [16, 13]);
}

#[test]
fn seek_home_in_the_only_controlled_system_aborts() {
    let (app, record) = seek_home_from_the_home_system(false);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MissionPicker
    );
    assert!(
        app.fleet_overlay
            .mission_picker_status
            .as_deref()
            .is_some_and(
                |status| status.ends_with("fleet is already in your only controlled star system.")
            )
    );
    assert_ne!(
        app.game_data.fleets[record - 1].order,
        nc_data::Order::SeekHome
    );
}

#[test]
fn coordinate_fleet_order_defaults_match_engine_recommendations() {
    let mut app = audit_ready_dash_app();
    app.overlay = ActiveOverlay::FleetList;

    for mission_code in [0, 1, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14] {
        select_first_fleet_row(&mut app);
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = mission_code.to_string();
        app.submit_fleet_mission_picker();

        let selected_row = app.selected_fleet_order_row().expect("selected fleet row");
        let snapshots = app
            .planet_intel_snapshots
            .iter()
            .cloned()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        let expected_target = recommended_coordinate_target(
            &app.game_data,
            &snapshots,
            app.player_record_index_1_based as u8,
            mission_code,
            selected_row.coords,
            &BTreeSet::new(),
        );

        assert_eq!(
            app.fleet_order_target_x_default_value(),
            expected_target
                .map(|coords| format!("{:02}", coords[0]))
                .unwrap_or_default(),
            "mission {mission_code} XX default drifted",
        );

        let expected_y = recommended_coordinate_target_y_for_entered_x(
            &app.game_data,
            &snapshots,
            app.player_record_index_1_based as u8,
            mission_code,
            selected_row.coords,
            &BTreeSet::new(),
            "",
        );
        assert_eq!(
            app.fleet_order_target_y_default_value(),
            expected_y
                .map(|value| format!("{value:02}"))
                .unwrap_or_default(),
            "mission {mission_code} YY default drifted",
        );

        app.fleet_overlay.order_target_x_input = app.fleet_order_target_x_default_value();
        let expected_y_for_entered_x = recommended_coordinate_target_y_for_entered_x(
            &app.game_data,
            &snapshots,
            app.player_record_index_1_based as u8,
            mission_code,
            selected_row.coords,
            &BTreeSet::new(),
            &app.fleet_overlay.order_target_x_input,
        );
        assert_eq!(
            app.fleet_order_target_y_default_value(),
            expected_y_for_entered_x
                .map(|value| format!("{value:02}"))
                .unwrap_or_default(),
            "mission {mission_code} YY entered-X adaptation drifted",
        );
    }
}

#[test]
fn join_fleet_empty_submission_uses_default_target() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    let selected_record = match fleet_list::table_rows(&app)[app.fleet_overlay.selected].key {
        FleetOverlayRowKey::Fleet(record_index) => record_index,
        FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
    };
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::JoinAnotherFleet.code().to_string();

    app.submit_fleet_mission_picker();

    let expected_host_fleet_number = app
        .fleet_order_target_default_value()
        .parse::<u16>()
        .expect("default host fleet number");
    app.submit_fleet_order().expect("submit join target");

    let selected_fleet = &app.game_data.fleets[selected_record - 1];
    let expected_host = app
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == expected_host_fleet_number)
        .expect("default host fleet");
    assert_eq!(selected_fleet.order, Order::JoinAnotherFleet);
    assert_eq!(
        selected_fleet.target.to_array(),
        expected_host.sector.to_array()
    );
}

#[test]
fn guard_starbase_empty_submission_uses_default_target() {
    let mut app = dash_app_with_starbase_store();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    let selected_record = match fleet_list::table_rows(&app)[app.fleet_overlay.selected].key {
        FleetOverlayRowKey::Fleet(record_index) => record_index,
        FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
    };
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::GuardStarbase.code().to_string();

    app.submit_fleet_mission_picker();

    let expected_base_id = app
        .fleet_order_target_default_value()
        .parse::<u16>()
        .expect("default starbase id");
    app.submit_fleet_order().expect("submit guard target");

    let selected_fleet = &app.game_data.fleets[selected_record - 1];
    let expected_base = app
        .game_data
        .bases
        .iter()
        .find(|base| base.base_id.0 == expected_base_id)
        .expect("default starbase");
    assert_eq!(selected_fleet.order, Order::GuardStarbase);
    assert_eq!(
        selected_fleet.target.to_array(),
        expected_base.sector.to_array()
    );
}

#[test]
fn view_world_empty_coordinate_submission_uses_unknown_intel_default() {
    let mut app = audit_ready_dash_app();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::ViewWorld.code().to_string();

    app.submit_fleet_mission_picker();

    let expected_x = app.fleet_order_target_x_default_value();
    let expected_y = app.fleet_order_target_y_default_value();
    assert!(!expected_x.is_empty());
    assert!(!expected_y.is_empty());

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.fleet_overlay.order_target_x_input, expected_x);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderTargetY
    );

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.fleet_overlay.order_target_y_input, expected_y);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderConfirm
    );
}

#[test]
fn view_world_falls_back_to_closest_non_owned_world_when_unknowns_are_exhausted() {
    let mut app = audit_ready_dash_app();
    for snapshot in &mut app.planet_intel_snapshots {
        if snapshot.known_owner_empire_id == Some(1) {
            continue;
        }
        snapshot.intel_tier = IntelTier::Partial;
        if snapshot.known_owner_empire_id.is_none() {
            snapshot.known_owner_empire_id = Some(0);
        }
    }
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::ViewWorld.code().to_string();

    app.submit_fleet_mission_picker();

    let selected_row = app.selected_fleet_order_row().expect("selected fleet row");
    let snapshots = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let expected_target = recommended_coordinate_target(
        &app.game_data,
        &snapshots,
        app.player_record_index_1_based as u8,
        Order::ViewWorld.code(),
        selected_row.coords,
        &BTreeSet::new(),
    )
    .expect("fallback target");

    assert_eq!(
        app.fleet_order_target_x_default_value(),
        format!("{:02}", expected_target[0])
    );
    assert_eq!(
        app.fleet_order_target_y_default_value(),
        format!("{:02}", expected_target[1])
    );
}

#[test]
fn fleet_selection_toggles_on_space() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    let rows = fleet_list::table_rows(&app);
    let record_index = match rows[0].key {
        FleetOverlayRowKey::Fleet(record_index) => record_index,
        FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
    };

    app.handle_key(key(KeyCode::Char(' ')));
    assert!(
        app.fleet_overlay
            .selected_fleet_record_indexes
            .contains(&record_index)
    );

    app.handle_key(key(KeyCode::Char(' ')));
    assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
}

#[test]
fn fleet_sort_preserves_checked_selection() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    let rows = fleet_list::table_rows(&app);
    let record_index = match rows[0].key {
        FleetOverlayRowKey::Fleet(record_index) => record_index,
        FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
    };
    app.handle_key(key(KeyCode::Char(' ')));

    app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Eta);

    assert!(
        app.fleet_overlay
            .selected_fleet_record_indexes
            .contains(&record_index)
    );
}

#[test]
fn fleet_sort_repeated_selection_toggles_direction() {
    let mut app = dash_app();

    assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Desc);

    app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Id);
    assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Asc);

    app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Id);
    assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Desc);
}

#[test]
fn fleet_sort_new_key_resets_default_direction() {
    let mut app = dash_app();
    app.fleet_overlay.sort_direction = SortDirection::Asc;

    app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Strength);

    assert_eq!(
        app.fleet_overlay.sort,
        crate::dashboard::app::state::FleetOverlaySort::Strength
    );
    assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Desc);
}

#[test]
fn intel_range_sort_same_anchor_toggles_direction() {
    let mut app = dash_app();
    let anchor = [8, 8];

    app.apply_intel_overlay_sort(crate::dashboard::app::state::IntelOverlaySort::Range(
        anchor,
    ));
    assert_eq!(app.intel_overlay.sort_direction, SortDirection::Asc);

    app.apply_intel_overlay_sort(crate::dashboard::app::state::IntelOverlaySort::Range(
        anchor,
    ));
    assert_eq!(app.intel_overlay.sort_direction, SortDirection::Desc);

    app.apply_intel_overlay_sort(crate::dashboard::app::state::IntelOverlaySort::Range([
        9, 9,
    ]));
    assert_eq!(app.intel_overlay.sort_direction, SortDirection::Asc);
}

#[test]
fn fleet_filter_clears_checked_selection() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.handle_key(key(KeyCode::Char(' ')));

    app.apply_fleet_overlay_filter(FleetOverlayFilter::Combat);

    assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
}

#[test]
fn fleet_list_excludes_starbases() {
    let mut app = dash_app_with_starbase();
    app.overlay = ActiveOverlay::FleetList;
    assert!(
        fleet_list::table_rows(&app)
            .iter()
            .all(|row| !matches!(row.key, FleetOverlayRowKey::Starbase(_)))
    );
}

#[test]
fn checked_fleets_open_group_order_flow() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.handle_key(key(KeyCode::Char(' ')));
    app.fleet_overlay.selected = 1;
    app.handle_key(key(KeyCode::Char(' ')));
    app.fleet_overlay.selected = 0;

    app.handle_key(key(KeyCode::Char('o')));

    assert_eq!(app.fleet_overlay.order_scope, FleetOrderScope::Group);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MissionPicker
    );
}

#[test]
fn checked_fleets_still_open_group_order_flow_when_starbases_exist_in_game() {
    let mut app = dash_app_with_starbase();
    app.overlay = ActiveOverlay::FleetList;
    app.fleet_overlay.selected = fleet_list::table_rows(&app)
        .iter()
        .position(|row| matches!(row.key, FleetOverlayRowKey::Fleet(_)))
        .expect("fleet row");
    app.handle_key(key(KeyCode::Char(' ')));

    app.handle_key(key(KeyCode::Char('o')));

    assert_eq!(app.fleet_overlay.order_scope, FleetOrderScope::Group);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MissionPicker
    );
    assert_eq!(app.fleet_overlay.selected_fleet_record_indexes.len(), 1);
}

#[test]
fn group_order_success_clears_checked_selection_and_unwinds_prompts() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    let selected_records = select_first_two_fleet_rows(&mut app);

    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::MoveOnly.code().to_string();
    app.submit_fleet_mission_picker();
    app.fleet_overlay.order_target_x_input = "10".to_string();
    app.submit_fleet_order().expect("submit x");
    app.fleet_overlay.order_target_y_input = "10".to_string();
    app.submit_fleet_order().expect("submit y");
    app.fleet_overlay.order_confirm_input = "Y".to_string();
    app.submit_fleet_order().expect("submit confirm");

    assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert_eq!(app.fleet_overlay.order_scope, FleetOrderScope::None);
    for record_index in selected_records {
        let fleet = &app.game_data.fleets[record_index - 1];
        assert_eq!(fleet.order, Order::MoveOnly);
        assert_eq!(fleet.target.to_array(), [10, 10]);
    }
}

#[test]
fn fleet_order_confirm_y_submits_without_enter() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;

    app.open_selected_fleet_order_flow();
    let selected_record_index = app
        .selected_fleet_order_row()
        .expect("selected fleet row")
        .fleet_record_index_1_based;
    app.fleet_overlay.mission_picker_input = Order::MoveOnly.code().to_string();
    app.submit_fleet_mission_picker();
    app.fleet_overlay.order_target_x_input = "10".to_string();
    app.submit_fleet_order().expect("submit x");
    app.fleet_overlay.order_target_y_input = "10".to_string();
    app.submit_fleet_order().expect("submit y");

    app.handle_key(key(KeyCode::Char('y')));

    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    let fleet = &app.game_data.fleets[selected_record_index - 1];
    assert_eq!(fleet.order, Order::MoveOnly);
    assert_eq!(fleet.target.to_array(), [10, 10]);
}

#[test]
fn backing_out_of_group_order_keeps_checked_selection() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    let selected_records = select_first_two_fleet_rows(&mut app);

    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::MoveOnly.code().to_string();
    app.submit_fleet_mission_picker();
    app.handle_key(key(KeyCode::Esc));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MissionPicker
    );
    assert_eq!(
        app.fleet_overlay.selected_fleet_record_indexes,
        selected_records.into_iter().collect()
    );
}

#[test]
fn checked_change_applies_roe_to_all_checked_fleets() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    let selected_records = select_first_two_fleet_rows(&mut app);

    app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeField
    );

    app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeValue
    );

    app.handle_key(key(KeyCode::Char('4')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
    assert_eq!(
        app.fleet_overlay.aux_status.as_deref(),
        Some("Set ROE 4 for 2 checked fleets.")
    );
    for record_index in selected_records {
        let fleet = &app.game_data.fleets[record_index - 1];
        assert_eq!(fleet.rules_of_engagement, 4);
    }
}

#[test]
fn fleet_change_prompt_renders_inline_in_fleet_list_footer() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;

    app.handle_key(key(KeyCode::Char('c')));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeField
    );
    assert!(render_fleet_title_line(&app, "FLEET LIST:").contains("FLEET LIST:"));
    assert!(
        render_fleet_footer_line(&app, "Change <R>OE, <I>D, or <S>peed")
            .contains("COMMAND <- Change <R>OE, <I>D, or <S>peed <ESC> ->")
    );
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("CHANGE FLEET"))
    );
}

#[test]
fn checked_fleet_change_rejects_id_field_inline() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    select_first_two_fleet_rows(&mut app);

    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Char('i')));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeField
    );
    assert_eq!(
        app.fleet_overlay.aux_status.as_deref(),
        Some("Enter R or S.")
    );
}

#[test]
fn selected_fleet_change_accepts_roe_id_and_speed() {
    let mut roe_app = dash_app_with_store();
    roe_app.overlay = ActiveOverlay::FleetList;
    let roe_row = roe_app
        .selected_fleet_order_row_from_table()
        .expect("selected fleet row");
    {
        let fleet = &mut roe_app.game_data.fleets[roe_row.fleet_record_index_1_based - 1];
        fleet.ships.destroyers = 1;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        nc_data::set_troop_transports(fleet, 0);
        nc_data::set_armies_aboard(fleet, 0);
        fleet.ships.etacs = 0;
        nc_data::recompute_max_speed(fleet);
        fleet.rules_of_engagement = 0;
    }
    roe_app.handle_key(key(KeyCode::Char('c')));
    roe_app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(
        roe_app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeValue
    );
    roe_app.handle_key(key(KeyCode::Char('3')));
    roe_app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        roe_app.game_data.fleets[roe_row.fleet_record_index_1_based - 1].rules_of_engagement,
        3
    );
    let roe_notice = format!("Fleet #{} ROE set to 3.", roe_row.fleet_number);
    assert_eq!(
        roe_app.fleet_overlay.aux_status.as_deref(),
        Some(roe_notice.as_str())
    );

    let mut id_app = dash_app_with_store();
    id_app.overlay = ActiveOverlay::FleetList;
    let id_row = id_app
        .selected_fleet_order_row_from_table()
        .expect("selected fleet row");
    let new_id = id_row.fleet_number + 100;
    id_app.handle_key(key(KeyCode::Char('c')));
    id_app.handle_key(key(KeyCode::Char('i')));
    assert_eq!(
        id_app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeValue
    );
    for ch in new_id.to_string().chars() {
        id_app.handle_key(key(KeyCode::Char(ch)));
    }
    id_app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        id_app.game_data.fleets[id_row.fleet_record_index_1_based - 1]
            .fleet_number
            .0,
        new_id
    );
    let id_notice = format!(
        "Fleet #{} renumbered to Fleet #{}.",
        id_row.fleet_number, new_id
    );
    assert_eq!(
        id_app.fleet_overlay.aux_status.as_deref(),
        Some(id_notice.as_str())
    );

    let mut speed_app = dash_app_with_store();
    speed_app.overlay = ActiveOverlay::FleetList;
    let speed_row = speed_app
        .selected_fleet_order_row_from_table()
        .expect("selected fleet row");
    // A fleet on its way somewhere; one at its target keeps speed 0.
    let moving = &mut speed_app.game_data.fleets[speed_row.fleet_record_index_1_based - 1];
    moving.order = nc_data::Order::MoveOnly;
    moving.target = nc_data::Coords::from_array([1, 1]);
    speed_app.handle_key(key(KeyCode::Char('c')));
    speed_app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(
        speed_app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeValue
    );
    speed_app.handle_key(key(KeyCode::Char('1')));
    speed_app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        speed_app.game_data.fleets[speed_row.fleet_record_index_1_based - 1].speed,
        1
    );
    let speed_notice = format!("Fleet #{} speed set to 1.", speed_row.fleet_number);
    assert_eq!(
        speed_app.fleet_overlay.aux_status.as_deref(),
        Some(speed_notice.as_str())
    );
}

#[test]
fn checked_change_clears_only_successful_fleets_on_partial_roe_update() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    let selected_records = select_first_two_fleet_rows(&mut app);
    let combat_record = selected_records[0];
    let support_record = selected_records[1];

    {
        let combat = &mut app.game_data.fleets[combat_record - 1];
        combat.ships.destroyers = 1;
        combat.ships.cruisers = 0;
        combat.ships.battleships = 0;
        combat.ships.scouts = 0;
        nc_data::set_troop_transports(combat, 0);
        nc_data::set_armies_aboard(combat, 0);
        combat.ships.etacs = 0;
        nc_data::recompute_max_speed(combat);
        combat.rules_of_engagement = 0;

        let support = &mut app.game_data.fleets[support_record - 1];
        support.ships.destroyers = 0;
        support.ships.cruisers = 0;
        support.ships.battleships = 0;
        support.ships.scouts = 0;
        nc_data::set_troop_transports(support, 1);
        nc_data::set_armies_aboard(support, 1);
        support.ships.etacs = 0;
        nc_data::recompute_max_speed(support);
        support.rules_of_engagement = 0;
    }

    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Char('r')));
    app.handle_key(key(KeyCode::Char('6')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::ChangeValue
    );
    assert_eq!(
        app.fleet_overlay.selected_fleet_record_indexes,
        [support_record].into_iter().collect()
    );
    assert_eq!(
        app.game_data.fleets[combat_record - 1].rules_of_engagement,
        6
    );
    assert_eq!(
        app.game_data.fleets[support_record - 1].rules_of_engagement,
        0
    );
    assert!(app.fleet_overlay.aux_status.is_some());
}

#[test]
fn checked_merge_uses_lowest_numbered_host() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    let selected_records = select_first_two_fleet_rows(&mut app);
    let mut selected_fleets = selected_records
        .iter()
        .map(|record_index| {
            let fleet = &app.game_data.fleets[*record_index - 1];
            (*record_index, fleet.fleet_number.0, fleet.sector.to_array())
        })
        .collect::<Vec<_>>();
    selected_fleets.sort_by_key(|(_, fleet_number, _)| *fleet_number);
    let host_record_index = selected_fleets[0].0;
    let host_coords = selected_fleets[0].2;

    app.handle_key(key(KeyCode::Char('m')));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MergeConfirm
    );

    app.handle_key(key(KeyCode::Char('y')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
    for (record_index, _, _) in selected_fleets.into_iter().skip(1) {
        let fleet = &app.game_data.fleets[record_index - 1];
        assert_eq!(fleet.order, Order::JoinAnotherFleet);
        assert_eq!(fleet.target.to_array(), host_coords);
    }
    let host = &app.game_data.fleets[host_record_index - 1];
    assert_ne!(host.order, Order::JoinAnotherFleet);
}

#[test]
fn checked_transfer_uses_highlighted_checked_fleet_as_donor() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    let selected_records = select_first_two_fleet_rows(&mut app);
    let donor_record_index = selected_records[1];
    app.fleet_overlay.selected = fleet_list::table_rows(&app)
        .iter()
        .position(|row| row.key == FleetOverlayRowKey::Fleet(donor_record_index))
        .expect("selected donor row");

    app.handle_key(key(KeyCode::Char('t')));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::TransferStage
    );
    assert_eq!(
        app.fleet_overlay.transfer_donor_record_index_1_based,
        Some(donor_record_index)
    );
    assert_eq!(
        app.fleet_overlay.transfer_host_record_index_1_based,
        Some(selected_records[0])
    );
}

#[test]
fn guard_starbase_mission_reports_specific_unavailable_target_message() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::GuardStarbase.code().to_string();

    app.submit_fleet_mission_picker();

    assert_eq!(
        app.fleet_overlay.mission_picker_status.as_deref(),
        Some("You have no starbases available to guard.")
    );
}

#[test]
fn join_fleet_mission_reports_specific_unavailable_target_message() {
    let mut app = dash_app();
    app.game_data.fleets.truncate(1);
    app.overlay = ActiveOverlay::FleetList;
    app.open_selected_fleet_order_flow();
    app.fleet_overlay.mission_picker_input = Order::JoinAnotherFleet.code().to_string();

    app.submit_fleet_mission_picker();

    assert_eq!(
        app.fleet_overlay.mission_picker_status.as_deref(),
        Some("You need another fleet available to join.")
    );
}

#[test]
fn fleet_order_default_target_ignores_existing_target_coords_shortcut() {
    let mut baseline = dash_app();
    baseline.overlay = ActiveOverlay::FleetList;
    baseline.open_selected_fleet_order_flow();
    baseline.fleet_overlay.order_mission_code = Some(Order::MoveOnly.code());
    baseline.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
    let baseline_footer = render_fleet_footer_line(&baseline, "COMMAND <- Target XX ");

    let mut stale_target = dash_app();
    let selected_record =
        match fleet_list::table_rows(&stale_target)[stale_target.fleet_overlay.selected].key {
            FleetOverlayRowKey::Fleet(record_index) => record_index,
            FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
        };
    stale_target.game_data.fleets[selected_record - 1].target =
        nc_data::Coords::from_array([18, 18]);
    stale_target.overlay = ActiveOverlay::FleetList;
    stale_target.open_selected_fleet_order_flow();
    stale_target.fleet_overlay.order_mission_code = Some(Order::MoveOnly.code());
    stale_target.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
    let stale_footer = render_fleet_footer_line(&stale_target, "COMMAND <- Target XX ");

    assert_eq!(stale_footer, baseline_footer);
}

#[test]
fn nested_intel_filter_modals_unwind_one_level_at_a_time() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;

    app.handle_key(key(KeyCode::Char('f')));
    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Char('o')));
    app.handle_key(key(KeyCode::Char('o')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.intel_overlay.prompt_mode,
        IntelOverlayPromptMode::FilterValueInput
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert_eq!(app.intel_overlay.prompt_mode, IntelOverlayPromptMode::None);
}

#[test]
fn intel_database_enter_on_owned_planet_opens_status_and_escape_returns_to_database() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;
    let record = first_owned_planet_record_index(&app);
    select_intel_row_by_record(&mut app, record);

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert_eq!(
        app.popup,
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based: record
        }
    );
    assert!(render_dashboard_line(&app, "PLANET STATUS").contains("PLANET STATUS"));
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("TOTAL PLANET DATABASE:"))
    );

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn intel_database_enter_on_non_owned_planet_opens_info_and_escape_returns_to_database() {
    let mut app = dash_app();
    app.planet_intel_snapshots = view_world_audit_snapshots(&app.game_data, 1);
    app.overlay = ActiveOverlay::IntelDatabase;
    let record = first_non_owned_planet_record_index(&app);
    select_intel_row_by_record(&mut app, record);

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert_eq!(
        app.popup,
        ActivePopup::PlanetDetail {
            planet_record_index_1_based: record
        }
    );
    assert!(render_dashboard_line(&app, "PLANET INFO").contains("PLANET INFO"));
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("TOTAL PLANET DATABASE:"))
    );

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn fleet_list_enter_on_fleet_opens_review_and_escape_returns_to_list() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    let record = match fleet_list::table_rows(&app)[app.fleet_overlay.selected].key {
        FleetOverlayRowKey::Fleet(record) => record,
        FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
    };
    {
        let fleet = &mut app.game_data.fleets[record - 1];
        nc_data::set_troop_transports(fleet, 3);
        nc_data::set_armies_aboard(fleet, 2);
        nc_data::recompute_max_speed(fleet);
    }

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    let fleet_number = app.game_data.fleets[record - 1].fleet_number.0;
    assert_eq!(
        app.popup,
        ActivePopup::FleetDetail {
            fleet_record_index_1_based: record
        }
    );
    assert!(
        render_dashboard_line(&app, &format!("REVIEW FLEET {fleet_number}"))
            .contains(&format!("REVIEW FLEET {fleet_number}"))
    );
    let lines = render_dashboard_lines(&app);
    assert!(!lines.iter().any(|line| line.contains("FLEET LIST:")));
    assert!(!lines.iter().any(|line| line.contains("Fleet ID")));
    assert!(!lines.iter().any(|line| line.contains("Standing Order")));
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("Current / Max Speed"))
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("Rules of Engagement"))
    );
    assert!(!lines.iter().any(|line| line.contains("Composition")));
    let location_line = render_fleet_detail_popup_line(&app, "Location");
    let current_speed_line = render_fleet_detail_popup_line(&app, "Speed");
    let max_speed_line = render_fleet_detail_popup_line(&app, "Max Speed");
    let roe_line = render_fleet_detail_popup_line(&app, "ROE");
    let orders_line = render_fleet_detail_popup_line(&app, "Orders");
    let target_line = render_fleet_detail_popup_line(&app, "Target");
    let ships_line = render_fleet_detail_popup_line(&app, "Ships");
    let armies_line = render_fleet_detail_popup_line(&app, "Armies");
    assert!(ships_line.contains("TT*"), "{ships_line}");
    assert!(armies_line.contains("2"), "{armies_line}");
    assert_eq!(location_line.find(" : "), current_speed_line.find(" : "));
    assert_eq!(current_speed_line.find(" : "), max_speed_line.find(" : "));
    assert_eq!(max_speed_line.find(" : "), roe_line.find(" : "));
    assert_eq!(orders_line.find(" : "), target_line.find(" : "));
    assert_eq!(target_line.find(" : "), ships_line.find(" : "));
    assert_eq!(ships_line.find(" : "), armies_line.find(" : "));

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn fleet_review_shows_crippled_ships_in_ship_notation() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    select_first_fleet_row(&mut app);
    let record = match fleet_list::table_rows(&app)[app.fleet_overlay.selected].key {
        FleetOverlayRowKey::Fleet(record) => record,
        FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
    };
    {
        let fleet = &mut app.game_data.fleets[record - 1];
        fleet.ships.battleships = 1;
        fleet.ships.destroyers = 3;
    }
    app.handle_key(key(KeyCode::Enter));
    let ships = render_fleet_detail_popup_line(&app, "Ships");
    assert!(ships.contains("BB1 DD3"), "{ships}");
    assert!(!ships.contains('!'), "{ships}");
    app.handle_key(key(KeyCode::Esc));

    {
        let fleet = &mut app.game_data.fleets[record - 1];
        fleet.crippled.destroyers.fresh = 2;
        fleet.crippled.battleships.turns[1] = 1;
    }
    app.handle_key(key(KeyCode::Enter));
    let ships = render_fleet_detail_popup_line(&app, "Ships");
    assert!(ships.contains("BB1!1 DD3!2"), "{ships}");
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("Damaged"))
    );
}

#[test]
fn fleet_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;

    app.handle_key(key(KeyCode::Char('f')));
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::FilterMenu
    );
    assert_eq!(
        app.fleet_overlay.filter_prompt_status.as_deref(),
        Some(" Ambiguous: sel/shi/spd")
    );
    assert!(
        render_fleet_footer_line(&app, "Ambiguous: sel/shi/spd")
            .contains("COMMAND <-  Ambiguous: sel/shi/spd")
    );

    app.handle_key(key(KeyCode::Char('p')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::FilterValueInput
    );
    assert_eq!(
        app.fleet_overlay
            .pending_filter_column
            .expect("pending column")
            .code,
        "spd"
    );
}

#[test]
fn intel_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;

    app.handle_key(key(KeyCode::Char('f')));
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert_eq!(
        app.intel_overlay.prompt_mode,
        IntelOverlayPromptMode::FilterMenu
    );
    assert_eq!(
        app.intel_overlay.prompt_status.as_deref(),
        Some(" Ambiguous: sbs/sco/see")
    );
    assert!(
        render_intel_footer_line(&app, "Ambiguous: sbs/sco/see")
            .contains("COMMAND <-  Ambiguous: sbs/sco/see")
    );

    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.intel_overlay.prompt_mode,
        IntelOverlayPromptMode::FilterValueInput
    );
    assert_eq!(
        app.intel_overlay
            .pending_filter_column
            .expect("pending column")
            .code,
        "sco"
    );
}

#[test]
fn empty_fleet_filter_clause_resets_to_all() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;

    app.handle_key(key(KeyCode::Char('f')));
    for ch in ['o', 'r', 'd'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    for ch in ['z', 'z', 'z', 'z'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert!(app.fleet_overlay.filter_clause.is_none());
    assert!(!fleet_list::table_rows(&app).is_empty());
}

#[test]
fn empty_planet_filter_clause_resets_to_all() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;

    app.handle_key(key(KeyCode::Char('f')));
    for ch in ['p', 'l', 'a'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    for ch in ['z', 'z', 'z', 'z'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert!(app.planet_overlay.filter_clause.is_none());
    assert!(!planet_list::table_rows(&app).is_empty());
}

#[test]
fn empty_intel_filter_clause_resets_to_all() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;

    app.handle_key(key(KeyCode::Char('f')));
    for ch in ['p', 'l', 'a'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    for ch in ['z', 'z', 'z', 'z'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
    assert!(app.intel_overlay.filter_clause.is_none());
    assert!(!intel_database::table_rows(&app).is_empty());
}

#[test]
fn stale_fleet_filter_clause_resets_to_all_after_rows_change() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;

    let owned_fleet_indexes = app
        .game_data
        .fleets
        .iter()
        .enumerate()
        .filter_map(|(idx, fleet)| (fleet.owner.0 == 1).then_some(idx))
        .collect::<Vec<_>>();
    assert!(
        !owned_fleet_indexes.is_empty(),
        "fixture should have owned fleets"
    );
    let target_fleet_number = app.game_data.fleets[owned_fleet_indexes[0]].fleet_number.0;

    app.handle_key(key(KeyCode::Char('f')));
    for ch in ['i', 'd'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    for ch in target_fleet_number.to_string().chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert!(app.fleet_overlay.filter_clause.is_some());

    app.game_data.fleets[owned_fleet_indexes[0]].owner.0 = 0;
    app.handle_key(key(KeyCode::Down));

    assert!(app.fleet_overlay.filter_clause.is_none());
    assert!(!fleet_list::table_rows(&app).is_empty());
    assert!(render_fleet_title_line(&app, "FLEET LIST:").contains("FLEET LIST: ID DESCENDING ALL"));
}

#[test]
fn stale_planet_filter_clause_resets_to_all_after_rows_change() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;

    let owned_planet_indexes = app
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter_map(|(idx, planet)| (planet.owner.0 == 1).then_some(idx))
        .collect::<Vec<_>>();
    let target_planet_idx = *owned_planet_indexes.first().expect("owned planet exists");
    for &idx in &owned_planet_indexes {
        app.game_data.planets[idx].armies = 0;
    }
    app.game_data.planets[target_planet_idx].armies = 5;

    app.handle_key(key(KeyCode::Char('f')));
    for ch in ['a', 'r', 's'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    for ch in "5".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert!(app.planet_overlay.filter_clause.is_some());

    app.game_data.planets[target_planet_idx].armies = 0;
    app.handle_key(key(KeyCode::Down));

    assert!(app.planet_overlay.filter_clause.is_none());
    assert!(!planet_list::table_rows(&app).is_empty());
    assert!(
        render_planet_title_line(&app, "PLANET LIST:").contains("PLANET LIST: CUR DESCENDING ALL")
    );
}

#[test]
fn stale_intel_filter_clause_resets_to_all_after_rows_change() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;

    for planet in &mut app.game_data.planets {
        planet.armies = 0;
    }
    let target_planet_idx = app
        .game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .expect("owned planet exists");
    app.game_data.planets[target_planet_idx].armies = 7;

    app.handle_key(key(KeyCode::Char('f')));
    for ch in ['a', 'r', 's'] {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    for ch in "7".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert!(app.intel_overlay.filter_clause.is_some());

    app.game_data.planets[target_planet_idx].armies = 0;
    app.handle_key(key(KeyCode::Down));

    assert!(app.intel_overlay.filter_clause.is_none());
    assert!(!intel_database::table_rows(&app).is_empty());
    assert!(
        render_intel_title_line(&app, "TOTAL PLANET DATABASE:")
            .contains("TOTAL PLANET DATABASE: COO ASCENDING ALL")
    );
}

#[test]
fn fleet_sort_prompt_accepts_every_column_code() {
    let cases = [
        ("id", FleetOverlaySort::Id, SortDirection::Asc, "ID"),
        (
            "sel",
            FleetOverlaySort::Selected,
            SortDirection::Desc,
            "SEL",
        ),
        ("loc", FleetOverlaySort::Location, SortDirection::Asc, "LOC"),
        ("ord", FleetOverlaySort::Order, SortDirection::Asc, "ORD"),
        ("tar", FleetOverlaySort::Target, SortDirection::Asc, "TAR"),
        ("spd", FleetOverlaySort::Speed, SortDirection::Desc, "SPD"),
        ("eta", FleetOverlaySort::Eta, SortDirection::Asc, "ETA"),
        ("roe", FleetOverlaySort::Roe, SortDirection::Desc, "ROE"),
        ("ars", FleetOverlaySort::Armies, SortDirection::Desc, "ARS"),
        (
            "shi",
            FleetOverlaySort::Strength,
            SortDirection::Desc,
            "SHI",
        ),
    ];

    for (code, expected_sort, expected_direction, expected_label) in cases {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::SortMenu
        );
        for ch in code.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
        assert_eq!(app.fleet_overlay.sort, expected_sort);
        assert_eq!(app.fleet_overlay.sort_direction, expected_direction);

        let title = render_fleet_title_line(&app, "FLEET LIST:");
        assert!(title.contains(expected_label), "{title}");
        assert!(
            title.contains(match expected_direction {
                SortDirection::Asc => "ASCENDING",
                SortDirection::Desc => "DESCENDING",
            }),
            "{title}"
        );
    }
}

#[test]
fn planet_sort_prompt_accepts_every_column_code() {
    let cases = [
        (
            "coo",
            PlanetOverlaySort::Location,
            SortDirection::Asc,
            "COO",
        ),
        (
            "pla",
            PlanetOverlaySort::PlanetName,
            SortDirection::Asc,
            "PLA",
        ),
        (
            "max",
            PlanetOverlaySort::MaxProduction,
            SortDirection::Desc,
            "MAX",
        ),
        (
            "cur",
            PlanetOverlaySort::CurrentProduction,
            SortDirection::Asc,
            "CUR",
        ),
        (
            "trs",
            PlanetOverlaySort::Treasury,
            SortDirection::Desc,
            "TRS",
        ),
        ("bdg", PlanetOverlaySort::Budget, SortDirection::Desc, "BDG"),
        (
            "rev",
            PlanetOverlaySort::Revenue,
            SortDirection::Desc,
            "REV",
        ),
        ("gro", PlanetOverlaySort::Growth, SortDirection::Desc, "GRO"),
        (
            "bui",
            PlanetOverlaySort::BuildQueue,
            SortDirection::Desc,
            "BUI",
        ),
        (
            "sta",
            PlanetOverlaySort::Stardock,
            SortDirection::Desc,
            "STA",
        ),
        (
            "sbs",
            PlanetOverlaySort::Starbase,
            SortDirection::Desc,
            "SBS",
        ),
        ("ars", PlanetOverlaySort::Armies, SortDirection::Desc, "ARS"),
        (
            "gbs",
            PlanetOverlaySort::Batteries,
            SortDirection::Desc,
            "GBS",
        ),
    ];

    for (code, expected_sort, expected_direction, expected_label) in cases {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::SortMenu
        );
        for ch in code.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::None
        );
        assert_eq!(app.planet_overlay.sort, expected_sort);
        assert_eq!(app.planet_overlay.sort_direction, expected_direction);

        let title = render_planet_title_line(&app, "PLANET LIST:");
        assert!(title.contains(expected_label), "{title}");
        assert!(
            title.contains(match expected_direction {
                SortDirection::Asc => "ASCENDING",
                SortDirection::Desc => "DESCENDING",
            }),
            "{title}"
        );
    }
}

#[test]
fn intel_sort_prompt_accepts_every_column_code() {
    let cases = [
        (
            "coo",
            IntelOverlaySort::Location,
            SortDirection::Desc,
            "COO",
        ),
        (
            "pla",
            IntelOverlaySort::PlanetName,
            SortDirection::Asc,
            "PLA",
        ),
        ("own", IntelOverlaySort::Owner, SortDirection::Asc, "OWN"),
        (
            "max",
            IntelOverlaySort::MaxProduction,
            SortDirection::Desc,
            "MAX",
        ),
        (
            "see",
            IntelOverlaySort::YearSeen,
            SortDirection::Desc,
            "SEE",
        ),
        ("ars", IntelOverlaySort::Armies, SortDirection::Desc, "ARS"),
        (
            "gbs",
            IntelOverlaySort::Batteries,
            SortDirection::Desc,
            "GBS",
        ),
        (
            "sbs",
            IntelOverlaySort::Starbases,
            SortDirection::Desc,
            "SBS",
        ),
        (
            "cur",
            IntelOverlaySort::CurrentProduction,
            SortDirection::Desc,
            "CUR",
        ),
        (
            "trs",
            IntelOverlaySort::Treasury,
            SortDirection::Desc,
            "TRS",
        ),
        (
            "sco",
            IntelOverlaySort::ScoutYear,
            SortDirection::Desc,
            "SCO",
        ),
    ];

    for (code, expected_sort, expected_direction, expected_label) in cases {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;
        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(
            app.intel_overlay.prompt_mode,
            IntelOverlayPromptMode::SortMenu
        );
        for ch in code.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.intel_overlay.prompt_mode, IntelOverlayPromptMode::None);
        assert_eq!(app.intel_overlay.sort, expected_sort);
        assert_eq!(app.intel_overlay.sort_direction, expected_direction);

        let title = render_intel_title_line(&app, "TOTAL PLANET DATABASE:");
        assert!(title.contains(expected_label), "{title}");
        assert!(
            title.contains(match expected_direction {
                SortDirection::Asc => "ASCENDING",
                SortDirection::Desc => "DESCENDING",
            }),
            "{title}"
        );
    }
}

#[test]
fn fleet_filter_prompt_accepts_every_appendix_e_column_code() {
    let codes = [
        "id", "loc", "ord", "tar", "spd", "eta", "roe", "ars", "shi", "sel",
    ];

    for code in codes {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::FilterMenu
        );
        for ch in code.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::FilterValueInput
        );
        assert_eq!(
            app.fleet_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some(code)
        );
    }
}

#[test]
fn planet_filter_prompt_accepts_every_appendix_e_column_code() {
    let codes = [
        "coo", "pla", "max", "cur", "trs", "bdg", "rev", "gro", "bui", "sta", "sbs", "ars", "gbs",
    ];

    for code in codes {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::FilterMenu
        );
        for ch in code.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::FilterValueInput
        );
        assert_eq!(
            app.planet_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some(code)
        );
    }
}

#[test]
fn intel_filter_prompt_accepts_every_appendix_e_column_code() {
    let codes = [
        "coo", "pla", "own", "max", "see", "ars", "gbs", "sbs", "cur", "trs", "sco",
    ];

    for code in codes {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;
        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(
            app.intel_overlay.prompt_mode,
            IntelOverlayPromptMode::FilterMenu
        );
        for ch in code.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.intel_overlay.prompt_mode,
            IntelOverlayPromptMode::FilterValueInput
        );
        assert_eq!(
            app.intel_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some(code)
        );
    }
}

#[test]
fn sort_prompts_accept_natural_column_names() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.handle_key(key(KeyCode::Char('s')));
    for ch in "speed".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.fleet_overlay.sort, FleetOverlaySort::Speed);

    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.handle_key(key(KeyCode::Char('s')));
    for ch in "dock".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.planet_overlay.sort, PlanetOverlaySort::Stardock);

    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.handle_key(key(KeyCode::Char('s')));
    for ch in "treasury points".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.planet_overlay.sort, PlanetOverlaySort::Treasury);

    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.handle_key(key(KeyCode::Char('s')));
    for ch in "bgdt".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.planet_overlay.sort, PlanetOverlaySort::Budget);

    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;
    app.handle_key(key(KeyCode::Char('s')));
    for ch in "year".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.intel_overlay.sort, IntelOverlaySort::YearSeen);
}

#[test]
fn filter_prompts_accept_natural_column_names() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.handle_key(key(KeyCode::Char('f')));
    for ch in "speed".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.fleet_overlay
            .pending_filter_column
            .map(|column| column.code),
        Some("spd")
    );

    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.handle_key(key(KeyCode::Char('f')));
    for ch in "dock".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.planet_overlay
            .pending_filter_column
            .map(|column| column.code),
        Some("sta")
    );

    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.handle_key(key(KeyCode::Char('f')));
    for ch in "treasury points".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.planet_overlay
            .pending_filter_column
            .map(|column| column.code),
        Some("trs")
    );

    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.handle_key(key(KeyCode::Char('f')));
    for ch in "bgdt".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.planet_overlay
            .pending_filter_column
            .map(|column| column.code),
        Some("bdg")
    );

    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;
    app.handle_key(key(KeyCode::Char('f')));
    for ch in "year".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.intel_overlay
            .pending_filter_column
            .map(|column| column.code),
        Some("see")
    );
}

#[test]
fn dragging_top_level_overlay_updates_position() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        popup.x + 9,
        popup.y + 4,
    ));

    assert!(app.overlay_position.is_some());

    let moved_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("moved popup rect");
    assert!(moved_popup.x > popup.x);
    assert!(moved_popup.y > popup.y);
}

#[test]
fn dragging_overlay_from_bottom_border_updates_position() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 4,
        popup.y + popup.height.saturating_sub(1),
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        popup.x + 9,
        popup.y + popup.height + 3,
    ));

    let moved_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("moved popup rect");
    assert!(moved_popup.y > popup.y);
}

#[test]
fn dragging_overlay_can_move_into_left_column() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let widgets = dashboard_layout(&app).widgets;
    let map_frame = widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        widgets.left_economy.outer.col as u16 + 2,
        popup.y + 1,
    ));

    let moved_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("moved popup rect");
    assert!(moved_popup.x < widgets.center_map.outer.col as u16);
}

#[test]
fn clicking_overlay_close_button_closes_overlay_without_dragging() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");
    let close_col =
        crate::dashboard::modal::modal_close_button_col(popup).expect("overlay close col");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        close_col,
        popup.y,
    ));

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert!(!<DashApp as NativeApp>::is_dragging_surface(&app));
}

#[test]
fn dragging_planet_detail_popup_updates_position() {
    let mut app = dash_app();
    app.popup = ActivePopup::PlanetDetail {
        planet_record_index_1_based: 1,
    };
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_popup_rect(map_frame)
        .expect("planet detail popup");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        popup.x + 12,
        popup.y + 3,
    ));

    let moved_popup = app.current_popup_rect(map_frame).expect("moved popup");
    assert!(moved_popup.x > popup.x);
    assert!(moved_popup.y > popup.y);
}

#[test]
fn clicking_popup_close_button_closes_popup_without_dragging() {
    let mut app = dash_app();
    app.popup = ActivePopup::PlanetDetail {
        planet_record_index_1_based: 1,
    };
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_popup_rect(map_frame)
        .expect("planet detail popup");
    let close_col =
        crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        close_col,
        popup.y,
    ));

    assert_eq!(app.popup, ActivePopup::None);
    assert!(!<DashApp as NativeApp>::is_dragging_surface(&app));
}

#[test]
fn clicking_quit_confirm_close_button_closes_popup() {
    let mut app = dash_app();
    app.popup = ActivePopup::QuitConfirm;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_popup_rect(map_frame)
        .expect("quit confirm popup");
    let close_col =
        crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        close_col,
        popup.y,
    ));

    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn clicking_help_overlay_close_button_restores_underlay_overlay() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    app.open_overlay_help(HelpContext::Inbox);
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("help popup rect");
    let close_col = crate::dashboard::modal::modal_close_button_col(popup).expect("help close col");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        close_col,
        popup.y,
    ));

    assert_eq!(app.overlay, ActiveOverlay::Inbox);
    assert_eq!(app.help_return_overlay, ActiveOverlay::None);
}

#[test]
fn clicking_quit_confirm_close_button_restores_underlying_popup() {
    let mut app = dash_app();
    app.popup = ActivePopup::PlanetDetail {
        planet_record_index_1_based: 1,
    };
    app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_popup_rect(map_frame)
        .expect("quit confirm popup");
    let close_col =
        crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        close_col,
        popup.y,
    ));

    assert_eq!(
        app.popup,
        ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1
        }
    );
}

#[test]
fn clicking_quit_confirm_close_button_restores_underlying_overlay() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_popup_rect(map_frame)
        .expect("quit confirm popup");
    let close_col =
        crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        close_col,
        popup.y,
    ));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.popup, ActivePopup::None);
    assert!(render_dashboard_line(&app, "FLEET LIST").contains("FLEET LIST"));
}

#[test]
fn fleet_helper_modal_rect_is_draggable_surface() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::MissionPicker;
    let map_frame = dashboard_layout(&app).widgets.center_map;

    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("mission picker popup");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        popup.x + 8,
        popup.y + 2,
    ));

    assert!(app.overlay_position.is_some());
}

#[test]
fn closing_and_reopening_overlay_recenters_dragged_position() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        popup.x + 12,
        popup.y + 3,
    ));
    let moved_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("moved popup rect");
    assert_ne!(moved_popup.x, popup.x);

    app.close_active_overlay();
    app.apply_action(super::input::Action::OpenOverlay(ActiveOverlay::Inbox));

    let recentered_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("reopened popup rect");
    assert_eq!(recentered_popup.x, popup.x);
    assert_eq!(recentered_popup.y, popup.y);
}

#[test]
fn help_overlay_restores_dragged_underlay_position() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    app.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        popup.x + 10,
        popup.y + 2,
    ));
    let moved_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("moved popup rect");

    app.open_overlay_help(crate::dashboard::app::state::HelpContext::Inbox);
    app.close_active_overlay();

    let restored_popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("restored popup rect");
    assert_eq!(restored_popup.x, moved_popup.x);
    assert_eq!(restored_popup.y, moved_popup.y);
}

#[test]
fn dragging_overlay_reports_dragging_surface_state() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Inbox;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("inbox popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    assert!(<DashApp as NativeApp>::is_dragging_surface(&app));

    app.handle_mouse(mouse(
        MouseEventKind::Up(MouseButton::Left),
        popup.x + 2,
        popup.y,
    ));
    assert!(!<DashApp as NativeApp>::is_dragging_surface(&app));
}

#[test]
fn clicking_map_sector_moves_crosshair() {
    let mut app = dash_app();
    let target = first_empty_sector_coords(&app);
    let (column, row) = screen_point_for_sector(&app, target);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

    assert_eq!([app.crosshair_x, app.crosshair_y], target);
}

#[test]
fn hovering_visible_sector_moves_crosshair() {
    let mut app = dash_app();
    let target = first_empty_sector_coords(&app);
    let (column, row) = screen_point_for_sector(&app, target);

    app.handle_mouse(mouse(MouseEventKind::Moved, column, row));

    assert_eq!([app.crosshair_x, app.crosshair_y], target);
}

#[test]
fn dispatch_mouse_move_without_visible_change_reports_no_redraw() {
    let mut app = dash_app();
    let target = [app.crosshair_x, app.crosshair_y];
    let (column, row) = screen_point_for_sector(&app, target);

    assert!(!app.dispatch_mouse_event(mouse(MouseEventKind::Moved, column, row)));
}

#[test]
fn dispatch_mouse_move_with_crosshair_change_reports_redraw() {
    let mut app = dash_app();
    let target = first_empty_sector_coords(&app);
    let (column, row) = screen_point_for_sector(&app, target);

    assert!(app.dispatch_mouse_event(mouse(MouseEventKind::Moved, column, row)));
}

#[test]
fn moving_mouse_outside_map_widget_resets_crosshair_to_homeworld() {
    let mut app = dash_app();
    let homeworld = [app.crosshair_x, app.crosshair_y];
    let target = first_empty_sector_coords(&app);
    let (column, row) = screen_point_for_sector(&app, target);
    let outside = outside_map_point(&app);

    app.handle_mouse(mouse(MouseEventKind::Moved, column, row));
    assert_eq!([app.crosshair_x, app.crosshair_y], target);

    app.handle_mouse(mouse(MouseEventKind::Moved, outside.0, outside.1));

    assert_eq!([app.crosshair_x, app.crosshair_y], homeworld);
}

#[test]
fn left_click_on_sector_with_player_fleets_opens_filtered_fleet_list() {
    let mut app = dash_app_with_starbase();
    let fleet_coords = first_owned_fleet_coords(&app);
    let (column, row) = screen_point_for_sector(&app, fleet_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.fleet_overlay.location_filter, Some(fleet_coords));
    assert!(fleet_list::table_rows(&app).iter().all(|row| {
        matches!(row.key, FleetOverlayRowKey::Fleet(_)) && row.coords == fleet_coords
    }));
}

#[test]
fn left_click_on_empty_sector_fleet_glyph_opens_filtered_fleet_list() {
    let mut app = dash_app();
    let empty_coords = first_empty_sector_coords(&app);
    let fleet = app
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && nc_data::has_any_force(&fleet))
        .expect("owned fleet");
    fleet.sector = nc_data::Coords::from_array(empty_coords);
    let (column, row) = screen_point_for_sector(&app, empty_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.fleet_overlay.location_filter, Some(empty_coords));
    assert!(
        fleet_list::table_rows(&app)
            .iter()
            .all(|row| row.coords == empty_coords)
    );
}

#[test]
fn left_click_without_player_fleets_does_not_open_anything() {
    let mut app = audit_ready_dash_app();
    let target = first_visible_foreign_planet_coords(&mut app);
    let (column, row) = screen_point_for_sector(&app, target);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!([app.crosshair_x, app.crosshair_y], target);
}

#[test]
fn right_click_on_owned_planet_opens_owned_planet_popup() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = app
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1 && planet.sector.to_array() == owned_coords)
        .map(|(idx, _)| idx + 1)
        .expect("owned planet");
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.planet_overlay.filter = PlanetOverlayFilter::Starbase;
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(app.planet_overlay.filter, PlanetOverlayFilter::Starbase);
    assert_eq!(
        app.popup,
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based: expected_record
        }
    );
}

#[test]
fn owned_planet_popup_footer_shows_command_rail() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

    assert!(
        render_owned_planet_popup_line(&app, "COMMAND <- ? B C A L U X <ESC> ->")
            .contains("COMMAND <- ? B C A L U X <ESC> ->")
    );
}

#[test]
fn planet_list_footer_shows_owned_planet_commands() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;

    assert!(
        render_planet_footer_line(&app, "COMMAND <- ? F S B C A L U X <ESC>")
            .contains("COMMAND <- ? F S B C A L U X <ESC>")
    );
}

#[test]
fn planet_list_mass_commission_empty_toast_uses_plural_stardocks() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    clear_owned_stardocks(&mut app);

    app.handle_key(key(KeyCode::Char('a')));

    assert_eq!(
        app.planet_overlay.footer_notice.as_deref(),
        Some("Stardocks empty")
    );
}

#[test]
fn owned_planet_mass_commission_empty_toast_uses_singular_stardock() {
    let mut app = dash_app();
    let record = selected_planet_record_index(&app);
    clear_owned_stardocks(&mut app);
    app.open_owned_planet_popup(record);

    app.handle_key(key(KeyCode::Char('a')));

    assert_eq!(
        app.owned_planet_popup.status.as_deref(),
        Some("Stardock empty")
    );
}

#[test]
fn owned_planet_scorch_takes_three_confirmations_and_one_rescinds_it() {
    let mut app = dash_app();
    app.initialize_hosted_turn_draft();
    let record = selected_planet_record_index(&app);
    app.open_owned_planet_popup(record);
    let scorched = |app: &DashApp| app.game_data.planets[record - 1].scorch_earth;
    let staged = |app: &DashApp| {
        app.hosted_turn_draft
            .as_ref()
            .expect("draft")
            .planets
            .iter()
            .find(|planet| planet.planet_record_index_1_based == record)
            .map(|planet| planet.actions.clone())
    };

    app.open_owned_planet_scorch_confirm();
    app.submit_owned_planet_scorch().expect("first confirm");
    app.submit_owned_planet_scorch().expect("second confirm");
    assert!(!scorched(&app), "two confirmations set nothing");
    app.submit_owned_planet_scorch().expect("third confirm");
    assert!(scorched(&app));
    assert_eq!(staged(&app), Some(vec![PlanetTurnAction::Scorch]));

    // The same command rescinds it after one confirmation.
    app.open_owned_planet_scorch_confirm();
    assert_eq!(
        app.owned_planet_popup.mode,
        OwnedPlanetPopupMode::ScorchConfirm1
    );
    app.submit_owned_planet_scorch().expect("rescind confirm");
    assert!(!scorched(&app));
    assert_eq!(staged(&app), Some(vec![PlanetTurnAction::RescindScorch]));
    assert!(
        app.owned_planet_popup
            .status
            .as_deref()
            .is_some_and(|status| status.starts_with("Scorch-Earth rescinded"))
    );
}

#[test]
fn planet_list_enter_opens_status_and_escape_returns_to_table() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    let record = selected_planet_record_index(&app);

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.popup,
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based: record
        }
    );
    assert!(render_dashboard_line(&app, "PLANET STATUS").contains("PLANET STATUS"));
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("PLANET LIST:"))
    );

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.popup, ActivePopup::None);
}

#[test]
fn planet_list_selected_planet_commands_open_inline_footer_prompts() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    let record = selected_planet_record_index(&app);
    prepare_selected_planet_for_owned_commands(&mut app, record);

    app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::CommissionSelect
    );
    assert!(render_planet_footer_line(&app, "Commission slot #").contains("Commission slot #"));
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );

    app.handle_key(key(KeyCode::Char('l')));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::TransportFleetSelect {
            mode: nc_engine::ArmyTransportMode::Load
        }
    );
    assert!(render_planet_footer_line(&app, "Load Fleet #").contains("Load Fleet #"));
    app.handle_key(key(KeyCode::Esc));

    app.handle_key(key(KeyCode::Char('u')));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::TransportFleetSelect {
            mode: nc_engine::ArmyTransportMode::Unload
        }
    );
    assert!(render_planet_footer_line(&app, "Unload Fleet #").contains("Unload Fleet #"));
    app.handle_key(key(KeyCode::Esc));

    app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(
        app.owned_planet_popup.mode,
        OwnedPlanetPopupMode::ScorchConfirm1
    );
}

#[test]
fn planet_list_commission_runs_inline_from_footer() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    let record = selected_planet_record_index(&app);
    prepare_selected_planet_for_owned_commands(&mut app, record);

    app.handle_key(key(KeyCode::Char('c')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
    assert_eq!(
        (app.game_data.planets[record - 1].stardock[0].min(u32::from(u16::MAX)) as u16),
        0
    );
    assert!(
        app.planet_overlay
            .footer_notice
            .as_deref()
            .unwrap_or_default()
            .contains("Commissioned")
    );
}

#[test]
fn planet_list_transport_runs_inline_from_footer() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    let record = selected_planet_record_index(&app);
    prepare_selected_planet_for_owned_commands(&mut app, record);

    app.handle_key(key(KeyCode::Char('l')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::TransportQuantity {
            mode: nc_engine::ArmyTransportMode::Load
        }
    );
    assert!(
        render_planet_footer_line(&app, "How many armies to load?")
            .contains("How many armies to load?")
    );

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
    assert_eq!(
        (app.game_data.planets[record - 1]
            .armies
            .min(u16::from(u8::MAX)) as u8),
        4
    );
    assert!(
        app.planet_overlay
            .footer_notice
            .as_deref()
            .unwrap_or_default()
            .contains("Loaded 1 armies onto Fleet")
    );
}

#[test]
fn planet_list_mass_commission_uses_all_owned_planets() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    let selected_record = selected_planet_record_index(&app);
    clear_stardock(&mut app, selected_record);
    let other_record = seed_extra_owned_planet_with_stardock(&mut app);
    select_planet_row_by_record(&mut app, selected_record);

    app.handle_key(key(KeyCode::Char('a')));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::MassCommissionConfirm
    );
    assert!(
        render_planet_footer_line(&app, "Mass commission? [Y]/N ->")
            .contains("Mass commission? [Y]/N ->")
    );

    app.handle_key(key(KeyCode::Char('y')));

    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
    assert!(
        app.planet_overlay
            .footer_notice
            .as_deref()
            .unwrap_or_default()
            .contains("Mass commissioned")
    );
    assert_eq!(
        (app.game_data.planets[other_record - 1].stardock[0].min(u32::from(u16::MAX)) as u16),
        0
    );
}

#[test]
fn owned_planet_popup_browse_uses_planet_status_title() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

    assert!(render_owned_planet_popup_line(&app, "PLANET STATUS").contains("PLANET STATUS"));
}

#[test]
fn owned_planet_popup_browse_keeps_regular_status_value_together() {
    let mut app = dash_app();
    let record = seed_extra_owned_regular_planet(&mut app);

    app.open_owned_planet_popup(record);

    let line = render_owned_planet_popup_line(&app, "Regular planet - industry intact");
    assert!(line.contains("Status"));
}

#[test]
fn planet_detail_popup_keeps_regular_status_value_together() {
    let mut app = dash_app();
    let record = seed_extra_owned_regular_planet(&mut app);

    app.popup = ActivePopup::PlanetDetail {
        planet_record_index_1_based: record,
    };

    let line = render_planet_detail_popup_line(&app, "Regular planet - industry intact");
    assert!(line.contains("Status"));
}

#[test]
fn owned_planet_popup_question_mark_opens_help_overlay() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = app
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1 && planet.sector.to_array() == owned_coords)
        .map(|(idx, _)| idx + 1)
        .expect("owned planet");
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('?')));

    assert_eq!(app.overlay, ActiveOverlay::Help);
    assert_eq!(app.help_context, HelpContext::OwnedPlanetPopup);
    assert_eq!(
        app.popup,
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based: expected_record
        }
    );
    assert!(
        render_dashboard_line(&app, "Commission a completed stardock slot")
            .contains("Commission a completed stardock slot")
    );

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(
        app.popup,
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based: expected_record
        }
    );
}

#[test]
fn owned_planet_popup_build_opens_shared_overlay_with_command_line_footer() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.production = nc_data::Real48::from_f64(f64::from(80));
    planet.stored_goods = nc_data::Real48::from_f64(f64::from(80));
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(
        app.planet_overlay.build_planet_record_index_1_based,
        Some(expected_record)
    );
    assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
    assert!(
        render_dashboard_line(&app, "COMMAND <- ? + - D <ESC> [0] ->")
            .contains("COMMAND <- ? + - D <ESC> [0] ->")
    );
}

#[test]
fn owned_planet_build_overlay_uses_planet_title_instead_of_build_on_planet() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.production = nc_data::Real48::from_f64(f64::from(80));
    planet.stored_goods = nc_data::Real48::from_f64(f64::from(80));
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));

    let title = app.planet_build_title();
    let title_line = render_dashboard_line(&app, &title);

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert!(title_line.contains(&title));
    assert!(!title_line.contains("SPECIFY BUILD ORDERS"));
    assert!(!title_line.contains("BUILD ON PLANET"));
}

#[test]
fn owned_planet_build_quantity_stays_on_shared_overlay() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.production = nc_data::Real48::from_f64(f64::from(80));
    planet.stored_goods = nc_data::Real48::from_f64(f64::from(80));
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));
    app.handle_key(key(KeyCode::Char('1')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildQuantity
    );
    assert_eq!(
        app.planet_overlay.build_planet_record_index_1_based,
        Some(expected_record)
    );
    assert!(
        render_dashboard_line(&app, "COMMAND <- Qty [16] ->").contains("COMMAND <- Qty [16] ->")
    );
}

#[test]
fn owned_planet_no_budget_still_opens_build_overlay() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.production = nc_data::Real48::from_f64(f64::from(1));
    planet.stored_goods = nc_data::Real48::from_f64(f64::from(1));
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(
        app.planet_overlay.build_planet_record_index_1_based,
        Some(expected_record)
    );
}

#[test]
fn popup_launched_build_escape_returns_to_owned_planet_popup() {
    let mut app = dash_app();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.production = nc_data::Real48::from_f64(f64::from(80));
    planet.stored_goods = nc_data::Real48::from_f64(f64::from(80));
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));
    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
    assert!(matches!(
        app.popup,
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based
        } if planet_record_index_1_based == expected_record
    ));
}

#[test]
fn popup_launched_build_success_stays_in_build_overlay() {
    let mut app = dash_app_with_store();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.production = nc_data::Real48::from_f64(f64::from(80));
    planet.stored_goods = nc_data::Real48::from_f64(f64::from(80));
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));
    app.handle_key(key(KeyCode::Char('1')));
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('1')));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert_eq!(
        app.planet_overlay.build_planet_record_index_1_based,
        Some(expected_record)
    );
    assert_eq!(app.owned_planet_popup.status, None);
}

#[test]
fn popup_launched_build_delete_clears_selected_kind_in_place() {
    let mut app = dash_app_with_store();
    let owned_coords = first_owned_planet_coords(&app);
    let expected_record = owned_planet_record_index(&app, owned_coords);
    let planet = app
        .game_data
        .planets
        .get_mut(expected_record.saturating_sub(1))
        .expect("owned planet record");
    planet.build_queue[0].kind = nc_data::ProductionItemKind::from_code(1);
    planet.build_queue[0].points_owed = 10;
    let (column, row) = screen_point_for_sector(&app, owned_coords);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
    app.handle_key(key(KeyCode::Char('b')));
    app.handle_key(key(KeyCode::Char('d')));

    assert_eq!(app.overlay, ActiveOverlay::PlanetList);
    assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::BuildSpecify
    );
    assert!(
        nc_engine::planet_build_orders(&app.game_data.planets[expected_record - 1]).is_empty(),
        "selected queued build should be deleted in place"
    );
}

#[test]
fn right_click_on_visible_foreign_planet_opens_planet_detail_popup() {
    let mut app = audit_ready_dash_app();
    let target = first_visible_foreign_planet_coords(&mut app);
    let (column, row) = screen_point_for_sector(&app, target);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

    assert!(matches!(app.popup, ActivePopup::PlanetDetail { .. }));
}

#[test]
fn clicks_do_not_leak_through_open_overlay_to_map() {
    let mut app = dash_app();
    let starting = [app.crosshair_x, app.crosshair_y];
    app.overlay = ActiveOverlay::Help;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app
        .current_overlay_popup_rect(map_frame)
        .expect("help popup rect");

    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        popup.x + popup.width / 2,
        popup.y + popup.height / 2,
    ));

    assert_eq!([app.crosshair_x, app.crosshair_y], starting);
}

#[test]
fn hover_and_clicks_do_not_leak_through_open_popup_to_map() {
    let mut app = dash_app();
    let starting = [app.crosshair_x, app.crosshair_y];
    app.popup = ActivePopup::PlanetDetail {
        planet_record_index_1_based: 1,
    };
    let target = first_empty_sector_coords(&app);
    let (column, row) = screen_point_for_sector(&app, target);

    app.handle_mouse(mouse(MouseEventKind::Moved, column, row));
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

    assert_eq!([app.crosshair_x, app.crosshair_y], starting);
}

#[test]
fn closing_location_filtered_fleet_overlay_clears_transient_filter() {
    let mut app = dash_app();
    let fleet_coords = first_owned_fleet_coords(&app);

    app.open_fleet_overlay_for_location(fleet_coords);
    assert_eq!(app.fleet_overlay.location_filter, Some(fleet_coords));

    app.close_active_overlay();

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(app.fleet_overlay.location_filter, None);
}

#[test]
fn keyboard_opening_fleet_list_clears_transient_location_filter() {
    let mut app = dash_app();
    let fleet_coords = first_owned_fleet_coords(&app);
    app.fleet_overlay.location_filter = Some(fleet_coords);

    app.apply_action(super::input::Action::OpenOverlay(ActiveOverlay::FleetList));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.fleet_overlay.location_filter, None);
}

#[test]
fn owned_planet_popup_browse_uses_popup_record_not_crosshair() {
    let mut app = dash_app();
    let other_coords = app.game_data.planets[1].sector.to_array();
    app.game_data.planets[0].build_queue[0].kind = nc_data::ProductionItemKind::from_code(3);
    app.game_data.planets[0].build_queue[0].points_owed = 90;
    app.open_owned_planet_popup(1);
    app.crosshair_x = other_coords[0];
    app.crosshair_y = other_coords[1];

    let buffer = app.render_playfield().expect("render");
    let text = buffer_text(&buffer);

    assert!(text.contains("Building"));
    assert!(text.contains("BB2"));
}

#[test]
fn hosted_draft_replay_refreshes_planet_status_cache() {
    let mut app = dash_app();
    // Turn-file builds must fit the planet budget, so fund the treasury.
    app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(50));
    app.crosshair_x = app.game_data.planets[0].sector.to_array()[0];
    app.crosshair_y = app.game_data.planets[0].sector.to_array()[1];
    let _ = planet_view::selected_planet_detail(&app).expect("cached detail");
    let before_revision = app.game_data_revision;
    let draft = TurnSubmission {
        player_record_index_1_based: 1,
        year: app.game_data.conquest.game_year,
        tax_rate: None,
        diplomacy: Vec::new(),
        planets: vec![PlanetTurnBlock {
            planet_record_index_1_based: 1,
            actions: vec![PlanetTurnAction::Build {
                points_remaining: 5,
                kind: 1,
            }],
        }],
        fleets: Vec::new(),
        messages: Vec::new(),
    };

    crate::dashboard::replay_hosted_draft(&mut app, &draft).expect("replay draft");

    assert!(app.game_data_revision > before_revision);
    let detail = planet_view::selected_planet_detail(&app).expect("detail after replay");
    assert_eq!(detail_value(&detail.popup_lines, "Building"), Some("DD1"));
}

#[test]
fn tax_prompt_updates_preview_and_hosted_draft() {
    let mut app = dash_app();
    app.initialize_hosted_turn_draft();
    let before_revision = app.game_data_revision;

    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(app.game_data.players[0].tax_rate, 42);
    assert_eq!(
        app.hosted_turn_draft
            .as_ref()
            .and_then(|draft| draft.tax_rate),
        Some(42)
    );
    assert!(app.game_data_revision > before_revision);
}

#[test]
fn tax_prompt_wraps_long_status_into_popup_body() {
    let mut app = dash_app();
    app.popup = ActivePopup::TaxPrompt;
    app.tax_prompt_status = Some(format!(
        "{}{}",
        "This-tax-status-message-is-deliberately-long-enough-to-wrap-inside-the-popup-body-without-spaces.",
        "This-tax-status-message-is-deliberately-long-enough-to-wrap-inside-the-popup-body-without-spaces."
    ));
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_popup_rect(map_frame).expect("tax popup");
    let buffer = crate::dashboard::app::render::render(&app).expect("dashboard render");

    assert!(popup.height > 7);
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("deliberately-long") || line.contains("popup-body")
    }));
}

#[test]
fn tax_prompt_wraps_explanation_instead_of_stretching_popup() {
    let mut app = dash_app();
    app.popup = ActivePopup::TaxPrompt;
    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_popup_rect(map_frame).expect("tax popup");
    let buffer = crate::dashboard::app::render::render(&app).expect("dashboard render");

    assert!(popup.width <= 34);
    assert_eq!(popup.height, 7);
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Enter 0-100. Rates above 65%")
    }));
    assert!(
        (0..buffer.height())
            .any(|row| { buffer.plain_line(row).contains("can damage production.") })
    );
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Enter 0-100. Rates above 65% can damage production.")
    }));
}

#[test]
fn diplomacy_overlay_updates_preview_and_hosted_draft() {
    let mut app = dash_app();
    app.initialize_hosted_turn_draft();
    app.overlay = ActiveOverlay::Diplomacy;
    while diplomacy::selected_empire_slot(&app) == Some(app.player_record_index_1_based as u8) {
        app.diplomacy_overlay.selected += 1;
    }
    let target = diplomacy::selected_empire_slot(&app).expect("target empire");

    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    assert_eq!(
        app.game_data
            .stored_diplomatic_relation(app.player_record_index_1_based as u8, target),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        app.hosted_turn_draft
            .as_ref()
            .and_then(|draft| draft.diplomacy.iter().find(|row| row.to_empire == target))
            .map(|row| row.relation),
        Some(DiplomaticRelation::Enemy)
    );
}

#[test]
fn diplomacy_overlay_renders_shared_table_header() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Diplomacy;

    let lines = render_diplomacy_overlay_lines(&app);
    let header = lines
        .iter()
        .find(|line| line.contains("Rnk") && line.contains("Empire"))
        .expect("diplomacy table header");

    assert!(header.contains("│"), "{header}");
    assert!(header.contains("Planets"), "{header}");
    assert!(header.contains("Prod"), "{header}");
    assert!(header.contains("Relations"), "{header}");
}

#[test]
fn diplomacy_overlay_uses_live_planets_and_production() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Diplomacy;
    let live_planets = app.game_data.player_owned_planet_count_current_known(1);
    let live_production = app.game_data.empire_present_production(1);
    assert!(live_planets > 0);
    assert!(live_production > 0);
    app.game_data.players[0].planet_count = 0;
    app.game_data.players[0].production = 0;

    let own_row = diplomacy::table_rows(&app)
        .into_iter()
        .find(|row| row.slot == 1)
        .expect("player row");
    assert_eq!(own_row.cells[2], live_planets.to_string());
    assert_eq!(own_row.cells[3], live_production.to_string());

    let visible_name = own_row.name.chars().take(17).collect::<String>();
    let rendered_row = render_diplomacy_overlay_lines(&app)
        .into_iter()
        .find(|line| line.contains(&visible_name))
        .expect("rendered player diplomacy row");
    assert!(
        rendered_row.contains(&live_planets.to_string()),
        "{rendered_row}"
    );
    assert!(
        rendered_row.contains(&live_production.to_string()),
        "{rendered_row}"
    );
}

#[test]
fn diplomacy_selection_uses_displayed_live_ranking_order() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Diplomacy;
    app.game_data.players[0].production = 0;
    app.game_data.players[1].production = u16::MAX;
    app.diplomacy_overlay.selected = 0;

    let first_displayed_slot = diplomacy::table_rows(&app)
        .first()
        .map(|row| row.slot)
        .expect("diplomacy row");

    assert_eq!(
        diplomacy::selected_empire_slot(&app),
        Some(first_displayed_slot)
    );
}

#[test]
fn inbox_quick_compose_stages_and_removes_hosted_message() {
    let mut app = dash_app_with_store();
    app.initialize_hosted_turn_draft();
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(app.overlay, ActiveOverlay::Inbox);

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let draft = app.hosted_turn_draft.as_ref().expect("draft");
    assert_eq!(draft.messages.len(), 1);
    assert_eq!(draft.messages[0].recipient_empire, 2);
    assert_eq!(draft.messages[0].subject, "Hi");
    assert_eq!(draft.messages[0].body, "Move");
    assert!(
        app.hosted_turn_text()
            .unwrap()
            .contains("message to=2 subject=\"Hi\" body=\"Move\"")
    );
    assert!(app.queued_mail.iter().any(|mail| {
        mail.sender_empire_id == 1 && mail.recipient_empire_id == 2 && mail.subject == "Hi"
    }));
    let store = app.campaign_store.clone().expect("campaign store");
    let runtime_state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime state");
    assert!(runtime_state.queued_mail.iter().any(|mail| {
        mail.sender_empire_id == 1 && mail.recipient_empire_id == 2 && mail.subject == "Hi"
    }));

    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.hosted_turn_draft.as_ref().unwrap().messages.is_empty());
    assert!(app.queued_mail.iter().any(|mail| {
        mail.sender_empire_id == 1
            && mail.recipient_empire_id == 2
            && mail.subject == "Hi"
            && mail.cancelled
    }));
    let runtime_state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime state");
    assert!(runtime_state.queued_mail.iter().any(|mail| {
        mail.sender_empire_id == 1
            && mail.recipient_empire_id == 2
            && mail.subject == "Hi"
            && mail.cancelled
    }));
}

#[test]
fn inbox_message_editor_alt_send_confirms_before_submission() {
    let mut app = dash_app_with_store();
    open_inbox_message_body_editor(&mut app, "Hi", "Move");

    assert!(render_dashboard_line(&app, "Alt-S").contains("Alt-X"));

    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT));

    assert_eq!(
        app.popup,
        ActivePopup::InboxMessageConfirm {
            action: InboxMessageConfirmAction::Send
        }
    );
    assert!(render_dashboard_line(&app, "Send Message? Y/[N]").contains("Send Message? Y/[N]"));
    assert!(app.hosted_turn_draft.as_ref().unwrap().messages.is_empty());

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
    assert_eq!(app.inbox_overlay.compose_subject, "Hi");
    assert_eq!(app.inbox_overlay.compose_body, "Move");

    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
    assert_eq!(app.inbox_overlay.compose_body, "Move");

    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let draft = app.hosted_turn_draft.as_ref().expect("draft");
    assert_eq!(draft.messages.len(), 1);
    assert_eq!(draft.messages[0].subject, "Hi");
    assert_eq!(draft.messages[0].body, "Move");
}

#[test]
fn inbox_message_editor_alt_discard_confirms_before_clearing_draft() {
    let mut app = dash_app();
    open_inbox_message_body_editor(&mut app, "Hi", "Move");

    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));

    assert_eq!(
        app.popup,
        ActivePopup::InboxMessageConfirm {
            action: InboxMessageConfirmAction::Discard
        }
    );
    assert!(
        render_dashboard_line(&app, "Discard Message? Y/[N]").contains("Discard Message? Y/[N]")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
    assert_eq!(app.inbox_overlay.compose_subject, "Hi");
    assert_eq!(app.inbox_overlay.compose_body, "Move");

    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
    assert_eq!(app.inbox_overlay.compose_body, "Move");

    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::None);
    assert!(app.inbox_overlay.compose_subject.is_empty());
    assert!(app.inbox_overlay.compose_body.is_empty());
}

#[test]
fn inbox_outbox_escape_returns_to_inbox() {
    let mut app = dash_app();
    app.initialize_hosted_turn_draft();
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

    assert_eq!(app.overlay, ActiveOverlay::Inbox);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::Outbox);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.overlay, ActiveOverlay::Inbox);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::None);
}

#[test]
fn inbox_help_key_uses_active_prompt_context() {
    let mut app = dash_app();
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

    assert_eq!(app.overlay, ActiveOverlay::Help);
    assert_eq!(app.help_context, HelpContext::Inbox);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

    assert_eq!(app.overlay, ActiveOverlay::Help);
    assert_eq!(app.help_context, HelpContext::InboxCompose);
    assert_eq!(app.help_return_overlay, ActiveOverlay::Inbox);
    assert_eq!(app.inbox_overlay.compose_body, "M");
}

fn open_inbox_message_body_editor(app: &mut DashApp, subject: &str, body: &str) {
    app.initialize_hosted_turn_draft();
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    for ch in subject.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    for ch in body.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    assert_eq!(app.overlay, ActiveOverlay::Inbox);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
}

fn detail_value<'a>(fields: &'a [planet_view::DetailLine], label: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|field| field.label == label)
        .map(|field| field.value.as_str())
}

fn buffer_text(buffer: &PlayfieldBuffer) -> String {
    let mut text = String::new();
    for row in 0..buffer.height() {
        for cell in buffer.row(row) {
            text.push(cell.ch);
        }
        text.push('\n');
    }
    text
}

fn dash_app() -> DashApp {
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
        crate::dashboard::geometry::ScreenGeometry::new(160, 40),
        crate::dashboard::geometry::ScreenGeometry::new(108, 26),
        1,
    )
}

fn audit_ready_dash_app() -> DashApp {
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        GameStateBuilder::new()
            .with_player_count(4)
            .with_homeworld_coords(vec![[16, 13], [12, 6], [4, 15], [15, 15]])
            .with_guard_starbase(1, 1, [16, 13], 1)
            .build_initialized_baseline()
            .expect("baseline"),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        crate::dashboard::geometry::ScreenGeometry::new(160, 40),
        crate::dashboard::geometry::ScreenGeometry::new(108, 26),
        1,
    );
    seed_unowned_target_world(&mut app.game_data, [8, 8]);
    strengthen_first_owned_fleet(&mut app.game_data);
    app.planet_intel_snapshots = view_world_audit_snapshots(&app.game_data, 1);
    app
}

fn dash_app_with_starbase() -> DashApp {
    DashApp::new_for_tests(
        PathBuf::from("."),
        GameStateBuilder::new()
            .with_player_count(4)
            .with_guard_starbase(1, 1, [16, 13], 1)
            .build_initialized_baseline()
            .expect("baseline with starbase"),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        crate::dashboard::geometry::ScreenGeometry::new(160, 40),
        crate::dashboard::geometry::ScreenGeometry::new(108, 26),
        1,
    )
}

fn dash_app_with_starbase_store() -> DashApp {
    let root = std::env::temp_dir().join(format!(
        "nc-helm-fleet-order-starbase-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("create temp test dir");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_guard_starbase(1, 1, [16, 13], 1)
        .build_initialized_baseline()
        .expect("baseline with starbase");
    strengthen_first_owned_fleet(&mut game_data);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    let planet_intel_by_viewer = (0..game_data.conquest.player_count)
        .map(|_| BTreeMap::new())
        .collect::<Vec<_>>();
    let player_activity_states = store
        .latest_player_activity_states(game_data.conquest.player_count)
        .expect("default player activity");
    let player_lifecycle_states = store
        .latest_player_lifecycle_states(game_data.conquest.player_count)
        .expect("default player lifecycle");
    store
        .save_runtime_state_structured_with_intel_and_activity(
            game_data.conquest.game_year,
            &game_data,
            &[],
            &[],
            &planet_intel_by_viewer,
            &player_activity_states,
        )
        .expect("seed campaign store");

    DashApp::new(
        root,
        Some(store),
        game_data,
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        player_activity_states,
        player_lifecycle_states,
        nc_data::WinnerState::default(),
        crate::dashboard::geometry::ScreenGeometry::new(160, 40),
        crate::dashboard::geometry::ScreenGeometry::new(108, 26),
        1,
    )
}

fn dash_app_with_store() -> DashApp {
    let root = std::env::temp_dir().join(format!(
        "nc-helm-fleet-order-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("create temp test dir");
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    let planet_intel_by_viewer = (0..game_data.conquest.player_count)
        .map(|_| BTreeMap::new())
        .collect::<Vec<_>>();
    let player_activity_states = store
        .latest_player_activity_states(game_data.conquest.player_count)
        .expect("default player activity");
    let player_lifecycle_states = store
        .latest_player_lifecycle_states(game_data.conquest.player_count)
        .expect("default player lifecycle");
    store
        .save_runtime_state_structured_with_intel_and_activity(
            game_data.conquest.game_year,
            &game_data,
            &[],
            &[],
            &planet_intel_by_viewer,
            &player_activity_states,
        )
        .expect("seed campaign store");

    DashApp::new(
        root,
        Some(store),
        game_data,
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        player_activity_states,
        player_lifecycle_states,
        nc_data::WinnerState::default(),
        crate::dashboard::geometry::ScreenGeometry::new(160, 40),
        crate::dashboard::geometry::ScreenGeometry::new(108, 26),
        1,
    )
}

fn strengthen_first_owned_fleet(game_data: &mut nc_data::CoreGameData) {
    for fleet in game_data
        .fleets
        .iter_mut()
        .filter(|fleet| fleet.owner.0 == 1 && nc_data::has_any_force(&fleet))
    {
        fleet.ships.battleships = 1;
        fleet.ships.destroyers = 1;
        nc_data::set_troop_transports(fleet, 1);
        nc_data::set_armies_aboard(fleet, 2);
        fleet.ships.scouts = 1;
        fleet.ships.etacs = 1;
        nc_data::recompute_max_speed(fleet);
        fleet.speed = fleet.max_speed;
    }
}

fn seed_unowned_target_world(game_data: &mut nc_data::CoreGameData, coords: [u8; 2]) {
    let planet = game_data
        .planets
        .iter_mut()
        .find(|planet| planet.owner.0 == 0 && planet.sector.to_array() == [0, 0])
        .expect("unused unowned planet slot");
    planet.sector = nc_data::Coords::from_array(coords);
    planet.owner.0 = 0;
}

fn view_world_audit_snapshots(
    game_data: &nc_data::CoreGameData,
    viewer_empire_id: u8,
) -> Vec<PlanetIntelSnapshot> {
    let target_record_index = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| {
            planet.owner.0 != viewer_empire_id && planet.sector.to_array() != [0, 0]
        })
        .map(|(idx, _)| idx + 1)
        .expect("non-owned target planet");

    game_data
        .planets
        .iter()
        .enumerate()
        .filter_map(|(planet_index, planet)| {
            let planet_record_index_1_based = planet_index + 1;
            if planet.owner.0 == viewer_empire_id {
                return None;
            }
            if planet_record_index_1_based == target_record_index {
                return Some(PlanetIntelSnapshot {
                    planet_record_index_1_based,
                    intel_tier: IntelTier::Unknown,
                    compat_is_orbit_seed: false,
                    last_intel_year: None,
                    seen_year: None,
                    scout_year: None,
                    known_name: None,
                    known_owner_empire_id: None,
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                    known_starbase_count: None,
                    known_current_production: None,
                    known_stored_points: None,
                    known_docked_summary: None,
                    known_orbit_summary: None,
                    compat_word_1e: None,
                });
            }
            Some(PlanetIntelSnapshot {
                planet_record_index_1_based,
                intel_tier: IntelTier::Partial,
                compat_is_orbit_seed: false,
                last_intel_year: Some(game_data.conquest.game_year),
                seen_year: Some(game_data.conquest.game_year),
                scout_year: None,
                known_name: None,
                known_owner_empire_id: Some(planet.owner.0),
                known_potential_production: None,
                known_armies: None,
                known_ground_batteries: None,
                known_starbase_count: None,
                known_current_production: None,
                known_stored_points: None,
                known_docked_summary: None,
                known_orbit_summary: None,
                compat_word_1e: None,
            })
        })
        .collect()
}

fn screen_point_for_sector(app: &DashApp, target: [u8; 2]) -> (u16, u16) {
    let map_frame = dashboard_layout(app).widgets.center_map;
    for row in map_frame.grid.row..map_frame.grid.row + map_frame.grid.height {
        for col in map_frame.grid.col..map_frame.grid.col + map_frame.grid.width {
            if crate::dashboard::panels::starmap::screen_sector_at_point(app, map_frame, col, row)
                == Some(target)
            {
                return (col as u16, row as u16);
            }
        }
    }
    panic!("no screen point for sector {target:?}");
}

fn outside_map_point(app: &DashApp) -> (u16, u16) {
    let outer = dashboard_layout(app).widgets.center_map.outer;
    if outer.col > 0 {
        return ((outer.col - 1) as u16, outer.row as u16);
    }
    ((outer.last_col() + 1) as u16, outer.row as u16)
}

fn first_owned_fleet_coords(app: &DashApp) -> [u8; 2] {
    app.game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && nc_data::has_any_force(&fleet))
        .map(|fleet| fleet.sector.to_array())
        .expect("owned fleet")
}

fn first_owned_planet_coords(app: &DashApp) -> [u8; 2] {
    app.game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 1 && planet.sector.to_array() != [0, 0])
        .map(|planet| planet.sector.to_array())
        .expect("owned planet")
}

fn first_owned_planet_record_index(app: &DashApp) -> usize {
    app.game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| {
            planet.owner.0 as usize == app.player_record_index_1_based
                && planet.sector.to_array() != [0, 0]
        })
        .map(|(idx, _)| idx + 1)
        .expect("owned planet")
}

fn first_non_owned_planet_record_index(app: &DashApp) -> usize {
    app.game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| {
            planet.owner.0 as usize != app.player_record_index_1_based
                && planet.sector.to_array() != [0, 0]
        })
        .map(|(idx, _)| idx + 1)
        .expect("non-owned planet")
}

fn selected_planet_record_index(app: &DashApp) -> usize {
    planet_list::table_rows(app)
        .get(app.planet_overlay.selected)
        .map(|row| row.planet_record_index_1_based)
        .expect("selected planet row")
}

fn select_intel_row_by_record(app: &mut DashApp, record: usize) {
    app.intel_overlay.selected = intel_database::table_rows(app)
        .iter()
        .position(|row| row.planet_record_index_1_based == record)
        .expect("intel row");
}

fn select_planet_row_by_record(app: &mut DashApp, record: usize) {
    app.planet_overlay.selected = planet_list::table_rows(app)
        .iter()
        .position(|row| row.planet_record_index_1_based == record)
        .expect("planet row");
}

fn owned_planet_record_index(app: &DashApp, coords: [u8; 2]) -> usize {
    app.game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1 && planet.sector.to_array() == coords)
        .map(|(idx, _)| idx + 1)
        .expect("owned planet")
}

fn clear_stardock(app: &mut DashApp, record: usize) {
    let planet = app
        .game_data
        .planets
        .get_mut(record.saturating_sub(1))
        .expect("planet record");
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        planet.stardock[slot] = 0;
    }
}

fn clear_owned_stardocks(app: &mut DashApp) {
    let owned_records = app
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter_map(|(idx, planet)| {
            (planet.owner.0 as usize == app.player_record_index_1_based).then_some(idx + 1)
        })
        .collect::<Vec<_>>();
    for record in owned_records {
        clear_stardock(app, record);
    }
}

fn prepare_selected_planet_for_owned_commands(app: &mut DashApp, record: usize) {
    let coords = app.game_data.planets[record - 1].sector.to_array();
    let planet = app
        .game_data
        .planets
        .get_mut(record.saturating_sub(1))
        .expect("planet record");
    planet.armies = 5;
    planet.stardock[0] = 1;

    let fleet = app
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("owned fleet");
    fleet.sector = nc_data::Coords::from_array(coords);
    nc_data::set_troop_transports(fleet, 2);
    nc_data::set_armies_aboard(fleet, 1);
}

fn seed_extra_owned_planet_with_stardock(app: &mut DashApp) -> usize {
    let (idx, planet) = app
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 != 1 && planet.sector.to_array() != [0, 0])
        .expect("non-player planet");
    planet.owner.0 = 1;
    planet.stardock[0] = 1;
    idx + 1
}

fn seed_extra_owned_regular_planet(app: &mut DashApp) -> usize {
    let coords = (1..=30)
        .flat_map(|x| (1..=30).map(move |y| [x, y]))
        .find(|coords| {
            !app.game_data
                .planets
                .iter()
                .any(|planet| planet.sector.to_array() == *coords)
        })
        .expect("unused regular planet coords");
    let (idx, planet) = app
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(_, planet)| planet.sector.to_array() == [0, 0])
        .expect("unused planet record");
    planet.sector = nc_data::Coords::from_array(coords);
    planet.potential_production = 50;
    planet.years_under_owner = 1;
    planet.owner.0 = 1;
    nc_data::set_planet_name(planet, "Outpost");
    let potential = u16::from(planet.potential_production).max(1);
    planet.production = nc_data::Real48::from_f64(f64::from(potential));
    app.game_data_revision += 1;
    idx + 1
}

fn first_empty_sector_coords(app: &DashApp) -> [u8; 2] {
    first_visible_sector_matching(app, |coords| {
        !sector_has_planet(app, coords) && !sector_has_player_fleet(app, coords)
    })
}

fn first_visible_foreign_planet_coords(app: &mut DashApp) -> [u8; 2] {
    for planet in app
        .game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 != 1 && planet.sector.to_array() != [0, 0])
    {
        let coords = planet.sector.to_array();
        app.crosshair_x = coords[0];
        app.crosshair_y = coords[1];
        if planet_view::selected_planet_detail(app).is_some() {
            return coords;
        }
    }
    panic!("visible foreign planet");
}

fn first_visible_sector_matching<F>(app: &DashApp, predicate: F) -> [u8; 2]
where
    F: Fn([u8; 2]) -> bool,
{
    let map_frame = dashboard_layout(app).widgets.center_map;
    for row in map_frame.grid.row..map_frame.grid.row + map_frame.grid.height {
        for col in map_frame.grid.col..map_frame.grid.col + map_frame.grid.width {
            let Some(coords) =
                crate::dashboard::panels::starmap::screen_sector_at_point(app, map_frame, col, row)
            else {
                continue;
            };
            if predicate(coords) {
                return coords;
            }
        }
    }
    panic!("expected visible sector");
}

fn sector_has_planet(app: &DashApp, coords: [u8; 2]) -> bool {
    app.game_data
        .planets
        .iter()
        .any(|planet| planet.sector.to_array() == coords)
}

fn sector_has_player_fleet(app: &DashApp, coords: [u8; 2]) -> bool {
    app.game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == app.player_record_index_1_based as u8
            && nc_data::has_any_force(&fleet)
            && fleet.sector.to_array() == coords
    })
}

fn select_first_fleet_row(app: &mut DashApp) {
    app.fleet_overlay.selected = fleet_list::table_rows(app)
        .iter()
        .position(|row| matches!(row.key, FleetOverlayRowKey::Fleet(_)))
        .expect("fleet row");
}

fn select_first_two_fleet_rows(app: &mut DashApp) -> Vec<usize> {
    let rows = fleet_list::table_rows(app);
    let fleet_indexes = rows
        .iter()
        .enumerate()
        .filter_map(|(row_index, row)| match row.key {
            FleetOverlayRowKey::Fleet(record_index) => Some((row_index, record_index)),
            FleetOverlayRowKey::Starbase(_) => None,
        })
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(fleet_indexes.len(), 2, "expected at least two fleet rows");
    app.fleet_overlay.selected = fleet_indexes[0].0;
    app.handle_key(key(KeyCode::Char(' ')));
    app.fleet_overlay.selected = fleet_indexes[1].0;
    app.handle_key(key(KeyCode::Char(' ')));
    app.fleet_overlay.selected = fleet_indexes[0].0;
    fleet_indexes
        .into_iter()
        .map(|(_, record_index)| record_index)
        .collect()
}

fn render_fleet_footer_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    fleet_list::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("fleet footer")
}

fn render_fleet_title_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    fleet_list::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("fleet title")
}

fn render_planet_footer_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    planet_list::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("planet footer")
}

fn render_planet_title_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    planet_list::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("planet title")
}

fn render_owned_planet_popup_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    let planet_record_index_1_based = match app.popup {
        ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        } => planet_record_index_1_based,
        other => panic!("expected owned planet popup, got {other:?}"),
    };
    crate::dashboard::popups::owned_planet::draw(
        &mut buffer,
        app,
        layout.widgets.center_map,
        planet_record_index_1_based,
    );
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("owned planet popup line")
}

fn render_planet_detail_popup_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    let planet_record_index_1_based = match app.popup {
        ActivePopup::PlanetDetail {
            planet_record_index_1_based,
        } => planet_record_index_1_based,
        other => panic!("expected planet detail popup, got {other:?}"),
    };
    crate::dashboard::popups::planet_detail::draw(
        &mut buffer,
        app,
        layout.widgets.center_map,
        planet_record_index_1_based,
    );
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("planet detail popup line")
}

fn render_fleet_detail_popup_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    let fleet_record_index_1_based = match app.popup {
        ActivePopup::FleetDetail {
            fleet_record_index_1_based,
        } => fleet_record_index_1_based,
        other => panic!("expected fleet detail popup, got {other:?}"),
    };
    crate::dashboard::popups::fleet_detail::draw(
        &mut buffer,
        app,
        layout.widgets.center_map,
        fleet_record_index_1_based,
    );
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("fleet detail popup line")
}

fn render_dashboard_line(app: &DashApp, needle: &str) -> String {
    render_dashboard_lines(app)
        .into_iter()
        .find(|line| line.contains(needle))
        .expect("dashboard line")
}

fn render_dashboard_lines(app: &DashApp) -> Vec<String> {
    let buffer = crate::dashboard::app::render::render(app).expect("dashboard render");
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .collect()
}

fn render_intel_footer_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    intel_database::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("intel footer")
}

fn render_intel_title_line(app: &DashApp, needle: &str) -> String {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    intel_database::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains(needle))
        .expect("intel title")
}

fn render_diplomacy_overlay_lines(app: &DashApp) -> Vec<String> {
    let layout = dashboard_layout(app);
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    diplomacy::draw(&mut buffer, app, layout.widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .collect()
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn command_line_toast_clears_after_one_second() {
    let mut app = dash_app();
    let now = Instant::now();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay.footer_notice = Some("Saved local settings".to_string());

    assert!(app.update_command_line_toast_state(now));
    assert_eq!(
        app.command_line_toast_deadline,
        Some(now + Duration::from_secs(1))
    );
    assert_eq!(
        app.planet_overlay.footer_notice.as_deref(),
        Some("Saved local settings")
    );

    assert!(app.update_command_line_toast_state(now + Duration::from_secs(1)));
    assert_eq!(app.planet_overlay.footer_notice, None);
}

#[test]
fn command_line_toast_key_dismissal_still_runs_normal_action() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay.footer_notice = Some("Saved local settings".to_string());

    app.dispatch_key_event(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(app.planet_overlay.footer_notice, None);
}

#[test]
fn prompt_validation_toast_clears_when_opening_help() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay
        .open_prompt(PlanetOverlayPromptMode::FilterMenu);
    app.planet_overlay.prompt_status = Some(" Ambiguous: sbs/sta".to_string());

    app.dispatch_key_event(key(KeyCode::Char('?')));

    assert_eq!(app.overlay, ActiveOverlay::Help);
    assert_eq!(app.planet_overlay.prompt_status, None);
}

#[test]
fn root_escape_opens_quit_confirm_popup() {
    let mut app = dash_app();

    app.dispatch_key_event(key(KeyCode::Esc));

    assert_eq!(app.popup, ActivePopup::QuitConfirm);
    assert_eq!(app.overlay, ActiveOverlay::None);
    assert_eq!(app.take_exit_request(), None);
}

#[test]
fn alt_q_opens_quit_confirm_popup() {
    let mut app = dash_app();

    app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));

    assert_eq!(app.popup, ActivePopup::QuitConfirm);
    assert_eq!(app.take_exit_request(), None);
}

#[test]
fn quit_confirm_renders_over_active_overlay() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;

    app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.popup, ActivePopup::QuitConfirm);
    assert!(render_dashboard_line(&app, "┐QUIT┌").contains("┐QUIT┌"));
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("FLEET LIST"))
    );

    app.dispatch_key_event(key(KeyCode::Esc));

    assert_eq!(app.overlay, ActiveOverlay::FleetList);
    assert_eq!(app.popup, ActivePopup::None);
    assert!(render_dashboard_line(&app, "FLEET LIST").contains("FLEET LIST"));
}

#[test]
fn quit_confirm_cancel_restores_underlying_popup() {
    let mut app = dash_app();
    app.popup = ActivePopup::PlanetDetail {
        planet_record_index_1_based: 1,
    };

    app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
    assert!(render_dashboard_line(&app, "┐QUIT┌").contains("┐QUIT┌"));
    assert!(
        !render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("PLANET INFO"))
    );
    app.dispatch_key_event(key(KeyCode::Enter));

    assert_eq!(
        app.popup,
        ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1
        }
    );
    assert_eq!(app.take_exit_request(), None);
}

#[test]
fn quit_confirm_yes_requests_return_to_lobby() {
    let mut app = dash_app();
    app.dispatch_key_event(key(KeyCode::Esc));

    app.dispatch_key_event(key(KeyCode::Char('y')));

    assert_eq!(
        app.take_exit_request(),
        Some(DashboardExitRequest::ReturnToLobby)
    );
    assert!(!app.should_quit);
}

#[test]
fn control_c_requests_client_quit() {
    let mut app = dash_app();

    app.dispatch_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(
        app.take_exit_request(),
        Some(DashboardExitRequest::QuitClient)
    );
}

#[test]
fn middle_drag_on_starmap_pans_viewport_without_moving_crosshair() {
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        ScreenGeometry::new(80, 45),
        ScreenGeometry::new(0, 0),
        1,
    );
    app.crosshair_x = 23;
    app.crosshair_y = 23;
    starmap::advance_starmap_viewport(&mut app);
    let initial_x_min = app.starmap_viewport_x_min;
    let initial_y_min = app.starmap_viewport_y_min;
    let initial_crosshair = [app.crosshair_x, app.crosshair_y];

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let col = map_frame.outer.col as u16 + 1;
    let row = map_frame.outer.row as u16 + 1;

    // Middle-down inside the map widget.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Middle),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    // Drag 8 columns right → viewport should pan 2 sectors left.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Middle),
        column: col + 8,
        row,
        modifiers: KeyModifiers::NONE,
    });

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Middle),
        column: col + 8,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!([app.crosshair_x, app.crosshair_y], initial_crosshair);
    assert_eq!(app.starmap_viewport_x_min, initial_x_min.saturating_sub(2));
    assert_eq!(app.starmap_viewport_y_min, initial_y_min);
}

#[test]
fn middle_click_without_drag_recenters_viewport() {
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        ScreenGeometry::new(80, 45),
        ScreenGeometry::new(0, 0),
        1,
    );
    app.crosshair_x = 23;
    app.crosshair_y = 23;
    starmap::advance_starmap_viewport(&mut app);
    let centered_x_min = app.starmap_viewport_x_min;
    let centered_y_min = app.starmap_viewport_y_min;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let col = map_frame.outer.col as u16 + 1;
    let row = map_frame.outer.row as u16 + 1;

    // Pan the viewport away from centre with a drag.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Middle),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Middle),
        column: col + 8,
        row,
        modifiers: KeyModifiers::NONE,
    });
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Middle),
        column: col + 8,
        row,
        modifiers: KeyModifiers::NONE,
    });
    let dragged_x_min = app.starmap_viewport_x_min;
    let _dragged_y_min = app.starmap_viewport_y_min;
    // Sanity: drag actually moved the viewport.
    assert_ne!(dragged_x_min, centered_x_min);

    // Middle-click without drag resets the viewport to centre on crosshair.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Middle),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Middle),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.starmap_viewport_x_min, centered_x_min);
    assert_eq!(app.starmap_viewport_y_min, centered_y_min);
}

#[test]
fn mouse_wheel_on_starmap_pans_viewport_without_moving_crosshair() {
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        ScreenGeometry::new(80, 45),
        ScreenGeometry::new(0, 0),
        1,
    );
    app.crosshair_x = 23;
    app.crosshair_y = 23;
    starmap::advance_starmap_viewport(&mut app);
    let initial_x_min = app.starmap_viewport_x_min;
    let initial_y_min = app.starmap_viewport_y_min;
    let initial_crosshair = [app.crosshair_x, app.crosshair_y];

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let col = map_frame.outer.col as u16 + 1;
    let row = map_frame.outer.row as u16 + 1;

    // Scroll up (positive lines) → viewport origin moves up (decrease y).
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: 2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!([app.crosshair_x, app.crosshair_y], initial_crosshair);
    assert_eq!(app.starmap_viewport_y_min, initial_y_min.saturating_sub(2));
    assert_eq!(app.starmap_viewport_x_min, initial_x_min);

    // Scroll down (negative lines) → viewport origin moves down (increase y).
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -3 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.starmap_viewport_y_min, initial_y_min.saturating_add(1));
}

#[test]
fn mouse_wheel_on_fitting_starmap_does_nothing() {
    let mut app = DashApp::new_for_tests(
        PathBuf::from("."),
        build_seeded_initialized_game(10, 3000, 1515).expect("seeded game"),
        BTreeMap::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        ScreenGeometry::new(80, 45),
        ScreenGeometry::new(0, 0),
        1,
    );
    app.crosshair_x = 5;
    app.crosshair_y = 5;
    starmap::advance_starmap_viewport(&mut app);
    let initial_x_min = app.starmap_viewport_x_min;
    let initial_y_min = app.starmap_viewport_y_min;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let col = map_frame.outer.col as u16 + 1;
    let row = map_frame.outer.row as u16 + 1;

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: 5 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.starmap_viewport_x_min, initial_x_min);
    assert_eq!(app.starmap_viewport_y_min, initial_y_min);
}

#[test]
fn mouse_wheel_in_diplomacy_overlay_scrolls_list() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Diplomacy;
    app.diplomacy_overlay.selected = 0;
    app.diplomacy_overlay.scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 2;
    let row = popup.y + 2;

    // Scroll down (negative lines) moves popup selection down.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -3 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(app.diplomacy_overlay.selected, 3);

    // Scroll up (positive lines) moves popup selection up.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: 2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(app.diplomacy_overlay.selected, 1);
}

#[test]
fn mouse_wheel_in_inbox_list_moves_selection() {
    let mut app = dash_app();
    app.report_block_rows = vec![
        ReportBlockRow {
            viewer_empire_id: 0,
            block_index: 0,
            decoded_text: "Stardate: 03/3012\nLine one.".to_string(),
            raw_bytes: None,
            recipient_deleted: false,
        },
        ReportBlockRow {
            viewer_empire_id: 0,
            block_index: 1,
            decoded_text: "Stardate: 04/3012\nLine two.".to_string(),
            raw_bytes: None,
            recipient_deleted: false,
        },
    ];
    app.queued_mail = vec![QueuedPlayerMail {
        sender_empire_id: 2,
        recipient_empire_id: 1,
        year: 3012,
        subject: "Test subject".to_string(),
        body: "Test body".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }];
    app.overlay = ActiveOverlay::Inbox;
    app.inbox_overlay.selected = 2;
    app.inbox_overlay.scroll = 0;
    app.inbox_overlay.preview_scroll = 5;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    // List pane is on the left side of the body.
    let col = popup.x + 3;
    let row = popup.y + 5;

    // Scroll up (positive lines) → selected decreases.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: 2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(app.inbox_overlay.selected, 0);
    assert_eq!(app.inbox_overlay.scroll, 0);
    assert_eq!(app.inbox_overlay.preview_scroll, 5); // unchanged

    // Scroll down (negative lines) → selected increases.
    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(app.inbox_overlay.selected, 1);
}

#[test]
fn mouse_wheel_in_inbox_preview_scrolls_preview() {
    let mut app = dash_app();
    app.report_block_rows = vec![ReportBlockRow {
        viewer_empire_id: 0,
        block_index: 0,
        decoded_text: "Stardate: 03/3012\nLine one.".to_string(),
        raw_bytes: None,
        recipient_deleted: false,
    }];
    app.queued_mail = vec![QueuedPlayerMail {
        sender_empire_id: 2,
        recipient_empire_id: 1,
        year: 3012,
        subject: "Test subject".to_string(),
        body: "Test body with enough length to create a preview.".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }];
    app.overlay = ActiveOverlay::Inbox;
    app.inbox_overlay.selected = 0;
    app.inbox_overlay.scroll = 0;
    app.inbox_overlay.preview_scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    // Preview pane is on the right side of the body.
    let col = popup.x + popup.width - 3;
    let row = popup.y + 5;

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    // First item has 2-line body; max preview scroll is 1.
    assert_eq!(app.inbox_overlay.preview_scroll, 1);
    assert_eq!(app.inbox_overlay.selected, 0); // unchanged
}

#[test]
fn mouse_wheel_in_planet_list_overlay_moves_selection() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay.selected = 3;
    app.planet_overlay.scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: 2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.planet_overlay.selected, 1);
    assert_eq!(app.planet_overlay.scroll, 0);
}

#[test]
fn mouse_wheel_ignored_when_planet_prompt_open() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay.selected = 3;
    app.planet_overlay.scroll = 0;
    app.planet_overlay
        .open_prompt(PlanetOverlayPromptMode::SortMenu);

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.planet_overlay.selected, 3);
    assert_eq!(app.planet_overlay.scroll, 0);
}

#[test]
fn mouse_wheel_in_fleet_list_overlay_moves_selection() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::FleetList;
    app.fleet_overlay.selected = 2;
    app.fleet_overlay.scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.fleet_overlay.selected, 3);
}

#[test]
fn mouse_wheel_in_intel_overlay_moves_selection() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;
    app.intel_overlay.selected = 1;
    app.intel_overlay.scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.intel_overlay.selected, 2);
}

#[test]
fn mouse_wheel_intel_overlay_requests_redraw() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::IntelDatabase;
    app.intel_overlay.selected = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    let changed = app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert!(
        changed,
        "wheel scroll in Intel overlay should request redraw"
    );
}

#[test]
fn mouse_wheel_planet_overlay_requests_redraw() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    app.planet_overlay.selected = 3;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    let changed = app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: 2 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert!(
        changed,
        "wheel scroll in Planet overlay should request redraw"
    );
}

#[test]
fn mouse_wheel_inbox_list_requests_redraw() {
    let mut app = dash_app();
    app.queued_mail = vec![
        QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: app.game_data.conquest.game_year - 1,
            subject: "Test A".to_string(),
            body: "Body A".to_string(),
            recipient_deleted: false,
            sender_deleted: false,
            cancelled: false,
        },
        QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: app.game_data.conquest.game_year - 1,
            subject: "Test B".to_string(),
            body: "Body B".to_string(),
            recipient_deleted: false,
            sender_deleted: false,
            cancelled: false,
        },
    ];
    app.overlay = ActiveOverlay::Inbox;
    app.inbox_overlay.selected = 0;
    app.inbox_overlay.focus = crate::dashboard::app::state::InboxFocus::List;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    let changed = app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert!(changed, "wheel scroll in Inbox list should request redraw");
}

#[test]
fn mouse_wheel_inbox_preview_requests_redraw() {
    let mut app = dash_app();
    app.queued_mail = vec![QueuedPlayerMail {
        sender_empire_id: 2,
        recipient_empire_id: 1,
        year: app.game_data.conquest.game_year - 1,
        subject: "Test subject".to_string(),
        body: "Test body with enough length to create a preview.".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }];
    app.overlay = ActiveOverlay::Inbox;
    app.inbox_overlay.selected = 0;
    app.inbox_overlay.focus = crate::dashboard::app::state::InboxFocus::List;
    app.inbox_overlay.preview_scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    // Preview pane is on the right side of the body.
    let col = popup.x + popup.width as u16 / 2 + 2;
    let row = popup.y + 3;

    let changed = app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert!(
        changed,
        "wheel scroll in Inbox preview should request redraw"
    );
}

#[test]
fn mouse_wheel_diplomacy_requests_redraw() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::Diplomacy;
    app.diplomacy_overlay.selected = 0;
    app.diplomacy_overlay.scroll = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let popup = app.current_overlay_popup_rect(map_frame).unwrap();
    let col = popup.x + 3;
    let row = popup.y + 3;

    let changed = app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert!(
        changed,
        "wheel scroll in Diplomacy overlay should request redraw"
    );
}

#[test]
fn mouse_wheel_starmap_pan_requests_redraw() {
    let mut app = dash_app();
    app.starmap_viewport_x_min = 0;
    app.starmap_viewport_y_min = 0;

    let map_frame = dashboard_layout(&app).widgets.center_map;
    let col = map_frame.outer.col as u16 + 3;
    let row = map_frame.outer.row as u16 + 3;

    let changed = app.dispatch_mouse_event_for_repro(MouseEvent {
        kind: MouseEventKind::Scroll { lines: -1 },
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    });

    assert!(changed, "wheel pan on starmap should request redraw");
}

mod fleet_class_choices;
