use nc_data::{ReportBlockRow, carry_unread_report_blocks};

fn row(viewer_empire_id: u8, block_index: usize, text: &str, deleted: bool) -> ReportBlockRow {
    ReportBlockRow {
        viewer_empire_id,
        block_index,
        decoded_text: text.to_string(),
        raw_bytes: None,
        recipient_deleted: deleted,
    }
}

fn blocks(rows: &[ReportBlockRow], viewer_empire_id: u8) -> Vec<(usize, &str)> {
    rows.iter()
        .filter(|row| row.viewer_empire_id == viewer_empire_id)
        .map(|row| (row.block_index, row.decoded_text.as_str()))
        .collect()
}

/// Last year's unread reports stay, ahead of this year's; what the player
/// deleted is gone. Block indexes run from 0 again for each viewer.
#[test]
fn unread_rows_lead_the_new_year_and_deleted_rows_are_dropped() {
    let previous = vec![
        row(0, 0, "Broadcast", false),
        row(1, 0, "One read", true),
        row(1, 1, "One unread", false),
        row(2, 0, "Two unread", false),
        row(2, 1, "Two read", true),
    ];
    let fresh = vec![row(1, 0, "One new", false), row(2, 0, "Two new", false)];

    let rows = carry_unread_report_blocks(&previous, fresh);

    assert_eq!(blocks(&rows, 0), vec![(0, "Broadcast")]);
    assert_eq!(blocks(&rows, 1), vec![(0, "One unread"), (1, "One new")]);
    assert_eq!(blocks(&rows, 2), vec![(0, "Two unread"), (1, "Two new")]);
    assert!(rows.iter().all(|row| !row.recipient_deleted));
}

#[test]
fn a_first_year_has_nothing_to_carry() {
    let fresh = vec![row(1, 0, "One new", false), row(1, 1, "One next", false)];
    let rows = carry_unread_report_blocks(&[], fresh.clone());
    assert_eq!(rows, fresh);
}
