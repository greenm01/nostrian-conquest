use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nc_compat::load_classic_core_files;
use nc_data::{EmpirePlanetEconomyRow, ProductionItemKind};
use nc_game::model::{ClassicLoginState, PlayerContext};
use nc_game::screen::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use nc_game::screen::table::{
    HorizontalAlign, LayoutRect, SplitTableRow, TableColumn, TableFooter, TableRowState,
    TableWidthMode, VerticalAlign, layout_standard_table_block, resolve_table_columns_for_widget,
    resolve_table_columns_for_widget_with_footer_floor, table_footer_scaffold_width,
    table_footer_width, table_render_width, write_split_table,
    write_stacked_table_window_with_states, write_table_row, write_table_window_with_cursor,
    write_table_window_with_cursor_at, write_table_window_with_states,
};
use nc_game::screen::{
    CommandMenu, EmpireProfileScreen, EnemiesScreen, FleetListFilter, FleetListScreen,
    FleetListSort, FleetRow, MessageComposeScreen, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetBuildScreen, PlanetDatabasePromptMode, PlanetDatabaseRow, PlanetDatabaseScreen,
    PlanetDatabaseSort, PlanetListFilter, PlanetListFilterPromptMode, PlanetListMode,
    PlanetListScreen, PlanetListSort, PlayfieldBuffer, RankingsScreen, ScreenFrame, ScreenGeometry,
};
use nc_game::theme::classic;

fn row_text(buffer: &PlayfieldBuffer, row: usize) -> String {
    buffer.row(row).iter().map(|cell| cell.ch).collect()
}

fn line_index_containing(buffer: &PlayfieldBuffer, needle: &str) -> usize {
    (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains(needle))
        .unwrap_or_else(|| panic!("missing line containing {needle:?}"))
}

fn sample_fleet_rows() -> Vec<FleetRow> {
    vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 1,
        coords: [12, 34],
        target_coords: [12, 34],
        order_code: 0,
        current_speed: 7,
        max_speed: 7,
        eta_label: "0".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 2,
        loaded_armies: 3,
        order_label: "Holding".to_string(),
        composition_label: "DD 05".to_string(),
        table_ships_label: "DD 05".to_string(),
        damaged_ships: 0,
        damaged_label: String::new(),
        join_host_fleet_number: None,
    }]
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn joined_player_context() -> PlayerContext {
    PlayerContext {
        record_index_1_based: 1,
        is_joined: true,
        classic_login_state: ClassicLoginState::ReturningPlayer,
        empire_name: "Player 1".to_string(),
        handle: "P1".to_string(),
    }
}

#[test]
fn playfield_geometry_is_80x25() {
    let buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    assert_eq!(PLAYFIELD_WIDTH, 80);
    assert_eq!(PLAYFIELD_HEIGHT, 25);
    assert_eq!(buffer.width(), 80);
    assert_eq!(buffer.height(), 25);
}

#[test]
fn standard_table_places_scrollbar_just_right_of_table_border() {
    let columns = [TableColumn::left("Name", 10)];
    let rows = vec![
        vec!["Alpha".to_string()],
        vec!["Beta".to_string()],
        vec!["Gamma".to_string()],
        vec!["Delta".to_string()],
    ];
    let row_states = vec![
        TableRowState::Normal,
        TableRowState::Disabled,
        TableRowState::Normal,
        TableRowState::Normal,
    ];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_table_window_with_states(
        &mut buffer,
        2,
        &columns,
        &rows,
        1,
        3,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(2),
        0,
        Some(&row_states),
    );

    let scrollbar_col = table_render_width(&columns);
    assert_eq!(buffer.row(5)[scrollbar_col].ch, '^');
    assert_eq!(buffer.row(7)[scrollbar_col].ch, 'v');
    assert_eq!(buffer.row(2)[0].style, classic::table_chrome_style());
    assert_eq!(buffer.row(4)[0].style, classic::table_chrome_style());
    assert_eq!(
        buffer.row(5)[scrollbar_col].style,
        classic::table_chrome_style()
    );
    assert!(row_text(&buffer, 5).contains("Beta"));
    assert!(row_text(&buffer, 6).contains("Gamma"));
}

#[test]
#[should_panic(expected = "scrollable table must leave a gutter to the right of its border")]
fn scrollable_table_panics_when_border_would_consume_last_playfield_col() {
    let columns = [TableColumn::left("Name", 78)];
    let rows = vec![
        vec!["Alpha".to_string()],
        vec!["Beta".to_string()],
        vec!["Gamma".to_string()],
        vec!["Delta".to_string()],
    ];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_table_window_with_states(
        &mut buffer,
        2,
        &columns,
        &rows,
        0,
        3,
        classic::status_value_style(),
        classic::status_value_style(),
        None,
        0,
        None,
    );
}

