use std::io::IsTerminal;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::app::{App, AppConfig, AppOutcome, RuntimeConfig, RuntimeSetupOverrides, apply_action};
use crate::dropfile;
use crate::screen::ScreenGeometry;
use crate::terminal::ColorMode;
use crate::terminal::OutputEncoding;
use crate::terminal::Terminal;
use crate::terminal::door::{DoorTerminal, DoorTimeout, DoorTransport};
use crate::terminal::stdout::StdoutTerminal;
use nc_data::{BbsGameConfig, CampaignStore, CampaignStoreError, SeatBindSource};
use nc_session::launch::{
    LaunchBindingRequest, LaunchPlayerBinding, LaunchPlayerBindingSource,
    resolve_launch_player_binding as shared_resolve_launch_player_binding,
};

struct ParsedLaunchArgs {
    game_dir: PathBuf,
    explicit_player_record_index_1_based: Option<usize>,
    export_root: Option<PathBuf>,
    queue_dir: Option<PathBuf>,
    log_file: Option<PathBuf>,
    log_level: nc_log::LogLevel,
    encoding: OutputEncoding,
    color_mode: ColorMode,
    screen_geometry: ScreenGeometry,
    dropfile_alias: Option<String>,
    dropfile_fallback_alias: Option<String>,
    dropfile_alias_is_real_name: bool,
    session_timeout_secs: Option<u32>,
    use_door_terminal: bool,
    door_transport: DoorTransport,
}
const LOOPBACK_DOOR_HOST: &str = "127.0.0.1";

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_args = args.into_iter().collect::<Vec<_>>();
    if matches!(
        parsed_args.get(1).map(String::as_str),
        Some("--help" | "-h")
    ) {
        print_usage();
        return Ok(());
    }
    if matches!(parsed_args.get(1).map(String::as_str), Some("submit-turn")) {
        return crate::submit_turn::run_submit_turn_args(&parsed_args[2..]);
    }
    let parsed = parse_args(&parsed_args)?;
    let mut app = match prepare_launch(&parsed) {
        Ok(app) => app,
        Err(err) => {
            show_door_startup_error(&parsed, err.as_ref());
            return Err(err);
        }
    };
    let idle_limit = app.game_config.idle_timeout_secs().map(Duration::from_secs);
    let mut terminal = build_terminal(&parsed, idle_limit)?;

    let stdout_is_terminal = std::io::stdout().is_terminal();
    let stdin_is_terminal = std::io::stdin().is_terminal();
    let interactive_terminal = should_use_interactive_loop(&parsed, stdout_is_terminal);
    let enable_raw_mode = stdin_is_terminal && stdout_is_terminal;
    let emit_local_exit_attribution =
        should_emit_local_exit_attribution(&parsed, stdout_is_terminal);
    let result = if interactive_terminal {
        run_interactive(&mut app, terminal.as_mut(), enable_raw_mode)
    } else {
        render_with_logging(&mut app, terminal.as_mut())
    };
    if result.is_ok() && emit_local_exit_attribution {
        emit_local_exit_lines();
    }
    result
}

fn prepare_launch(parsed: &ParsedLaunchArgs) -> Result<App, Box<dyn std::error::Error>> {
    if let Some(log_file) = &parsed.log_file {
        nc_log::init_file_logging(log_file, parsed.log_level)?;
        tracing::info!(
            log_file = %log_file.display(),
            level = ?parsed.log_level,
            "nc-game logging initialized"
        );
    }

    let campaign_store = CampaignStore::open_default_in_dir(&parsed.game_dir)?;
    let game_config = load_runtime_game_config(&parsed.game_dir, &campaign_store)?;
    validate_runtime_game_config(&campaign_store, &game_config)?;
    let (launch_binding, mut caller_alias, mut caller_alias_source) =
        resolve_launch_player_binding(parsed, &game_config, &campaign_store)?;
    let player_record_index_1_based = launch_binding.player_record_index_1_based().unwrap_or(1);
    let config = AppConfig {
        game_dir: parsed.game_dir.clone(),
        player_record_index_1_based,
        export_root: parsed.export_root.clone(),
        queue_dir: parsed.queue_dir.clone(),
        session_timeout_secs: parsed.session_timeout_secs,
        game_config: game_config.clone(),
    };

    let mut app = App::load(config)?;
    if launch_binding.source() == Some(LaunchPlayerBindingSource::StoredHandle) {
        if let Some(handle) = caller_handle(parsed) {
            if app.bind_seat_to_caller_handle(handle)? {
                caller_alias = Some(handle.to_string());
                caller_alias_source = SeatBindSource::Handle;
            }
        }
    }
    tracing::info!(
        game_dir = %parsed.game_dir.display(),
        player = player_record_index_1_based,
        door_mode = parsed.use_door_terminal,
        dropfile_alias = caller_alias.as_deref().unwrap_or(""),
        "loaded nc-game app"
    );
    apply_launch_context(
        &mut app,
        parsed,
        launch_binding,
        caller_alias,
        caller_alias_source,
    )?;
    if app.door_mode {
        crate::theme::apply_door_theme();
    }
    Ok(app)
}

fn build_terminal(
    parsed: &ParsedLaunchArgs,
    idle_limit: Option<Duration>,
) -> Result<Box<dyn Terminal>, Box<dyn std::error::Error>> {
    if parsed.use_door_terminal {
        Ok(Box::new(build_door_terminal(parsed, idle_limit)?))
    } else {
        Ok(Box::new(StdoutTerminal::with_encoding_and_color_mode(
            parsed.encoding,
            parsed.color_mode,
        )))
    }
}

fn build_door_terminal(
    parsed: &ParsedLaunchArgs,
    idle_limit: Option<Duration>,
) -> io::Result<DoorTerminal> {
    let session_limit = parsed
        .session_timeout_secs
        .map(|secs| Duration::from_secs(u64::from(secs)));
    Ok(DoorTerminal::with_transport_and_color_mode(
        parsed.encoding,
        parsed.color_mode,
        parsed.screen_geometry,
        parsed.door_transport,
    )?
    .with_input_limits(session_limit, idle_limit))
}

