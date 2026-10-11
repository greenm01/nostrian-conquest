use crate::dashboard::inbox::{DashInboxItemSource, matches_filter, project_inbox_items};
use crate::dashboard::input::{KeyCode, KeyEvent, KeyModifiers};
use crate::dashboard::overlays::{diplomacy, fleet_list, inbox, intel_database, planet_list};
use crate::dashboard::table_filter::{
    TableFilterClause, format_column_code_error, is_filter_column_char, parse_column_code,
    parse_filter_clause,
};
use crate::dashboard::table_selection;
use crate::dashboard::table_selection::{sync_scroll_to_cursor, wrap_next_index, wrap_prev_index};

use super::state;
use super::state::{
    ActiveMouseGesture, ActiveOverlay, ActivePopup, DashApp, FleetOverlayFilter,
    FleetOverlayPromptMode, FleetOverlayRowKey, FleetOverlaySort, HelpContext,
    InboxMessageConfirmAction, IntelOverlayFilter, IntelOverlayPromptMode, IntelOverlaySort,
    PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
    default_fleet_overlay_sort_direction, default_intel_overlay_sort_direction,
    default_planet_overlay_sort_direction,
};

impl DashApp {
    fn set_selected_diplomatic_relation(
        &mut self,
        relation: nc_data::DiplomaticRelation,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(empire_slot) = diplomacy::selected_empire_slot(self) else {
            return Ok(());
        };
        if empire_slot as usize == self.player_record_index_1_based {
            return Ok(());
        }
        self.game_data.set_stored_diplomatic_relation(
            self.player_record_index_1_based as u8,
            empire_slot,
            relation,
        )?;
        self.stage_hosted_diplomacy(empire_slot, relation);
        self.save_and_refresh_runtime()?;
        Ok(())
    }

