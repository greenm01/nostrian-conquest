use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::screen::layout::{ScreenGeometry, last_body_row_for};
use crate::screen::{GameColor, PlayfieldBuffer};
use crate::terminal::cp437;
use crate::terminal::{ColorMode, OutputEncoding, Terminal};
use crate::theme::classic;

use super::door_telnet::TelnetFilter;
pub use super::door_transport::DoorTransport;
use super::door_transport::{DoorIo, build_door_io};
use super::stdout::resolve_color;

const DOOR_ESCAPE_TIMEOUT_MS: i32 = 100;
/// How long after a timed-out bare Esc is delivered a late `[`/`O` suffix
/// burst is still treated as the rest of that key (BBS hosts can split a
/// SyncTERM key sequence by several hundred milliseconds). A key typed on its
/// own is a single-byte read and is never dropped.
const DOOR_ORPHAN_SUFFIX_WINDOW_MS: u64 = 1000;
const DOOR_DOS_PREFIX_TIMEOUT_MS: i32 = 2000;
const MAX_ESCAPE_SEQUENCE_BYTES: usize = 16;
/// How long before the session time limit the caller is warned.
const SESSION_WARNING_LEAD: Duration = Duration::from_secs(120);
const SESSION_WARNING_TEXT: &str = "Two minutes remain in your time limit for this session.";
const DEFAULT_CURSOR_STYLE_SEQUENCE: &[u8] = b"\x1b[0 q";
const BLINKING_BLOCK_CURSOR_STYLE_SEQUENCE: &[u8] = b"\x1b[1 q";

pub struct DoorTerminal {
    encoding: OutputEncoding,
    color_mode: ColorMode,
    geometry: ScreenGeometry,
    trace_dir: Option<PathBuf>,
    frame_seq: u64,
    decoder: DoorInputDecoder,
    io: Box<dyn DoorIo>,
    session_deadline: Option<Instant>,
    /// When to warn that the session limit is near; cleared once shown.
    session_warning_at: Option<Instant>,
    idle_limit: Option<Duration>,
    last_key_at: Instant,
}

/// Why a door read stopped waiting for the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorTimeout {
    /// The session time limit from the dropfile or `--timeout` ran out.
    SessionLimit,
    /// No key arrived within the campaign's idle limit.
    Idle,
}

impl std::fmt::Display for DoorTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SessionLimit => {
                "Your time limit for this session has expired. Returning to the BBS."
            }
            Self::Idle => "No key was pressed for too long. Returning to the BBS.",
        })
    }
}

impl std::error::Error for DoorTimeout {}

impl DoorTerminal {
    pub fn with_transport_and_color_mode(
        encoding: OutputEncoding,
        color_mode: ColorMode,
        geometry: ScreenGeometry,
        transport: DoorTransport,
    ) -> io::Result<Self> {
        let trace_dir = std::env::var_os("NC_GAME_DOOR_TRACE_DIR").map(PathBuf::from);
        if let Some(path) = trace_dir.as_deref() {
            let _ = std::fs::create_dir_all(path);
        }
        Ok(Self {
            encoding,
            color_mode,
            geometry,
            trace_dir,
            frame_seq: 0,
            decoder: DoorInputDecoder::new(transport),
            io: build_door_io(transport)?,
            session_deadline: None,
            session_warning_at: None,
            idle_limit: None,
            last_key_at: Instant::now(),
        })
    }

    /// Stop waiting for keys once `session_limit` has passed since this call
    /// or no key has arrived for `idle_limit`; `read_key` then fails with
    /// [`DoorTimeout`]. A session limit longer than two minutes also gets a
    /// one-line warning two minutes before it runs out.
    pub fn with_input_limits(
        self,
        session_limit: Option<Duration>,
        idle_limit: Option<Duration>,
    ) -> Self {
        self.with_input_limits_and_warning_lead(session_limit, idle_limit, SESSION_WARNING_LEAD)
    }

    fn with_input_limits_and_warning_lead(
        mut self,
        session_limit: Option<Duration>,
        idle_limit: Option<Duration>,
        warning_lead: Duration,
    ) -> Self {
        let now = Instant::now();
        self.session_deadline = session_limit.map(|limit| now + limit);
        self.session_warning_at = session_limit
            .filter(|limit| *limit > warning_lead)
            .map(|limit| now + limit - warning_lead);
        self.idle_limit = idle_limit;
        self.last_key_at = now;
        self
    }

    pub fn with_encoding_and_color_mode(
        encoding: OutputEncoding,
        color_mode: ColorMode,
        geometry: ScreenGeometry,
    ) -> io::Result<Self> {
        Self::with_transport_and_color_mode(encoding, color_mode, geometry, DoorTransport::Stdio)
    }

