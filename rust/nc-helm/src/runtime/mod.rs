mod primitives;
mod renderer;

use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use tracing::debug;
use winit::application::ApplicationHandler;
use winit::event::{Ime, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{
    ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy,
};
use winit::keyboard::ModifiersState;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::startup_notify::{
    EventLoopExtStartupNotify, WindowAttributesExtStartupNotify, reset_activation_token_env,
};
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::wayland::{
    EventLoopBuilderExtWayland, EventLoopExtWayland, WindowAttributesExtWayland,
};
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{Fullscreen, Window, WindowAttributes};

use nc_client::hosted::store::{HostedDraftStatus, HostedStateStore};

use crate::Point;
use crate::app::{
    App, Effect, MATRIX_FRAME_STEP, MIN_SUPPORTED_GEOMETRY, Msg, Route, route_supports_session_lock,
};
use crate::dashboard;
use crate::geometry;
use crate::input::{
    MouseButton, MouseEvent, MouseEventKind, key_event_from_winit, key_modifiers_from_winit,
};
use crate::startup::{
    LaunchTargetOptions, LocalLaunchOptions, NativeBackendPreference, NativeLaunchOptions,
    NativeWindowMode,
};
use crate::storage::{BootSnapshot, StorageActor, StoredSession};
use crate::transport::{
    HostedGameOpenResult, HostedGameOpenSuccess, LobbySnapshot, SandboxJoinResult,
    SandboxReleaseSuccess, TransportActor,
};

const FULL_MAP_FIT_SCALE_FLOOR: f64 = 0.65;

pub fn run(options: LaunchTargetOptions) -> Result<(), Box<dyn std::error::Error>> {
    let (app, effects) = App::new(options.relay_override.clone());
    run_with_app(options.native, app, effects)
}

pub fn run_local(options: LocalLaunchOptions) -> Result<(), Box<dyn std::error::Error>> {
    let (app, effects) = App::new_local_dashboard(options.game_dir)?;
    run_with_app(options.native, app, effects)
}

fn run_with_app(
    native_options: NativeLaunchOptions,
    app: App,
    effects: Vec<Effect>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = EventLoop::<RuntimeEvent>::with_user_event();
    apply_backend_preference(&mut builder, native_options.backend_preference);
    let event_loop = builder.build()?;
    let session_backend = detect_session_backend(&event_loop, native_options.backend_preference);
    let proxy = event_loop.create_proxy();
    let storage = StorageActor::start().map_err(|err| format!("storage init failed: {err}"))?;
    let transport = TransportActor::start();
    let mut runtime = Runtime::new(
        native_options,
        session_backend,
        proxy,
        app,
        storage,
        transport,
        effects,
    );
    event_loop.run_app(&mut runtime)?;
    Ok(())
}

#[derive(Debug, Clone)]
enum RuntimeEvent {
    BootLoaded(Result<BootSnapshot, String>),
    IdentityCreated(Result<StoredSession, String>),
    Unlocked(Result<StoredSession, String>),
    LobbyUpdated(Result<LobbySnapshot, String>),
    LobbyRefreshed(Result<LobbySnapshot, String>),
    SandboxJoined(Result<SandboxJoinResult, String>),
    SandboxReleased(Result<SandboxReleaseSuccess, String>),
    HostedGameOpened(Result<HostedGameOpenResult, String>),
    FirstJoinSetupCompleted(Result<HostedGameOpenSuccess, String>),
    RelaySaved(Result<String, String>),
    HostedTurnDraftSaved(Result<(), String>),
}

#[derive(Debug)]
struct HostedDraftSaveRequest {
    game_id: String,
    player_pubkey: String,
    password: String,
    base_hash: String,
    draft: Option<nc_data::TurnSubmission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionBackend {
    Wayland,
    X11,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResizeObservation {
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResizeShrinkTracker {
    baseline_height: u32,
    consecutive_shrinks: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeVerdict {
    Accept,
    SpuriousShrink { restore_to: (u32, u32) },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameTimingSample {
    view_build: Duration,
    view_cache_hits: usize,
    view_cache_misses: usize,
    hosted_dashboard_render: Duration,
    hosted_convert: Duration,
    hosted_dirty_regions: usize,
    hosted_full_rebuild: bool,
    playfield_prepare: Duration,
    glyph_prepare: Duration,
    gpu_submit_present: Duration,
    total: Duration,
    dirty_rows: usize,
    raw_spans: usize,
    text_rebuild_spans: usize,
    text_rebuild_cells: usize,
    text_buffer_misses: usize,
    upload_rects: usize,
    full_rebuild: bool,
}

#[derive(Debug, Default)]
struct FrameTimingSummary {
    samples: Vec<FrameTimingSample>,
    started_at: Option<Instant>,
}

struct Runtime {
    native_options: NativeLaunchOptions,
    session_backend: SessionBackend,
    proxy: EventLoopProxy<RuntimeEvent>,
    app: App,
    storage: StorageActor,
    transport: TransportActor,
    hosted_draft_saves: mpsc::Sender<HostedDraftSaveRequest>,
    pending_effects: Vec<Effect>,
    window: Option<Arc<Window>>,
    renderer: Option<renderer::Renderer>,
    modifiers: ModifiersState,
    pointer_cell: Option<Point>,
    pending_mouse_motion: Option<MouseEvent>,
    grid_metrics: Option<geometry::GridMetrics>,
    window_pixel_width: u32,
    window_pixel_height: u32,
    last_resize_observation: Option<ResizeObservation>,
    needs_redraw: bool,
    last_user_input: Option<Instant>,
    left_mouse_down: bool,
    pending_wheel_residual: f32,
    shrink_tracker: Option<ResizeShrinkTracker>,
    next_matrix_frame_at: Option<Instant>,
    frame_timings: FrameTimingSummary,
}

impl Runtime {
    fn new(
        native_options: NativeLaunchOptions,
        session_backend: SessionBackend,
        proxy: EventLoopProxy<RuntimeEvent>,
        app: App,
        storage: StorageActor,
        transport: TransportActor,
        pending_effects: Vec<Effect>,
    ) -> Self {
        let hosted_draft_saves = start_hosted_draft_save_actor(proxy.clone());
        Self {
            native_options,
            session_backend,
            proxy,
            app,
            storage,
            transport,
            hosted_draft_saves,
            pending_effects,
            window: None,
            renderer: None,
            modifiers: ModifiersState::empty(),
            pointer_cell: None,
            pending_mouse_motion: None,
            grid_metrics: None,
            window_pixel_width: 0,
            window_pixel_height: 0,
            last_resize_observation: None,
            needs_redraw: true,
            last_user_input: None,
            left_mouse_down: false,
            pending_wheel_residual: 0.0,
            shrink_tracker: None,
            next_matrix_frame_at: None,
            frame_timings: FrameTimingSummary::default(),
        }
    }

    fn create_window(
        &self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Arc<Window>, Box<dyn std::error::Error>> {
        let geometry = self.app.model().geometry;
        let size = geometry::logical_window_size_for_grid(geometry.width(), geometry.height());
        let min_size = minimum_window_size();
        let mut attributes = WindowAttributes::default()
            .with_title("Nostrian Conquest - nc-helm")
            .with_resizable(true)
            .with_decorations(window_decorations_for_session(
                self.session_backend,
                std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
            ))
            .with_inner_size(size)
            .with_min_inner_size(min_size);
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            attributes = attributes.with_name("nc-helm", "nc-helm");
        }
        attributes = window_attributes_for_mode(attributes, self.native_options.window_mode);
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            let token = event_loop.read_token_from_env();
            reset_activation_token_env();
            if let Some(token) = token {
                attributes = attributes.with_activation_token(token);
            }
        }
        Ok(Arc::new(event_loop.create_window(attributes)?))
    }

    fn dispatch(&mut self, msg: Msg, event_loop: &ActiveEventLoop) {
        let msg_label = msg_label(&msg);
        let sync_window_input_state = !matches!(msg, Msg::Resize(_));
        let should_sync_geometry = msg_could_change_desired_geometry(&msg);
        let outcome = self.app.dispatch_with_outcome(msg);
        self.diagnostic_log(&format!(
            "state: msg={} route={} focus={} network={:?}",
            msg_label,
            route_label(&self.app.model().route),
            self.app.model().window_focused,
            self.app.model().network
        ));
        self.pending_effects.extend(outcome.effects);
        self.process_effects(event_loop);
        if self.app.model().session.is_some() && self.last_user_input.is_none() {
            self.last_user_input = Some(Instant::now());
        }
        if !route_uses_mouse(&self.app.model().route) {
            self.pointer_cell = None;
            self.pending_mouse_motion = None;
        }
        if !matches!(self.app.model().route, Route::MatrixLocked) {
            self.next_matrix_frame_at = None;
        }
        if sync_window_input_state {
            if should_sync_geometry {
                self.sync_geometry_for_current_window(event_loop);
            }
            self.sync_window_input_state();
        }
        self.needs_redraw |= outcome.needs_redraw;
        if self.app.model().should_quit {
            event_loop.exit();
        }
    }

    fn process_effects(&mut self, event_loop: &ActiveEventLoop) {
        while let Some(effect) = self.pending_effects.pop() {
            match effect {
                Effect::LoadBoot => {
                    self.diagnostic_log("dispatch effect: LoadBoot");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.load_boot(tx) {
                        self.dispatch(Msg::BootLoaded(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::BootLoaded(result));
                        }
                    });
                }
                Effect::CreateIdentity {
                    handle,
                    password,
                    relay_url,
                } => {
                    self.diagnostic_log("dispatch effect: CreateIdentity");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self
                        .storage
                        .create_identity(handle, password, relay_url, tx)
                    {
                        self.dispatch(Msg::IdentityCreated(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::IdentityCreated(result));
                        }
                    });
                }
                Effect::SaveRelayUrl { relay_url } => {
                    self.diagnostic_log("dispatch effect: SaveRelayUrl");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.save_relay_url(relay_url, tx) {
                        self.dispatch(Msg::RelaySaved(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::RelaySaved(result));
                        }
                    });
                }
                Effect::Unlock { password } => {
                    self.diagnostic_log("dispatch effect: Unlock");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.unlock(password, tx) {
                        self.dispatch(Msg::Unlocked(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::Unlocked(result));
                        }
                    });
                }
                Effect::ConnectTransport {
                    relay_url,
                    nsec,
                    cache,
                } => {
                    self.diagnostic_log("dispatch effect: ConnectTransport");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.connect(relay_url, nsec, cache, tx) {
                        self.dispatch(Msg::LobbyUpdated(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        while let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::LobbyUpdated(result));
                        }
                    });
                }
                Effect::DisconnectTransport => {
                    self.diagnostic_log("dispatch effect: DisconnectTransport");
                    if let Err(err) = self.transport.disconnect() {
                        self.dispatch(Msg::LobbyUpdated(Err(err)), event_loop);
                    }
                }
                Effect::SaveClientCache { cache, password } => {
                    self.diagnostic_log("dispatch effect: SaveClientCache");
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.save_client_cache(cache, password, tx) {
                        self.diagnostic_log(&format!("save client cache failed: {err}"));
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(Err(err)) = rx.recv() {
                            eprintln!("nc-helm cache save failed: {err}");
                        }
                    });
                }
                Effect::SaveLockTimeout {
                    lock_timeout_minutes,
                } => {
                    self.diagnostic_log("dispatch effect: SaveLockTimeout");
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.save_lock_timeout(lock_timeout_minutes, tx) {
                        self.diagnostic_log(&format!("save lock timeout failed: {err}"));
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(Err(err)) = rx.recv() {
                            eprintln!("nc-helm lock-timeout save failed: {err}");
                        }
                    });
                }
                Effect::RefreshLobby => {
                    self.diagnostic_log("dispatch effect: RefreshLobby");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.refresh(tx) {
                        self.dispatch(Msg::LobbyRefreshed(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::LobbyRefreshed(result));
                        }
                    });
                }
                Effect::JoinSandboxGame {
                    row,
                    password,
                    handle,
                } => {
                    self.diagnostic_log("dispatch effect: JoinSandboxGame");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.join_sandbox(row, password, handle, tx) {
                        self.dispatch(Msg::SandboxJoined(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::SandboxJoined(result));
                        }
                    });
                }
                Effect::ReleaseSandboxGame { row } => {
                    self.diagnostic_log("dispatch effect: ReleaseSandboxGame");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.release_sandbox(row, tx) {
                        self.dispatch(Msg::SandboxReleased(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::SandboxReleased(result));
                        }
                    });
                }
                Effect::OpenHostedGame {
                    row,
                    password,
                    handle,
                } => {
                    self.diagnostic_log("dispatch effect: OpenHostedGame");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.open_hosted_game(row, password, handle, tx) {
                        self.dispatch(Msg::HostedGameOpened(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::HostedGameOpened(result));
                        }
                    });
                }
                Effect::SaveHostedTurnDraft {
                    game_id,
                    player_pubkey,
                    password,
                    base_hash,
                    draft,
                } => {
                    self.diagnostic_log("dispatch effect: SaveHostedTurnDraft");
                    if let Err(err) = self.hosted_draft_saves.send(HostedDraftSaveRequest {
                        game_id,
                        player_pubkey,
                        password,
                        base_hash,
                        draft,
                    }) {
                        self.diagnostic_log(&format!("queue hosted turn draft save failed: {err}"));
                    }
                }
                Effect::CompleteFirstJoinSetup {
                    row,
                    empire_name,
                    homeworld_name,
                    password,
                } => {
                    self.diagnostic_log("dispatch effect: CompleteFirstJoinSetup");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.complete_first_join_setup(
                        row,
                        empire_name,
                        homeworld_name,
                        password,
                        tx,
                    ) {
                        self.dispatch(Msg::FirstJoinSetupCompleted(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::FirstJoinSetupCompleted(result));
                        }
                    });
                }
                Effect::Quit => event_loop.exit(),
            }
        }
    }

    fn diagnostic_log(&self, message: &str) {
        if self.native_options.diagnostic_mode {
            eprintln!("nc-helm diagnostic: {message}");
        }
    }

    fn record_frame_timing(
        &mut self,
        view_build: Duration,
        view_cache_hit: bool,
        view_timings: crate::app::ViewRenderTimings,
        render: renderer::RenderTimings,
    ) {
        if !self.native_options.diagnostic_mode {
            return;
        }
        if let Some(message) = self.frame_timings.record(FrameTimingSample {
            view_build,
            view_cache_hits: usize::from(view_cache_hit),
            view_cache_misses: usize::from(!view_cache_hit),
            hosted_dashboard_render: view_timings.hosted_dashboard_render,
            hosted_convert: view_timings.hosted_convert,
            hosted_dirty_regions: view_timings.hosted_dirty_regions,
            hosted_full_rebuild: view_timings.hosted_full_rebuild,
            playfield_prepare: render.playfield_prepare,
            glyph_prepare: render.glyph_prepare,
            gpu_submit_present: render.gpu_submit_present,
            total: render.total,
            dirty_rows: render.dirty_rows,
            raw_spans: render.raw_spans,
            text_rebuild_spans: render.text_rebuild_spans,
            text_rebuild_cells: render.text_rebuild_cells,
            text_buffer_misses: render.text_buffer_misses,
            upload_rects: render.upload_rects,
            full_rebuild: render.full_rebuild,
        }) {
            self.diagnostic_log(&message);
        }
    }

    fn sync_window_input_state(&self) {
        let Some(window) = &self.window else {
            return;
        };
        window.set_ime_allowed(self.app.model().wants_text_input());
        if backend_supports_programmatic_focus(self.session_backend)
            && self.app.model().wants_window_focus()
            && !self.app.model().window_focused
        {
            window.focus_window();
        }
    }

    fn observe_resize(&mut self, observation: ResizeObservation) -> ResizeVerdict {
        let now = Instant::now();
        let input_recency_ms = self
            .last_user_input
            .map(|t| now.duration_since(t).as_millis() as u32);
        let (verdict, new_tracker) = classify_resize(
            &self.last_resize_observation,
            observation,
            &self.shrink_tracker,
            input_recency_ms,
        );
        self.shrink_tracker = Some(new_tracker);
        self.last_resize_observation = Some(observation);
        verdict
    }

    fn diagnostic_resize_event(
        &self,
        label: &str,
        event_width: u32,
        event_height: u32,
        scale_factor: f64,
    ) {
        if !self.native_options.diagnostic_mode {
            return;
        }
        let backend = session_backend_label(self.session_backend);
        if let Some(window) = self.window.as_ref() {
            let inner = window.inner_size();
            let outer = window.outer_size();
            self.diagnostic_log(&format!(
                "event: {label} backend={backend} event={}x{} inner={}x{} outer={}x{} scale={scale_factor:.3}",
                event_width,
                event_height,
                inner.width,
                inner.height,
                outer.width,
                outer.height
            ));
        } else {
            self.diagnostic_log(&format!(
                "event: {label} backend={backend} event={}x{} scale={scale_factor:.3}",
                event_width, event_height
            ));
        }
    }

    fn sync_geometry_from_size(
        &mut self,
        pixel_width: u32,
        pixel_height: u32,
        scale_factor: f64,
        event_loop: &ActiveEventLoop,
    ) {
        self.window_pixel_width = pixel_width;
        self.window_pixel_height = pixel_height;
        let ui_scale_multiplier =
            self.desired_ui_scale_multiplier(pixel_width, pixel_height, scale_factor);
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        renderer.set_ui_scale_multiplier(scale_factor, ui_scale_multiplier);
        let geometry = geometry_for_pixels(
            pixel_width,
            pixel_height,
            scale_factor * ui_scale_multiplier,
        );
        self.grid_metrics = Some(renderer.grid_metrics());
        if self.app.model().geometry == geometry {
            return;
        }
        self.dispatch(Msg::Resize(geometry), event_loop);
    }

    fn sync_geometry_for_current_window(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        self.sync_geometry_from_size(size.width, size.height, window.scale_factor(), event_loop);
    }

    fn desired_ui_scale_multiplier(
        &self,
        pixel_width: u32,
        pixel_height: u32,
        scale_factor: f64,
    ) -> f64 {
        let Some(window) = self.window.as_ref() else {
            return 1.0;
        };
        let Route::HostedGame(hosted) = &self.app.model().route else {
            return 1.0;
        };
        let fit_eligible = window.is_maximized() || window.fullscreen().is_some();
        let preferred = crate::dashboard::layout::preferred_dashboard_frame(&hosted.dashboard);
        full_map_fit_scale_multiplier(
            fit_eligible,
            pixel_width,
            pixel_height,
            scale_factor,
            preferred.width(),
            preferred.height(),
        )
    }

    fn update_pointer_cell(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        if !route_uses_mouse(&self.app.model().route) {
            self.pointer_cell = None;
            return;
        }
        let Some(grid_metrics) = self.grid_metrics else {
            self.pointer_cell = None;
            return;
        };
        self.pointer_cell = map_pointer_cell(
            self.window_pixel_width,
            self.window_pixel_height,
            self.app.model().geometry,
            grid_metrics.cell,
            position,
        );
    }

    fn scheduled_wakeup(&mut self, now: Instant) -> Option<Instant> {
        let idle_deadline = match (
            self.app.model().session.is_some(),
            route_supports_session_lock(&self.app.model().route),
            self.app.model().lock_timeout_minutes,
            self.last_user_input,
        ) {
            (true, true, timeout, Some(last_input)) if timeout != 0 => {
                Some(last_input + Duration::from_secs(u64::from(timeout) * 60))
            }
            _ => None,
        };

        let matrix_deadline = if matches!(self.app.model().route, Route::MatrixLocked) {
            Some(
                *self
                    .next_matrix_frame_at
                    .get_or_insert(now + MATRIX_FRAME_STEP),
            )
        } else {
            self.next_matrix_frame_at = None;
            None
        };

        combine_deadlines(
            combine_deadlines(idle_deadline, matrix_deadline),
            hosted_route_next_wakeup(&self.app.model().route),
        )
    }
}

