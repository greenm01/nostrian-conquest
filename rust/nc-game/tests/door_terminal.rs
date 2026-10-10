use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_game::screen::{FirstTimeMenuScreen, PlayfieldBuffer, ScreenGeometry};
use nc_game::terminal::door::{
    DoorTimeout, DoorTransport, decode_fragmented_input_for_test, decode_input_bytes_for_test,
    decode_input_stream_for_test, decode_timed_input_stream_for_test,
    read_scripted_door_keys_for_test, read_scripted_door_session_for_test,
    read_scripted_door_session_with_warning_for_test, serialize_playfield_frame,
};
use nc_game::terminal::{ColorMode, OutputEncoding};

fn apply_mag16_theme() {
    let theme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/themes/mag16.kdl");
    nc_game::theme::load_theme_from_path(&theme_path).expect("load mag16 theme");
}

#[test]
fn door_serializer_renders_first_time_menu_rows_and_prompt_cursor() {
    apply_mag16_theme();
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None, false).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[2;1H"));
    assert!(frame_text.contains("FIRST TIME MENU:"));
    assert!(frame_text.contains("\x1b[3;1H"));
    assert!(frame_text.contains("elp with commands"));
    assert!(frame_text.contains("\x1b[4;1H"));
    assert!(frame_text.contains("uit back to BBS"));
    assert!(buffer.plain_line(5).starts_with(" FIRST TIME COMMAND"));
    assert!(frame_text.contains("FIRST TIME COMMAND"));

    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor set");
    let final_cursor = format!("\x1b[{};{}H", cursor_row + 1, cursor_col + 1);
    assert!(frame_text.ends_with(&final_cursor));
}

#[test]
fn door_serializer_emits_classic_ansi16_colors_for_mag16_theme() {
    apply_mag16_theme();
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None, false).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[0;94;40;1m"));
    assert!(frame_text.contains("\x1b[0;33;40;1m"));
}

#[test]
fn door_serializer_projects_ansi_off_to_plain_greyscale_without_bold() {
    nc_game::theme::apply_door_theme();
    nc_game::theme::toggle_ansi_mode().expect("toggle ansi mode off");
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None, true).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[0;37;40m"));
    assert!(!frame_text.contains(";1m"));
    assert!(!frame_text.contains("\x1b[0;97;40"));
}

#[test]
fn door_serializer_avoids_alt_screen_and_hides_no_cursor() {
    apply_mag16_theme();
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None, false).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(!frame_text.contains("\x1b[?1049"));
    assert!(frame_text.contains("\x1b[1 q\x1b[?25h"));
    assert!(frame_text.contains("\x1b[?25h"));
}

#[test]
fn door_serializer_hides_cursor_when_playfield_has_no_cursor() {
    apply_mag16_theme();
    let mut buffer = PlayfieldBuffer::new(80, 25, nc_game::theme::classic::body_style());
    nc_game::screen::help::render_help_popup(
        &mut buffer,
        "HELP WITH COMMANDS",
        &["A ansi toggle".to_string(), "Q quit".to_string()],
    );
    assert!(buffer.cursor().is_none());

    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("HELP WITH COMMANDS"));
    assert!(frame_text.ends_with("\x1b[0;37;40m\x1b[?25l"));
    assert!(!frame_text.contains("\x1b[?25h"));
}

#[test]
fn door_serializer_trims_blank_rows_and_trailing_fill_spaces() {
    apply_mag16_theme();
    let mut buffer = PlayfieldBuffer::new(80, 25, nc_game::theme::classic::body_style());
    buffer.write_text(0, 0, "ABC", nc_game::theme::classic::title_style());
    buffer.set_cursor(2, 0);

    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("ABC"));
    assert!(!frame_text.contains("ABC "));
    assert!(frame_text.contains("\x1b[1 q"));
    assert_eq!(frame.iter().filter(|byte| **byte == b' ').count(), 1);
    assert!(!frame_text.contains("\x1b[2;1H\x1b[0;37;40m"));
}

const SOCKET: DoorTransport = DoorTransport::TcpConnect {
    host: "127.0.0.1",
    port: 1,
};

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn socket_keys(reads: &[(u64, &[u8])]) -> Vec<KeyEvent> {
    read_scripted_door_keys_for_test(reads, SOCKET)
}

fn assert_decode(bytes: &[u8], expected: KeyEvent) {
    let got = decode_input_bytes_for_test(bytes).expect("decode input");
    assert_eq!(got, expected, "bytes: {bytes:?}");
}