    pub(super) fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match self.overlay {
            ActiveOverlay::None => false,
            ActiveOverlay::PlanetList => {
                self.handle_planet_overlay_key(key);
                true
            }
            ActiveOverlay::FleetList => {
                self.handle_fleet_overlay_key(key);
                true
            }
            ActiveOverlay::IntelDatabase => {
                self.handle_intel_overlay_key(key);
                true
            }
            ActiveOverlay::Diplomacy => {
                if self.handle_overlay_close_or_help(key, HelpContext::Diplomacy) {
                    return true;
                }
                match key.code {
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        if let Err(err) = self
                            .set_selected_diplomatic_relation(nc_data::DiplomaticRelation::Enemy)
                        {
                            self.command_line_toast_message = Some(err.to_string());
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        if let Err(err) = self
                            .set_selected_diplomatic_relation(nc_data::DiplomaticRelation::Neutral)
                        {
                            self.command_line_toast_message = Some(err.to_string());
                        }
                    }
                    _ => {
                        let total_rows = self.game_data.players.len();
                        handle_list_overlay_key(
                            key,
                            &mut self.diplomacy_overlay.selected,
                            &mut self.diplomacy_overlay.scroll,
                            total_rows,
                        );
                    }
                }
                true
            }
            ActiveOverlay::Inbox => {
                if self.inbox_overlay.prompt_mode != state::InboxPromptMode::None
                    || self.inbox_overlay.delete_confirm
                {
                    self.handle_inbox_overlay_key(key);
                    return true;
                }
                if self.handle_overlay_close_or_help(key, HelpContext::Inbox) {
                    return true;
                }
                self.handle_inbox_overlay_key(key);
                true
            }
            ActiveOverlay::Help => {
                let _ = key;
                self.close_active_overlay();
                true
            }
        }
    }

    fn handle_overlay_close_or_help(&mut self, key: KeyEvent, help_context: HelpContext) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.close_active_overlay();
                true
            }
            KeyCode::Char('?') if self.overlay == ActiveOverlay::Help => {
                self.close_active_overlay();
                true
            }
            KeyCode::Char('?') => {
                self.help_return_overlay = self.overlay;
                self.help_return_overlay_position = self.overlay_position.take();
                self.help_context = help_context;
                self.overlay = ActiveOverlay::Help;
                self.mouse_gesture = ActiveMouseGesture::None;
                true
            }
            _ => false,
        }
    }

    pub(super) fn close_active_overlay(&mut self) {
        if self.overlay == ActiveOverlay::Help {
            self.overlay = self.help_return_overlay;
            self.help_return_overlay = ActiveOverlay::None;
            self.overlay_position = self.help_return_overlay_position.take();
            self.help_context = HelpContext::Global;
        } else {
            if self.overlay == ActiveOverlay::FleetList {
                self.fleet_overlay.clear_group_selection();
                self.fleet_overlay.clear_transient_location_filter();
            }
            self.overlay = ActiveOverlay::None;
            self.overlay_position = None;
        }
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    fn handle_planet_overlay_key(&mut self, key: KeyEvent) {
        let prompt_mode = self.planet_overlay.prompt_mode;
        match prompt_mode {
            PlanetOverlayPromptMode::BuildSpecify => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetBuildSpecify),
                KeyCode::Esc => self.close_planet_build_overlay(),
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if let Err(err) = self.queue_selected_planet_build_unit() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                KeyCode::Char('-') => {
                    if let Err(err) = self.remove_selected_planet_build_unit() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.select_previous_planet_build_kind()
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.select_next_planet_build_kind()
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.select_left_planet_build_kind()
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.select_right_planet_build_kind()
                }
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_build_browse_input() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_planet_build_unit_input(),
                KeyCode::Char(ch) if ch.is_ascii_digit() => self.append_planet_build_unit_char(ch),
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if let Err(err) = self.clear_selected_planet_build_kind_queue() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                _ => {}
            },
            PlanetOverlayPromptMode::BuildQuantity => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetBuildQuantity),
                KeyCode::Esc => {
                    self.cancel_planet_build_quantity();
                }
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_build_quantity() {
                        self.planet_overlay.build_quantity_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_planet_build_quantity_input(),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.append_planet_build_quantity_char(ch)
                }
                _ => {}
            },
            PlanetOverlayPromptMode::CommissionSelect => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
                KeyCode::Esc => self.planet_overlay.clear_prompt(),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_overlay_commission() {
                        self.planet_overlay.prompt_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::MassCommissionConfirm => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Err(err) = self.confirm_planet_overlay_mass_commission() {
                        self.planet_overlay.prompt_status = Some(err.to_string());
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter | KeyCode::Esc => {
                    self.planet_overlay.clear_prompt()
                }
                _ => {}
            },
            PlanetOverlayPromptMode::TransportFleetSelect { mode } => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
                KeyCode::Esc => self.planet_overlay.clear_prompt(),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_overlay_transport_fleet(mode) {
                        self.planet_overlay.prompt_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::TransportQuantity { mode } => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
                KeyCode::Esc => self.planet_overlay.clear_prompt(),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_overlay_transport_quantity(mode) {
                        self.planet_overlay.prompt_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetListSort),
                KeyCode::Esc => {
                    self.planet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = prompt_raw_value(
                        &self.planet_overlay.prompt_input,
                        &self.planet_overlay.prompt_default,
                    );
                    match parse_column_code(planet_list::filter_columns(), raw) {
                        Ok(column) => match planet_sort_from_code(column.code) {
                            Some(sort) => self.apply_planet_overlay_sort(sort),
                            None => {
                                self.planet_overlay.prompt_input.clear();
                                self.planet_overlay.prompt_status =
                                    Some(" Enter a valid sort column.".to_string());
                            }
                        },
                        Err(err) => {
                            self.planet_overlay.prompt_status =
                                Some(format!(" {}", format_column_code_error(&err)));
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetListFilter),
                KeyCode::Esc => {
                    self.planet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = prompt_raw_value(
                        &self.planet_overlay.prompt_input,
                        &self.planet_overlay.prompt_default,
                    );
                    if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                        self.apply_planet_overlay_filter_clause(None);
                    } else {
                        match parse_column_code(planet_list::filter_columns(), raw) {
                            Ok(column) => {
                                self.planet_overlay.pending_filter_column = Some(column);
                                self.planet_overlay.prompt_mode =
                                    PlanetOverlayPromptMode::FilterValueInput;
                                self.planet_overlay.prompt_input.clear();
                                self.planet_overlay.prompt_default =
                                    planet_list::filter_default_value(self, column);
                                self.planet_overlay.prompt_status = None;
                            }
                            Err(err) => {
                                self.planet_overlay.prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.planet_overlay.clear_prompt();
                }
                KeyCode::Enter => {
                    let Some(column) = self.planet_overlay.pending_filter_column else {
                        return;
                    };
                    let raw = prompt_raw_value(
                        &self.planet_overlay.prompt_input,
                        &self.planet_overlay.prompt_default,
                    );
                    match parse_filter_clause(column, raw) {
                        Ok(clause) => self.apply_planet_overlay_filter_clause(Some(clause)),
                        Err(err) => self.planet_overlay.prompt_status = Some(format!(" {err}")),
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_value_char(ch) => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::None => {}
        }
        if prompt_mode != PlanetOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
            KeyCode::Enter => self.open_selected_planet_status_popup(),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay
                    .open_prompt(PlanetOverlayPromptMode::FilterMenu);
                self.planet_overlay.prompt_input.clear();
                self.planet_overlay.prompt_default = "all".to_string();
                self.planet_overlay.prompt_status = None;
                self.planet_overlay.pending_filter_column = None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay
                    .open_prompt(PlanetOverlayPromptMode::SortMenu);
                self.planet_overlay.prompt_input.clear();
                self.planet_overlay.prompt_default =
                    planet_sort_code(self.planet_overlay.sort).to_string();
                self.planet_overlay.prompt_status = None;
            }
            KeyCode::Char('b') | KeyCode::Char('B') => self.open_planet_build_specify(),
            KeyCode::Char('c') | KeyCode::Char('C') => self.open_planet_overlay_commission_select(),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.open_planet_overlay_mass_commission_confirm()
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.open_planet_overlay_transport_fleet_select(nc_engine::ArmyTransportMode::Load)
            }
            KeyCode::Char('u') | KeyCode::Char('U') => self
                .open_planet_overlay_transport_fleet_select(nc_engine::ArmyTransportMode::Unload),
            KeyCode::Char('x') | KeyCode::Char('X') => self.open_selected_planet_scorch_confirm(),
            KeyCode::Char(ch)
                if self.planet_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay.jump_input.push(ch);
                if planet_list::sync_cursor_to_jump_input(self) {
                    self.planet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay.jump_input.pop();
            }
            _ => {
                self.clear_planet_overlay_footer_notice();
                let total_rows = planet_list::selection_rows(self).len();
                handle_list_overlay_key(
                    key,
                    &mut self.planet_overlay.selected,
                    &mut self.planet_overlay.scroll,
                    total_rows,
                );
            }
        }
    }

    fn handle_fleet_overlay_key(&mut self, key: KeyEvent) {
        let prompt_mode = self.fleet_overlay.prompt_mode;
        match prompt_mode {
            FleetOverlayPromptMode::ChangeField => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Esc => {
                    self.cancel_fleet_aux_prompt();
                }
                KeyCode::Char(ch) => self.select_fleet_change_field(ch),
                _ => {}
            },
            FleetOverlayPromptMode::ChangeValue
            | FleetOverlayPromptMode::MergeHost
            | FleetOverlayPromptMode::MergeConfirm
            | FleetOverlayPromptMode::TransferHost
            | FleetOverlayPromptMode::TransferStage => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => {
                    let result = match prompt_mode {
                        FleetOverlayPromptMode::ChangeValue => self.submit_fleet_change_prompt(),
                        FleetOverlayPromptMode::MergeHost
                        | FleetOverlayPromptMode::MergeConfirm => self.submit_fleet_merge_prompt(),
                        FleetOverlayPromptMode::TransferHost
                        | FleetOverlayPromptMode::TransferStage => {
                            self.submit_fleet_transfer_prompt()
                        }
                        _ => Ok(()),
                    };
                    if let Err(err) = result {
                        self.fleet_overlay.aux_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.aux_input.pop();
                    self.fleet_overlay.aux_status = None;
                }
                KeyCode::Esc => {
                    self.cancel_fleet_aux_prompt();
                }
                KeyCode::Char(ch)
                    if match prompt_mode {
                        FleetOverlayPromptMode::ChangeValue
                        | FleetOverlayPromptMode::TransferHost
                        | FleetOverlayPromptMode::MergeHost => ch.is_ascii_alphanumeric(),
                        FleetOverlayPromptMode::MergeConfirm => matches!(ch, 'y' | 'Y' | 'n' | 'N'),
                        FleetOverlayPromptMode::TransferStage => {
                            match self.fleet_overlay.transfer_mode {
                                crate::dashboard::app::state::FleetOverlayTransferMode::ChoosingClass => {
                                    ch.is_ascii_alphanumeric() || ch == '*'
                                }
                                crate::dashboard::app::state::FleetOverlayTransferMode::EnteringQuantity(
                                    _,
                                ) => ch.is_ascii_digit(),
                            }
                        }
                        _ => false,
                    } =>
                {
                    self.fleet_overlay.aux_input.push(ch.to_ascii_uppercase());
                    self.fleet_overlay.aux_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::MissionPicker => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetMissionPicker),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.move_fleet_mission_picker(-1)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.move_fleet_mission_picker(1)
                }
                KeyCode::PageUp => self.move_fleet_mission_picker(-8),
                KeyCode::PageDown => self.move_fleet_mission_picker(8),
                KeyCode::Enter => self.submit_fleet_mission_picker(),
                KeyCode::Backspace => self.backspace_fleet_mission_picker_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.append_fleet_mission_picker_char(ch)
                }
                _ => {}
            },
            FleetOverlayPromptMode::OrderTarget => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_fleet_order() {
                        self.fleet_overlay.order_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_fleet_order_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch)
                    if match nc_engine::fleet_target_input_kind(
                        self.fleet_overlay.order_mission_code,
                    ) {
                        nc_engine::FleetTargetInputKind::Coordinates
                        | nc_engine::FleetTargetInputKind::None => {
                            table_selection::is_coordinate_input_char(ch)
                        }
                        nc_engine::FleetTargetInputKind::StarbaseId
                        | nc_engine::FleetTargetInputKind::FleetId => ch.is_ascii_digit(),
                    } =>
                {
                    self.append_fleet_order_char(ch)
                }
                _ => {}
            },
            FleetOverlayPromptMode::OrderTargetX | FleetOverlayPromptMode::OrderTargetY => {
                match key.code {
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_fleet_order() {
                            self.fleet_overlay.order_status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_fleet_order_input(),
                    KeyCode::Esc => {
                        self.cancel_fleet_order_input();
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => self.append_fleet_order_char(ch),
                    _ => {}
                }
            }
            FleetOverlayPromptMode::SeekNextClosest => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => self.answer_seek_next_closest(true),
                KeyCode::Esc => self.cancel_fleet_order_input(),
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    self.answer_seek_next_closest(ch.eq_ignore_ascii_case(&'y'));
                }
                _ => {}
            },
            FleetOverlayPromptMode::OrderConfirm => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => {
                    self.fleet_overlay.order_confirm_input.clear();
                    if let Err(err) = self.submit_fleet_order() {
                        self.fleet_overlay.order_status = Some(err.to_string());
                    }
                }
                KeyCode::Esc => {
                    self.fleet_overlay.order_confirm_input.clear();
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    self.fleet_overlay.order_confirm_input = ch.to_ascii_uppercase().to_string();
                    if let Err(err) = self.submit_fleet_order() {
                        self.fleet_overlay.order_status = Some(err.to_string());
                    }
                }
                _ => {}
            },
            FleetOverlayPromptMode::StarbaseMoveDecision => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::StarbaseMove),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_starbase_move() {
                        self.fleet_overlay.starbase_move_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_starbase_move_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => self.append_starbase_move_char(ch),
                _ => {}
            },
            FleetOverlayPromptMode::StarbaseMoveDestination => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::StarbaseMove),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_starbase_move() {
                        self.fleet_overlay.starbase_move_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_starbase_move_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if table_selection::is_coordinate_input_char(ch) => {
                    self.append_starbase_move_char(ch)
                }
                _ => {}
            },
            FleetOverlayPromptMode::StarbaseHaltConfirm => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::StarbaseMove),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_starbase_move() {
                        self.fleet_overlay.starbase_move_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_starbase_move_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => self.cancel_fleet_order_input(),
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y') => self.append_starbase_move_char(ch),
                _ => {}
            },
            FleetOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetListSort),
                KeyCode::Esc => {
                    self.fleet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = prompt_raw_value(
                        &self.fleet_overlay.filter_prompt_input,
                        &self.fleet_overlay.filter_prompt_default,
                    );
                    match parse_column_code(fleet_list::filter_columns(), raw) {
                        Ok(column) => match fleet_sort_from_code(column.code) {
                            Some(sort) => self.apply_fleet_overlay_sort(sort),
                            None => {
                                self.fleet_overlay.filter_prompt_input.clear();
                                self.fleet_overlay.filter_prompt_status =
                                    Some(" Enter a valid sort column.".to_string());
                            }
                        },
                        Err(err) => {
                            self.fleet_overlay.filter_prompt_status =
                                Some(format!(" {}", format_column_code_error(&err)));
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.filter_prompt_input.pop();
                    self.fleet_overlay.filter_prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    self.fleet_overlay.filter_prompt_input.push(ch);
                    self.fleet_overlay.filter_prompt_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetListFilter),
                KeyCode::Esc => {
                    self.fleet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = prompt_raw_value(
                        &self.fleet_overlay.filter_prompt_input,
                        &self.fleet_overlay.filter_prompt_default,
                    );
                    if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                        self.apply_fleet_overlay_filter_clause(None);
                    } else {
                        match parse_column_code(fleet_list::filter_columns(), raw) {
                            Ok(column) => {
                                self.fleet_overlay.pending_filter_column = Some(column);
                                self.fleet_overlay.prompt_mode =
                                    FleetOverlayPromptMode::FilterValueInput;
                                self.fleet_overlay.filter_prompt_input.clear();
                                self.fleet_overlay.filter_prompt_default =
                                    fleet_list::filter_default_value(self, column);
                                self.fleet_overlay.filter_prompt_status = None;
                            }
                            Err(err) => {
                                self.fleet_overlay.filter_prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.filter_prompt_input.pop();
                    self.fleet_overlay.filter_prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    self.fleet_overlay.filter_prompt_input.push(ch);
                    self.fleet_overlay.filter_prompt_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.fleet_overlay.clear_prompt();
                }
                KeyCode::Enter => {
                    let Some(column) = self.fleet_overlay.pending_filter_column else {
                        return;
                    };
                    let raw = prompt_raw_value(
                        &self.fleet_overlay.filter_prompt_input,
                        &self.fleet_overlay.filter_prompt_default,
                    );
                    match parse_filter_clause(column, raw) {
                        Ok(clause) => self.apply_fleet_overlay_filter_clause(Some(clause)),
                        Err(err) => {
                            self.fleet_overlay.filter_prompt_status = Some(format!(" {err}"))
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.filter_prompt_input.pop();
                    self.fleet_overlay.filter_prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_value_char(ch) => {
                    self.fleet_overlay.filter_prompt_input.push(ch);
                    self.fleet_overlay.filter_prompt_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::None => {}
        }
        if prompt_mode != FleetOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Enter => self.open_selected_fleet_detail_popup(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetList),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::FilterMenu);
                self.fleet_overlay.filter_prompt_input.clear();
                self.fleet_overlay.filter_prompt_default = "all".to_string();
                self.fleet_overlay.filter_prompt_status = None;
                self.fleet_overlay.pending_filter_column = None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::SortMenu);
                self.fleet_overlay.filter_prompt_input.clear();
                self.fleet_overlay.filter_prompt_default =
                    fleet_sort_code(self.fleet_overlay.sort).to_string();
                self.fleet_overlay.filter_prompt_status = None;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => self.open_selected_fleet_order_flow(),
            KeyCode::Char('c') | KeyCode::Char('C') => self.open_selected_fleet_change_flow(),
            KeyCode::Char('m') | KeyCode::Char('M') => self.open_selected_fleet_merge_flow(),
            KeyCode::Char('t') | KeyCode::Char('T') => self.open_selected_fleet_transfer_flow(),
            KeyCode::Char(' ') => self.toggle_selected_fleet_row_for_group_order(),
            KeyCode::Char(ch)
                if self.fleet_overlay.jump_input.len() < 8 && ch.is_ascii_alphanumeric() =>
            {
                self.fleet_overlay.jump_input.push(ch);
                if fleet_list::sync_cursor_to_jump_input(self) {
                    self.fleet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.fleet_overlay.jump_input.pop();
            }
            _ => {
                let total_rows = fleet_list::selection_rows(self).len();
                handle_list_overlay_key(
                    key,
                    &mut self.fleet_overlay.selected,
                    &mut self.fleet_overlay.scroll,
                    total_rows,
                );
            }
        }
    }

    fn open_selected_fleet_detail_popup(&mut self) {
        let rows = fleet_list::table_rows(self);
        let Some(row) = rows.get(self.fleet_overlay.selected) else {
            return;
        };
        let FleetOverlayRowKey::Fleet(fleet_record_index_1_based) = row.key else {
            return;
        };
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.popup = ActivePopup::FleetDetail {
            fleet_record_index_1_based,
        };
    }

    fn handle_intel_overlay_key(&mut self, key: KeyEvent) {
        let prompt_mode = self.intel_overlay.prompt_mode;
        match prompt_mode {
            IntelOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabaseSort),
                KeyCode::Esc => {
                    self.intel_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = prompt_raw_value(
                        &self.intel_overlay.prompt_input,
                        &self.intel_overlay.prompt_default,
                    );
                    if raw.eq_ignore_ascii_case("rng") || raw.eq_ignore_ascii_case("range") {
                        self.intel_overlay
                            .open_prompt(IntelOverlayPromptMode::SortRangeInput);
                        self.intel_overlay.prompt_input.clear();
                        self.intel_overlay.prompt_default = match self.intel_overlay.sort {
                            IntelOverlaySort::Range(anchor) => {
                                crate::dashboard::coords::format_sector_coords_default(anchor)
                            }
                            _ => intel_database::table_rows(self)
                                .get(self.intel_overlay.selected)
                                .map(|row| {
                                    crate::dashboard::coords::format_sector_coords_default(
                                        row.coords,
                                    )
                                })
                                .unwrap_or_else(|| "00,00".to_string()),
                        };
                        self.intel_overlay.prompt_status = None;
                    } else {
                        match parse_column_code(intel_database::filter_columns(), raw) {
                            Ok(column) => match intel_sort_from_code(column.code) {
                                Some(sort) => self.apply_intel_overlay_sort(sort),
                                None => {
                                    self.intel_overlay.prompt_input.clear();
                                    self.intel_overlay.prompt_status =
                                        Some(" Enter a valid sort column or RNG.".to_string());
                                }
                            },
                            Err(err) => {
                                self.intel_overlay.prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                    self.intel_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    self.intel_overlay.prompt_input.push(ch);
                    self.intel_overlay.prompt_status = None;
                }
                _ => {}
            },
            IntelOverlayPromptMode::SortRangeInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.intel_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let default = intel_database::parse_coords_input(
                        &self.intel_overlay.prompt_default,
                        [0, 0],
                    )
                    .unwrap_or([0, 0]);
                    if let Some(anchor) = intel_database::parse_coords_input(
                        &self.intel_overlay.prompt_input,
                        default,
                    ) {
                        self.apply_intel_overlay_sort(IntelOverlaySort::Range(anchor));
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if table_selection::is_coordinate_input_char(ch) => {
                    self.intel_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabaseFilter),
                KeyCode::Esc => {
                    self.intel_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = prompt_raw_value(
                        &self.intel_overlay.prompt_input,
                        &self.intel_overlay.prompt_default,
                    );
                    if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                        self.apply_intel_overlay_filter_clause(None);
                    } else {
                        match parse_column_code(intel_database::filter_columns(), raw) {
                            Ok(column) => {
                                self.intel_overlay.pending_filter_column = Some(column);
                                self.intel_overlay.prompt_mode =
                                    IntelOverlayPromptMode::FilterValueInput;
                                self.intel_overlay.prompt_input.clear();
                                self.intel_overlay.prompt_default =
                                    intel_database::filter_default_value(self, column);
                                self.intel_overlay.prompt_status = None;
                            }
                            Err(err) => {
                                self.intel_overlay.prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                    self.intel_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    self.intel_overlay.prompt_input.push(ch);
                    self.intel_overlay.prompt_status = None;
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.intel_overlay.clear_prompt();
                }
                KeyCode::Enter => {
                    let Some(column) = self.intel_overlay.pending_filter_column else {
                        return;
                    };
                    let raw = prompt_raw_value(
                        &self.intel_overlay.prompt_input,
                        &self.intel_overlay.prompt_default,
                    );
                    match parse_filter_clause(column, raw) {
                        Ok(clause) => self.apply_intel_overlay_filter_clause(Some(clause)),
                        Err(err) => self.intel_overlay.prompt_status = Some(format!(" {err}")),
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                    self.intel_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_value_char(ch) => {
                    self.intel_overlay.prompt_input.push(ch);
                    self.intel_overlay.prompt_status = None;
                }
                _ => {}
            },
            IntelOverlayPromptMode::None => {}
        }
        if prompt_mode != IntelOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabase),
            KeyCode::Enter => self.open_selected_intel_planet_popup(),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.intel_overlay
                    .open_prompt(IntelOverlayPromptMode::FilterMenu);
                self.intel_overlay.prompt_input.clear();
                self.intel_overlay.prompt_default = "all".to_string();
                self.intel_overlay.prompt_status = None;
                self.intel_overlay.pending_filter_column = None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.intel_overlay
                    .open_prompt(IntelOverlayPromptMode::SortMenu);
                self.intel_overlay.prompt_input.clear();
                self.intel_overlay.prompt_default =
                    intel_sort_code(self.intel_overlay.sort).to_string();
                self.intel_overlay.prompt_status = None;
            }
            KeyCode::Char(ch)
                if self.intel_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.intel_overlay.jump_input.push(ch);
                if intel_database::sync_cursor_to_jump_input(self) {
                    self.intel_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.intel_overlay.jump_input.pop();
            }
            _ => {
                let total_rows = intel_database::selection_rows(self).len();
                handle_list_overlay_key(
                    key,
                    &mut self.intel_overlay.selected,
                    &mut self.intel_overlay.scroll,
                    total_rows,
                );
            }
        }
    }

    fn open_selected_intel_planet_popup(&mut self) {
        let Some(row) = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .cloned()
        else {
            return;
        };
        let owner = self
            .game_data
            .planets
            .get(row.planet_record_index_1_based.saturating_sub(1))
            .map(|planet| planet.owner.0)
            .unwrap_or(0);
        if owner == self.player_record_index_1_based as u8 {
            self.open_owned_planet_popup(row.planet_record_index_1_based);
        } else {
            self.popup_position = None;
            self.mouse_gesture = ActiveMouseGesture::None;
            self.popup = state::ActivePopup::PlanetDetail {
                planet_record_index_1_based: row.planet_record_index_1_based,
            };
        }
    }

    fn handle_inbox_overlay_key(&mut self, key: KeyEvent) {
        if self.inbox_overlay.prompt_mode != state::InboxPromptMode::None {
            self.handle_inbox_prompt_key(key);
            return;
        }
        if self.inbox_overlay.delete_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    delete_selected_inbox_item(self);
                    if let Err(err) = self.save_and_refresh_runtime() {
                        self.command_line_toast_message = Some(err.to_string());
                    }
                    let total_rows = inbox::selection_rows(self).len();
                    self.inbox_overlay.selected = self
                        .inbox_overlay
                        .selected
                        .min(total_rows.saturating_sub(1));
                    sync_scroll_to_cursor(
                        &mut self.inbox_overlay.scroll,
                        self.inbox_overlay.selected,
                        10,
                    );
                    self.inbox_overlay.preview_scroll = 0;
                    self.inbox_overlay.delete_confirm = false;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.inbox_overlay.delete_confirm = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.inbox_overlay.focus = match self.inbox_overlay.focus {
                    state::InboxFocus::List => state::InboxFocus::Preview,
                    state::InboxFocus::Preview => state::InboxFocus::List,
                };
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.inbox_overlay.filter = state::InboxFilter::All;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
                self.inbox_overlay.preview_scroll = 0;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.inbox_overlay.filter = state::InboxFilter::Reports;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
                self.inbox_overlay.preview_scroll = 0;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.inbox_overlay.filter = state::InboxFilter::Messages;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
                self.inbox_overlay.preview_scroll = 0;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.inbox_overlay.current_year_only = !self.inbox_overlay.current_year_only;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
                self.inbox_overlay.preview_scroll = 0;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if inbox::selection_rows(self).is_empty() {
                    self.inbox_overlay.prompt_status = Some("No inbox item selected.".to_string());
                } else {
                    self.inbox_overlay.delete_confirm = true;
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => self.open_inbox_compose_recipient(),
            KeyCode::Char('o') | KeyCode::Char('O') => self.open_inbox_outbox(),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                self.inbox_overlay.jump_input.push(ch);
                if self.sync_inbox_overlay_cursor_to_input() {
                    self.inbox_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.inbox_overlay.jump_input.pop();
            }
            KeyCode::Up | KeyCode::Char('k') => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    self.inbox_overlay.selected =
                        wrap_prev_index(self.inbox_overlay.selected, total_rows);
                    if self.inbox_overlay.selected < self.inbox_overlay.scroll {
                        self.inbox_overlay.scroll = self.inbox_overlay.selected;
                    }
                    self.inbox_overlay.preview_scroll = 0;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll =
                        self.inbox_overlay.preview_scroll.saturating_sub(1);
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    self.inbox_overlay.selected =
                        wrap_next_index(self.inbox_overlay.selected, total_rows);
                    sync_scroll_to_cursor(
                        &mut self.inbox_overlay.scroll,
                        self.inbox_overlay.selected,
                        10,
                    );
                    self.inbox_overlay.preview_scroll = 0;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll += 1;
                }
            },
            KeyCode::PageUp => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    let last = total_rows.saturating_sub(1);
                    self.inbox_overlay.selected = self.inbox_overlay.selected.saturating_sub(10);
                    self.inbox_overlay.selected = self.inbox_overlay.selected.min(last);
                    self.inbox_overlay.scroll = self.inbox_overlay.scroll.saturating_sub(10);
                    self.inbox_overlay.preview_scroll = 0;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll =
                        self.inbox_overlay.preview_scroll.saturating_sub(10);
                }
            },
            KeyCode::PageDown => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    let last = total_rows.saturating_sub(1);
                    self.inbox_overlay.selected =
                        self.inbox_overlay.selected.saturating_add(10).min(last);
                    sync_scroll_to_cursor(
                        &mut self.inbox_overlay.scroll,
                        self.inbox_overlay.selected,
                        10,
                    );
                    self.inbox_overlay.preview_scroll = 0;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll += 10;
                }
            },
            KeyCode::Home => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    self.inbox_overlay.selected = 0;
                    self.inbox_overlay.scroll = 0;
                    self.inbox_overlay.preview_scroll = 0;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll = 0;
                }
            },
            KeyCode::End => {
                if matches!(self.inbox_overlay.focus, state::InboxFocus::List) {
                    let last = inbox::selection_rows(self).len().saturating_sub(1);
                    self.inbox_overlay.selected = last;
                    self.inbox_overlay.scroll = self.inbox_overlay.selected.saturating_sub(5);
                    self.inbox_overlay.preview_scroll = 0;
                } else {
                    self.inbox_overlay.preview_scroll = usize::MAX / 4;
                }
            }
            _ => {}
        }
    }

    fn handle_inbox_prompt_key(&mut self, key: KeyEvent) {
        match self.inbox_overlay.prompt_mode {
            state::InboxPromptMode::ComposeRecipient => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::InboxCompose),
                KeyCode::Esc => self.close_inbox_prompt(),
                KeyCode::Enter => self.submit_inbox_compose_recipient(),
                KeyCode::Up | KeyCode::Char('k') => {
                    let total = inbox::compose::recipient_rows(self).len();
                    self.inbox_overlay.compose_recipient_selected =
                        wrap_prev_index(self.inbox_overlay.compose_recipient_selected, total);
                    self.sync_compose_recipient_input_to_selected();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let total = inbox::compose::recipient_rows(self).len();
                    self.inbox_overlay.compose_recipient_selected =
                        wrap_next_index(self.inbox_overlay.compose_recipient_selected, total);
                    self.sync_compose_recipient_input_to_selected();
                }
                KeyCode::Backspace => {
                    self.inbox_overlay.prompt_input.pop();
                    self.inbox_overlay.prompt_status = None;
                    self.sync_compose_recipient_selected_to_input();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.inbox_overlay.prompt_input.push(ch);
                    self.inbox_overlay.prompt_status = None;
                    self.sync_compose_recipient_selected_to_input();
                }
                _ => {}
            },
            state::InboxPromptMode::ComposeSubject => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::InboxCompose),
                KeyCode::Esc => self.open_inbox_compose_recipient(),
                KeyCode::Enter => self.submit_inbox_compose_subject(),
                KeyCode::Backspace => {
                    self.inbox_overlay.prompt_input.pop();
                    self.inbox_overlay.prompt_status = None;
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.inbox_overlay.prompt_input.chars().count()
                            < inbox::editor::COMPOSE_SUBJECT_LIMIT =>
                {
                    self.inbox_overlay.prompt_input.push(ch);
                    self.inbox_overlay.prompt_status = None;
                }
                _ => {}
            },
            state::InboxPromptMode::ComposeBody => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return;
                }
                match key.code {
                    KeyCode::F(1) => self.open_overlay_help(HelpContext::InboxCompose),
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::InboxCompose),
                    KeyCode::Char('s' | 'S') if key.modifiers.contains(KeyModifiers::ALT) => {
                        self.open_inbox_message_confirm(InboxMessageConfirmAction::Send);
                    }
                    KeyCode::Char('x' | 'X') if key.modifiers.contains(KeyModifiers::ALT) => {
                        self.open_inbox_message_confirm(InboxMessageConfirmAction::Discard);
                    }
                    KeyCode::Esc => {}
                    KeyCode::Backspace => self.handle_compose_body_backspace(),
                    KeyCode::Delete => self.handle_compose_body_delete(),
                    KeyCode::Enter => self.handle_compose_body_newline(),
                    KeyCode::Tab => self.handle_compose_body_tab(),
                    KeyCode::Left => self.handle_compose_body_cursor_left(),
                    KeyCode::Right => self.handle_compose_body_cursor_right(),
                    KeyCode::Up => self.handle_compose_body_cursor_up(),
                    KeyCode::Down => self.handle_compose_body_cursor_down(),
                    KeyCode::Home => self.handle_compose_body_cursor_home(),
                    KeyCode::End => self.handle_compose_body_cursor_end(),
                    KeyCode::Char(_) if key.modifiers.contains(KeyModifiers::ALT) => {}
                    KeyCode::Char(ch) => self.handle_compose_body_char(ch),
                    _ => {}
                }
            }
            state::InboxPromptMode::ComposeConfirm => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::InboxCompose),
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    if let Err(err) = self.confirm_inbox_compose_message() {
                        self.inbox_overlay.prompt_status = Some(err.to_string());
                        self.inbox_overlay.prompt_mode = state::InboxPromptMode::ComposeBody;
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.inbox_overlay.prompt_mode = state::InboxPromptMode::ComposeBody;
                }
                _ => {}
            },
            state::InboxPromptMode::Outbox => match self.inbox_overlay.outbox_focus {
                state::InboxFocus::List => match key.code {
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::Inbox),
                    KeyCode::Esc => self.close_inbox_prompt(),
                    KeyCode::Tab => {
                        self.inbox_overlay.outbox_focus = state::InboxFocus::Preview;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Err(err) = self.delete_selected_outbox_message() {
                            self.inbox_overlay.prompt_status = Some(err.to_string());
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let total_rows = inbox::list::staged_outbox_messages(self).len();
                        self.inbox_overlay.outbox_selected =
                            wrap_prev_index(self.inbox_overlay.outbox_selected, total_rows);
                        sync_scroll_to_cursor(
                            &mut self.inbox_overlay.outbox_scroll,
                            self.inbox_overlay.outbox_selected,
                            10,
                        );
                        self.inbox_overlay.outbox_preview_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let total_rows = inbox::list::staged_outbox_messages(self).len();
                        self.inbox_overlay.outbox_selected =
                            wrap_next_index(self.inbox_overlay.outbox_selected, total_rows);
                        sync_scroll_to_cursor(
                            &mut self.inbox_overlay.outbox_scroll,
                            self.inbox_overlay.outbox_selected,
                            10,
                        );
                        self.inbox_overlay.outbox_preview_scroll = 0;
                    }
                    KeyCode::Backspace => {
                        self.inbox_overlay.prompt_input.pop();
                        self.sync_outbox_cursor_to_input();
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.inbox_overlay.prompt_input.push(ch);
                        self.sync_outbox_cursor_to_input();
                    }
                    _ => {}
                },
                state::InboxFocus::Preview => match key.code {
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::Inbox),
                    KeyCode::Esc => self.close_inbox_prompt(),
                    KeyCode::Tab => {
                        self.inbox_overlay.outbox_focus = state::InboxFocus::List;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.inbox_overlay.outbox_preview_scroll =
                            self.inbox_overlay.outbox_preview_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.inbox_overlay.outbox_preview_scroll += 1;
                    }
                    KeyCode::PageUp => {
                        self.inbox_overlay.outbox_preview_scroll =
                            self.inbox_overlay.outbox_preview_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        self.inbox_overlay.outbox_preview_scroll += 10;
                    }
                    KeyCode::Home => {
                        self.inbox_overlay.outbox_preview_scroll = 0;
                    }
                    _ => {}
                },
            },
            state::InboxPromptMode::None => {}
        }
    }

    fn handle_compose_body_char(&mut self, ch: char) {
        if self.inbox_overlay.compose_body.chars().count() >= inbox::editor::COMPOSE_BODY_LIMIT {
            return;
        }
        let cursor = inbox::editor::ComposeCursor {
            row: self.inbox_overlay.compose_body_cursor_row,
            col: self.inbox_overlay.compose_body_cursor_col,
        };
        if let Some(index) =
            inbox::editor::materialize_compose_cursor(&mut self.inbox_overlay.compose_body, cursor)
        {
            let byte_idx =
                inbox::editor::char_to_byte_index(&self.inbox_overlay.compose_body, index);
            self.inbox_overlay.compose_body.insert(byte_idx, ch);
            self.handle_compose_body_cursor_right();
        }
    }

    fn handle_compose_body_backspace(&mut self) {
        let cursor = inbox::editor::ComposeCursor {
            row: self.inbox_overlay.compose_body_cursor_row,
            col: self.inbox_overlay.compose_body_cursor_col,
        };
        if cursor.row == 0 && cursor.col == 0 {
            return;
        }
        if cursor.col > 0 {
            self.handle_compose_body_cursor_left();
            self.handle_compose_body_delete();
        } else {
            // Merge with previous line
            self.handle_compose_body_cursor_up();
            self.handle_compose_body_cursor_end();
            self.handle_compose_body_delete();
        }
    }

    fn handle_compose_body_delete(&mut self) {
        let cursor = inbox::editor::ComposeCursor {
            row: self.inbox_overlay.compose_body_cursor_row,
            col: self.inbox_overlay.compose_body_cursor_col,
        };
        if let Some(index) = inbox::editor::compose_existing_index_for_cursor(
            &self.inbox_overlay.compose_body,
            cursor,
        ) {
            if index < self.inbox_overlay.compose_body.chars().count() {
                let byte_idx =
                    inbox::editor::char_to_byte_index(&self.inbox_overlay.compose_body, index);
                self.inbox_overlay.compose_body.remove(byte_idx);
            }
        }
    }

    fn handle_compose_body_newline(&mut self) {
        self.handle_compose_body_char('\n');
    }

    fn handle_compose_body_tab(&mut self) {
        for _ in 0..4 {
            self.handle_compose_body_char(' ');
        }
    }

    fn handle_compose_body_cursor_left(&mut self) {
        if self.inbox_overlay.compose_body_cursor_col > 0 {
            self.inbox_overlay.compose_body_cursor_col -= 1;
        } else if self.inbox_overlay.compose_body_cursor_row > 0 {
            self.handle_compose_body_cursor_up();
            self.handle_compose_body_cursor_end();
        }
    }

    fn handle_compose_body_cursor_right(&mut self) {
        let row_end = inbox::editor::compose_row_end_col(
            &self.inbox_overlay.compose_body,
            self.inbox_overlay.compose_body_cursor_row,
        );
        if self.inbox_overlay.compose_body_cursor_col < row_end {
            self.inbox_overlay.compose_body_cursor_col += 1;
        } else {
            let total_rows = inbox::editor::wrap_body_segments(
                &self.inbox_overlay.compose_body,
                inbox::editor::COMPOSE_BODY_WRAP_WIDTH,
            )
            .len();
            if self.inbox_overlay.compose_body_cursor_row + 1 < total_rows {
                self.inbox_overlay.compose_body_cursor_row += 1;
                self.inbox_overlay.compose_body_cursor_col = 0;
            }
        }
    }

    fn handle_compose_body_cursor_up(&mut self) {
        if self.inbox_overlay.compose_body_cursor_row > 0 {
            self.inbox_overlay.compose_body_cursor_row -= 1;
            let row_end = inbox::editor::compose_row_end_col(
                &self.inbox_overlay.compose_body,
                self.inbox_overlay.compose_body_cursor_row,
            );
            self.inbox_overlay.compose_body_cursor_col =
                self.inbox_overlay.compose_body_cursor_col.min(row_end);
        }
    }

    fn handle_compose_body_cursor_down(&mut self) {
        let total_rows = inbox::editor::wrap_body_segments(
            &self.inbox_overlay.compose_body,
            inbox::editor::COMPOSE_BODY_WRAP_WIDTH,
        )
        .len();
        if self.inbox_overlay.compose_body_cursor_row + 1 < total_rows {
            self.inbox_overlay.compose_body_cursor_row += 1;
            let row_end = inbox::editor::compose_row_end_col(
                &self.inbox_overlay.compose_body,
                self.inbox_overlay.compose_body_cursor_row,
            );
            self.inbox_overlay.compose_body_cursor_col =
                self.inbox_overlay.compose_body_cursor_col.min(row_end);
        }
    }

    fn handle_compose_body_cursor_home(&mut self) {
        self.inbox_overlay.compose_body_cursor_col = 0;
    }

    fn handle_compose_body_cursor_end(&mut self) {
        self.inbox_overlay.compose_body_cursor_col = inbox::editor::compose_row_end_col(
            &self.inbox_overlay.compose_body,
            self.inbox_overlay.compose_body_cursor_row,
        );
    }

    fn sync_compose_recipient_input_to_selected(&mut self) {
        let rows = inbox::compose::recipient_rows(self);
        if let Some(row) = rows.get(self.inbox_overlay.compose_recipient_selected) {
            if let Some(id) = row.first() {
                self.inbox_overlay.prompt_input = id.trim().to_string();
            }
        }
    }

    fn sync_compose_recipient_selected_to_input(&mut self) {
        let input = self.inbox_overlay.prompt_input.trim();
        if input.is_empty() {
            return;
        }
        let rows = inbox::compose::recipient_rows(self);
        if let Some(idx) = rows
            .iter()
            .position(|row: &Vec<String>| row.first().map(|s: &String| s.trim()) == Some(input))
        {
            self.inbox_overlay.compose_recipient_selected = idx;
        }
    }

    fn open_inbox_compose_recipient(&mut self) {
        self.inbox_overlay.prompt_mode = state::InboxPromptMode::ComposeRecipient;
        self.inbox_overlay.prompt_input.clear();
        self.inbox_overlay.prompt_default = first_message_recipient_default(self);
        self.inbox_overlay.prompt_status = None;
        self.inbox_overlay.compose_recipient_empire = None;
        self.inbox_overlay.compose_recipient_selected = 0;
        self.inbox_overlay.compose_recipient_scroll = 0;
        self.inbox_overlay.compose_subject.clear();
        self.inbox_overlay.compose_body.clear();
        self.inbox_overlay.compose_body_cursor_row = 0;
        self.inbox_overlay.compose_body_cursor_col = 0;
        self.sync_compose_recipient_selected_to_input();
    }

    fn submit_inbox_compose_recipient(&mut self) {
        let raw = prompt_raw_value(
            &self.inbox_overlay.prompt_input,
            &self.inbox_overlay.prompt_default,
        );
        let Ok(recipient) = raw.parse::<u8>() else {
            self.inbox_overlay.prompt_status = Some("Enter an empire number.".to_string());
            return;
        };
        let max_empire = self.game_data.conquest.player_count;
        if recipient == 0 || recipient > max_empire {
            self.inbox_overlay.prompt_status =
                Some(format!("Enter an empire number in 1..={max_empire}."));
            return;
        }
        if recipient as usize == self.player_record_index_1_based {
            self.inbox_overlay.prompt_status = Some("You cannot message yourself.".to_string());
            return;
        }
        self.inbox_overlay.compose_recipient_empire = Some(recipient);
        self.inbox_overlay.prompt_mode = state::InboxPromptMode::ComposeSubject;
        self.inbox_overlay.prompt_input.clear();
        self.inbox_overlay.prompt_default.clear();
        self.inbox_overlay.prompt_status = None;
    }

    fn submit_inbox_compose_subject(&mut self) {
        self.inbox_overlay.compose_subject = self.inbox_overlay.prompt_input.trim().to_string();
        self.inbox_overlay.prompt_mode = state::InboxPromptMode::ComposeBody;
        self.inbox_overlay.prompt_input.clear();
        self.inbox_overlay.prompt_status = None;
    }

    fn open_inbox_message_confirm(&mut self, action: InboxMessageConfirmAction) {
        self.inbox_overlay.prompt_status = None;
        self.popup = ActivePopup::InboxMessageConfirm { action };
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    pub(super) fn confirm_inbox_compose_message(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recipient = self
            .inbox_overlay
            .compose_recipient_empire
            .ok_or("Choose a recipient first.")?;
        let (subject, body) = nc_data::validate_queued_player_message(
            &self.queued_mail,
            self.player_record_index_1_based as u8,
            recipient,
            self.game_data.conquest.game_year,
            self.game_data.conquest.player_count,
            &self.inbox_overlay.compose_subject,
            &self.inbox_overlay.compose_body,
        )?;
        self.queued_mail.push(nc_data::QueuedPlayerMail {
            sender_empire_id: self.player_record_index_1_based as u8,
            recipient_empire_id: recipient,
            year: self.game_data.conquest.game_year,
            subject: subject.clone(),
            body: body.clone(),
            recipient_deleted: false,
        });
        self.stage_hosted_message(recipient, subject, body);
        self.save_and_refresh_runtime()?;
        self.close_inbox_prompt();
        Ok(())
    }

    fn open_inbox_outbox(&mut self) {
        self.inbox_overlay.prompt_mode = state::InboxPromptMode::Outbox;
        self.inbox_overlay.prompt_input.clear();
        self.inbox_overlay.prompt_default.clear();
        self.inbox_overlay.prompt_status = None;
        self.inbox_overlay.outbox_focus = state::InboxFocus::List;
        self.inbox_overlay.outbox_preview_scroll = 0;
        let total_rows = inbox::staged_outbox_messages(self).len();
        self.inbox_overlay.outbox_selected = self
            .inbox_overlay
            .outbox_selected
            .min(total_rows.saturating_sub(1));
        sync_scroll_to_cursor(
            &mut self.inbox_overlay.outbox_scroll,
            self.inbox_overlay.outbox_selected,
            10,
        );
    }

    fn delete_selected_outbox_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let messages = inbox::staged_outbox_messages(self);
        if messages.is_empty() {
            return Err("No outbox message selected.".into());
        }
        let selected = self
            .inbox_overlay
            .outbox_selected
            .min(messages.len().saturating_sub(1));
        let Some(message) = self.remove_hosted_message(selected) else {
            return Ok(());
        };
        remove_preview_queued_message(self, &message);
        self.save_and_refresh_runtime()?;
        let remaining = inbox::staged_outbox_messages(self).len();
        self.inbox_overlay.outbox_selected = self
            .inbox_overlay
            .outbox_selected
            .min(remaining.saturating_sub(1));
        sync_scroll_to_cursor(
            &mut self.inbox_overlay.outbox_scroll,
            self.inbox_overlay.outbox_selected,
            10,
        );
        Ok(())
    }

    pub(super) fn close_inbox_prompt(&mut self) {
        self.inbox_overlay.prompt_mode = state::InboxPromptMode::None;
        self.inbox_overlay.prompt_input.clear();
        self.inbox_overlay.prompt_default.clear();
        self.inbox_overlay.prompt_status = None;
        self.inbox_overlay.compose_recipient_empire = None;
        self.inbox_overlay.compose_subject.clear();
        self.inbox_overlay.compose_body.clear();
    }

    fn sync_outbox_cursor_to_input(&mut self) {
        let rows = (1..=inbox::staged_outbox_messages(self).len())
            .map(|idx| vec![format!("{idx:02}")])
            .collect::<Vec<_>>();
        let Some(matched) =
            table_selection::find_typed_jump(&rows, 0, &self.inbox_overlay.prompt_input)
        else {
            return;
        };
        self.inbox_overlay.outbox_selected = matched.index;
        self.inbox_overlay.outbox_preview_scroll = 0;
        sync_scroll_to_cursor(&mut self.inbox_overlay.outbox_scroll, matched.index, 10);
        if matched.is_terminal_exact_match {
            self.inbox_overlay.prompt_input.clear();
        }
    }

    fn sync_inbox_overlay_cursor_to_input(&mut self) -> bool {
        let rows = inbox::selection_rows(self);
        let Some(matched) =
            table_selection::find_typed_jump(&rows, 0, &self.inbox_overlay.jump_input)
        else {
            return false;
        };
        self.inbox_overlay.selected = matched.index;
        sync_scroll_to_cursor(&mut self.inbox_overlay.scroll, matched.index, 10);
        self.inbox_overlay.preview_scroll = 0;
        matched.is_terminal_exact_match
    }
}
impl DashApp {
    pub(super) fn open_overlay_help(&mut self, help_context: HelpContext) {
        self.help_return_overlay = self.overlay;
        self.help_return_overlay_position = self.overlay_position.take();
        self.help_context = help_context;
        self.overlay = ActiveOverlay::Help;
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    pub(super) fn reset_planet_overlay_prompt(&mut self) {
        self.planet_overlay.clear_prompt();
    }

    fn apply_planet_overlay_sort(&mut self, sort: PlanetOverlaySort) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        if self.planet_overlay.sort == sort {
            self.planet_overlay.sort_direction = self.planet_overlay.sort_direction.toggle();
        } else {
            self.planet_overlay.sort = sort;
            self.planet_overlay.sort_direction = default_planet_overlay_sort_direction(sort);
        }
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        self.planet_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.planet_overlay.scroll,
            self.planet_overlay.selected,
            1_000,
        );
    }

    pub(super) fn enforce_valid_planet_filter(&mut self) {
        if self.planet_overlay.filter == crate::dashboard::app::state::PlanetOverlayFilter::All
            && self.planet_overlay.filter_clause.is_none()
        {
            return;
        }
        if !crate::dashboard::overlays::planet_list::table_rows(self).is_empty() {
            return;
        }

        let previous_filter = self.planet_overlay.filter;
        let previous_clause = self.planet_overlay.filter_clause.clone();
        self.planet_overlay.filter = crate::dashboard::app::state::PlanetOverlayFilter::All;
        self.planet_overlay.filter_clause = None;
        if crate::dashboard::overlays::planet_list::table_rows(self).is_empty() {
            self.planet_overlay.filter = previous_filter;
            self.planet_overlay.filter_clause = previous_clause;
            return;
        }

        self.planet_overlay.selected = 0;
        self.planet_overlay.scroll = 0;
    }

    pub(super) fn apply_fleet_overlay_sort(&mut self, sort: FleetOverlaySort) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        if self.fleet_overlay.sort == sort {
            self.fleet_overlay.sort_direction = self.fleet_overlay.sort_direction.toggle();
        } else {
            self.fleet_overlay.sort = sort;
            self.fleet_overlay.sort_direction = default_fleet_overlay_sort_direction(sort);
        }
        self.fleet_overlay.clear_prompt();
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    pub(super) fn enforce_valid_fleet_filter(&mut self) {
        if self.fleet_overlay.filter == FleetOverlayFilter::All
            && self.fleet_overlay.filter_clause.is_none()
        {
            return;
        }
        if !fleet_list::table_rows(self).is_empty() {
            return;
        }

        let previous_filter = self.fleet_overlay.filter;
        let previous_clause = self.fleet_overlay.filter_clause.clone();
        self.fleet_overlay.filter = FleetOverlayFilter::All;
        self.fleet_overlay.filter_clause = None;
        if fleet_list::table_rows(self).is_empty() {
            self.fleet_overlay.filter = previous_filter;
            self.fleet_overlay.filter_clause = previous_clause;
            return;
        }

        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.selected = 0;
        self.fleet_overlay.scroll = 0;
    }

    pub(super) fn reset_intel_overlay_prompt(&mut self) {
        self.intel_overlay.clear_prompt();
    }

    fn enforce_valid_intel_filter(&mut self) {
        if self.intel_overlay.filter == IntelOverlayFilter::All
            && self.intel_overlay.filter_clause.is_none()
        {
            return;
        }
        if !intel_database::table_rows(self).is_empty() {
            return;
        }

        let previous_filter = self.intel_overlay.filter;
        let previous_clause = self.intel_overlay.filter_clause.clone();
        self.intel_overlay.filter = IntelOverlayFilter::All;
        self.intel_overlay.filter_clause = None;
        if intel_database::table_rows(self).is_empty() {
            self.intel_overlay.filter = previous_filter;
            self.intel_overlay.filter_clause = previous_clause;
            return;
        }

        self.intel_overlay.selected = 0;
        self.intel_overlay.scroll = 0;
    }

    pub(super) fn normalize_table_overlay_filters(&mut self) {
        self.enforce_valid_fleet_filter();
        self.enforce_valid_planet_filter();
        self.enforce_valid_intel_filter();
    }

    pub(super) fn apply_intel_overlay_sort(&mut self, sort: IntelOverlaySort) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        if self.intel_overlay.sort == sort {
            self.intel_overlay.sort_direction = self.intel_overlay.sort_direction.toggle();
        } else {
            self.intel_overlay.sort = sort;
            self.intel_overlay.sort_direction = default_intel_overlay_sort_direction(sort);
        }
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        self.intel_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.intel_overlay.scroll,
            self.intel_overlay.selected,
            10_000,
        );
    }

    fn apply_planet_overlay_filter_clause(&mut self, clause: Option<TableFilterClause>) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.planet_overlay.filter = PlanetOverlayFilter::All;
        self.planet_overlay.filter_clause = clause;
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        if rows.is_empty() {
            self.planet_overlay.filter_clause = None;
        }
        let rows = planet_list::table_rows(self);
        self.planet_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.planet_overlay.scroll,
            self.planet_overlay.selected,
            1_000,
        );
    }

    fn apply_fleet_overlay_filter_clause(&mut self, clause: Option<TableFilterClause>) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        self.fleet_overlay.filter = FleetOverlayFilter::All;
        self.fleet_overlay.filter_clause = clause;
        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.clear_prompt();
        let rows = fleet_list::table_rows(self);
        if rows.is_empty() {
            self.fleet_overlay.filter_clause = None;
        }
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    fn apply_intel_overlay_filter_clause(&mut self, clause: Option<TableFilterClause>) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.intel_overlay.filter = IntelOverlayFilter::All;
        self.intel_overlay.filter_clause = clause;
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        if rows.is_empty() {
            self.intel_overlay.filter_clause = None;
        }
        let rows = intel_database::table_rows(self);
        self.intel_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.intel_overlay.scroll,
            self.intel_overlay.selected,
            10_000,
        );
    }
}

