use tracing::debug;

use crate::dashboard::app::input::Action;
use crate::dashboard::input::{MouseButton, MouseEvent, MouseEventKind};
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::{Rect, modal_close_button_contains};
use crate::dashboard::overlays::{diplomacy, fleet_list, inbox, intel_database, planet_list};
use crate::dashboard::panels::starmap;
use crate::dashboard::planet_view;
use crate::dashboard::table_selection::sync_scroll_to_cursor;
use std::time::Instant;

use super::overlays::{scroll_clamp, scroll_selected};
use super::quit_confirm_message;
use super::state;
use super::state::{
    ActiveMouseGesture, ActiveOverlay, ActivePopup, DashApp, FleetOrderScope, FleetOverlayFilter,
    FleetOverlayPromptMode, FleetOverlayRowKey, IntelOverlayPromptMode, PanelFocus,
    PlanetOverlayPromptMode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct MouseRenderState {
    overlay: ActiveOverlay,
    popup: ActivePopup,
    overlay_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    popup_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    mouse_gesture: ActiveMouseGesture,
    focus: PanelFocus,
    crosshair_x: u8,
    crosshair_y: u8,
    starmap_viewport_x_min: u8,
    starmap_viewport_y_min: u8,
    diplomacy_scroll: usize,
    diplomacy_overlay_selected: usize,
    diplomacy_overlay_scroll: usize,
    inbox_selected: usize,
    inbox_scroll: usize,
    inbox_preview_scroll: usize,
    planet_selected: usize,
    planet_scroll: usize,
    fleet_selected: usize,
    fleet_scroll: usize,
    intel_selected: usize,
    intel_scroll: usize,
}

impl DashApp {
    fn mouse_render_state(&self) -> MouseRenderState {
        MouseRenderState {
            overlay: self.overlay,
            popup: self.popup,
            overlay_position: self.overlay_position,
            popup_position: self.popup_position,
            mouse_gesture: self.mouse_gesture,
            focus: self.focus,
            crosshair_x: self.crosshair_x,
            crosshair_y: self.crosshair_y,
            starmap_viewport_x_min: self.starmap_viewport_x_min,
            starmap_viewport_y_min: self.starmap_viewport_y_min,
            diplomacy_scroll: self.diplomacy_scroll,
            diplomacy_overlay_selected: self.diplomacy_overlay.selected,
            diplomacy_overlay_scroll: self.diplomacy_overlay.scroll,
            inbox_selected: self.inbox_overlay.selected,
            inbox_scroll: self.inbox_overlay.scroll,
            inbox_preview_scroll: self.inbox_overlay.preview_scroll,
            planet_selected: self.planet_overlay.selected,
            planet_scroll: self.planet_overlay.scroll,
            fleet_selected: self.fleet_overlay.selected,
            fleet_scroll: self.fleet_overlay.scroll,
            intel_selected: self.intel_overlay.selected,
            intel_scroll: self.intel_overlay.scroll,
        }
    }

    pub(crate) fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
        if !self.is_terminal_too_small {
            let before = self.mouse_render_state();
            self.handle_mouse(mouse);
            let changed = before != self.mouse_render_state();
            let toast_changed = self.update_command_line_toast_state(Instant::now());
            changed || toast_changed
        } else {
            false
        }
    }

    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) {
        let widgets = dashboard::dashboard_layout(self).widgets;
        let map_frame = widgets.center_map;
        let modal_parent =
            crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(widgets);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.popup != ActivePopup::None {
                    self.handle_popup_mouse_down(mouse, map_frame);
                    return;
                }
                if self.overlay != ActiveOverlay::None {
                    self.handle_overlay_mouse_down(mouse, map_frame);
                    return;
                }
                self.mouse_gesture = ActiveMouseGesture::None;
                self.handle_map_left_click(mouse, map_frame);
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                self.mouse_gesture = ActiveMouseGesture::None;
                self.handle_map_right_click(mouse, map_frame);
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                if let Some((cell_width, cell_height)) = self.last_starmap_cell_dims {
                    if map_frame
                        .outer
                        .contains_point(mouse.column as usize, mouse.row as usize)
                    {
                        self.mouse_gesture = ActiveMouseGesture::DraggingStarmap {
                            anchor_col: mouse.column,
                            anchor_row: mouse.row,
                            start_x_min: self.starmap_viewport_x_min,
                            start_y_min: self.starmap_viewport_y_min,
                            cell_width,
                            cell_height,
                            drag_occurred: false,
                        };
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.handle_mouse_move(mouse, modal_parent);
                } else {
                    self.handle_map_hover(mouse, map_frame);
                }
            }
            MouseEventKind::Moved => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                self.handle_map_hover(mouse, map_frame);
            }
            MouseEventKind::Drag(MouseButton::Middle) => {
                if let ActiveMouseGesture::DraggingStarmap {
                    anchor_col,
                    anchor_row,
                    start_x_min: _,
                    start_y_min: _,
                    cell_width,
                    cell_height,
                    ref mut drag_occurred,
                } = self.mouse_gesture
                {
                    *drag_occurred = true;
                    let dx_cells =
                        (anchor_col as i32 - mouse.column as i32) / cell_width.max(1) as i32;
                    let dy_cells =
                        (anchor_row as i32 - mouse.row as i32) / cell_height.max(1) as i32;
                    self.pan_starmap_viewport(dx_cells, dy_cells);
                }
            }
            MouseEventKind::Up(MouseButton::Middle) => {
                if let ActiveMouseGesture::DraggingStarmap { drag_occurred, .. } =
                    self.mouse_gesture
                {
                    if !drag_occurred {
                        // Click-without-drag: re-centre viewport on crosshair.
                        self.starmap_viewport_x_min = 0;
                        self.starmap_viewport_y_min = 0;
                        starmap::advance_starmap_viewport(self);
                    }
                }
                self.mouse_gesture = ActiveMouseGesture::None;
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_gesture = ActiveMouseGesture::None;
            }
            MouseEventKind::Scroll { lines } => {
                debug!(
                    "DashApp::handle_mouse Scroll lines={} popup={:?} overlay={:?} pos=({},{})",
                    lines, self.popup, self.overlay, mouse.column, mouse.row
                );
                if self.popup != ActivePopup::None {
                    debug!("Scroll ignored: popup open");
                    return;
                }
                if self.overlay != ActiveOverlay::None {
                    self.handle_overlay_scroll(mouse, lines, map_frame);
                    return;
                }
                if map_frame
                    .outer
                    .contains_point(mouse.column as usize, mouse.row as usize)
                {
                    self.pan_starmap_viewport(0, -lines);
                } else {
                    debug!("Scroll ignored: outside map_frame");
                }
            }
            _ => {}
        }
    }

    fn handle_overlay_scroll(
        &mut self,
        mouse: MouseEvent,
        lines: i32,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        debug!(
            "handle_overlay_scroll overlay={:?} lines={} pos=({},{})",
            self.overlay, lines, mouse.column, mouse.row
        );
        match self.overlay {
            ActiveOverlay::Diplomacy => {
                let total = diplomacy::table_rows(self).len();
                self.diplomacy_overlay.selected =
                    scroll_selected(self.diplomacy_overlay.selected, lines, total);
                if lines > 0 {
                    self.diplomacy_overlay.scroll = self
                        .diplomacy_overlay
                        .scroll
                        .min(self.diplomacy_overlay.selected);
                }
            }
            ActiveOverlay::Inbox => {
                match inbox::hit_test_inbox_pane(
                    self,
                    map_frame,
                    mouse.column as usize,
                    mouse.row as usize,
                ) {
                    Some(inbox::list::InboxPane::List) => {
                        let total = inbox::inbox_items(self).len();
                        self.inbox_overlay.selected =
                            scroll_selected(self.inbox_overlay.selected, lines, total);
                        if lines > 0 {
                            self.inbox_overlay.scroll =
                                self.inbox_overlay.scroll.min(self.inbox_overlay.selected);
                        }
                    }
                    Some(inbox::list::InboxPane::Preview) => {
                        let selected = self.inbox_overlay.selected;
                        let items = inbox::inbox_items(self);
                        let max_preview = items
                            .get(selected)
                            .map(|item| item.body_lines.len().saturating_sub(1))
                            .unwrap_or(0);
                        self.inbox_overlay.preview_scroll = scroll_clamp(
                            self.inbox_overlay.preview_scroll as i32 - lines,
                            max_preview as i32,
                        );
                    }
                    None => {
                        debug!("handle_overlay_scroll Inbox: no pane hit");
                    }
                }
            }
            ActiveOverlay::PlanetList => {
                if self.planet_overlay.prompt_mode != PlanetOverlayPromptMode::None {
                    debug!("handle_overlay_scroll PlanetList: prompt_mode open");
                    return;
                }
                let total = planet_list::selection_rows(self).len();
                self.planet_overlay.selected =
                    scroll_selected(self.planet_overlay.selected, lines, total);
                if lines > 0 {
                    self.planet_overlay.scroll =
                        self.planet_overlay.scroll.min(self.planet_overlay.selected);
                }
            }
            ActiveOverlay::FleetList => {
                if self.fleet_overlay.prompt_mode != FleetOverlayPromptMode::None {
                    debug!("handle_overlay_scroll FleetList: prompt_mode open");
                    return;
                }
                let total = fleet_list::selection_rows(self).len();
                self.fleet_overlay.selected =
                    scroll_selected(self.fleet_overlay.selected, lines, total);
                if lines > 0 {
                    self.fleet_overlay.scroll =
                        self.fleet_overlay.scroll.min(self.fleet_overlay.selected);
                }
            }
            ActiveOverlay::IntelDatabase => {
                if self.intel_overlay.prompt_mode != IntelOverlayPromptMode::None {
                    debug!("handle_overlay_scroll IntelDatabase: prompt_mode open");
                    return;
                }
                let total = intel_database::selection_rows(self).len();
                let old_selected = self.intel_overlay.selected;
                self.intel_overlay.selected =
                    scroll_selected(self.intel_overlay.selected, lines, total);
                debug!(
                    "IntelDatabase wheel: old_selected={} new_selected={} total={} lines={}",
                    old_selected, self.intel_overlay.selected, total, lines
                );
                if lines > 0 {
                    self.intel_overlay.scroll =
                        self.intel_overlay.scroll.min(self.intel_overlay.selected);
                }
            }
            ActiveOverlay::Help => {
                debug!("handle_overlay_scroll: Help no-op");
            }
            ActiveOverlay::None => {
                debug!("handle_overlay_scroll: overlay=None");
            }
        }
    }

    fn handle_overlay_mouse_down(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(popup) = self.current_overlay_popup_rect(map_frame) else {
            self.mouse_gesture = ActiveMouseGesture::None;
            return;
        };
        let mouse_col = mouse.column as usize;
        let mouse_row = mouse.row as usize;
        if modal_close_button_contains(popup, mouse_col, mouse_row) {
            self.close_active_overlay();
            return;
        }
        if self.overlay.is_draggable() && modal_chrome_contains(popup, mouse_col, mouse_row) {
            self.mouse_gesture = ActiveMouseGesture::DraggingOverlay {
                grab_col_offset: mouse_col.saturating_sub(popup.x as usize),
                grab_row_offset: mouse_row.saturating_sub(popup.y as usize),
            };
        } else {
            self.mouse_gesture = ActiveMouseGesture::None;
        }
    }

    fn handle_popup_mouse_down(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(popup) = self.current_popup_rect(map_frame) else {
            self.mouse_gesture = ActiveMouseGesture::None;
            return;
        };
        let mouse_col = mouse.column as usize;
        let mouse_row = mouse.row as usize;
        if modal_close_button_contains(popup, mouse_col, mouse_row) {
            self.apply_action(Action::ClosePopup);
            return;
        }
        if modal_chrome_contains(popup, mouse_col, mouse_row) {
            self.mouse_gesture = ActiveMouseGesture::DraggingPopup {
                grab_col_offset: mouse_col.saturating_sub(popup.x as usize),
                grab_row_offset: mouse_row.saturating_sub(popup.y as usize),
            };
        } else {
            self.mouse_gesture = ActiveMouseGesture::None;
        }
    }

    fn handle_map_left_click(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(coords) = starmap::screen_sector_at_point(
            self,
            map_frame,
            mouse.column as usize,
            mouse.row as usize,
        ) else {
            return;
        };
        self.set_crosshair_coords(coords);
        if !self.player_has_fleets_at(coords) {
            return;
        }
        self.open_fleet_overlay_for_location(coords);
    }

    fn handle_map_right_click(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(coords) = starmap::screen_sector_at_point(
            self,
            map_frame,
            mouse.column as usize,
            mouse.row as usize,
        ) else {
            return;
        };
        self.set_crosshair_coords(coords);
        let Some(detail) = planet_view::selected_planet_detail(self) else {
            return;
        };
        let owner = self
            .game_data
            .planets
            .get(detail.planet_record_index_1_based.saturating_sub(1))
            .map(|planet| planet.owner.0)
            .unwrap_or(0);
        if owner == self.player_record_index_1_based as u8 {
            self.open_owned_planet_popup(detail.planet_record_index_1_based);
        } else {
            self.open_planet_detail_popup_at_cursor();
        }
    }

    fn handle_map_hover(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        self.mouse_gesture = ActiveMouseGesture::None;
        let mouse_col = mouse.column as usize;
        let mouse_row = mouse.row as usize;
        if let Some(coords) = starmap::screen_sector_at_point(self, map_frame, mouse_col, mouse_row)
        {
            self.set_crosshair_coords(coords);
        } else if !map_frame.outer.contains_point(mouse_col, mouse_row) {
            self.reset_crosshair_to_homeworld();
        }
    }

    pub(super) fn set_crosshair_coords(&mut self, [x, y]: [u8; 2]) {
        if self.crosshair_x == x
            && self.crosshair_y == y
            && self.focus == state::PanelFocus::Map
            && self.map_coord_input.is_empty()
        {
            return;
        }
        self.crosshair_x = x;
        self.crosshair_y = y;
        self.focus = state::PanelFocus::Map;
        self.map_coord_input.clear();
        starmap::advance_starmap_viewport(self);
    }

    fn reset_crosshair_to_homeworld(&mut self) {
        let coords =
            state::initial_crosshair_coords(&self.game_data, self.player_record_index_1_based);
        if self.crosshair_x == coords[0]
            && self.crosshair_y == coords[1]
            && self.focus == state::PanelFocus::Map
            && self.map_coord_input.is_empty()
        {
            return;
        }
        self.set_crosshair_coords(coords);
    }

    fn pan_starmap_viewport(&mut self, dx_cells: i32, dy_cells: i32) {
        let map_size = nc_data::map_size_for_player_count(self.game_data.conquest.player_count);
        let frame = crate::dashboard::layout::dashboard_layout(self)
            .widgets
            .center_map;
        let lattice_width = frame
            .grid
            .width
            .saturating_sub(frame.row_label_cols)
            .saturating_sub(1);
        let visible_x =
            starmap::max_visible_sector_count(lattice_width, map_size, frame.cell_width.max(1));
        let visible_y = starmap::max_visible_sector_rows(frame.grid.height, map_size);
        let max_start_x = map_size.saturating_sub(visible_x).saturating_add(1).max(1);
        let max_start_y = map_size.saturating_sub(visible_y).saturating_add(1).max(1);
        self.starmap_viewport_x_min =
            (self.starmap_viewport_x_min as i32 + dx_cells).clamp(1, max_start_x as i32) as u8;
        self.starmap_viewport_y_min =
            (self.starmap_viewport_y_min as i32 + dy_cells).clamp(1, max_start_y as i32) as u8;
    }

    fn player_has_fleets_at(&self, coords: [u8; 2]) -> bool {
        let owner_slot = self.player_record_index_1_based as u8;
        self.game_data.fleets.iter().any(|fleet| {
            fleet.owner.0 == owner_slot
                && nc_data::has_any_force(&fleet)
                && fleet.sector.to_array() == coords
        })
    }

    pub(super) fn open_fleet_overlay_for_location(&mut self, coords: [u8; 2]) {
        self.fleet_overlay.location_filter = Some(coords);
        self.fleet_overlay.filter = FleetOverlayFilter::All;
        self.fleet_overlay.selected = 0;
        self.fleet_overlay.scroll = 0;
        self.fleet_overlay.jump_input.clear();
        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.clear_prompt();
        self.fleet_overlay.order_scope = FleetOrderScope::None;
        self.fleet_overlay.active_row_key = None;
        self.overlay_position = None;
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.overlay = ActiveOverlay::FleetList;
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = rows
            .iter()
            .position(|row| matches!(row.key, FleetOverlayRowKey::Fleet(_)))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    fn handle_mouse_move(&mut self, mouse: MouseEvent, parent: Rect) {
        match self.mouse_gesture {
            ActiveMouseGesture::DraggingOverlay {
                grab_col_offset,
                grab_row_offset,
            } => {
                if self.overlay == ActiveOverlay::None || !self.overlay.is_draggable() {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                let target_x = mouse.column.saturating_sub(grab_col_offset as u16);
                let target_y = mouse.row.saturating_sub(grab_row_offset as u16);
                self.overlay_position =
                    Some(crate::dashboard::overlays::frame::RelativePopupOrigin {
                        col_offset: target_x.saturating_sub(parent.x) as usize,
                        row_offset: target_y.saturating_sub(parent.y) as usize,
                    });
            }
            ActiveMouseGesture::DraggingPopup {
                grab_col_offset,
                grab_row_offset,
            } => {
                if self.popup == ActivePopup::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                let target_x = mouse.column.saturating_sub(grab_col_offset as u16);
                let target_y = mouse.row.saturating_sub(grab_row_offset as u16);
                self.popup_position =
                    Some(crate::dashboard::overlays::frame::RelativePopupOrigin {
                        col_offset: target_x.saturating_sub(parent.x) as usize,
                        row_offset: target_y.saturating_sub(parent.y) as usize,
                    });
            }
            ActiveMouseGesture::DraggingStarmap {
                anchor_col,
                anchor_row,
                start_x_min: _,
                start_y_min: _,
                cell_width,
                cell_height,
                ref mut drag_occurred,
            } => {
                *drag_occurred = true;
                let dx_cells = (anchor_col as i32 - mouse.column as i32) / cell_width.max(1) as i32;
                let dy_cells = (anchor_row as i32 - mouse.row as i32) / cell_height.max(1) as i32;
                self.pan_starmap_viewport(dx_cells, dy_cells);
            }
            ActiveMouseGesture::None => {}
        }
    }

    /// Returns true if any active overlay or popup covers the given cell rect
    /// (inclusive, screen cell coordinates). Keeps ratatui confined to this module.
    pub(crate) fn popup_covers_cell_rect(
        &self,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
        left: usize,
        right: usize,
        top: usize,
        bottom: usize,
    ) -> bool {
        let covers = |r: Rect| {
            let r_left = r.x as usize;
            let r_top = r.y as usize;
            let r_right = r_left + r.width.saturating_sub(1) as usize;
            let r_bottom = r_top + r.height.saturating_sub(1) as usize;
            left <= r_right && right >= r_left && top <= r_bottom && bottom >= r_top
        };
        self.current_overlay_popup_rect(map_frame)
            .is_some_and(covers)
            || self.current_popup_rect(map_frame).is_some_and(covers)
    }

    pub(super) fn current_overlay_popup_rect(
        &self,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) -> Option<Rect> {
        match self.overlay {
            ActiveOverlay::None => None,
            ActiveOverlay::PlanetList => planet_list::popup_rect(self, map_frame),
            ActiveOverlay::FleetList => fleet_list::popup_rect(self, map_frame),
            ActiveOverlay::IntelDatabase => Some(intel_database::popup_rect(self, map_frame)),
            ActiveOverlay::Inbox => Some(inbox::popup_rect(self, map_frame)),
            ActiveOverlay::Diplomacy => Some(crate::dashboard::overlays::diplomacy::popup_rect(
                self, map_frame,
            )),
            ActiveOverlay::Help => Some(crate::dashboard::overlays::help::popup_rect(
                self, map_frame,
            )),
        }
    }

    pub(super) fn current_popup_rect(
        &self,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) -> Option<Rect> {
        match self.popup {
            ActivePopup::None => None,
            ActivePopup::QuitConfirm => Some(
                crate::dashboard::overlays::frame::overlay_popup_rect_in_map(
                    map_frame,
                    crate::dashboard::QUIT_CONFIRM_TITLE,
                    crate::dashboard::quit_confirm_popup_width(quit_confirm_message()),
                    crate::dashboard::QUIT_CONFIRM_HEIGHT,
                    self.popup_position,
                ),
            ),
            ActivePopup::InboxMessageConfirm { action } => Some(
                crate::dashboard::overlays::frame::overlay_popup_rect_in_map(
                    map_frame,
                    super::inbox_message_confirm_title(),
                    super::confirm_popup_width(
                        super::inbox_message_confirm_title(),
                        super::inbox_message_confirm_message(action),
                    ),
                    crate::dashboard::QUIT_CONFIRM_HEIGHT,
                    self.popup_position,
                ),
            ),
            ActivePopup::TaxPrompt => Some(crate::dashboard::popups::tax_prompt::popup_rect(
                self, map_frame,
            )),
            ActivePopup::PlanetDetail {
                planet_record_index_1_based,
            } => Some(crate::dashboard::popups::planet_detail::popup_rect(
                self,
                map_frame,
                planet_record_index_1_based,
            )),
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based,
            } => Some(crate::dashboard::popups::owned_planet::popup_rect(
                self,
                map_frame,
                planet_record_index_1_based,
            )),
            ActivePopup::FleetDetail {
                fleet_record_index_1_based,
            } => Some(crate::dashboard::popups::fleet_detail::popup_rect(
                self,
                map_frame,
                fleet_record_index_1_based,
            )),
            ActivePopup::StartupReview => Some(
                crate::dashboard::popups::startup_review::popup_rect(self, map_frame),
            ),
        }
    }
}

fn modal_chrome_contains(popup: Rect, col: usize, row: usize) -> bool {
    let left = popup.x as usize;
    let top = popup.y as usize;
    let right = left + popup.width as usize - 1;
    let bottom = top + popup.height as usize - 1;
    (row == top && col >= left && col <= right)
        || (row == bottom && col >= left && col <= right)
        || (col == left && row >= top && row <= bottom)
        || (col == right && row >= top && row <= bottom)
}