    /// Once its time comes, draw the session warning over the last body row,
    /// leaving the cursor where the frame put it. The next frame redraws the
    /// row.
    fn show_due_session_warning(&mut self) -> io::Result<()> {
        if self
            .session_warning_at
            .is_none_or(|warning_at| warning_at > Instant::now())
        {
            return Ok(());
        }
        self.session_warning_at = None;
        let notice = classic::notice_style();
        let mut bytes = b"\x1b[s".to_vec();
        bytes.extend_from_slice(cursor_to(last_body_row_for(self.geometry) as u16, 0).as_bytes());
        bytes.extend_from_slice(&style_sgr(
            notice.fg,
            notice.bg,
            notice.bold,
            self.color_mode,
        ));
        bytes.extend_from_slice(b"\x1b[2K");
        bytes.extend_from_slice(SESSION_WARNING_TEXT.as_bytes());
        bytes.extend_from_slice(&style_sgr(
            classic::terminal_foreground(),
            classic::app_background(),
            false,
            self.color_mode,
        ));
        bytes.extend_from_slice(b"\x1b[u");
        self.io.write_all(&bytes)?;
        self.io.flush()
    }
}

impl Terminal for DoorTerminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let frame =
            serialize_playfield_frame(playfield, self.geometry, self.encoding, self.color_mode);
        if let Some(trace_dir) = self.trace_dir.as_deref() {
            trace_output_frame(trace_dir, self.frame_seq, &frame)?;
            self.frame_seq = self.frame_seq.saturating_add(1);
        }
        self.io.write_all(&frame)?;
        self.io.flush()?;
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        loop {
            self.show_due_session_warning()?;
            let session = self
                .session_deadline
                .map(|deadline| (deadline, DoorTimeout::SessionLimit));
            let idle = self
                .idle_limit
                .map(|limit| (self.last_key_at + limit, DoorTimeout::Idle));
            let limit = [session, idle]
                .into_iter()
                .flatten()
                .min_by_key(|(deadline, _)| *deadline);
            let wait_until = [limit.map(|(deadline, _)| deadline), self.session_warning_at]
                .into_iter()
                .flatten()
                .min();
            if let Some(key) =
                self.decoder
                    .next_key(self.io.as_mut(), self.trace_dir.as_deref(), wait_until)?
            {
                self.last_key_at = Instant::now();
                return Ok(key);
            }
            // Otherwise the wait ended at the warning time; show it and wait on.
            if let Some((deadline, timeout)) = limit
                && deadline <= Instant::now()
            {
                return Err(Box::new(timeout));
            }
        }
    }

    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.io
            .write_all(&reset_visible_screen_prefix(self.color_mode))?;
        self.io.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            self.io.write_all(b"\r\n")?;
        }
        self.io.flush()?;
        Ok(())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.io
            .write_all(&reset_visible_screen_prefix(self.color_mode))?;
        self.io.flush()?;
        Ok(())
    }
}

pub fn serialize_playfield_frame(
    playfield: &PlayfieldBuffer,
    geometry: ScreenGeometry,
    encoding: OutputEncoding,
    color_mode: ColorMode,
) -> Vec<u8> {
    let color_mode = match color_mode {
        ColorMode::Ansi16 => ColorMode::Ansi16,
        ColorMode::Color256 | ColorMode::TrueColor => ColorMode::Ansi16,
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&style_sgr(
        classic::terminal_foreground(),
        classic::app_background(),
        false,
        color_mode,
    ));
    bytes.extend_from_slice(b"\x1b[2J\x1b[H");

    let visible_height = playfield.height().min(geometry.height());
    for row_idx in 0..visible_height {
        let row = playfield.row(row_idx);
        let Some(last_visible_idx) = row.iter().rposition(|cell| cell.ch != ' ') else {
            continue;
        };

        bytes.extend_from_slice(cursor_to(row_idx as u16, 0).as_bytes());
        let mut current_style = None;
        let mut run = String::new();
        for cell in &row[..=last_visible_idx] {
            if current_style != Some(cell.style) {
                if !run.is_empty() {
                    push_text(&mut bytes, &run, encoding);
                    run.clear();
                }
                bytes.extend_from_slice(&style_sgr(
                    cell.style.fg,
                    cell.style.bg,
                    cell.style.bold,
                    color_mode,
                ));
                current_style = Some(cell.style);
            }
            run.push(cell.ch);
        }
        if !run.is_empty() {
            push_text(&mut bytes, &run, encoding);
        }
    }

    bytes.extend_from_slice(&style_sgr(
        classic::terminal_foreground(),
        classic::app_background(),
        false,
        color_mode,
    ));
    match playfield.cursor() {
        Some((cursor_col, cursor_row)) => {
            let default_cursor_row = visible_height.saturating_sub(1) as u16;
            let clamped_cursor_row = cursor_row.min(default_cursor_row);
            bytes.extend_from_slice(BLINKING_BLOCK_CURSOR_STYLE_SEQUENCE);
            bytes.extend_from_slice(b"\x1b[?25h");
            bytes.extend_from_slice(cursor_to(clamped_cursor_row, cursor_col).as_bytes());
        }
        None => bytes.extend_from_slice(b"\x1b[?25l"),
    }
    bytes
}

