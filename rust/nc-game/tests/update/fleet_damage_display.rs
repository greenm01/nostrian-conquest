//! Crippled ships in the BBS door (t066). The fleet list shows at most a
//! compact "N damaged" in the Ships cell, only where it fits; the review
//! screen lists the crippled ships by class in a row it already had. The
//! tables keep their columns and rows, and there is no recovery-time column.

use nc_game::screen::{
    FleetListFilter, FleetListSort, FleetReviewScreen, FleetRow, PlayfieldBuffer, ScreenGeometry,
    SortDirection,
};

fn row(ships: &str, damaged_ships: u32, damaged_label: &str) -> FleetRow {
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
        composition_label: String::new(),
        table_ships_label: ships.to_string(),
        damaged_ships,
        damaged_label: damaged_label.to_string(),
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
fn the_list_notes_damage_in_the_ships_cell_when_it_fits() {
    let damaged = list(row("CA 2TT*", 3, "3CA"));
    assert!(damaged.plain_line(4).contains("CA 2TT* 3 damaged"));
    let plain = list(row("CA 2TT*", 0, ""));
    assert!(!plain.plain_line(4).contains("damaged"));
    // The table keeps its shape: the same rows and the same line widths.
    assert_eq!(damaged.width(), plain.width());
    assert_eq!(damaged.height(), plain.height());
    for (a, b) in lines(&damaged).iter().zip(lines(&plain)) {
        assert_eq!(a.chars().count(), b.chars().count());
    }
    // No new column: the header is the one it was.
    assert_eq!(damaged.plain_line(2), plain.plain_line(2));
}

#[test]
fn a_note_that_does_not_fit_leaves_the_list_exactly_as_it_was() {
    // "2SC BB 4DD 2TT* ET" fills the Ships column; so would a mixed fleet
    // the column already truncates with a +.
    for ships in ["2SC BB 4DD 2TT* ET", "2SC 4BB 3CA 5DD 2TT* 3TT ET"] {
        assert_eq!(
            lines(&list(row(ships, 4, "2BB 2DD"))),
            lines(&list(row(ships, 0, ""))),
            "{ships}"
        );
    }
}

#[test]
fn the_review_screen_lists_crippled_ships_by_class_in_a_free_row() {
    let plain = review(&row("BB 3DD", 0, ""));
    let damaged = review(&row("BB 3DD", 3, "BB 2DD"));
    assert!(damaged.plain_line(9).contains("Composition: BB 3DD"));
    assert!(damaged.plain_line(10).contains("Damaged: BB 2DD"));
    assert_eq!(plain.plain_line(10).trim(), "");
    // Nothing else moves: the prompt is on the row it was, every other line
    // is the same, and the widths are the same.
    let (plain_lines, damaged_lines) = (lines(&plain), lines(&damaged));
    for (y, (a, b)) in plain_lines.iter().zip(&damaged_lines).enumerate() {
        if y != 10 {
            assert_eq!(a, b, "row {y}");
        }
    }
    assert_eq!(plain.width(), damaged.width());
    assert!(!damaged_lines.join("\n").to_lowercase().contains("turn"));
}