#[test]
fn door_input_decoder_maps_ascii_and_control_keys() {
    assert_decode(b"j", KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_decode(b"\r", KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_decode(b"\t", KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_decode(
        &[0x08],
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );
    assert_decode(
        &[0x7f],
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );
    assert_decode(
        &[0x03],
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert_decode(
        &[0x04],
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_decode(
        &[0x05],
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
    );
    assert_decode(
        &[0x15],
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_decode(
        &[0x18],
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
    );
    assert_decode(&[0x1b], KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
}

#[test]
fn door_input_decoder_maps_arrow_and_page_sequences() {
    assert_decode(b"\x1b[A", KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_decode(b"\x1b[B", KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_decode(b"\x1b[C", KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_decode(b"\x1b[D", KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert_decode(
        b"\x1b[U",
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
    );
    assert_decode(
        b"\x1b[V",
        KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
    );
    assert_decode(b"\x1bOA", KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_decode(b"\x1bOB", KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_decode(b"\x1bOC", KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_decode(b"\x1bOD", KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert_decode(b"\x1b[1;2A", KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_decode(
        b"\x1b[5~",
        KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
    );
    assert_decode(
        b"\x1b[6~",
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'H'],
        KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'P'],
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'M'],
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'K'],
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
    );
}

#[test]
fn door_input_decoder_keeps_home_end_delete_sequences() {
    assert_decode(b"\x1b[H", KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_decode(b"\x1b[K", KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_decode(b"\x1b[F", KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_decode(b"\x1bOH", KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_decode(b"\x1bOF", KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_decode(
        b"\x1b[3~",
        KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'G'],
        KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'O'],
        KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
    );
    assert_decode(
        &[0xe0, b'S'],
        KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
    );
}

#[test]
fn door_input_decoder_reassembles_fragmented_arrow_sequences() {
    let got = decode_fragmented_input_for_test(b"\x1b", b"[A").expect("decode fragmented");
    assert_eq!(got, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    let got = decode_fragmented_input_for_test(b"\x1b[", b"A").expect("decode fragmented");
    assert_eq!(got, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    let got = decode_fragmented_input_for_test(b"\x1b[1;", b"2D").expect("decode fragmented");
    assert_eq!(got, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
}

#[test]
fn door_input_decoder_handles_back_to_back_csi_arrows() {
    let got = decode_input_stream_for_test(b"\x1b[B\x1b[D").expect("decode stream");
    assert_eq!(
        got,
        vec![
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        ]
    );
}

#[test]
fn door_input_decoder_uses_one_short_deadline_for_fragmented_arrows() {
    let got = decode_timed_input_stream_for_test(&[(0, b"\x1b"), (15, b"[A")])
        .expect("decode timed stream");
    assert_eq!(got, vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)]);
}

#[test]
fn door_input_decoder_finalizes_bare_escape_quickly() {
    let got = decode_timed_input_stream_for_test(&[(0, b"\x1b")]).expect("decode timed stream");
    assert_eq!(got, vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)]);
}

#[test]
fn door_input_decoder_turns_around_on_opposite_direction_without_stale_timeout_lag() {
    let got = decode_timed_input_stream_for_test(&[
        (0, b"\x1b"),
        (10, b"[B"),
        (15, b"\x1b"),
        (25, b"[D"),
    ])
    .expect("decode timed stream");
    assert_eq!(
        got,
        vec![
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        ]
    );
}

#[test]
fn timed_out_escape_drops_late_sync_term_suffix_instead_of_leaking_menu_keys() {
    let got = decode_timed_input_stream_for_test(&[(0, b"\x1b"), (600, b"[V")])
        .expect("decode timed stream");
    assert_eq!(got, vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)]);
}

#[test]
fn linux_console_function_keys_decode_without_leaking_letters() {
    for (bytes, n) in [
        (b"\x1b[[A", 1),
        (b"\x1b[[B", 2),
        (b"\x1b[[C", 3),
        (b"\x1b[[D", 4),
        (b"\x1b[[E", 5),
    ] {
        assert_decode(bytes, plain(KeyCode::F(n)));
    }
    assert_eq!(
        decode_input_stream_for_test(b"\x1b[[Aj\x1b[[Zk").expect("decode stream"),
        vec![
            plain(KeyCode::F(1)),
            plain(KeyCode::Char('j')),
            plain(KeyCode::Char('k')),
        ]
    );
    assert_eq!(
        stdio_keys(&[(0, b"\x1b[["), (5, b"Cj")]),
        vec![plain(KeyCode::F(3)), plain(KeyCode::Char('j'))]
    );
    assert_eq!(
        stdio_keys(&[(0, b"\x1b"), (600, b"[[A")]),
        vec![plain(KeyCode::Esc)]
    );
}

#[test]
fn door_decoder_drops_late_sync_term_suffix_after_timed_out_escape() {
    let got = stdio_keys(&[(0, b"\x1b"), (600, b"[V")]);
    assert_eq!(got, vec![plain(KeyCode::Esc)]);
}

#[test]
fn door_decoder_keeps_keys_typed_after_a_timed_out_escape() {
    // A single typed '[' is its own read, never an orphaned suffix.
    assert_eq!(
        stdio_keys(&[(0, b"\x1b"), (300, b"[")]),
        vec![plain(KeyCode::Esc), plain(KeyCode::Char('['))]
    );
    // Past the orphan window, a read starting with '[' is ordinary input.
    assert_eq!(
        stdio_keys(&[(0, b"\x1b"), (1600, b"[V")]),
        vec![
            plain(KeyCode::Esc),
            plain(KeyCode::Char('[')),
            plain(KeyCode::Char('V'))
        ]
    );
}

#[test]
fn timed_sync_term_page_keys_decode_without_falling_back_to_escape() {
    let got = decode_timed_input_stream_for_test(&[
        (0, b"\x1b"),
        (15, b"[U"),
        (1000, b"\x1b"),
        (1015, b"[V"),
    ])
    .expect("decode timed stream");
    assert_eq!(
        got,
        vec![
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
        ]
    );
}

#[test]
fn incomplete_known_escape_prefix_does_not_fall_back_to_escape() {
    assert_decode(b"\x1b[", KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
    assert_decode(b"\x1bO", KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
}

#[test]
fn door_serializer_uses_bbs_safe_selected_row_cyan_background() {
    apply_mag16_theme();
    let mut buffer = PlayfieldBuffer::new(80, 25, nc_game::theme::classic::body_style());
    buffer.write_text(5, 1, "01", nc_game::theme::classic::selected_row_style());

    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[0;97;46m01"));
    assert!(!frame_text.contains("\x1b[0;97;106m"));
}

#[test]
fn socket_input_drops_telnet_option_negotiation() {
    // IAC WILL SGA, IAC WILL TTYPE, IAC DO ECHO, IAC DONT LINEMODE.
    let got = socket_keys(&[(0, b"\xff\xfb\x03\xff\xfb\x18\xff\xfd\x01\xff\xfe\x22y")]);
    assert_eq!(got, vec![plain(KeyCode::Char('y'))]);
}

#[test]
fn socket_input_drops_naws_subnegotiation_for_any_window_size() {
    for size in [
        &b"\xff\xfa\x1f\x00\x50\x00\x18\xff\xf0"[..],
        &b"\xff\xfa\x1f\x00\x84\x00\x2b\xff\xf0"[..],
        &b"\xff\xfa\x1f\x00\x48\x00\x4b\xff\xf0"[..],
        &b"\xff\xfa\x1f\x00\x4d\x00\x53\xff\xf0"[..],
    ] {
        let mut read = size.to_vec();
        read.push(b'q');
        assert_eq!(
            socket_keys(&[(0, &read)]),
            vec![plain(KeyCode::Char('q'))],
            "NAWS report: {size:02x?}"
        );
    }
}

#[test]
fn socket_input_drops_other_two_byte_telnet_commands() {
    // IAC NOP, IAC GA, IAC AYT between keys.
    let got = socket_keys(&[(0, b"a\xff\xf1b\xff\xf9c\xff\xf6d")]);
    assert_eq!(
        got,
        vec![
            plain(KeyCode::Char('a')),
            plain(KeyCode::Char('b')),
            plain(KeyCode::Char('c')),
            plain(KeyCode::Char('d')),
        ]
    );
}

#[test]
fn socket_input_strips_telnet_commands_split_across_reads() {
    let got = socket_keys(&[
        (0, b"\xff"),
        (5, b"\xfb"),
        (10, b"\x03j"),
        (15, b"\xff\xfa\x1f\x00"),
        (20, b"\x50\x00\x18\xff"),
        (25, b"\xf0k"),
        (30, b"\xff\xfa\x18\x00xterm\xff"),
        (35, b"\xf0l"),
    ]);
    assert_eq!(
        got,
        vec![
            plain(KeyCode::Char('j')),
            plain(KeyCode::Char('k')),
            plain(KeyCode::Char('l')),
        ]
    );
}

#[test]
fn socket_input_unescapes_doubled_iac_as_data() {
    // IAC IAC is a literal 0xFF data byte, which is not a key; the following
    // bytes must still decode normally instead of being read as a command.
    let got = socket_keys(&[(0, b"\xff\xff\x03"), (5, b"\xff"), (10, b"\xffz")]);
    assert_eq!(
        got,
        vec![
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            plain(KeyCode::Char('z')),
        ]
    );
}

#[test]
fn socket_input_drops_negotiation_inside_a_pending_escape_sequence() {
    let got = socket_keys(&[(0, b"\x1b"), (10, b"\xff\xfb\x03"), (20, b"[A")]);
    assert_eq!(got, vec![plain(KeyCode::Up)]);
}

#[test]
fn socket_input_resumes_after_an_unterminated_subnegotiation() {
    // IAC SB TTYPE and 1023 more payload bytes reach the 1024-byte cap with
    // no IAC SE; the bytes after it are keys again.
    let mut read = b"\xff\xfa\x18".to_vec();
    read.extend(std::iter::repeat_n(b'x', 1023));
    read.push(b'q');
    assert_eq!(socket_keys(&[(0, &read)]), vec![plain(KeyCode::Char('q'))]);
}

#[test]
fn socket_input_drops_a_subnegotiation_at_the_length_cap() {
    let mut read = b"\xff\xfa\x18".to_vec();
    read.extend(std::iter::repeat_n(b'x', 1023));
    read.extend_from_slice(b"\xff\xf0r");
    assert_eq!(socket_keys(&[(0, &read)]), vec![plain(KeyCode::Char('r'))]);
}

#[test]
fn stdio_input_is_not_telnet_filtered() {
    // Stdio hosts hand the door already-processed input; a stray 0xFF must
    // not swallow the key after it.
    let got = read_scripted_door_keys_for_test(&[(0, b"\xff\xfbx")], DoorTransport::Stdio);
    assert_eq!(got, vec![plain(KeyCode::Char('x'))]);
}

fn stdio_keys(reads: &[(u64, &[u8])]) -> Vec<KeyEvent> {
    read_scripted_door_keys_for_test(reads, DoorTransport::Stdio)
}

#[test]
fn telnet_enter_sequences_decode_as_one_enter() {
    let enter_then_y = vec![plain(KeyCode::Enter), plain(KeyCode::Char('y'))];
    assert_eq!(stdio_keys(&[(0, b"\r\ny")]), enter_then_y);
    assert_eq!(stdio_keys(&[(0, b"\r\0y")]), enter_then_y);
    assert_eq!(socket_keys(&[(0, b"\r\ny")]), enter_then_y);
    assert_eq!(socket_keys(&[(0, b"\r\0y")]), enter_then_y);
    // CR NUL must not turn the next byte into a DOS scan code.
    assert_eq!(
        stdio_keys(&[(0, b"\r\0H")]),
        vec![plain(KeyCode::Enter), plain(KeyCode::Char('H'))]
    );
}

#[test]
fn telnet_enter_sequences_split_across_reads_decode_as_one_enter() {
    let enter_then_y = vec![plain(KeyCode::Enter), plain(KeyCode::Char('y'))];
    assert_eq!(stdio_keys(&[(0, b"\r"), (5, b"\ny")]), enter_then_y);
    assert_eq!(
        stdio_keys(&[(0, b"\r"), (5, b"\0"), (10, b"y")]),
        enter_then_y
    );
}

#[test]
fn bare_cr_and_bare_lf_each_stay_enter() {
    assert_eq!(stdio_keys(&[(0, b"\r")]), vec![plain(KeyCode::Enter)]);
    assert_eq!(stdio_keys(&[(0, b"\n")]), vec![plain(KeyCode::Enter)]);
    assert_eq!(
        stdio_keys(&[(0, b"\r\r\n")]),
        vec![plain(KeyCode::Enter), plain(KeyCode::Enter)]
    );
    assert_eq!(
        stdio_keys(&[(0, b"\n\n")]),
        vec![plain(KeyCode::Enter), plain(KeyCode::Enter)]
    );
}

#[test]
fn dos_extended_keys_still_decode_when_not_preceded_by_cr() {
    assert_eq!(stdio_keys(&[(0, b"\0H")]), vec![plain(KeyCode::Up)]);
    assert_eq!(
        stdio_keys(&[(0, b"\r"), (5, b"a"), (10, b"\0P")]),
        vec![
            plain(KeyCode::Enter),
            plain(KeyCode::Char('a')),
            plain(KeyCode::Down)
        ]
    );
    assert_eq!(
        stdio_keys(&[(0, b"\r\xe0H")]),
        vec![plain(KeyCode::Enter), plain(KeyCode::Up)]
    );
}

fn ms(value: u64) -> Option<Duration> {
    Some(Duration::from_millis(value))
}

#[test]
fn idle_limit_ends_a_silent_door_session() {
    let started = Instant::now();
    let got = read_scripted_door_session_for_test(&[], DoorTransport::Stdio, None, ms(150));
    assert_eq!(got, (vec![], Some(DoorTimeout::Idle)));
    assert!(started.elapsed() >= Duration::from_millis(150));
}

#[test]
fn each_key_restarts_the_idle_limit() {
    let started = Instant::now();
    let got = read_scripted_door_session_for_test(
        &[(100, b"a"), (200, b"b")],
        DoorTransport::Stdio,
        None,
        ms(150),
    );
    assert_eq!(
        got,
        (
            vec![plain(KeyCode::Char('a')), plain(KeyCode::Char('b'))],
            Some(DoorTimeout::Idle)
        )
    );
    assert!(started.elapsed() >= Duration::from_millis(350));
}

#[test]
fn telnet_traffic_does_not_restart_the_idle_limit() {
    let got = read_scripted_door_session_for_test(
        &[
            (50, b"\xff\xfa\x1f\x00\x50\x00\x18\xff\xf0"),
            (100, b"\xff\xf1"),
            (400, b"a"),
        ],
        SOCKET,
        None,
        ms(200),
    );
    assert_eq!(got, (vec![], Some(DoorTimeout::Idle)));
}

#[test]
fn session_limit_ends_the_door_session_even_while_typing() {
    let got = read_scripted_door_session_for_test(
        &[(0, b"a"), (100, b"b"), (400, b"c")],
        DoorTransport::Stdio,
        ms(250),
        ms(1000),
    );
    assert_eq!(
        got,
        (
            vec![plain(KeyCode::Char('a')), plain(KeyCode::Char('b'))],
            Some(DoorTimeout::SessionLimit)
        )
    );
}

#[test]
fn door_session_without_limits_reads_until_input_closes() {
    let got = read_scripted_door_session_for_test(&[(300, b"a")], DoorTransport::Stdio, None, None);
    assert_eq!(got, (vec![plain(KeyCode::Char('a'))], None));
}

const WARNING: &str = "Two minutes remain in your time limit for this session.";

#[test]
fn session_limit_warning_is_shown_once_before_time_runs_out() {
    let started = Instant::now();
    let (keys, timeout, output) = read_scripted_door_session_with_warning_for_test(
        &[(0, b"a"), (250, b"b"), (350, b"c")],
        ms(500).unwrap(),
        ms(200).unwrap(),
    );
    assert_eq!(
        keys,
        vec![
            plain(KeyCode::Char('a')),
            plain(KeyCode::Char('b')),
            plain(KeyCode::Char('c'))
        ]
    );
    assert_eq!(timeout, Some(DoorTimeout::SessionLimit));
    assert!(started.elapsed() >= Duration::from_millis(500));
    let output = String::from_utf8_lossy(&output);
    assert_eq!(output.matches(WARNING).count(), 1, "{output:?}");
    // Drawn on the last body row, with the cursor saved and restored.
    assert!(output.starts_with("\x1b[s\x1b[24;1H"), "{output:?}");
    assert!(output.ends_with("\x1b[u"), "{output:?}");
}

#[test]
fn session_limit_warning_arrives_while_the_caller_is_idle() {
    let (_, timeout, output) = read_scripted_door_session_with_warning_for_test(
        &[(0, b"a")],
        ms(300).unwrap(),
        ms(100).unwrap(),
    );
    assert_eq!(timeout, Some(DoorTimeout::SessionLimit));
    assert!(String::from_utf8_lossy(&output).contains(WARNING));
}

#[test]
fn short_session_limits_get_no_warning() {
    for limit in [100, 200] {
        let (_, timeout, output) = read_scripted_door_session_with_warning_for_test(
            &[],
            ms(limit).unwrap(),
            ms(200).unwrap(),
        );
        assert_eq!(timeout, Some(DoorTimeout::SessionLimit));
        assert!(output.is_empty(), "limit {limit} ms");
    }
}
