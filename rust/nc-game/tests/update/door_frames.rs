//! Door renders send only screen changes (t208): replayed on a virtual ANSI
//! screen, the diffed stream leaves the caller's screen exactly as a full
//! redraw of every frame would.

use crate::support::*;
use nc_game::screen::{PlayfieldBuffer, ScreenGeometry};
use nc_game::terminal::door::{capture_door_terminal_for_test, serialize_playfield_frame};
use nc_game::terminal::{ColorMode, OutputEncoding};

const WIDTH: usize = 80;
const HEIGHT: usize = 25;

/// The parts of the terminal state a caller sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pen {
    fg: u8,
    bg: u8,
    bold: bool,
}

/// An 80x25 ANSI screen, interpreting the sequences the door sends.
#[derive(Debug, Clone, PartialEq, Eq)]
struct VirtualScreen {
    cells: Vec<(u8, Pen)>,
    cursor: (usize, usize),
    cursor_visible: bool,
    pen: Pen,
}

impl VirtualScreen {
    fn new() -> Self {
        let pen = Pen {
            fg: 37,
            bg: 40,
            bold: false,
        };
        Self {
            cells: vec![(b' ', pen); WIDTH * HEIGHT],
            cursor: (0, 0),
            cursor_visible: true,
            pen,
        }
    }

    fn apply(&mut self, bytes: &[u8]) {
        let mut idx = 0;
        while idx < bytes.len() {
            if bytes[idx] != 0x1b {
                let (row, col) = self.cursor;
                assert!(col < WIDTH, "text written past column {WIDTH} on row {row}");
                self.cells[row * WIDTH + col] = (bytes[idx], self.pen);
                self.cursor.1 += 1;
                idx += 1;
                continue;
            }
            assert_eq!(
                bytes.get(idx + 1),
                Some(&b'['),
                "only CSI sequences are sent"
            );
            let start = idx + 2;
            let end = start
                + bytes[start..]
                    .iter()
                    .position(|byte| (0x40..=0x7e).contains(byte))
                    .expect("a CSI sequence ends");
            let params = std::str::from_utf8(&bytes[start..end]).expect("ASCII parameters");
            self.csi(params, bytes[end]);
            idx = end + 1;
        }
    }

    fn csi(&mut self, params: &str, command: u8) {
        let numbers = || params.split(';').map(|p| p.parse::<u16>().unwrap_or(0));
        match command {
            b'H' => {
                let mut n = numbers();
                let row = usize::from(n.next().unwrap_or(1).max(1)) - 1;
                let col = usize::from(n.next().unwrap_or(1).max(1)) - 1;
                self.cursor = (row, col);
            }
            b'K' => {
                let (row, col) = self.cursor;
                for cell in &mut self.cells[row * WIDTH + col.min(WIDTH)..(row + 1) * WIDTH] {
                    *cell = (b' ', self.pen);
                }
            }
            b'J' => self.cells.fill((b' ', self.pen)),
            b'm' => {
                for code in numbers() {
                    match code {
                        0 => {
                            self.pen = Pen {
                                fg: 37,
                                bg: 40,
                                bold: false,
                            }
                        }
                        1 => self.pen.bold = true,
                        30..=37 | 90..=97 => self.pen.fg = code as u8,
                        40..=47 => self.pen.bg = code as u8,
                        other => panic!("unexpected SGR {other}"),
                    }
                }
            }
            b'h' | b'l' if params == "?25" => self.cursor_visible = command == b'h',
            b'q' | b's' | b'u' => {}
            other => panic!("unexpected CSI {params}{}", other as char),
        }
    }

    /// What the caller sees: a blank shows only its background.
    fn visible(&self) -> Vec<(u8, Pen)> {
        self.cells
            .iter()
            .map(|&(byte, pen)| {
                if byte == b' ' {
                    (
                        byte,
                        Pen {
                            fg: 0,
                            bold: false,
                            ..pen
                        },
                    )
                } else {
                    (byte, pen)
                }
            })
            .collect()
    }
}

/// Renders the door screens a short session visits, checking after each
/// that the diffed stream and a full redraw show the same screen. Returns
/// the bytes each render sent.
fn replay(keys: &[KeyCode]) -> Vec<(ScreenId, usize)> {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    enable_door_mode(&mut app);
    advance_to_main_menu(&mut app);

    let (mut door, output) = capture_door_terminal_for_test();
    let mut diffed = VirtualScreen::new();
    let mut full = VirtualScreen::new();
    let mut sizes = Vec::new();
    let mut check = |app: &mut App, door: &mut dyn Terminal| {
        // One render feeds both: a render can consume a one-shot notice.
        let mut both = Both {
            door,
            full: Vec::new(),
        };
        app.render(&mut both).expect("door render");
        let sent = output.take();
        diffed.apply(&sent);
        sizes.push((app.current_screen(), sent.len()));
        full.apply(&both.full);
        let (got, want) = (diffed.visible(), full.visible());
        if let Some(cell) = (0..got.len()).find(|&cell| got[cell] != want[cell]) {
            panic!(
                "after {:?}, row {} column {}: diffed {:?}, full {:?}",
                app.current_screen(),
                cell / WIDTH,
                cell % WIDTH,
                got[cell],
                want[cell]
            );
        }
        assert_eq!(diffed.cursor_visible, full.cursor_visible);
        if full.cursor_visible {
            assert_eq!(diffed.cursor, full.cursor);
        }
    };
    check(&mut app, &mut door);
    for &code in keys {
        let action = app.handle_key(key(code));
        let _ = apply_action(&mut app, action);
        check(&mut app, &mut door);
    }
    sizes
}

/// The door terminal, plus a full redraw of each frame it renders.
struct Both<'a> {
    door: &'a mut dyn Terminal,
    full: Vec<u8>,
}

impl Terminal for Both<'_> {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        self.full = serialize_playfield_frame(
            playfield,
            ScreenGeometry::local_default(),
            OutputEncoding::Cp437,
            ColorMode::Ansi16,
        );
        self.door.render(playfield)
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        Err("not used in tests".into())
    }

    fn dump_text_capture(&mut self, _text: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[test]
fn diffed_door_frames_rebuild_the_full_redraw() {
    use KeyCode::{Char, Down, Enter, Esc};
    let sizes = replay(&[
        Char('g'),
        Char('?'),
        Esc,
        Char('q'),
        Char('t'),
        Down,
        Down,
        Esc,
        Char('v'),
        Esc,
        Char('i'),
        Char('1'),
        Char('2'),
        Enter,
        Esc,
        Char('p'),
        Char('q'),
        Char('f'),
        Char('q'),
    ]);
    // The first frame is a full redraw; menus and prompts change little.
    assert!(sizes[0].1 > 1000, "first frame {sizes:?}");
    let typed = sizes
        .iter()
        .skip(1)
        .filter(|(screen, _)| *screen != ScreenId::PartialStarmapView)
        .map(|(_, bytes)| *bytes)
        .min()
        .expect("later frames");
    assert!(
        typed < 100,
        "a small change costs under 100 bytes: {sizes:?}"
    );
    for (screen, bytes) in &sizes {
        println!("{screen:?} {bytes}");
    }
}