fn geometry_for_pixels(
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f64,
) -> crate::ScreenGeometry {
    let metrics = geometry::GridMetrics::for_scale(scale_factor);
    crate::ScreenGeometry::new(
        (pixel_width.max(1) as usize / metrics.cell.width_px).max(1),
        (pixel_height.max(1) as usize / metrics.cell.height_px).max(1),
    )
}

fn full_map_fit_scale_multiplier(
    fit_eligible: bool,
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f64,
    preferred_width: usize,
    preferred_height: usize,
) -> f64 {
    if !fit_eligible {
        return 1.0;
    }

    for step in ((FULL_MAP_FIT_SCALE_FLOOR * 100.0) as u32..=100).rev() {
        let candidate = f64::from(step) / 100.0;
        let geometry = geometry_for_pixels(pixel_width, pixel_height, scale_factor * candidate);
        if geometry.width() >= preferred_width && geometry.height() >= preferred_height {
            return candidate.max(FULL_MAP_FIT_SCALE_FLOOR);
        }
    }

    1.0
}

impl ApplicationHandler<RuntimeEvent> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.diagnostic_log(&format!(
                "event: resumed backend={}",
                session_backend_label(self.session_backend)
            ));
            match self.create_window(event_loop) {
                Ok(window) => {
                    let renderer = match renderer::Renderer::new(window.clone(), event_loop) {
                        Ok(renderer) => renderer,
                        Err(err) => {
                            self.dispatch(
                                Msg::BootLoaded(Err(format!("renderer init failed: {err}"))),
                                event_loop,
                            );
                            return;
                        }
                    };
                    self.window = Some(window);
                    self.renderer = Some(renderer);
                    if let Some(window) = self.window.as_ref() {
                        let size = window.inner_size();
                        let scale_factor = window.scale_factor();
                        self.window_pixel_width = size.width;
                        self.window_pixel_height = size.height;
                        let _ = self.observe_resize(ResizeObservation {
                            pixel_width: size.width,
                            pixel_height: size.height,
                            scale_factor,
                        });
                        self.sync_geometry_from_size(
                            size.width,
                            size.height,
                            scale_factor,
                            event_loop,
                        );
                    }
                    self.needs_redraw = true;
                    self.process_effects(event_loop);
                    self.sync_window_input_state();
                }
                Err(err) => {
                    self.dispatch(
                        Msg::BootLoaded(Err(format!("window creation failed: {err}"))),
                        event_loop,
                    );
                }
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent) {
        match event {
            RuntimeEvent::BootLoaded(result) => {
                self.diagnostic_log("event: BootLoaded");
                self.dispatch(Msg::BootLoaded(result), event_loop)
            }
            RuntimeEvent::IdentityCreated(result) => {
                self.diagnostic_log("event: IdentityCreated");
                self.dispatch(Msg::IdentityCreated(result), event_loop)
            }
            RuntimeEvent::Unlocked(result) => {
                self.diagnostic_log("event: Unlocked");
                self.dispatch(Msg::Unlocked(result), event_loop)
            }
            RuntimeEvent::LobbyUpdated(result) => {
                self.diagnostic_log("event: LobbyUpdated");
                self.dispatch(Msg::LobbyUpdated(result), event_loop)
            }
            RuntimeEvent::LobbyRefreshed(result) => {
                self.diagnostic_log("event: LobbyRefreshed");
                self.dispatch(Msg::LobbyRefreshed(result), event_loop)
            }
            RuntimeEvent::SandboxJoined(result) => {
                self.diagnostic_log("event: SandboxJoined");
                self.dispatch(Msg::SandboxJoined(result), event_loop)
            }
            RuntimeEvent::SandboxReleased(result) => {
                self.diagnostic_log("event: SandboxReleased");
                self.dispatch(Msg::SandboxReleased(result), event_loop)
            }
            RuntimeEvent::HostedGameOpened(result) => {
                self.diagnostic_log("event: HostedGameOpened");
                self.dispatch(Msg::HostedGameOpened(result), event_loop)
            }
            RuntimeEvent::FirstJoinSetupCompleted(result) => {
                self.diagnostic_log("event: FirstJoinSetupCompleted");
                self.dispatch(Msg::FirstJoinSetupCompleted(result), event_loop)
            }
            RuntimeEvent::RelaySaved(result) => {
                self.diagnostic_log("event: RelaySaved");
                self.dispatch(Msg::RelaySaved(result), event_loop)
            }
            RuntimeEvent::HostedTurnDraftSaved(result) => match result {
                Ok(()) => self.diagnostic_log("event: HostedTurnDraftSaved"),
                Err(err) => self.diagnostic_log(&format!("save hosted turn draft failed: {err}")),
            },
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.dispatch(
                    Msg::Key(crate::input::KeyEvent::new(
                        crate::input::KeyCode::Char('q'),
                        crate::input::KeyModifiers::ALT,
                    )),
                    event_loop,
                );
            }
            WindowEvent::Focused(focused) => {
                if self.app.model().window_focused != focused {
                    self.dispatch(Msg::FocusChanged(focused), event_loop);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.last_user_input = Some(Instant::now());
                if let Some(key) = key_event_from_winit(&event, self.modifiers) {
                    self.dispatch(Msg::Key(key), event_loop);
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if should_dispatch_text_commit(&text) {
                    self.dispatch(Msg::TextInput(text.to_string()), event_loop);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_enabled = route_uses_mouse(&self.app.model().route);
                let previous_pointer = self.pointer_cell;
                if self.native_options.diagnostic_mode {
                    self.diagnostic_log(&format!(
                        "event: CursorMoved route={} x={:.1} y={:.1} mouse_enabled={mouse_enabled}",
                        route_label(&self.app.model().route),
                        position.x,
                        position.y
                    ));
                }
                self.update_pointer_cell(position);
                if mouse_enabled {
                    if let Some(pointer_cell) = self.pointer_cell {
                        if should_dispatch_pointer_move(previous_pointer, Some(pointer_cell)) {
                            self.last_user_input = Some(Instant::now());
                            store_pending_pointer_motion(
                                &mut self.pending_mouse_motion,
                                MouseEvent {
                                    kind: pointer_move_event_kind(self.left_mouse_down),
                                    position: pointer_cell,
                                    modifiers: key_modifiers_from_winit(self.modifiers),
                                },
                            );
                        }
                    } else {
                        self.pending_mouse_motion = None;
                    }
                } else {
                    self.pointer_cell = None;
                    self.pending_mouse_motion = None;
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if self.native_options.diagnostic_mode {
                    self.diagnostic_log(&format!(
                        "event: CursorLeft route={}",
                        route_label(&self.app.model().route)
                    ));
                }
                self.pointer_cell = None;
                self.pending_mouse_motion = None;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let mouse_enabled = route_uses_mouse(&self.app.model().route);
                debug!(
                    "MouseInput button={:?} pressed={} mouse_enabled={} pointer={:?}",
                    button,
                    state.is_pressed(),
                    mouse_enabled,
                    self.pointer_cell
                );
                if state.is_pressed() && mouse_enabled {
                    self.last_user_input = Some(Instant::now());
                }
                if button == WinitMouseButton::Left {
                    self.left_mouse_down = state.is_pressed();
                }
                let button = match button {
                    WinitMouseButton::Left => Some(MouseButton::Left),
                    WinitMouseButton::Right => Some(MouseButton::Right),
                    WinitMouseButton::Middle => Some(MouseButton::Middle),
                    _ => None,
                };
                if self.native_options.diagnostic_mode {
                    self.diagnostic_log(&format!(
                        "event: MouseInput route={} pressed={} mouse_enabled={mouse_enabled}",
                        route_label(&self.app.model().route),
                        state.is_pressed()
                    ));
                }
                self.pending_mouse_motion = None;
                if mouse_enabled {
                    if let (Some(button), Some(position)) = (button, self.pointer_cell) {
                        self.dispatch(
                            Msg::Mouse(MouseEvent {
                                kind: if state.is_pressed() {
                                    MouseEventKind::Down(button)
                                } else {
                                    MouseEventKind::Up(button)
                                },
                                position,
                                modifiers: key_modifiers_from_winit(self.modifiers),
                            }),
                            event_loop,
                        );
                    }
                } else {
                    self.pointer_cell = None;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let mouse_enabled = route_uses_mouse(&self.app.model().route);
                debug!(
                    "MouseWheel delta={:?} mouse_enabled={} pointer={:?}",
                    delta, mouse_enabled, self.pointer_cell
                );
                if self.native_options.diagnostic_mode {
                    self.diagnostic_log(&format!(
                        "event: MouseWheel route={} mouse_enabled={mouse_enabled} delta={delta:?}",
                        route_label(&self.app.model().route)
                    ));
                }
                if mouse_enabled {
                    if let Some(position) = self.pointer_cell {
                        let lines =
                            mouse_wheel_delta_to_lines(delta, &mut self.pending_wheel_residual);
                        debug!(
                            "MouseWheel lines={} residual={}",
                            lines, self.pending_wheel_residual
                        );
                        if self.native_options.diagnostic_mode {
                            self.diagnostic_log(&format!(
                                "event: MouseWheel lines={lines} residual={}",
                                self.pending_wheel_residual
                            ));
                        }
                        if lines != 0 {
                            self.dispatch(
                                Msg::Mouse(MouseEvent {
                                    kind: MouseEventKind::Scroll { lines },
                                    position,
                                    modifiers: key_modifiers_from_winit(self.modifiers),
                                }),
                                event_loop,
                            );
                        }
                    } else if self.native_options.diagnostic_mode {
                        self.diagnostic_log("event: MouseWheel no pointer_cell");
                    }
                }
            }
            WindowEvent::Resized(size) => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);
                let observation = ResizeObservation {
                    pixel_width: size.width,
                    pixel_height: size.height,
                    scale_factor,
                };
                let verdict = self.observe_resize(observation);
                match verdict {
                    ResizeVerdict::SpuriousShrink { restore_to } => {
                        if self.native_options.diagnostic_mode {
                            self.diagnostic_log(&format!(
                                "event: Resized backend={} spurious=true event={}x{} baseline={}x{}",
                                session_backend_label(self.session_backend),
                                size.width,
                                size.height,
                                restore_to.1,
                                restore_to.0
                            ));
                        }
                        // Counter-configure was tried and cosmic-comp ignored it; min_inner_size
                        // is now the primary defense against sctk-adwaita #67/#68. We still sync
                        // geometry so the app model matches the actual surface.
                        self.sync_geometry_from_size(
                            size.width,
                            size.height,
                            scale_factor,
                            event_loop,
                        );
                    }
                    ResizeVerdict::Accept => {
                        self.diagnostic_resize_event(
                            "Resized",
                            size.width,
                            size.height,
                            scale_factor,
                        );
                        self.sync_geometry_from_size(
                            size.width,
                            size.height,
                            scale_factor,
                            event_loop,
                        );
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    let observation = ResizeObservation {
                        pixel_width: size.width,
                        pixel_height: size.height,
                        scale_factor,
                    };
                    let _ = self.observe_resize(observation);
                    self.diagnostic_resize_event(
                        "ScaleFactorChanged",
                        size.width,
                        size.height,
                        scale_factor,
                    );
                    self.sync_geometry_from_size(size.width, size.height, scale_factor, event_loop);
                }
            }
            WindowEvent::RedrawRequested => {
                self.diagnostic_log(&format!(
                    "event: RedrawRequested route={}",
                    route_label(&self.app.model().route)
                ));
                if let Some(mouse) =
                    take_pending_pointer_motion_before_redraw(&mut self.pending_mouse_motion)
                {
                    self.dispatch(Msg::Mouse(mouse), event_loop);
                }
                if let Some(renderer) = &mut self.renderer {
                    let render_mode = if matches!(self.app.model().route, Route::MatrixLocked) {
                        renderer::RenderMode::MatrixLocked
                    } else {
                        renderer::RenderMode::Grid
                    };
                    let view_started = Instant::now();
                    let (view_cache_hit, view_timings, buffer) =
                        self.app.view_with_cache_hit_and_timings();
                    let view_build = view_started.elapsed();
                    match renderer.render(buffer, render_mode) {
                        Ok(render_timings) => {
                            self.record_frame_timing(
                                view_build,
                                view_cache_hit,
                                view_timings,
                                render_timings,
                            );
                            self.needs_redraw = false;
                        }
                        Err(err) => {
                            eprintln!("nc-helm render error: {err}");
                            event_loop.exit();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        _event_loop.set_control_flow(ControlFlow::Wait);
        let now = Instant::now();
        if matches!(self.app.model().route, Route::MatrixLocked)
            && self
                .next_matrix_frame_at
                .map(|deadline| deadline <= now)
                .unwrap_or(false)
        {
            self.next_matrix_frame_at = Some(now + MATRIX_FRAME_STEP);
            self.dispatch(Msg::MatrixFrame, _event_loop);
        }
        if self.app.model().session.is_some()
            && route_supports_session_lock(&self.app.model().route)
            && self.app.model().lock_timeout_minutes != 0
            && self
                .last_user_input
                .map(|last_input| {
                    now >= last_input
                        + Duration::from_secs(u64::from(self.app.model().lock_timeout_minutes) * 60)
                })
                .unwrap_or(false)
        {
            self.dispatch(Msg::IdleLock, _event_loop);
            self.last_user_input = None;
        }
        if hosted_route_next_wakeup(&self.app.model().route)
            .map(|deadline| deadline <= now)
            .unwrap_or(false)
            && self.app.hosted_on_idle()
        {
            self.needs_redraw = true;
        }
        if let Some(mouse) =
            take_pending_pointer_motion_before_redraw(&mut self.pending_mouse_motion)
        {
            self.dispatch(Msg::Mouse(mouse), _event_loop);
        }
        if self.needs_redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        if let Some(deadline) = self.scheduled_wakeup(Instant::now()) {
            _event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}

/// Returns `true` for messages that may change the desired window geometry or
/// UI scale multiplier (e.g. route transitions into/out of `HostedGame`).
/// High-frequency input events (`Mouse`, `Key`, `MatrixFrame`, etc.) return
/// `false` so the main thread skips the per-dispatch Wayland round-trips in
/// `sync_geometry_for_current_window`.
fn msg_could_change_desired_geometry(msg: &Msg) -> bool {
    matches!(
        msg,
        Msg::HostedGameOpened(_)
            | Msg::FirstJoinSetupCompleted(_)
            | Msg::SandboxJoined(_)
            | Msg::SandboxReleased(_)
            | Msg::BootLoaded(_)
            | Msg::Unlocked(_)
    )
}

fn combine_deadlines(left: Option<Instant>, right: Option<Instant>) -> Option<Instant> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

impl FrameTimingSummary {
    fn record(&mut self, sample: FrameTimingSample) -> Option<String> {
        let now = Instant::now();
        let started_at = *self.started_at.get_or_insert(now);
        self.samples.push(sample);
        if now.duration_since(started_at) < Duration::from_secs(1) && self.samples.len() < 120 {
            return None;
        }
        let frames = self.samples.len();
        let view_ms = percentile_duration_ms(&self.samples, |sample| sample.view_build);
        let prepare_ms = percentile_duration_ms(&self.samples, |sample| sample.playfield_prepare);
        let glyph_ms = percentile_duration_ms(&self.samples, |sample| sample.glyph_prepare);
        let gpu_ms = percentile_duration_ms(&self.samples, |sample| sample.gpu_submit_present);
        let total_ms = percentile_duration_ms(&self.samples, |sample| sample.total);
        let avg_view_cache_hits = self
            .samples
            .iter()
            .map(|sample| sample.view_cache_hits as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_view_cache_misses = self
            .samples
            .iter()
            .map(|sample| sample.view_cache_misses as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_hosted_dashboard_render_ms = self
            .samples
            .iter()
            .map(|sample| sample.hosted_dashboard_render.as_secs_f64() * 1000.0)
            .sum::<f64>()
            / frames as f64;
        let avg_hosted_convert_ms = self
            .samples
            .iter()
            .map(|sample| sample.hosted_convert.as_secs_f64() * 1000.0)
            .sum::<f64>()
            / frames as f64;
        let avg_hosted_dirty_regions = self
            .samples
            .iter()
            .map(|sample| sample.hosted_dirty_regions as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_dirty_rows = self
            .samples
            .iter()
            .map(|sample| sample.dirty_rows as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_raw_spans = self
            .samples
            .iter()
            .map(|sample| sample.raw_spans as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_text_rebuild_spans = self
            .samples
            .iter()
            .map(|sample| sample.text_rebuild_spans as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_text_rebuild_cells = self
            .samples
            .iter()
            .map(|sample| sample.text_rebuild_cells as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_text_buffer_misses = self
            .samples
            .iter()
            .map(|sample| sample.text_buffer_misses as f64)
            .sum::<f64>()
            / frames as f64;
        let avg_upload_rects = self
            .samples
            .iter()
            .map(|sample| sample.upload_rects as f64)
            .sum::<f64>()
            / frames as f64;
        let full_rebuilds = self
            .samples
            .iter()
            .filter(|sample| sample.full_rebuild)
            .count();
        let hosted_full_rebuilds = self
            .samples
            .iter()
            .filter(|sample| sample.hosted_full_rebuild)
            .count();
        self.samples.clear();
        self.started_at = Some(now);
        Some(format!(
            "frame timings [{} frames] total p50={:.2}ms p95={:.2}ms view p50={:.2}ms p95={:.2}ms prepare p50={:.2}ms p95={:.2}ms glyph p50={:.2}ms p95={:.2}ms gpu p50={:.2}ms p95={:.2}ms avg_view_cache_hits={avg_view_cache_hits:.1} avg_view_cache_misses={avg_view_cache_misses:.1} avg_hosted_dashboard_render_ms={avg_hosted_dashboard_render_ms:.2} avg_hosted_convert_ms={avg_hosted_convert_ms:.2} avg_hosted_dirty_regions={avg_hosted_dirty_regions:.1} avg_dirty_rows={avg_dirty_rows:.1} avg_raw_spans={avg_raw_spans:.1} avg_text_rebuild_spans={avg_text_rebuild_spans:.1} avg_text_rebuild_cells={avg_text_rebuild_cells:.1} avg_text_buffer_misses={avg_text_buffer_misses:.1} avg_upload_rects={avg_upload_rects:.1} full_rebuilds={full_rebuilds} hosted_full_rebuilds={hosted_full_rebuilds}",
            frames,
            total_ms.0,
            total_ms.1,
            view_ms.0,
            view_ms.1,
            prepare_ms.0,
            prepare_ms.1,
            glyph_ms.0,
            glyph_ms.1,
            gpu_ms.0,
            gpu_ms.1,
        ))
    }
}

fn percentile_duration_ms(
    samples: &[FrameTimingSample],
    project: impl Fn(&FrameTimingSample) -> Duration,
) -> (f64, f64) {
    let mut values = samples
        .iter()
        .map(|sample| project(sample).as_secs_f64() * 1000.0)
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    (
        percentile_sorted(&values, 0.50),
        percentile_sorted(&values, 0.95),
    )
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index.min(values.len() - 1)]
}

fn mouse_wheel_delta_to_lines(delta: winit::event::MouseScrollDelta, residual: &mut f32) -> i32 {
    const PIXELS_PER_LINE: f32 = 16.0;
    let raw = match delta {
        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
        winit::event::MouseScrollDelta::PixelDelta(physical_position) => {
            *residual += physical_position.y as f32;
            let whole = (*residual / PIXELS_PER_LINE).trunc();
            *residual -= whole * PIXELS_PER_LINE;
            return whole as i32;
        }
    };
    raw as i32
}

fn pointer_move_event_kind(left_mouse_down: bool) -> MouseEventKind {
    if left_mouse_down {
        MouseEventKind::Drag(MouseButton::Left)
    } else {
        MouseEventKind::Moved
    }
}

fn should_dispatch_pointer_move(previous: Option<Point>, next: Option<Point>) -> bool {
    previous != next
}

fn store_pending_pointer_motion(pending: &mut Option<MouseEvent>, mouse: MouseEvent) {
    *pending = Some(mouse);
}

fn take_pending_pointer_motion_before_redraw(
    pending: &mut Option<MouseEvent>,
) -> Option<MouseEvent> {
    pending.take()
}

fn hosted_route_next_wakeup(route: &Route) -> Option<Instant> {
    match route {
        Route::HostedGame(hosted) => dashboard::hosted_next_wakeup(&hosted.dashboard),
        _ => None,
    }
}

fn route_uses_mouse(route: &Route) -> bool {
    matches!(route, Route::Lobby(_) | Route::HostedGame(_))
}

fn window_attributes_for_mode(
    attributes: WindowAttributes,
    window_mode: NativeWindowMode,
) -> WindowAttributes {
    match window_mode {
        NativeWindowMode::Windowed => attributes.with_maximized(true),
        NativeWindowMode::BorderlessFullscreen => {
            attributes.with_fullscreen(Some(Fullscreen::Borderless(None)))
        }
    }
}

fn map_pointer_cell(
    window_pixel_width: u32,
    window_pixel_height: u32,
    geometry: crate::ScreenGeometry,
    cell: geometry::CellMetrics,
    position: winit::dpi::PhysicalPosition<f64>,
) -> Option<Point> {
    geometry::GridMapper::centered(
        window_pixel_width as usize,
        window_pixel_height as usize,
        geometry,
        cell,
    )
    .pixel_to_cell(position)
}

fn backend_supports_programmatic_focus(session_backend: SessionBackend) -> bool {
    session_backend != SessionBackend::Wayland
}

fn classify_resize(
    last_observation: &Option<ResizeObservation>,
    observation: ResizeObservation,
    shrink_tracker: &Option<ResizeShrinkTracker>,
    input_recency_ms: Option<u32>,
) -> (ResizeVerdict, ResizeShrinkTracker) {
    let fresh_tracker = ResizeShrinkTracker {
        baseline_height: observation.pixel_height,
        consecutive_shrinks: 0,
    };
    let Some(last) = *last_observation else {
        return (ResizeVerdict::Accept, fresh_tracker);
    };
    let same_width = observation.pixel_width == last.pixel_width;
    let shrink = observation.pixel_height < last.pixel_height;
    let grow = observation.pixel_height > last.pixel_height;
    let no_recent_input = input_recency_ms.map_or(true, |ms| ms > 250);
    let scale_unchanged = (observation.scale_factor - last.scale_factor).abs() < 0.001;

    // User-driven or compositor-intended change: reset tracker baseline.
    if !same_width || !scale_unchanged || !no_recent_input {
        return (ResizeVerdict::Accept, fresh_tracker);
    }

    // Grow without recent input (e.g. compositor restore): new baseline at the
    // larger height.
    if grow {
        return (ResizeVerdict::Accept, fresh_tracker);
    }

    // No height change.
    if !shrink {
        let unchanged_tracker = shrink_tracker.unwrap_or(ResizeShrinkTracker {
            baseline_height: last.pixel_height,
            consecutive_shrinks: 0,
        });
        return (ResizeVerdict::Accept, unchanged_tracker);
    }

    // same_width && shrink && scale_unchanged && no_recent_input
    // Baseline is sticky during a shrink run; only the first shrink establishes it
    // when no tracker exists.
    let baseline = shrink_tracker
        .map(|t| t.baseline_height)
        .unwrap_or(last.pixel_height);
    let new_shrinks = shrink_tracker
        .map(|t| t.consecutive_shrinks)
        .unwrap_or(0)
        .saturating_add(1);
    let new_tracker = ResizeShrinkTracker {
        baseline_height: baseline,
        consecutive_shrinks: new_shrinks,
    };
    if new_shrinks >= 2 && observation.pixel_height <= baseline {
        (
            ResizeVerdict::SpuriousShrink {
                restore_to: (baseline, last.pixel_width),
            },
            new_tracker,
        )
    } else {
        (ResizeVerdict::Accept, new_tracker)
    }
}

fn start_hosted_draft_save_actor(
    proxy: EventLoopProxy<RuntimeEvent>,
) -> mpsc::Sender<HostedDraftSaveRequest> {
    let (tx, rx) = mpsc::channel::<HostedDraftSaveRequest>();
    thread::spawn(move || {
        while let Ok(request) = rx.recv() {
            let result = save_hosted_turn_draft(request).map_err(|err| err.to_string());
            let _ = proxy.send_event(RuntimeEvent::HostedTurnDraftSaved(result));
        }
    });
    tx
}

fn save_hosted_turn_draft(
    request: HostedDraftSaveRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = HostedStateStore::open_default()?;
    if let Some(draft) = request.draft {
        store.save_draft(
            &request.password,
            &request.player_pubkey,
            &request.game_id,
            &request.base_hash,
            &draft,
            HostedDraftStatus::Local,
            None,
        )?;
    } else {
        store.clear_draft(&request.player_pubkey, &request.game_id)?;
    }
    Ok(())
}

fn session_backend_label(session_backend: SessionBackend) -> &'static str {
    match session_backend {
        SessionBackend::Wayland => "wayland",
        SessionBackend::X11 => "x11",
        SessionBackend::Unknown => "unknown",
    }
}

/// Decide whether the window should be created with native decorations.
///
/// Cosmic-comp's server-side decorations path has a bug that shrinks the
/// xdg_toplevel by 36 px on every `wl_surface.leave/enter(output)` pair,
/// which fires whenever the pointer crosses the SSD titlebar edge. Requesting
/// `decorations=false` makes winit ask for client-side decorations instead,
/// which cosmic-comp honors and the shrink loop disappears. On every other
/// platform/compositor we keep native decorations so users get a normal
/// titlebar and standard resize/move behavior.
///
/// TODO: remove this workaround once cosmic-comp stops shrinking SSD
/// windows by 36 px on every drag. Tracked upstream as
/// pop-os/cosmic-comp#2300 (follow-up to #1469, which was closed
/// won't-fix but only covered the client-visible leave/enter events,
/// not the SSD geometry recomputation that causes the shrink).
/// COSMIC is currently in alpha/beta; revisit when the compositor stabilizes.
fn window_decorations_for_session(
    session_backend: SessionBackend,
    xdg_current_desktop: Option<&str>,
) -> bool {
    if session_backend != SessionBackend::Wayland {
        return true;
    }
    let is_cosmic = xdg_current_desktop
        .map(|value| {
            value
                .split(':')
                .any(|entry| entry.eq_ignore_ascii_case("COSMIC"))
        })
        .unwrap_or(false);
    !is_cosmic
}

fn detect_session_backend(
    event_loop: &EventLoop<RuntimeEvent>,
    backend_preference: NativeBackendPreference,
) -> SessionBackend {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        if event_loop.is_wayland() {
            SessionBackend::Wayland
        } else if matches!(backend_preference, NativeBackendPreference::X11)
            || std::env::var_os("DISPLAY").is_some()
            || matches!(
                std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
                Some("x11")
            )
        {
            SessionBackend::X11
        } else {
            SessionBackend::Unknown
        }
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    {
        let _ = event_loop;
        let _ = backend_preference;
        SessionBackend::Unknown
    }
}

fn should_dispatch_text_commit(text: &str) -> bool {
    let mut chars = text.chars().filter(|ch| !ch.is_control());
    let Some(first) = chars.next() else {
        return false;
    };
    chars.next().is_some() || !first.is_ascii()
}

fn route_label(route: &Route) -> &'static str {
    match route {
        Route::Boot(_) => "Boot",
        Route::FirstRun(_) => "FirstRun",
        Route::MatrixLocked => "MatrixLocked",
        Route::Locked(_) => "Locked",
        Route::Lobby(_) => "Lobby",
        Route::SandboxJoinConfirm(_) => "SandboxJoinConfirm",
        Route::SandboxJoinUnavailable { .. } => "SandboxJoinUnavailable",
        Route::SandboxDeleteConfirm(_) => "SandboxDeleteConfirm",
        Route::FirstJoinSetup(_) => "FirstJoinSetup",
        Route::HostedGame(_) => "HostedGame",
        Route::FatalError(_) => "FatalError",
    }
}

fn msg_label(msg: &Msg) -> &'static str {
    match msg {
        Msg::Resize(_) => "Resize",
        Msg::FocusChanged(_) => "FocusChanged",
        Msg::MatrixFrame => "MatrixFrame",
        Msg::IdleLock => "IdleLock",
        Msg::Key(_) => "Key",
        Msg::TextInput(_) => "TextInput",
        Msg::Mouse(_) => "Mouse",
        Msg::BootLoaded(_) => "BootLoaded",
        Msg::IdentityCreated(_) => "IdentityCreated",
        Msg::Unlocked(_) => "Unlocked",
        Msg::LobbyUpdated(_) => "LobbyUpdated",
        Msg::LobbyRefreshed(_) => "LobbyRefreshed",
        Msg::SandboxJoined(_) => "SandboxJoined",
        Msg::SandboxReleased(_) => "SandboxReleased",
        Msg::HostedGameOpened(_) => "HostedGameOpened",
        Msg::FirstJoinSetupCompleted(_) => "FirstJoinSetupCompleted",
        Msg::RelaySaved(_) => "RelaySaved",
    }
}

fn apply_backend_preference(
    builder: &mut EventLoopBuilder<RuntimeEvent>,
    backend_preference: NativeBackendPreference,
) {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    match backend_preference {
        NativeBackendPreference::Auto => {}
        NativeBackendPreference::Wayland => {
            builder.with_wayland();
        }
        NativeBackendPreference::X11 => {
            builder.with_x11();
        }
    }
}

fn minimum_window_size() -> winit::dpi::LogicalSize<f64> {
    geometry::logical_window_size_for_grid(
        MIN_SUPPORTED_GEOMETRY.width(),
        MIN_SUPPORTED_GEOMETRY.height(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use nc_data::GameStateBuilder;
    use nc_engine::build_seeded_initialized_game;

    use super::{
        FrameTimingSample, FrameTimingSummary, ResizeObservation, ResizeShrinkTracker,
        ResizeVerdict, SessionBackend, backend_supports_programmatic_focus, classify_resize,
        combine_deadlines, full_map_fit_scale_multiplier, hosted_route_next_wakeup,
        map_pointer_cell, minimum_window_size, pointer_move_event_kind, route_uses_mouse,
        session_backend_label, should_dispatch_pointer_move, store_pending_pointer_motion,
        take_pending_pointer_motion_before_redraw, window_attributes_for_mode,
        window_decorations_for_session,
    };
    use crate::Point;
    use crate::app::{
        BootModel, HostedGameModel, LobbyModel, LobbyTab, MATRIX_FRAME_STEP,
        MIN_SUPPORTED_GEOMETRY, MyGameRow, Route,
    };
    use crate::dashboard::DashApp;
    use crate::geometry;
    use crate::input::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use crate::startup::NativeWindowMode;
    use winit::window::WindowAttributes;

    #[test]
    fn matrix_frame_step_matches_lockme_runtime_default() {
        assert_eq!(MATRIX_FRAME_STEP, Duration::from_millis(40));
    }

    #[test]
    fn wayland_backend_disables_programmatic_focus() {
        assert!(!backend_supports_programmatic_focus(
            SessionBackend::Wayland
        ));
        assert!(backend_supports_programmatic_focus(SessionBackend::X11));
        assert!(backend_supports_programmatic_focus(SessionBackend::Unknown));
    }

    #[test]
    fn backend_labels_match_expected_cli_terms() {
        assert_eq!(session_backend_label(SessionBackend::Wayland), "wayland");
        assert_eq!(session_backend_label(SessionBackend::X11), "x11");
        assert_eq!(session_backend_label(SessionBackend::Unknown), "unknown");
    }

    #[test]
    fn cosmic_wayland_session_disables_decorations() {
        assert!(!window_decorations_for_session(
            SessionBackend::Wayland,
            Some("COSMIC")
        ));
        assert!(!window_decorations_for_session(
            SessionBackend::Wayland,
            Some("cosmic")
        ));
        assert!(!window_decorations_for_session(
            SessionBackend::Wayland,
            Some("pop:COSMIC")
        ));
    }

    #[test]
    fn non_cosmic_wayland_keeps_decorations() {
        assert!(window_decorations_for_session(
            SessionBackend::Wayland,
            Some("GNOME")
        ));
        assert!(window_decorations_for_session(
            SessionBackend::Wayland,
            Some("KDE")
        ));
        assert!(window_decorations_for_session(
            SessionBackend::Wayland,
            None
        ));
    }

    #[test]
    fn non_wayland_always_keeps_decorations() {
        assert!(window_decorations_for_session(
            SessionBackend::X11,
            Some("COSMIC")
        ));
        assert!(window_decorations_for_session(
            SessionBackend::Unknown,
            Some("COSMIC")
        ));
        assert!(window_decorations_for_session(SessionBackend::X11, None));
    }

    #[test]
    fn minimum_window_size_matches_supported_grid_floor() {
        assert_eq!(
            minimum_window_size(),
            geometry::logical_window_size_for_grid(
                MIN_SUPPORTED_GEOMETRY.width(),
                MIN_SUPPORTED_GEOMETRY.height()
            )
        );
    }

    #[test]
    fn combine_deadlines_picks_the_earliest_present_deadline() {
        let now = Instant::now();
        assert_eq!(combine_deadlines(None, None), None);
        assert_eq!(combine_deadlines(Some(now), None), Some(now));
        assert_eq!(combine_deadlines(None, Some(now)), Some(now));
        assert_eq!(
            combine_deadlines(
                Some(now + Duration::from_secs(2)),
                Some(now + Duration::from_secs(1))
            ),
            Some(now + Duration::from_secs(1))
        );
    }

    #[test]
    fn pointer_move_without_left_button_emits_hover_event() {
        assert_eq!(pointer_move_event_kind(false), MouseEventKind::Moved);
    }

    #[test]
    fn pointer_move_with_left_button_emits_left_drag() {
        assert_eq!(
            pointer_move_event_kind(true),
            MouseEventKind::Drag(MouseButton::Left)
        );
    }

    #[test]
    fn pointer_move_skips_same_cell() {
        let cell = Point::from_usize(4, 7);
        assert!(!should_dispatch_pointer_move(Some(cell), Some(cell)));
        assert!(should_dispatch_pointer_move(
            Some(cell),
            Some(Point::from_usize(5, 7))
        ));
    }

    #[test]
    fn frame_timing_summary_emits_after_full_sample_window() {
        let mut summary = FrameTimingSummary::default();
        let sample = FrameTimingSample {
            view_build: Duration::from_millis(1),
            view_cache_hits: 1,
            view_cache_misses: 0,
            hosted_dashboard_render: Duration::from_millis(2),
            hosted_convert: Duration::from_millis(1),
            hosted_dirty_regions: 3,
            hosted_full_rebuild: false,
            playfield_prepare: Duration::from_millis(2),
            glyph_prepare: Duration::from_millis(3),
            gpu_submit_present: Duration::from_millis(4),
            total: Duration::from_millis(5),
            dirty_rows: 2,
            raw_spans: 3,
            text_rebuild_spans: 4,
            text_rebuild_cells: 8,
            text_buffer_misses: 6,
            upload_rects: 1,
            full_rebuild: false,
        };

        let mut message = None;
        for _ in 0..120 {
            message = summary.record(sample);
        }

        let message = message.expect("summary should emit after enough samples");
        assert!(message.contains("frame timings [120 frames]"));
        assert!(message.contains("avg_view_cache_hits=1.0"));
        assert!(message.contains("avg_view_cache_misses=0.0"));
        assert!(message.contains("avg_hosted_dashboard_render_ms=2.00"));
        assert!(message.contains("avg_hosted_convert_ms=1.00"));
        assert!(message.contains("avg_hosted_dirty_regions=3.0"));
        assert!(message.contains("avg_dirty_rows=2.0"));
        assert!(message.contains("avg_raw_spans=3.0"));
        assert!(message.contains("avg_text_rebuild_spans=4.0"));
        assert!(message.contains("avg_text_rebuild_cells=8.0"));
        assert!(message.contains("avg_text_buffer_misses=6.0"));
        assert!(message.contains("avg_upload_rects=1.0"));
        assert!(message.contains("hosted_full_rebuilds=0"));
    }

    #[test]
    fn storing_pending_pointer_motion_keeps_latest_event() {
        let mut pending = None;
        store_pending_pointer_motion(
            &mut pending,
            MouseEvent {
                kind: MouseEventKind::Moved,
                position: Point::from_usize(2, 3),
                modifiers: KeyModifiers::NONE,
            },
        );
        store_pending_pointer_motion(
            &mut pending,
            MouseEvent {
                kind: MouseEventKind::Moved,
                position: Point::from_usize(4, 5),
                modifiers: KeyModifiers::SHIFT,
            },
        );

        let pending = pending.expect("latest pointer motion should be retained");
        assert_eq!(pending.position, Point::from_usize(4, 5));
        assert_eq!(pending.modifiers, KeyModifiers::SHIFT);
    }

    #[test]
    fn redraw_takes_latest_pending_pointer_motion() {
        let mut pending = None;
        store_pending_pointer_motion(
            &mut pending,
            MouseEvent {
                kind: MouseEventKind::Moved,
                position: Point::from_usize(2, 3),
                modifiers: KeyModifiers::NONE,
            },
        );
        store_pending_pointer_motion(
            &mut pending,
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                position: Point::from_usize(6, 7),
                modifiers: KeyModifiers::CONTROL,
            },
        );

        let mouse =
            take_pending_pointer_motion_before_redraw(&mut pending).expect("pending motion");
        assert_eq!(mouse.position, Point::from_usize(6, 7));
        assert_eq!(mouse.kind, MouseEventKind::Drag(MouseButton::Left));
        assert_eq!(mouse.modifiers, KeyModifiers::CONTROL);
        assert!(pending.is_none());
    }

    #[test]
    fn hosted_route_next_wakeup_uses_dashboard_toast_deadline() {
        let now = Instant::now();
        let mut dashboard = hosted_dash_app();
        dashboard.command_line_toast_deadline = Some(now + Duration::from_secs(1));
        let route = Route::HostedGame(HostedGameModel {
            row: hosted_game_row(),
            dashboard,
            status: None,
        });

        assert_eq!(
            hosted_route_next_wakeup(&route),
            Some(now + Duration::from_secs(1))
        );
    }

    #[test]
    fn route_mouse_policy_keeps_lobby_enabled() {
        assert!(route_uses_mouse(&Route::Lobby(LobbyModel {
            active_tab: LobbyTab::MyGames,
            help_open: false,
            quit_confirm_open: false,
            selected_my_game: 0,
            my_games_scroll: 0,
            selected_open_game: 0,
            open_games_scroll: 0,
            settings_scroll: 0,
            editing_relay: false,
            relay_draft: String::new(),
            status: None,
        })));
        assert!(!route_uses_mouse(&Route::MatrixLocked));
        assert!(!route_uses_mouse(&Route::Boot(BootModel {
            status: String::new(),
        })));
    }

    #[test]
    fn windowed_mode_starts_maximized() {
        let attributes =
            window_attributes_for_mode(WindowAttributes::default(), NativeWindowMode::Windowed);

        assert!(attributes.maximized);
        assert!(attributes.fullscreen.is_none());
    }

    #[test]
    fn fullscreen_mode_stays_borderless() {
        let attributes = window_attributes_for_mode(
            WindowAttributes::default(),
            NativeWindowMode::BorderlessFullscreen,
        );

        assert!(!attributes.maximized);
        assert!(attributes.fullscreen.is_some());
    }

    #[test]
    fn full_map_fit_scale_requires_maximized_or_fullscreen() {
        assert_eq!(
            full_map_fit_scale_multiplier(false, 4000, 3000, 1.0, 236, 99),
            1.0
        );
    }

    #[test]
    fn full_map_fit_scale_picks_largest_readable_uniform_scale() {
        let dashboard = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(300, 140),
            crate::dashboard::geometry::ScreenGeometry::new(0, 0),
            1,
        );
        let preferred = crate::dashboard::layout::preferred_dashboard_frame(&dashboard);
        let fit_metrics = geometry::GridMetrics::for_scale(0.80);
        let scale = full_map_fit_scale_multiplier(
            true,
            (preferred.width() * fit_metrics.cell.width_px) as u32,
            (preferred.height() * fit_metrics.cell.height_px) as u32,
            1.0,
            preferred.width(),
            preferred.height(),
        );

        assert!(scale >= 0.80);
        assert!(scale < 1.0);
    }

    #[test]
    fn full_map_fit_scale_falls_back_when_only_sub_floor_scale_would_fit() {
        let dashboard = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(300, 140),
            crate::dashboard::geometry::ScreenGeometry::new(0, 0),
            1,
        );
        let preferred = crate::dashboard::layout::preferred_dashboard_frame(&dashboard);
        let too_small_metrics = geometry::GridMetrics::for_scale(0.64);

        assert_eq!(
            full_map_fit_scale_multiplier(
                true,
                (preferred.width() * too_small_metrics.cell.width_px) as u32,
                (preferred.height() * too_small_metrics.cell.height_px) as u32,
                1.0,
                preferred.width(),
                preferred.height(),
            ),
            1.0
        );
    }

    #[test]
    fn pointer_mapping_uses_cached_window_metrics() {
        let point = map_pointer_cell(
            1200,
            900,
            crate::ScreenGeometry::new(100, 36),
            geometry::CellMetrics {
                width_px: 12,
                height_px: 24,
            },
            winit::dpi::PhysicalPosition::new(121.0, 97.0),
        );
        assert_eq!(point, Some(Point::from_usize(10, 3)));
    }

    #[test]
    fn pointer_mapping_rejects_invalid_and_outside_positions() {
        let screen = crate::ScreenGeometry::new(100, 36);
        let cell = geometry::CellMetrics {
            width_px: 12,
            height_px: 24,
        };
        assert_eq!(
            map_pointer_cell(
                1200,
                900,
                screen,
                cell,
                winit::dpi::PhysicalPosition::new(-1.0, 0.0)
            ),
            None
        );
        assert_eq!(
            map_pointer_cell(
                1200,
                900,
                screen,
                cell,
                winit::dpi::PhysicalPosition::new(f64::NAN, 0.0)
            ),
            None
        );
        assert_eq!(
            map_pointer_cell(
                1200,
                900,
                screen,
                cell,
                winit::dpi::PhysicalPosition::new(1199.0, 899.0)
            ),
            None
        );
    }

    #[test]
    fn resize_observation_equality_requires_exact_match() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 800,
            scale_factor: 1.25,
        };
        assert_eq!(base, base);
        assert_ne!(
            base,
            ResizeObservation {
                pixel_width: 1201,
                ..base
            }
        );
        assert_ne!(
            base,
            ResizeObservation {
                pixel_height: 801,
                ..base
            }
        );
        assert_ne!(
            base,
            ResizeObservation {
                scale_factor: 1.5,
                ..base
            }
        );
    }

    // Helper: drive classify_resize through a sequence of observations, threading
    // the tracker state between calls just as observe_resize does at runtime.
    fn run_sequence(
        heights: &[u32],
        width: u32,
        scale: f64,
        input_recency_ms: Option<u32>,
    ) -> Vec<ResizeVerdict> {
        let mut last: Option<ResizeObservation> = None;
        let mut tracker: Option<ResizeShrinkTracker> = None;
        let mut verdicts = Vec::new();
        for &h in heights {
            let obs = ResizeObservation {
                pixel_width: width,
                pixel_height: h,
                scale_factor: scale,
            };
            let (verdict, new_tracker) = classify_resize(&last, obs, &tracker, input_recency_ms);
            verdicts.push(verdict);
            last = Some(obs);
            tracker = Some(new_tracker);
        }
        verdicts
    }

    #[test]
    fn classify_resize_rejects_monotonic_shrink_sequence() {
        // Thread tracker state through calls, same as observe_resize does at runtime.
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let shrink1 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let shrink2 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 756,
            scale_factor: 1.0,
        };

        // First call: no prior observation → Accept, tracker initialised.
        let (v0, t0) = classify_resize(&None, base, &None, Some(1000));
        assert_eq!(v0, ResizeVerdict::Accept);
        assert_eq!(t0.consecutive_shrinks, 0);
        assert_eq!(t0.baseline_height, 828);

        // Second call: first shrink → Accept (shrinks=1, baseline stays 828).
        let (v1, t1) = classify_resize(&Some(base), shrink1, &Some(t0), Some(1000));
        assert_eq!(v1, ResizeVerdict::Accept);
        assert_eq!(t1.consecutive_shrinks, 1);
        assert_eq!(t1.baseline_height, 828);

        // Third call: second shrink → SpuriousShrink (shrinks=2 >= threshold).
        let (v2, t2) = classify_resize(&Some(shrink1), shrink2, &Some(t1), Some(1000));
        assert_eq!(
            v2,
            ResizeVerdict::SpuriousShrink {
                restore_to: (828, 1200)
            }
        );
        assert_eq!(t2.consecutive_shrinks, 2);
        assert_eq!(t2.baseline_height, 828);
    }

    #[test]
    fn classify_resize_accepts_when_user_input_is_recent() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let shrink1 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let tracker_with_shrinks = Some(ResizeShrinkTracker {
            baseline_height: 828,
            consecutive_shrinks: 1,
        });

        let (verdict, _) = classify_resize(&Some(base), shrink1, &tracker_with_shrinks, Some(50));
        assert_eq!(verdict, ResizeVerdict::Accept);
    }

    #[test]
    fn classify_resize_accepts_first_shrink() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let shrink1 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };

        let (verdict, tracker) = classify_resize(&Some(base), shrink1, &None, Some(1000));
        assert_eq!(verdict, ResizeVerdict::Accept);
        assert_eq!(tracker.consecutive_shrinks, 1);
        assert_eq!(tracker.baseline_height, 828);
    }

    #[test]
    fn classify_resize_accepts_different_width() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let different_width = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 792,
            scale_factor: 1.0,
        };

        let (verdict, tracker) = classify_resize(&Some(base), different_width, &None, Some(1000));
        assert_eq!(verdict, ResizeVerdict::Accept);
        // Width change resets tracker: baseline is the new height, shrinks=0.
        assert_eq!(tracker.consecutive_shrinks, 0);
        assert_eq!(tracker.baseline_height, 792);
    }

    // --- new regression tests ---

    #[test]
    fn classify_resize_cosmic_full_sequence() {
        // COSMIC sctk-adwaita shrink loop: 864 → 828 → 792 → … → 540 (10 steps).
        // No user input throughout.
        // Expected: first observation Accept (no prior), second Accept (shrinks=1),
        // third onward all SpuriousShrink with restore_to=(864, 1200).
        let heights: Vec<u32> = vec![864, 828, 792, 756, 720, 684, 648, 612, 576, 540];
        let verdicts = run_sequence(&heights, 1200, 1.0, None);

        assert_eq!(verdicts[0], ResizeVerdict::Accept); // no prior
        assert_eq!(verdicts[1], ResizeVerdict::Accept); // shrinks=1
        for v in &verdicts[2..] {
            assert_eq!(
                *v,
                ResizeVerdict::SpuriousShrink {
                    restore_to: (864, 1200)
                }
            );
        }
    }

    #[test]
    fn classify_resize_grow_resets_baseline() {
        // After a grow the baseline rises; a subsequent shrink loop restores to the
        // new (larger) baseline, not the old one.
        // Sequence: 864 → 828 (shrinks=1) → 900 (grow, baseline=900, shrinks=0)
        //           → 864 (shrinks=1) → 828 (SpuriousShrink restore_to=900)
        let heights: Vec<u32> = vec![864, 828, 900, 864, 828];
        let verdicts = run_sequence(&heights, 1200, 1.0, None);

        assert_eq!(verdicts[0], ResizeVerdict::Accept); // no prior
        assert_eq!(verdicts[1], ResizeVerdict::Accept); // first shrink
        assert_eq!(verdicts[2], ResizeVerdict::Accept); // grow → baseline=900
        assert_eq!(verdicts[3], ResizeVerdict::Accept); // shrink from 900, shrinks=1
        assert_eq!(
            verdicts[4],
            ResizeVerdict::SpuriousShrink {
                restore_to: (900, 1200)
            }
        );
    }

    #[test]
    fn classify_resize_recent_input_resets_tracker() {
        // A shrink that arrives within 250 ms of user input is treated as
        // user-driven: it resets the baseline to the shrunken height.
        // A later shrink loop is then detected relative to the new baseline.
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 864,
            scale_factor: 1.0,
        };
        let user_shrink = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        // First: accept with recent input (50 ms) → baseline resets to 828.
        let (v0, t0) = classify_resize(&Some(base), user_shrink, &None, Some(50));
        assert_eq!(v0, ResizeVerdict::Accept);
        assert_eq!(t0.baseline_height, 828);
        assert_eq!(t0.consecutive_shrinks, 0);

        // Now feed two compositor shrinks without input → spurious at the third.
        let shrink2 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let shrink3 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 756,
            scale_factor: 1.0,
        };
        let (v1, t1) = classify_resize(&Some(user_shrink), shrink2, &Some(t0), Some(1000));
        assert_eq!(v1, ResizeVerdict::Accept);
        assert_eq!(t1.baseline_height, 828); // sticky

        let (v2, _) = classify_resize(&Some(shrink2), shrink3, &Some(t1), Some(1000));
        assert_eq!(
            v2,
            ResizeVerdict::SpuriousShrink {
                restore_to: (828, 1200)
            }
        );
    }

    #[test]
    fn classify_resize_width_change_resets_tracker() {
        // Accumulate two shrinks, then a width change resets the tracker so
        // the following shrink run starts fresh.
        let heights_before: Vec<u32> = vec![864, 828, 792]; // shrinks=0,1,spurious
        let v_before = run_sequence(&heights_before, 1200, 1.0, None);
        assert_eq!(
            v_before[2],
            ResizeVerdict::SpuriousShrink {
                restore_to: (864, 1200)
            }
        );

        // Now: different width resize → tracker reset → next two shrinks at new
        // width are Accept (shrinks=1) then SpuriousShrink on the third.
        let obs_wide = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 864,
            scale_factor: 1.0,
        };
        // Simulate starting fresh from a width-changed state.
        let (v_wide, t_wide) = classify_resize(
            &Some(ResizeObservation {
                pixel_width: 1200,
                pixel_height: 792,
                scale_factor: 1.0,
            }),
            obs_wide,
            &None,
            Some(1000),
        );
        assert_eq!(v_wide, ResizeVerdict::Accept);
        assert_eq!(t_wide.consecutive_shrinks, 0);
        assert_eq!(t_wide.baseline_height, 864);

        // Subsequent shrinks at the new width behave as a fresh sequence.
        let shrink_a = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let (va, ta) = classify_resize(&Some(obs_wide), shrink_a, &Some(t_wide), Some(1000));
        assert_eq!(va, ResizeVerdict::Accept);
        assert_eq!(ta.consecutive_shrinks, 1);
        assert_eq!(ta.baseline_height, 864);

        let shrink_b = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let (vb, _) = classify_resize(&Some(shrink_a), shrink_b, &Some(ta), Some(1000));
        assert_eq!(
            vb,
            ResizeVerdict::SpuriousShrink {
                restore_to: (864, 1000)
            }
        );
    }

    fn hosted_dash_app() -> DashApp {
        DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn hosted_game_row() -> MyGameRow {
        MyGameRow {
            game_id: "game".to_string(),
            status: "joined".to_string(),
            game_tier: "sandbox".to_string(),
            game: "Test Game".to_string(),
            host: "host".to_string(),
            host_contact_npub: None,
            relay_url: "ws://127.0.0.1:8080".to_string(),
            daemon_pubkey: "daemon".to_string(),
            seat: Some(1),
            turn_summary: "Y1:T1".to_string(),
            last_turn: Some(1),
            last_hash: Some("hash".to_string()),
        }
    }
}
