use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::input::{KeyCode, KeyEvent, KeyModifiers};
use crate::dashboard::layout::dashboard;
use crate::dashboard::panels::starmap;
use crate::dashboard::planet_view;
use crate::dashboard::table_selection;
use std::time::Instant;

use super::input::{Action, key_to_action};
use super::state;
use super::state::{
    ActiveMouseGesture, ActiveOverlay, ActivePopup, DashApp, DashboardExitRequest,
    FleetOverlayPromptMode, HelpContext, InboxMessageConfirmAction, IntelOverlayPromptMode,
    OwnedPlanetPopupMode, PlanetOverlayPromptMode,
};
use super::{COMMAND_LINE_TOAST_STEP, map_coord_rows, parse_table_coord};
use crate::dashboard::app::render;

impl DashApp {
    pub(crate) fn is_at_root_surface(&self) -> bool {
        self.overlay == ActiveOverlay::None && self.popup == ActivePopup::None
    }

    pub(crate) fn dispatch_key_event(&mut self, key: crate::dashboard::input::KeyEvent) {
        if !self.is_terminal_too_small {
            self.dismiss_active_command_line_toast();
            self.handle_key(key);
            let _ = self.update_command_line_toast_state(Instant::now());
        } else if key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('q' | 'Q'))
        {
            self.request_exit(DashboardExitRequest::QuitClient);
        }
    }
    pub(crate) fn resize_canvas(&mut self, cols: u16, rows: u16) {
        self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        self.refresh_terminal_fit_state();
    }

    #[allow(dead_code)]
    pub(crate) fn render_playfield(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        render::render(self)
    }

    pub(crate) fn take_exit_request(&mut self) -> Option<DashboardExitRequest> {
        let request = self.exit_request.take();
        if matches!(request, Some(DashboardExitRequest::QuitClient)) {
            self.should_quit = false;
        }
        request
    }
    pub(crate) fn active_command_line_toast(&self) -> Option<&str> {
        match self.overlay {
            ActiveOverlay::None => match self.popup {
                ActivePopup::OwnedPlanet { .. } => self.owned_planet_popup.status.as_deref(),
                ActivePopup::TaxPrompt => self.tax_prompt_status.as_deref(),
                ActivePopup::QuitConfirm
                | ActivePopup::InboxMessageConfirm { .. }
                | ActivePopup::PlanetDetail { .. }
                | ActivePopup::FleetDetail { .. }
                | ActivePopup::StartupReview
                | ActivePopup::None => None,
            },
            ActiveOverlay::PlanetList => match self.planet_overlay.prompt_mode {
                PlanetOverlayPromptMode::None => self.planet_overlay.footer_notice.as_deref(),
                PlanetOverlayPromptMode::SortMenu
                | PlanetOverlayPromptMode::FilterMenu
                | PlanetOverlayPromptMode::FilterValueInput
                | PlanetOverlayPromptMode::CommissionSelect
                | PlanetOverlayPromptMode::MassCommissionConfirm
                | PlanetOverlayPromptMode::TransportFleetSelect { .. }
                | PlanetOverlayPromptMode::TransportQuantity { .. } => {
                    self.planet_overlay.prompt_status.as_deref()
                }
                PlanetOverlayPromptMode::BuildSpecify => {
                    self.planet_overlay.build_unit_status.as_deref()
                }
                PlanetOverlayPromptMode::BuildQuantity => {
                    self.planet_overlay.build_quantity_status.as_deref()
                }
            },
            ActiveOverlay::FleetList => match self.fleet_overlay.prompt_mode {
                FleetOverlayPromptMode::None => self
                    .fleet_overlay
                    .aux_status
                    .as_deref()
                    .or(self.fleet_overlay.filter_prompt_status.as_deref()),
                FleetOverlayPromptMode::SortMenu
                | FleetOverlayPromptMode::FilterMenu
                | FleetOverlayPromptMode::FilterValueInput => {
                    self.fleet_overlay.filter_prompt_status.as_deref()
                }
                FleetOverlayPromptMode::ChangeField
                | FleetOverlayPromptMode::ChangeValue
                | FleetOverlayPromptMode::MergeHost
                | FleetOverlayPromptMode::MergeConfirm
                | FleetOverlayPromptMode::TransferHost
                | FleetOverlayPromptMode::TransferStage => self.fleet_overlay.aux_status.as_deref(),
                FleetOverlayPromptMode::MissionPicker => {
                    self.fleet_overlay.mission_picker_status.as_deref()
                }
                FleetOverlayPromptMode::OrderTarget
                | FleetOverlayPromptMode::OrderTargetX
                | FleetOverlayPromptMode::OrderTargetY
                | FleetOverlayPromptMode::SeekNextClosest
                | FleetOverlayPromptMode::OrderConfirm => {
                    self.fleet_overlay.order_status.as_deref()
                }
                FleetOverlayPromptMode::StarbaseMoveDecision
                | FleetOverlayPromptMode::StarbaseMoveDestination
                | FleetOverlayPromptMode::StarbaseHaltConfirm => {
                    self.fleet_overlay.starbase_move_status.as_deref()
                }
            },
            ActiveOverlay::IntelDatabase => self.intel_overlay.prompt_status.as_deref(),
            ActiveOverlay::Inbox => self.inbox_overlay.prompt_status.as_deref(),
            ActiveOverlay::Diplomacy | ActiveOverlay::Help => None,
        }
    }

    fn any_command_line_toast_present(&self) -> bool {
        self.planet_overlay.footer_notice.is_some()
            || self.planet_overlay.prompt_status.is_some()
            || self
                .planet_overlay
                .prompt_stack
                .iter()
                .any(|frame| frame.prompt_status.is_some())
            || self.planet_overlay.build_unit_status.is_some()
            || self.planet_overlay.build_quantity_status.is_some()
            || self.fleet_overlay.filter_prompt_status.is_some()
            || self.fleet_overlay.aux_status.is_some()
            || self.fleet_overlay.mission_picker_status.is_some()
            || self.fleet_overlay.order_status.is_some()
            || self.fleet_overlay.starbase_move_status.is_some()
            || self.intel_overlay.prompt_status.is_some()
            || self
                .intel_overlay
                .prompt_stack
                .iter()
                .any(|frame| frame.prompt_status.is_some())
            || self.owned_planet_popup.status.is_some()
            || self.tax_prompt_status.is_some()
            || self.inbox_overlay.prompt_status.is_some()
    }

    fn clear_all_command_line_toasts(&mut self) {
        self.command_line_toast_message = None;
        self.command_line_toast_deadline = None;
        self.planet_overlay.footer_notice = None;
        self.planet_overlay.prompt_status = None;
        for frame in &mut self.planet_overlay.prompt_stack {
            frame.prompt_status = None;
        }
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_quantity_status = None;
        self.fleet_overlay.filter_prompt_status = None;
        self.fleet_overlay.aux_status = None;
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.order_status = None;
        self.fleet_overlay.starbase_move_status = None;
        self.intel_overlay.prompt_status = None;
        for frame in &mut self.intel_overlay.prompt_stack {
            frame.prompt_status = None;
        }
        self.owned_planet_popup.status = None;
        self.tax_prompt_status = None;
        self.inbox_overlay.prompt_status = None;
    }

    fn dismiss_active_command_line_toast(&mut self) {
        if self.active_command_line_toast().is_some() {
            self.clear_all_command_line_toasts();
        }
    }

    pub(super) fn update_command_line_toast_state(&mut self, now: Instant) -> bool {
        let active = self.active_command_line_toast().map(str::to_string);
        if active != self.command_line_toast_message {
            self.command_line_toast_message = active.clone();
            self.command_line_toast_deadline =
                active.as_ref().map(|_| now + COMMAND_LINE_TOAST_STEP);
            return true;
        }
        if active.is_some() || self.any_command_line_toast_present() {
            if let Some(deadline) = self.command_line_toast_deadline {
                if deadline <= now {
                    self.clear_all_command_line_toasts();
                    return true;
                }
            } else {
                self.command_line_toast_deadline = Some(now + COMMAND_LINE_TOAST_STEP);
                return true;
            }
        } else if self.command_line_toast_deadline.is_some()
            || self.command_line_toast_message.is_some()
        {
            self.command_line_toast_deadline = None;
            self.command_line_toast_message = None;
            return true;
        }
        false
    }

    fn request_exit(&mut self, request: DashboardExitRequest) {
        self.exit_request = Some(request);
        self.should_quit = matches!(request, DashboardExitRequest::QuitClient);
    }

    fn open_quit_confirm(&mut self) {
        if self.popup == ActivePopup::QuitConfirm {
            return;
        }
        self.quit_confirm_return_popup = self.popup;
        self.quit_confirm_return_popup_position = self.popup_position.take();
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.popup = ActivePopup::QuitConfirm;
    }

    fn close_active_popup(&mut self) {
        if self.popup == ActivePopup::QuitConfirm
            && self.quit_confirm_return_popup != ActivePopup::None
        {
            self.popup = self.quit_confirm_return_popup;
            self.popup_position = self.quit_confirm_return_popup_position.take();
            self.quit_confirm_return_popup = ActivePopup::None;
            self.mouse_gesture = ActiveMouseGesture::None;
            return;
        }
        if matches!(self.popup, ActivePopup::OwnedPlanet { .. }) {
            self.owned_planet_popup.reset();
        } else if self.popup == ActivePopup::TaxPrompt {
            self.tax_prompt_input.clear();
            self.tax_prompt_status = None;
        } else if self.popup == ActivePopup::StartupReview {
            self.startup_review.reset();
        }
        self.popup = ActivePopup::None;
        self.popup_position = None;
        self.quit_confirm_return_popup = ActivePopup::None;
        self.quit_confirm_return_popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    pub(crate) fn advance_startup_review(&mut self) {
        match self.startup_review.mode {
            state::StartupReviewMode::Results => {
                if self.startup_review.results_block + 1
                    < self.startup_review.reports.result_blocks.len()
                {
                    self.startup_review.results_block += 1;
                } else {
                    if !self.startup_review.reports.message_blocks.is_empty() {
                        self.startup_review.mode = state::StartupReviewMode::Messages;
                    } else {
                        self.close_active_popup();
                    }
                }
            }
            state::StartupReviewMode::Messages => {
                if self.startup_review.messages_block + 1
                    < self.startup_review.reports.message_blocks.len()
                {
                    self.startup_review.messages_block += 1;
                } else {
                    self.close_active_popup();
                }
            }
        }
    }

    pub(crate) fn is_startup_review_nonstop(&self) -> bool {
        if self.popup != state::ActivePopup::StartupReview {
            return false;
        }
        match self.startup_review.mode {
            state::StartupReviewMode::Results => self.startup_review.results_nonstop,
            state::StartupReviewMode::Messages => self.startup_review.messages_nonstop,
        }
    }

    pub(crate) fn advance_startup_review_if_nonstop(&mut self) -> bool {
        if self.is_startup_review_nonstop() {
            self.advance_startup_review();
            true
        } else {
            false
        }
    }

    pub(super) fn handle_key(&mut self, key: crate::dashboard::input::KeyEvent) {
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char('q' | 'Q'))
        {
            self.open_quit_confirm();
            return;
        }
        if self.caller_overlay_popup_has_key_priority() && self.handle_popup_key(key) {
            return;
        }
        if self.overlay != ActiveOverlay::None && self.handle_overlay_key(key) {
            self.normalize_table_overlay_filters();
            return;
        }
        if self.popup != ActivePopup::None && self.handle_popup_key(key) {
            return;
        }
        if self.is_at_root_surface() && key.code == KeyCode::Esc {
            self.open_quit_confirm();
            return;
        }
        if self.handle_map_coord_key(key) {
            return;
        }
        let action = key_to_action(key, self.focus, self.overlay);
        if action != Action::None {
            self.map_coord_input.clear();
        }
        self.apply_action(action);
        self.normalize_table_overlay_filters();
    }

    fn caller_overlay_popup_has_key_priority(&self) -> bool {
        if matches!(
            self.popup,
            ActivePopup::QuitConfirm | ActivePopup::InboxMessageConfirm { .. }
        ) {
            return true;
        }
        let popup_over_caller = matches!(
            self.popup,
            ActivePopup::OwnedPlanet { .. }
                | ActivePopup::PlanetDetail { .. }
                | ActivePopup::FleetDetail { .. }
                | ActivePopup::StartupReview
        );
        if !popup_over_caller {
            return false;
        }
        match self.overlay {
            ActiveOverlay::PlanetList => {
                self.planet_overlay.prompt_mode == PlanetOverlayPromptMode::None
            }
            ActiveOverlay::IntelDatabase => {
                self.intel_overlay.prompt_mode == IntelOverlayPromptMode::None
            }
            ActiveOverlay::FleetList => {
                self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::None
            }
            _ => false,
        }
    }

    pub(super) fn apply_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.request_exit(DashboardExitRequest::QuitClient),
            Action::FocusNext => self.focus = self.focus.next(),
            Action::FocusPrev => self.focus = self.focus.prev(),
            Action::OpenOverlay(overlay) => {
                if overlay == ActiveOverlay::Help {
                    self.help_context = HelpContext::Global;
                    self.help_return_overlay = ActiveOverlay::None;
                    self.help_return_overlay_position = None;
                }
                if overlay == ActiveOverlay::FleetList {
                    self.fleet_overlay.clear_transient_location_filter();
                }
                self.overlay_position = None;
                self.mouse_gesture = ActiveMouseGesture::None;
                self.overlay = overlay;
            }
            Action::CloseOverlay => self.close_active_overlay(),
            Action::ClosePopup => self.close_active_popup(),
            Action::MoveCrosshairUp => {
                // Up arrow → higher Y (row 18 at top of screen).
                let map_size =
                    nc_data::map_size_for_player_count(self.game_data.conquest.player_count);
                if self.crosshair_y < map_size {
                    self.crosshair_y += 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::MoveCrosshairDown => {
                // Down arrow → lower Y (row 1 at bottom of screen).
                if self.crosshair_y > 1 {
                    self.crosshair_y -= 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::MoveCrosshairLeft => {
                if self.crosshair_x > 1 {
                    self.crosshair_x -= 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::MoveCrosshairRight => {
                let map_size =
                    nc_data::map_size_for_player_count(self.game_data.conquest.player_count);
                if self.crosshair_x < map_size {
                    self.crosshair_x += 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::JumpPlanetBackward => {
                self.jump_crosshair_to_planet(starmap::PlanetJumpDirection::Backward);
            }
            Action::JumpPlanetForward => {
                self.jump_crosshair_to_planet(starmap::PlanetJumpDirection::Forward);
            }
            Action::OpenPlanetDetailPopup => self.open_planet_detail_popup_at_cursor(),
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),
            Action::PageUp => {
                for _ in 0..10 {
                    self.scroll_up();
                }
            }
            Action::PageDown => {
                for _ in 0..10 {
                    self.scroll_down();
                }
            }
            Action::Home => self.scroll_home(),
            Action::End => self.scroll_end(),
            Action::SetTaxRate => self.open_tax_prompt(),
            Action::None => {}
        }
    }

    fn handle_map_coord_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(ch)
                if self.map_coord_input.len() < 16
                    && !matches!(ch, '[' | ']')
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.map_coord_input.push(ch);
                if self.sync_map_cursor_to_input() {
                    self.map_coord_input.clear();
                }
                true
            }
            KeyCode::Backspace => {
                self.map_coord_input.pop();
                true
            }
            _ => false,
        }
    }

    fn open_tax_prompt(&mut self) {
        self.tax_prompt_input.clear();
        self.tax_prompt_status = None;
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.popup = ActivePopup::TaxPrompt;
    }

    fn submit_tax_prompt(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let raw = self.tax_prompt_input.trim();
        let tax_rate = if raw.is_empty() {
            self.game_data
                .players
                .get(self.player_record_index_1_based.saturating_sub(1))
                .map(|player| player.tax_rate)
                .unwrap_or(0)
        } else {
            raw.parse::<u8>()
                .map_err(|_| "Enter an integer tax rate from 0 to 100.")?
        };
        if tax_rate > 100 {
            return Err("Enter an integer tax rate from 0 to 100.".into());
        }
        let player_record = self.player_record_index_1_based;
        self.change(|state| {
            state
                .game_data
                .set_player_tax_rate(player_record, tax_rate)?;
            Ok(())
        })?;
        self.stage_hosted_tax_rate(tax_rate);
        self.close_active_popup();
        let revenue = self
            .game_data
            .empire_available_production_points(self.player_record_index_1_based);
        self.command_line_toast_message = Some(format!(
            "Empire tax rate set to {tax_rate}%. Revenue next year: {revenue}."
        ));
        self.command_line_toast_deadline = Some(Instant::now() + COMMAND_LINE_TOAST_STEP);
        Ok(())
    }

    fn handle_popup_key(&mut self, key: KeyEvent) -> bool {
        match self.popup {
            ActivePopup::QuitConfirm => match key.code {
                KeyCode::Char('y' | 'Y') => self.request_exit(DashboardExitRequest::ReturnToLobby),
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('n' | 'N') => {
                    self.apply_action(Action::ClosePopup);
                }
                _ => {}
            },
            ActivePopup::InboxMessageConfirm { action } => match key.code {
                KeyCode::Char('y' | 'Y') => match action {
                    InboxMessageConfirmAction::Send => {
                        self.apply_action(Action::ClosePopup);
                        if let Err(err) = self.confirm_inbox_compose_message() {
                            self.inbox_overlay.prompt_status = Some(err.to_string());
                            self.inbox_overlay.prompt_mode = state::InboxPromptMode::ComposeBody;
                        }
                    }
                    InboxMessageConfirmAction::Discard => {
                        self.apply_action(Action::ClosePopup);
                        self.close_inbox_prompt();
                    }
                },
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('n' | 'N') => {
                    self.apply_action(Action::ClosePopup);
                }
                _ => {}
            },
            ActivePopup::TaxPrompt => match key.code {
                KeyCode::Esc => self.apply_action(Action::ClosePopup),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_tax_prompt() {
                        self.tax_prompt_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    self.tax_prompt_input.pop();
                    self.tax_prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() && self.tax_prompt_input.len() < 3 => {
                    self.tax_prompt_input.push(ch);
                    self.tax_prompt_status = None;
                }
                _ => {}
            },
            ActivePopup::StartupReview => match key.code {
                KeyCode::Esc => {
                    if self.startup_review.results_nonstop || self.startup_review.messages_nonstop {
                        self.startup_review.results_nonstop = false;
                        self.startup_review.messages_nonstop = false;
                    } else {
                        self.close_active_popup();
                    }
                }
                KeyCode::Char('s' | 'S') => match self.startup_review.mode {
                    state::StartupReviewMode::Results => self.startup_review.results_nonstop = true,
                    state::StartupReviewMode::Messages => {
                        self.startup_review.messages_nonstop = true
                    }
                },
                _ => {
                    self.advance_startup_review();
                }
            },
            ActivePopup::OwnedPlanet { .. } => match self.owned_planet_popup.mode {
                OwnedPlanetPopupMode::Browse => match key.code {
                    KeyCode::Esc => self.close_owned_planet_popup(),
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::OwnedPlanetPopup),
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        self.open_owned_planet_build_specify()
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.open_owned_planet_commission_select()
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.open_owned_planet_mass_commission_confirm()
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => self
                        .open_owned_planet_transport_fleet_select(
                            nc_engine::ArmyTransportMode::Load,
                        ),
                    KeyCode::Char('u') | KeyCode::Char('U') => self
                        .open_owned_planet_transport_fleet_select(
                            nc_engine::ArmyTransportMode::Unload,
                        ),
                    KeyCode::Char('x') | KeyCode::Char('X') => {
                        self.open_owned_planet_scorch_confirm()
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::CommissionSelect => match key.code {
                    KeyCode::Esc => self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_owned_planet_commission() {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_owned_planet_input(),
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.append_owned_planet_input_char(ch)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::CommissionResult
                | OwnedPlanetPopupMode::MassCommissionReport => match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::MassCommissionConfirm => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        if let Err(err) = self.confirm_owned_planet_mass_commission() {
                            self.show_owned_planet_status(err.to_string());
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::TransportFleetSelect { mode } => match key.code {
                    KeyCode::Esc => self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_owned_planet_transport_fleet(mode) {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_owned_planet_input(),
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.append_owned_planet_input_char(ch)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::TransportQuantity { mode } => match key.code {
                    KeyCode::Esc => self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_owned_planet_transport_quantity(mode) {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_owned_planet_input(),
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.append_owned_planet_input_char(ch)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::ScorchConfirm1
                | OwnedPlanetPopupMode::ScorchConfirm2
                | OwnedPlanetPopupMode::ScorchConfirm3 => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Err(err) = self.submit_owned_planet_scorch() {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter | KeyCode::Esc => {
                        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse)
                    }
                    _ => {}
                },
            },
            _ => {
                let _ = key;
                self.apply_action(Action::ClosePopup);
            }
        }
        true
    }
    fn jump_crosshair_to_planet(&mut self, direction: starmap::PlanetJumpDirection) {
        if let Some(target) = starmap::jump_planet_target_for_app(
            self,
            [self.crosshair_x, self.crosshair_y],
            direction,
        ) {
            self.set_crosshair_coords(target);
        }
    }

    pub(super) fn open_planet_detail_popup_at_cursor(&mut self) {
        let Some(detail) = planet_view::selected_planet_detail(self) else {
            return;
        };
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: detail.planet_record_index_1_based,
        };
    }

    fn sync_map_cursor_to_input(&mut self) -> bool {
        let rows = map_coord_rows(self);
        let Some(matched) = table_selection::find_typed_jump(&rows, 0, &self.map_coord_input)
        else {
            return false;
        };
        let Some(coords) = rows
            .get(matched.index)
            .and_then(|row| row.first())
            .and_then(|coords| parse_table_coord(coords))
        else {
            return false;
        };
        self.crosshair_x = coords[0];
        self.crosshair_y = coords[1];
        starmap::advance_starmap_viewport(self);
        matched.is_terminal_exact_match
    }

    fn refresh_terminal_fit_state(&mut self) {
        let required = dashboard::required_dashboard_frame(self);
        let layout = dashboard::dashboard_layout(self);
        self.is_terminal_too_small = self.geometry.width() < required.width()
            || self.geometry.height() < required.height()
            || !dashboard::dashboard_fits_canvas(self.geometry, &layout);
        if !self.is_terminal_too_small {
            self.frame = required;
        }
    }

    fn scroll_up(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll = self.diplomacy_scroll.saturating_sub(1),
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll += 1,
            _ => {}
        }
    }

    fn scroll_home(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll = 0,
            _ => {}
        }
    }

    fn scroll_end(&mut self) {
        // End scrolling: set to a large number; render will clamp.
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll = usize::MAX,
            _ => {}
        }
    }
}
