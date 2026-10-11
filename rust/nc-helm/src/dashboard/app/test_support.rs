use crate::dashboard::input::MouseEvent;
use crate::dashboard::layout::dashboard::dashboard_layout;
use crate::dashboard::overlays::{fleet_list, intel_database, planet_list};
use crate::dashboard::panels::starmap;
use crate::dashboard::table_selection::sync_scroll_to_cursor;

use super::state::{DashApp, FleetOverlayFilter, IntelOverlayFilter, PlanetOverlayFilter};

impl DashApp {
    #[allow(dead_code)]
    pub(crate) fn dispatch_key_event_for_repro(&mut self, key: crate::dashboard::input::KeyEvent) {
        Self::dispatch_key_event(self, key);
    }

    pub(crate) fn dispatch_mouse_event_for_repro(&mut self, mouse: MouseEvent) -> bool {
        Self::dispatch_mouse_event(self, mouse)
    }

    pub(crate) fn screen_point_for_sector_for_repro(&self, target: [u8; 2]) -> Option<(u16, u16)> {
        let map_frame = dashboard_layout(self).widgets.center_map;
        for row in map_frame.grid.row..map_frame.grid.row + map_frame.grid.height {
            for col in map_frame.grid.col..map_frame.grid.col + map_frame.grid.width {
                if starmap::screen_sector_at_point(self, map_frame, col, row) == Some(target) {
                    return Some((col as u16, row as u16));
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub(crate) fn first_owned_planet_coords_for_repro(&self) -> Option<[u8; 2]> {
        self.game_data
            .planets
            .iter()
            .find(|planet| planet.owner_empire_slot_raw() == 1 && planet.coords_raw() != [0, 0])
            .map(|planet| planet.coords_raw())
    }

    pub(crate) fn apply_planet_overlay_filter(&mut self, filter: PlanetOverlayFilter) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.planet_overlay.filter = filter;
        self.planet_overlay.filter_clause = None;
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        if rows.is_empty() {
            self.planet_overlay.filter = PlanetOverlayFilter::All;
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

    pub(crate) fn apply_fleet_overlay_filter(&mut self, filter: FleetOverlayFilter) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        self.fleet_overlay.filter = filter;
        self.fleet_overlay.filter_clause = None;
        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.clear_prompt();
        let rows = fleet_list::table_rows(self);
        if rows.is_empty() {
            self.fleet_overlay.filter = FleetOverlayFilter::All;
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

    pub(crate) fn apply_intel_overlay_filter(&mut self, filter: IntelOverlayFilter) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.intel_overlay.filter = filter;
        self.intel_overlay.filter_clause = None;
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        if rows.is_empty() {
            self.intel_overlay.filter = IntelOverlayFilter::All;
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
