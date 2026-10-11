//! Crippled ships in the BBS door (t066, t093). Compositions use the ship
//! notation: `CA12!3` is twelve cruisers, three of them crippled. The fleet
//! list shows it in the Ships cell and, when a damaged fleet's composition
//! must be cut, ends the cell in `+!` so the damage stays visible; the review
//! screen repeats the notation and adds a legend in the row below it. The
//! tables keep their columns and rows, and there is no recovery-time column.

use nc_game::screen::{
    FleetListFilter, FleetListSort, FleetReviewScreen, FleetRow, PlayfieldBuffer, ScreenGeometry,
    SortDirection,
};

fn row(ships: &str, damaged_ships: u32) -> FleetRow {
    FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 7,
        coords: [16, 13],
        target_coords: [19, 13],
        order_code: 1,
        current_speed: 3,
        max_speed: 3,
        eta_label: "1".to_string(),
        list_eta_label: "1".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Move fleet to Sector (19,13)".to_string(),
        table_ships_label: ships.to_string(),
        damaged_ships,
        join_host_fleet_number: None,
    }
}

fn list(row: FleetRow) -> PlayfieldBuffer {
    nc_game::screen::FleetListScreen::new()
        .render(
            ScreenGeometry::local_default(),
            &[row],
            FleetListSort::Id,
            SortDirection::Desc,
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
        .expect("fleet list renders")
}

fn review(row: &FleetRow) -> PlayfieldBuffer {
    FleetReviewScreen::new()
        .render(row, 0, 1, false)
        .expect("fleet review renders")
}

fn lines(buffer: &PlayfieldBuffer) -> Vec<String> {
    (0..buffer.height()).map(|y| buffer.plain_line(y)).collect()
}

#[test]
fn the_list_shows_the_notation_and_keeps_the_table_shape() {
    let damaged = list(row("CA12!3 TT*2", 3));
    assert!(damaged.plain_line(4).contains("CA12!3 TT*2"));
    let healthy = list(row("CA12 TT*2", 0));
    assert!(healthy.plain_line(4).contains("CA12 TT*2"));
    // The table keeps its shape: the same rows, line widths, and header.
    assert_eq!(damaged.width(), healthy.width());
    assert_eq!(damaged.height(), healthy.height());
    for (a, b) in lines(&damaged).iter().zip(lines(&healthy)) {
        assert_eq!(a.chars().count(), b.chars().count());
    }
    assert_eq!(damaged.plain_line(2), healthy.plain_line(2));
}

#[test]
fn a_damaged_composition_that_has_to_be_cut_ends_in_a_lone_bang() {
    // The column is 18 wide at 80x25, so this one does not fit.
    let ships = "SC1 BB2!1 CA12!3 DD8 TT*4!1 TT5 ETAC2";
    let cell = list(row(ships, 5)).plain_line(4);
    assert!(cell.contains("SC1 BB2!1 +!"), "{cell}");
    assert!(!cell.contains("CA12"), "{cell}");
    // The same composition with no damage is cut the way it always was.
    let healthy = list(row("SC1 BB2 CA12 DD8 TT*4 TT5 ETAC2", 0)).plain_line(4);
    assert!(healthy.contains("SC1 BB2 CA12 DD8 +"), "{healthy}");
    assert!(!healthy.contains('!'), "{healthy}");
    // A fleet whose damage is in a class the cut drops still shows it.
    let hidden = list(row("SC1 BB2 CA12 DD8 TT*4 TT5 ETAC2!1", 1)).plain_line(4);
    assert!(hidden.contains("+!"), "{hidden}");
    assert!(!hidden.contains("ETAC2"), "{hidden}");
}

#[test]
fn a_cut_that_fits_nothing_still_shows_the_marker() {
    let cell = list(row("CA12345678901234567!3", 3)).plain_line(4);
    assert!(cell.contains("+!"), "{cell}");
}

#[test]
fn the_review_screen_shows_the_notation_and_a_legend_in_a_free_row() {
    let plain = review(&row("BB3 DD3", 0));
    let damaged = review(&row("BB3!1 DD3!2", 3));
    assert!(damaged.plain_line(9).contains("Composition: BB3!1 DD3!2"));
    assert!(
        damaged
            .plain_line(10)
            .contains("!n = crippled ships included in total; TT* = loaded transports")
    );
    assert_eq!(plain.plain_line(10).trim(), "");
    assert!(plain.plain_line(9).contains("Composition: BB3 DD3"));
    // Nothing else moves: the prompt is on the row it was, every other line
    // is the same, and the widths are the same.
    let (plain_lines, damaged_lines) = (lines(&plain), lines(&damaged));
    for (y, (a, b)) in plain_lines.iter().zip(&damaged_lines).enumerate() {
        if y != 9 && y != 10 {
            assert_eq!(a, b, "row {y}");
        }
    }
    assert_eq!(plain.width(), damaged.width());
    assert!(!damaged_lines.join("\n").to_lowercase().contains("turn"));
}
