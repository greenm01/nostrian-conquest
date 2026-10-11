use crate::reports::{ReportsPreview, ReviewBlock, wrap_review_text_preserving_spacing};
use crate::screen::layout::{
    LEFT_WINDOW_PAD_COL, PLAYFIELD_WIDTH, ScreenGeometry, centered_row, command_line_row_for,
    dismiss_prompt_row_for, draw_bottom_aligned_transcript_rows, draw_plain_prompt_at_col,
    draw_plain_prompt_padded, last_body_row_for, new_playfield_for,
};
use crate::screen::{CellStyle, PlayfieldBuffer, ScreenFrame, StyledSpan};
use crate::startup::{StartupPhase, StartupSummary};
use crate::theme::classic;
use crate::util::Lcg;
use nc_ui::branding::NOSTRIAN_CONQUEST_LOGO;

fn is_star_decoration(ch: char) -> bool {
    matches!(ch, '.' | '*' | 'o')
}

const SPLASH_RNG_TAG: u64 = 0xEC15_5350_4C41_5348;

const STARTUP_LEFT_COL: usize = LEFT_WINDOW_PAD_COL;

fn startup_content_width() -> usize {
    PLAYFIELD_WIDTH.saturating_sub(STARTUP_LEFT_COL)
}

fn centered_startup_col(content_width: usize) -> usize {
    STARTUP_LEFT_COL + startup_content_width().saturating_sub(content_width) / 2
}

/// Split a text line into styled spans, highlighting specific phrases.
fn highlighted_spans<'a>(
    line: &'a str,
    phrases: &[&'a str],
    base: CellStyle,
    accent: CellStyle,
) -> Vec<StyledSpan<'a>> {
    let mut spans = Vec::new();
    let mut pos = 0;
    while pos < line.len() {
        let mut found = None;
        for phrase in phrases {
            if line[pos..].starts_with(phrase) {
                found = Some(*phrase);
                break;
            }
        }
        if let Some(phrase) = found {
            spans.push(StyledSpan::new(phrase, accent));
            pos += phrase.len();
        } else {
            // Scan ahead to the next phrase match or end of string.
            let next = phrases
                .iter()
                .filter_map(|p| line[pos..].find(p).map(|i| pos + i))
                .min()
                .unwrap_or(line.len());
            spans.push(StyledSpan::new(&line[pos..next], base));
            pos = next;
        }
    }
    spans
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupReviewMode {
    ViewPrompt,
    ItemBody,
    DeletePrompt,
    ContinuePrompt,
    EndStatus,
}

pub struct StartupScreen {
    summary: StartupSummary,
    result_blocks: Vec<ReviewBlock>,
    message_blocks: Vec<ReviewBlock>,
}

const ITEM_PREFIX: &str = " -> ";

pub(crate) fn startup_review_visible_lines(geometry: ScreenGeometry) -> usize {
    geometry.height().saturating_sub(5)
}

impl StartupScreen {
    pub fn new(summary: StartupSummary, reports: ReportsPreview) -> Self {
        Self {
            summary,
            result_blocks: reports.result_blocks,
            message_blocks: reports.message_blocks,
        }
    }

    pub fn replace(&mut self, summary: StartupSummary, reports: ReportsPreview) {
        self.summary = summary;
        self.result_blocks = reports.result_blocks;
        self.message_blocks = reports.message_blocks;
    }

    pub fn result_block_count(&self) -> usize {
        self.result_blocks.len()
    }

    pub fn message_block_count(&self) -> usize {
        self.message_blocks.len()
    }

    pub fn result_blocks(&self) -> &[ReviewBlock] {
        &self.result_blocks
    }

    pub fn message_blocks(&self) -> &[ReviewBlock] {
        &self.message_blocks
    }

    pub fn results_block_row_count(&self, block: usize) -> usize {
        block_review_rows(
            block_lines(&self.result_blocks, block),
            "Reports are marked pending, but no review text is available yet.",
        )
        .len()
    }

    pub fn messages_block_row_count(&self, block: usize) -> usize {
        block_review_rows(
            block_lines(&self.message_blocks, block),
            "Messages are marked pending, but no review text is available yet.",
        )
        .len()
    }