#[test]
fn centered_table_block_expands_to_match_command_footer_width() {
    let columns = [TableColumn::left("ID", 2), TableColumn::left("Name", 8)];
    let footer = TableFooter::CommandBar {
        hotkeys_markup: "? <Q>",
        default: Some("02,02"),
        input: "",
    };
    let layout = layout_standard_table_block(
        LayoutRect::new(0, 0, PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT),
        &columns,
        3,
        Some("COMMISSION SHIPS:"),
        Some(footer),
        false,
        HorizontalAlign::Center,
        VerticalAlign::Center,
    );

    let table_width = table_render_width(&columns);
    assert!(table_footer_width(footer) > table_width);
    assert_eq!(
        layout.table_col,
        (PLAYFIELD_WIDTH - (table_footer_scaffold_width(footer) + 1)) / 2
    );
    assert_eq!(layout.title_col, layout.table_col + 1);
    assert_eq!(layout.command_col, layout.table_col + 1);
}

#[test]
fn widget_minimum_width_ignores_live_footer_input() {
    let base_columns = [
        TableColumn::center("", 1),
        TableColumn::left("Theme", 22),
        TableColumn::left("Type", 8),
    ];
    let rows = vec![vec![
        "*".to_string(),
        "Matrix".to_string(),
        "Theme".to_string(),
    ]];
    let without_input = resolve_table_columns_for_widget(
        &base_columns,
        &rows,
        PLAYFIELD_WIDTH,
        false,
        TableWidthMode::Compact,
        Some("COLOR THEMES:"),
        Some(TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: Some("Matrix"),
            input: "",
        }),
    );
    let with_input = resolve_table_columns_for_widget(
        &base_columns,
        &rows,
        PLAYFIELD_WIDTH,
        false,
        TableWidthMode::Compact,
        Some("COLOR THEMES:"),
        Some(TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: Some("Matrix"),
            input: "tokyonight",
        }),
    );

    assert_eq!(without_input, with_input);
}

#[test]
fn widget_minimum_width_can_freeze_selection_driven_footer_defaults() {
    let base_columns = [
        TableColumn::center("", 1),
        TableColumn::left("Theme", 22),
        TableColumn::left("Type", 8),
    ];
    let rows = vec![
        vec!["*".to_string(), "Matrix".to_string(), "Theme".to_string()],
        vec!["".to_string(), "One Dark".to_string(), "Theme".to_string()],
        vec![
            "".to_string(),
            "Catppuccin Mocha".to_string(),
            "Theme".to_string(),
        ],
    ];
    let footer_floor = table_footer_scaffold_width(TableFooter::CommandBar {
        hotkeys_markup: "? <Q>",
        default: Some("Catppuccin Mocha"),
        input: "",
    });
    let short_default = resolve_table_columns_for_widget_with_footer_floor(
        &base_columns,
        &rows,
        PLAYFIELD_WIDTH,
        false,
        TableWidthMode::Compact,
        Some("COLOR THEMES:"),
        Some(TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: Some("Matrix"),
            input: "",
        }),
        footer_floor,
    );
    let long_default = resolve_table_columns_for_widget_with_footer_floor(
        &base_columns,
        &rows,
        PLAYFIELD_WIDTH,
        false,
        TableWidthMode::Compact,
        Some("COLOR THEMES:"),
        Some(TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: Some("One Dark"),
            input: "",
        }),
        footer_floor,
    );

    assert_eq!(short_default, long_default);
}

#[test]
fn centered_table_column_centers_single_character_values() {
    let columns = [TableColumn::center("Sel", 3)];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_table_row(
        &mut buffer,
        2,
        &columns,
        &["X"],
        classic::status_value_style(),
    );

    assert_eq!(row_text(&buffer, 2).trim_end(), "│ X │");
}

#[test]
fn stacked_header_table_renders_top_and_bottom_headers() {
    let columns = [
        TableColumn::left("Coord", 7),
        TableColumn::left("Planet", 14),
        TableColumn::right("Own", 3),
    ];
    let rows = vec![vec![
        "12,34".to_string(),
        "Aurora".to_string(),
        "01".to_string(),
    ]];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_stacked_table_window_with_states(
        &mut buffer,
        1,
        &["Meta", "", ""],
        &columns,
        &rows,
        0,
        1,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(0),
        0,
        None,
    );

    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(2).contains("│Meta"));
    assert_eq!(buffer.plain_line(2).matches('│').count(), columns.len() + 1);
    assert!(buffer.plain_line(3).contains("Coord"));
    assert!(buffer.plain_line(5).contains("Aurora"));
}

