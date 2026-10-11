use nc_data::{EmpirePlanetEconomyRow, ProductionItemKind};
use nc_engine::{
    BUILD_UNITS, build_unit_spec_by_kind, planet_build_max_quantity, planet_build_orders,
    planet_build_specify_entries, planet_build_unavailable_message, planet_build_view,
    production_item_kind_code,
};

use crate::dashboard::coords::format_sector_coords_table;
use crate::dashboard::overlays::planet_list;

use super::state::{ActiveOverlay, ActivePopup, DashApp, HelpContext, PlanetOverlayPromptMode};

#[derive(Debug, Clone)]
pub(crate) struct PlanetBuildOverlayView {
    pub row: EmpirePlanetEconomyRow,
    pub points_left: u32,
}

impl DashApp {
    pub(crate) fn planet_build_view(&self) -> Option<PlanetBuildOverlayView> {
        let planet_record_index_1_based = self.planet_build_planet_record_index_1_based()?;
        let row = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_record_index_1_based)?;
        let view = planet_build_view(&self.game_data, &row).ok()?;
        Some(PlanetBuildOverlayView {
            row,
            points_left: view.points_left,
        })
    }

    pub(crate) fn planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let Some(view) = self.planet_build_view() else {
            return Ok(0);
        };
        planet_build_max_quantity(&self.game_data, &view.row, kind).map_err(Into::into)
    }

    pub(crate) fn planet_build_specify_entries(&self) -> Vec<nc_engine::PlanetBuildSpecifyEntry> {
        let Some(view) = self.planet_build_view() else {
            return Vec::new();
        };
        planet_build_specify_entries(view.points_left, &planet_build_orders_for_dash(self))
    }

    pub(crate) fn open_planet_build_specify(&mut self) {
        let Some(planet_record_index_1_based) = self.focus_planet_build_target_from_selection()
        else {
            return;
        };
        self.open_planet_build_specify_for_record(planet_record_index_1_based);
    }

    pub(crate) fn open_planet_build_specify_for_record(
        &mut self,
        planet_record_index_1_based: usize,
    ) {
        if !self.focus_planet_build_target(planet_record_index_1_based) {
            return;
        }
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.initialize_planet_build_selection();
        self.overlay = ActiveOverlay::PlanetList;
        self.clear_planet_overlay_footer_notice();
        self.planet_overlay
            .open_prompt(PlanetOverlayPromptMode::BuildSpecify);
    }

    pub(crate) fn select_previous_planet_build_kind(&mut self) {
        self.adjust_planet_build_selection(0, -1);
    }

    pub(crate) fn select_next_planet_build_kind(&mut self) {
        self.adjust_planet_build_selection(0, 1);
    }

    pub(crate) fn select_left_planet_build_kind(&mut self) {
        self.adjust_planet_build_selection(-1, 0);
    }

    pub(crate) fn select_right_planet_build_kind(&mut self) {
        self.adjust_planet_build_selection(1, 0);
    }

    pub(crate) fn open_selected_planet_build_quantity(&mut self) {
        let Some(kind) = self.selected_planet_build_kind() else {
            return;
        };
        let Ok(max_qty) = self.planet_build_max_quantity_for(kind) else {
            self.planet_overlay.build_unit_status = Some("No build budget available.".to_string());
            return;
        };
        if max_qty == 0 {
            self.planet_overlay.build_unit_status =
                Some(self.planet_build_unavailable_message(kind));
            return;
        }

        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay
            .open_prompt(PlanetOverlayPromptMode::BuildQuantity);
    }

    pub(crate) fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.planet_overlay.build_unit_input.len() < 2 {
            self.planet_overlay.build_unit_input.push(ch);
            self.planet_overlay.build_unit_status = None;
            self.sync_planet_build_selection_from_input();
        }
    }

    pub(crate) fn backspace_planet_build_unit_input(&mut self) {
        self.planet_overlay.build_unit_input.pop();
        self.planet_overlay.build_unit_status = None;
        self.sync_planet_build_selection_from_input();
    }

    pub(crate) fn submit_planet_build_browse_input(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw_input = self.planet_overlay.build_unit_input.trim();
        if raw_input == "0" {
            self.close_planet_build_overlay();
            return Ok(());
        }
        if raw_input.is_empty() {
            self.open_selected_planet_build_quantity();
            return Ok(());
        }
        let number = match raw_input.parse::<u8>() {
            Ok(value) => value,
            Err(_) => {
                self.planet_overlay.build_unit_status =
                    Some("Enter a valid unit number.".to_string());
                return Ok(());
            }
        };
        let Some(entry) = self
            .planet_build_specify_entries()
            .into_iter()
            .find(|entry| entry.number == number)
        else {
            self.planet_overlay.build_unit_status = Some("Enter a valid unit number.".to_string());
            return Ok(());
        };
        self.planet_overlay.build_selected_kind = Some(entry.kind);
        self.planet_overlay.build_unit_input.clear();
        self.open_selected_planet_build_quantity();
        Ok(())
    }

    pub(crate) fn append_planet_build_quantity_char(&mut self, ch: char) {
        if self.planet_overlay.build_quantity_input.len() < 3 {
            self.planet_overlay.build_quantity_input.push(ch);
            self.planet_overlay.build_quantity_status = None;
        }
    }

    pub(crate) fn backspace_planet_build_quantity_input(&mut self) {
        self.planet_overlay.build_quantity_input.pop();
        self.planet_overlay.build_quantity_status = None;
    }

    pub(crate) fn submit_planet_build_quantity(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.planet_overlay.build_selected_kind else {
            self.planet_overlay.close_prompt();
            return Ok(());
        };
        if build_unit_spec_by_kind(kind).is_none() {
            self.planet_overlay.close_prompt();
            return Ok(());
        }
        let max_qty = self.planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet_overlay.build_quantity_status =
                Some(self.planet_build_unavailable_message(kind));
            return Ok(());
        }

        let qty = if self.planet_overlay.build_quantity_input.trim().is_empty() {
            max_qty
        } else {
            match self
                .planet_overlay
                .build_quantity_input
                .trim()
                .parse::<u32>()
            {
                Ok(value) => value,
                Err(_) => {
                    self.planet_overlay.build_quantity_status =
                        Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };

        if qty == 0 {
            self.cancel_planet_build_quantity();
            return Ok(());
        }
        if qty > max_qty {
            self.planet_overlay.build_quantity_status =
                Some(format!("Enter a quantity from 0 to {}.", max_qty));
            return Ok(());
        }

        self.queue_planet_build_units(kind, qty, Some(max_qty))?;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.close_prompt();
        Ok(())
    }

    pub(crate) fn cancel_planet_build_quantity(&mut self) {
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.close_prompt();
    }

    pub(crate) fn queue_selected_planet_build_unit(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.selected_planet_build_kind() else {
            return Ok(());
        };
        let max_qty = self.planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet_overlay.build_unit_status =
                Some(self.planet_build_unavailable_message(kind));
            return Ok(());
        }
        self.queue_planet_build_units(kind, 1, Some(max_qty))?;
        if let Some(status) = self.planet_overlay.build_quantity_status.take() {
            self.planet_overlay.build_unit_status = Some(status);
        }
        Ok(())
    }

    pub(crate) fn remove_selected_planet_build_unit(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.selected_planet_build_kind() else {
            return Ok(());
        };
        let queued_qty = self
            .planet_build_specify_entries()
            .into_iter()
            .find(|entry| entry.kind == kind)
            .map(|entry| entry.queued_qty)
            .unwrap_or(0);
        if queued_qty == 0 {
            self.planet_overlay.build_unit_status =
                Some("No queued units of this type.".to_string());
            return Ok(());
        }

        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based()
        else {
            self.close_planet_build_overlay();
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            return Ok(());
        };
        self.game_data.remove_planet_build_points_by_kind(
            planet_record_index_1_based,
            kind,
            unit.cost,
        )?;
        self.stage_hosted_planet_remove_build(
            planet_record_index_1_based,
            1,
            production_item_kind_code(kind),
        );
        self.save_and_refresh_runtime()?;
        self.reselect_planet_overlay_row(planet_record_index_1_based);
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_quantity_status = None;
        Ok(())
    }

    pub(crate) fn clear_selected_planet_build_kind_queue(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.selected_planet_build_kind() else {
            return Ok(());
        };
        let queued_qty = self
            .planet_build_specify_entries()
            .into_iter()
            .find(|entry| entry.kind == kind)
            .map(|entry| entry.queued_qty)
            .unwrap_or(0);
        if queued_qty == 0 {
            self.planet_overlay.build_unit_status =
                Some("No queued units of this type.".to_string());
            return Ok(());
        }

        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based()
        else {
            self.close_planet_build_overlay();
            return Ok(());
        };
        self.game_data
            .clear_planet_build_orders_by_kind(planet_record_index_1_based, kind)?;
        self.stage_hosted_planet_clear_build_kind(
            planet_record_index_1_based,
            production_item_kind_code(kind),
        );
        self.save_and_refresh_runtime()?;
        self.reselect_planet_overlay_row(planet_record_index_1_based);
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_quantity_status = None;
        Ok(())
    }

    pub(crate) fn close_planet_build_overlay(&mut self) {
        self.planet_overlay.clear_prompt();
        self.clear_planet_build_target();
        if self.planet_build_returns_to_owned_popup() {
            self.overlay = ActiveOverlay::None;
        }
        self.help_context = HelpContext::PlanetList;
    }

    pub(crate) fn clear_planet_overlay_footer_notice(&mut self) {
        self.planet_overlay.footer_notice = None;
    }

    #[allow(dead_code)]
    pub(crate) fn show_planet_overlay_footer_notice(&mut self, message: impl Into<String>) {
        self.planet_overlay.footer_notice = Some(message.into());
    }

    pub(crate) fn planet_build_title(&self) -> String {
        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based()
        else {
            return "Unknown".to_string();
        };
        if let Some(row) = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_record_index_1_based)
        {
            return format!(
                "{} {}",
                row.planet_name,
                format_sector_coords_table(row.coords)
            );
        }
        let Some(record) = self
            .game_data
            .planets
            .get(planet_record_index_1_based.saturating_sub(1))
        else {
            return "Unknown".to_string();
        };
        format!(
            "{} {}",
            record.name.trim().to_string(),
            format_sector_coords_table(record.sector.to_array())
        )
    }

    fn planet_build_planet_record_index_1_based(&self) -> Option<usize> {
        self.planet_overlay.build_planet_record_index_1_based
    }

    fn focus_planet_build_target_from_selection(&mut self) -> Option<usize> {
        let rows = planet_list::table_rows(self);
        let selected = self
            .planet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        let row = rows.get(selected)?;
        self.focus_planet_build_target(row.planet_record_index_1_based);
        Some(row.planet_record_index_1_based)
    }

    fn focus_planet_build_target(&mut self, planet_record_index_1_based: usize) -> bool {
        if self
            .game_data
            .planets
            .get(planet_record_index_1_based.saturating_sub(1))
            .is_none()
        {
            return false;
        }
        self.planet_overlay.clear_prompt();
        self.planet_overlay.build_planet_record_index_1_based = Some(planet_record_index_1_based);
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.reselect_planet_overlay_row(planet_record_index_1_based);
        true
    }

    fn planet_build_unavailable_message(&self, kind: ProductionItemKind) -> String {
        let Some(view) = self.planet_build_view() else {
            return "No build budget available.".to_string();
        };
        planet_build_unavailable_message(view.points_left, kind).to_string()
    }

    fn clear_planet_build_target(&mut self) {
        self.planet_overlay.build_planet_record_index_1_based = None;
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
    }

    fn planet_build_returns_to_owned_popup(&self) -> bool {
        matches!(self.popup, ActivePopup::OwnedPlanet { .. })
    }

    fn reselect_planet_overlay_row(&mut self, planet_record_index_1_based: usize) {
        self.enforce_valid_planet_filter();
        if let Some(index) = planet_list::table_rows(self)
            .iter()
            .position(|row| row.planet_record_index_1_based == planet_record_index_1_based)
        {
            self.planet_overlay.selected = index;
        }
    }

    fn initialize_planet_build_selection(&mut self) {
        self.planet_overlay.build_selected_kind = self
            .planet_build_specify_entries()
            .into_iter()
            .find(|entry| entry.queued_qty > 0)
            .map(|entry| entry.kind)
            .or_else(|| {
                BUILD_UNITS
                    .iter()
                    .find(|unit| self.planet_build_max_quantity_for(unit.kind).unwrap_or(0) > 0)
                    .map(|unit| unit.kind)
            })
            .or_else(|| BUILD_UNITS.first().map(|unit| unit.kind));
    }

    fn selected_planet_build_kind(&mut self) -> Option<ProductionItemKind> {
        if self.planet_overlay.build_selected_kind.is_none() {
            self.initialize_planet_build_selection();
        }
        self.planet_overlay.build_selected_kind
    }

    fn adjust_planet_build_selection(&mut self, column_delta: isize, row_delta: isize) {
        let Some(kind) = self.selected_planet_build_kind() else {
            return;
        };
        let Some(index) = BUILD_UNITS.iter().position(|unit| unit.kind == kind) else {
            self.initialize_planet_build_selection();
            return;
        };
        let next_index = if column_delta < 0 {
            build_table_left_index(index)
        } else if column_delta > 0 {
            build_table_right_index(index)
        } else if row_delta < 0 {
            build_table_up_index(index)
        } else if row_delta > 0 {
            build_table_down_index(index)
        } else {
            index
        };
        self.planet_overlay.build_selected_kind = BUILD_UNITS.get(next_index).map(|unit| unit.kind);
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
    }

    fn sync_planet_build_selection_from_input(&mut self) {
        let raw_input = self.planet_overlay.build_unit_input.trim();
        if raw_input.is_empty() || raw_input == "0" {
            return;
        }
        let Ok(number) = raw_input.parse::<u8>() else {
            return;
        };
        let Some(entry) = self
            .planet_build_specify_entries()
            .into_iter()
            .find(|entry| entry.number == number)
        else {
            return;
        };
        self.planet_overlay.build_selected_kind = Some(entry.kind);
    }

    fn queue_planet_build_units(
        &mut self,
        kind: ProductionItemKind,
        qty: u32,
        max_qty: Option<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            return Ok(());
        };
        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based()
        else {
            self.close_planet_build_overlay();
            return Ok(());
        };
        if let Some(limit) = max_qty {
            if qty > limit {
                self.planet_overlay.build_quantity_status =
                    Some(format!("Enter a quantity from 0 to {}.", limit));
                return Ok(());
            }
        }

        let points = qty.saturating_mul(unit.cost);
        self.game_data.append_planet_build_order(
            planet_record_index_1_based,
            points,
            production_item_kind_code(kind),
        )?;

        self.stage_hosted_planet_build(
            planet_record_index_1_based,
            points,
            production_item_kind_code(kind),
        );
        self.save_and_refresh_runtime()?;
        self.reselect_planet_overlay_row(planet_record_index_1_based);
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_quantity_status = None;
        Ok(())
    }
}

fn planet_build_orders_for_dash(app: &DashApp) -> Vec<nc_engine::PlanetBuildOrderLine> {
    let Some(planet_record_index_1_based) = app.planet_build_planet_record_index_1_based() else {
        return Vec::new();
    };
    let Some(record) = app
        .game_data
        .planets
        .get(planet_record_index_1_based.saturating_sub(1))
    else {
        return Vec::new();
    };
    planet_build_orders(record)
}

fn build_table_up_index(index: usize) -> usize {
    match index {
        0..=4 => index.saturating_sub(1),
        5..=8 => index.saturating_sub(1).max(5),
        _ => index,
    }
}

fn build_table_down_index(index: usize) -> usize {
    match index {
        0..=4 => (index + 1).min(4),
        5..=8 => (index + 1).min(8),
        _ => index,
    }
}

fn build_table_left_index(index: usize) -> usize {
    match index {
        5..=8 => index - 5,
        _ => index,
    }
}

fn build_table_right_index(index: usize) -> usize {
    match index {
        0..=3 => index + 5,
        _ => index,
    }
}