fn show_door_startup_error(parsed: &ParsedLaunchArgs, error: &(dyn std::error::Error + 'static)) {
    if !parsed.use_door_terminal {
        return;
    }
    let Ok(mut terminal) = build_door_terminal(parsed, None) else {
        return;
    };
    let message = format_door_startup_error(error);
    if terminal.dump_text_capture(&message).is_ok() {
        match terminal.read_key() {
            Ok(_) => {}
            Err(err) if is_hangup_error(err.as_ref()) => {}
            Err(_) => {}
        }
    }
    let _ = terminal.clear_and_restore();
}

fn format_door_startup_error(error: &(dyn std::error::Error + 'static)) -> String {
    let mut lines = vec!["NOSTRIAN CONQUEST DOOR ERROR".to_string(), String::new()];
    match error.downcast_ref::<CampaignStoreError>() {
        Some(CampaignStoreError::SchemaVersionMismatch { expected, found }) => {
            lines.push("This game directory does not match this build.".to_string());
            match found {
                Some(found) => lines.push(format!(
                    "Found runtime schema {found}, but this build expects {expected}."
                )),
                None => {
                    lines.push(
                        "This game directory is missing a supported runtime schema.".to_string(),
                    );
                    lines.push(format!("This build expects runtime schema {expected}."));
                }
            }
            lines.push("Recreate or refresh ncgame.db before launching again.".to_string());
        }
        _ => {
            lines.push("The door could not start.".to_string());
            lines.push(error.to_string());
        }
    }
    lines.push(String::new());
    lines.push("(slap a key)".to_string());
    lines.join("\r\n")
}

fn should_emit_local_exit_attribution(
    parsed: &ParsedLaunchArgs,
    interactive_terminal: bool,
) -> bool {
    interactive_terminal && !parsed.use_door_terminal
}

fn emit_local_exit_lines() {
    for line in local_exit_lines() {
        eprintln!("{line}");
    }
}

fn should_use_interactive_loop(parsed: &ParsedLaunchArgs, stdout_is_terminal: bool) -> bool {
    parsed.use_door_terminal || stdout_is_terminal
}

#[doc(hidden)]
pub fn local_exit_lines() -> Vec<String> {
    vec![]
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn run_interactive(
    app: &mut App,
    terminal: &mut dyn Terminal,
    enable_raw_mode_for_session: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = if enable_raw_mode_for_session {
        enable_raw_mode()?;
        Some(RawModeGuard)
    } else {
        None
    };
    let result = run_interactive_inner(app, terminal);
    drop(_guard);
    match result {
        Ok(SessionEnd::Quit) => terminal.clear_and_restore(),
        // The caller hung up, so the screen reset usually fails too (EIO on
        // a hung-up pty); that must not turn the hangup into an error exit.
        Ok(SessionEnd::InputClosed) => {
            let _ = terminal.clear_and_restore();
            Ok(())
        }
        // The caller may be gone (half-open socket); the message is best
        // effort and must not turn a normal exit into an error.
        Ok(SessionEnd::TimedOut(timeout)) => {
            let _ = terminal.dump_text_capture(&timeout.to_string());
            Ok(())
        }
        Err(err) => {
            let _ = terminal.clear_and_restore();
            Err(err)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum SessionEnd {
    Quit,
    InputClosed,
    TimedOut(DoorTimeout),
}

fn run_interactive_inner(
    app: &mut App,
    terminal: &mut dyn Terminal,
) -> Result<SessionEnd, Box<dyn std::error::Error>> {
    loop {
        match render_with_logging(app, terminal) {
            Ok(()) => {}
            Err(err) if is_hangup_error(err.as_ref()) => return Ok(SessionEnd::InputClosed),
            Err(err) => return Err(err),
        }
        let key = match terminal.read_key() {
            Ok(key) => key,
            Err(err) if is_hangup_error(err.as_ref()) => return Ok(SessionEnd::InputClosed),
            Err(err) => match err.downcast_ref::<DoorTimeout>() {
                Some(timeout) => {
                    tracing::info!(
                        player = app.player.record_index_1_based,
                        reason = ?timeout,
                        "door session timed out"
                    );
                    return Ok(SessionEnd::TimedOut(*timeout));
                }
                None => return Err(err),
            },
        };
        let screen_before = app.current_screen();
        tracing::debug!(
            screen = ?screen_before,
            key_code = ?key.code,
            key_modifiers = ?key.modifiers,
            key_kind = ?key.kind,
            "nc-game key received"
        );
        let action = app.handle_key(key);
        if matches!(action, crate::app::Action::Noop) {
            let guards = app.active_navigation_guards();
            tracing::debug!(
                screen = ?screen_before,
                key_code = ?key.code,
                key_modifiers = ?key.modifiers,
                key_kind = ?key.kind,
                active_guards = ?guards,
                "nc-game key resolved to noop"
            );
        } else {
            tracing::debug!(
                screen = ?screen_before,
                key_code = ?key.code,
                key_modifiers = ?key.modifiers,
                key_kind = ?key.kind,
                action = ?action,
                "nc-game key mapped to action"
            );
        }
        let outcome = apply_action(app, action);
        tracing::debug!(
            screen_before = ?screen_before,
            screen_after = ?app.current_screen(),
            outcome = ?outcome,
            "nc-game action applied"
        );
        if matches!(outcome, AppOutcome::Quit) {
            return Ok(SessionEnd::Quit);
        }
    }
}

/// The caller hung up: reads and writes on the door connection fail this way
/// after a carrier drop.
fn is_hangup_error(err: &(dyn std::error::Error + 'static)) -> bool {
    err.downcast_ref::<io::Error>().is_some_and(|io_err| {
        matches!(
            io_err.kind(),
            ErrorKind::UnexpectedEof
                | ErrorKind::BrokenPipe
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
        ) || is_pty_hangup(io_err)
    })
}

/// I/O on a pty whose other side hung up (carrier drop) fails with EIO.
#[cfg(unix)]
fn is_pty_hangup(err: &io::Error) -> bool {
    const EIO: i32 = 5;
    err.raw_os_error() == Some(EIO)
}

#[cfg(not(unix))]
fn is_pty_hangup(_err: &io::Error) -> bool {
    false
}

fn render_with_logging(
    app: &mut App,
    terminal: &mut dyn Terminal,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(err) = app.render(terminal) {
        tracing::error!(
            player = app.player.record_index_1_based,
            screen = ?app.current_screen(),
            error = %err,
            "nc-game render failed"
        );
        return Err(err);
    }
    Ok(())
}

fn apply_launch_context(
    app: &mut App,
    parsed: &ParsedLaunchArgs,
    launch_binding: LaunchPlayerBinding,
    caller_alias: Option<String>,
    caller_alias_source: SeatBindSource,
) -> Result<(), Box<dyn std::error::Error>> {
    app.screen_geometry = parsed.screen_geometry;
    app.door_mode = parsed.use_door_terminal;
    app.startup_state.fixed_player_launch = matches!(
        launch_binding.source(),
        Some(LaunchPlayerBindingSource::ExplicitPlayer)
    );
    app.startup_state.caller_alias = caller_alias;
    app.startup_state.caller_alias_source = caller_alias_source;
    if launch_binding == LaunchPlayerBinding::UnboundDropfile {
        app.enter_unbound_bbs_first_time_mode();
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<ParsedLaunchArgs, Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player: Option<usize> = None;
    let mut export_root = std::env::var_os("NC_GAME_EXPORT_ROOT")
        .or_else(|| std::env::var_os("EC_CLIENT_EXPORT_ROOT"))
        .map(PathBuf::from);
    let mut queue_dir = std::env::var_os("NC_GAME_QUEUE_DIR")
        .or_else(|| std::env::var_os("EC_CLIENT_QUEUE_DIR"))
        .map(PathBuf::from);
    let mut log_file = std::env::var_os("NC_GAME_LOG_FILE")
        .or_else(|| std::env::var_os("EC_GAME_LOG_FILE"))
        .map(PathBuf::from);
    let mut log_level = std::env::var("NC_GAME_LOG_LEVEL")
        .ok()
        .or_else(|| std::env::var("EC_GAME_LOG_LEVEL").ok())
        .map(|value| nc_log::LogLevel::parse(&value))
        .transpose()?
        .unwrap_or(nc_log::LogLevel::Info);
    let mut encoding = OutputEncoding::Utf8;
    let mut explicit_color_mode: Option<ColorMode> = None;
    let mut dropfile_path: Option<PathBuf> = None;
    let mut explicit_timeout_minutes: Option<u32> = None;
    let mut explicit_socket_descriptor: Option<u64> = None;
    let mut explicit_socket_port: Option<u16> = None;
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --dir".into());
                };
                dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--player" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --player".into());
                };
                player = Some(value.parse::<usize>()?);
                idx += 2;
            }
            "--export-root" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --export-root".into());
                };
                export_root = Some(PathBuf::from(value));
                idx += 2;
            }
            "--queue-dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --queue-dir".into());
                };
                queue_dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--log-file" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --log-file".into());
                };
                log_file = Some(PathBuf::from(value));
                idx += 2;
            }
            "--log-level" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "missing value for --log-level (error, warn, info, debug, or trace)".into(),
                    );
                };
                log_level = nc_log::LogLevel::parse(value)?;
                idx += 2;
            }
            "--encoding" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --encoding (utf8 or cp437)".into());
                };
                encoding = match value.as_str() {
                    "utf8" | "utf-8" => OutputEncoding::Utf8,
                    "cp437" | "CP437" => OutputEncoding::Cp437,
                    other => {
                        return Err(
                            format!("unknown encoding '{other}'; expected utf8 or cp437").into(),
                        );
                    }
                };
                idx += 2;
            }
            "--color-mode" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "missing value for --color-mode (ansi16, 256, truecolor, or auto)".into(),
                    );
                };
                explicit_color_mode = Some(match value.as_str() {
                    "ansi16" | "ansi-16" | "16" => ColorMode::Ansi16,
                    "256" | "color256" | "color-256" => ColorMode::Color256,
                    "truecolor" | "true-color" | "24bit" | "24-bit" => ColorMode::TrueColor,
                    "auto" => {
                        // Treat "auto" as an explicit no-op: use detection below.
                        explicit_color_mode = None;
                        idx += 2;
                        continue;
                    }
                    other => {
                        return Err(format!(
                            "unknown color mode '{other}'; expected ansi16, 256, truecolor, or auto"
                        )
                        .into());
                    }
                });
                idx += 2;
            }
            "--dropfile" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --dropfile".into());
                };
                dropfile_path = Some(PathBuf::from(value));
                idx += 2;
            }
            "--timeout" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --timeout (minutes)".into());
                };
                let minutes = value.parse::<u32>().map_err(|_| {
                    format!("--timeout value must be a positive integer, got '{value}'")
                })?;
                explicit_timeout_minutes = Some(minutes);
                idx += 2;
            }
            "--socket-descriptor" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --socket-descriptor".into());
                };
                explicit_socket_descriptor = Some(value.parse::<u64>().map_err(|_| {
                    format!("--socket-descriptor value must be an unsigned integer, got '{value}'")
                })?);
                idx += 2;
            }
            "--socket-port" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --socket-port".into());
                };
                let port = value.parse::<u16>().map_err(|_| {
                    format!("--socket-port value must be an integer from 1 to 65535, got '{value}'")
                })?;
                if port == 0 {
                    return Err("--socket-port must be between 1 and 65535".into());
                }
                explicit_socket_port = Some(port);
                idx += 2;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
    }

    // Parse dropfile and apply any fields not already set by explicit flags.
    let mut dropfile_alias: Option<String> = None;
    let mut dropfile_fallback_alias: Option<String> = None;
    let mut dropfile_alias_is_real_name = false;
    let mut dropfile_timeout_minutes: Option<u32> = None;
    let mut screen_geometry = ScreenGeometry::local_default();
    let mut use_door_terminal = false;
    let mut door_transport = DoorTransport::Stdio;

    if let Some(path) = &dropfile_path {
        let info = dropfile::parse(path).map_err(|e| format!("{e}"))?;
        door_transport =
            select_door_transport(explicit_socket_descriptor, explicit_socket_port, &info)?;
        dropfile_alias = info
            .alias
            .map(|alias| alias.trim().to_string())
            .filter(|alias| !alias.is_empty());
        dropfile_fallback_alias = info.fallback_alias;
        dropfile_alias_is_real_name = info.alias_is_real_name;
        dropfile_timeout_minutes = info.timeout_minutes;
        screen_geometry = ScreenGeometry::for_door(info.screen_rows);
        use_door_terminal = true;
    } else if let Some(transport) =
        explicit_door_transport(explicit_socket_descriptor, explicit_socket_port)?
    {
        door_transport = transport;
        use_door_terminal = true;
    }

    // If a dropfile was given without an explicit --encoding, default to cp437.
    if dropfile_path.is_some() && !args.iter().any(|a| a == "--encoding") {
        encoding = OutputEncoding::Cp437;
    }

    let dir = dir.ok_or("usage: nc-game --dir <game_dir> [--player <1-based empire index>]")?;

    if matches!(player, Some(0)) {
        return Err("--player must be >= 1".into());
    }

    // Explicit --timeout overrides the dropfile value.
    let timeout_minutes = explicit_timeout_minutes.or(dropfile_timeout_minutes);
    let session_timeout_secs = timeout_minutes.map(|m| m.saturating_mul(60));

    // Resolve color mode: explicit flag > cp437-default > env-based detection.
    let color_mode = explicit_color_mode.unwrap_or_else(|| {
        if encoding == OutputEncoding::Cp437 {
            // BBS/door clients: assume classic 16-color unless explicitly overridden.
            ColorMode::Ansi16
        } else {
            detect_color_mode()
        }
    });

    Ok(ParsedLaunchArgs {
        game_dir: dir,
        explicit_player_record_index_1_based: player,
        export_root,
        queue_dir,
        log_file,
        log_level,
        encoding,
        color_mode,
        screen_geometry,
        dropfile_alias,
        dropfile_fallback_alias,
        dropfile_alias_is_real_name,
        session_timeout_secs,
        use_door_terminal,
        door_transport,
    })
}