#[test]
fn split_table_renders_both_halves() {
    let columns = [
        TableColumn::left("NO.", 4),
        TableColumn::left("UNIT TYPE", 19),
        TableColumn::right("COST", 4),
        TableColumn::right("QTY.", 5),
    ];
    let rows = vec![SplitTableRow {
        left_cells: vec![
            "<0>".to_string(),
            "DONE".to_string(),
            String::new(),
            String::new(),
        ],
        right_cells: vec![
            "<5>".to_string(),
            "Scouts".to_string(),
            "1".to_string(),
            "(0)".to_string(),
        ],
    }];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_split_table(
        &mut buffer,
        5,
        &columns,
        &columns,
        &rows,
        classic::status_value_style(),
    );

    assert!(buffer.plain_line(5).starts_with("┌"));
    assert!(buffer.plain_line(5).ends_with("┐"));
    assert!(!buffer.plain_line(5).contains("          "));
    let header = row_text(&buffer, 6);
    assert_eq!(header.matches("NO.").count(), 2);
    assert!(header.contains("QTY."));
    assert!(buffer.plain_line(8).contains("DONE"));
    assert!(buffer.plain_line(8).contains("Scouts"));
}

#[test]
fn empire_profile_aligns_armies_and_ground_batteries_with_active_duty_list() {
    let mut screen = EmpireProfileScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };

    let buffer = screen
        .render_with_menu(&frame, CommandMenu::General)
        .expect("render empire profile");

    let destroyers_colon = buffer.plain_line(9).find(':').expect("destroyers colon");
    let armies_colon = buffer.plain_line(16).find(':').expect("armies colon");
    let batteries_colon = buffer
        .plain_line(17)
        .find(':')
        .expect("ground batteries colon");

    assert!(buffer.plain_line(8).starts_with(' '));
    assert!(buffer.plain_line(9).starts_with(' '));
    assert!(buffer.plain_line(16).starts_with(' '));
    assert!(buffer.plain_line(17).starts_with(' '));
    assert_eq!(armies_colon, destroyers_colon);
    assert_eq!(batteries_colon, destroyers_colon);
}

#[test]
fn planet_database_screen_uses_stacked_header_table() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = vec![PlanetDatabaseRow {
        planet_record_index_1_based: 1,
        coords: [12, 34],
        known_name: Some("Aurora".to_string()),
        known_owner_empire_id: Some(1),
        known_owner_name: Some("01".to_string()),
        known_max_production: Some(120),
        known_year_seen: Some(3001),
        known_armies: Some(10),
        known_batteries: Some(4),
        known_starbase_count: Some(1),
        known_current_production: Some(80),
        known_stored_points: Some(25),
        known_scout_year: Some(3001),
        name_label: "Aurora".to_string(),
        owner_label: "01".to_string(),
        max_prod_label: "120".to_string(),
        year_seen_label: "3001".to_string(),
        armies_label: "10".to_string(),
        batteries_label: "4".to_string(),
        starbase_count_label: "1".to_string(),
        current_prod_label: "80".to_string(),
        stored_points_label: "25".to_string(),
        year_scout_label: "3001".to_string(),
    }];

    let buffer = screen
        .render_list(
            ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            [12, 34],
            "",
            None,
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render database list");

    let title_col = buffer
        .plain_line(0)
        .find("TOTAL PLANET DATABASE:")
        .expect("title col");
    let border_col = buffer.plain_line(1).find('┌').expect("table col");
    assert_eq!(title_col, border_col + 1);
    assert!(buffer.plain_line(2).contains("│Coord"));
    assert!(buffer.plain_line(2).contains("Max"));
    assert!(buffer.plain_line(2).contains("Year"));
    assert!(buffer.plain_line(2).contains("Curr"));
    assert!(buffer.plain_line(2).contains("Trsry"));
    assert_eq!(buffer.plain_line(2).matches('│').count(), 12);
    assert!(buffer.plain_line(2).trim_end().ends_with('│'));
    assert!(buffer.plain_line(3).contains("(XX,YY)"));
    assert!(buffer.plain_line(3).contains("Planet Name"));
    assert!(buffer.plain_line(3).contains("ARs"));
    assert!(buffer.plain_line(3).contains("GBs"));
    assert!(buffer.plain_line(3).contains("SBs"));
    assert!(buffer.plain_line(3).contains("Scout"));
    assert!(!buffer.plain_line(3).contains("Intel"));
    assert!(buffer.plain_line(5).contains("(12,34)"));
    assert!(buffer.plain_line(5).contains("Aurora"));
    assert_eq!(
        buffer.plain_line(7).find("COMMAND").expect("command col"),
        border_col + 1
    );
}