fn push_text(bytes: &mut Vec<u8>, text: &str, encoding: OutputEncoding) {
    match encoding {
        OutputEncoding::Utf8 => bytes.extend_from_slice(text.as_bytes()),
        OutputEncoding::Cp437 => bytes.extend_from_slice(&cp437::str_to_cp437(text)),
    }
}

fn cursor_to(row_zero_based: u16, col_zero_based: u16) -> String {
    format!("\x1b[{};{}H", row_zero_based + 1, col_zero_based + 1)
}

fn style_sgr(fg: GameColor, bg: GameColor, bold: bool, color_mode: ColorMode) -> Vec<u8> {
    let fg_code = ansi_fg_code(resolve_color(fg, color_mode));
    let bg_code = ansi_bg_code(resolve_color(bg, color_mode));
    let mut seq = format!("\x1b[0;{fg_code};{bg_code}");
    if bold {
        seq.push_str(";1");
    }
    seq.push('m');
    seq.into_bytes()
}

fn reset_visible_screen_prefix(color_mode: ColorMode) -> Vec<u8> {
    let mut bytes = style_sgr(
        classic::terminal_foreground(),
        classic::app_background(),
        false,
        color_mode,
    );
    bytes.extend_from_slice(DEFAULT_CURSOR_STYLE_SEQUENCE);
    bytes.extend_from_slice(b"\x1b[?25h\x1b[2J\x1b[H");
    bytes
}

fn ansi_fg_code(color: crossterm::style::Color) -> u8 {
    match color {
        crossterm::style::Color::Black => 30,
        crossterm::style::Color::DarkRed => 31,
        crossterm::style::Color::DarkGreen => 32,
        crossterm::style::Color::DarkYellow => 33,
        crossterm::style::Color::DarkBlue => 34,
        crossterm::style::Color::DarkMagenta => 35,
        crossterm::style::Color::DarkCyan => 36,
        crossterm::style::Color::Grey => 37,
        crossterm::style::Color::DarkGrey => 90,
        crossterm::style::Color::Red => 91,
        crossterm::style::Color::Green => 92,
        crossterm::style::Color::Yellow => 93,
        crossterm::style::Color::Blue => 94,
        crossterm::style::Color::Magenta => 95,
        crossterm::style::Color::Cyan => 96,
        crossterm::style::Color::White => 97,
        _ => 37, // fallback to grey
    }
}

fn ansi_bg_code(color: crossterm::style::Color) -> u8 {
    match color {
        crossterm::style::Color::Black => 40,
        crossterm::style::Color::DarkRed | crossterm::style::Color::Red => 41,
        crossterm::style::Color::DarkGreen | crossterm::style::Color::Green => 42,
        crossterm::style::Color::DarkYellow | crossterm::style::Color::Yellow => 43,
        crossterm::style::Color::DarkBlue | crossterm::style::Color::Blue => 44,
        crossterm::style::Color::DarkMagenta | crossterm::style::Color::Magenta => 45,
        crossterm::style::Color::DarkCyan | crossterm::style::Color::Cyan => 46,
        crossterm::style::Color::Grey
        | crossterm::style::Color::DarkGrey
        | crossterm::style::Color::White => 47,
        _ => 40, // fallback to black
    }
}

struct DoorInputDecoder {
    /// Present for socket transports, which carry the caller's raw telnet
    /// stream. Stdio doors get input the BBS has already de-telnetted.
    telnet: Option<TelnetFilter>,
    /// The last input byte was CR, so a following LF or NUL is the rest of a
    /// telnet end-of-line (CR LF / CR NUL), not a second key.
    after_cr: bool,
    pending: VecDeque<u8>,
    sequence_deadline: Option<Instant>,
    orphaned_escape_suffix_deadline: Option<Instant>,
}

enum ParseResult {
    Event(KeyEvent, usize),
    NeedMore,
    Drop(usize),
}

impl DoorInputDecoder {
    fn new(transport: DoorTransport) -> Self {
        Self {
            telnet: (transport != DoorTransport::Stdio).then(TelnetFilter::default),
            after_cr: false,
            pending: VecDeque::new(),
            sequence_deadline: None,
            orphaned_escape_suffix_deadline: None,
        }
    }