fn select_door_transport(
    explicit_socket_descriptor: Option<u64>,
    explicit_socket_port: Option<u16>,
    _info: &dropfile::DropfileInfo,
) -> Result<DoorTransport, Box<dyn std::error::Error>> {
    if let Some(transport) =
        explicit_door_transport(explicit_socket_descriptor, explicit_socket_port)?
    {
        return Ok(transport);
    }

    #[cfg(windows)]
    {
        if matches!(
            _info.connection_type,
            Some(dropfile::DoorConnectionType::TelnetSocket)
        ) {
            let Some(descriptor) = _info.socket_descriptor else {
                return Ok(DoorTransport::Stdio);
            };
            return Ok(DoorTransport::SocketDescriptor { descriptor });
        }
    }

    Ok(DoorTransport::Stdio)
}

fn explicit_door_transport(
    explicit_socket_descriptor: Option<u64>,
    explicit_socket_port: Option<u16>,
) -> Result<Option<DoorTransport>, Box<dyn std::error::Error>> {
    if explicit_socket_descriptor.is_some() && explicit_socket_port.is_some() {
        return Err("--socket-descriptor and --socket-port cannot be used together".into());
    }

    #[cfg(windows)]
    if let Some(descriptor) = explicit_socket_descriptor {
        return Ok(Some(DoorTransport::SocketDescriptor { descriptor }));
    }

    #[cfg(not(windows))]
    if explicit_socket_descriptor.is_some() {
        return Err("--socket-descriptor is only supported on Windows".into());
    }

    if let Some(port) = explicit_socket_port {
        return Ok(Some(DoorTransport::TcpConnect {
            host: LOOPBACK_DOOR_HOST,
            port,
        }));
    }

    Ok(None)
}