#[test]
fn planet_database_filter_prompt_aligns_with_centered_table() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = vec![PlanetDatabaseRow {
        planet_record_index_1_based: 1,
        coords: [12, 34],
        known_name: Some("Aurora".to_string()),
        known_owner_empire_id: Some(1),
        known_owner_name: Some("01".to_string()),
        known_max_production: Some(120),
        known_year_seen: Some(3001),
        known_armies: Some(10),
        known_batteries: Some(4),
        known_starbase_count: Some(1),
        known_current_production: Some(80),
        known_stored_points: Some(25),
        known_scout_year: Some(3001),
        name_label: "Aurora".to_string(),
        owner_label: "01".to_string(),
        max_prod_label: "120".to_string(),
        year_seen_label: "3001".to_string(),
        armies_label: "10".to_string(),
        batteries_label: "4".to_string(),
        starbase_count_label: "1".to_string(),
        current_prod_label: "80".to_string(),
        stored_points_label: "25".to_string(),
        year_scout_label: "3001".to_string(),
    }];

    let browse = screen
        .render_list(
            ScreenGeometry::local_default(),
            &rows,
            PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            [12, 34],
            "",
            None,
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render database list");
    let browse_row = line_index_containing(&browse, "COMMAND <- ");

    let filter = screen
        .render_filter_prompt(
            ScreenGeometry::local_default(),
            &rows,
            PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            PlanetDatabasePromptMode::FilterMenu,
            "",
            "",
            None,
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render database filter prompt");
    let border_col = filter.plain_line(1).find('┌').expect("table col");
    let prompt_row = line_index_containing(&filter, "COMMAND <- Filter column [?] ");
    assert_eq!(
        filter
            .plain_line(prompt_row)
            .find("COMMAND")
            .expect("prompt col"),
        border_col + 1
    );
    assert_eq!(prompt_row, browse_row);

    let sort = screen
        .render_filter_prompt(
            ScreenGeometry::local_default(),
            &rows,
            PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            PlanetDatabasePromptMode::SortMenu,
            "",
            "",
            None,
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render database sort prompt");
    assert_eq!(
        line_index_containing(&sort, "COMMAND <- Sort column [?] "),
        browse_row
    );
}

#[test]
fn planet_database_24_row_door_keeps_bottom_border_above_command_line() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = (0..30)
        .map(|idx| PlanetDatabaseRow {
            planet_record_index_1_based: idx + 1,
            coords: [idx as u8 % 20, idx as u8 / 20],
            known_name: Some(format!("Aurora {idx:02}")),
            known_owner_empire_id: Some(1),
            known_owner_name: Some("01".to_string()),
            known_max_production: Some(120),
            known_year_seen: Some(3001),
            known_armies: Some(10),
            known_batteries: Some(4),
            known_starbase_count: Some(1),
            known_current_production: Some(80),
            known_stored_points: Some(25),
            known_scout_year: Some(3001),
            name_label: format!("Aurora {idx:02}"),
            owner_label: "01".to_string(),
            max_prod_label: "120".to_string(),
            year_seen_label: "3001".to_string(),
            armies_label: "10".to_string(),
            batteries_label: "4".to_string(),
            starbase_count_label: "1".to_string(),
            current_prod_label: "80".to_string(),
            stored_points_label: "25".to_string(),
            year_scout_label: "3001".to_string(),
        })
        .collect::<Vec<_>>();

    let buffer = screen
        .render_list(
            ScreenGeometry::for_door(Some(24)),
            &rows,
            nc_game::screen::PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            [0, 0],
            "",
            None,
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render 24-row database list");

    assert_eq!(buffer.height(), 24);
    assert!(buffer.plain_line(22).contains('└'));
    assert!(buffer.plain_line(23).contains("COMMAND"));
}

#[test]
fn fleet_list_sort_and_filter_prompts_use_browse_footer_row() {
    let mut screen = FleetListScreen::new();
    let rows = sample_fleet_rows();

    let browse = screen
        .render(
            ScreenGeometry::local_default(),
            &rows,
            FleetListSort::Id,
            nc_game::screen::SortDirection::Asc,
            FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("render fleet browse");
    let browse_row = line_index_containing(&browse, "COMMAND <- ");

    let sort = screen
        .render_sort_prompt(
            ScreenGeometry::local_default(),
            &rows,
            FleetListSort::Id,
            nc_game::screen::SortDirection::Asc,
            FleetListFilter::All,
            0,
            0,
            "id",
            "",
            None,
            None,
        )
        .expect("render fleet sort prompt");
    assert_eq!(
        line_index_containing(&sort, "COMMAND <- Sort column [?]"),
        browse_row
    );

    let filter = screen
        .render_filter_prompt(
            ScreenGeometry::local_default(),
            &rows,
            FleetListSort::Id,
            nc_game::screen::SortDirection::Asc,
            FleetListFilter::All,
            0,
            0,
        )
        .expect("render fleet filter prompt");
    assert_eq!(
        line_index_containing(&filter, "COMMAND <- Filter column [?] "),
        browse_row
    );
}

#[test]
fn planet_brief_list_uses_database_style_stacked_header_and_owned_planet_columns() {
    let mut screen = PlanetListScreen::new();
    let mut game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let planet = &mut game_data.planets[0];
    planet.build_queue[0].points_owed = 5;
    planet.build_queue[0].kind = nc_data::ProductionItemKind::from_code(1);
    planet.build_queue[1].points_owed = 40;
    planet.build_queue[1].kind = nc_data::ProductionItemKind::from_code(6);
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };
    let rows = vec![EmpirePlanetEconomyRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        planet_name: "Player 1 HW".to_string(),
        present_production: 100,
        potential_production: 100,
        stored_production_points: 165,
        yearly_tax_revenue: 65,
        yearly_growth_delta: 0,
        build_capacity: 100,
        has_friendly_starbase: false,
        armies: 10,
        ground_batteries: 4,
        is_homeworld_seed: true,
    }];

    let buffer = screen
        .render_brief_list(
            &frame,
            nc_game::screen::PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::PlanetListFilter::All,
            0,
            0,
            "",
            None,
            false,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("render brief list");

    let title_col = buffer
        .plain_line(0)
        .find("PLANET LIST: CUR DESCENDING ALL")
        .expect("title col");
    let border_col = buffer.plain_line(1).find('┌').expect("table col");
    assert_eq!(title_col, border_col + 1);
    assert!(border_col > 0);
    assert!(buffer.plain_line(2).contains("│Coord"));
    assert!(buffer.plain_line(2).contains("Max"));
    assert!(buffer.plain_line(2).contains("Curr"));
    assert!(buffer.plain_line(2).contains("Trsry"));
    assert!(buffer.plain_line(2).contains("Build"));
    assert!(buffer.plain_line(2).contains("Star"));
    assert_eq!(buffer.plain_line(2).matches('│').count(), 14);
    assert!(buffer.plain_line(3).contains("(XX,YY)"));
    assert!(buffer.plain_line(3).contains("Planet Name"));
    assert!(buffer.plain_line(3).contains("Prod"));
    assert!(buffer.plain_line(3).contains("Points"));
    assert!(buffer.plain_line(3).contains("Queue"));
    assert!(buffer.plain_line(3).contains("Dock"));
    assert!(buffer.plain_line(3).contains("SBs"));
    assert!(buffer.plain_line(3).contains("ARs"));
    assert!(buffer.plain_line(3).contains("GBs"));
    assert!(buffer.plain_line(5).contains("Player 1 HW"));
    assert!(buffer.plain_line(5).contains("120"));
    assert!(buffer.plain_line(5).contains("55"));
    assert!(buffer.plain_line(5).contains("3"));
    assert!(buffer.plain_line(5).contains("0"));
    assert_eq!(
        buffer.plain_line(7).find("COMMAND").expect("command col"),
        border_col + 1
    );
    assert_eq!(
        buffer.row(5)[border_col + 1].style,
        classic::selected_row_style()
    );
    let name_col = buffer.plain_line(5).find("Player 1 HW").expect("name col");
    assert_eq!(buffer.row(5)[name_col].style, classic::table_body_style());
    assert_ne!(
        buffer.row(5)[border_col].style,
        classic::selected_row_style()
    );
}

#[test]
fn planet_brief_list_shows_zero_build_queue_when_no_orders_are_queued() {
    let mut screen = PlanetListScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };
    let rows = vec![EmpirePlanetEconomyRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        planet_name: "Player 1 HW".to_string(),
        present_production: 100,
        potential_production: 100,
        stored_production_points: 165,
        yearly_tax_revenue: 65,
        yearly_growth_delta: 0,
        build_capacity: 100,
        has_friendly_starbase: false,
        armies: 10,
        ground_batteries: 4,
        is_homeworld_seed: true,
    }];

    let buffer = screen
        .render_brief_list(
            &frame,
            nc_game::screen::PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::PlanetListFilter::All,
            0,
            0,
            "",
            None,
            false,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("render brief list without queued builds");

    let line = buffer.plain_line(5);
    let cells = line.split('│').collect::<Vec<_>>();
    assert_eq!(cells[9].trim(), "0");
}

#[test]
fn planet_brief_list_keeps_table_width_stable_across_footer_modes() {
    let mut screen = PlanetListScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };
    let rows = vec![EmpirePlanetEconomyRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        planet_name: "Player 1 HW".to_string(),
        present_production: 100,
        potential_production: 100,
        stored_production_points: 165,
        yearly_tax_revenue: 65,
        yearly_growth_delta: 0,
        build_capacity: 100,
        has_friendly_starbase: false,
        armies: 10,
        ground_batteries: 4,
        is_homeworld_seed: true,
    }];

    let command_bar = screen
        .render_brief_list(
            &frame,
            nc_game::screen::PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::PlanetListFilter::All,
            0,
            0,
            "",
            None,
            false,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("render brief list command bar");
    let auto_commission = screen
        .render_brief_list(
            &frame,
            nc_game::screen::PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::PlanetListFilter::All,
            0,
            0,
            "",
            None,
            true,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("render brief list auto-commission prompt");
    let load_quantity = screen
        .render_brief_list(
            &frame,
            nc_game::screen::PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::PlanetListFilter::All,
            0,
            0,
            "",
            None,
            false,
            false,
            Some("Load Armies: How many armies? "),
            "255",
            "",
            None,
            None,
            None,
        )
        .expect("render brief list load quantity prompt");

    let command_bar_right = command_bar
        .plain_line(1)
        .rfind('┐')
        .expect("command bar border");
    let auto_commission_right = auto_commission
        .plain_line(1)
        .rfind('┐')
        .expect("auto-commission border");
    let load_quantity_right = load_quantity
        .plain_line(1)
        .rfind('┐')
        .expect("load quantity border");

    assert_eq!(command_bar_right, auto_commission_right);
    assert_eq!(command_bar_right, load_quantity_right);
}

#[test]
fn planet_brief_list_sort_and_filter_prompts_use_browse_footer_row() {
    let mut screen = PlanetListScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };
    let rows = vec![EmpirePlanetEconomyRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        planet_name: "Player 1 HW".to_string(),
        present_production: 100,
        potential_production: 100,
        stored_production_points: 165,
        yearly_tax_revenue: 65,
        yearly_growth_delta: 0,
        build_capacity: 100,
        has_friendly_starbase: false,
        armies: 10,
        ground_batteries: 4,
        is_homeworld_seed: true,
    }];

    let browse = screen
        .render_brief_list(
            &frame,
            PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            PlanetListFilter::All,
            0,
            0,
            "",
            None,
            false,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("render planet brief list");
    let browse_row = line_index_containing(&browse, "COMMAND <- ");

    let sort = screen
        .render_sort_prompt(
            &frame,
            PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            PlanetListFilter::All,
            0,
            0,
            "cur",
            "",
            None,
            None,
        )
        .expect("render planet sort prompt");
    assert_eq!(
        line_index_containing(&sort, "SORT DESC <- Sort column [?]"),
        browse_row
    );

    let filter = screen
        .render_filter_prompt(
            &frame,
            PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            PlanetListFilter::All,
            0,
            0,
            PlanetListFilterPromptMode::FilterMenu,
            "",
            "",
            None,
        )
        .expect("render planet filter prompt");
    assert_eq!(
        line_index_containing(&filter, "COMMAND <- Filter column [?] "),
        browse_row
    );
}

