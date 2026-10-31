use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_data::{EmpirePlanetEconomyRow, ProductionItemKind};
use nc_game::app::Action;
use nc_game::domains::planet::PlanetAction;
use nc_game::screen::table::{TABLE_TEXT_INSET, TableColumn, table_render_width};
use nc_game::screen::{
    BuildUnitSpec, PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetBuildScreen, ScreenGeometry,
};

#[test]
fn build_menu_renders_compact_queue_and_stardock_counts() {
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
        treasury_left: 40,
        points_left: 40,
        building_count: 5,
        docked_count: 3,
    };

    let buffer = screen
        .render_menu(&view, &[], None, false, false, [0, 0], "", None, false)
        .expect("render menu");

    assert_eq!(
        buffer.plain_line(7),
        " BUILD COMMAND <- ? X V P R C N S A D I <Q> ->"
    );
    assert_eq!(
        buffer.plain_line(13),
        " There are no starbases orbiting planet \"Not Named Yet\"."
    );
    assert_eq!(
        buffer.plain_line(14),
        " Standard building restrictions apply, except to a first starbase."
    );
    assert_eq!(
        buffer.plain_line(15),
        " Treasury: 40 PP.  Build capacity: 100 PP.  Budget remaining: 40."
    );
    assert_eq!(buffer.plain_line(16), "");
    assert_eq!(buffer.plain_line(17), " Building: 5   Docked: 3");
}

#[test]
fn abort_confirmation_says_cancelling_releases_only_unpaid_points() {
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
        committed_points: 45,
        budget: 50,
        treasury_left: 50,
        points_left: 5,
        building_count: 1,
        docked_count: 0,
    };
    let orders = [PlanetBuildOrder {
        kind: ProductionItemKind::Battleship,
        points_remaining: 45,
    }];

    let buffer = screen
        .render_abort_confirm(&view, &orders)
        .expect("render abort confirm");

    let line = buffer.plain_line(12);
    assert_eq!(
        line,
        " Cancelling releases 45 unpaid points. Points already spent are not refunded."
    );
    assert!(
        line.chars().count() <= 80,
        "the line fits the 80-column screen"
    );
}

#[test]
fn build_list_renders_queue_columns_without_dock() {
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
        treasury_left: 50,
        points_left: 40,
        building_count: 2,
        docked_count: 0,
    };

    let rows = vec![
        PlanetBuildListRow {
            kind: ProductionItemKind::Destroyer,
            unit_label: "Destroyers".to_string(),
            points: 5,
            queue_qty: 2,
        },
        PlanetBuildListRow {
            kind: ProductionItemKind::Army,
            unit_label: "Armies".to_string(),
            points: 2,
            queue_qty: 4,
        },
    ];

    let buffer = screen
        .render_list(
            ScreenGeometry::local_default(),
            &view,
            &rows,
            0,
            0,
            false,
            false,
            "",
            None,
            None,
        )
        .expect("render list");

    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(2).contains("Unit"));
    assert!(buffer.plain_line(2).contains("Points"));
    assert!(buffer.plain_line(2).contains("Queue"));
    assert!(!buffer.plain_line(2).contains("Dock"));
    assert!(buffer.plain_line(4).contains("Destroyers"));
    assert!(buffer.plain_line(4).contains("2"));
    assert!(buffer.plain_line(5).contains("Armies"));
    assert!(!buffer.plain_line(5).contains("N/A"));
    let command_row = (0..25)
        .find(|&row| buffer.plain_line(row).contains("COMMAND <- ? D <Q> ->"))
        .expect("build list command row should render");
    let _ = command_row;
    assert!(!(0..25).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 50 points.")
    }));
}

#[test]
fn build_list_confirmation_renders_delete_question_below_command_row() {
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
        treasury_left: 50,
        points_left: 40,
        building_count: 2,
        docked_count: 0,
    };

    let rows = vec![PlanetBuildListRow {
        kind: ProductionItemKind::Destroyer,
        unit_label: "Destroyers".to_string(),
        points: 5,
        queue_qty: 2,
    }];

    let buffer = screen
        .render_list(
            ScreenGeometry::local_default(),
            &view,
            &rows,
            0,
            0,
            true,
            false,
            "",
            None,
            Some(1),
        )
        .expect("render confirming build list");

    let command_row = (0..25)
        .find(|&row| {
            buffer
                .plain_line(row)
                .contains("COMMAND <- Delete 1 Destroyer? Y/[N] ->")
        })
        .expect("build list confirm prompt should render");
    let _ = command_row;
    assert!(!(0..25).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 50 points.")
    }));
    assert!(!(0..25).any(|row| {
        buffer
            .plain_line(row)
            .contains("Notice: Delete 1 Destroyer?")
    }));
}

