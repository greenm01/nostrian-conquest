//! App struct, main loop, and action dispatch.

mod events;
mod fleet_orders;
mod hosted_turns;
pub mod input;
mod mouse;
mod overlays;
mod owned_planet;
pub(crate) mod panel_cache;
mod persistence;
pub(crate) mod planet_build;
pub mod render;
pub mod state;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::input::MouseEvent;
use crate::dashboard::native::NativeApp;
use crate::dashboard::ui::UiScene;
use state::{ActiveMouseGesture, DashApp, InboxMessageConfirmAction};
use std::time::{Duration, Instant};

const COMMAND_LINE_TOAST_STEP: Duration = Duration::from_secs(1);

fn quit_confirm_message() -> &'static str {
    "Quit to Lobby? Y/[N]"
}

fn inbox_message_confirm_title() -> &'static str {
    "MESSAGE"
}

fn inbox_message_confirm_message(action: InboxMessageConfirmAction) -> &'static str {
    match action {
        InboxMessageConfirmAction::Send => "Send Message? Y/[N]",
        InboxMessageConfirmAction::Discard => "Discard Message? Y/[N]",
    }
}

fn confirm_popup_width(title: &str, message: &str) -> usize {
    (message.chars().count() + 4).max(crate::dashboard::modal::modal_min_width_for_title(title))
}

fn map_coord_rows(app: &DashApp) -> Vec<Vec<String>> {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count);
    let mut rows = Vec::with_capacity(usize::from(map_size) * usize::from(map_size));
    for x in 1..=map_size {
        for y in 1..=map_size {
            rows.push(vec![format!("({x:02},{y:02})")]);
        }
    }
    rows
}

fn parse_table_coord(cell: &str) -> Option<[u8; 2]> {
    let digits = cell
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let [x, y] = digits.as_slice() else {
        return None;
    };
    Some([x.parse().ok()?, y.parse().ok()?])
}

impl NativeApp for DashApp {
    fn window_title(&self) -> &'static str {
        "Nostrian Conquest Dashboard"
    }

    fn geometry(&self) -> ScreenGeometry {
        self.geometry
    }

    fn wants_window_focus(&self) -> bool {
        true
    }

    fn saved_window_state(
        &self,
    ) -> Option<crate::dashboard::client_settings::PersistedWindowState> {
        self.client_settings.persisted_window_state()
    }

    fn persist_window_state(
        &mut self,
        state: crate::dashboard::client_settings::PersistedWindowState,
    ) -> Result<(), String> {
        let Some(path) = self.client_settings_path.as_deref() else {
            return Ok(());
        };
        self.client_settings.set_persisted_window_state(state);
        crate::dashboard::client_settings::save_client_settings_to(&self.client_settings, path)
            .map_err(|err| err.to_string())
    }

    fn dispatch_key_event(&mut self, key: crate::dashboard::input::KeyEvent) {
        Self::dispatch_key_event(self, key);
    }

    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
        Self::dispatch_mouse_event(self, mouse)
    }

    fn resize_canvas(&mut self, cols: u16, rows: u16) {
        Self::resize_canvas(self, cols, rows);
    }

    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>> {
        Ok(UiScene::from(Self::render_playfield(self)?))
    }

    fn debug_render_signature(&self) -> Option<String> {
        Some(format!(
            "focus={:?} overlay={:?} popup={:?} crosshair={},{} map_view={:?} popup_pos={} overlay_pos={} gesture={:?} toast={} report_blocks={} mail={} too_small={}",
            self.focus,
            self.overlay,
            self.popup,
            self.crosshair_x,
            self.crosshair_y,
            self.map_view_mode,
            self.popup_position.is_some(),
            self.overlay_position.is_some(),
            self.mouse_gesture,
            self.active_command_line_toast().unwrap_or("-"),
            self.report_block_rows.len(),
            self.queued_mail.len(),
            self.is_terminal_too_small,
        ))
    }

    fn on_idle(&mut self) -> bool {
        let mut changed = self.update_command_line_toast_state(Instant::now());
        if self.advance_startup_review_if_nonstop() {
            changed = true;
        }
        changed
    }

    fn next_wakeup(&self) -> Option<Instant> {
        let toast = self.command_line_toast_deadline;
        let review = if self.is_startup_review_nonstop() {
            Some(Instant::now() + std::time::Duration::from_millis(100))
        } else {
            None
        };
        match (toast, review) {
            (Some(t), Some(r)) => Some(t.min(r)),
            (t, r) => t.or(r),
        }
    }

    fn is_dragging_surface(&self) -> bool {
        matches!(
            self.mouse_gesture,
            ActiveMouseGesture::DraggingOverlay { .. }
                | ActiveMouseGesture::DraggingPopup { .. }
                | ActiveMouseGesture::DraggingStarmap { .. }
        )
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn set_should_quit(&mut self, should_quit: bool) {
        self.should_quit = should_quit;
    }
}
