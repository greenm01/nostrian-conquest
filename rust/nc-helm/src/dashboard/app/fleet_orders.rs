use std::collections::{BTreeMap, BTreeSet};

use nc_data::{FleetDetachSelection, Order, PlanetIntelSnapshot, map_size_for_player_count};
use nc_engine::{
    FLEET_MISSION_OPTIONS, SeekNextClosestQuestion, SelectedFleetRef, automatic_order_targets,
    default_host_fleet_target, default_starbase_target, fleet_mission_option,
    fleet_order_target_rejects_owned_planet, fleet_order_target_rejects_owned_scout_target,
    fleet_order_target_requires_owned_planet, fleet_order_target_requires_planet_system,
    fleet_record_supports_mission_code, fleet_target_input_kind, fleet_target_status_line,
    format_guard_fleet_clause, guard_fleet_numbers_for_starbase, owned_fleet_targets,
    owned_starbase_targets, recommended_coordinate_target,
    recommended_coordinate_target_y_for_entered_x, resolve_checked_fleet_merge_plan,
    resolve_checked_fleet_transfer_plan, seek_next_closest_question, target_available_for_mission,
};

use crate::dashboard::overlays::fleet_list;

use super::state::{
    DashApp, FleetOrderScope, FleetOverlayChangeField, FleetOverlayPromptMode, FleetOverlayRowKey,
    FleetOverlayTransferClass, FleetOverlayTransferMode, HelpContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrderFleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub order_code: u8,
    pub current_speed: u8,
    pub max_speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrderStarbaseRow {
    pub base_record_index_1_based: usize,
    pub base_id: u16,
    pub coords: [u8; 2],
    pub destination_coords: [u8; 2],
}

impl DashApp {
    fn selected_checked_fleet_refs(&self) -> Vec<SelectedFleetRef> {
        self.selected_group_order_rows()
            .into_iter()
            .map(|row| SelectedFleetRef {
                fleet_record_index_1_based: row.fleet_record_index_1_based,
                fleet_number: row.fleet_number,
                coords: row.coords,
            })
            .collect()
    }

    fn order_intel_snapshots(&self) -> BTreeMap<usize, PlanetIntelSnapshot> {
        self.planet_intel_snapshots
            .iter()
            .cloned()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect()
    }

    pub(crate) fn open_selected_fleet_order_flow(&mut self) {
        self.normalize_selected_fleet_order_selection();
        let rows = fleet_list::table_rows(self);
        let selected = self
            .fleet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        let Some(row) = rows.get(selected) else {
            return;
        };
        self.fleet_overlay.active_row_key = None;
        self.fleet_overlay.order_scope = FleetOrderScope::None;
        self.fleet_overlay.order_status = None;
        self.fleet_overlay.order_input.clear();
        self.fleet_overlay.order_target_x_input.clear();
        self.fleet_overlay.order_target_y_input.clear();
        self.fleet_overlay.order_confirm_input.clear();
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.starbase_move_input.clear();
        self.fleet_overlay.starbase_move_status = None;
        match row.key {
            FleetOverlayRowKey::Fleet(idx)
                if !self.fleet_overlay.selected_fleet_record_indexes.is_empty() =>
            {
                self.fleet_overlay.order_scope = FleetOrderScope::Group;
                self.fleet_overlay.active_row_key = Some(FleetOverlayRowKey::Fleet(idx));
                self.fleet_overlay.order_mission_code = None;
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::MissionPicker);
                self.fleet_overlay.mission_picker_cursor =
                    self.first_enabled_fleet_mission_index().unwrap_or(0);
                self.help_context = HelpContext::FleetMissionPicker;
            }
            FleetOverlayRowKey::Fleet(idx) => {
                self.fleet_overlay.order_scope = FleetOrderScope::SingleFleet;
                self.fleet_overlay.active_row_key = Some(FleetOverlayRowKey::Fleet(idx));
                self.fleet_overlay.order_mission_code = None;
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::MissionPicker);
                self.fleet_overlay.mission_picker_cursor =
                    self.first_enabled_fleet_mission_index().unwrap_or(0);
                self.help_context = HelpContext::FleetMissionPicker;
            }
            FleetOverlayRowKey::Starbase(idx) => {
                self.fleet_overlay.order_scope = FleetOrderScope::StarbaseMove;
                self.fleet_overlay.active_row_key = Some(FleetOverlayRowKey::Starbase(idx));
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::StarbaseMoveDecision);
                self.help_context = HelpContext::StarbaseMove;
            }
        }
    }

    pub(crate) fn open_selected_fleet_change_flow(&mut self) {
        self.normalize_selected_fleet_order_selection();
        self.fleet_overlay.aux_input.clear();
        self.fleet_overlay.aux_default.clear();
        self.fleet_overlay.aux_status = None;
        self.fleet_overlay.change_field = None;
        self.fleet_overlay
            .open_prompt(FleetOverlayPromptMode::ChangeField);
        self.help_context = HelpContext::FleetOrderInput;
    }

    pub(crate) fn select_fleet_change_field(&mut self, ch: char) {
        let field = match ch.to_ascii_uppercase() {
            'R' => FleetOverlayChangeField::Roe,
            'S' => FleetOverlayChangeField::Speed,
            'I' if self.fleet_overlay.selected_fleet_record_indexes.is_empty() => {
                FleetOverlayChangeField::Id
            }
            _ if self.fleet_overlay.selected_fleet_record_indexes.is_empty() => {
                self.fleet_overlay.aux_status = Some("Enter R, I, or S.".to_string());
                return;
            }
            _ => {
                self.fleet_overlay.aux_status = Some("Enter R or S.".to_string());
                return;
            }
        };
        self.fleet_overlay.change_field = Some(field);
        self.fleet_overlay.aux_input.clear();
        self.fleet_overlay.aux_status = None;
        self.fleet_overlay.aux_default = self.fleet_change_value_default(field);
        self.fleet_overlay
            .open_prompt(FleetOverlayPromptMode::ChangeValue);
    }

    pub(crate) fn open_selected_fleet_merge_flow(&mut self) {
        self.normalize_selected_fleet_order_selection();
        if !self.fleet_overlay.selected_fleet_record_indexes.is_empty() {
            match resolve_checked_fleet_merge_plan(&self.selected_checked_fleet_refs()) {
                Ok(_) => {
                    self.fleet_overlay.aux_input.clear();
                    self.fleet_overlay.aux_default = "Y".to_string();
                    self.fleet_overlay.aux_status = None;
                    self.fleet_overlay
                        .open_prompt(FleetOverlayPromptMode::MergeConfirm);
                    self.help_context = HelpContext::FleetOrderInput;
                }
                Err(err) => self.fleet_overlay.aux_status = Some(err.to_string()),
            }
            return;
        }
        let Some(selected_row) = self.selected_fleet_order_row_from_table() else {
            return;
        };
        let mut hosts = self.owned_fleet_rows_for_orders();
        hosts.retain(|row| {
            row.fleet_record_index_1_based != selected_row.fleet_record_index_1_based
                && row.coords == selected_row.coords
                && row.fleet_number < selected_row.fleet_number
        });
        if hosts.is_empty() {
            self.fleet_overlay.aux_status =
                Some("Selected fleet must share a sector with a lower-numbered host.".to_string());
            return;
        }
        self.fleet_overlay.active_row_key = Some(FleetOverlayRowKey::Fleet(
            selected_row.fleet_record_index_1_based,
        ));
        self.fleet_overlay.aux_input.clear();
        self.fleet_overlay.aux_default = hosts[0].fleet_number.to_string();
        self.fleet_overlay.aux_status = None;
        self.fleet_overlay
            .open_prompt(FleetOverlayPromptMode::MergeHost);
        self.help_context = HelpContext::FleetOrderInput;
    }

    pub(crate) fn open_selected_fleet_transfer_flow(&mut self) {
        self.normalize_selected_fleet_order_selection();
        self.fleet_overlay.transfer_selection = FleetDetachSelection::default();
        self.fleet_overlay.transfer_mode = FleetOverlayTransferMode::ChoosingClass;
        self.fleet_overlay.aux_input.clear();
        self.fleet_overlay.aux_status = None;
        if !self.fleet_overlay.selected_fleet_record_indexes.is_empty() {
            match resolve_checked_fleet_transfer_plan(
                &self.selected_checked_fleet_refs(),
                self.selected_fleet_order_row_from_table()
                    .map(|row| row.fleet_record_index_1_based),
            ) {
                Ok(plan) => {
                    self.fleet_overlay.transfer_donor_record_index_1_based =
                        Some(plan.donor_record_index_1_based);
                    self.fleet_overlay.transfer_host_record_index_1_based =
                        Some(plan.host_record_index_1_based);
                    self.fleet_overlay
                        .open_prompt(FleetOverlayPromptMode::TransferStage);
                }
                Err(err) => self.fleet_overlay.aux_status = Some(err.to_string()),
            }
            self.help_context = HelpContext::FleetOrderInput;
            return;
        }
        let Some(selected_row) = self.selected_fleet_order_row_from_table() else {
            return;
        };
        let mut hosts = self.owned_fleet_rows_for_orders();
        hosts.retain(|row| {
            row.fleet_record_index_1_based != selected_row.fleet_record_index_1_based
                && row.coords == selected_row.coords
        });
        if hosts.is_empty() {
            self.fleet_overlay.aux_status =
                Some("Selected fleet must share a sector with another fleet.".to_string());
            return;
        }
        self.fleet_overlay.transfer_donor_record_index_1_based =
            Some(selected_row.fleet_record_index_1_based);
        self.fleet_overlay.transfer_host_record_index_1_based = None;
        self.fleet_overlay.aux_input.clear();
        self.fleet_overlay.aux_default = hosts[0].fleet_number.to_string();
        self.fleet_overlay
            .open_prompt(FleetOverlayPromptMode::TransferHost);
        self.help_context = HelpContext::FleetOrderInput;
    }

    pub(crate) fn append_fleet_mission_picker_char(&mut self, ch: char) {
        if self.fleet_overlay.mission_picker_input.len() >= 2 {
            return;
        }
        self.fleet_overlay.mission_picker_input.push(ch);
        self.sync_fleet_mission_picker_cursor_to_input();
        self.fleet_overlay.mission_picker_status = None;
    }

    pub(crate) fn backspace_fleet_mission_picker_input(&mut self) {
        self.fleet_overlay.mission_picker_input.pop();
        self.sync_fleet_mission_picker_cursor_to_input();
        self.fleet_overlay.mission_picker_status = None;
    }

    pub(crate) fn move_fleet_mission_picker(&mut self, delta: i8) {
        let enabled = self.fleet_mission_picker_enabled_flags();
        if !enabled.iter().any(|flag| *flag) {
            self.fleet_overlay.mission_picker_status = Some(match self.fleet_overlay.order_scope {
                FleetOrderScope::Group => {
                    "No missions are available for the selected fleets.".to_string()
                }
                _ => "No missions are available for the selected fleet.".to_string(),
            });
            return;
        }
        let total = FLEET_MISSION_OPTIONS.len();
        let mut next = self.fleet_overlay.mission_picker_cursor as isize;
        let step = if delta >= 0 { 1 } else { -1 };
        let hops = delta.unsigned_abs().max(1) as usize;
        for _ in 0..hops {
            loop {
                next = (next + step).rem_euclid(total as isize);
                if enabled[next as usize] {
                    break;
                }
            }
        }
        self.fleet_overlay.mission_picker_cursor = next as usize;
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
    }

    pub(crate) fn submit_fleet_mission_picker(&mut self) {
        let mission_code = match self.fleet_overlay.mission_picker_input.trim() {
            "" => FLEET_MISSION_OPTIONS
                .get(self.fleet_overlay.mission_picker_cursor)
                .map(|option| option.code)
                .unwrap_or(1),
            raw => match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet_overlay.mission_picker_status =
                        Some("Enter a mission number from 0 to 15.".to_string());
                    return;
                }
            },
        };
        let Some(index) = FLEET_MISSION_OPTIONS
            .iter()
            .position(|option| option.code == mission_code)
        else {
            self.fleet_overlay.mission_picker_status =
                Some("Enter a mission number from 0 to 15.".to_string());
            return;
        };
        if !self
            .fleet_mission_picker_enabled_flags()
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            self.fleet_overlay.mission_picker_status = Some(match self.fleet_overlay.order_scope {
                FleetOrderScope::Group => {
                    "That mission does not apply to all selected fleets.".to_string()
                }
                _ => "That mission does not apply to the selected fleet.".to_string(),
            });
            return;
        }
        let has_target_available = match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.fleet_group_has_target_available(mission_code),
            _ => self.fleet_order_has_target_available(mission_code),
        };
        if !has_target_available {
            self.fleet_overlay.mission_picker_status =
                Some(self.fleet_target_unavailable_message(mission_code));
            return;
        }
        self.fleet_overlay.mission_picker_cursor = index;
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.order_mission_code = Some(mission_code);
        self.fleet_overlay.order_input.clear();
        self.fleet_overlay.order_target_x_input.clear();
        self.fleet_overlay.order_target_y_input.clear();
        self.fleet_overlay.order_confirm_input.clear();
        let records = match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self
                .selected_group_order_rows()
                .iter()
                .map(|row| row.fleet_record_index_1_based)
                .collect::<Vec<_>>(),
            _ => self
                .selected_fleet_order_row()
                .map(|row| vec![row.fleet_record_index_1_based])
                .unwrap_or_default(),
        };
        match automatic_order_targets(&self.game_data, mission_code, &records) {
            Err(err) => {
                self.fleet_overlay.order_mission_code = None;
                self.fleet_overlay.mission_picker_status = Some(err);
                return;
            }
            // Each fleet of a group seeks its own nearest world.
            Ok(Some(_)) if mission_code == Order::SeekHome.to_raw() && records.len() > 1 => {
                if let Err(err) = self.apply_fleet_group_order(mission_code, [0, 0], None, None) {
                    self.fleet_overlay.mission_picker_status = Some(err.to_string());
                }
                return;
            }
            Ok(Some(targets)) => {
                self.fleet_overlay.order_target_x_input = format!("{:02}", targets[0][0]);
                self.fleet_overlay.order_target_y_input = format!("{:02}", targets[0][1]);
                self.fleet_overlay
                    .open_prompt(if self.fleet_order_seek_next_closest().is_some() {
                        FleetOverlayPromptMode::SeekNextClosest
                    } else {
                        FleetOverlayPromptMode::OrderConfirm
                    });
                self.help_context = HelpContext::FleetOrderInput;
                return;
            }
            Ok(None) => {}
        }
        let next_prompt = match fleet_target_input_kind(Some(mission_code)) {
            nc_engine::FleetTargetInputKind::Coordinates => FleetOverlayPromptMode::OrderTargetX,
            nc_engine::FleetTargetInputKind::StarbaseId
            | nc_engine::FleetTargetInputKind::FleetId
            | nc_engine::FleetTargetInputKind::None => FleetOverlayPromptMode::OrderTarget,
        };
        self.fleet_overlay.open_prompt(next_prompt);
        self.help_context = HelpContext::FleetOrderInput;
    }

    pub(crate) fn cancel_fleet_order_input(&mut self) {
        self.fleet_overlay.order_status = None;
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::MissionPicker => self.close_fleet_order_overlay(),
            FleetOverlayPromptMode::OrderTarget
            | FleetOverlayPromptMode::OrderTargetX
            | FleetOverlayPromptMode::OrderTargetY
            | FleetOverlayPromptMode::SeekNextClosest
            | FleetOverlayPromptMode::OrderConfirm => {
                self.fleet_overlay.close_prompt();
                if self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::MissionPicker {
                    self.help_context = HelpContext::FleetMissionPicker;
                }
            }
            FleetOverlayPromptMode::StarbaseMoveDecision => self.close_fleet_order_overlay(),
            FleetOverlayPromptMode::StarbaseMoveDestination
            | FleetOverlayPromptMode::StarbaseHaltConfirm => {
                self.fleet_overlay.close_prompt();
                if self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::StarbaseMoveDecision {
                    self.help_context = HelpContext::StarbaseMove;
                }
            }
            FleetOverlayPromptMode::FilterMenu
            | FleetOverlayPromptMode::FilterValueInput
            | FleetOverlayPromptMode::SortMenu
            | FleetOverlayPromptMode::ChangeField
            | FleetOverlayPromptMode::ChangeValue
            | FleetOverlayPromptMode::MergeHost
            | FleetOverlayPromptMode::MergeConfirm
            | FleetOverlayPromptMode::TransferHost
            | FleetOverlayPromptMode::TransferStage => {
                self.fleet_overlay.close_prompt();
            }
            FleetOverlayPromptMode::None => {}
        }
        if self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::OrderTargetY {
            self.fleet_overlay.order_confirm_input.clear();
        }
    }

    /// Answers ECGAME's Seek Home question: yes heads for the next closest
    /// world, no stays on the world the fleet is at. The order then asks for
    /// confirmation.
    pub(crate) fn answer_seek_next_closest(&mut self, yes: bool) {
        if let Some(question) = self.fleet_order_seek_next_closest() {
            let target = if yes { question.next } else { question.stay };
            self.fleet_overlay.order_target_x_input = format!("{:02}", target[0]);
            self.fleet_overlay.order_target_y_input = format!("{:02}", target[1]);
        }
        self.fleet_overlay.order_confirm_input.clear();
        self.fleet_overlay.order_status = None;
        self.fleet_overlay
            .open_prompt(FleetOverlayPromptMode::OrderConfirm);
    }

    /// ECGAME's Seek Home question for the fleets being ordered, when it asks.
    fn fleet_order_seek_next_closest(&self) -> Option<SeekNextClosestQuestion> {
        let records = match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self
                .selected_group_order_rows()
                .iter()
                .map(|row| row.fleet_record_index_1_based)
                .collect::<Vec<_>>(),
            _ => self
                .selected_fleet_order_row()
                .map(|row| vec![row.fleet_record_index_1_based])
                .unwrap_or_default(),
        };
        seek_next_closest_question(
            &self.game_data,
            self.fleet_overlay.order_mission_code?,
            &records,
        )
    }

    pub(crate) fn append_fleet_order_char(&mut self, ch: char) {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::OrderTarget => {
                if self.fleet_overlay.order_input.len() < 16 {
                    self.fleet_overlay.order_input.push(ch);
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::OrderTargetX => {
                if self.fleet_overlay.order_target_x_input.len() < 2 {
                    self.fleet_overlay.order_target_x_input.push(ch);
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::OrderTargetY => {
                if self.fleet_overlay.order_target_y_input.len() < 2 {
                    self.fleet_overlay.order_target_y_input.push(ch);
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::OrderConfirm => {
                if self.fleet_overlay.order_confirm_input.is_empty() {
                    self.fleet_overlay
                        .order_confirm_input
                        .push(ch.to_ascii_uppercase());
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::StarbaseMoveDecision
            | FleetOverlayPromptMode::StarbaseMoveDestination
            | FleetOverlayPromptMode::StarbaseHaltConfirm
            | FleetOverlayPromptMode::SeekNextClosest
            | FleetOverlayPromptMode::MissionPicker
            | FleetOverlayPromptMode::FilterMenu
            | FleetOverlayPromptMode::FilterValueInput
            | FleetOverlayPromptMode::ChangeField
            | FleetOverlayPromptMode::ChangeValue
            | FleetOverlayPromptMode::MergeHost
            | FleetOverlayPromptMode::MergeConfirm
            | FleetOverlayPromptMode::TransferHost
            | FleetOverlayPromptMode::TransferStage
            | FleetOverlayPromptMode::SortMenu
            | FleetOverlayPromptMode::None => {}
        }
    }

    pub(crate) fn backspace_fleet_order_input(&mut self) {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::OrderTarget => {
                self.fleet_overlay.order_input.pop();
            }
            FleetOverlayPromptMode::OrderTargetX => {
                self.fleet_overlay.order_target_x_input.pop();
            }
            FleetOverlayPromptMode::OrderTargetY => {
                self.fleet_overlay.order_target_y_input.pop();
            }
            FleetOverlayPromptMode::OrderConfirm => {
                self.fleet_overlay.order_confirm_input.pop();
            }
            _ => {}
        }
        self.fleet_overlay.order_status = None;
    }

    pub(crate) fn submit_fleet_order(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(mission_code) = self.fleet_overlay.order_mission_code else {
            self.fleet_overlay.order_status = Some(
                match self.fleet_overlay.order_scope {
                    FleetOrderScope::Group => "Choose a group mission first.",
                    _ => "Choose a fleet mission first.",
                }
                .to_string(),
            );
            return Ok(());
        };
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::OrderTarget => {
                let (destination, aux0, aux1) = match fleet_target_input_kind(Some(mission_code)) {
                    nc_engine::FleetTargetInputKind::Coordinates => {
                        self.fleet_overlay
                            .open_prompt(FleetOverlayPromptMode::OrderTargetX);
                        return Ok(());
                    }
                    nc_engine::FleetTargetInputKind::StarbaseId => {
                        let Some(base) = self.resolve_starbase_target_for_current_mission() else {
                            self.fleet_overlay.order_status = Some(
                                "Enter a starbase number from your starbase list.".to_string(),
                            );
                            return Ok(());
                        };
                        (base.coords, Some(1), Some(base.base_id as u8))
                    }
                    nc_engine::FleetTargetInputKind::FleetId => {
                        let Some(host) = self.resolve_host_fleet_for_current_mission() else {
                            self.fleet_overlay.order_status = Some(
                                "Enter another fleet number from your fleet list.".to_string(),
                            );
                            return Ok(());
                        };
                        return match self.fleet_overlay.order_scope {
                            FleetOrderScope::Group => self.apply_fleet_group_join_order(host),
                            _ => self.apply_fleet_single_join_order(host),
                        };
                    }
                    nc_engine::FleetTargetInputKind::None => ([0, 0], None, None),
                };
                if let Err(err) = self.validate_fleet_target_for_mission(mission_code, destination)
                {
                    self.fleet_overlay.order_status = Some(err);
                    return Ok(());
                }
                match self.fleet_overlay.order_scope {
                    FleetOrderScope::Group => {
                        self.apply_fleet_group_order(mission_code, destination, aux0, aux1)
                    }
                    _ => self.apply_fleet_single_order(mission_code, destination, aux0, aux1),
                }
            }
            FleetOverlayPromptMode::OrderTargetX => {
                let default = self.fleet_order_target_x_default_value().parse::<u8>().ok();
                match self.resolve_target_axis_input(
                    &self.fleet_overlay.order_target_x_input,
                    default,
                    "XX",
                ) {
                    Ok(value) => {
                        if self.fleet_overlay.order_target_x_input.trim().is_empty() {
                            self.fleet_overlay.order_target_x_input = format!("{value:02}");
                        }
                        self.fleet_overlay
                            .open_prompt(FleetOverlayPromptMode::OrderTargetY);
                        self.fleet_overlay.order_status = None;
                    }
                    Err(err) => self.fleet_overlay.order_status = Some(err),
                }
                Ok(())
            }
            FleetOverlayPromptMode::OrderTargetY => {
                let destination = match self.resolve_fleet_order_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet_overlay.close_prompt();
                        self.fleet_overlay.order_status = Some(err);
                        return Ok(());
                    }
                };
                if self.fleet_overlay.order_target_y_input.trim().is_empty()
                    && let Some(default_y) = self.fleet_order_default_target_y_value()
                {
                    self.fleet_overlay.order_target_y_input = format!("{default_y:02}");
                }
                if let Err(err) = self.validate_fleet_target_for_mission(mission_code, destination)
                {
                    self.fleet_overlay.close_prompt();
                    self.fleet_overlay.order_status = Some(err);
                    return Ok(());
                }
                self.fleet_overlay.order_confirm_input.clear();
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::OrderConfirm);
                self.fleet_overlay.order_status = None;
                Ok(())
            }
            FleetOverlayPromptMode::OrderConfirm => {
                if !resolve_yes_no_input(&self.fleet_overlay.order_confirm_input, true) {
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::MissionPicker;
                    self.help_context = HelpContext::FleetMissionPicker;
                    self.fleet_overlay.prompt_stack.clear();
                    self.fleet_overlay.order_mission_code = None;
                    self.fleet_overlay.order_input.clear();
                    self.fleet_overlay.order_target_x_input.clear();
                    self.fleet_overlay.order_target_y_input.clear();
                    self.fleet_overlay.order_confirm_input.clear();
                    return Ok(());
                }
                let destination = match self.resolve_fleet_order_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet_overlay.close_prompt();
                        self.fleet_overlay.close_prompt();
                        self.fleet_overlay.order_status = Some(err);
                        return Ok(());
                    }
                };
                if let Err(err) = self.validate_fleet_target_for_mission(mission_code, destination)
                {
                    self.fleet_overlay.close_prompt();
                    self.fleet_overlay.close_prompt();
                    self.fleet_overlay.order_status = Some(err);
                    return Ok(());
                }
                match self.fleet_overlay.order_scope {
                    FleetOrderScope::Group => {
                        self.apply_fleet_group_order(mission_code, destination, None, None)
                    }
                    _ => self.apply_fleet_single_order(mission_code, destination, None, None),
                }
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn append_starbase_move_char(&mut self, ch: char) {
        let max_len = match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::StarbaseMoveDecision => 1,
            FleetOverlayPromptMode::StarbaseMoveDestination => 16,
            FleetOverlayPromptMode::StarbaseHaltConfirm => 1,
            _ => 0,
        };
        if max_len > 0 && self.fleet_overlay.starbase_move_input.len() < max_len {
            self.fleet_overlay.starbase_move_input.push(ch);
            self.fleet_overlay.starbase_move_status = None;
        }
    }

    pub(crate) fn backspace_starbase_move_input(&mut self) {
        self.fleet_overlay.starbase_move_input.pop();
        self.fleet_overlay.starbase_move_status = None;
    }

    pub(crate) fn submit_starbase_move(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(row) = self.selected_starbase_move_row() else {
            self.close_fleet_order_overlay();
            return Ok(());
        };
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::StarbaseMoveDecision => {
                let raw = self.fleet_overlay.starbase_move_input.trim();
                let choice = raw
                    .chars()
                    .next()
                    .map(|ch| ch.to_ascii_uppercase())
                    .unwrap_or('M');
                match choice {
                    'H' => {
                        self.fleet_overlay.starbase_move_input.clear();
                        self.fleet_overlay.starbase_move_status = None;
                        self.fleet_overlay
                            .open_prompt(FleetOverlayPromptMode::StarbaseHaltConfirm);
                    }
                    'M' => {
                        self.fleet_overlay.starbase_move_input.clear();
                        self.fleet_overlay.starbase_move_status = None;
                        self.fleet_overlay
                            .open_prompt(FleetOverlayPromptMode::StarbaseMoveDestination);
                    }
                    _ => {
                        self.fleet_overlay.starbase_move_status =
                            Some("Choose H or M.".to_string());
                    }
                }
                Ok(())
            }
            FleetOverlayPromptMode::StarbaseMoveDestination => {
                let destination = resolve_default_coords_input(
                    &self.fleet_overlay.starbase_move_input,
                    row.destination_coords,
                )
                .ok_or_else(|| "Enter coordinates like 10,13".to_string())?;
                let map_size = map_size_for_player_count(self.game_data.conquest.player_count());
                if destination[0] == 0
                    || destination[1] == 0
                    || destination[0] > map_size
                    || destination[1] > map_size
                {
                    self.fleet_overlay.starbase_move_status =
                        Some(format!("Enter coordinates within 1..{map_size}"));
                    return Ok(());
                }
                self.finalize_starbase_destination(row, destination)
            }
            FleetOverlayPromptMode::StarbaseHaltConfirm => {
                if !resolve_yes_no_input(&self.fleet_overlay.starbase_move_input, true) {
                    self.fleet_overlay.starbase_move_input.clear();
                    self.fleet_overlay.close_prompt();
                    return Ok(());
                }
                self.finalize_starbase_destination(row, row.coords)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn close_fleet_order_overlay(&mut self) {
        self.fleet_overlay.clear_prompt();
        self.fleet_overlay.order_scope = FleetOrderScope::None;
        self.fleet_overlay.active_row_key = None;
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.order_mission_code = None;
        self.fleet_overlay.order_status = None;
        self.fleet_overlay.order_input.clear();
        self.fleet_overlay.order_target_x_input.clear();
        self.fleet_overlay.order_target_y_input.clear();
        self.fleet_overlay.order_confirm_input.clear();
        self.fleet_overlay.starbase_move_input.clear();
        self.fleet_overlay.starbase_move_status = None;
        self.help_context = HelpContext::FleetList;
    }

    pub(crate) fn cancel_fleet_aux_prompt(&mut self) {
        self.fleet_overlay.clear_prompt();
        self.help_context = HelpContext::FleetList;
    }

    pub(crate) fn submit_fleet_change_prompt(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::ChangeField => {
                self.fleet_overlay.aux_status = Some(
                    if self.fleet_overlay.selected_fleet_record_indexes.is_empty() {
                        "Enter R, I, or S."
                    } else {
                        "Enter R or S."
                    }
                    .to_string(),
                );
                Ok(())
            }
            FleetOverlayPromptMode::ChangeValue => {
                let Some(field) = self.fleet_overlay.change_field else {
                    self.fleet_overlay.aux_status = Some("Choose a field first.".to_string());
                    return Ok(());
                };
                let raw = if self.fleet_overlay.aux_input.trim().is_empty() {
                    self.fleet_overlay.aux_default.trim().to_string()
                } else {
                    self.fleet_overlay.aux_input.trim().to_string()
                };
                let rows = self.change_target_rows();
                if rows.is_empty() {
                    self.fleet_overlay.aux_status =
                        Some("Selected fleet is no longer available.".to_string());
                    return Ok(());
                }
                let checked_scope = !self.fleet_overlay.selected_fleet_record_indexes.is_empty();
                match field {
                    FleetOverlayChangeField::Roe => {
                        let roe = raw
                            .parse::<u8>()
                            .map_err(|_| "Enter an ROE from 0 to 10.".to_string())?;
                        let mut successful = Vec::new();
                        let mut failure_detail = None;
                        for row in &rows {
                            match self.game_data.set_fleet_rules_of_engagement(
                                self.player_record_index_1_based,
                                row.fleet_record_index_1_based,
                                roe,
                            ) {
                                Ok(_) => successful.push(row.fleet_record_index_1_based),
                                Err(err) => {
                                    if failure_detail.is_none() {
                                        failure_detail = Some(err.to_string());
                                    }
                                }
                            }
                        }
                        if successful.is_empty() {
                            self.fleet_overlay.aux_status = Some(
                                failure_detail
                                    .unwrap_or_else(|| "Unable to change ROE.".to_string()),
                            );
                            return Ok(());
                        }
                        for record_index in &successful {
                            self.stage_hosted_fleet_roe(*record_index, roe);
                        }
                        self.save_and_refresh_runtime()?;
                        for record_index in &successful {
                            self.fleet_overlay
                                .selected_fleet_record_indexes
                                .remove(record_index);
                        }
                        let failure_count = rows.len().saturating_sub(successful.len());
                        if failure_count == 0 {
                            self.fleet_overlay.clear_group_selection();
                            self.cancel_fleet_aux_prompt();
                            self.fleet_overlay.aux_status = if checked_scope {
                                Some(format!(
                                    "Set ROE {} for {} checked fleets.",
                                    roe,
                                    rows.len()
                                ))
                            } else {
                                Some(format!(
                                    "Fleet #{} ROE set to {}.",
                                    rows[0].fleet_number, roe
                                ))
                            };
                        } else {
                            self.fleet_overlay.aux_status = Some(format!(
                                "Set ROE {} for {} fleets. {} {} remain selected: {}",
                                roe,
                                successful.len(),
                                failure_count,
                                if failure_count == 1 {
                                    "fleet"
                                } else {
                                    "fleets"
                                },
                                failure_detail
                                    .as_deref()
                                    .unwrap_or("Some fleets could not be changed.")
                            ));
                        }
                    }
                    FleetOverlayChangeField::Id => {
                        if self.is_hosted_mode() {
                            self.fleet_overlay.aux_status = Some(
                                "Hosted play does not support fleet renumbering yet.".to_string(),
                            );
                            return Ok(());
                        }
                        let row = rows[0];
                        let id = raw
                            .parse::<u16>()
                            .map_err(|_| "Enter a fleet ID from 1 up.".to_string())?;
                        let old_number = row.fleet_number;
                        self.game_data
                            .set_fleet_local_slot(
                                self.player_record_index_1_based,
                                row.fleet_record_index_1_based,
                                id,
                            )
                            .map_err(|err| err.to_string())?;
                        self.save_and_refresh_runtime()?;
                        self.cancel_fleet_aux_prompt();
                        self.fleet_overlay.aux_status = Some(format!(
                            "Fleet #{} renumbered to Fleet #{}.",
                            old_number, id
                        ));
                    }
                    FleetOverlayChangeField::Speed => {
                        let speed = raw
                            .parse::<u8>()
                            .map_err(|_| "Enter a speed from 1 up.".to_string())?;
                        let mut successful = Vec::new();
                        let mut failure_detail = None;
                        let mut applied_speed = speed;
                        for row in &rows {
                            match self.game_data.set_fleet_speed(
                                self.player_record_index_1_based,
                                row.fleet_record_index_1_based,
                                speed,
                            ) {
                                Ok(applied) => {
                                    applied_speed = applied;
                                    successful.push(row.fleet_record_index_1_based);
                                }
                                Err(err) => {
                                    if failure_detail.is_none() {
                                        failure_detail = Some(err.to_string());
                                    }
                                }
                            }
                        }
                        if successful.is_empty() {
                            self.fleet_overlay.aux_status = Some(
                                failure_detail
                                    .unwrap_or_else(|| "Unable to change speed.".to_string()),
                            );
                            return Ok(());
                        }
                        for record_index in &successful {
                            self.stage_hosted_fleet_speed(*record_index, speed);
                        }
                        self.save_and_refresh_runtime()?;
                        for record_index in &successful {
                            self.fleet_overlay
                                .selected_fleet_record_indexes
                                .remove(record_index);
                        }
                        let failure_count = rows.len().saturating_sub(successful.len());
                        if failure_count == 0 {
                            self.fleet_overlay.clear_group_selection();
                            self.cancel_fleet_aux_prompt();
                            self.fleet_overlay.aux_status = if checked_scope {
                                Some(format!(
                                    "Set speed {} for {} checked fleets.",
                                    speed,
                                    rows.len()
                                ))
                            } else if applied_speed == speed {
                                Some(format!(
                                    "Fleet #{} speed set to {}.",
                                    rows[0].fleet_number, speed
                                ))
                            } else {
                                Some(format!(
                                    "Fleet #{} is at its intended location or has no mission; its speed stays 0.",
                                    rows[0].fleet_number
                                ))
                            };
                        } else {
                            self.fleet_overlay.aux_status = Some(format!(
                                "Set speed {} for {} fleets. {} {} remain selected: {}",
                                speed,
                                successful.len(),
                                failure_count,
                                if failure_count == 1 {
                                    "fleet"
                                } else {
                                    "fleets"
                                },
                                failure_detail
                                    .as_deref()
                                    .unwrap_or("Some fleets could not be changed.")
                            ));
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn submit_fleet_merge_prompt(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::MergeConfirm => {
                if !resolve_yes_no_input(&self.fleet_overlay.aux_input, true) {
                    self.cancel_fleet_aux_prompt();
                    return Ok(());
                }
                let plan = resolve_checked_fleet_merge_plan(&self.selected_checked_fleet_refs())
                    .map_err(|err| err.to_string())?;
                for row in self
                    .selected_group_order_rows()
                    .into_iter()
                    .filter(|row| row.fleet_record_index_1_based != plan.host_record_index_1_based)
                {
                    self.game_data.set_join_fleet_order(
                        self.player_record_index_1_based,
                        row.fleet_record_index_1_based,
                        plan.host_record_index_1_based,
                    )?;
                    self.stage_hosted_fleet_join(
                        row.fleet_record_index_1_based,
                        plan.host_record_index_1_based,
                    );
                }
                self.save_and_refresh_runtime()?;
                self.fleet_overlay.clear_group_selection();
                self.cancel_fleet_aux_prompt();
                Ok(())
            }
            FleetOverlayPromptMode::MergeHost => {
                let Some(source) = self.selected_fleet_order_row() else {
                    self.fleet_overlay.aux_status =
                        Some("Selected fleet is no longer available.".to_string());
                    return Ok(());
                };
                let raw = if self.fleet_overlay.aux_input.trim().is_empty() {
                    self.fleet_overlay.aux_default.trim().to_string()
                } else {
                    self.fleet_overlay.aux_input.trim().to_string()
                };
                let host_number = raw
                    .parse::<u16>()
                    .map_err(|_| "Enter one of your fleet numbers.".to_string())?;
                let host = self
                    .owned_fleet_rows_for_orders()
                    .into_iter()
                    .find(|row| row.fleet_number == host_number)
                    .ok_or_else(|| "Enter one of your fleet numbers.".to_string())?;
                self.game_data.set_join_fleet_order(
                    self.player_record_index_1_based,
                    source.fleet_record_index_1_based,
                    host.fleet_record_index_1_based,
                )?;
                self.stage_hosted_fleet_join(
                    source.fleet_record_index_1_based,
                    host.fleet_record_index_1_based,
                );
                self.save_and_refresh_runtime()?;
                self.cancel_fleet_aux_prompt();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn submit_fleet_transfer_prompt(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::TransferHost => {
                let Some(donor_index) = self.fleet_overlay.transfer_donor_record_index_1_based
                else {
                    self.fleet_overlay.aux_status = Some("Select a donor fleet first.".to_string());
                    return Ok(());
                };
                let raw = if self.fleet_overlay.aux_input.trim().is_empty() {
                    self.fleet_overlay.aux_default.trim().to_string()
                } else {
                    self.fleet_overlay.aux_input.trim().to_string()
                };
                let host_number = raw
                    .parse::<u16>()
                    .map_err(|_| "Enter one of your fleet numbers.".to_string())?;
                let host = self
                    .owned_fleet_rows_for_orders()
                    .into_iter()
                    .find(|row| row.fleet_number == host_number)
                    .ok_or_else(|| "Enter one of your fleet numbers.".to_string())?;
                if host.fleet_record_index_1_based == donor_index {
                    return Err("Choose a different destination fleet.".into());
                }
                self.fleet_overlay.transfer_host_record_index_1_based =
                    Some(host.fleet_record_index_1_based);
                self.fleet_overlay.transfer_mode = FleetOverlayTransferMode::ChoosingClass;
                self.fleet_overlay.aux_input.clear();
                self.fleet_overlay.aux_status = None;
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::TransferStage);
                Ok(())
            }
            FleetOverlayPromptMode::TransferStage => match self.fleet_overlay.transfer_mode {
                FleetOverlayTransferMode::ChoosingClass => {
                    let raw = self.fleet_overlay.aux_input.trim().to_ascii_uppercase();
                    if raw.is_empty() {
                        self.fleet_overlay.aux_status =
                            Some("Use BB, CA, DD, TT*, TT, SC, ET, C, X, or Q.".to_string());
                        return Ok(());
                    }
                    match raw.as_str() {
                        "C" => self.finish_fleet_transfer_prompt(),
                        "X" => {
                            self.fleet_overlay.transfer_selection = FleetDetachSelection::default();
                            self.fleet_overlay.aux_input.clear();
                            self.fleet_overlay.aux_status = None;
                            Ok(())
                        }
                        "Q" => {
                            self.cancel_fleet_aux_prompt();
                            Ok(())
                        }
                        _ => {
                            let Some(class) = self.parse_fleet_transfer_class_code(&raw) else {
                                self.fleet_overlay.aux_status = Some(
                                    "Use BB, CA, DD, TT*, TT, SC, ET, C, X, or Q.".to_string(),
                                );
                                return Ok(());
                            };
                            let available = self.fleet_transfer_available_for_class(class);
                            if available == 0 {
                                self.fleet_overlay.aux_status =
                                    Some("That class is not available for transfer.".to_string());
                                return Ok(());
                            }
                            self.fleet_overlay.transfer_mode =
                                FleetOverlayTransferMode::EnteringQuantity(class);
                            self.fleet_overlay.aux_default = "1".to_string();
                            self.fleet_overlay.aux_input.clear();
                            self.fleet_overlay.aux_status = None;
                            Ok(())
                        }
                    }
                }
                FleetOverlayTransferMode::EnteringQuantity(class) => {
                    let available = self.fleet_transfer_available_for_class(class);
                    let raw = if self.fleet_overlay.aux_input.trim().is_empty() {
                        self.fleet_overlay.aux_default.trim().to_string()
                    } else {
                        self.fleet_overlay.aux_input.trim().to_string()
                    };
                    let qty = raw
                        .parse::<u16>()
                        .map_err(|_| "Enter an integer value.".to_string())?;
                    if qty == 0 || qty > available {
                        self.fleet_overlay.aux_status =
                            Some(format!("Enter a quantity from 1 to {available}."));
                        return Ok(());
                    }
                    match class {
                        FleetOverlayTransferClass::Battleships => {
                            self.fleet_overlay.transfer_selection.battleships += qty
                        }
                        FleetOverlayTransferClass::Cruisers => {
                            self.fleet_overlay.transfer_selection.cruisers += qty
                        }
                        FleetOverlayTransferClass::Destroyers => {
                            self.fleet_overlay.transfer_selection.destroyers += qty
                        }
                        FleetOverlayTransferClass::FullTransports => {
                            self.fleet_overlay.transfer_selection.full_transports += qty
                        }
                        FleetOverlayTransferClass::EmptyTransports => {
                            self.fleet_overlay.transfer_selection.empty_transports += qty
                        }
                        FleetOverlayTransferClass::Scouts => {
                            self.fleet_overlay.transfer_selection.scouts = self
                                .fleet_overlay
                                .transfer_selection
                                .scouts
                                .saturating_add(qty)
                        }
                        FleetOverlayTransferClass::Etacs => {
                            self.fleet_overlay.transfer_selection.etacs += qty
                        }
                    }
                    self.fleet_overlay.transfer_mode = FleetOverlayTransferMode::ChoosingClass;
                    self.fleet_overlay.aux_default.clear();
                    self.fleet_overlay.aux_input.clear();
                    self.fleet_overlay.aux_status = None;
                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }

    pub(crate) fn fleet_mission_picker_enabled_flags(&self) -> Vec<bool> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => {
                let selected = self.selected_group_order_rows();
                FLEET_MISSION_OPTIONS
                    .iter()
                    .map(|option| {
                        !selected.is_empty()
                            && selected
                                .iter()
                                .all(|row| self.fleet_row_supports_mission(row, option.code))
                    })
                    .collect()
            }
            _ => {
                let Some(row) = self.selected_fleet_order_row() else {
                    return vec![false; FLEET_MISSION_OPTIONS.len()];
                };
                let Some(fleet) = self
                    .game_data
                    .fleets
                    .records
                    .get(row.fleet_record_index_1_based.saturating_sub(1))
                else {
                    return vec![false; FLEET_MISSION_OPTIONS.len()];
                };
                FLEET_MISSION_OPTIONS
                    .iter()
                    .map(|option| fleet_record_supports_mission_code(fleet, option.code))
                    .collect()
            }
        }
    }

    pub(crate) fn fleet_order_target_status_line(&self) -> String {
        if self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::SeekNextClosest
            && let Some(question) = self.fleet_order_seek_next_closest()
        {
            return question.notice;
        }
        if self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::OrderConfirm
            && let (Some(mission_code), Ok(destination)) = (
                self.fleet_overlay.order_mission_code,
                self.resolve_order_split_target(),
            )
        {
            return match self.fleet_overlay.order_scope {
                FleetOrderScope::Group => self
                    .fleet_group_confirmation_eta_message(mission_code, destination)
                    .unwrap_or_else(|| {
                        format!(
                            "Confirm [{:02},{:02}] for {}.",
                            destination[0],
                            destination[1],
                            self.fleet_order_new_order_label()
                        )
                    }),
                _ => format!(
                    "Confirm [{:02},{:02}] for {}.",
                    destination[0],
                    destination[1],
                    self.fleet_order_new_order_label()
                ),
            };
        }
        fleet_target_status_line(self.fleet_overlay.order_mission_code)
    }

    pub(crate) fn fleet_order_target_prompt(&self) -> String {
        match fleet_target_input_kind(self.fleet_overlay.order_mission_code) {
            nc_engine::FleetTargetInputKind::StarbaseId => "Starbase # ".to_string(),
            nc_engine::FleetTargetInputKind::FleetId => "Fleet # ".to_string(),
            nc_engine::FleetTargetInputKind::Coordinates
            | nc_engine::FleetTargetInputKind::None => "Target ".to_string(),
        }
    }

    pub(crate) fn fleet_order_target_default_value(&self) -> String {
        match fleet_target_input_kind(self.fleet_overlay.order_mission_code) {
            nc_engine::FleetTargetInputKind::StarbaseId => self
                .default_starbase_target_for_scope()
                .map(|row| row.base_id.to_string())
                .unwrap_or_else(|| "1".to_string()),
            nc_engine::FleetTargetInputKind::FleetId => self
                .default_host_fleet_for_scope()
                .map(|row| row.fleet_number.to_string())
                .unwrap_or_else(|| "1".to_string()),
            nc_engine::FleetTargetInputKind::Coordinates
            | nc_engine::FleetTargetInputKind::None => self
                .default_target_coords_for_scope()
                .map(|target| format!("{},{}", target[0], target[1]))
                .unwrap_or_default(),
        }
    }

    pub(crate) fn fleet_order_target_x_default_value(&self) -> String {
        self.default_target_coords_for_scope()
            .map(|coords| format!("{:02}", coords[0]))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_order_target_x_display_input(&self) -> String {
        self.fleet_overlay.order_target_x_input.clone()
    }

    pub(crate) fn fleet_order_target_y_default_value(&self) -> String {
        self.default_target_y_value_for_scope()
            .map(|value| format!("{value:02}"))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_order_target_y_display_input(&self) -> String {
        self.fleet_overlay.order_target_y_input.clone()
    }

    pub(crate) fn fleet_order_current_order_label(&self) -> String {
        self.selected_fleet_order_row()
            .and_then(|row| fleet_mission_option(row.order_code))
            .map(|option| option.mission.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(crate) fn fleet_order_new_order_label(&self) -> String {
        self.fleet_overlay
            .order_mission_code
            .and_then(fleet_mission_option)
            .map(|option| option.mission.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(crate) fn fleet_order_confirm_target_label(&self) -> String {
        match fleet_target_input_kind(self.fleet_overlay.order_mission_code) {
            nc_engine::FleetTargetInputKind::Coordinates => self
                .resolve_order_split_target()
                .map(|destination| format!("({:02},{:02})", destination[0], destination[1]))
                .unwrap_or_else(|_| self.fleet_order_target_status_line()),
            nc_engine::FleetTargetInputKind::StarbaseId => self
                .resolve_starbase_target_for_current_mission()
                .map(|row| format!("Starbase #{}", row.base_id))
                .unwrap_or_else(|| self.fleet_order_target_status_line()),
            nc_engine::FleetTargetInputKind::FleetId => self
                .resolve_host_fleet_for_current_mission()
                .map(|row| format!("Fleet #{}", row.fleet_number))
                .unwrap_or_else(|| self.fleet_order_target_status_line()),
            nc_engine::FleetTargetInputKind::None => self.fleet_order_target_status_line(),
        }
    }

    pub(crate) fn fleet_order_confirmation_eta_label(&self) -> Option<String> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        let destination = self.resolve_order_split_target().ok()?;
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => {
                self.fleet_group_confirmation_eta_label(mission_code, destination)
            }
            _ => {
                let row = self.selected_fleet_order_row()?;
                let estimate = self.fleet_target_eta_estimate(&row, mission_code, destination);
                Some(self.format_target_eta_label(None, estimate))
            }
        }
    }

    pub(crate) fn selected_fleet_order_row(&self) -> Option<OrderFleetRow> {
        match self.fleet_overlay.active_row_key {
            Some(FleetOverlayRowKey::Fleet(record_index)) => self
                .owned_fleet_rows_for_orders()
                .into_iter()
                .find(|row| row.fleet_record_index_1_based == record_index),
            _ => None,
        }
    }

    pub(crate) fn selected_fleet_order_row_from_table(&self) -> Option<OrderFleetRow> {
        let rows = fleet_list::table_rows(self);
        let selected = self
            .fleet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        let row = rows.get(selected)?;
        match row.key {
            FleetOverlayRowKey::Fleet(record_index) => self
                .owned_fleet_rows_for_orders()
                .into_iter()
                .find(|candidate| candidate.fleet_record_index_1_based == record_index),
            FleetOverlayRowKey::Starbase(_) => None,
        }
    }

    pub(crate) fn selected_starbase_move_row(&self) -> Option<OrderStarbaseRow> {
        match self.fleet_overlay.active_row_key {
            Some(FleetOverlayRowKey::Starbase(record_index)) => self
                .owned_starbase_rows_for_orders()
                .into_iter()
                .find(|row| row.base_record_index_1_based == record_index),
            _ => None,
        }
    }

    fn first_enabled_fleet_mission_index(&self) -> Option<usize> {
        self.fleet_mission_picker_enabled_flags()
            .iter()
            .position(|flag| *flag)
    }

    fn sync_fleet_mission_picker_cursor_to_input(&mut self) {
        let rows = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| vec![format!("{:02}", option.code)])
            .collect::<Vec<_>>();
        let Some(index) = crate::dashboard::table_selection::find_typed_jump_index(
            &rows,
            0,
            &self.fleet_overlay.mission_picker_input,
        ) else {
            return;
        };
        if self
            .fleet_mission_picker_enabled_flags()
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            self.fleet_overlay.mission_picker_cursor = index;
        }
    }

    fn resolve_target_axis_input(
        &self,
        input: &str,
        default: Option<u8>,
        axis_label: &str,
    ) -> Result<u8, String> {
        let max = map_size_for_player_count(self.game_data.conquest.player_count());
        let raw = input.trim();
        let value = if raw.is_empty() {
            default.ok_or_else(|| format!("Enter {axis_label} from 1 to {max}."))?
        } else {
            raw.parse::<u8>()
                .map_err(|_| format!("Enter {axis_label} from 1 to {max}."))?
        };
        if value == 0 || value > max {
            return Err(format!("Enter {axis_label} from 1 to {max}."));
        }
        Ok(value)
    }

    fn resolve_fleet_order_split_target(&self) -> Result<[u8; 2], String> {
        let default_x = self
            .default_target_coords_for_scope()
            .map(|coords| coords[0]);
        let default_y = self.default_target_y_value_for_scope();
        Ok([
            self.resolve_target_axis_input(
                &self.fleet_overlay.order_target_x_input,
                default_x,
                "XX",
            )?,
            self.resolve_target_axis_input(
                &self.fleet_overlay.order_target_y_input,
                default_y,
                "YY",
            )?,
        ])
    }

    pub(crate) fn resolve_order_split_target(&self) -> Result<[u8; 2], String> {
        self.resolve_fleet_order_split_target()
    }

    fn validate_fleet_target_for_mission(
        &self,
        mission_code: u8,
        destination: [u8; 2],
    ) -> Result<(), String> {
        let target_planet = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| planet.coords_raw() == destination);
        if fleet_order_target_requires_planet_system(mission_code) && target_planet.is_none() {
            return Err("That mission needs a system with a planet at the target.".to_string());
        }
        if fleet_order_target_rejects_owned_planet(mission_code)
            && target_planet
                .map(|planet| {
                    planet.owner_empire_slot_raw() as usize == self.player_record_index_1_based
                })
                .unwrap_or(false)
        {
            return Err("You cannot send that mission to your own world.".to_string());
        }
        if fleet_order_target_rejects_owned_scout_target(mission_code)
            && target_planet
                .map(|planet| {
                    planet.owner_empire_slot_raw() as usize == self.player_record_index_1_based
                })
                .unwrap_or(false)
        {
            return Err("You cannot scout your own planet or system.".to_string());
        }
        if fleet_order_target_requires_owned_planet(mission_code)
            && target_planet
                .map(|planet| {
                    planet.owner_empire_slot_raw() as usize != self.player_record_index_1_based
                })
                .unwrap_or(true)
        {
            return Err("That mission requires one of your owned planets.".to_string());
        }
        if mission_code == Order::ColonizeWorld.to_raw() {
            let selected_records = self.selected_fleet_order_record_indexes();
            if selected_records.is_empty() {
                return Err(match self.fleet_overlay.order_scope {
                    FleetOrderScope::Group => "Selected fleets are no longer available.",
                    _ => "Selected fleet is no longer available.",
                }
                .to_string());
            }
            self.game_data
                .validate_friendly_colonize_target_available(
                    self.player_record_index_1_based as u8,
                    destination,
                    &selected_records,
                )
                .map_err(|err| match err {
                    nc_data::FleetOrderValidationError::DuplicateFriendlyColonizeTarget {
                        ..
                    } => {
                        if self.fleet_overlay.order_scope == FleetOrderScope::Group {
                            "You cannot order multiple ETAC fleets to colonize the same world."
                                .to_string()
                        } else {
                            "Another one of your ETAC fleets is already ordered to colonize that world."
                                .to_string()
                        }
                    }
                    other => other.to_string(),
                })?;
        }
        Ok(())
    }

    fn apply_fleet_single_order(
        &mut self,
        mission_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.selected_fleet_order_row() else {
            self.fleet_overlay.order_status =
                Some("Selected fleet is no longer available.".to_string());
            return Ok(());
        };
        let speed = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based.saturating_sub(1))
            .map(|fleet| {
                let speed = fleet.current_speed();
                if speed == 0 { fleet.max_speed() } else { speed }
            })
            .unwrap_or(selected_row.current_speed);
        self.game_data.set_fleet_order(
            selected_row.fleet_record_index_1_based,
            speed,
            mission_code,
            target,
            aux0,
            aux1,
        )?;
        self.stage_hosted_fleet_order(
            selected_row.fleet_record_index_1_based,
            speed,
            mission_code,
            target,
            aux0,
            aux1,
        );
        self.save_and_refresh_runtime()?;
        self.reselect_fleet_overlay_row(FleetOverlayRowKey::Fleet(
            selected_row.fleet_record_index_1_based,
        ));
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn apply_fleet_group_order(
        &mut self,
        mission_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let selected_rows = self.selected_group_order_rows();
        if selected_rows.is_empty() {
            self.fleet_overlay.order_status = Some("Select at least one fleet.".to_string());
            return Ok(());
        }
        // A lone fleet keeps the target its order prompt chose.
        let seek_home_targets =
            if mission_code == Order::SeekHome.to_raw() && selected_rows.len() > 1 {
                let records = selected_rows
                    .iter()
                    .map(|row| row.fleet_record_index_1_based)
                    .collect::<Vec<_>>();
                automatic_order_targets(&self.game_data, mission_code, &records)?
            } else {
                None
            };
        for (index, row) in selected_rows.iter().enumerate() {
            let target = seek_home_targets
                .as_ref()
                .map_or(target, |targets| targets[index]);
            let speed = self
                .game_data
                .fleets
                .records
                .get(row.fleet_record_index_1_based.saturating_sub(1))
                .map(|fleet| {
                    let speed = fleet.current_speed();
                    if speed == 0 { fleet.max_speed() } else { speed }
                })
                .unwrap_or(row.current_speed);
            self.game_data.set_fleet_order(
                row.fleet_record_index_1_based,
                speed,
                mission_code,
                target,
                aux0,
                aux1,
            )?;
            self.stage_hosted_fleet_order(
                row.fleet_record_index_1_based,
                speed,
                mission_code,
                target,
                aux0,
                aux1,
            );
        }
        let reselect = self.fleet_overlay.active_row_key;
        self.save_and_refresh_runtime()?;
        self.fleet_overlay.clear_group_selection();
        if let Some(key) = reselect {
            self.reselect_fleet_overlay_row(key);
        }
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn apply_fleet_single_join_order(
        &mut self,
        host: OrderFleetRow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.selected_fleet_order_row() else {
            self.fleet_overlay.order_status =
                Some("Selected fleet is no longer available.".to_string());
            return Ok(());
        };
        self.game_data.set_join_fleet_order(
            self.player_record_index_1_based,
            selected_row.fleet_record_index_1_based,
            host.fleet_record_index_1_based,
        )?;
        self.stage_hosted_fleet_join(
            selected_row.fleet_record_index_1_based,
            host.fleet_record_index_1_based,
        );
        self.save_and_refresh_runtime()?;
        self.reselect_fleet_overlay_row(FleetOverlayRowKey::Fleet(
            selected_row.fleet_record_index_1_based,
        ));
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn apply_fleet_group_join_order(
        &mut self,
        host: OrderFleetRow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let selected_rows = self.selected_group_order_rows();
        if selected_rows.is_empty() {
            self.fleet_overlay.order_status = Some("Select at least one fleet.".to_string());
            return Ok(());
        }
        for row in &selected_rows {
            self.game_data.set_join_fleet_order(
                self.player_record_index_1_based,
                row.fleet_record_index_1_based,
                host.fleet_record_index_1_based,
            )?;
            self.stage_hosted_fleet_join(
                row.fleet_record_index_1_based,
                host.fleet_record_index_1_based,
            );
        }
        let reselect = self.fleet_overlay.active_row_key;
        self.save_and_refresh_runtime()?;
        self.fleet_overlay.clear_group_selection();
        if let Some(key) = reselect {
            self.reselect_fleet_overlay_row(key);
        }
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn resolve_fleet_order_starbase_target_for_current_mission(&self) -> Option<OrderStarbaseRow> {
        let default_base_id = self.fleet_order_default_starbase()?.base_id;
        let base_id = resolve_default_u16_input(&self.fleet_overlay.order_input, default_base_id)?;
        self.owned_starbase_rows_for_orders()
            .into_iter()
            .find(|row| row.base_id == base_id)
    }

    fn resolve_fleet_order_host_fleet_for_current_mission(&self) -> Option<OrderFleetRow> {
        let default_fleet_number = self.fleet_order_default_host_fleet()?.fleet_number;
        let fleet_number =
            resolve_default_u16_input(&self.fleet_overlay.order_input, default_fleet_number)?;
        let selected_record = self.selected_fleet_order_row()?.fleet_record_index_1_based;
        self.owned_fleet_rows_for_orders().into_iter().find(|row| {
            row.fleet_number == fleet_number && row.fleet_record_index_1_based != selected_record
        })
    }

    fn fleet_order_default_starbase(&self) -> Option<OrderStarbaseRow> {
        let anchor = self.selected_fleet_order_row()?.coords;
        let target = default_starbase_target(
            &self.game_data,
            self.player_record_index_1_based as u8,
            anchor,
        )?;
        self.owned_starbase_rows_for_orders()
            .into_iter()
            .find(|row| row.base_record_index_1_based == target.base_record_index_1_based)
    }

    fn fleet_order_default_host_fleet(&self) -> Option<OrderFleetRow> {
        let selected = self.selected_fleet_order_row()?;
        let excluded = BTreeSet::from([selected.fleet_record_index_1_based]);
        let target = default_host_fleet_target(
            &self.game_data,
            self.player_record_index_1_based as u8,
            selected.coords,
            &excluded,
        )?;
        self.owned_fleet_rows_for_orders()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == target.fleet_record_index_1_based)
    }

    fn fleet_order_default_target_for_mission(&self, mission_code: u8) -> Option<[u8; 2]> {
        let selected = self
            .selected_fleet_order_row()
            .map(|row| vec![row])
            .unwrap_or_default();
        self.recommended_fleet_target(mission_code, &selected, BTreeSet::new())
    }

    fn fleet_order_default_target_coords(&self) -> Option<[u8; 2]> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        self.fleet_order_default_target_for_mission(mission_code)
    }

    fn fleet_order_default_target_y_value(&self) -> Option<u8> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        let Some(selected) = self.selected_fleet_order_row() else {
            return None;
        };
        let snapshots = self.order_intel_snapshots();
        let selected_records = BTreeSet::from([selected.fleet_record_index_1_based]);
        recommended_coordinate_target_y_for_entered_x(
            &self.game_data,
            &snapshots,
            self.player_record_index_1_based as u8,
            mission_code,
            selected.coords,
            &selected_records,
            self.fleet_overlay.order_target_x_input.trim(),
        )
    }

    fn finalize_starbase_destination(
        &mut self,
        row: OrderStarbaseRow,
        destination: [u8; 2],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_hosted_mode() {
            self.fleet_overlay.order_status =
                Some("Hosted play does not support starbase move orders yet.".to_string());
            return Ok(());
        }
        if destination == row.coords {
            self.game_data.halt_starbase(
                self.player_record_index_1_based,
                row.base_record_index_1_based,
            )?;
        } else {
            self.game_data.set_starbase_destination(
                self.player_record_index_1_based,
                row.base_record_index_1_based,
                destination,
            )?;
        }
        self.append_report_block(self.starbase_move_report_text(row, destination));
        self.save_and_refresh_runtime()?;
        self.reselect_fleet_overlay_row(FleetOverlayRowKey::Starbase(
            row.base_record_index_1_based,
        ));
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn starbase_move_report_text(&self, row: OrderStarbaseRow, destination: [u8; 2]) -> String {
        if destination == row.coords {
            return format!(
                "Starbase {} halted at [{:02},{:02}].",
                row.base_id, destination[0], destination[1]
            );
        }
        let mut text = format!(
            "Starbase {} is moving to [{:02},{:02}].",
            row.base_id, destination[0], destination[1]
        );
        let guard_fleets = guard_fleet_numbers_for_starbase(
            &self.game_data,
            self.player_record_index_1_based,
            row.base_id,
        );
        if let Some(clause) = format_guard_fleet_clause(&guard_fleets) {
            text.push(' ');
            text.push_str(&clause);
        }
        text
    }

    fn reselect_fleet_overlay_row(&mut self, key: FleetOverlayRowKey) {
        self.enforce_valid_fleet_filter();
        if let Some(index) = fleet_list::table_rows(self)
            .iter()
            .position(|row| row.key == key)
        {
            self.fleet_overlay.selected = index;
        }
    }

    fn owned_fleet_rows_for_orders(&self) -> Vec<OrderFleetRow> {
        let mut rows = owned_fleet_targets(&self.game_data, self.player_record_index_1_based as u8)
            .into_iter()
            .map(|row| OrderFleetRow {
                fleet_record_index_1_based: row.fleet_record_index_1_based,
                fleet_number: row.fleet_number,
                coords: row.coords,
                target_coords: row.target_coords,
                order_code: row.order_code,
                current_speed: row.current_speed,
                max_speed: row.max_speed,
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.fleet_number);
        rows
    }

    fn owned_starbase_rows_for_orders(&self) -> Vec<OrderStarbaseRow> {
        let mut rows =
            owned_starbase_targets(&self.game_data, self.player_record_index_1_based as u8)
                .into_iter()
                .map(|row| OrderStarbaseRow {
                    base_record_index_1_based: row.base_record_index_1_based,
                    base_id: row.base_id,
                    coords: row.coords,
                    destination_coords: row.destination_coords,
                })
                .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.base_id);
        rows
    }

    pub(crate) fn normalize_selected_fleet_order_selection(&mut self) {
        let valid = self
            .owned_fleet_rows_for_orders()
            .into_iter()
            .map(|row| row.fleet_record_index_1_based)
            .collect::<BTreeSet<_>>();
        self.fleet_overlay
            .selected_fleet_record_indexes
            .retain(|record| valid.contains(record));
    }

    pub(crate) fn toggle_selected_fleet_row_for_group_order(&mut self) {
        self.normalize_selected_fleet_order_selection();
        let rows = fleet_list::table_rows(self);
        let selected = self
            .fleet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        let Some(row) = rows.get(selected) else {
            return;
        };
        let FleetOverlayRowKey::Fleet(record_index) = row.key else {
            return;
        };
        if !self
            .fleet_overlay
            .selected_fleet_record_indexes
            .insert(record_index)
        {
            self.fleet_overlay
                .selected_fleet_record_indexes
                .remove(&record_index);
        }
        self.fleet_overlay.order_status = None;
    }

    pub(crate) fn selected_group_order_rows(&self) -> Vec<OrderFleetRow> {
        let mut rows = self
            .owned_fleet_rows_for_orders()
            .into_iter()
            .filter(|row| {
                self.fleet_overlay
                    .selected_fleet_record_indexes
                    .contains(&row.fleet_record_index_1_based)
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.fleet_number);
        rows
    }

    pub(crate) fn selected_group_order_fleet_summary(&self) -> String {
        self.selected_group_order_rows()
            .into_iter()
            .map(|row| format!("{:02}", row.fleet_number))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn change_target_rows(&self) -> Vec<OrderFleetRow> {
        if self.fleet_overlay.selected_fleet_record_indexes.is_empty() {
            self.selected_fleet_order_row_from_table()
                .into_iter()
                .collect()
        } else {
            self.selected_group_order_rows()
        }
    }

    fn fleet_change_value_default(&self, field: FleetOverlayChangeField) -> String {
        let rows = self.change_target_rows();
        if rows.is_empty() {
            return String::new();
        }
        match field {
            FleetOverlayChangeField::Roe => rows
                .first()
                .map(|row| {
                    self.game_data
                        .fleets
                        .records
                        .get(row.fleet_record_index_1_based - 1)
                        .map(|fleet| fleet.rules_of_engagement())
                        .unwrap_or(0)
                })
                .filter(|roe| {
                    rows.iter().all(|row| {
                        self.game_data
                            .fleets
                            .records
                            .get(row.fleet_record_index_1_based - 1)
                            .map(|fleet| fleet.rules_of_engagement() == *roe)
                            .unwrap_or(false)
                    })
                })
                .map(|roe| roe.to_string())
                .unwrap_or_default(),
            FleetOverlayChangeField::Id => rows[0].fleet_number.to_string(),
            FleetOverlayChangeField::Speed => rows
                .first()
                .map(|row| row.current_speed)
                .filter(|speed| rows.iter().all(|row| row.current_speed == *speed))
                .map(|speed| speed.to_string())
                .unwrap_or_default(),
        }
    }

    pub(crate) fn fleet_order_is_group_scope(&self) -> bool {
        self.fleet_overlay.order_scope == FleetOrderScope::Group
    }

    fn parse_fleet_transfer_class_code(&self, raw: &str) -> Option<FleetOverlayTransferClass> {
        match raw {
            "BB" => Some(FleetOverlayTransferClass::Battleships),
            "CA" => Some(FleetOverlayTransferClass::Cruisers),
            "DD" => Some(FleetOverlayTransferClass::Destroyers),
            "TT*" => Some(FleetOverlayTransferClass::FullTransports),
            "TT" => Some(FleetOverlayTransferClass::EmptyTransports),
            "SC" => Some(FleetOverlayTransferClass::Scouts),
            "ET" => Some(FleetOverlayTransferClass::Etacs),
            _ => None,
        }
    }

    fn fleet_transfer_class_label(&self, class: FleetOverlayTransferClass) -> &'static str {
        match class {
            FleetOverlayTransferClass::Battleships => "BB",
            FleetOverlayTransferClass::Cruisers => "CA",
            FleetOverlayTransferClass::Destroyers => "DD",
            FleetOverlayTransferClass::FullTransports => "TT*",
            FleetOverlayTransferClass::EmptyTransports => "TT",
            FleetOverlayTransferClass::Scouts => "SC",
            FleetOverlayTransferClass::Etacs => "ET",
        }
    }

    fn fleet_transfer_available_for_class(&self, class: FleetOverlayTransferClass) -> u16 {
        let Some(donor_index) = self.fleet_overlay.transfer_donor_record_index_1_based else {
            return 0;
        };
        let Some(fleet) = self.game_data.fleets.records.get(donor_index - 1) else {
            return 0;
        };
        match class {
            FleetOverlayTransferClass::Battleships => fleet
                .battleship_count()
                .saturating_sub(self.fleet_overlay.transfer_selection.battleships),
            FleetOverlayTransferClass::Cruisers => fleet
                .cruiser_count()
                .saturating_sub(self.fleet_overlay.transfer_selection.cruisers),
            FleetOverlayTransferClass::Destroyers => fleet
                .destroyer_count()
                .saturating_sub(self.fleet_overlay.transfer_selection.destroyers),
            FleetOverlayTransferClass::FullTransports => fleet
                .army_count()
                .saturating_sub(self.fleet_overlay.transfer_selection.full_transports),
            FleetOverlayTransferClass::EmptyTransports => fleet
                .troop_transport_count()
                .saturating_sub(fleet.army_count())
                .saturating_sub(self.fleet_overlay.transfer_selection.empty_transports),
            FleetOverlayTransferClass::Scouts => fleet
                .scout_count()
                .saturating_sub(self.fleet_overlay.transfer_selection.scouts),
            FleetOverlayTransferClass::Etacs => fleet
                .etac_count()
                .saturating_sub(self.fleet_overlay.transfer_selection.etacs),
        }
    }

    pub(crate) fn fleet_transfer_prompt_and_default(&self) -> (String, String) {
        match self.fleet_overlay.transfer_mode {
            FleetOverlayTransferMode::ChoosingClass => (
                "Class <BB,CA,DD,TT*,TT,SC,ET,C,X> ".to_string(),
                String::new(),
            ),
            FleetOverlayTransferMode::EnteringQuantity(class) => (
                format!(
                    "{} to stage (max {}) ",
                    self.fleet_transfer_class_label(class),
                    self.fleet_transfer_available_for_class(class)
                ),
                "1".to_string(),
            ),
        }
    }

    pub(crate) fn fleet_transfer_donor_row(&self) -> Option<OrderFleetRow> {
        self.fleet_overlay
            .transfer_donor_record_index_1_based
            .and_then(|idx| {
                self.owned_fleet_rows_for_orders()
                    .into_iter()
                    .find(|row| row.fleet_record_index_1_based == idx)
            })
    }

    pub(crate) fn fleet_transfer_host_row(&self) -> Option<OrderFleetRow> {
        self.fleet_overlay
            .transfer_host_record_index_1_based
            .and_then(|idx| {
                self.owned_fleet_rows_for_orders()
                    .into_iter()
                    .find(|row| row.fleet_record_index_1_based == idx)
            })
    }

    fn format_transfer_summary_from_selection(&self, selection: FleetDetachSelection) -> String {
        let mut parts = Vec::new();
        for (label, count) in [
            ("SC", selection.scouts),
            ("BB", selection.battleships),
            ("CA", selection.cruisers),
            ("DD", selection.destroyers),
            ("TT*", selection.full_transports),
            ("TT", selection.empty_transports),
            ("ET", selection.etacs),
        ] {
            if count > 0 {
                parts.push(format!("{label}={count}"));
            }
        }
        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(" ")
        }
    }

    pub(crate) fn fleet_transfer_staged_summary(&self) -> String {
        self.format_transfer_summary_from_selection(self.fleet_overlay.transfer_selection.clone())
    }

    fn finish_fleet_transfer_prompt(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(donor) = self.fleet_overlay.transfer_donor_record_index_1_based else {
            return Err("Select two fleets for transfer.".into());
        };
        let Some(host) = self.fleet_overlay.transfer_host_record_index_1_based else {
            return Err("Select two fleets for transfer.".into());
        };
        if self.fleet_overlay.transfer_selection.total_ships() == 0 {
            self.fleet_overlay.aux_status =
                Some("Stage at least one ship before committing.".to_string());
            return Ok(());
        }
        let selection = self.fleet_overlay.transfer_selection.clone();
        self.game_data.transfer_ships_between_fleets(
            self.player_record_index_1_based,
            donor,
            host,
            selection.clone(),
        )?;
        self.stage_hosted_fleet_transfer(donor, host, selection);
        self.save_and_refresh_runtime()?;
        self.fleet_overlay.clear_group_selection();
        self.cancel_fleet_aux_prompt();
        Ok(())
    }

    fn selected_fleet_order_record_indexes(&self) -> BTreeSet<usize> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.fleet_overlay.selected_fleet_record_indexes.clone(),
            _ => self
                .selected_fleet_order_row()
                .map(|row| BTreeSet::from([row.fleet_record_index_1_based]))
                .unwrap_or_default(),
        }
    }

    fn fleet_row_supports_mission(&self, row: &OrderFleetRow, mission_code: u8) -> bool {
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(row.fleet_record_index_1_based.saturating_sub(1))
        else {
            return false;
        };
        fleet_record_supports_mission_code(fleet, mission_code)
    }

    fn default_starbase_target_for_scope(&self) -> Option<OrderStarbaseRow> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.fleet_group_default_starbase(),
            _ => self.fleet_order_default_starbase(),
        }
    }

    fn default_host_fleet_for_scope(&self) -> Option<OrderFleetRow> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.fleet_group_default_host_fleet(),
            _ => self.fleet_order_default_host_fleet(),
        }
    }

    fn default_target_coords_for_scope(&self) -> Option<[u8; 2]> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.fleet_group_default_target_coords(),
            _ => self.fleet_order_default_target_coords(),
        }
    }

    fn default_target_y_value_for_scope(&self) -> Option<u8> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.fleet_group_default_target_y_value(),
            _ => self.fleet_order_default_target_y_value(),
        }
    }

    fn resolve_starbase_target_for_current_mission(&self) -> Option<OrderStarbaseRow> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => {
                self.resolve_fleet_group_starbase_target_for_current_mission()
            }
            _ => self.resolve_fleet_order_starbase_target_for_current_mission(),
        }
    }

    fn resolve_host_fleet_for_current_mission(&self) -> Option<OrderFleetRow> {
        match self.fleet_overlay.order_scope {
            FleetOrderScope::Group => self.resolve_fleet_group_host_fleet_for_current_mission(),
            _ => self.resolve_fleet_order_host_fleet_for_current_mission(),
        }
    }

    fn fleet_group_default_target_for_mission(&self, mission_code: u8) -> Option<[u8; 2]> {
        let selected = self.selected_group_order_rows();
        self.recommended_fleet_target(
            mission_code,
            &selected,
            self.fleet_overlay.selected_fleet_record_indexes.clone(),
        )
    }

    fn fleet_group_default_target_coords(&self) -> Option<[u8; 2]> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        self.fleet_group_default_target_for_mission(mission_code)
    }

    fn fleet_group_default_target_y_value(&self) -> Option<u8> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        let selected = self.selected_group_order_rows();
        let anchor = selected.first()?.coords;
        let snapshots = self.order_intel_snapshots();
        recommended_coordinate_target_y_for_entered_x(
            &self.game_data,
            &snapshots,
            self.player_record_index_1_based as u8,
            mission_code,
            anchor,
            &self.fleet_overlay.selected_fleet_record_indexes,
            self.fleet_overlay.order_target_x_input.trim(),
        )
    }

    fn fleet_group_default_starbase(&self) -> Option<OrderStarbaseRow> {
        let anchor = self.selected_group_order_rows().first()?.coords;
        let target = default_starbase_target(
            &self.game_data,
            self.player_record_index_1_based as u8,
            anchor,
        )?;
        self.owned_starbase_rows_for_orders()
            .into_iter()
            .find(|row| row.base_record_index_1_based == target.base_record_index_1_based)
    }

    fn fleet_group_default_host_fleet(&self) -> Option<OrderFleetRow> {
        let anchor = self.selected_group_order_rows().first()?.coords;
        let target = default_host_fleet_target(
            &self.game_data,
            self.player_record_index_1_based as u8,
            anchor,
            &self.fleet_overlay.selected_fleet_record_indexes,
        )?;
        self.owned_fleet_rows_for_orders()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == target.fleet_record_index_1_based)
    }

    fn resolve_fleet_group_starbase_target_for_current_mission(&self) -> Option<OrderStarbaseRow> {
        let default_base_id = self.fleet_group_default_starbase()?.base_id;
        let base_id = resolve_default_u16_input(&self.fleet_overlay.order_input, default_base_id)?;
        self.owned_starbase_rows_for_orders()
            .into_iter()
            .find(|row| row.base_id == base_id)
    }

    fn resolve_fleet_group_host_fleet_for_current_mission(&self) -> Option<OrderFleetRow> {
        let default_fleet_number = self.fleet_group_default_host_fleet()?.fleet_number;
        let fleet_number =
            resolve_default_u16_input(&self.fleet_overlay.order_input, default_fleet_number)?;
        self.owned_fleet_rows_for_orders().into_iter().find(|row| {
            row.fleet_number == fleet_number
                && !self
                    .fleet_overlay
                    .selected_fleet_record_indexes
                    .contains(&row.fleet_record_index_1_based)
        })
    }

    fn fleet_order_has_target_available(&self, mission_code: u8) -> bool {
        let anchor = self
            .selected_fleet_order_row()
            .map(|row| row.coords)
            .unwrap_or([self.crosshair_x, self.crosshair_y]);
        let selected_records = self
            .selected_fleet_order_row()
            .map(|row| BTreeSet::from([row.fleet_record_index_1_based]))
            .unwrap_or_default();
        target_available_for_mission(
            &self.game_data,
            &self.order_intel_snapshots(),
            self.player_record_index_1_based as u8,
            mission_code,
            anchor,
            &selected_records,
        )
    }

    fn fleet_group_has_target_available(&self, mission_code: u8) -> bool {
        let selected = self.selected_group_order_rows();
        let anchor = selected
            .first()
            .map(|row| row.coords)
            .unwrap_or([self.crosshair_x, self.crosshair_y]);
        target_available_for_mission(
            &self.game_data,
            &self.order_intel_snapshots(),
            self.player_record_index_1_based as u8,
            mission_code,
            anchor,
            &self.fleet_overlay.selected_fleet_record_indexes,
        )
    }

    fn recommended_fleet_target(
        &self,
        mission_code: u8,
        selected_rows: &[OrderFleetRow],
        selected_records: BTreeSet<usize>,
    ) -> Option<[u8; 2]> {
        let anchor = selected_rows
            .first()
            .map(|row| row.coords)
            .unwrap_or([self.crosshair_x, self.crosshair_y]);
        recommended_coordinate_target(
            &self.game_data,
            &self.order_intel_snapshots(),
            self.player_record_index_1_based as u8,
            mission_code,
            anchor,
            &selected_records,
        )
    }

    fn fleet_target_unavailable_message(&self, mission_code: u8) -> String {
        match mission_code {
            4 => "You have no starbases available to guard.".to_string(),
            12 => "No colonize target available.".to_string(),
            13 => "You need another fleet available to join.".to_string(),
            _ => "No valid target available for that mission.".to_string(),
        }
    }

    fn fleet_group_confirmation_eta_message(
        &self,
        mission_code: u8,
        destination: [u8; 2],
    ) -> Option<String> {
        let selected = self.selected_group_order_rows();
        let (row, estimate) = selected
            .iter()
            .map(|row| {
                (
                    row,
                    self.fleet_target_eta_estimate(row, mission_code, destination),
                )
            })
            .max_by_key(|(_, estimate)| self.fleet_target_eta_sort_key(*estimate))?;
        let subject = if selected.len() == 1 {
            format!("Fleet {}", row.fleet_number)
        } else {
            format!("Slowest selected fleet (Fleet {})", row.fleet_number)
        };
        Some(self.format_target_eta_message(&subject, destination, estimate))
    }

    fn fleet_group_confirmation_eta_label(
        &self,
        mission_code: u8,
        destination: [u8; 2],
    ) -> Option<String> {
        let selected = self.selected_group_order_rows();
        let (row, estimate) = selected
            .iter()
            .map(|row| {
                (
                    row,
                    self.fleet_target_eta_estimate(row, mission_code, destination),
                )
            })
            .max_by_key(|(_, estimate)| self.fleet_target_eta_sort_key(*estimate))?;
        let subject = if selected.len() == 1 {
            None
        } else {
            Some(format!("Slowest Fleet {}", row.fleet_number))
        };
        Some(self.format_target_eta_label(subject.as_deref(), estimate))
    }

    fn fleet_target_eta_estimate(
        &self,
        row: &OrderFleetRow,
        mission_code: u8,
        destination: [u8; 2],
    ) -> nc_engine::FleetEtaEstimate {
        if fleet_target_input_kind(Some(mission_code))
            != nc_engine::FleetTargetInputKind::Coordinates
        {
            return nc_engine::FleetEtaEstimate::Arrived;
        }
        nc_engine::estimate_fleet_eta_to_destination(
            &self.game_data,
            row.fleet_record_index_1_based.saturating_sub(1),
            destination,
            false,
            true,
        )
    }

    fn fleet_target_eta_sort_key(&self, estimate: nc_engine::FleetEtaEstimate) -> (u8, u16) {
        match estimate {
            nc_engine::FleetEtaEstimate::Stopped => (3, 0),
            nc_engine::FleetEtaEstimate::Unreachable => (2, 0),
            nc_engine::FleetEtaEstimate::Arrived => (0, 0),
            nc_engine::FleetEtaEstimate::Years(years) => (1, years),
        }
    }

    fn format_target_eta_message(
        &self,
        subject: &str,
        destination: [u8; 2],
        estimate: nc_engine::FleetEtaEstimate,
    ) -> String {
        let target = format!("({:02},{:02})", destination[0], destination[1]);
        match estimate {
            nc_engine::FleetEtaEstimate::Arrived => format!(
                "{subject} reaches {target} in 0 year(s), arriving in {}.",
                self.game_data.conquest.game_year()
            ),
            nc_engine::FleetEtaEstimate::Years(years) => format!(
                "{subject} reaches {target} in {years} year(s), arriving in {}.",
                self.game_data.conquest.game_year() + years
            ),
            nc_engine::FleetEtaEstimate::Stopped => {
                format!("{subject} is stopped and cannot reach {target}.")
            }
            nc_engine::FleetEtaEstimate::Unreachable => {
                format!("No route found for {subject} to {target}.")
            }
        }
    }

    fn format_target_eta_label(
        &self,
        subject: Option<&str>,
        estimate: nc_engine::FleetEtaEstimate,
    ) -> String {
        let summary = match estimate {
            nc_engine::FleetEtaEstimate::Arrived => {
                format!("0 year(s), arrive {}", self.game_data.conquest.game_year())
            }
            nc_engine::FleetEtaEstimate::Years(years) => format!(
                "{years} year(s), arrive {}",
                self.game_data.conquest.game_year() + years
            ),
            nc_engine::FleetEtaEstimate::Stopped => "Stopped".to_string(),
            nc_engine::FleetEtaEstimate::Unreachable => "No route".to_string(),
        };
        match subject {
            Some(subject) => format!("{subject} - {summary}"),
            None => summary,
        }
    }
}

fn resolve_default_u16_input(input: &str, default: u16) -> Option<u16> {
    let raw = input.trim();
    if raw.is_empty() {
        Some(default)
    } else {
        raw.parse::<u16>().ok()
    }
}

fn resolve_default_coords_input(input: &str, default: [u8; 2]) -> Option<[u8; 2]> {
    let raw = input.trim();
    if raw.is_empty() {
        return Some(default);
    }
    let Some((left, right)) = raw.split_once(',') else {
        return None;
    };
    let x = left.trim().parse::<u8>().ok()?;
    let y = right.trim().parse::<u8>().ok()?;
    Some([x, y])
}

fn resolve_yes_no_input(input: &str, default: bool) -> bool {
    match input.trim().to_ascii_uppercase().as_str() {
        "" => default,
        "Y" | "YES" => true,
        "N" | "NO" => false,
        _ => default,
    }
}