/// Detect the terminal's color depth from standard environment variables.
///
/// Detection order:
/// 1. `COLORTERM=truecolor` or `COLORTERM=24bit` → [`ColorMode::TrueColor`]
/// 2. known modern truecolor `TERM` values       → [`ColorMode::TrueColor`]
/// 3. `TERM` containing `256color`               → [`ColorMode::Color256`]
/// 4. any non-empty, non-`dumb` `TERM`           → [`ColorMode::Color256`]
/// 5. fallback                                   → [`ColorMode::Ansi16`]
///
/// The 16-color default is reserved for BBS/door mode or genuinely minimal
/// terminals. For normal local/SSH play, an interactive terminal should get at
/// least 256-color output unless it explicitly identifies as `dumb`.
pub fn detect_color_mode() -> ColorMode {
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        let ct = colorterm.to_ascii_lowercase();
        if ct == "truecolor" || ct == "24bit" {
            return ColorMode::TrueColor;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        let term = term.to_ascii_lowercase();
        if [
            "kitty",
            "wezterm",
            "ghostty",
            "alacritty",
            "foot",
            "contour",
            "rio",
        ]
        .iter()
        .any(|needle| term.contains(needle))
        {
            return ColorMode::TrueColor;
        }
        if term.contains("256color") {
            return ColorMode::Color256;
        }
        if !term.trim().is_empty() && term != "dumb" {
            return ColorMode::Color256;
        }
    }
    ColorMode::Ansi16
}

fn print_usage() {
    println!("Usage:");
    println!(
        "  nc-game --dir <game_dir> [--player <1-based empire index>] \
             [--encoding <utf8|cp437>] [--color-mode <ansi16|256|truecolor|auto>] \
         [--dropfile <path>] [--timeout <minutes>] [--socket-descriptor <value>] [--socket-port <port>] \
         [--export-root <dir>] [--queue-dir <dir>] \
         [--log-file <path>] [--log-level <error|warn|info|debug|trace>]"
    );
    println!(
        "  nc-game submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>"
    );
    println!();
    println!("BBS door flags:");
    println!("  --dropfile <path>   Parse a BBS dropfile (DOOR32.SYS, DOOR.SYS, or CHAIN.TXT).");
    println!("                      Supplies alias and timeout; explicit flags always override.");
    println!("                      Defaults encoding to cp437 when no --encoding is given.");
    println!("                      Reserved aliases in ncgame.db can omit --player.");
    println!("  --timeout <minutes> Session time limit in minutes.");
    println!("  --socket-descriptor <value>");
    println!("                      Override the socket descriptor for native Windows doors.");
    println!("  --socket-port <port>");
    println!(
        "                      Connect back to a localhost door socket (ENiGMA abracadabra mode)."
    );
    println!();
    println!("Logging:");
    println!("  --log-file <path>   Append diagnostic logs to a text file.");
    println!("  --log-level <lvl>   error, warn, info, debug, or trace (default: info).");
    println!();
    println!("Color modes:");
    println!("  ansi16     Classic 16-color ANSI (BBS doors, legacy terminals)");
    println!("  256        256-color xterm palette");
    println!("  truecolor  24-bit RGB (default for modern local/SSH terminals)");
    println!("  auto       Detect from COLORTERM/TERM environment variables (default)");
}