    /// Return the next key, or `None` once `deadline` passes with no key.
    fn next_key(
        &mut self,
        input: &mut dyn DoorIo,
        trace_dir: Option<&Path>,
        deadline: Option<Instant>,
    ) -> Result<Option<KeyEvent>, Box<dyn std::error::Error>> {
        loop {
            match try_decode_complete(&self.pending) {
                ParseResult::Event(event, consumed) => {
                    drain_pending(&mut self.pending, consumed);
                    self.sequence_deadline = None;
                    self.orphaned_escape_suffix_deadline = None;
                    return Ok(Some(event));
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut self.pending, consumed);
                    self.sequence_deadline = None;
                    self.orphaned_escape_suffix_deadline = None;
                }
                ParseResult::NeedMore => {
                    if self.pending.is_empty() {
                        self.sequence_deadline = None;
                        let wait_ms = match deadline {
                            Some(deadline) => match millis_until(deadline) {
                                0 => return Ok(None),
                                ms => Some(ms),
                            },
                            None => None,
                        };
                        self.read_burst(input, trace_dir, wait_ms)?;
                        continue;
                    }
                    let timeout_ms = self.remaining_sequence_timeout_ms();
                    if timeout_ms > 0 && self.read_burst(input, trace_dir, Some(timeout_ms))? {
                        continue;
                    }
                    match finalize_pending_after_timeout(&self.pending) {
                        ParseResult::Event(event, consumed) => {
                            self.orphaned_escape_suffix_deadline =
                                orphaned_suffix_deadline(event, &self.pending, consumed);
                            drain_pending(&mut self.pending, consumed);
                            self.sequence_deadline = None;
                            return Ok(Some(event));
                        }
                        ParseResult::Drop(consumed) => {
                            drain_pending(&mut self.pending, consumed);
                            self.sequence_deadline = None;
                            self.orphaned_escape_suffix_deadline = None;
                        }
                        ParseResult::NeedMore => {}
                    }
                }
            }
        }
    }

    fn read_burst(
        &mut self,
        input: &mut dyn DoorIo,
        trace_dir: Option<&Path>,
        timeout_ms: Option<i32>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let raw = read_burst(input, trace_dir, timeout_ms)?;
        let received = !raw.is_empty();
        let mut burst = match self.telnet.as_mut() {
            Some(telnet) => telnet.filter(&raw),
            None => raw,
        };
        let mut after_cr = self.after_cr;
        burst.retain(|&byte| {
            let end_of_line_suffix = after_cr && matches!(byte, b'\n' | 0x00);
            after_cr = byte == b'\r';
            !end_of_line_suffix
        });
        self.after_cr = after_cr;
        if received && burst.is_empty() {
            // Only telnet commands or an end-of-line suffix arrived; keep
            // waiting for keys.
            return Ok(true);
        }
        drop_orphaned_escape_suffix(
            &mut burst,
            self.orphaned_escape_suffix_deadline
                .filter(|deadline| *deadline > Instant::now()),
        );
        if burst.is_empty() {
            self.orphaned_escape_suffix_deadline = None;
            return Ok(false);
        }
        self.orphaned_escape_suffix_deadline = None;
        self.pending.extend(burst);
        Ok(true)
    }

    fn remaining_sequence_timeout_ms(&mut self) -> i32 {
        let timeout = match self.pending.front().copied() {
            Some(0x00 | 0xe0) => DOOR_DOS_PREFIX_TIMEOUT_MS,
            _ => DOOR_ESCAPE_TIMEOUT_MS,
        };
        let deadline = self
            .sequence_deadline
            .get_or_insert_with(|| Instant::now() + Duration::from_millis(timeout as u64));
        millis_until(*deadline)
    }
}

/// Milliseconds left until `deadline` as a poll timeout: 0 only once it has
/// passed, rounded up to 1 inside the last millisecond.
fn millis_until(deadline: Instant) -> i32 {
    let remaining = deadline.saturating_duration_since(Instant::now());
    let ms = remaining.as_millis().min(i32::MAX as u128) as i32;
    if ms == 0 && !remaining.is_zero() {
        1
    } else {
        ms
    }
}

fn try_decode_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(&first) = pending.front() else {
        return ParseResult::NeedMore;
    };

    match first {
        0x03 => ParseResult::Event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), 1),
        0x04 => ParseResult::Event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL), 1),
        0x05 => ParseResult::Event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL), 1),
        0x15 => ParseResult::Event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), 1),
        0x18 => ParseResult::Event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL), 1),
        b'\r' | b'\n' => ParseResult::Event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), 1),
        b'\t' => ParseResult::Event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), 1),
        0x08 | 0x7f => ParseResult::Event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), 1),
        0x20..=0x7e => ParseResult::Event(
            KeyEvent::new(KeyCode::Char(first as char), KeyModifiers::NONE),
            1,
        ),
        0x00 | 0xe0 => try_decode_dos_complete(pending),
        0x1b => try_decode_escape_complete(pending),
        _ => ParseResult::Drop(1),
    }
}

