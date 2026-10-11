use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow};
use nc_game::domains::messaging::state::{InboxFocus, InboxPromptMode, InboxTypeFilter};
use nc_game::reports::{InboxDisplayItem, InboxItem, InboxItemType, runtime_inbox_items};
use nc_game::screen::{CommandMenu, ReportsScreen, ScreenGeometry};
use nc_game::theme::classic;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_game_data() -> CoreGameData {
    load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5")).expect("load fixture")
}

fn visible_mail(sender: u8, recipient: u8, year: u16, subject: &str) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id: sender,
        recipient_empire_id: recipient,
        year,
        subject: subject.to_string(),
        body: String::new(),
        recipient_deleted: false,
    }
}

fn report_row(viewer_empire_id: u8, block_index: usize, decoded_text: &str) -> ReportBlockRow {
    ReportBlockRow {
        viewer_empire_id,
        block_index,
        decoded_text: decoded_text.to_string(),
        raw_bytes: None,
        recipient_deleted: false,
    }
}

fn with_display_ids(items: Vec<InboxItem>) -> Vec<InboxDisplayItem> {
    items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| InboxDisplayItem {
            display_id: idx + 1,
            item,
        })
        .collect()
}

#[test]
fn report_subjects_are_event_first_without_stardate_suffix() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[
            report_row(0, 0, "Stardate: 03/3003\nFleet contact report"),
            report_row(0, 1, "Stardate: 11/3003\nBombardment complete."),
            report_row(0, 2, "Stardate: 02/3003\nWe were attacked by enemy forces."),
            report_row(0, 3, "Starbase 1 is moving to (08,12)."),
        ],
        &[],
    );

    let subjects = items
        .into_iter()
        .map(|item| item.subject)
        .collect::<Vec<_>>();
    assert!(subjects.contains(&"Scout".to_string()));
    assert!(subjects.contains(&"Bombard".to_string()));
    assert!(subjects.contains(&"Combat".to_string()));
    assert!(subjects.contains(&"Starbase".to_string()));
}

#[test]
fn runtime_inbox_items_sort_newest_to_oldest_by_stardate() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[
            report_row(0, 0, "Stardate: 01/3003\nPatrol mission update."),
            report_row(0, 0, "Stardate: 02/3003\nFleet contact report"),
            report_row(0, 0, "Stardate: 03/3003\nGeneral command advisory."),
            report_row(0, 1, "Stardate: 52/3002\nBombardment complete."),
        ],
        &[visible_mail(2, 1, 3003, "Diplomatic")],
    );

    let labels = items
        .iter()
        .map(|item| format!("{} {}", item.stardate_label(), item.subject))
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "03/3003 General".to_string(),
            "02/3003 Scout".to_string(),
            "01/3003 Patrol".to_string(),
            "00/3003 Diplomatic".to_string(),
            "52/3002 Bombard".to_string(),
        ]
    );
}

#[test]
fn runtime_inbox_items_only_include_reports_visible_to_active_viewer() {
    let game_data = fixture_game_data();
    let rows = vec![
        report_row(0, 0, "Stardate: 01/3003\nBroadcast advisory."),
        report_row(1, 0, "Stardate: 02/3003\nPlayer 1 only report."),
        report_row(3, 0, "Stardate: 03/3003\nPlayer 3 only report."),
    ];

    let player_1_items = runtime_inbox_items(&game_data, 1, &rows, &[]);
    let player_3_items = runtime_inbox_items(&game_data, 3, &rows, &[]);

    let player_1_bodies = player_1_items
        .iter()
        .map(|item| item.body_lines.join(" "))
        .collect::<Vec<_>>();
    assert_eq!(player_1_bodies.len(), 2);
    assert!(
        player_1_bodies
            .iter()
            .any(|body| body.contains("Broadcast advisory."))
    );
    assert!(
        player_1_bodies
            .iter()
            .any(|body| body.contains("Player 1 only report."))
    );
    assert!(
        !player_1_bodies
            .iter()
            .any(|body| body.contains("Player 3 only report."))
    );

    let player_3_bodies = player_3_items
        .iter()
        .map(|item| item.body_lines.join(" "))
        .collect::<Vec<_>>();
    assert_eq!(player_3_bodies.len(), 2);
    assert!(
        player_3_bodies
            .iter()
            .any(|body| body.contains("Broadcast advisory."))
    );
    assert!(
        player_3_bodies
            .iter()
            .any(|body| body.contains("Player 3 only report."))
    );
    assert!(
        !player_3_bodies
            .iter()
            .any(|body| body.contains("Player 1 only report."))
    );
}

#[test]
fn message_rows_display_zero_week_stardate() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[],
        &[visible_mail(2, 1, 3003, "Diplomatic")],
    );
    let message = items
        .iter()
        .find(|item| item.item_type == InboxItemType::Message)
        .expect("message row");
    assert_eq!(message.stardate_label(), "00/3003");
}