fn load_runtime_game_config(
    game_dir: &std::path::Path,
    campaign_store: &CampaignStore,
) -> Result<RuntimeConfig, Box<dyn std::error::Error>> {
    let config_path = game_dir.join("config.kdl");
    if config_path.exists() {
        let bbs_config = BbsGameConfig::load_kdl(&config_path)?;
        return Ok(RuntimeConfig {
            game_name: humanize_slug(&slug_from_dir(game_dir)?),
            theme: None,
            reservations: bbs_config.reservations,
            setup_overrides: RuntimeSetupOverrides::default(),
        });
    }

    let settings = campaign_store.load_campaign_settings()?;
    Ok(RuntimeConfig {
        game_name: settings.game_name,
        theme: Some(PathBuf::from(format!(
            "themes/{}.kdl",
            settings.default_theme_key
        ))),
        reservations: settings.reservations,
        setup_overrides: RuntimeSetupOverrides {
            snoop_enabled: Some(settings.snoop_enabled),
            session_max_idle_minutes: Some(settings.session_max_idle_minutes),
            session_minimum_time_minutes: Some(settings.session_minimum_time_minutes),
            session_local_timeout: Some(settings.session_local_timeout),
            session_remote_timeout: Some(settings.session_remote_timeout),
            inactivity_purge_after_turns: Some(settings.inactivity_purge_after_turns),
            inactivity_autopilot_after_turns: Some(settings.inactivity_autopilot_after_turns),
        },
    })
}

/// Bind the caller to a seat, returning the binding, the caller alias it
/// used, and whether that alias is the handle or the real name. The dropfile
/// alias is tried first; when it binds no seat, the fallback alias (the
/// DOOR.SYS real name) is tried against reservations and seats bound by real
/// name, so seats bound under the real name before the handle was read keep
/// working.
fn resolve_launch_player_binding(
    parsed: &ParsedLaunchArgs,
    game_config: &RuntimeConfig,
    campaign_store: &CampaignStore,
) -> Result<(LaunchPlayerBinding, Option<String>, SeatBindSource), Box<dyn std::error::Error>> {
    let runtime_state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    let resolve = |dropfile_alias: Option<&str>, skip_stored_handle_seats: &[usize]| {
        shared_resolve_launch_player_binding(LaunchBindingRequest {
            explicit_player_record_index_1_based: parsed.explicit_player_record_index_1_based,
            dropfile_alias,
            skip_stored_handle_seats,
            use_door_terminal: parsed.use_door_terminal,
            reservations: &game_config.reservations,
            game_data: &runtime_state.game_data,
        })
    };
    let binding = resolve(parsed.dropfile_alias.as_deref(), &[])?;
    if !binds_by_alias(binding) {
        if let Some(fallback) = parsed.dropfile_fallback_alias.as_deref() {
            let handle_bound_seats = campaign_store.handle_bound_seats()?;
            let fallback_binding = resolve(Some(fallback), &handle_bound_seats)?;
            if binds_by_alias(fallback_binding) {
                return Ok((
                    fallback_binding,
                    Some(fallback.to_string()),
                    SeatBindSource::RealName,
                ));
            }
        }
    }
    let source = if parsed.dropfile_alias_is_real_name {
        SeatBindSource::RealName
    } else {
        SeatBindSource::Handle
    };
    Ok((binding, parsed.dropfile_alias.clone(), source))
}

/// The caller's BBS handle, when the dropfile supplied one.
fn caller_handle(parsed: &ParsedLaunchArgs) -> Option<&str> {
    parsed
        .dropfile_alias
        .as_deref()
        .filter(|_| !parsed.dropfile_alias_is_real_name)
}

fn binds_by_alias(binding: LaunchPlayerBinding) -> bool {
    matches!(
        binding.source(),
        Some(LaunchPlayerBindingSource::ReservedAlias | LaunchPlayerBindingSource::StoredHandle)
    )
}

fn validate_runtime_game_config(
    campaign_store: &CampaignStore,
    game_config: &RuntimeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if game_config.reservations.is_empty() {
        return Ok(());
    }
    let runtime_state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    game_config
        .validate_reservations_for_player_count(runtime_state.game_data.player.records.len())?;
    Ok(())
}