#[test]
fn empty_build_list_keeps_table_frame_and_shows_notice_below_command_row() {
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
        committed_points: 0,
        budget: 50,
        treasury_left: 50,
        points_left: 50,
        building_count: 0,
        docked_count: 0,
    };

    let buffer = screen
        .render_list(
            ScreenGeometry::local_default(),
            &view,
            &[],
            0,
            0,
            false,
            false,
            "",
            None,
            None,
        )
        .expect("render empty build list");

    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(2).contains("Unit"));
    assert!(buffer.plain_line(3).contains("├"));
    assert!(buffer.plain_line(4).contains("└"));
    let command_row = (0..25)
        .find(|&row| {
            buffer
                .plain_line(row)
                .contains("COMMAND <- No build orders are queued.")
        })
        .expect("build list command row should render");
    let _ = command_row;
}

#[test]
fn build_list_deletion_requires_d() {
    let screen = PlanetBuildScreen::new();
    for code in [KeyCode::Enter, KeyCode::Delete] {
        assert_eq!(
            screen.handle_list_key(KeyEvent::new(code, KeyModifiers::NONE), false, false),
            Action::Noop
        );
    }
    for code in [KeyCode::Char('d'), KeyCode::Char('D')] {
        assert_eq!(
            screen.handle_list_key(KeyEvent::new(code, KeyModifiers::NONE), false, false),
            Action::Planet(PlanetAction::DeleteBuildSlotRequest)
        );
    }
    assert_eq!(
        screen.handle_list_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            false,
            true
        ),
        Action::Planet(PlanetAction::SubmitDeleteBuildQty)
    );
}

#[test]
fn build_list_delete_prompts_keep_table_width_stable() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            coords: [21, 21],
            planet_name: "Pillar".to_string(),
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
        treasury_left: 50,
        points_left: 40,
        building_count: 2,
        docked_count: 0,
    };
    let rows = vec![PlanetBuildListRow {
        kind: ProductionItemKind::Destroyer,
        unit_label: "Destroyers".to_string(),
        points: 5,
        queue_qty: 2,
    }];

    let buffer = screen
        .render_list(
            ScreenGeometry::local_default(),
            &view,
            &rows,
            0,
            0,
            false,
            true,
            "",
            None,
            None,
        )
        .expect("render build list delete quantity prompt");

    assert!((0..25).any(|row| { buffer.plain_line(row).contains("Delete <A> or 1-2 <Q> ->") }));
    for (confirming, editing, input) in [(false, false, ""), (false, true, "2"), (true, false, "")]
    {
        let other = screen
            .render_list(
                ScreenGeometry::for_door(Some(24)),
                &view,
                &rows,
                0,
                0,
                confirming,
                editing,
                input,
                None,
                Some(2),
            )
            .expect("render queue deletion state");
        for line in 1..=5 {
            assert_eq!(buffer.plain_line(line), other.plain_line(line));
        }
    }
}

#[test]
fn build_change_renders_pp_and_spent_columns() {
    let mut screen = PlanetBuildScreen::new();
    let rows = vec![PlanetBuildChangeRow {
        planet_name: "Not Named Yet".to_string(),
        coords: [6, 5],
        present_production: 100,
        potential_production: 100,
        budget: 50,
        committed_points: 20,
    }];

    let buffer = screen
        .render_change(ScreenGeometry::local_default(), &rows, 0, 0)
        .expect("render change");

    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(2).contains("Planet Name"));
    assert!(buffer.plain_line(2).contains("Location"));
    assert!(buffer.plain_line(2).contains("Production"));
    assert!(buffer.plain_line(2).contains("PP"));
    assert!(buffer.plain_line(2).contains("Spent"));
    assert!(buffer.plain_line(4).contains("50"));
    assert!(buffer.plain_line(4).contains("20"));
}

#[test]
fn build_change_24_row_door_keeps_command_row_off_table_bottom() {
    let mut screen = PlanetBuildScreen::new();
    let geometry = ScreenGeometry::for_door(Some(24));
    let rows = (0..32)
        .map(|idx| PlanetBuildChangeRow {
            planet_name: format!("Planet {idx:02}"),
            coords: [idx as u8, idx as u8],
            present_production: 100,
            potential_production: 100,
            budget: 50,
            committed_points: 20,
        })
        .collect::<Vec<_>>();

    let buffer = screen
        .render_change(geometry, &rows, 0, 0)
        .expect("render 24-row change");

    assert_eq!(buffer.height(), 24);
    assert_eq!(buffer.plain_line(22).chars().next(), Some('└'));
    assert!(buffer.plain_line(23).contains("COMMAND <- ? <Q> ->"));
}

