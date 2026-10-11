//! A broadcast report row (viewer `0`) reaches every empire. Deleting it from
//! one empire's reviews must leave it with every other empire, in their
//! reviews and in their classic RESULTS.DAT chains.

use crate::support::*;
use nc_data::{ReportBlockRow, classic_results_header_viewers, layout_classic_results};
use nc_game::reports::ReportsPreview;

const BROADCAST: &str = "Broadcast outlook";
const EMPIRE_ONE: &str = "Empire one report";
const EMPIRE_TWO: &str = "Empire two report";

fn report_row(viewer_empire_id: u8, block_index: usize, text: &str) -> ReportBlockRow {
    ReportBlockRow {
        viewer_empire_id,
        block_index,
        decoded_text: text.to_string(),
        raw_bytes: Some(classic_chunked_report_bytes(text)),
        recipient_deleted: false,
    }
}

/// A campaign where empires 1 and 2 are joined humans and empire 1 has
/// pending results.
fn two_empire_game(rows: Vec<ReportBlockRow>) -> TempGameDir {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state
        .game_data
        .join_player(2, "Second Dominion")
        .expect("join empire 2");
    state.report_block_rows = rows;
    state.queued_mail.clear();
    state.game_data.players[0].set_classic_messages_pending_flag_raw(0);
    state.game_data.players[0].set_classic_reports_pending_flag_raw(1);
    save_runtime_state(&fixture_dir, &state);
    fixture_dir
}

fn load_empire_one(fixture_dir: &Path) -> App {
    App::load(AppConfig {
        game_dir: fixture_dir.to_path_buf(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load")
}

fn visible_results(state: &CampaignRuntimeState, viewer_empire_id: u8) -> Vec<String> {
    ReportsPreview::from_block_rows(
        &state.game_data,
        viewer_empire_id,
        &state.report_block_rows,
        &state.queued_mail,
    )
    .results_lines
}

fn mentions(lines: &[String], text: &str) -> bool {
    lines.iter().any(|line| line.contains(text))
}

/// Empire 1 no longer sees the broadcast report; empire 2 still does, and
/// the classic export puts it in empire 2's chain only.
fn assert_broadcast_kept_only_for_empire_two(fixture_dir: &Path) {
    let state = latest_runtime_state(fixture_dir);
    let empire_one = visible_results(&state, 1);
    assert!(!mentions(&empire_one, BROADCAST), "{empire_one:?}");
    let empire_two = visible_results(&state, 2);
    assert!(mentions(&empire_two, BROADCAST), "{empire_two:?}");
    assert!(mentions(&empire_two, EMPIRE_TWO), "{empire_two:?}");

    let mut players = state.game_data.players.clone();
    let layout = layout_classic_results(&players, &state.report_block_rows);
    assert_eq!(layout.chain_for(1), None);
    assert!(layout.chain_for(2).is_some());
    layout.apply_to_players(&mut players);
    let header_viewers = classic_results_header_viewers(&players, &layout.bytes);
    assert_eq!(
        header_viewers.values().copied().collect::<Vec<_>>(),
        vec![2, 2],
        "empire 2's chain holds the broadcast and its own report"
    );
    assert!(
        layout
            .bytes
            .windows(BROADCAST.len())
            .any(|window| window == BROADCAST.as_bytes()),
        "broadcast report missing from the export"
    );
}

#[test]
fn deleting_all_reviewables_keeps_broadcast_report_for_other_empires() {
    let fixture_dir = two_empire_game(vec![
        report_row(0, 0, BROADCAST),
        report_row(1, 0, EMPIRE_ONE),
        report_row(2, 0, EMPIRE_TWO),
    ]);
    let mut app = load_empire_one(&fixture_dir);
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteReviewables)
        ),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    assert!(visible_results(&state, 1).is_empty());
    assert_broadcast_kept_only_for_empire_two(&fixture_dir);
}

#[test]
fn deleting_broadcast_report_from_inbox_keeps_it_for_other_empires() {
    let fixture_dir = two_empire_game(vec![
        report_row(0, 0, BROADCAST),
        report_row(2, 0, EMPIRE_TWO),
    ]);
    let mut app = load_empire_one(&fixture_dir);
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenInboxDeleteConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteInboxItem)
        ),
        AppOutcome::Continue
    );

    assert_broadcast_kept_only_for_empire_two(&fixture_dir);
}

#[test]
fn deleting_broadcast_report_during_startup_review_keeps_it_for_other_empires() {
    let fixture_dir = two_empire_game(vec![
        report_row(0, 0, BROADCAST),
        report_row(2, 0, EMPIRE_TWO),
    ]);
    let mut app = load_empire_one(&fixture_dir);

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );
    // ViewPrompt -> ItemBody, then accept the default delete at its end.
    app.advance_startup();
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );

    assert_broadcast_kept_only_for_empire_two(&fixture_dir);
}