#[test]
fn filter_prompt_inline_status_stays_on_browse_footer_row_for_all_tables() {
    let mut fleet_screen = FleetListScreen::new();
    let fleet_rows = sample_fleet_rows();
    let fleet_browse = fleet_screen
        .render(
            ScreenGeometry::local_default(),
            &fleet_rows,
            FleetListSort::Id,
            nc_game::screen::SortDirection::Asc,
            FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("render fleet browse");
    let fleet_browse_row = line_index_containing(&fleet_browse, "COMMAND <- ");
    let fleet_filter = fleet_screen
        .render_filter_prompt_with_filter_clause(
            ScreenGeometry::local_default(),
            &fleet_rows,
            FleetListSort::Id,
            nc_game::screen::SortDirection::Asc,
            FleetListFilter::All,
            None,
            0,
            0,
            nc_game::screen::FleetListFilterPromptMode::Column,
            "all",
            "",
            Some(" Ambiguous: spd/shi"),
            None,
            None,
        )
        .expect("render fleet filter prompt");
    assert_eq!(
        line_index_containing(&fleet_filter, "COMMAND <- Ambiguous: spd/shi"),
        fleet_browse_row
    );

    let mut planet_screen = PlanetListScreen::new();
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };
    let planet_rows = vec![EmpirePlanetEconomyRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        planet_name: "Player 1 HW".to_string(),
        present_production: 100,
        potential_production: 100,
        stored_production_points: 165,
        yearly_tax_revenue: 65,
        yearly_growth_delta: 0,
        build_capacity: 100,
        has_friendly_starbase: false,
        armies: 10,
        ground_batteries: 4,
        is_homeworld_seed: true,
    }];
    let planet_browse = planet_screen
        .render_brief_list(
            &frame,
            PlanetListMode::Brief,
            &planet_rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            PlanetListFilter::All,
            0,
            0,
            "",
            None,
            false,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("render planet browse");
    let planet_browse_row = line_index_containing(&planet_browse, "COMMAND <- ");
    let planet_filter = planet_screen
        .render_filter_prompt(
            &frame,
            PlanetListMode::Brief,
            &planet_rows,
            PlanetListSort::CurrentProduction,
            nc_game::screen::SortDirection::Desc,
            PlanetListFilter::All,
            0,
            0,
            PlanetListFilterPromptMode::FilterMenu,
            "",
            "",
            Some(" Ambiguous: sta/sbs"),
        )
        .expect("render planet filter prompt");
    assert_eq!(
        line_index_containing(&planet_filter, "COMMAND <- Ambiguous: sta/sbs"),
        planet_browse_row
    );

    let mut database_screen = PlanetDatabaseScreen::new();
    let database_rows = vec![PlanetDatabaseRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        known_name: Some("Aurora".to_string()),
        known_owner_empire_id: Some(1),
        known_owner_name: Some("01".to_string()),
        known_max_production: Some(120),
        known_year_seen: Some(3001),
        known_armies: Some(10),
        known_batteries: Some(4),
        known_starbase_count: Some(1),
        known_current_production: Some(80),
        known_stored_points: Some(25),
        known_scout_year: Some(3001),
        name_label: "Aurora".to_string(),
        owner_label: "01".to_string(),
        max_prod_label: "120".to_string(),
        year_seen_label: "3001".to_string(),
        armies_label: "10".to_string(),
        batteries_label: "4".to_string(),
        starbase_count_label: "1".to_string(),
        current_prod_label: "80".to_string(),
        stored_points_label: "25".to_string(),
        year_scout_label: "3001".to_string(),
    }];
    let database_browse = database_screen
        .render_list(
            ScreenGeometry::local_default(),
            &database_rows,
            PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            [0, 0],
            "",
            None,
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render database browse");
    let database_browse_row = line_index_containing(&database_browse, "COMMAND <- ");
    let database_filter = database_screen
        .render_filter_prompt(
            ScreenGeometry::local_default(),
            &database_rows,
            PlanetDatabaseSort::Location,
            nc_game::screen::SortDirection::Asc,
            nc_game::screen::PlanetDatabaseFilter::All,
            0,
            0,
            PlanetDatabasePromptMode::FilterMenu,
            "",
            "",
            Some(" Ambiguous: see/sbs/sco"),
            nc_game::screen::CommandMenu::Planet,
        )
        .expect("render database filter prompt");
    assert_eq!(
        line_index_containing(&database_filter, "COMMAND <- Ambiguous: see/sbs/sco"),
        database_browse_row
    );
}