fn slug_from_dir(dir: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let slug = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("cannot derive slug from {}", dir.display()))?
        .to_string();
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(format!(
            "game slug '{}' must use only lowercase ascii letters, digits, and dashes",
            slug
        )
        .into());
    }
    Ok(slug)
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.extend(first.to_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        LOOPBACK_DOOR_HOST, LaunchPlayerBinding, LaunchPlayerBindingSource, ParsedLaunchArgs,
        apply_launch_context, format_door_startup_error, local_exit_lines, prepare_launch,
        resolve_launch_player_binding, run_interactive, run_interactive_inner,
        select_door_transport, should_emit_local_exit_attribution,
    };
    use crate::app::{App, AppConfig, RuntimeConfig};
    use crate::domains::fleet::state::FleetCommandContext;
    use crate::screen::ScreenGeometry;
    use crate::screen::ScreenId;
    use crate::terminal::door::{DoorTimeout, DoorTransport};
    use crate::terminal::{ColorMode, OutputEncoding, Terminal};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use nc_compat::import_directory_snapshot;
    use nc_data::{BbsGameConfig, CampaignStore, CoreGameData, SeatBindSource, SeatReservation};
    use std::collections::VecDeque;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        prior: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let prior = std::env::var_os(key);
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            Self { key, prior }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match self.prior.as_ref() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
        fs::create_dir_all(dst).expect("create temp dir");
        for entry in fs::read_dir(src).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let target = dst.join(entry.file_name());
            if path.is_dir() {
                copy_dir_all(&path, &target);
            } else {
                fs::copy(&path, &target).expect("copy file");
            }
        }
    }

    fn temp_first_time_game_copy() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "nc-game-cli-test-{}-{}-{}",
            std::process::id(),
            TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time ok")
                .as_nanos()
        ));
        copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
        let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
        import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
        root
    }

    fn write_reserved_config(root: &std::path::Path, alias: &str, player: usize) {
        BbsGameConfig {
            players: 4,
            reservations: vec![SeatReservation {
                player_record_index_1_based: player,
                alias: alias.to_string(),
            }],
        }
        .save_kdl(&root.join("config.kdl"))
        .expect("write BBS config");
    }

    fn seed_joined_player_handle(root: &std::path::Path, player: usize, alias: &str) {
        let mut data = CoreGameData::load(root).expect("load fixture");
        data.join_player(player, &format!("Empire {player}"))
            .expect("join player");
        data.player.records[player - 1].set_assigned_player_handle_raw(alias);
        data.save(root).expect("save fixture");
        let store = CampaignStore::open_default_in_dir(root).expect("open campaign store");
        import_directory_snapshot(&store, root).expect("refresh sqlite snapshot");
    }

    fn parsed_args(use_door_terminal: bool) -> ParsedLaunchArgs {
        ParsedLaunchArgs {
            game_dir: PathBuf::from("/tmp/test"),
            explicit_player_record_index_1_based: Some(1),
            export_root: None,
            queue_dir: None,
            log_file: None,
            log_level: nc_log::LogLevel::Info,
            encoding: OutputEncoding::Utf8,
            color_mode: ColorMode::TrueColor,
            screen_geometry: ScreenGeometry::local_default(),
            dropfile_alias: None,
            dropfile_fallback_alias: None,
            dropfile_alias_is_real_name: false,
            session_timeout_secs: None,
            use_door_terminal,
            door_transport: DoorTransport::Stdio,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    struct ScriptedTerminal {
        keys: VecDeque<KeyEvent>,
        frames: Vec<Vec<String>>,
        captures: Vec<String>,
        timeout_after_keys: Option<DoorTimeout>,
        output_hung_up: bool,
        render_error: Option<io::ErrorKind>,
    }

    impl ScriptedTerminal {
        fn new(keys: impl IntoIterator<Item = KeyEvent>) -> Self {
            Self {
                keys: keys.into_iter().collect(),
                frames: Vec::new(),
                captures: Vec::new(),
                timeout_after_keys: None,
                output_hung_up: false,
                render_error: None,
            }
        }
    }

    impl Terminal for ScriptedTerminal {
        fn render(
            &mut self,
            playfield: &crate::screen::PlayfieldBuffer,
        ) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(kind) = self.render_error {
                return Err(Box::new(io::Error::new(kind, "render to dropped carrier")));
            }
            self.frames.push(
                (0..playfield.height())
                    .map(|row| playfield.plain_line(row))
                    .collect(),
            );
            Ok(())
        }

        fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
            self.captures.push(text.to_string());
            Ok(())
        }

        fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
            self.keys
                .pop_front()
                .ok_or_else(|| match self.timeout_after_keys {
                    Some(timeout) => Box::new(timeout) as Box<dyn std::error::Error>,
                    None => Box::new(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "end of scripted keys",
                    )) as Box<dyn std::error::Error>,
                })
        }

        fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            if self.output_hung_up {
                return Err(Box::new(io::Error::other("write to hung-up terminal")));
            }
            Ok(())
        }
    }

    #[test]
    fn parse_args_uses_env_log_defaults_when_present() {
        let _lock = env_lock().lock().expect("env lock");
        let _file = EnvVarGuard::set("NC_GAME_LOG_FILE", Some("/tmp/nc-game-env.log"));
        let _level = EnvVarGuard::set("NC_GAME_LOG_LEVEL", Some("debug"));

        let parsed = super::parse_args(&[
            "nc-game".to_string(),
            "--dir".to_string(),
            "/tmp/test".to_string(),
            "--player".to_string(),
            "1".to_string(),
        ])
        .expect("parse args");

        assert_eq!(parsed.log_file, Some(PathBuf::from("/tmp/nc-game-env.log")));
        assert_eq!(parsed.log_level, nc_log::LogLevel::Debug);
    }

    #[test]
    fn resolve_launch_binding_uses_stored_handle_for_returning_dropfile_caller() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "RIVAL");
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.clone();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("rival".to_string());

        let (binding, _, _) =
            resolve_launch_player_binding(&parsed, &RuntimeConfig::default(), &campaign_store)
                .expect("binding should resolve");

        assert_eq!(
            binding,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 2,
                source: LaunchPlayerBindingSource::StoredHandle,
            }
        );

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn resolve_launch_binding_uses_unbound_dropfile_for_new_bbs_caller() {
        let game_dir = temp_first_time_game_copy();
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.clone();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("RIVAL".to_string());

        let (binding, _, _) =
            resolve_launch_player_binding(&parsed, &RuntimeConfig::default(), &campaign_store)
                .expect("binding should resolve");

        assert_eq!(binding, LaunchPlayerBinding::UnboundDropfile);

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn resolve_launch_binding_rejects_stored_handle_on_other_players_reserved_seat() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "RIVAL");
        write_reserved_config(&game_dir, "SYSOP", 2);
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.clone();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("RIVAL".to_string());

        let err = resolve_launch_player_binding(
            &parsed,
            &RuntimeConfig {
                reservations: vec![SeatReservation {
                    player_record_index_1_based: 2,
                    alias: "SYSOP".to_string(),
                }],
                ..RuntimeConfig::default()
            },
            &campaign_store,
        )
        .expect_err("binding should reject reservation conflict");

        assert!(
            err.to_string()
                .contains("conflicts with reserved alias 'SYSOP' for seat 2")
        );

        let _ = fs::remove_dir_all(&game_dir);
    }

    /// A DOOR.SYS caller with handle `Starlord` on line 36 and real name
    /// `Tachyon` on line 10.
    fn door_sys_caller(game_dir: &std::path::Path) -> ParsedLaunchArgs {
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.to_path_buf();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("Starlord".to_string());
        parsed.dropfile_fallback_alias = Some("Tachyon".to_string());
        parsed
    }

    #[test]
    fn door_sys_seat_bound_under_the_real_name_still_resolves() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "Tachyon");
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");

        let (binding, caller_alias, _) = resolve_launch_player_binding(
            &door_sys_caller(&game_dir),
            &RuntimeConfig::default(),
            &campaign_store,
        )
        .expect("binding should resolve");

        assert_eq!(
            binding,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 2,
                source: LaunchPlayerBindingSource::StoredHandle,
            }
        );
        assert_eq!(caller_alias.as_deref(), Some("Tachyon"));

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn door_sys_seat_bound_under_the_real_name_is_rebound_to_the_handle() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "Tachyon");
        BbsGameConfig {
            players: 4,
            reservations: Vec::new(),
        }
        .save_kdl(&game_dir.join("config.kdl"))
        .expect("write BBS config");
        let app = prepare_launch(&door_sys_caller(&game_dir)).expect("launch should prepare");

        assert_eq!(app.player.record_index_1_based, 2);
        assert_eq!(app.startup_state.caller_alias.as_deref(), Some("Starlord"));
        assert_eq!(
            app.startup_state.caller_alias_source,
            SeatBindSource::Handle
        );
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let runtime_state = campaign_store
            .load_latest_runtime_state()
            .expect("load runtime state")
            .expect("runtime state");
        assert_eq!(
            runtime_state.game_data.player.records[1].assigned_player_handle_summary(),
            "Starlord"
        );
        assert_eq!(
            campaign_store
                .handle_bound_seats()
                .expect("handle-bound seats"),
            vec![2]
        );

        let (binding, caller_alias, caller_alias_source) = resolve_launch_player_binding(
            &door_sys_caller(&game_dir),
            &RuntimeConfig::default(),
            &campaign_store,
        )
        .expect("binding should resolve");
        assert_eq!(
            binding,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 2,
                source: LaunchPlayerBindingSource::StoredHandle,
            }
        );
        assert_eq!(caller_alias.as_deref(), Some("Starlord"));
        assert_eq!(caller_alias_source, SeatBindSource::Handle);

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn door_sys_real_name_does_not_bind_a_seat_bound_by_handle() {
        let game_dir = temp_first_time_game_copy();
        // Another player's handle equals this caller's real name.
        seed_joined_player_handle(&game_dir, 2, "Tachyon");
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        campaign_store
            .set_seat_bind_source(2, SeatBindSource::Handle)
            .expect("record handle binding");

        let (binding, caller_alias, caller_alias_source) = resolve_launch_player_binding(
            &door_sys_caller(&game_dir),
            &RuntimeConfig::default(),
            &campaign_store,
        )
        .expect("binding should resolve");

        assert_eq!(binding, LaunchPlayerBinding::UnboundDropfile);
        assert_eq!(caller_alias.as_deref(), Some("Starlord"));
        assert_eq!(caller_alias_source, SeatBindSource::Handle);

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn door_sys_seat_reserved_under_the_real_name_still_resolves() {
        let game_dir = temp_first_time_game_copy();
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");

        let (binding, caller_alias, _) = resolve_launch_player_binding(
            &door_sys_caller(&game_dir),
            &RuntimeConfig {
                reservations: vec![SeatReservation {
                    player_record_index_1_based: 3,
                    alias: "TACHYON".to_string(),
                }],
                ..RuntimeConfig::default()
            },
            &campaign_store,
        )
        .expect("binding should resolve");

        assert_eq!(
            binding,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 3,
                source: LaunchPlayerBindingSource::ReservedAlias,
            }
        );
        assert_eq!(caller_alias.as_deref(), Some("Tachyon"));

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn door_sys_handle_binds_before_the_real_name() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "Tachyon");
        seed_joined_player_handle(&game_dir, 3, "Starlord");
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");

        let (binding, caller_alias, _) = resolve_launch_player_binding(
            &door_sys_caller(&game_dir),
            &RuntimeConfig::default(),
            &campaign_store,
        )
        .expect("binding should resolve");

        assert_eq!(binding.player_record_index_1_based(), Some(3));
        assert_eq!(caller_alias.as_deref(), Some("Starlord"));

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn new_door_sys_caller_joins_under_the_handle() {
        let game_dir = temp_first_time_game_copy();
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");

        let (binding, caller_alias, _) = resolve_launch_player_binding(
            &door_sys_caller(&game_dir),
            &RuntimeConfig::default(),
            &campaign_store,
        )
        .expect("binding should resolve");

        assert_eq!(binding, LaunchPlayerBinding::UnboundDropfile);
        assert_eq!(caller_alias.as_deref(), Some("Starlord"));

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn local_exit_lines_returns_empty() {
        assert_eq!(local_exit_lines(), Vec::<String>::new());
    }

    #[test]
    fn interactive_loop_survives_fleet_list_transfer_host_redraw() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: RuntimeConfig::default(),
        };
        let mut app = App::load(config).expect("app should load");
        app.current_screen = ScreenId::MainMenu;

        let donor = app
            .game_data
            .fleets
            .records
            .iter_mut()
            .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
            .expect("fleet #4 should exist");
        donor.set_current_location_coords_raw([16, 13]);
        donor.set_standing_order_target_coords_raw([16, 13]);
        donor.set_battleship_count(0);
        donor.set_cruiser_count(0);
        donor.set_destroyer_count(0);
        donor.set_troop_transport_count(2);
        donor.set_army_count(0);
        donor.set_scout_count(0);
        donor.set_etac_count(0);
        donor.recompute_max_speed_from_composition();

        let host = app
            .game_data
            .fleets
            .records
            .iter_mut()
            .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
            .expect("fleet #3 should exist");
        host.set_current_location_coords_raw([16, 13]);
        host.set_standing_order_target_coords_raw([16, 13]);
        host.set_battleship_count(0);
        host.set_cruiser_count(0);
        host.set_destroyer_count(1);
        host.set_troop_transport_count(0);
        host.set_army_count(0);
        host.set_scout_count(0);
        host.set_etac_count(0);
        host.recompute_max_speed_from_composition();

        app.open_fleet_menu();
        app.open_fleet_list();
        app.fleet.command_context = FleetCommandContext::List;

        let mut terminal =
            ScriptedTerminal::new([key(KeyCode::Char('t')), key(KeyCode::Char('q'))]);

        run_interactive_inner(&mut app, &mut terminal)
            .expect("interactive loop should stay alive through transfer prompt");

        assert!(
            terminal
                .frames
                .iter()
                .flatten()
                .any(|line| line.contains("Transfer To Fleet #")),
            "{:#?}",
            terminal.frames
        );
        assert_eq!(app.current_screen, ScreenId::FleetList);

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[cfg(unix)]
    #[test]
    fn pty_hangup_eio_counts_as_closed_input() {
        const EIO: i32 = 5;
        let err: Box<dyn std::error::Error> = Box::new(io::Error::from_raw_os_error(EIO));
        assert!(super::is_hangup_error(err.as_ref()));
    }

    #[test]
    fn hangup_exits_cleanly_even_when_the_screen_reset_fails() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: RuntimeConfig::default(),
        };
        let mut app = App::load(config).expect("app should load");
        let mut terminal = ScriptedTerminal::new([]);
        terminal.output_hung_up = true;

        run_interactive(&mut app, &mut terminal, false).expect("a hangup is a normal exit");

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn carrier_drop_during_a_render_exits_cleanly() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: RuntimeConfig::default(),
        };
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::ConnectionReset] {
            let mut app = App::load(config.clone()).expect("app should load");
            let mut terminal = ScriptedTerminal::new([key(KeyCode::Char('x'))]);
            terminal.render_error = Some(kind);
            terminal.output_hung_up = true;

            run_interactive(&mut app, &mut terminal, false).expect("a hangup is a normal exit");
        }

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn door_timeout_ends_the_session_cleanly_with_a_message() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: RuntimeConfig::default(),
        };
        for timeout in [DoorTimeout::SessionLimit, DoorTimeout::Idle] {
            let mut app = App::load(config.clone()).expect("app should load");
            let mut terminal = ScriptedTerminal::new([key(KeyCode::Char('x'))]);
            terminal.timeout_after_keys = Some(timeout);

            run_interactive(&mut app, &mut terminal, false)
                .expect("a timed-out door session is a normal exit");

            assert_eq!(terminal.captures, vec![timeout.to_string()]);
        }

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn interactive_loop_runs_for_door_sessions_even_without_tty_stdout() {
        assert!(super::should_use_interactive_loop(
            &parsed_args(true),
            false
        ));
        assert!(super::should_use_interactive_loop(&parsed_args(true), true));
        assert!(!super::should_use_interactive_loop(
            &parsed_args(false),
            false
        ));
        assert!(super::should_use_interactive_loop(
            &parsed_args(false),
            true
        ));
    }

    #[test]
    fn attribution_only_emits_for_local_interactive_stdout_sessions() {
        assert!(should_emit_local_exit_attribution(
            &parsed_args(false),
            true
        ));
        assert!(!should_emit_local_exit_attribution(
            &parsed_args(true),
            true
        ));
        assert!(!should_emit_local_exit_attribution(
            &parsed_args(false),
            false
        ));
    }

    #[test]
    fn dropfile_transport_defaults_to_stdio_without_socket_metadata() {
        let info = crate::dropfile::DropfileInfo::default();

        assert_eq!(
            select_door_transport(None, None, &info).expect("transport should resolve"),
            DoorTransport::Stdio
        );
    }

    #[test]
    fn explicit_socket_port_uses_localhost_tcp_connect_transport() {
        let info = crate::dropfile::DropfileInfo::default();

        assert_eq!(
            select_door_transport(None, Some(2323), &info).expect("transport should resolve"),
            DoorTransport::TcpConnect {
                host: LOOPBACK_DOOR_HOST,
                port: 2323,
            }
        );
    }

    #[cfg(windows)]
    #[test]
    fn dropfile_transport_uses_socket_descriptor_for_door32_telnet_launches() {
        let info = crate::dropfile::DropfileInfo {
            connection_type: Some(crate::dropfile::DoorConnectionType::TelnetSocket),
            socket_descriptor: Some(77),
            ..crate::dropfile::DropfileInfo::default()
        };

        assert_eq!(
            select_door_transport(None, None, &info).expect("transport should resolve"),
            DoorTransport::SocketDescriptor { descriptor: 77 }
        );
    }

    #[cfg(windows)]
    #[test]
    fn dropfile_transport_falls_back_to_stdio_when_socket_descriptor_is_missing() {
        let info = crate::dropfile::DropfileInfo {
            connection_type: Some(crate::dropfile::DoorConnectionType::TelnetSocket),
            socket_descriptor: None,
            ..crate::dropfile::DropfileInfo::default()
        };

        assert!(matches!(
            select_door_transport(None, None, &info).expect("transport should resolve"),
            DoorTransport::Stdio
        ));
    }

    #[cfg(windows)]
    #[test]
    fn explicit_socket_descriptor_overrides_dropfile_transport_metadata() {
        let info = crate::dropfile::DropfileInfo {
            connection_type: Some(crate::dropfile::DoorConnectionType::TelnetSocket),
            socket_descriptor: Some(77),
            ..crate::dropfile::DropfileInfo::default()
        };

        assert_eq!(
            select_door_transport(Some(1234), None, &info).expect("transport should resolve"),
            DoorTransport::SocketDescriptor { descriptor: 1234 }
        );
    }

    #[test]
    fn explicit_socket_descriptor_and_port_cannot_be_combined() {
        let info = crate::dropfile::DropfileInfo::default();

        let err = select_door_transport(Some(7), Some(2323), &info)
            .expect_err("transport selection should reject conflicting flags");

        assert!(
            err.to_string()
                .contains("--socket-descriptor and --socket-port cannot be used together")
        );
    }

    #[test]
    fn closed_input_errors_are_treated_as_clean_disconnects() {
        assert!(super::is_hangup_error(&std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "eof"
        )));
        assert!(super::is_hangup_error(&std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe ended"
        )));
        assert!(!super::is_hangup_error(&std::io::Error::other("boom")));
    }

    #[test]
    fn apply_launch_context_moves_unbound_dropfile_to_first_time_menu() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: RuntimeConfig::default(),
        };
        let mut app = App::load(config).expect("app should load");
        let parsed = parsed_args(true);

        apply_launch_context(
            &mut app,
            &parsed,
            LaunchPlayerBinding::UnboundDropfile,
            Some("RIVAL".to_string()),
            SeatBindSource::Handle,
        )
        .expect("launch context should apply");

        assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
        assert!(app.startup_state.unbound_bbs_caller);
        assert_eq!(app.startup_state.caller_alias.as_deref(), Some("RIVAL"));

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn schema_mismatch_startup_error_mentions_expected_refresh_and_dismiss() {
        let message =
            format_door_startup_error(&nc_data::CampaignStoreError::SchemaVersionMismatch {
                expected: 12,
                found: Some(7),
            });

        assert!(message.contains("This game directory does not match this build."));
        assert!(message.contains("Found runtime schema 7, but this build expects 12."));
        assert!(message.contains("Recreate or refresh ncgame.db before launching again."));
        assert!(message.contains("(slap a key)"));
    }
}
