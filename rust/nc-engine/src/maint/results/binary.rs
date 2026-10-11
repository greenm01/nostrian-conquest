use super::format::empire_label;
use nc_data::{CoreGameData, is_classic_text_char, set_login_reviewables_present};

pub const RESULTS_RECORD_SIZE: usize = 84;
pub const RESULTS_TEXT_SIZE: usize = 72;
pub const RESULTS_TEXT_START: usize = 2;
pub const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;
pub const RESULTS_END_OF_TRANSMISSION: &str = "<end of transmission>";

pub fn classic_results_tail_for_year(mut template: [u8; 10], year: u16) -> [u8; 10] {
    let year_bytes = year.to_le_bytes();
    template[8] = year_bytes[0];
    template[9] = year_bytes[1];
    template
}

pub fn classic_results_chain_tail_for_year(
    template: [u8; 10],
    year: u16,
    chain_id: u16,
    next_chain_id: u16,
) -> [u8; 10] {
    let mut tail = classic_results_tail_for_year(template, year);
    tail[0..2].copy_from_slice(&chain_id.to_le_bytes());
    tail[2..4].fill(0);
    tail[4..6].copy_from_slice(&next_chain_id.to_le_bytes());
    tail[6..8].fill(0);
    tail
}

pub fn push_classic_results_chunked(
    data: &mut Vec<u8>,
    header_tail: [u8; 10],
    continuation_tail: [u8; 10],
    text: &str,
) {
    let lines = classic_results_lines(text);
    if lines.is_empty() {
        return;
    }
    // ECGAME reads exactly `kind` records per report.  The kind byte doubles
    // as the record count.  Compute it from the actual text so every report
    // is exactly the right size — no padding, no truncation.
    let kind = (lines.len() + 1) as u8; // text lines + EOT

    for (line_idx, line) in lines.iter().enumerate() {
        let chunk = &line.as_bytes()[..line.len().min(RESULTS_TEXT_SIZE)];
        let mut record = [0u8; RESULTS_RECORD_SIZE];
        record[0] = kind;
        record[1] = chunk.len() as u8;
        record[RESULTS_TEXT_START..RESULTS_TEXT_START + chunk.len()].copy_from_slice(chunk);
        let tail = if line_idx == 0 {
            header_tail
        } else {
            continuation_tail
        };
        record[RESULTS_TEXT_END..RESULTS_RECORD_SIZE].copy_from_slice(&tail);
        data.extend_from_slice(&record);
    }

    let eot = RESULTS_END_OF_TRANSMISSION.as_bytes();
    let mut record = [0u8; RESULTS_RECORD_SIZE];
    record[0] = kind;
    record[1] = eot.len() as u8;
    record[RESULTS_TEXT_START..RESULTS_TEXT_START + eot.len()].copy_from_slice(eot);
    record[RESULTS_TEXT_END..RESULTS_RECORD_SIZE].copy_from_slice(&continuation_tail);
    data.extend_from_slice(&record);
}

pub fn classic_results_record_count(text: &str, _kind: u8) -> usize {
    let line_count = classic_results_lines(text).len();
    if line_count == 0 { 0 } else { line_count + 1 }
}

/// Map report text onto the classic single-byte character set: printable
/// ASCII and line breaks pass through, anything else becomes `?`.  After this
/// every character is exactly one record byte.
pub fn classic_results_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch == '\n' || is_classic_text_char(ch) {
                ch
            } else {
                '?'
            }
        })
        .collect()
}

pub fn classic_results_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let text = classic_results_text(text);
    let split = text.len().min(RESULTS_TEXT_SIZE);
    let first_line = text[..split].to_string();
    let mut lines = vec![first_line];
    let body = text[split..].trim_start();
    if body.is_empty() {
        return lines;
    }
    for paragraph in body.split('\n') {
        let paragraph = paragraph.trim_end();
        if paragraph.trim_start().is_empty() {
            lines.push(String::new());
            continue;
        }
        // Lines that already fit were laid out by the report builder (labeled
        // rows, hanging indents); only free prose needs wrapping here.
        if paragraph.len() <= RESULTS_TEXT_SIZE {
            lines.push(paragraph.to_string());
        } else {
            wrap_classic_paragraph(paragraph, RESULTS_TEXT_SIZE, &mut lines);
        }
    }
    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }
    lines
}

#[allow(dead_code)]
pub fn classic_message_text(text: &str) -> String {
    classic_results_lines(text).join("\n")
}

pub fn char_width(text: &str) -> usize {
    text.chars().count()
}

pub fn wrap_classic_paragraph(paragraph: &str, width: usize, lines: &mut Vec<String>) {
    let mut current = String::new();
    for word in paragraph.split_whitespace() {
        let word_width = char_width(word);
        if current.is_empty() {
            if word_width <= width {
                current.push_str(word);
            } else {
                push_split_long_word(word, width, lines, &mut current);
            }
            continue;
        }

        let candidate_width = char_width(&current) + 1 + word_width;
        if candidate_width <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            if word_width <= width {
                current.push_str(word);
            } else {
                push_split_long_word(word, width, lines, &mut current);
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
}

pub fn push_split_long_word(
    word: &str,
    width: usize,
    lines: &mut Vec<String>,
    current: &mut String,
) {
    let mut chunk = String::new();
    for ch in word.chars() {
        if char_width(&chunk) == width {
            lines.push(std::mem::take(&mut chunk));
        }
        chunk.push(ch);
    }
    if chunk.is_empty() {
        return;
    }
    if char_width(&chunk) == width {
        lines.push(chunk);
    } else {
        current.push_str(&chunk);
    }
}

#[allow(dead_code)]
pub fn push_routed_message_legacy_chunked(
    data: &mut Vec<u8>,
    kind: u8,
    tail: [u8; 10],
    text: &str,
) {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return;
    }
    for chunk in bytes.chunks(75) {
        let mut record = [0u8; RESULTS_RECORD_SIZE];
        record[0] = kind;
        record[1..1 + chunk.len()].copy_from_slice(chunk);
        record[76..84].copy_from_slice(&tail[2..]);
        data.extend_from_slice(&record);
    }
}

#[allow(dead_code)]
pub fn push_routed_message_chunked(
    data: &mut Vec<u8>,
    game_data: &mut CoreGameData,
    recipient_empire_raw: u8,
    kind: u8,
    tail: [u8; 10],
    text: &str,
) {
    if recipient_empire_raw == 0 {
        return;
    }
    let routed = format!(
        "For {}: {}",
        empire_label(game_data, recipient_empire_raw),
        text
    );
    if let Some(player) = game_data
        .players
        .get_mut(recipient_empire_raw.saturating_sub(1) as usize)
    {
        set_login_reviewables_present(player, true);
    }
    push_routed_message_legacy_chunked(data, kind, tail, &routed);
}