#[test]
fn selected_column_can_target_second_column_without_highlighting_first() {
    let columns = [
        TableColumn::center("", 1),
        TableColumn::left("Theme", 12),
        TableColumn::left("Type", 8),
    ];
    let rows = vec![vec![
        "*".to_string(),
        "Mono".to_string(),
        "Mono".to_string(),
    ]];
    let mut buffer = PlayfieldBuffer::new(40, 8, classic::body_style());

    write_table_window_with_cursor(
        &mut buffer,
        1,
        &columns,
        &rows,
        0,
        1,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(0),
        1,
    );

    let marker_col = buffer
        .row(4)
        .iter()
        .position(|cell| cell.ch == '*')
        .expect("marker col");
    let name_col = buffer
        .row(4)
        .iter()
        .position(|cell| cell.ch == 'M')
        .expect("name col");
    assert_eq!(buffer.row(4)[marker_col].style, classic::table_body_style());
    assert_eq!(buffer.row(4)[name_col].style, classic::selected_row_style());
}

#[test]
fn cursor_table_can_render_at_nonzero_column() {
    let columns = [
        TableColumn::center("", 1),
        TableColumn::left("Theme", 12),
        TableColumn::left("Type", 8),
    ];
    let rows = vec![vec![
        "*".to_string(),
        "Mono".to_string(),
        "Mono".to_string(),
    ]];
    let mut buffer = PlayfieldBuffer::new(40, 8, classic::body_style());

    write_table_window_with_cursor_at(
        &mut buffer,
        1,
        6,
        &columns,
        &rows,
        0,
        1,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(0),
        1,
    );

    assert_eq!(buffer.plain_line(1).find('┌'), Some(6));
    assert!(buffer.plain_line(4).contains("│*│Mono"));
}