const fn planet_sort_code(sort: PlanetOverlaySort) -> &'static str {
    match sort {
        PlanetOverlaySort::Location => "coo",
        PlanetOverlaySort::PlanetName => "pla",
        PlanetOverlaySort::MaxProduction => "max",
        PlanetOverlaySort::CurrentProduction => "cur",
        PlanetOverlaySort::Treasury => "trs",
        PlanetOverlaySort::Budget => "bdg",
        PlanetOverlaySort::Revenue => "rev",
        PlanetOverlaySort::Growth => "gro",
        PlanetOverlaySort::BuildQueue => "bui",
        PlanetOverlaySort::Stardock => "sta",
        PlanetOverlaySort::Starbase => "sbs",
        PlanetOverlaySort::Armies => "ars",
        PlanetOverlaySort::Batteries => "gbs",
    }
}

fn planet_sort_from_code(code: &str) -> Option<PlanetOverlaySort> {
    match code {
        "coo" => Some(PlanetOverlaySort::Location),
        "pla" => Some(PlanetOverlaySort::PlanetName),
        "max" => Some(PlanetOverlaySort::MaxProduction),
        "cur" => Some(PlanetOverlaySort::CurrentProduction),
        "trs" => Some(PlanetOverlaySort::Treasury),
        "bdg" => Some(PlanetOverlaySort::Budget),
        "rev" => Some(PlanetOverlaySort::Revenue),
        "gro" => Some(PlanetOverlaySort::Growth),
        "bui" => Some(PlanetOverlaySort::BuildQueue),
        "sta" => Some(PlanetOverlaySort::Stardock),
        "sbs" => Some(PlanetOverlaySort::Starbase),
        "ars" => Some(PlanetOverlaySort::Armies),
        "gbs" => Some(PlanetOverlaySort::Batteries),
        _ => None,
    }
}

