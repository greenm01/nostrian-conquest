use super::binary::{RESULTS_TEXT_SIZE, wrap_classic_paragraph};

pub enum StructuredBodyItem {
    Title(String),
    Label { label: String, value: String },
    Text(String),
    Blank,
}

pub enum MissionReportBody {
    Narrative(String),
    Structured(Vec<StructuredBodyItem>),
}

pub fn structured_report_text(header: &str, items: Vec<StructuredBodyItem>) -> String {
    let body = render_structured_body(&items);
    if body.is_empty() {
        header.to_string()
    } else {
        format!("{header}\n{body}")
    }
}

pub fn render_structured_body(items: &[StructuredBodyItem]) -> String {
    let mut lines = Vec::new();
    for item in items {
        match item {
            StructuredBodyItem::Title(text) | StructuredBodyItem::Text(text) => {
                lines.extend(wrap_text_lines(text, RESULTS_TEXT_SIZE));
            }
            StructuredBodyItem::Label { label, value } => {
                lines.extend(wrap_labeled_lines(label, value));
            }
            StructuredBodyItem::Blank => lines.push(String::new()),
        }
    }
    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

pub fn push_structured_section(
    items: &mut Vec<StructuredBodyItem>,
    section: Vec<StructuredBodyItem>,
) {
    if section.is_empty() {
        return;
    }
    items.push(StructuredBodyItem::Blank);
    items.extend(section);
}

pub fn structured_combat_body(
    title: impl Into<String>,
    context_rows: Vec<StructuredBodyItem>,
    force_rows: Vec<StructuredBodyItem>,
    outcome_rows: Vec<StructuredBodyItem>,
) -> Vec<StructuredBodyItem> {
    let mut items = vec![StructuredBodyItem::Title(title.into())];
    push_structured_section(&mut items, context_rows);
    push_structured_section(&mut items, force_rows);
    push_structured_section(&mut items, outcome_rows);
    items
}

pub fn wrap_text_lines(text: &str, width: usize) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    wrap_classic_paragraph(trimmed, width, &mut lines);
    lines
}

/// A labeled row as classic writes it: the label, one space, the value.
pub fn wrap_labeled_lines(label: &str, value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        return vec![label.trim_end().to_string()];
    }
    wrap_text_lines(&format!("{} {value}", label.trim_end()), RESULTS_TEXT_SIZE)
}