    pub fn render_phase(
        &self,
        frame: &ScreenFrame<'_>,
        phase: StartupPhase,
        status: Option<&str>,
        splash_page: usize,
        intro_page: usize,
        results_block: usize,
        results_scroll_offset: usize,
        results_mode: StartupReviewMode,
        results_nonstop: bool,
        results_history_rows: &[String],
        messages_block: usize,
        messages_scroll_offset: usize,
        messages_mode: StartupReviewMode,
        messages_nonstop: bool,
        messages_history_rows: &[String],
        results_deleted_any: bool,
        messages_deleted_any: bool,
        game_year: u16,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        match phase {
            StartupPhase::Splash => {
                render_splash(frame.geometry, splash_page, Some(frame.campaign_seed))
            }
            StartupPhase::Intro => {
                render_game_intro_page(frame.geometry, intro_page, "(slap a key)")
            }
            StartupPhase::LoginSummary => self.render_login_summary(frame, status),
            StartupPhase::Results => self.render_review(
                frame,
                "report",
                "reports",
                "Reports",
                &self.result_blocks,
                self.summary.pending_results,
                "Reports are marked pending, but no review text is available yet.",
                &[],
                results_history_rows,
                results_block,
                results_scroll_offset,
                results_mode,
                results_nonstop,
                results_deleted_any,
                game_year,
                status,
            ),
            StartupPhase::Messages => self.render_review(
                frame,
                "message",
                "messages",
                "Messages",
                &self.message_blocks,
                self.summary.pending_messages,
                "Messages are marked pending, but no review text is available yet.",
                results_history_rows,
                messages_history_rows,
                messages_block,
                messages_scroll_offset,
                messages_mode,
                messages_nonstop,
                messages_deleted_any,
                game_year,
                status,
            ),
            StartupPhase::Complete => Ok(new_playfield_for(frame.geometry)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_review(
        &self,
        frame: &ScreenFrame<'_>,
        singular: &str,
        plural: &str,
        section_label: &str,
        blocks: &[ReviewBlock],
        pending: bool,
        empty_notice: &str,
        prior_transcript_rows: &[String],
        review_history_rows: &[String],
        block: usize,
        scroll_offset: usize,
        mode: StartupReviewMode,
        nonstop: bool,
        deleted_any: bool,
        game_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(frame.geometry);
        let delete_prompt = format!("Delete this {singular} [Y]/N ->");
        let view_prompt =
            format!("You have undeleted {plural}. View them? [Y]es, <N>o, <NS> (non-stop) ->");
        let continue_prompt =
            format!("There are more {plural}. Continue? [Y]es, <N>o, <NS> (non-stop) ->");

        let command_line_row = command_line_row_for(frame.geometry);

        match mode {
            StartupReviewMode::ViewPrompt => {
                let mut transcript_rows = startup_login_summary_rows(frame, game_year);
                if !prior_transcript_rows.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend_from_slice(prior_transcript_rows);
                }
                if pending && blocks.is_empty() {
                    transcript_rows.push(String::new());
                    transcript_rows.extend(
                        wrap_review_text_preserving_spacing(empty_notice, PLAYFIELD_WIDTH)
                            .into_iter(),
                    );
                    render_review_transcript(frame.geometry, &mut buffer, &transcript_rows);
                    draw_plain_prompt_padded(&mut buffer, command_line_row, "(slap a key)");
                } else {
                    render_review_transcript(frame.geometry, &mut buffer, &transcript_rows);
                    draw_plain_prompt_padded(&mut buffer, command_line_row, &view_prompt);
                }
            }
            StartupReviewMode::ItemBody | StartupReviewMode::DeletePrompt => {
                let mut transcript_rows = active_review_transcript_rows(
                    prior_transcript_rows,
                    review_history_rows,
                    section_label,
                    game_year,
                );
                if review_history_rows.is_empty() {
                    for previous_block in 0..block {
                        let previous_rows =
                            block_review_rows(block_lines(blocks, previous_block), empty_notice);
                        push_completed_block_transcript(
                            &mut transcript_rows,
                            previous_rows,
                            &delete_prompt,
                            &continue_prompt,
                            true,
                        );
                    }
                }

                let rows = block_review_rows(block_lines(blocks, block), empty_notice);
                let revealed_end = usize::min(
                    scroll_offset + startup_review_visible_lines(frame.geometry),
                    rows.len(),
                );
                transcript_rows.extend(rows[..revealed_end].iter().cloned());

                render_review_transcript(frame.geometry, &mut buffer, &transcript_rows);

                let prompt_row = command_line_row;
                if revealed_end < rows.len() {
                    draw_plain_prompt_padded(&mut buffer, prompt_row, "(Slap a key for more)");
                } else if !nonstop {
                    draw_plain_prompt_padded(&mut buffer, prompt_row, &delete_prompt);
                } else {
                    draw_plain_prompt_padded(&mut buffer, prompt_row, "(slap a key)");
                }
            }
            StartupReviewMode::ContinuePrompt => {
                let mut transcript_rows = active_review_transcript_rows(
                    prior_transcript_rows,
                    review_history_rows,
                    section_label,
                    game_year,
                );
                if review_history_rows.is_empty() {
                    for previous_block in 0..block {
                        let previous_rows =
                            block_review_rows(block_lines(blocks, previous_block), empty_notice);
                        let include_continue_prompt = previous_block + 1 < block;
                        push_completed_block_transcript(
                            &mut transcript_rows,
                            previous_rows,
                            &delete_prompt,
                            &continue_prompt,
                            include_continue_prompt,
                        );
                    }
                }
                render_review_transcript(frame.geometry, &mut buffer, &transcript_rows);
                draw_plain_prompt_padded(&mut buffer, command_line_row, &continue_prompt);
            }
            StartupReviewMode::EndStatus => {
                let mut transcript_rows = active_review_transcript_rows(
                    prior_transcript_rows,
                    review_history_rows,
                    section_label,
                    game_year,
                );
                if deleted_any {
                    transcript_rows.push(String::new());
                    transcript_rows.push(format!("{} deleted.", capitalize(plural)));
                }
                render_review_transcript(frame.geometry, &mut buffer, &transcript_rows);
                draw_plain_prompt_padded(
                    &mut buffer,
                    command_line_row,
                    &format!("All {plural} seen. (Slap a key)"),
                );
            }
        }
        draw_startup_status(&mut buffer, frame.geometry, status);

        Ok(buffer)
    }

    fn render_login_summary(
        &self,
        frame: &ScreenFrame<'_>,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(frame.geometry);
        let rows = startup_login_summary_rows(frame, self.summary.game_year);
        render_review_transcript(frame.geometry, &mut buffer, &rows);
        draw_startup_status(&mut buffer, frame.geometry, status);
        draw_plain_prompt_padded(
            &mut buffer,
            command_line_row_for(frame.geometry),
            "(slap a key)",
        );
        Ok(buffer)
    }
}

fn draw_startup_status(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    status: Option<&str>,
) {
    let Some(status) = status else {
        return;
    };
    let row = command_line_row_for(geometry).saturating_sub(1);
    buffer.write_text(
        row,
        LEFT_WINDOW_PAD_COL,
        status,
        classic::status_value_style(),
    );
}

pub const STARTUP_INTRO_PAGE_COUNT: usize = INTRO_PAGES.len();
pub const STARTUP_SPLASH_PAGE_COUNT: usize = 1 + INTRO_PAGES.len();
pub fn version_title() -> String {
    format!("NC v{}", env!("CARGO_PKG_VERSION"))
}

const ATTRIBUTION: &str = "Slow turns are a feature, not a bug.";

const INTRO_ACCENT_PHRASES: &[&str] = &[
    "Nostrian dominion",
    "stations",
    "empires",
    "Star Masters",
    "persuasion from orbit",
    "maintenance",
    "industry, cunning, and will",
];

pub fn render_game_intro_page(
    geometry: ScreenGeometry,
    intro_page: usize,
    final_prompt: &str,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield_for(geometry);
    let text_start_row = 2;
    let lines = INTRO_PAGES
        .get(intro_page)
        .copied()
        .unwrap_or(INTRO_PAGES.last().copied().unwrap_or(&[]));

    let is_tribute = intro_page == 1;
    let mut last_content_row = 0usize;

    for (row, line) in lines.iter().enumerate() {
        let y = row + text_start_row;
        last_content_row = y;
        if is_tribute {
            buffer.write_text(y, STARTUP_LEFT_COL, line, classic::intro_tribute_style());
        } else if INTRO_ACCENT_PHRASES.iter().any(|p| line.contains(p)) {
            let spans = highlighted_spans(
                line,
                INTRO_ACCENT_PHRASES,
                classic::body_style(),
                classic::intro_accent_style(),
            );
            buffer.write_spans(y, STARTUP_LEFT_COL, &spans);
        } else {
            buffer.write_text(y, STARTUP_LEFT_COL, line, classic::body_style());
        }
    }
    let prompt = if intro_page + 1 < INTRO_PAGES.len() {
        "(slap a key)"
    } else {
        final_prompt
    };
    draw_plain_prompt_at_col(
        &mut buffer,
        dismiss_prompt_row_for(geometry, last_content_row),
        STARTUP_LEFT_COL,
        prompt,
    );
    Ok(buffer)
}

fn render_splash(
    geometry: ScreenGeometry,
    splash_page: usize,
    campaign_seed: Option<u64>,
) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let mut buffer = new_playfield_for(geometry);

    if splash_page == 0 {
        // First page: centered logo with attribution and version.
        let logo_width = NOSTRIAN_CONQUEST_LOGO
            .iter()
            .map(|line| line.len())
            .max()
            .unwrap_or(0);
        let logo_left = centered_startup_col(logo_width);
        let block_height = NOSTRIAN_CONQUEST_LOGO.len() + 4 + 1;
        let start_row = centered_row(0, last_body_row_for(geometry), block_height);

        // Render logo with randomized star-decoration colors.
        let mut rng = campaign_seed
            .map(|seed| Lcg::from_campaign_seed(seed, SPLASH_RNG_TAG))
            .unwrap_or_else(Lcg::from_time);
        for (row, line) in NOSTRIAN_CONQUEST_LOGO.iter().enumerate() {
            let y = row + start_row;
            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let style = if is_star_decoration(ch) {
                    classic::star_decoration_style(rng.next_usize())
                } else {
                    classic::logo_style()
                };
                buffer.write_text(y, logo_left + col, &ch.to_string(), style);
            }
        }

        buffer.write_text(
            start_row + NOSTRIAN_CONQUEST_LOGO.len() + 2,
            logo_left,
            ATTRIBUTION,
            classic::bright_style(),
        );
        buffer.write_text(
            start_row + NOSTRIAN_CONQUEST_LOGO.len() + 4,
            logo_left,
            &version_title(),
            classic::bright_style(),
        );
        draw_plain_prompt_at_col(
            &mut buffer,
            command_line_row_for(geometry),
            STARTUP_LEFT_COL,
            "View the game introduction? Y/[N] -> ",
        );
    } else {
        // Subsequent pages: transcript-style scrolling intro text.
        let mut transcript: Vec<String> = Vec::new();
        // Seed with the logo block as plain text lines.
        let logo_width = NOSTRIAN_CONQUEST_LOGO
            .iter()
            .map(|line| line.len())
            .max()
            .unwrap_or(0);
        let logo_left = centered_startup_col(logo_width);
        let logo_pad: String = " ".repeat(logo_left.saturating_sub(STARTUP_LEFT_COL));
        for line in &NOSTRIAN_CONQUEST_LOGO {
            transcript.push(format!("{logo_pad}{line}"));
        }
        transcript.push(String::new());
        transcript.push(format!("{logo_pad}{ATTRIBUTION}"));
        transcript.push(String::new());
        transcript.push(format!("{logo_pad}{}", version_title()));
        transcript.push(String::new());
        // Append intro pages up to the current splash page.
        let intro_index = splash_page - 1;
        for page_idx in 0..=intro_index.min(INTRO_PAGES.len().saturating_sub(1)) {
            transcript.push(String::new());
            for line in INTRO_PAGES[page_idx] {
                transcript.push((*line).to_string());
            }
        }
        render_review_transcript(geometry, &mut buffer, &transcript);
        let prompt = if intro_index + 1 < INTRO_PAGES.len() {
            "(slap a key)"
        } else {
            "(slap a key)"
        };
        draw_plain_prompt_at_col(
            &mut buffer,
            command_line_row_for(geometry),
            STARTUP_LEFT_COL,
            prompt,
        );
    }