const fn fleet_sort_code(sort: FleetOverlaySort) -> &'static str {
    match sort {
        FleetOverlaySort::Id => "id",
        FleetOverlaySort::Selected => "sel",
        FleetOverlaySort::Location => "loc",
        FleetOverlaySort::Order => "ord",
        FleetOverlaySort::Target => "tar",
        FleetOverlaySort::Speed => "spd",
        FleetOverlaySort::Eta => "eta",
        FleetOverlaySort::Roe => "roe",
        FleetOverlaySort::Armies => "ars",
        FleetOverlaySort::Strength => "shi",
    }
}

fn fleet_sort_from_code(code: &str) -> Option<FleetOverlaySort> {
    match code {
        "id" => Some(FleetOverlaySort::Id),
        "sel" => Some(FleetOverlaySort::Selected),
        "loc" => Some(FleetOverlaySort::Location),
        "ord" => Some(FleetOverlaySort::Order),
        "tar" => Some(FleetOverlaySort::Target),
        "spd" => Some(FleetOverlaySort::Speed),
        "eta" => Some(FleetOverlaySort::Eta),
        "roe" => Some(FleetOverlaySort::Roe),
        "ars" => Some(FleetOverlaySort::Armies),
        "shi" => Some(FleetOverlaySort::Strength),
        _ => None,
    }
}