#[test]
fn compose_recipient_picker_centers_block_and_pins_prompt_to_table() {
    let mut screen = MessageComposeScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };

    let buffer = screen
        .render_recipient(&frame, "", None, 0, 0)
        .expect("render recipient picker");

    let title_row = (0..buffer.height())
        .find(|row| {
            buffer
                .plain_line(*row)
                .contains("COMMUNICATE (SEND MESSAGE):")
        })
        .expect("title row");
    let title_col = buffer
        .plain_line(title_row)
        .find("COMMUNICATE (SEND MESSAGE):")
        .expect("title col");
    let table_row = (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains('┌'))
        .expect("table row");
    let table_col = buffer.plain_line(table_row).find('┌').expect("table col");
    let command_row = (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains("COMMAND <- ? D <Q>"))
        .expect("command row");
    let command_col = buffer
        .plain_line(command_row)
        .find("COMMAND")
        .expect("command col");

    assert_eq!(title_col, table_col + 1);
    assert_eq!(command_col, table_col + 1);
    assert!((0..buffer.height()).all(|row| !buffer.plain_line(row).contains("Available empires:")));
    assert!(
        (0..buffer.height())
            .all(|row| !buffer.plain_line(row).contains("queued outgoing messages"))
    );
}

#[test]
fn rankings_screen_centers_block_and_pins_dismiss_prompt_to_table() {
    let mut screen = RankingsScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };

    let buffer = screen
        .render_table(
            &frame,
            nc_data::EmpireProductionRankingSort::Production,
            nc_game::screen::CommandMenu::General,
        )
        .expect("render rankings screen");

    let title_row = (0..buffer.height())
        .find(|row| {
            buffer
                .plain_line(*row)
                .contains("OTHER EMPIRES (RANKINGS):")
        })
        .expect("title row");
    let title_col = buffer
        .plain_line(title_row)
        .find("OTHER EMPIRES (RANKINGS):")
        .expect("title col");
    let table_row = (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains('┌'))
        .expect("table row");
    let table_col = buffer.plain_line(table_row).find('┌').expect("table col");
    let dismiss_row = (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains("(slap a key)"))
        .expect("dismiss row");
    let dismiss_col = buffer
        .plain_line(dismiss_row)
        .find("(slap a key)")
        .expect("dismiss col");

    assert_eq!(title_col, table_col + 1);
    assert_eq!(dismiss_col, table_col + 1);
}