    Ok(buffer)
}

fn block_lines<'a>(blocks: &'a [ReviewBlock], block: usize) -> &'a [String] {
    blocks.get(block).map(|b| b.lines.as_slice()).unwrap_or(&[])
}

pub(crate) fn block_review_rows(lines: &[String], empty_notice: &str) -> Vec<String> {
    if lines.is_empty() {
        if empty_notice.is_empty() {
            return Vec::new();
        }
        return wrap_review_text_preserving_spacing(
            empty_notice,
            startup_content_width().saturating_sub(ITEM_PREFIX.len()),
        )
        .into_iter()
        .map(|line| format!("{ITEM_PREFIX}{line}"))
        .collect();
    }

    let mut rows = Vec::new();
    let max_width = startup_content_width().saturating_sub(ITEM_PREFIX.len());
    for line in lines.iter() {
        if line.trim().is_empty() {
            rows.push(ITEM_PREFIX.trim_end().to_string());
            continue;
        }
        let wrapped = wrap_review_text_preserving_spacing(line, max_width);
        for segment in wrapped.iter() {
            rows.push(format!("{ITEM_PREFIX}{segment}"));
        }
    }
    rows
}

fn push_completed_block_transcript(
    transcript_rows: &mut Vec<String>,
    block_rows: Vec<String>,
    delete_prompt: &str,
    continue_prompt: &str,
    include_continue_prompt: bool,
) {
    transcript_rows.extend(block_rows);
    transcript_rows.push(String::new());
    transcript_rows.push(delete_prompt.to_string());
    transcript_rows.push(String::new());
    if include_continue_prompt {
        transcript_rows.push(continue_prompt.to_string());
        transcript_rows.push(String::new());
    }
}

