//! What a door caller's screen shows, and the bytes that turn one screen
//! into the next (t208). A full frame redraws every row; after it, each
//! render sends only the changed spans of changed rows, with the cursor
//! hidden while they draw and colour codes cut to the parts that change.

use crate::screen::PlayfieldBuffer;
use crate::screen::layout::ScreenGeometry;
use crate::terminal::{ColorMode, OutputEncoding};
use crate::theme::classic;

use super::door::{BLINKING_BLOCK_CURSOR_STYLE_SEQUENCE, cursor_to, pen_codes, push_text};

/// Unchanged cells between two changed spans that are rewritten rather
/// than skipped with a cursor move, which costs about as many bytes.
const MERGE_GAP: usize = 4;

/// A cell's colour codes as the door sends them: ANSI foreground and
/// background, and bold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Pen {
    pub(super) fg: u8,
    pub(super) bg: u8,
    pub(super) bold: bool,
}

impl Pen {
    /// The codes that change the terminal from `self` to `to`. Turning bold
    /// off needs a reset, which then names both colours.
    fn change_to(self, to: Pen) -> Vec<u8> {
        if self == to {
            return Vec::new();
        }
        let mut params = Vec::new();
        if self.bold && !to.bold {
            params.extend(["0".to_string(), to.fg.to_string(), to.bg.to_string()]);
        } else {
            if self.fg != to.fg {
                params.push(to.fg.to_string());
            }
            if self.bg != to.bg {
                params.push(to.bg.to_string());
            }
            if to.bold && !self.bold {
                params.push("1".to_string());
            }
        }
        format!("\x1b[{}m", params.join(";")).into_bytes()
    }
}

/// The screen a frame leaves on the caller's terminal: every cell, as a
/// full frame draws it, the cursor, and the pen the frame ends with.
#[derive(Debug, Clone)]
pub(super) struct SentScreen {
    width: usize,
    cells: Vec<(char, Pen)>,
    cursor: Option<(u16, u16)>,
    base: Pen,
    /// The terminal's current colours after this screen was drawn.
    pub(super) pen: Pen,
}

impl SentScreen {
    /// The screen a full frame of `playfield` draws: each row up to its last
    /// non-blank cell, the rest erased in the base colours.
    pub(super) fn capture(
        playfield: &PlayfieldBuffer,
        geometry: ScreenGeometry,
        color_mode: ColorMode,
    ) -> Self {
        let base = pen_codes(
            classic::terminal_foreground(),
            classic::app_background(),
            false,
            color_mode,
        );
        let width = geometry.width();
        let visible_height = playfield.height().min(geometry.height());
        let mut cells = vec![(' ', base); width * geometry.height()];
        for row_idx in 0..visible_height {
            let row = playfield.row(row_idx);
            let Some(last) = row.iter().rposition(|cell| cell.ch != ' ') else {
                continue;
            };
            for (col, cell) in row[..=last].iter().take(width).enumerate() {
                let style = cell.style;
                cells[row_idx * width + col] = (
                    cell.ch,
                    pen_codes(style.fg, style.bg, style.bold, color_mode),
                );
            }
        }
        let cursor = playfield.cursor().map(|(col, row)| {
            let last_row = visible_height.saturating_sub(1) as u16;
            (col, row.min(last_row))
        });
        Self {
            width,
            cells,
            cursor,
            base,
            pen: base,
        }
    }

    fn row(&self, row: usize) -> &[(char, Pen)] {
        &self.cells[row * self.width..(row + 1) * self.width]
    }
}

/// The bytes that turn `prev` into `next` on the caller's screen, and the
/// pen they leave. Nothing at all when the two are the same.
pub(super) fn diff_frame(
    prev: &SentScreen,
    next: &SentScreen,
    encoding: OutputEncoding,
) -> (Vec<u8>, Pen) {
    let mut bytes = Vec::new();
    let mut pen = prev.pen;
    let rows = next.cells.len() / next.width;
    for row in 0..rows {
        let (old, new) = (prev.row(row), next.row(row));
        if old == new {
            continue;
        }
        if bytes.is_empty() {
            bytes.extend_from_slice(b"\x1b[?25l");
        }
        // Past `trail` the new row is blank in the base colours, so an
        // erase to the end of the line draws it.
        let trail = new
            .iter()
            .rposition(|cell| *cell != (' ', next.base))
            .map_or(0, |last| last + 1);
        for (start, end) in changed_spans(old, new) {
            bytes.extend_from_slice(cursor_to(row as u16, start as u16).as_bytes());
            let erase = end > trail;
            write_cells(
                &mut bytes,
                &new[start..if erase { trail.max(start) } else { end }],
                &mut pen,
                encoding,
            );
            if erase {
                bytes.extend_from_slice(&pen.change_to(next.base));
                pen = next.base;
                bytes.extend_from_slice(b"\x1b[K");
            }
        }
    }
    let redrawn = !bytes.is_empty();
    match next.cursor {
        Some((col, row)) if redrawn || prev.cursor != next.cursor => {
            if redrawn || prev.cursor.is_none() {
                bytes.extend_from_slice(BLINKING_BLOCK_CURSOR_STYLE_SEQUENCE);
                bytes.extend_from_slice(b"\x1b[?25h");
            }
            bytes.extend_from_slice(cursor_to(row, col).as_bytes());
        }
        None if !redrawn && prev.cursor.is_some() => bytes.extend_from_slice(b"\x1b[?25l"),
        _ => {}
    }
    (bytes, pen)
}

/// Runs of changed cells, as half-open column ranges, with short unchanged
/// gaps folded in.
fn changed_spans(old: &[(char, Pen)], new: &[(char, Pen)]) -> Vec<(usize, usize)> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for col in (0..new.len()).filter(|&col| old[col] != new[col]) {
        match spans.last_mut() {
            Some((_, end)) if col - *end <= MERGE_GAP => *end = col + 1,
            _ => spans.push((col, col + 1)),
        }
    }
    spans
}

fn write_cells(
    bytes: &mut Vec<u8>,
    cells: &[(char, Pen)],
    pen: &mut Pen,
    encoding: OutputEncoding,
) {
    let mut run = String::new();
    for &(ch, cell_pen) in cells {
        if cell_pen != *pen {
            push_text(bytes, &run, encoding);
            run.clear();
            bytes.extend_from_slice(&pen.change_to(cell_pen));
            *pen = cell_pen;
        }
        run.push(ch);
    }
    push_text(bytes, &run, encoding);
}