const fn intel_sort_code(sort: IntelOverlaySort) -> &'static str {
    match sort {
        IntelOverlaySort::Location => "coo",
        IntelOverlaySort::Range(_) => "rng",
        IntelOverlaySort::PlanetName => "pla",
        IntelOverlaySort::Owner => "own",
        IntelOverlaySort::MaxProduction => "max",
        IntelOverlaySort::YearSeen => "see",
        IntelOverlaySort::Armies => "ars",
        IntelOverlaySort::Batteries => "gbs",
        IntelOverlaySort::Starbases => "sbs",
        IntelOverlaySort::CurrentProduction => "cur",
        IntelOverlaySort::Treasury => "trs",
        IntelOverlaySort::ScoutYear => "sco",
    }
}

fn intel_sort_from_code(code: &str) -> Option<IntelOverlaySort> {
    match code {
        "coo" => Some(IntelOverlaySort::Location),
        "pla" => Some(IntelOverlaySort::PlanetName),
        "own" => Some(IntelOverlaySort::Owner),
        "max" => Some(IntelOverlaySort::MaxProduction),
        "see" => Some(IntelOverlaySort::YearSeen),
        "ars" => Some(IntelOverlaySort::Armies),
        "gbs" => Some(IntelOverlaySort::Batteries),
        "sbs" => Some(IntelOverlaySort::Starbases),
        "cur" => Some(IntelOverlaySort::CurrentProduction),
        "trs" => Some(IntelOverlaySort::Treasury),
        "sco" => Some(IntelOverlaySort::ScoutYear),
        _ => None,
    }
}