#[test]
fn enemies_screen_centers_block_and_pins_prompt_to_table() {
    let mut screen = EnemiesScreen::new();
    let game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let owned_planet_years = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        player_activity_states: &[],
        player_lifecycle_states: &[],
        winner_state: nc_data::WinnerState::default(),
        planet_intel_snapshots: &planet_intel_snapshots,
        owned_planet_years: &owned_planet_years,
        geometry: ScreenGeometry::local_default(),
    };

    let buffer = screen
        .render(&frame, "", None, 0, 0)
        .expect("render enemies screen");

    let title_row = (0..buffer.height())
        .find(|row| {
            buffer
                .plain_line(*row)
                .contains("ENEMIES, DECLARE OR LIST:")
        })
        .expect("title row");
    let title_col = buffer
        .plain_line(title_row)
        .find("ENEMIES, DECLARE OR LIST:")
        .expect("title col");
    let table_row = (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains('┌'))
        .expect("table row");
    let table_col = buffer.plain_line(table_row).find('┌').expect("table col");
    let command_row = (0..buffer.height())
        .find(|row| buffer.plain_line(*row).contains("COMMAND <- ? <Q>"))
        .expect("command row");
    let command_col = buffer
        .plain_line(command_row)
        .find("COMMAND")
        .expect("command col");

    assert_eq!(title_col, table_col + 1);
    assert_eq!(command_col, table_col + 1);
    assert!(table_col > 0);
}

#[test]
fn planet_build_specify_screen_uses_split_table() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            coords: [6, 5],
            planet_name: "Not Named Yet".to_string(),
            present_production: 100,
            potential_production: 100,
            stored_production_points: 50,
            yearly_tax_revenue: 50,
            yearly_growth_delta: 0,
            build_capacity: 100,
            has_friendly_starbase: false,
            armies: 10,
            ground_batteries: 4,
            is_homeworld_seed: true,
        },
        committed_points: 10,
        budget: 50,
        points_left: 40,
        building_count: 2,
        docked_count: 3,
    };
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 10,
    }];

    let buffer = screen
        .render_specify(&view, &orders, "", None, None)
        .expect("render specify");

    let table_col = buffer.plain_line(1).find('┌').expect("table col");
    assert!(table_col > 0);
    assert_eq!(buffer.plain_line(2).matches("NO.").count(), 2);
    assert!(buffer.plain_line(2).contains("QTY."));
    assert!(buffer.plain_line(4).contains("<01>"));
    assert!(buffer.plain_line(4).contains("<06>"));
    assert!(buffer.plain_line(4).contains("Destroyers"));
    assert!(buffer.plain_line(4).contains("05"));
    assert!(buffer.plain_line(6).contains("<09>"));
    assert!(buffer.plain_line(6).contains("02"));
    assert!(buffer.plain_line(7).contains("<10>"));
    assert!(buffer.plain_line(7).contains("20"));
    assert!(buffer.plain_line(8).contains("<05>"));
    assert!(!buffer.plain_line(4).contains("DONE"));
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 50 points.")
    }));
}