fn try_decode_dos_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(byte) = pending.get(1).copied() else {
        return ParseResult::NeedMore;
    };
    match map_dos_extended(byte) {
        Some(event) => ParseResult::Event(event, 2),
        None => ParseResult::Drop(2),
    }
}

fn try_decode_escape_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(second) = pending.get(1).copied() else {
        return ParseResult::NeedMore;
    };
    match second {
        b'[' => try_decode_csi_complete(pending),
        b'O' => try_decode_ss3_complete(pending),
        _ => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn try_decode_csi_complete(pending: &VecDeque<u8>) -> ParseResult {
    if pending.get(2) == Some(&b'[') {
        return try_decode_linux_console_function_key(pending);
    }
    let bytes = pending.iter().copied().collect::<Vec<_>>();
    for (idx, byte) in bytes.iter().copied().enumerate().skip(2) {
        if let Some(event) = map_csi_event(byte, &bytes[2..=idx]) {
            return ParseResult::Event(event, idx + 1);
        }
        if is_csi_final(byte) {
            return ParseResult::Drop(idx + 1);
        }
    }
    if pending.len() >= MAX_ESCAPE_SEQUENCE_BYTES {
        ParseResult::Drop(pending.len())
    } else {
        ParseResult::NeedMore
    }
}

/// Linux console F1-F5 send `ESC [ [ A` through `ESC [ [ E`.
fn try_decode_linux_console_function_key(pending: &VecDeque<u8>) -> ParseResult {
    match pending.get(3).copied() {
        None => ParseResult::NeedMore,
        Some(byte @ b'A'..=b'E') => ParseResult::Event(
            KeyEvent::new(KeyCode::F(byte - b'A' + 1), KeyModifiers::NONE),
            4,
        ),
        Some(_) => ParseResult::Drop(4),
    }
}

fn try_decode_ss3_complete(pending: &VecDeque<u8>) -> ParseResult {
    let Some(byte) = pending.get(2).copied() else {
        return ParseResult::NeedMore;
    };
    let event = match byte {
        b'A' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'B' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'C' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'D' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'H' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'F' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        _ => None,
    };
    match event {
        Some(event) => ParseResult::Event(event, 3),
        None => ParseResult::Drop(3),
    }
}

fn finalize_pending_after_timeout(pending: &VecDeque<u8>) -> ParseResult {
    match pending.front().copied() {
        Some(0x1b) => finalize_escape_after_timeout(pending),
        Some(0x00 | 0xe0) => ParseResult::Drop(pending.len().min(2)),
        Some(_) => ParseResult::Drop(1),
        None => ParseResult::NeedMore,
    }
}

fn finalize_escape_after_timeout(pending: &VecDeque<u8>) -> ParseResult {
    match pending.get(1).copied() {
        None => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
        Some(b'[' | b'O') => ParseResult::Drop(pending.len()),
        Some(_) => ParseResult::Event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), 1),
    }
}