#[test]
fn build_specify_table_is_centered_and_footer_tracks_table_inset() {
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
        committed_points: 0,
        budget: 50,
        treasury_left: 50,
        points_left: 50,
        building_count: 0,
        docked_count: 0,
    };

    let buffer = screen
        .render_specify(&view, &[], "", None, None)
        .expect("render specify build orders");

    let half_columns = [
        TableColumn::left("NO.", 4),
        TableColumn::left("UNIT TYPE", 19),
        TableColumn::right("COST", 4),
        TableColumn::right("QTY.", 5),
    ];
    let combined_columns = half_columns
        .iter()
        .chain(half_columns.iter())
        .copied()
        .collect::<Vec<_>>();
    let expected_table_col = (buffer.width() - table_render_width(&combined_columns)) / 2;

    let title_col = buffer
        .plain_line(0)
        .find("SPECIFY BUILD ORDERS:")
        .expect("title col");
    let border_col = buffer.plain_line(1).find('┌').expect("table col");
    let points_left = "TREASURY: 50  BUDGET: 50";
    let points_left_col = buffer
        .plain_line(0)
        .find(points_left)
        .expect("points-left label col");
    assert_eq!(border_col, expected_table_col);
    assert_eq!(title_col, border_col + 1);
    assert_eq!(
        points_left_col,
        border_col + table_render_width(&combined_columns) - TABLE_TEXT_INSET - points_left.len()
    );

    let command_row = (0..buffer.height())
        .find(|&row| {
            buffer
                .plain_line(row)
                .contains("COMMAND <- Unit number or 0 if done")
        })
        .expect("specify command row");
    assert_eq!(
        buffer
            .plain_line(command_row)
            .find("COMMAND")
            .expect("command col"),
        border_col + 1
    );
}

#[test]
fn build_specify_title_shows_current_points_left_after_partial_commit() {
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
        committed_points: 20,
        budget: 50,
        treasury_left: 50,
        points_left: 30,
        building_count: 0,
        docked_count: 0,
    };

    let buffer = screen
        .render_specify(&view, &[], "", None, None)
        .expect("render specify build orders");

    assert!(buffer.plain_line(0).contains("BUDGET: 30"));
}

#[test]
fn build_quantity_prompt_keeps_points_left_visible_in_title_row() {
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
        committed_points: 20,
        budget: 50,
        treasury_left: 50,
        points_left: 30,
        building_count: 0,
        docked_count: 0,
    };

    let unit = BuildUnitSpec {
        number: 1,
        kind: ProductionItemKind::Destroyer,
        label: "Destroyers",
        singular_label: "destroyers",
        cost: 5,
    };
    let buffer = screen
        .render_quantity_prompt(&view, &[], unit, 6, "", None)
        .expect("render quantity prompt");

    assert!(buffer.plain_line(0).contains("BUDGET: 30"));
}

#[test]
fn build_specify_qty_column_fits_large_queued_counts_in_the_same_table() {
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
        committed_points: 0,
        budget: 50,
        treasury_left: 50,
        points_left: 50,
        building_count: 0,
        docked_count: 0,
    };
    let order = |kind, units: u32| PlanetBuildOrder {
        kind,
        points_remaining: units * kind.build_cost().expect("known kind"),
    };

    let empty = screen
        .render_specify(&view, &[], "", None, None)
        .expect("render empty specify table");
    // Queued counts of 3, 120, 1,000, 32,767, and 100,000: the QTY. column
    // is five wide, so brackets give way to bare digits and then to ">99k".
    let orders = [
        order(ProductionItemKind::Destroyer, 3),
        order(ProductionItemKind::Cruiser, 120),
        order(ProductionItemKind::Battleship, 1_000),
        order(ProductionItemKind::Army, 32_767),
        order(ProductionItemKind::Transport, 100_000),
    ];
    let loaded = screen
        .render_specify(&view, &orders, "", None, None)
        .expect("render loaded specify table");

    // The frame, the rules between rows, and the row count do not change.
    for row in 0..empty.height() {
        let frame = |text: String| {
            text.chars()
                .map(|ch| {
                    if "┌┬┐├┼┤└┴┘─│".contains(ch) {
                        ch
                    } else {
                        ' '
                    }
                })
                .collect::<String>()
        };
        assert_eq!(
            frame(loaded.plain_line(row)),
            frame(empty.plain_line(row)),
            "row {row}"
        );
    }

    let find_row = |label: &str| {
        (0..loaded.height())
            .map(|row| loaded.plain_line(row))
            .find(|line| line.contains(label))
            .unwrap_or_else(|| panic!("row with {label}"))
    };
    assert!(find_row("Destroyers").contains("(3)"));
    assert!(find_row("Cruisers").contains("(120)"));
    assert!(find_row("Battleships").contains("1000"));
    assert!(!find_row("Battleships").contains("(1000"));
    assert!(find_row("Armies").contains("32767"));
    assert!(find_row("Troop transports").contains(">99k"));
}
