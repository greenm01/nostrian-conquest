use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBlock {
    pub lines: Vec<String>,
    pub raw_chunked_bytes: Option<Vec<u8>>,
    pub runtime_report_index: Option<usize>,
    pub runtime_mail_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportsPreview {
    pub results_lines: Vec<String>,
    pub message_lines: Vec<String>,
    pub result_blocks: Vec<ReviewBlock>,
    pub message_blocks: Vec<ReviewBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxItemType {
    Message,
    Report,
    /// The viewer's own message, queued this turn and not yet delivered.
    Outgoing,
    /// A delivered copy retained by its sender.
    Sent,
}

impl InboxItemType {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Message => "M",
            Self::Report => "R",
            Self::Outgoing => "OUT",
            Self::Sent => "SENT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InboxItemSource {
    QueuedMail(usize),
    ReportBlock(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportSummaryBucket {
    Combat,
    Intel,
    Ops,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxItem {
    pub source: InboxItemSource,
    pub item_type: InboxItemType,
    pub year: u16,
    pub week: Option<u8>,
    pub subject: String,
    pub correspondent: String,
    pub body_lines: Vec<String>,
    pub report_bucket: Option<ReportSummaryBucket>,
}

impl InboxItem {
    pub fn stardate_label(&self) -> String {
        format!("{:02}/{}", self.stardate_week(), self.year)
    }

    pub fn stardate_week(&self) -> u8 {
        self.week.unwrap_or(0)
    }
}

impl ReportsPreview {
    pub fn from_block_rows(
        game_data: &CoreGameData,
        viewer_empire_id: u8,
        report_blocks: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
    ) -> Self {
        let result_blocks = review_blocks_from_rows(viewer_empire_id, report_blocks);
        let message_blocks = runtime_message_blocks(game_data, viewer_empire_id, queued_mail);
        Self {
            results_lines: flatten_block_lines(&result_blocks),
            message_lines: flatten_block_lines(&message_blocks),
            result_blocks,
            message_blocks,
        }
    }
}

pub fn has_visible_runtime_reports(viewer_empire_id: u8, report_blocks: &[ReportBlockRow]) -> bool {
    report_blocks
        .iter()
        .any(|row| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
}

/// Whether maintenance has delivered mail the viewer has not deleted.
pub fn has_visible_runtime_messages(
    viewer_empire_id: u8,
    queued_mail: &[QueuedPlayerMail],
    current_year: u16,
) -> bool {
    queued_mail
        .iter()
        .any(|mail| mail.is_visible_to_recipient(viewer_empire_id, current_year))
}

pub fn runtime_inbox_items(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    report_blocks: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
) -> Vec<InboxItem> {
    let current_year = game_data.conquest.game_year;
    let mut items = report_blocks
        .iter()
        .enumerate()
        .filter(|(_, row)| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
        .map(|(idx, row)| inbox_item_from_report_row(idx, row, current_year))
        .chain(
            queued_mail
                .iter()
                .enumerate()
                .filter(|(_, mail)| mail.is_visible_to_recipient(viewer_empire_id, current_year))
                .map(|(idx, mail)| inbox_item_from_mail(game_data, idx, mail)),
        )
        .collect::<Vec<_>>();
    items.sort_by_key(inbox_item_sort_key);
    items
}

/// The viewer's own messages queued this turn, newest first. A message leaves
/// this list when maintenance delivers it.
pub fn runtime_outgoing_inbox_items(
    game_data: &CoreGameData,
    sender_empire_id: u8,
    queued_mail: &[QueuedPlayerMail],
) -> Vec<InboxItem> {
    let current_year = game_data.conquest.game_year;
    queued_mail
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, mail)| mail.is_queued_by(sender_empire_id, current_year))
        .map(|(idx, mail)| InboxItem {
            source: InboxItemSource::QueuedMail(idx),
            item_type: InboxItemType::Outgoing,
            year: mail.year,
            week: None,
            subject: inbox_message_subject(mail),
            correspondent: empire_label(game_data, mail.recipient_empire_id),
            body_lines: runtime_outgoing_message_lines(game_data, mail),
            report_bucket: None,
        })
        .collect()
}

/// The sender's visible copies, pending first, then newest delivered first.
pub fn runtime_sent_inbox_items(
    game_data: &CoreGameData,
    sender_empire_id: u8,
    queued_mail: &[QueuedPlayerMail],
) -> Vec<InboxItem> {
    let year = game_data.conquest.game_year;
    let mut items: Vec<_> = queued_mail
        .iter()
        .enumerate()
        .filter(|(_, mail)| mail.is_visible_to_sender(sender_empire_id))
        .map(|(idx, mail)| {
            let delivered = mail.is_delivered(year);
            InboxItem {
                source: InboxItemSource::QueuedMail(idx),
                item_type: if delivered {
                    InboxItemType::Sent
                } else {
                    InboxItemType::Outgoing
                },
                year: if delivered {
                    mail.year.saturating_add(1)
                } else {
                    mail.year
                },
                week: None,
                subject: inbox_message_subject(mail),
                correspondent: empire_label(game_data, mail.recipient_empire_id),
                body_lines: runtime_outgoing_message_lines(game_data, mail),
                report_bucket: None,
            }
        })
        .collect();
    items.sort_by_key(|item| {
        std::cmp::Reverse((
            item.item_type == InboxItemType::Outgoing,
            item.year,
            item.source,
        ))
    });
    items
}

/// A queued message as its sender reads it: the recipient takes the place
/// of the sender line.
pub fn runtime_outgoing_message_lines(
    game_data: &CoreGameData,
    mail: &QueuedPlayerMail,
) -> Vec<String> {
    message_lines(
        format!(
            "To {} (Empire #{})",
            empire_label(game_data, mail.recipient_empire_id),
            mail.recipient_empire_id
        ),
        mail,
    )
}

pub fn runtime_inbox_preview_lines(lines: &[String], width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut wrapped = Vec::new();
    for line in lines {
        if line.is_empty() {
            wrapped.push(String::new());
        } else {
            wrapped.extend(wrap_review_text_preserving_spacing(line, width));
        }
    }
    wrapped
}

pub fn wrap_review_text_preserving_spacing(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() || width == 0 {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    let mut remaining = text;
    while remaining.len() > width {
        let prefix = &remaining[..width];
        let break_at = prefix.rfind(' ').filter(|idx| *idx > 0);
        match break_at {
            Some(idx) => {
                rows.push(remaining[..idx].to_string());
                remaining = &remaining[idx + 1..];
            }
            None => {
                rows.push(prefix.to_string());
                remaining = &remaining[width..];
            }
        }
    }
    rows.push(remaining.to_string());
    rows
}

fn review_blocks_from_rows(viewer_empire_id: u8, rows: &[ReportBlockRow]) -> Vec<ReviewBlock> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
        .map(|(idx, row)| ReviewBlock {
            lines: row
                .decoded_text
                .split('\n')
                .map(ToOwned::to_owned)
                .collect(),
            raw_chunked_bytes: row.raw_bytes.clone(),
            runtime_report_index: Some(idx),
            runtime_mail_index: None,
        })
        .collect()
}

fn flatten_block_lines(blocks: &[ReviewBlock]) -> Vec<String> {
    blocks
        .iter()
        .flat_map(|block| block.lines.iter().cloned())
        .collect()
}

fn runtime_message_blocks(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    queued_mail: &[QueuedPlayerMail],
) -> Vec<ReviewBlock> {
    let current_year = game_data.conquest.game_year;
    queued_mail
        .iter()
        .enumerate()
        .filter(|(_, mail)| mail.is_visible_to_recipient(viewer_empire_id, current_year))
        .map(|(idx, mail)| ReviewBlock {
            lines: runtime_message_lines(game_data, mail),
            raw_chunked_bytes: None,
            runtime_report_index: None,
            runtime_mail_index: Some(idx),
        })
        .collect()
}

fn runtime_message_lines(game_data: &CoreGameData, mail: &QueuedPlayerMail) -> Vec<String> {
    message_lines(
        format!(
            "From {} (Empire #{})",
            empire_label(game_data, mail.sender_empire_id),
            mail.sender_empire_id
        ),
        mail,
    )
}

fn message_lines(header: String, mail: &QueuedPlayerMail) -> Vec<String> {
    let mut lines = vec![header];
    if !mail.subject.trim().is_empty() {
        lines.push(format!("Subject: {}", mail.subject.trim()));
    }
    let body_lines = decode_text_lines(&mail.body);
    if body_lines.is_empty() {
        lines.push(String::new());
    } else {
        lines.extend(body_lines);
    }
    lines.push("<end of message>".to_string());
    lines
}

fn inbox_item_from_mail(
    game_data: &CoreGameData,
    idx: usize,
    mail: &QueuedPlayerMail,
) -> InboxItem {
    InboxItem {
        source: InboxItemSource::QueuedMail(idx),
        item_type: InboxItemType::Message,
        year: mail.year,
        week: None,
        subject: inbox_message_subject(mail),
        correspondent: empire_label(game_data, mail.sender_empire_id),
        body_lines: runtime_message_lines(game_data, mail),
        report_bucket: None,
    }
}

fn inbox_item_from_report_row(idx: usize, row: &ReportBlockRow, current_year: u16) -> InboxItem {
    let body_lines = decode_text_lines(&row.decoded_text);
    let (week, parsed_year) = parse_stardate_week_year(&row.decoded_text)
        .map(|(week, year)| (Some(week), year))
        .unwrap_or((None, current_year));
    InboxItem {
        source: InboxItemSource::ReportBlock(idx),
        item_type: InboxItemType::Report,
        year: parsed_year,
        week,
        subject: synthesize_report_subject(&body_lines),
        correspondent: String::new(),
        body_lines,
        report_bucket: Some(classify_report_bucket(&row.decoded_text)),
    }
}

fn inbox_message_subject(mail: &QueuedPlayerMail) -> String {
    let subject = mail.subject.trim();
    if subject.is_empty() {
        "<no subject>".to_string()
    } else {
        subject.to_string()
    }
}

fn parse_stardate_week_year(text: &str) -> Option<(u8, u16)> {
    let marker = text.find("Stardate")?;
    let tail = &text[marker + "Stardate".len()..];
    let digit_start = tail.find(|ch: char| ch.is_ascii_digit())?;
    let digits = &tail[digit_start..];
    let slash = digits.find('/')?;
    let week_text = digits[..slash]
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let year_text = digits[slash + 1..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let week = week_text.parse::<u8>().ok()?;
    let year = year_text.parse::<u16>().ok()?;
    Some((week, year))
}

fn synthesize_report_subject(lines: &[String]) -> String {
    classify_report_subject(lines).to_string()
}

fn classify_report_subject(lines: &[String]) -> &'static str {
    let normalized = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    if normalized.is_empty() {
        return "General";
    }
    if normalized.starts_with("game notice") {
        return "Notice";
    }

    if contains_any(&normalized, &["bombard"]) {
        return "Bombard";
    }
    if contains_any(&normalized, &["blitz"]) {
        return "Blitz";
    }
    if contains_any(&normalized, &["invade", "invasion", "assault"]) {
        return "Invade";
    }
    if contains_any(&normalized, &["salvage"]) {
        return "Salvage";
    }
    if contains_any(&normalized, &["coloniz"]) {
        return "Colonize";
    }
    if contains_any(
        &normalized,
        &["merged into", "merged with", "merged at", "absorbed into"],
    ) {
        return "Merge";
    }
    if contains_any(
        &normalized,
        &[
            "contact report",
            "scout report",
            "sighted",
            "detected",
            "sensor contact",
            "contact with",
        ],
    ) {
        return "Scout";
    }
    if contains_any(
        &normalized,
        &[
            "we were attacked",
            "we intercepted",
            "lost all contact",
            "captured by",
            "has been captured",
            "destroyed in combat",
            "engaged in combat",
            "battle at",
        ],
    ) {
        return "Combat";
    }
    if contains_any(&normalized, &["patrol"]) {
        return "Patrol";
    }
    if contains_any(&normalized, &["rendezvous"]) {
        return "Rendez";
    }
    if contains_any(&normalized, &["starbase", "is moving to", "halted at"]) {
        return "Starbase";
    }
    "General"
}

fn classify_report_bucket(text: &str) -> ReportSummaryBucket {
    let normalized = text.to_ascii_lowercase();

    if contains_any(
        &normalized,
        &[
            "bombard",
            "blitz",
            "invade",
            "invasion",
            "assault",
            "we were attacked",
            "we intercepted",
            "lost all contact",
            "captured by",
            "has been captured",
            "destroyed in combat",
            "engaged in combat",
            "battle at",
        ],
    ) {
        return ReportSummaryBucket::Combat;
    }

    if contains_any(
        &normalized,
        &[
            "contact report",
            "scout report",
            "sighted",
            "detected",
            "sensor contact",
            "contact with",
            "patrol",
        ],
    ) {
        return ReportSummaryBucket::Intel;
    }

    ReportSummaryBucket::Ops
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn inbox_source_index(source: InboxItemSource) -> usize {
    match source {
        InboxItemSource::QueuedMail(index) | InboxItemSource::ReportBlock(index) => index,
    }
}

fn inbox_item_type_rank(item_type: InboxItemType) -> u8 {
    match item_type {
        InboxItemType::Report => 1,
        InboxItemType::Message | InboxItemType::Outgoing | InboxItemType::Sent => 0,
    }
}

fn inbox_item_sort_key(item: &InboxItem) -> (u16, u8, u8, usize) {
    (
        item.year,
        item.stardate_week(),
        inbox_item_type_rank(item.item_type),
        inbox_source_index(item.source),
    )
}

fn decode_text_lines(text: &str) -> Vec<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized
        .split('\n')
        .map(str::trim_end)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn empire_label(game_data: &CoreGameData, empire_id: u8) -> String {
    let Some(player) = game_data.players.get(empire_id.saturating_sub(1) as usize) else {
        return format!("Empire #{empire_id}");
    };
    let controlled = player.empire_name.trim().to_string();
    if !controlled.is_empty() {
        return controlled;
    }
    let legacy = player.empire_name.trim().to_string();
    if !legacy.is_empty() {
        return legacy;
    }
    format!("Empire #{empire_id}")
}
