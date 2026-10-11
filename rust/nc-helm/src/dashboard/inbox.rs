//! Dash-local inbox adapters over the shared nc-data runtime projection.

use nc_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow, runtime_inbox_items};

use crate::dashboard::app::state::InboxFilter;

pub use nc_data::{
    InboxItem as DashInboxItem, InboxItemSource as DashInboxItemSource,
    InboxItemType as DashInboxItemType, ReportSummaryBucket,
};

pub fn project_inbox_items(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    report_blocks: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
) -> Vec<DashInboxItem> {
    runtime_inbox_items(game_data, viewer_empire_id, report_blocks, queued_mail)
}

/// The year the "current year only" filter keeps. Maintenance delivers mail
/// and writes reports under the year before the one it advances to, so the
/// newest delivered items carry `current_year - 1`. The viewer's own queued
/// (outgoing) messages carry this turn's year.
fn latest_delivered_year(item_type: DashInboxItemType, current_year: u16) -> u16 {
    match item_type {
        DashInboxItemType::Outgoing => current_year,
        DashInboxItemType::Message | DashInboxItemType::Report => current_year.saturating_sub(1),
    }
}

pub fn matches_filter(
    item: &DashInboxItem,
    filter: InboxFilter,
    current_year_only: bool,
    current_year: u16,
) -> bool {
    if current_year_only && item.year != latest_delivered_year(item.item_type, current_year) {
        return false;
    }
    match filter {
        InboxFilter::All => true,
        InboxFilter::Reports => matches!(item.item_type, DashInboxItemType::Report),
        InboxFilter::Messages => matches!(item.item_type, DashInboxItemType::Message),
    }
}
