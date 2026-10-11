pub use nc_data::{
    InboxItem, InboxItemSource, InboxItemType, ReportsPreview, ReviewBlock,
    has_visible_runtime_messages, has_visible_runtime_reports, runtime_inbox_items,
    runtime_inbox_preview_lines, runtime_outgoing_inbox_items, runtime_outgoing_message_lines,
    runtime_sent_inbox_items, wrap_review_text_preserving_spacing,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxDisplayItem {
    pub display_id: usize,
    pub item: InboxItem,
}
