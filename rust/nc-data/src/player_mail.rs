use std::fs;
use std::path::{Path, PathBuf};

const QUEUE_FILE_NAME: &str = "RUSTMAIL.QUE";
pub const MAX_MESSAGE_SUBJECT_CHARS: usize = 60;
pub const MAX_MESSAGE_BODY_CHARS: usize = 1000;
pub const MAX_QUEUED_MESSAGES_PER_RECIPIENT_PER_YEAR: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPlayerMail {
    pub sender_empire_id: u8,
    pub recipient_empire_id: u8,
    pub year: u16,
    pub subject: String,
    pub body: String,
    /// Hide from the recipient; the stored message remains intact.
    pub recipient_deleted: bool,
    /// Hide from the sender without changing the recipient's copy.
    pub sender_deleted: bool,
    /// Retain the queued message but never deliver it.
    pub cancelled: bool,
}

impl QueuedPlayerMail {
    /// Maintenance delivers mail. A message carries the year it was queued and
    /// every maintenance advances the game year, so a message from an earlier
    /// year has been delivered and one from the current year has not.
    pub fn is_delivered(&self, current_year: u16) -> bool {
        !self.cancelled && self.year < current_year
    }

    /// The sender's own message, queued this turn and not yet delivered: the
    /// outbox and the inbox's outgoing rows list it, and it can be cancelled.
    pub fn is_queued_by(&self, sender_empire_id: u8, current_year: u16) -> bool {
        self.sender_empire_id == sender_empire_id
            && !self.cancelled
            && !self.is_delivered(current_year)
    }

    /// Delivered to this recipient and not deleted by them.
    pub fn is_visible_to_recipient(&self, recipient_empire_id: u8, current_year: u16) -> bool {
        self.recipient_empire_id == recipient_empire_id
            && !self.recipient_deleted
            && self.is_delivered(current_year)
    }

    pub fn is_visible_to_sender(&self, sender_empire_id: u8) -> bool {
        self.sender_empire_id == sender_empire_id && !self.sender_deleted && !self.cancelled
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.sender_deleted = true;
    }

    pub fn mark_deleted_by_recipient(&mut self) {
        self.recipient_deleted = true;
    }
}

pub fn queued_message_count_for_sender_recipient_year(
    queue: &[QueuedPlayerMail],
    sender_empire_id: u8,
    recipient_empire_id: u8,
    year: u16,
) -> usize {
    queue
        .iter()
        .filter(|mail| {
            !mail.cancelled
                && mail.sender_empire_id == sender_empire_id
                && mail.recipient_empire_id == recipient_empire_id
                && mail.year == year
        })
        .count()
}

pub fn validate_queue_message_limit(
    queue: &[QueuedPlayerMail],
    sender_empire_id: u8,
    recipient_empire_id: u8,
    year: u16,
) -> Result<(), String> {
    let queued = queued_message_count_for_sender_recipient_year(
        queue,
        sender_empire_id,
        recipient_empire_id,
        year,
    );
    if queued >= MAX_QUEUED_MESSAGES_PER_RECIPIENT_PER_YEAR {
        return Err(format!(
            "You may only queue {} messages to Empire {} this turn.",
            MAX_QUEUED_MESSAGES_PER_RECIPIENT_PER_YEAR, recipient_empire_id
        ));
    }
    Ok(())
}

pub fn validate_queued_player_message(
    queue: &[QueuedPlayerMail],
    sender_empire_id: u8,
    recipient_empire_id: u8,
    year: u16,
    player_count: u8,
    subject: &str,
    body: &str,
) -> Result<(String, String), String> {
    if recipient_empire_id == 0 || recipient_empire_id > player_count {
        return Err(format!(
            "message recipient must be in 1..={player_count}, got {recipient_empire_id}"
        ));
    }
    if recipient_empire_id == sender_empire_id {
        return Err("message recipient cannot be the submitting player".to_string());
    }

    let subject = subject.trim();
    let body = body.trim();
    if body.is_empty() {
        return Err("message body cannot be empty".to_string());
    }
    if subject.chars().count() > MAX_MESSAGE_SUBJECT_CHARS {
        return Err(format!(
            "message subject exceeds {} characters",
            MAX_MESSAGE_SUBJECT_CHARS
        ));
    }
    if body.chars().count() > MAX_MESSAGE_BODY_CHARS {
        return Err(format!(
            "message body exceeds {} characters",
            MAX_MESSAGE_BODY_CHARS
        ));
    }
    validate_queue_message_limit(queue, sender_empire_id, recipient_empire_id, year)?;

    Ok((subject.to_string(), body.to_string()))
}

pub fn queue_path(dir: &Path) -> PathBuf {
    dir.join(QUEUE_FILE_NAME)
}

pub fn load_mail_queue(dir: &Path) -> Result<Vec<QueuedPlayerMail>, Box<dyn std::error::Error>> {
    let path = queue_path(dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let parts = line.split('\t').collect::<Vec<_>>();
        let (sender_deleted, cancelled) = if parts.len() == 8 {
            (parse_deleted_flag(parts[6]), parse_deleted_flag(parts[7]))
        } else {
            (false, false)
        };
        let legacy = if parts.len() == 8 {
            &parts[..6]
        } else {
            parts.as_slice()
        };
        let (sender, recipient, year, subject, body, recipient_deleted) = match legacy {
            [sender, recipient, year, body] => (*sender, *recipient, *year, "", *body, false),
            [sender, recipient, year, subject, body] => {
                (*sender, *recipient, *year, *subject, *body, false)
            }
            [sender, recipient, year, subject, body, recipient_deleted] => (
                *sender,
                *recipient,
                *year,
                *subject,
                *body,
                parse_deleted_flag(recipient_deleted),
            ),
            _ => continue,
        };
        out.push(QueuedPlayerMail {
            sender_empire_id: sender.parse()?,
            recipient_empire_id: recipient.parse()?,
            year: year.parse()?,
            subject: unescape_field(subject),
            body: unescape_field(body),
            recipient_deleted,
            sender_deleted,
            cancelled,
        });
    }
    Ok(out)
}

pub fn append_mail_queue(
    dir: &Path,
    mail: &QueuedPlayerMail,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut queue = load_mail_queue(dir)?;
    queue.push(mail.clone());
    save_mail_queue(dir, &queue)
}

pub fn save_mail_queue(
    dir: &Path,
    queue: &[QueuedPlayerMail],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = queue_path(dir);
    if queue.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    let mut text = String::new();
    for mail in queue {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            mail.sender_empire_id,
            mail.recipient_empire_id,
            mail.year,
            escape_field(&mail.subject),
            escape_field(&mail.body),
            u8::from(mail.recipient_deleted),
            u8::from(mail.sender_deleted),
            u8::from(mail.cancelled)
        ));
    }
    fs::write(path, text)?;
    Ok(())
}

pub fn clear_mail_queue(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = queue_path(dir);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn parse_deleted_flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y"
    )
}
