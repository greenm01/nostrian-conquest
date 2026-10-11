use nc_data::{
    Player, ReportBlockRow, delete_report_block_for_viewer, delete_report_blocks_for_viewer,
    layout_classic_results,
};

/// Empires 1 and 2 are active humans; empire 3 is rogue.
fn players() -> Vec<Player> {
    [0x01, 0x01, 0xff]
        .into_iter()
        .map(|mode| {
            let mut player = Player::default();
            player.set_player_mode_raw(mode);
            player
        })
        .collect()
}

fn report_row(viewer_empire_id: u8, block_index: usize, text: &str) -> ReportBlockRow {
    let mut raw_bytes = vec![0u8; 84];
    raw_bytes[0] = 1;
    raw_bytes[1] = text.len() as u8;
    raw_bytes[2..2 + text.len()].copy_from_slice(text.as_bytes());
    ReportBlockRow {
        viewer_empire_id,
        block_index,
        decoded_text: text.to_string(),
        raw_bytes: Some(raw_bytes),
        recipient_deleted: false,
    }
}

/// One broadcast row whose block index collides with targeted rows.
fn rows() -> Vec<ReportBlockRow> {
    vec![
        report_row(0, 0, "Broadcast"),
        report_row(1, 0, "One first"),
        report_row(1, 1, "One second"),
        report_row(2, 0, "Two first"),
    ]
}

fn visible(rows: &[ReportBlockRow], viewer_empire_id: u8) -> Vec<&str> {
    rows.iter()
        .filter(|row| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
        .map(|row| row.decoded_text.as_str())
        .collect()
}

#[test]
fn deleting_a_targeted_report_leaves_the_broadcast_with_the_same_index() {
    let players = players();
    let mut rows = rows();

    delete_report_block_for_viewer(&mut rows, &players, 1, 1);

    assert_eq!(visible(&rows, 1), ["Broadcast", "One second"]);
    assert_eq!(visible(&rows, 2), ["Broadcast", "Two first"]);
    assert_eq!(rows.len(), 4);
}

#[test]
fn deleting_a_broadcast_report_keeps_it_for_other_active_humans() {
    let players = players();
    let mut rows = rows();

    delete_report_block_for_viewer(&mut rows, &players, 0, 1);

    assert_eq!(visible(&rows, 1), ["One first", "One second"]);
    assert_eq!(visible(&rows, 2), ["Two first", "Broadcast"]);
    assert_eq!(visible(&rows, 3), Vec::<&str>::new());
    let copy = rows.last().unwrap();
    assert_eq!((copy.viewer_empire_id, copy.block_index), (2, 2));
}

#[test]
fn deleting_all_then_by_index_keeps_the_broadcast_in_other_chains() {
    let players = players();
    let mut rows = rows();

    delete_report_blocks_for_viewer(&mut rows, &players, 1);
    assert_eq!(visible(&rows, 1), Vec::<&str>::new());
    assert_eq!(visible(&rows, 2), ["Two first", "Broadcast"]);

    // A second delete by index from empire 1 changes nothing for empire 2.
    delete_report_block_for_viewer(&mut rows, &players, 0, 1);
    assert_eq!(visible(&rows, 2), ["Two first", "Broadcast"]);

    let layout = layout_classic_results(&players, &rows);
    assert_eq!(layout.chain_for(1), None);
    let chain = layout.chain_for(2).expect("empire 2 keeps its chain");
    assert_eq!((chain.first_header, chain.last_header), (1, 2));
    assert_eq!(&layout.bytes[84 + 2..84 + 2 + 9], b"Broadcast");
}