fn map_dos_extended(byte: u8) -> Option<KeyEvent> {
    match byte {
        b'G' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'H' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'I' => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        b'K' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'M' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'O' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        b'P' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'Q' => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        b'S' => Some(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
        _ => None,
    }
}

fn map_csi_event(byte: u8, sequence: &[u8]) -> Option<KeyEvent> {
    match byte {
        b'A' => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        b'B' => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        b'C' => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        b'D' => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        b'H' => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        b'F' | b'K' => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        b'U' => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        b'V' => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        b'~' => match sequence {
            b"1~" | b"7~" => Some(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
            b"4~" | b"8~" => Some(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
            b"3~" => Some(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
            b"5~" => Some(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
            b"6~" => Some(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            _ => None,
        },
        _ => None,
    }
}

fn is_csi_final(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

fn orphaned_suffix_deadline(
    event: KeyEvent,
    pending: &VecDeque<u8>,
    consumed: usize,
) -> Option<Instant> {
    let is_bare_escape = event.code == KeyCode::Esc
        && matches!(pending.front().copied(), Some(0x1b))
        && consumed == 1
        && pending.len() == 1;
    is_bare_escape.then(|| Instant::now() + Duration::from_millis(DOOR_ORPHAN_SUFFIX_WINDOW_MS))
}

fn drop_orphaned_escape_suffix(burst: &mut Vec<u8>, orphan_deadline: Option<Instant>) {
    if orphan_deadline.is_none() || burst.is_empty() {
        return;
    }
    let consumed = match burst[0] {
        b'[' => orphan_csi_suffix_len(&burst[1..]).map(|len| len + 1),
        b'O' => orphan_ss3_suffix_len(&burst[1..]).map(|len| len + 1),
        _ => None,
    };
    if let Some(consumed) = consumed {
        burst.drain(..consumed.min(burst.len()));
    }
}

fn orphan_csi_suffix_len(bytes: &[u8]) -> Option<usize> {
    if bytes.first() == Some(&b'[') {
        // Linux console function key tail: `[` plus one letter.
        return Some(bytes.len().min(2));
    }
    for (idx, byte) in bytes.iter().copied().enumerate() {
        if is_csi_final(byte) {
            return Some(idx + 1);
        }
    }
    (!bytes.is_empty()).then_some(bytes.len())
}

fn orphan_ss3_suffix_len(bytes: &[u8]) -> Option<usize> {
    match bytes.first().copied() {
        Some(b'A' | b'B' | b'C' | b'D' | b'H' | b'F') => Some(1),
        Some(_) => Some(1),
        None => None,
    }
}

fn read_burst(
    input: &mut dyn DoorIo,
    trace_dir: Option<&Path>,
    timeout_ms: Option<i32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if let Some(timeout_ms) = timeout_ms {
        if !input.wait_ready(timeout_ms)? {
            return Ok(Vec::new());
        }
    }

    let mut burst = vec![read_one(input)?];
    while burst.len() < MAX_ESCAPE_SEQUENCE_BYTES && input.wait_ready(0)? {
        burst.push(read_one(input)?);
    }
    if let Some(trace_dir) = trace_dir {
        append_input_trace(trace_dir, &burst)?;
    }
    Ok(burst)
}

fn drain_pending(pending: &mut VecDeque<u8>, consumed: usize) {
    for _ in 0..consumed.min(pending.len()) {
        pending.pop_front();
    }
}

fn read_one(input: &mut dyn Read) -> Result<u8, Box<dyn std::error::Error>> {
    let mut byte = [0u8; 1];
    input.read_exact(&mut byte)?;
    Ok(byte[0])
}

fn trace_output_frame(
    trace_dir: &Path,
    frame_seq: u64,
    frame: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(trace_dir)?;
    let path = trace_dir.join(format!("frame-{frame_seq:06}.bin"));
    std::fs::write(path, frame)?;
    Ok(())
}

fn append_input_trace(trace_dir: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(trace_dir)?;
    let raw_path = trace_dir.join("input.bin");
    let mut raw = OpenOptions::new()
        .create(true)
        .append(true)
        .open(raw_path)?;
    raw.write_all(bytes)?;

    let log_path = trace_dir.join("input.log");
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let ts = trace_clock_start().elapsed().as_millis();
    write!(log, "{ts}")?;
    for byte in bytes {
        write!(log, " {:02x}", byte)?;
    }
    writeln!(log)?;
    Ok(())
}

fn trace_clock_start() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

#[doc(hidden)]
pub fn decode_input_bytes_for_test(bytes: &[u8]) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let mut pending = bytes.iter().copied().collect::<VecDeque<_>>();
    if pending.is_empty() {
        return Ok(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
    }
    loop {
        match try_decode_complete(&pending) {
            ParseResult::Event(event, _) => return Ok(event),
            ParseResult::Drop(consumed) => {
                drain_pending(&mut pending, consumed);
                if pending.is_empty() {
                    return Ok(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
                }
            }
            ParseResult::NeedMore => {
                return Ok(match finalize_pending_after_timeout(&pending) {
                    ParseResult::Event(event, _) => event,
                    ParseResult::Drop(_) | ParseResult::NeedMore => {
                        KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)
                    }
                });
            }
        }
    }
}

#[doc(hidden)]
pub fn decode_fragmented_input_for_test(
    initial: &[u8],
    continuation: &[u8],
) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    let mut pending = initial.iter().copied().collect::<VecDeque<_>>();
    if matches!(try_decode_complete(&pending), ParseResult::NeedMore) {
        pending.extend(continuation.iter().copied());
    }
    decode_input_bytes_for_test(&pending.iter().copied().collect::<Vec<_>>())
}

#[doc(hidden)]
pub fn decode_input_stream_for_test(
    bytes: &[u8],
) -> Result<Vec<KeyEvent>, Box<dyn std::error::Error>> {
    let mut pending = bytes.iter().copied().collect::<VecDeque<_>>();
    let mut events = Vec::new();
    while !pending.is_empty() {
        match try_decode_complete(&pending) {
            ParseResult::Event(event, consumed) => {
                drain_pending(&mut pending, consumed);
                events.push(event);
            }
            ParseResult::Drop(consumed) => {
                drain_pending(&mut pending, consumed);
            }
            ParseResult::NeedMore => match finalize_pending_after_timeout(&pending) {
                ParseResult::Event(event, consumed) => {
                    drain_pending(&mut pending, consumed);
                    events.push(event);
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut pending, consumed);
                }
                ParseResult::NeedMore => break,
            },
        }
    }
    Ok(events)
}

#[doc(hidden)]
pub fn decode_timed_input_stream_for_test(
    chunks: &[(u64, &[u8])],
) -> Result<Vec<KeyEvent>, Box<dyn std::error::Error>> {
    let mut pending = VecDeque::new();
    let mut events = Vec::new();
    let mut deadline_ms = None;
    let mut orphan_deadline_ms = None;

    for &(chunk_time_ms, bytes) in chunks {
        if let Some(deadline) = deadline_ms {
            if chunk_time_ms > deadline {
                // Production finalizes when the sequence wait expires, not
                // when the late bytes show up.
                finalize_timed_pending(
                    &mut pending,
                    &mut events,
                    &mut orphan_deadline_ms,
                    deadline,
                );
                deadline_ms = None;
            }
        }

        let mut burst = bytes.to_vec();
        if orphan_deadline_ms.is_some_and(|deadline| chunk_time_ms <= deadline) {
            drop_orphaned_escape_suffix(
                &mut burst,
                Some(Instant::now() + Duration::from_millis(DOOR_ESCAPE_TIMEOUT_MS as u64)),
            );
        }
        orphan_deadline_ms = None;
        pending.extend(burst.iter().copied());
        loop {
            match try_decode_complete(&pending) {
                ParseResult::Event(event, consumed) => {
                    drain_pending(&mut pending, consumed);
                    events.push(event);
                    deadline_ms = None;
                    orphan_deadline_ms = None;
                }
                ParseResult::Drop(consumed) => {
                    drain_pending(&mut pending, consumed);
                    deadline_ms = None;
                    orphan_deadline_ms = None;
                }
                ParseResult::NeedMore => {
                    if pending.is_empty() {
                        deadline_ms = None;
                    } else if deadline_ms.is_none() {
                        deadline_ms = Some(chunk_time_ms + DOOR_ESCAPE_TIMEOUT_MS as u64);
                    }
                    break;
                }
            }
        }
    }

    if deadline_ms.is_some() {
        finalize_timed_pending(&mut pending, &mut events, &mut orphan_deadline_ms, u64::MAX);
    }
    while !pending.is_empty() {
        match try_decode_complete(&pending) {
            ParseResult::Event(event, consumed) => {
                drain_pending(&mut pending, consumed);
                events.push(event);
            }
            ParseResult::Drop(consumed) => {
                drain_pending(&mut pending, consumed);
            }
            ParseResult::NeedMore => {
                finalize_timed_pending(
                    &mut pending,
                    &mut events,
                    &mut orphan_deadline_ms,
                    u64::MAX,
                );
            }
        }
    }
    Ok(events)
}

/// Read keys through the production door decoder from a scripted transport.
///
/// Each `(at_ms, bytes)` entry is one separate read from the wire, available
/// no earlier than `at_ms` after the start. Reading stops when the script is
/// exhausted.
#[doc(hidden)]
pub fn read_scripted_door_keys_for_test(
    reads: &[(u64, &[u8])],
    transport: DoorTransport,
) -> Vec<KeyEvent> {
    read_scripted_door_session_for_test(reads, transport, None, None).0
}

/// Like [`read_scripted_door_keys_for_test`], with session and idle limits.
/// Also returns the timeout that ended the reads, if any.
#[doc(hidden)]
pub fn read_scripted_door_session_for_test(
    reads: &[(u64, &[u8])],
    transport: DoorTransport,
    session_limit: Option<Duration>,
    idle_limit: Option<Duration>,
) -> (Vec<KeyEvent>, Option<DoorTimeout>) {
    let (keys, timeout, _) = read_scripted_door_session(reads, transport, |terminal| {
        terminal.with_input_limits(session_limit, idle_limit)
    });
    (keys, timeout)
}

/// Like [`read_scripted_door_session_for_test`] over stdio with a session
/// limit, warning `warning_lead` before it instead of two minutes. Also
/// returns everything the door wrote.
#[doc(hidden)]
pub fn read_scripted_door_session_with_warning_for_test(
    reads: &[(u64, &[u8])],
    session_limit: Duration,
    warning_lead: Duration,
) -> (Vec<KeyEvent>, Option<DoorTimeout>, Vec<u8>) {
    read_scripted_door_session(reads, DoorTransport::Stdio, |terminal| {
        terminal.with_input_limits_and_warning_lead(Some(session_limit), None, warning_lead)
    })
}

fn read_scripted_door_session(
    reads: &[(u64, &[u8])],
    transport: DoorTransport,
    limits: impl FnOnce(DoorTerminal) -> DoorTerminal,
) -> (Vec<KeyEvent>, Option<DoorTimeout>, Vec<u8>) {
    let output = Rc::new(RefCell::new(Vec::new()));
    let mut terminal = limits(DoorTerminal {
        encoding: OutputEncoding::Cp437,
        color_mode: ColorMode::Ansi16,
        geometry: ScreenGeometry::local_default(),
        trace_dir: None,
        frame_seq: 0,
        decoder: DoorInputDecoder::new(transport),
        io: Box::new(ScriptedDoorIo::new(reads, Rc::clone(&output))),
        session_deadline: None,
        session_warning_at: None,
        idle_limit: None,
        last_key_at: Instant::now(),
    });
    let mut keys = Vec::new();
    loop {
        match terminal.read_key() {
            Ok(key) => keys.push(key),
            Err(err) => {
                let timeout = err.downcast_ref::<DoorTimeout>().copied();
                return (keys, timeout, output.take());
            }
        }
    }
}

struct ScriptedDoorIo {
    start: Instant,
    reads: VecDeque<(u64, Vec<u8>)>,
    current: VecDeque<u8>,
    output: Rc<RefCell<Vec<u8>>>,
}

impl ScriptedDoorIo {
    fn new(reads: &[(u64, &[u8])], output: Rc<RefCell<Vec<u8>>>) -> Self {
        Self {
            start: Instant::now(),
            reads: reads
                .iter()
                .map(|(at_ms, bytes)| (*at_ms, bytes.to_vec()))
                .collect(),
            current: VecDeque::new(),
            output,
        }
    }

    /// Move to the next scripted read if it arrives within `timeout_ms`
    /// (`None` waits for it). Returns whether a read arrived.
    fn next_read(&mut self, timeout_ms: Option<i32>) -> bool {
        let wait_limit =
            timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms.max(0) as u64));
        let due = self
            .reads
            .front()
            .map(|(at_ms, _)| self.start + Duration::from_millis(*at_ms));
        match (due, wait_limit) {
            (Some(due), Some(limit)) if due > limit => sleep_until(limit),
            (Some(due), _) => {
                sleep_until(due);
                if let Some((_, bytes)) = self.reads.pop_front() {
                    self.current = bytes.into();
                }
                return true;
            }
            (None, Some(limit)) => sleep_until(limit),
            (None, None) => {}
        }
        false
    }
}

fn sleep_until(deadline: Instant) {
    std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
}

impl Read for ScriptedDoorIo {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current.is_empty() && !self.next_read(None) {
            return Ok(0);
        }
        match (buf.first_mut(), self.current.pop_front()) {
            (Some(slot), Some(byte)) => {
                *slot = byte;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

impl Write for ScriptedDoorIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl DoorIo for ScriptedDoorIo {
    /// Bytes of one scripted read are ready together; the next read is never
    /// merged into a zero-timeout poll, so every script entry is its own burst.
    fn wait_ready(&mut self, timeout_ms: i32) -> io::Result<bool> {
        if !self.current.is_empty() {
            return Ok(true);
        }
        if timeout_ms == 0 {
            return Ok(false);
        }
        Ok(self.next_read(Some(timeout_ms)))
    }
}

fn finalize_timed_pending(
    pending: &mut VecDeque<u8>,
    events: &mut Vec<KeyEvent>,
    orphan_deadline_ms: &mut Option<u64>,
    now_ms: u64,
) {
    match finalize_pending_after_timeout(pending) {
        ParseResult::Event(event, consumed) => {
            if event.code == KeyCode::Esc
                && matches!(pending.front().copied(), Some(0x1b))
                && consumed == 1
                && pending.len() == 1
            {
                *orphan_deadline_ms = Some(now_ms.saturating_add(DOOR_ORPHAN_SUFFIX_WINDOW_MS));
            } else {
                *orphan_deadline_ms = None;
            }
            drain_pending(pending, consumed);
            events.push(event);
        }
        ParseResult::Drop(consumed) => {
            drain_pending(pending, consumed);
            *orphan_deadline_ms = None;
        }
        ParseResult::NeedMore => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorMode, reset_visible_screen_prefix};

    #[test]
    fn reset_visible_screen_prefix_restores_default_cursor_shape() {
        let bytes = reset_visible_screen_prefix(ColorMode::Ansi16);
        let output = String::from_utf8_lossy(&bytes);
        assert!(output.contains("\x1b[0 q\x1b[?25h\x1b[2J\x1b[H"));
    }
}