pub(crate) fn completed_block_transcript_rows(
    singular: &str,
    plural: &str,
    block_rows: Vec<String>,
    include_header: bool,
    section_label: &str,
    game_year: u16,
    include_continue_prompt: bool,
) -> Vec<String> {
    let delete_prompt = format!("Delete this {singular} [Y]/N ->");
    let continue_prompt =
        format!("There are more {plural}. Continue? [Y]es, <N>o, <NS> (non-stop) ->");
    let mut transcript_rows = if include_header {
        review_roll_header_rows(section_label, game_year)
    } else {
        Vec::new()
    };
    push_completed_block_transcript(
        &mut transcript_rows,
        block_rows,
        &delete_prompt,
        &continue_prompt,
        include_continue_prompt,
    );
    transcript_rows
}

const REVIEW_FROM_HEADER_MARKER: &str = " -> From";
const REVIEW_SUBJECT_HEADER_MARKER: &str = " -> Subject:";

fn review_line_style(line: &str) -> CellStyle {
    if line.starts_with(REVIEW_FROM_HEADER_MARKER) {
        classic::report_header_style()
    } else if line.starts_with(REVIEW_SUBJECT_HEADER_MARKER) {
        classic::status_label_style()
    } else {
        classic::body_style()
    }
}

fn render_review_transcript(
    geometry: ScreenGeometry,
    buffer: &mut PlayfieldBuffer,
    transcript_rows: &[String],
) {
    let transcript_last_row = command_line_row_for(geometry).saturating_sub(2);
    let visible_lines = startup_review_visible_lines(geometry);
    draw_bottom_aligned_transcript_rows(
        buffer,
        transcript_rows,
        transcript_rows.len(),
        transcript_last_row + 1 - visible_lines,
        transcript_last_row,
        |buffer, y, line| {
            let line_style = review_line_style(line);
            if let Some(stardate_pos) = line.find("Stardate: ") {
                let label_end = stardate_pos + "Stardate: ".len();
                // Parse: week digits, slash, year digits.
                let rest = &line[label_end..];
                let week_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
                let after_week = label_end + week_len;
                let has_slash = line.as_bytes().get(after_week) == Some(&b'/');
                let year_start = if has_slash {
                    after_week + 1
                } else {
                    after_week
                };
                let year_len = line[year_start..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .count();
                let value_end = year_start + year_len;

                // Zero-pad week to 2 digits at display time.
                let week = &line[label_end..after_week];
                let week_padded = if week_len == 1 {
                    format!("0{week}")
                } else {
                    week.to_string()
                };

                let mut col = STARTUP_LEFT_COL;
                if stardate_pos > 0 {
                    col += buffer.write_text(y, col, &line[..stardate_pos], line_style);
                }
                col += buffer.write_text(
                    y,
                    col,
                    &line[stardate_pos..label_end],
                    classic::stardate_label_style(),
                );
                if week_len > 0 {
                    col += buffer.write_text(y, col, &week_padded, classic::stardate_week_style());
                }
                if has_slash {
                    col += buffer.write_text(y, col, "/", classic::stardate_label_style());
                }
                if year_len > 0 {
                    col += buffer.write_text(
                        y,
                        col,
                        &line[year_start..value_end],
                        classic::stardate_year_style(),
                    );
                }
                if value_end < line.len() {
                    buffer.write_text(y, col, &line[value_end..], line_style);
                }
            } else {
                buffer.write_text(y, STARTUP_LEFT_COL, line, line_style);
            }
        },
    );
}

fn startup_login_summary_rows(frame: &ScreenFrame<'_>, game_year: u16) -> Vec<String> {
    let identity = if !frame.player.handle.is_empty() {
        frame.player.handle.as_str()
    } else {
        display_or_unknown(&frame.player.empire_name)
    };
    vec![
        format!(
            "You are \"{identity}\", (Empire #{})",
            frame.player.record_index_1_based
        ),
        String::new(),
        format!("The year is: {game_year} A.D."),
        String::new(),
        format!("Last year on: {} A.D.", game_year.saturating_sub(1)),
    ]
}

fn active_review_transcript_rows(
    prior_transcript_rows: &[String],
    review_history_rows: &[String],
    section_label: &str,
    game_year: u16,
) -> Vec<String> {
    let mut transcript_rows = Vec::new();
    if !prior_transcript_rows.is_empty() {
        transcript_rows.extend_from_slice(prior_transcript_rows);
    }
    if !prior_transcript_rows.is_empty()
        && (!review_history_rows.is_empty() || !section_label.is_empty())
    {
        transcript_rows.push(String::new());
    }
    if review_history_rows.is_empty() {
        transcript_rows.extend(review_roll_header_rows(section_label, game_year));
    } else {
        transcript_rows.extend_from_slice(review_history_rows);
    }
    transcript_rows
}

fn review_roll_header_rows(section_label: &str, game_year: u16) -> Vec<String> {
    vec![
        format!("{section_label}: Current game year is {game_year} A.D."),
        String::new(),
    ]
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}

const INTRO_PAGE_1: [&str; 13] = [
    "Beyond the mapped frontiers of the old Nostrian dominion lies a small galaxy of",
    "contested solar systems. The old masters are gone. Their stations are silent,",
    "their patrols vanished, and their subjects left with fleets, industry, and",
    "enough knowledge to build empires.",
    "",
    "You rise as one of the new Star Masters. From a single world and a few small",
    "fleets, you must tax, build, scout, bargain, threaten, and strike before rival",
    "powers can do the same. Some systems will join your banner willingly. Others",
    "will require persuasion from orbit.",
    "",
    "Each maintenance marks the passage of a year. In that span, fleets cross the",
    "dark between stars, colonies grow or starve, alliances turn cold, and victory",
    "is decided by industry, cunning, and will.",
];

const INTRO_PAGES: [&[&str]; 1] = [&INTRO_PAGE_1];