fn handle_list_overlay_key(
    key: KeyEvent,
    selected: &mut usize,
    scroll: &mut usize,
    total_rows: usize,
) {
    let last = total_rows.saturating_sub(1);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            *selected = wrap_prev_index(*selected, total_rows);
            if *selected < *scroll {
                *scroll = *selected;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *selected = wrap_next_index(*selected, total_rows);
        }
        KeyCode::PageUp => {
            *selected = (*selected).saturating_sub(10);
            *selected = (*selected).min(last);
            *scroll = (*scroll).saturating_sub(10);
        }
        KeyCode::PageDown => {
            *selected = (*selected).saturating_add(10).min(last);
        }
        KeyCode::Home => {
            *selected = 0;
            *scroll = 0;
        }
        KeyCode::End => {
            *selected = last;
            *scroll = last.saturating_sub(10);
        }
        _ => {}
    }
}

pub(super) fn scroll_clamp(value: i32, max: i32) -> usize {
    value.clamp(0, max.max(0)) as usize
}

pub(super) fn scroll_selected(selected: usize, lines: i32, total: usize) -> usize {
    let last = total.saturating_sub(1);
    if lines < 0 {
        selected.saturating_add((-lines) as usize).min(last)
    } else {
        selected.saturating_sub(lines as usize)
    }
}
fn delete_selected_inbox_item(app: &mut DashApp) {
    let viewer = app.player_record_index_1_based as u8;
    let state = &app.inbox_overlay;
    let current_year = app.game_data.conquest.game_year;
    let items = project_inbox_items(
        &app.game_data,
        viewer,
        &app.report_block_rows,
        &app.queued_mail,
    )
    .into_iter()
    .filter(|item| matches_filter(item, state.filter, state.current_year_only, current_year))
    .collect::<Vec<_>>();

    let selected = state.selected.min(items.len().saturating_sub(1));
    let Some(item) = items.get(selected) else {
        return;
    };

    match item.source {
        DashInboxItemSource::ReportBlock(idx) => nc_data::delete_report_block_for_viewer(
            &mut app.report_block_rows,
            &app.game_data.players,
            idx,
            viewer,
        ),
        DashInboxItemSource::QueuedMail(idx) => {
            if let Some(mail) = app.queued_mail.get_mut(idx) {
                mail.mark_deleted_by_recipient();
            }
        }
    }
}

fn first_message_recipient_default(app: &DashApp) -> String {
    (1..=app.game_data.conquest.player_count)
        .find(|empire| *empire as usize != app.player_record_index_1_based)
        .map(|empire| empire.to_string())
        .unwrap_or_default()
}

fn remove_preview_queued_message(app: &mut DashApp, message: &nc_data::TurnMessage) {
    let sender = app.player_record_index_1_based as u8;
    let year = app.game_data.conquest.game_year;
    if let Some(index) = app.queued_mail.iter().position(|mail| {
        mail.sender_empire_id == sender
            && mail.recipient_empire_id == message.recipient_empire
            && mail.year == year
            && mail.subject == message.subject
            && mail.body == message.body
    }) {
        app.queued_mail.remove(index);
    }
}

fn prompt_raw_value<'a>(input: &'a str, default: &'a str) -> &'a str {
    if input.trim().is_empty() {
        default.trim()
    } else {
        input.trim()
    }
}

const fn is_filter_value_char(ch: char) -> bool {
    matches!(
        ch,
        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
    ) || ch.is_ascii_alphanumeric()
}