#[test]
fn reports_screen_themes_status_labels_and_highlights_focused_pane_border() {
    let game_data = fixture_game_data();
    let items = with_display_ids(runtime_inbox_items(
        &game_data,
        1,
        &[report_row(0, 0, "Stardate: 03/3003\nFleet contact report")],
        &[visible_mail(2, 1, 3003, "Diplomatic")],
    ));
    let mut screen = ReportsScreen::new();

    let inbox_buffer = screen
        .render_inbox(
            ScreenGeometry::local_default(),
            CommandMenu::General,
            &items,
            InboxTypeFilter::All,
            None,
            0,
            0,
            0,
            InboxFocus::Inbox,
            "",
            "",
            InboxPromptMode::Normal,
            None,
            3003,
        )
        .expect("inbox render");
    let status_line = inbox_buffer.plain_line(0);
    let type_col = status_line.find("Type:").expect("type label");
    let year_col = status_line.find("Year:").expect("year label");
    let focus_col = status_line.find("Focus:").expect("focus label");
    assert_eq!(type_col, 1);
    assert_eq!(
        inbox_buffer.row(0)[type_col].style,
        classic::status_label_style()
    );
    assert_eq!(
        inbox_buffer.row(0)[year_col].style,
        classic::status_label_style()
    );
    assert_eq!(
        inbox_buffer.row(0)[focus_col].style,
        classic::status_label_style()
    );
    assert_eq!(inbox_buffer.row(1)[0].style, classic::notice_style());
    assert_eq!(inbox_buffer.row(3)[0].style, classic::notice_style());
    let separator_cols = inbox_buffer
        .row(2)
        .iter()
        .enumerate()
        .filter_map(|(idx, cell)| (cell.ch == '│').then_some(idx))
        .collect::<Vec<_>>();
    assert!(separator_cols.len() >= 3);
    assert_eq!(
        inbox_buffer.row(2)[separator_cols[1]].style,
        classic::notice_style()
    );
    assert!(inbox_buffer.plain_line(2).contains("Stardate"));
    assert!(inbox_buffer.plain_line(24).contains("<TAB>"));
    assert!(
        inbox_buffer
            .plain_line(24)
            .contains("COMMAND <- ? M R A Y D")
    );
    assert!(inbox_buffer.plain_line(24).contains("<Q> [01] ->"));
    assert!(inbox_buffer.plain_line(24).starts_with(" COMMAND <-"));
    assert!(!inbox_buffer.plain_line(7).contains("PREVIEW:"));

    let preview_buffer = screen
        .render_inbox(
            ScreenGeometry::local_default(),
            CommandMenu::General,
            &items,
            InboxTypeFilter::All,
            None,
            0,
            0,
            0,
            InboxFocus::Preview,
            "",
            "",
            InboxPromptMode::Normal,
            None,
            3003,
        )
        .expect("preview render");
    let preview_top_row = 1
        + 4
        + items
            .len()
            .min(nc_game::domains::messaging::state::INBOX_VISIBLE_ROWS);
    assert_eq!(
        preview_buffer.row(1)[0].style,
        classic::table_chrome_style()
    );
    assert_eq!(
        preview_buffer.row(preview_top_row)[0].style,
        classic::notice_style()
    );
}

#[test]
fn reports_screen_pads_year_and_delete_prompts_one_column_right() {
    let game_data = fixture_game_data();
    let items = with_display_ids(runtime_inbox_items(
        &game_data,
        1,
        &[report_row(0, 0, "Stardate: 03/3003\nFleet contact report")],
        &[visible_mail(2, 1, 3003, "Diplomatic")],
    ));
    let mut screen = ReportsScreen::new();

    let year_buffer = screen
        .render_inbox(
            ScreenGeometry::local_default(),
            CommandMenu::General,
            &items,
            InboxTypeFilter::All,
            None,
            0,
            0,
            0,
            InboxFocus::Inbox,
            "",
            "",
            InboxPromptMode::YearInput,
            None,
            3003,
        )
        .expect("year prompt render");
    assert!(
        year_buffer
            .plain_line(24)
            .starts_with(" COMMAND <- Year [3003] <Q> ->")
    );

    let confirm_buffer = screen
        .render_inbox(
            ScreenGeometry::local_default(),
            CommandMenu::General,
            &items,
            InboxTypeFilter::All,
            None,
            0,
            0,
            0,
            InboxFocus::Inbox,
            "",
            "",
            InboxPromptMode::DeleteConfirm,
            None,
            3003,
        )
        .expect("delete confirm render");
    assert!(
        confirm_buffer
            .plain_line(24)
            .starts_with(" COMMAND <- Delete item 01? [Y]/N ->")
    );
}

#[test]
fn reports_screen_uses_themed_scrollbar_thumb_style() {
    let game_data = fixture_game_data();
    let items = (0..12)
        .map(|idx| InboxDisplayItem {
            display_id: idx + 1,
            item: InboxItem {
                source: nc_game::reports::InboxItemSource::QueuedMail(idx),
                item_type: InboxItemType::Message,
                year: 3003,
                week: None,
                subject: format!("Message {}", idx + 1),
                body_lines: vec![format!("Body {}", idx + 1)],
                report_bucket: None,
            },
        })
        .collect::<Vec<_>>();
    let mut screen = ReportsScreen::new();
    let buffer = screen
        .render_inbox(
            ScreenGeometry::local_default(),
            CommandMenu::General,
            &items,
            InboxTypeFilter::All,
            None,
            0,
            0,
            0,
            InboxFocus::Inbox,
            "",
            "",
            InboxPromptMode::Normal,
            None,
            game_data.conquest.game_year,
        )
        .expect("inbox render");
    let thumb = (0..25)
        .find_map(|row| {
            buffer
                .row(row)
                .iter()
                .enumerate()
                .find_map(|(col, cell)| (cell.ch == '#').then_some((row, col, cell.style)))
        })
        .expect("scrollbar thumb");
    assert_eq!(thumb.2, classic::scrollbar_thumb_style());
}
