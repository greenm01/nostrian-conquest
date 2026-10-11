//! Center panel: ASCII sector lattice, centered markers, and axis labels.

use std::collections::{BTreeMap, BTreeSet};

use crate::dashboard::buffer::{CellStyle, OverlaySelection, PlayfieldBuffer};
#[cfg(test)]
use nc_data::CoreGameData;
use nc_data::{
    DiplomaticRelation, PlanetIntelSnapshot, PlayerStarmapProjection, PlayerStarmapWorld,
    build_player_starmap_projection_from_snapshots,
};

use crate::dashboard::app::panel_cache::CachedStarmapProjection;
use crate::dashboard::app::state::DashApp;
use crate::dashboard::layout::{self, MapWidgetFrame};
use crate::dashboard::theme;

const GRID_WALL: char = '|';
const GRID_DASH: char = '-';
const FLEET_MARKER_EMPTY: char = '△';
const FLEET_MARKER_WORLD: char = '⨁';
const FLEET_MARKER_OWNED_WORLD: char = '@';
const MIN_SECTOR_WIDTH: usize = 4;
const MIN_SECTOR_HEIGHT: usize = 2;
const STARMAP_SCROLL_MARGIN: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StarmapMarkerKind {
    Owned,
    Unowned,
    Icd,
    Enemy,
    Neutral,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlanetJumpDirection {
    Backward,
    Forward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedMapGeometry {
    axis_row: usize,
    bottom_axis_row: usize,
    row_label_col: usize,
    right_label_col: usize,
    row_label_width: usize,
    grid_top_row: usize,
    grid_bottom_row: usize,
    cell_area_col: usize,
    grid_right_col: usize,
    x_min: u8,
    x_max: u8,
    y_min: u8,
    y_max: u8,
    visible_x: u8,
    visible_y: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SectorRect {
    col: usize,
    row: usize,
    width: usize,
    height: usize,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: MapWidgetFrame) {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count);

    let player_empire = app.player_record_index_1_based as u8;
    // Cache hit: no projection clone, no BTreeMap/BTreeSet rebuild.
    let cache = cached_projection_for_app(app);
    let projected = projected_map_geometry(app, frame, map_size);
    apply_selection_overlay_from_projected(buf, app, &projected, frame);

    for world_x in projected.x_min..=projected.x_max {
        let label_col = projected.sector_label_col(world_x);
        let highlighted = world_x == app.crosshair_x;
        draw_column_axis_label(buf, projected.axis_row, label_col, world_x, highlighted);
        draw_column_axis_label(
            buf,
            projected.bottom_axis_row,
            label_col,
            world_x,
            highlighted,
        );
    }

    for row_y in (projected.y_min..=projected.y_max).rev() {
        let Some(row_rect) = projected.sector_rect([projected.x_min, row_y]) else {
            continue;
        };
        draw_separator_row(
            buf,
            &projected,
            row_rect.separator_row(),
            row_y == projected.y_max,
        );
        draw_content_row(buf, &projected, row_rect.content_row());
        let row_label_style = if row_y == app.crosshair_y {
            theme::map_selection_style()
        } else {
            theme::dim_style()
        };
        draw_row_axis_label(
            buf,
            projected.row_label_col,
            projected.row_label_width,
            row_rect.content_row(),
            row_y,
            false,
            row_label_style,
        );
        draw_row_axis_label(
            buf,
            projected.right_label_col,
            projected.row_label_width,
            row_rect.content_row(),
            row_y,
            true,
            row_label_style,
        );
        for col_x in projected.x_min..=projected.x_max {
            let has_viewer_fleet = cache.viewer_fleet_sectors.contains(&[col_x, row_y]);
            let planet = cache
                .world_index
                .get(&[col_x, row_y])
                .map(|&i| &cache.projection.worlds[i]);
            let is_selected = col_x == app.crosshair_x && row_y == app.crosshair_y;
            if !is_selected && planet.is_none() && !has_viewer_fleet {
                continue;
            }

            let (symbol, marker_style) = if has_viewer_fleet {
                (
                    fleet_marker_for_sector(app, player_empire, planet),
                    marker_background_style(is_selected),
                )
            } else if let Some(snapshot) = planet {
                marker_for_world_on_background(
                    app,
                    player_empire,
                    snapshot,
                    selected_sector_background(is_selected),
                )
            } else {
                continue;
            };
            let marker_col = projected.sector_marker_col(col_x);
            let marker_row = projected.sector_marker_row(row_y);
            buf.set_cell(marker_row, marker_col, symbol, marker_style);
        }
    }

    draw_separator_row(buf, &projected, projected.grid_bottom_row, true);
}

/// Set the selection overlay on `buf` based on the current app crosshair
/// position and the projected map geometry. Called by `draw` and also by the
/// render loop after a panel-cache hit restores cell content without re-running
/// `draw`, so the overlay is always consistent.
pub(crate) fn apply_selection_overlay(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    frame: MapWidgetFrame,
) {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count);
    let projected = projected_map_geometry(app, frame, map_size);
    apply_selection_overlay_from_projected(buf, app, &projected, frame);
}

fn apply_selection_overlay_from_projected(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    projected: &ProjectedMapGeometry,
    frame: MapWidgetFrame,
) {
    let Some(overlay) = projected.selection_overlay(app) else {
        return;
    };

    if app.popup_covers_cell_rect(
        frame,
        overlay.left_col,
        overlay.right_col,
        overlay.top_row,
        overlay.bottom_row,
    ) {
        buf.clear_overlay_selection();
        return;
    }

    buf.set_overlay_selection(overlay);
}

pub(crate) fn projected_map_geometry(
    app: &DashApp,
    frame: MapWidgetFrame,
    map_size: u8,
) -> ProjectedMapGeometry {
    let lattice_width = frame
        .grid
        .width
        .saturating_sub(frame.row_label_cols)
        .saturating_sub(1);
    let projection = projected_display_bounds(app, frame, map_size, lattice_width);
    let rendered_block_width =
        rendered_map_block_width(frame.row_label_cols, projection.visible_x, frame.cell_width);
    let rendered_block_height = rendered_map_block_height(projection.visible_y);
    let row_label_col =
        frame.map_block.col + frame.map_block.width.saturating_sub(rendered_block_width) / 2;
    let axis_row =
        frame.map_block.row + frame.map_block.height.saturating_sub(rendered_block_height) / 2;
    let grid_top_row = axis_row + 1;
    let cell_area_col = row_label_col + frame.row_label_cols;
    let grid_right_col = cell_area_col + usize::from(projection.visible_x) * frame.cell_width;
    let grid_bottom_row = grid_top_row + rendered_lattice_height(projection.visible_y) - 1;

    ProjectedMapGeometry {
        axis_row,
        bottom_axis_row: grid_bottom_row + 1,
        row_label_col,
        right_label_col: grid_right_col + 1,
        row_label_width: frame.row_label_cols,
        grid_top_row,
        grid_bottom_row,
        cell_area_col,
        grid_right_col,
        x_min: projection.x_min,
        x_max: projection.x_max,
        y_min: projection.y_min,
        y_max: projection.y_max,
        visible_x: projection.visible_x,
        visible_y: projection.visible_y,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProjectionBounds {
    x_min: u8,
    x_max: u8,
    y_min: u8,
    y_max: u8,
    visible_x: u8,
    visible_y: u8,
}

/// Advance the starmap viewport so the crosshair stays within the dead-zone
/// margin of the viewport edges.  Called after every crosshair move.
pub fn advance_starmap_viewport(app: &mut DashApp) {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count);
    let frame = crate::dashboard::layout::dashboard_layout(app)
        .widgets
        .center_map;
    let lattice_width = frame
        .grid
        .width
        .saturating_sub(frame.row_label_cols)
        .saturating_sub(1);
    let visible_x = max_visible_sector_count(lattice_width, map_size, frame.cell_width.max(1));
    let visible_y = max_visible_sector_rows(frame.grid.height, map_size);
    app.starmap_viewport_x_min = viewport_start(
        app.starmap_viewport_x_min,
        app.crosshair_x,
        visible_x,
        map_size,
    );
    app.starmap_viewport_y_min = viewport_start(
        app.starmap_viewport_y_min,
        app.crosshair_y,
        visible_y,
        map_size,
    );
    app.last_starmap_cell_dims = Some((frame.cell_width as u16, MIN_SECTOR_HEIGHT as u16));
}

fn projected_display_bounds(
    app: &DashApp,
    frame: MapWidgetFrame,
    map_size: u8,
    lattice_width: usize,
) -> ProjectionBounds {
    let visible_x = max_visible_sector_count(lattice_width, map_size, frame.cell_width.max(1));
    let visible_y = max_visible_sector_rows(frame.grid.height, map_size);
    let x_min = viewport_start(
        app.starmap_viewport_x_min,
        app.crosshair_x,
        visible_x,
        map_size,
    );
    let y_min = viewport_start(
        app.starmap_viewport_y_min,
        app.crosshair_y,
        visible_y,
        map_size,
    );
    let x_max = x_min + visible_x.saturating_sub(1);
    let y_max = y_min + visible_y.saturating_sub(1);
    ProjectionBounds {
        x_min,
        x_max,
        y_min,
        y_max,
        visible_x,
        visible_y,
    }
}

pub(crate) fn max_visible_sector_count(
    extent: usize,
    map_size: u8,
    min_extent_per_sector: usize,
) -> u8 {
    extent
        .saturating_div(min_extent_per_sector.max(1))
        .max(1)
        .min(usize::from(map_size)) as u8
}

pub(crate) fn max_visible_sector_rows(grid_height: usize, map_size: u8) -> u8 {
    grid_height
        .saturating_sub(1)
        .saturating_div(MIN_SECTOR_HEIGHT)
        .max(1)
        .min(usize::from(map_size)) as u8
}

fn rendered_lattice_height(visible_y: u8) -> usize {
    usize::from(visible_y) * MIN_SECTOR_HEIGHT + 1
}

fn rendered_map_block_height(visible_y: u8) -> usize {
    rendered_lattice_height(visible_y) + 2
}

fn rendered_map_block_width(row_label_width: usize, visible_x: u8, cell_width: usize) -> usize {
    row_label_width * 2 + 1 + usize::from(visible_x) * cell_width
}

fn viewport_start(prev_start: u8, center: u8, visible: u8, map_size: u8) -> u8 {
    if visible >= map_size {
        return 1;
    }
    let max_start = map_size.saturating_sub(visible).saturating_add(1).max(1);

    // If the viewport nearly fills the map, disable the margin and fall back
    // to clamp-only behaviour so the user is not locked to the centre.
    let effective_margin =
        if u16::from(visible) + 2 * u16::from(STARMAP_SCROLL_MARGIN) >= u16::from(map_size) {
            0
        } else {
            STARMAP_SCROLL_MARGIN
        };

    // Sentinel 0 means "not yet initialised — centre on crosshair".
    if prev_start == 0 {
        let half = visible / 2;
        return center.saturating_sub(half).clamp(1, max_start);
    }

    let prev_start = prev_start.clamp(1, max_start);
    let prev_end = prev_start + visible - 1;
    let left_trigger = prev_start + effective_margin;
    let right_trigger = prev_end.saturating_sub(effective_margin);

    let new_start = if center < left_trigger {
        center.saturating_sub(effective_margin).max(1)
    } else if center > right_trigger {
        (center + effective_margin + 1)
            .saturating_sub(visible)
            .max(1)
    } else {
        prev_start
    };
    new_start.clamp(1, max_start)
}

impl ProjectedMapGeometry {
    fn selection_overlay(&self, app: &DashApp) -> Option<OverlaySelection> {
        let rect = self.sector_rect([app.crosshair_x, app.crosshair_y])?;
        // Anchor the selection to the sector's true lattice bounds rather than
        // the centered marker cell. The right and bottom bounds land on the
        // next separator wall/row, or the closing border for the last visible
        // sector on that axis.
        Some(OverlaySelection {
            fg: theme::map_selection_style().fg,
            left_col: rect.col,
            right_col: rect.right_boundary_col().min(self.grid_right_col),
            top_row: rect.row,
            bottom_row: rect.bottom_boundary_row().min(self.grid_bottom_row),
        })
    }

    fn sector_rect(&self, coords: [u8; 2]) -> Option<SectorRect> {
        if coords[0] < self.x_min
            || coords[0] > self.x_max
            || coords[1] < self.y_min
            || coords[1] > self.y_max
        {
            return None;
        }
        let x_idx = usize::from(coords[0] - self.x_min);
        let y_idx = usize::from(self.y_max - coords[1]);
        Some(SectorRect {
            col: self.cell_area_col + x_idx * MIN_SECTOR_WIDTH,
            row: self.grid_top_row + y_idx * MIN_SECTOR_HEIGHT,
            width: MIN_SECTOR_WIDTH,
            height: MIN_SECTOR_HEIGHT,
        })
    }

    fn sector_label_col(&self, world_x: u8) -> usize {
        self.sector_rect([world_x, self.y_max])
            .map(SectorRect::label_col)
            .unwrap_or(self.cell_area_col)
    }

    fn sector_marker_col(&self, world_x: u8) -> usize {
        self.sector_rect([world_x, self.y_max])
            .map(SectorRect::marker_col)
            .unwrap_or(0)
    }

    fn sector_marker_row(&self, world_y: u8) -> usize {
        self.sector_rect([self.x_min, world_y])
            .map(SectorRect::marker_row)
            .unwrap_or(0)
    }
}

impl SectorRect {
    fn separator_row(self) -> usize {
        self.row
    }

    fn content_row(self) -> usize {
        self.row + 1
    }

    fn label_col(self) -> usize {
        self.col + 1
    }

    fn marker_row(self) -> usize {
        self.content_row()
    }

    fn marker_col(self) -> usize {
        self.col + 2
    }

    fn right_boundary_col(self) -> usize {
        self.col + self.width
    }

    fn bottom_boundary_row(self) -> usize {
        self.row + self.height
    }
}

pub(crate) fn screen_sector_at_point(
    app: &DashApp,
    frame: MapWidgetFrame,
    col: usize,
    row: usize,
) -> Option<[u8; 2]> {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count);
    let projected = projected_map_geometry(app, frame, map_size);
    if col < projected.cell_area_col
        || col > projected.grid_right_col
        || row < projected.grid_top_row
        || row > projected.grid_bottom_row
    {
        return None;
    }
    let x_index = if col == projected.grid_right_col {
        usize::from(projected.visible_x.saturating_sub(1))
    } else {
        (col - projected.cell_area_col) / MIN_SECTOR_WIDTH
    };
    let y_index = if row == projected.grid_bottom_row {
        usize::from(projected.visible_y.saturating_sub(1))
    } else {
        (row - projected.grid_top_row) / MIN_SECTOR_HEIGHT
    };
    Some([
        projected.x_min + x_index as u8,
        projected.y_max - y_index as u8,
    ])
}

fn draw_column_axis_label(
    buf: &mut PlayfieldBuffer,
    axis_row: usize,
    label_col: usize,
    world_x: u8,
    highlighted: bool,
) {
    let label = format!("{world_x:02}");
    let style = if highlighted {
        theme::map_selection_style()
    } else {
        theme::dim_style()
    };
    layout::write_clipped(buf, axis_row, label_col, 2, &label, style);
}

fn draw_row_axis_label(
    buf: &mut PlayfieldBuffer,
    col: usize,
    width: usize,
    row: usize,
    world_y: u8,
    right_aligned: bool,
    style: CellStyle,
) {
    let label = if right_aligned {
        format!(" {world_y:02}")
    } else {
        format!("{world_y:02} ")
    };
    layout::write_clipped(buf, row, col, width, &label, style);
}

fn draw_content_row(buf: &mut PlayfieldBuffer, projected: &ProjectedMapGeometry, row: usize) {
    let border_style = theme::dim_style();
    buf.set_cell(row, projected.cell_area_col, GRID_WALL, border_style);
    for col in projected.cell_area_col + 1..projected.grid_right_col {
        buf.set_cell(row, col, ' ', theme::body_style());
    }
    buf.set_cell(row, projected.grid_right_col, GRID_WALL, border_style);
}

fn draw_separator_row(
    buf: &mut PlayfieldBuffer,
    projected: &ProjectedMapGeometry,
    row: usize,
    solid: bool,
) {
    let border_style = theme::dim_style();
    buf.set_cell(
        row,
        projected.row_label_col + projected.row_label_width.saturating_sub(1),
        GRID_DASH,
        border_style,
    );
    for x_idx in 0..usize::from(projected.visible_x) {
        let start = projected.cell_area_col + x_idx * MIN_SECTOR_WIDTH;
        buf.set_cell(row, start, GRID_WALL, border_style);
        buf.set_cell(row, start + 1, GRID_DASH, border_style);
        buf.set_cell(
            row,
            start + 2,
            if solid { GRID_DASH } else { ' ' },
            border_style,
        );
        buf.set_cell(row, start + 3, GRID_DASH, border_style);
    }
    buf.set_cell(row, projected.grid_right_col, GRID_WALL, border_style);
    buf.set_cell(row, projected.right_label_col, GRID_DASH, border_style);
}

fn selected_sector_background(selected: bool) -> CellStyle {
    if selected {
        theme::map_center_style()
    } else {
        theme::body_style()
    }
}

fn marker_background_style(selected: bool) -> CellStyle {
    let background = selected_sector_background(selected);
    theme::map_fleet_marker_style_on(background.bg, background.bold)
}

pub(crate) fn jump_planet_target_for_app(
    app: &DashApp,
    current: [u8; 2],
    direction: PlanetJumpDirection,
) -> Option<[u8; 2]> {
    let cache = cached_projection_for_app(app);
    jump_planet_target_coords(
        cache.projection.map_width,
        &cache.projection.worlds,
        current,
        direction,
    )
}

#[cfg(test)]
fn projection_world_at(
    projection: &PlayerStarmapProjection,
    coords: [u8; 2],
) -> Option<&PlayerStarmapWorld> {
    projection
        .worlds
        .iter()
        .find(|world| world.coords == coords)
}

/// Build a `[x, y] → world index` lookup from a projection's world list.
/// Used by `draw` to turn the per-sector O(N) linear search into O(log N).
fn world_index_for_projection(projection: &PlayerStarmapProjection) -> BTreeMap<[u8; 2], usize> {
    projection
        .worlds
        .iter()
        .enumerate()
        .map(|(i, world)| (world.coords, i))
        .collect()
}

/// Build the set of sector coords that contain at least one viewer fleet
/// in a single O(F) pass over all fleet records, avoiding the previous
/// O(visible_sectors × fleets) nested scan.
fn viewer_fleet_sector_coords_fast(app: &DashApp, viewer_empire_id: u8) -> BTreeSet<[u8; 2]> {
    app.game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == viewer_empire_id && nc_data::has_any_force(&fleet))
        .map(|fleet| fleet.sector.to_array())
        .collect()
}

fn snapshot_map_for_app(app: &DashApp) -> BTreeMap<usize, PlanetIntelSnapshot> {
    app.planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>()
}

fn projection_for_snapshot_map(
    app: &DashApp,
    snapshot_map: &BTreeMap<usize, PlanetIntelSnapshot>,
) -> PlayerStarmapProjection {
    build_player_starmap_projection_from_snapshots(
        &app.game_data,
        snapshot_map,
        app.player_record_index_1_based as u8,
    )
}

/// Return a shared reference to the cached `CachedStarmapProjection` for the
/// current app state. The cache (projection, world index, and viewer fleet
/// sectors) is rebuilt only when `game_data_revision` or
/// `player_record_index_1_based` changes; on a cache hit nothing is cloned or
/// allocated.
pub(crate) fn cached_projection_for_app(
    app: &DashApp,
) -> std::cell::Ref<'_, CachedStarmapProjection> {
    let revision = app.game_data_revision;
    let player = app.player_record_index_1_based;

    // Check whether the cache is still valid (drop the borrow immediately).
    let needs_update = !app
        .starmap_projection_cache
        .borrow()
        .as_ref()
        .is_some_and(|c| c.revision == revision && c.player == player);

    if needs_update {
        let player_empire = player as u8;
        let snapshot_map = snapshot_map_for_app(app);
        let projection = projection_for_snapshot_map(app, &snapshot_map);
        let world_index = world_index_for_projection(&projection);
        let viewer_fleet_sectors = viewer_fleet_sector_coords_fast(app, player_empire);
        *app.starmap_projection_cache.borrow_mut() = Some(CachedStarmapProjection {
            revision,
            player,
            projection,
            world_index,
            viewer_fleet_sectors,
        });
    }

    std::cell::Ref::map(app.starmap_projection_cache.borrow(), |opt| {
        opt.as_ref()
            .expect("starmap projection cache should be populated")
    })
}

fn fleet_marker_for_sector(
    app: &DashApp,
    viewer_empire_id: u8,
    world: Option<&PlayerStarmapWorld>,
) -> char {
    match world {
        None => FLEET_MARKER_EMPTY,
        Some(world) => match marker_kind_for_world(app, viewer_empire_id, world) {
            StarmapMarkerKind::Owned => FLEET_MARKER_OWNED_WORLD,
            _ => FLEET_MARKER_WORLD,
        },
    }
}

fn jump_planet_target_coords(
    map_size: u8,
    worlds: &[PlayerStarmapWorld],
    current: [u8; 2],
    direction: PlanetJumpDirection,
) -> Option<[u8; 2]> {
    let mut coords = worlds.iter().map(|world| world.coords).collect::<Vec<_>>();
    if coords.is_empty() {
        return None;
    }
    coords.sort_by_key(|coords| screen_order_index(*coords, map_size));
    coords.dedup();

    let current_index = screen_order_index(current, map_size);
    match direction {
        PlanetJumpDirection::Forward => coords
            .iter()
            .copied()
            .find(|coords| screen_order_index(*coords, map_size) > current_index)
            .or_else(|| coords.first().copied()),
        PlanetJumpDirection::Backward => coords
            .iter()
            .rev()
            .copied()
            .find(|coords| screen_order_index(*coords, map_size) < current_index)
            .or_else(|| coords.last().copied()),
    }
}

fn screen_order_index(coords: [u8; 2], map_size: u8) -> usize {
    let y_rank = usize::from(map_size.saturating_sub(coords[1]));
    let x_rank = usize::from(coords[0].saturating_sub(1));
    y_rank * usize::from(map_size) + x_rank
}

pub(crate) fn marker_kind_for_world(
    app: &DashApp,
    viewer_empire_id: u8,
    world: &PlayerStarmapWorld,
) -> StarmapMarkerKind {
    match world.known_owner_empire_id {
        Some(owner) if owner == viewer_empire_id => StarmapMarkerKind::Owned,
        Some(0) => StarmapMarkerKind::Unowned,
        Some(owner) => {
            let is_icd = app
                .game_data
                .players
                .get(owner.saturating_sub(1) as usize)
                .map(|player| player.mode == nc_data::PlayerMode::CivilDisorder)
                .unwrap_or(false);
            if is_icd {
                StarmapMarkerKind::Icd
            } else {
                let viewer = app
                    .game_data
                    .players
                    .get(viewer_empire_id.saturating_sub(1) as usize);
                let is_enemy = viewer
                    .and_then(|viewer| nc_data::diplomatic_relation_toward(viewer, owner))
                    == Some(DiplomaticRelation::Enemy);
                if is_enemy {
                    StarmapMarkerKind::Enemy
                } else {
                    StarmapMarkerKind::Neutral
                }
            }
        }
        None if world.known_name.is_some()
            || world.known_potential_production.is_some()
            || world.known_armies.is_some()
            || world.known_ground_batteries.is_some() =>
        {
            StarmapMarkerKind::Partial
        }
        None => StarmapMarkerKind::Unknown,
    }
}

fn marker_for_world_on_background(
    app: &DashApp,
    viewer_empire_id: u8,
    world: &PlayerStarmapWorld,
    background: CellStyle,
) -> (char, CellStyle) {
    match marker_kind_for_world(app, viewer_empire_id, world) {
        StarmapMarkerKind::Owned => (
            'O',
            theme::empire_slot_style_on(
                world.known_owner_empire_id.unwrap_or(viewer_empire_id),
                background.bg,
                background.bold,
            ),
        ),
        StarmapMarkerKind::Unowned => (
            '#',
            CellStyle::new(theme::dim_style().fg, background.bg, background.bold),
        ),
        StarmapMarkerKind::Icd => (
            '◊',
            theme::empire_slot_style_on(
                world.known_owner_empire_id.unwrap_or(viewer_empire_id),
                background.bg,
                background.bold,
            ),
        ),
        StarmapMarkerKind::Enemy => (
            '#',
            theme::empire_slot_style_on(
                world.known_owner_empire_id.unwrap_or(viewer_empire_id),
                background.bg,
                background.bold,
            ),
        ),
        StarmapMarkerKind::Neutral => (
            '#',
            theme::empire_slot_style_on(
                world.known_owner_empire_id.unwrap_or(viewer_empire_id),
                background.bg,
                background.bold,
            ),
        ),
        StarmapMarkerKind::Partial => (
            '*',
            CellStyle::new(theme::value_style().fg, background.bg, background.bold),
        ),
        StarmapMarkerKind::Unknown => (
            '?',
            CellStyle::new(theme::value_style().fg, background.bg, background.bold),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::buffer::PlayfieldBuffer;
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::layout::dashboard_layout;
    use crate::dashboard::theme;
    use nc_data::{GameStateBuilder, IntelTier};
    use nc_engine::build_seeded_initialized_game;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn owner_markers_use_empire_slot_colors() {
        let owner = Some(4);
        let expected = crate::dashboard::theme::classic::empire_slot_color(4);

        let (_, owned_style) = marker_for_world_kind(owner, StarmapMarkerKind::Owned);
        let (_, enemy_style) = marker_for_world_kind(owner, StarmapMarkerKind::Enemy);
        let (_, neutral_style) = marker_for_world_kind(owner, StarmapMarkerKind::Neutral);
        let (_, icd_style) = marker_for_world_kind(owner, StarmapMarkerKind::Icd);

        assert_eq!(owned_style.fg, expected);
        assert_eq!(enemy_style.fg, expected);
        assert_eq!(neutral_style.fg, expected);
        assert_eq!(icd_style.fg, expected);
    }

    fn marker_for_world_kind(owner: Option<u8>, kind: StarmapMarkerKind) -> (char, CellStyle) {
        match kind {
            StarmapMarkerKind::Owned => ('O', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Unowned => ('#', theme::dim_style()),
            StarmapMarkerKind::Icd => ('◊', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Enemy => ('#', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Neutral => ('#', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Partial => ('*', theme::value_style()),
            StarmapMarkerKind::Unknown => ('?', theme::value_style()),
        }
    }

    #[test]
    fn unknown_markers_use_bright_neutral_value_color() {
        let (_, style) = marker_for_world_kind(None, StarmapMarkerKind::Unknown);

        assert_eq!(style, theme::value_style());
        assert_ne!(style, theme::dim_style());
    }

    #[test]
    fn world_status_handles_unknown_and_special_owners() {
        let mut game_data = GameStateBuilder::new()
            .with_player_count(4)
            .with_year(3006)
            .build_initialized_baseline()
            .expect("baseline game data");
        game_data.players[2].mode = nc_data::PlayerMode::from_raw(0x00);

        assert_eq!(owner_label(&game_data, Some(0)), "Unowned");
        assert_eq!(owner_label(&game_data, Some(3)), "ICD");
        assert_eq!(owner_label(&game_data, None), "?");
        assert_eq!(known_u16(None), "?");
        assert_eq!(known_u8(None), "?");
    }

    #[test]
    fn planet_jump_moves_in_wrapped_screen_order() {
        let worlds = vec![make_world([2, 5]), make_world([4, 4]), make_world([1, 1])];

        assert_eq!(
            jump_planet_target_coords(5, &worlds, [1, 5], PlanetJumpDirection::Forward),
            Some([2, 5])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [2, 5], PlanetJumpDirection::Forward),
            Some([4, 4])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [4, 4], PlanetJumpDirection::Backward),
            Some([2, 5])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [5, 1], PlanetJumpDirection::Forward),
            Some([2, 5])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [1, 5], PlanetJumpDirection::Backward),
            Some([1, 1])
        );
    }

    #[test]
    fn planet_jump_stays_on_single_world() {
        let worlds = vec![make_world([3, 3])];

        assert_eq!(
            jump_planet_target_coords(5, &worlds, [3, 3], PlanetJumpDirection::Forward),
            Some([3, 3])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [1, 1], PlanetJumpDirection::Backward),
            Some([3, 3])
        );
    }

    #[test]
    fn draw_bottom_map_row_matches_current_padding_mode() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 2;
        app.crosshair_y = 3;
        let widgets = dashboard_layout(&app).widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, widgets.center_map);

        assert!(
            !buffer
                .plain_line(widgets.center_map.bottom_pad_row)
                .is_empty()
        );
    }

    #[test]
    fn readable_mode_uses_full_widget_for_projected_map_block() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 2;
        app.crosshair_y = 3;
        let widgets = dashboard_layout(&app).widgets;
        let layout = dashboard_layout(&app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, widgets.center_map);

        let axis_line = buffer.plain_line(widgets.center_map.axis_row);
        let bottom_axis_line = buffer.plain_line(widgets.center_map.bottom_pad_row);

        assert!(layout.frame.width() < app.geometry.width());
        // After the map-size snap, `map_block` may be smaller than `outer`
        // (centred inside) but must still fit within it.
        assert!(widgets.center_map.map_block.last_col() <= widgets.center_map.outer.last_col());
        assert!(widgets.center_map.map_block.last_row() <= widgets.center_map.outer.last_row());
        // Axis row sits at or below map_block top (inside the top gutter).
        assert!(widgets.center_map.axis_row >= widgets.center_map.map_block.row);
        assert!(widgets.center_map.axis_row <= widgets.center_map.map_block.last_row());
        assert_eq!(
            widgets.center_map.grid.col,
            widgets.center_map.map_block.col
        );
        assert_eq!(
            widgets.center_map.bottom_pad_row,
            widgets.center_map.grid.last_row() + 1
        );
        assert!(axis_line.contains("01"));
        assert!(axis_line.contains("18"));
        assert_eq!(axis_line, bottom_axis_line);
    }

    #[test]
    fn projected_geometry_is_correctly_aligned() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 10;
        app.crosshair_y = 11;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        assert!(projected.x_min <= app.crosshair_x && projected.x_max >= app.crosshair_x);
        assert!(projected.y_min <= app.crosshair_y && projected.y_max >= app.crosshair_y);
        assert_eq!(
            projected.cell_area_col,
            projected.row_label_col + frame.row_label_cols
        );
        assert_eq!(
            projected.axis_row,
            frame.map_block.row
                + frame
                    .map_block
                    .height
                    .saturating_sub(rendered_map_block_height(projected.visible_y))
                    / 2
        );
        assert_eq!(projected.grid_top_row, projected.axis_row + 1);
        assert_eq!(
            projected.grid_bottom_row,
            projected.grid_top_row + rendered_lattice_height(projected.visible_y) - 1
        );
        assert_eq!(
            projected.bottom_axis_row,
            projected.axis_row + rendered_map_block_height(projected.visible_y) - 1
        );
        assert_eq!(
            projected.grid_right_col,
            projected.cell_area_col + usize::from(projected.visible_x) * MIN_SECTOR_WIDTH
        );
        assert_eq!(
            projected.row_label_col,
            frame.map_block.col
                + frame
                    .map_block
                    .width
                    .saturating_sub(rendered_map_block_width(
                        frame.row_label_cols,
                        projected.visible_x,
                        frame.cell_width
                    ))
                    / 2
        );
    }

    #[test]
    fn readable_mode_uses_full_map_fit_inside_shrunk_dashboard_frame() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 10;
        app.crosshair_y = 11;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        assert_eq!(projected.x_min, 1);
        assert_eq!(projected.y_min, 1);
        assert_eq!(projected.x_max, 18);
        assert_eq!(projected.y_max, 18);
        assert_eq!(projected.visible_x, 18);
        assert_eq!(projected.visible_y, 18);
        assert_eq!(
            projected.cell_area_col,
            frame.grid.col + frame.row_label_cols
        );
        assert_eq!(projected.axis_row, frame.axis_row);
        assert_eq!(projected.grid_top_row, frame.grid.row);
        assert_eq!(projected.grid_bottom_row, frame.grid.last_row());
        assert_eq!(projected.grid_right_col, frame.grid.last_col());
    }

    #[test]
    fn fill_mode_projects_into_full_center_widget() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.map_view_mode = crate::dashboard::app::state::MapViewMode::Fill;
        let frame = dashboard_layout(&app).widgets.center_map;

        // Fill mode uses the full canvas, but the map block is still snapped
        // to a multiple of `map_size` and centred inside `outer`.
        assert!(frame.map_block.last_col() <= frame.outer.last_col());
        assert!(frame.map_block.last_row() <= frame.outer.last_row());
        assert!(frame.axis_row >= frame.map_block.row);
        assert!(frame.axis_row <= frame.map_block.last_row());
        assert_eq!(frame.grid.col, frame.map_block.col);
        assert_eq!(frame.bottom_pad_row, frame.grid.last_row() + 1);
    }

    #[test]
    fn boxed_grid_draws_sector_borders_and_highlights_crosshair_sector() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 4;
        app.crosshair_y = 5;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let rect = projected
            .sector_rect([app.crosshair_x, app.crosshair_y])
            .expect("crosshair rect");
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        let marker_row = projected.sector_marker_row(app.crosshair_y);
        let marker_col = projected.sector_marker_col(app.crosshair_x);
        let label_col = projected.sector_label_col(app.crosshair_x);
        assert_eq!(buffer.row(rect.row)[rect.col].ch, GRID_WALL);
        assert_eq!(buffer.row(rect.row)[rect.col + 1].ch, GRID_DASH);
        assert_eq!(buffer.row(rect.row)[rect.col + 2].ch, ' ');
        assert_eq!(buffer.row(rect.row)[rect.col + 3].ch, GRID_DASH);
        assert_eq!(buffer.row(rect.row + 1)[rect.col].ch, ' ');
        assert_eq!(
            buffer.row(rect.row + 1)[projected.grid_right_col].ch,
            GRID_WALL
        );
        assert_eq!(buffer.row(marker_row)[marker_col].ch, ' ');
        assert_eq!(
            projected.selection_overlay(&app),
            Some(OverlaySelection {
                fg: theme::map_selection_style().fg,
                left_col: rect.col,
                right_col: rect.right_boundary_col(),
                top_row: rect.row,
                bottom_row: rect.bottom_boundary_row(),
            })
        );
        assert_eq!(
            buffer.overlay_selection(),
            projected.selection_overlay(&app)
        );
        assert_eq!(buffer.row(marker_row)[projected.row_label_col].ch, '0');
        assert_eq!(buffer.row(marker_row)[projected.row_label_col + 1].ch, '5');
        assert_eq!(
            buffer.row(marker_row)[projected.right_label_col + 1].ch,
            '0'
        );
        assert_eq!(
            buffer.row(marker_row)[projected.right_label_col + 2].ch,
            '5'
        );
        assert_eq!(buffer.row(projected.axis_row)[label_col].ch, '0');
        assert_eq!(buffer.row(projected.axis_row)[label_col + 1].ch, '4');
        assert_eq!(buffer.row(projected.bottom_axis_row)[label_col].ch, '0');
        assert_eq!(buffer.row(projected.bottom_axis_row)[label_col + 1].ch, '4');
    }

    #[test]
    fn bottom_row_selection_overlay_uses_closing_border_not_axis_row() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 4;
        app.crosshair_y = 1;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let rect = projected
            .sector_rect([app.crosshair_x, app.crosshair_y])
            .expect("crosshair rect");

        assert_eq!(
            projected.selection_overlay(&app),
            Some(OverlaySelection {
                fg: theme::map_selection_style().fg,
                left_col: rect.col,
                right_col: rect.right_boundary_col(),
                top_row: rect.row,
                bottom_row: projected.grid_bottom_row,
            })
        );
        assert_eq!(projected.grid_bottom_row + 1, projected.bottom_axis_row);
    }

    #[test]
    fn empty_sector_renders_without_dot_fill() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let coords = visible_empty_sector_matching(&app, frame, |coords| {
            coords[0] != app.crosshair_x && coords[1] != app.crosshair_y
        });
        let projected = projected_map_geometry(&app, frame, 18);
        let rect = projected.sector_rect(coords).expect("empty sector rect");
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        assert_eq!(count_char_in_rect(&buffer, rect, '·'), 0);
        assert_eq!(buffer.row(rect.row)[rect.col].ch, GRID_WALL);
        assert_eq!(
            buffer.row(projected.sector_marker_row(coords[1]))
                [projected.sector_marker_col(coords[0])]
            .ch,
            ' '
        );
    }

    #[test]
    fn viewer_fleet_empty_sector_draws_triangle_at_sector_marker_on_18_map() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 2;
        app.crosshair_y = 2;
        let frame = dashboard_layout(&app).widgets.center_map;
        let coords = visible_empty_sector_matching(&app, frame, |coords| {
            coords[0] > 3 && coords[0] < 16 && coords[1] > 3 && coords[1] < 16 && coords != [2, 2]
        });
        place_viewer_fleet(&mut app, coords);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        let fleet_col = projected.sector_marker_col(coords[0]);
        let fleet_row = projected.sector_marker_row(coords[1]);
        let fleet_cell = &buffer.row(fleet_row)[fleet_col];
        assert_eq!(fleet_cell.ch, FLEET_MARKER_EMPTY);
        assert_eq!(fleet_cell.style, theme::map_fleet_marker_style());
        let xh_row = projected.sector_marker_row(app.crosshair_y);
        let xh_col = projected.sector_marker_col(app.crosshair_x);
        assert_eq!(buffer.row(xh_row)[xh_col].ch, ' ');
        assert_eq!(
            buffer.overlay_selection(),
            projected.selection_overlay(&app)
        );
    }

    #[test]
    fn viewer_fleet_on_owned_world_draws_owned_world_marker() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let coords =
            visible_world_coords_matching_marker_kind(&app, frame, StarmapMarkerKind::Owned);
        place_viewer_fleet(&mut app, coords);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        let fleet_col = projected.sector_marker_col(coords[0]);
        let fleet_row = projected.sector_marker_row(coords[1]);
        let fleet_cell = &buffer.row(fleet_row)[fleet_col];
        assert_eq!(fleet_cell.ch, FLEET_MARKER_OWNED_WORLD);
        assert_eq!(fleet_cell.style.fg, theme::map_fleet_marker_style().fg);
        assert_eq!(fleet_cell.style.bg, theme::body_style().bg);
    }

    #[test]
    fn fleet_marker_uses_world_variant_for_non_owned_markers() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let mut unowned = make_world([1, 1]);
        unowned.known_owner_empire_id = Some(0);
        let mut partial = make_world([1, 1]);
        partial.known_name = Some(String::from("Partial"));
        let unknown = make_world([1, 1]);

        assert_eq!(
            fleet_marker_for_sector(&app, 1, Some(&unowned)),
            FLEET_MARKER_WORLD
        );
        assert_eq!(
            fleet_marker_for_sector(&app, 1, Some(&partial)),
            FLEET_MARKER_WORLD
        );
        assert_eq!(
            fleet_marker_for_sector(&app, 1, Some(&unknown)),
            FLEET_MARKER_WORLD
        );
    }

    #[test]
    fn fleet_marker_uses_crosshair_background_on_active_sector() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let coords = visible_empty_sector(&app, frame);
        app.crosshair_x = coords[0];
        app.crosshair_y = coords[1];
        place_viewer_fleet(&mut app, coords);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let rect = projected.sector_rect(coords).expect("fleet sector rect");
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        let marker_row = projected.sector_marker_row(app.crosshair_y);
        let marker_col = projected.sector_marker_col(app.crosshair_x);
        let fleet_cell = &buffer.row(marker_row)[marker_col];
        assert_eq!(fleet_cell.ch, FLEET_MARKER_EMPTY);
        assert_eq!(
            fleet_cell.style,
            theme::map_fleet_marker_style_on(
                theme::map_center_style().bg,
                theme::map_center_style().bold,
            )
        );
        let expected_separator_midpoint = if coords[1] == projected.y_max {
            GRID_DASH
        } else {
            ' '
        };
        assert_eq!(buffer.row(rect.row)[rect.col + 1].ch, GRID_DASH);
        assert_eq!(
            buffer.row(rect.row)[rect.col + 2].ch,
            expected_separator_midpoint
        );
        assert_eq!(buffer.row(rect.row)[rect.col + 2].style, theme::dim_style());
        assert_eq!(buffer.row(rect.row + 1)[rect.col].style, theme::dim_style());
    }

    #[test]
    fn adjacent_fleet_markers_keep_both_sector_centers_visible() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let (left_coords, right_coords) = adjacent_empty_sector_pair(&app, frame);
        place_viewer_fleet(&mut app, left_coords);
        place_viewer_fleet_at_record(&mut app, 1, right_coords);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        let left_row = projected.sector_marker_row(left_coords[1]);
        let left_col = projected.sector_marker_col(left_coords[0]);
        assert_eq!(buffer.row(left_row)[left_col].ch, FLEET_MARKER_EMPTY);
        let right_row = projected.sector_marker_row(right_coords[1]);
        let right_col = projected.sector_marker_col(right_coords[0]);
        assert_eq!(buffer.row(right_row)[right_col].ch, FLEET_MARKER_EMPTY);
    }

    #[test]
    fn benchmark_laptop_height_uses_vertical_viewport_on_large_maps() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(187, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 45);

        assert_eq!(
            projected.visible_x as usize,
            (frame
                .grid
                .width
                .saturating_sub(frame.row_label_cols)
                .saturating_sub(1)
                / MIN_SECTOR_WIDTH)
                .min(45)
        );
        assert!(projected.visible_y < 45);
        assert_eq!(
            projected.visible_y as usize,
            frame.grid.height.saturating_sub(1) / MIN_SECTOR_HEIGHT
        );
        assert!(projected.y_min <= app.crosshair_y && projected.y_max >= app.crosshair_y);
    }

    #[test]
    fn narrow_terminal_caps_horizontal_viewport_independently() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(93, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 45);

        assert!(projected.visible_x < 45);
        assert!(projected.visible_y < 45);
        assert_eq!(
            projected.visible_x as usize,
            frame
                .grid
                .width
                .saturating_sub(frame.row_label_cols)
                .saturating_sub(1)
                / MIN_SECTOR_WIDTH
        );
        assert_eq!(
            projected.visible_y as usize,
            frame.grid.height.saturating_sub(1) / MIN_SECTOR_HEIGHT
        );
    }

    #[test]
    fn map_hit_test_returns_visible_sector_coordinates() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let rect = projected.sector_rect([6, 7]).expect("visible sector rect");

        assert_eq!(
            screen_sector_at_point(&app, frame, rect.marker_col(), rect.marker_row()),
            Some([6, 7])
        );
        assert_eq!(
            screen_sector_at_point(&app, frame, rect.col, rect.separator_row()),
            Some([6, 7])
        );
    }

    #[test]
    fn map_hit_test_ignores_axis_labels_and_padding() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;

        assert_eq!(
            screen_sector_at_point(&app, frame, frame.grid.col, frame.axis_row),
            None
        );
        assert_eq!(
            screen_sector_at_point(
                &app,
                frame,
                frame.grid.col.saturating_sub(1),
                frame.grid.row.saturating_sub(1)
            ),
            None
        );
    }

    #[test]
    fn map_hit_test_returns_none_outside_lattice() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 10;
        app.crosshair_y = 11;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        assert_eq!(
            screen_sector_at_point(
                &app,
                frame,
                projected.cell_area_col,
                projected.grid_top_row.saturating_sub(1)
            ),
            None
        );
        assert_eq!(
            screen_sector_at_point(
                &app,
                frame,
                projected.cell_area_col.saturating_sub(1),
                projected.grid_top_row
            ),
            None
        );
        assert_eq!(
            screen_sector_at_point(
                &app,
                frame,
                projected.grid_right_col + 1,
                projected.grid_top_row
            ),
            None
        );
        assert_eq!(
            screen_sector_at_point(
                &app,
                frame,
                projected.cell_area_col,
                projected.grid_bottom_row + 1
            ),
            None
        );
    }

    #[test]
    fn sector_marker_positions_match_sector_rect_content_centers() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        for x in projected.x_min..=projected.x_max {
            for y in projected.y_min..=projected.y_max {
                let rect = projected.sector_rect([x, y]).expect("sector rect");
                let expected_col = rect.marker_col();
                let expected_row = rect.marker_row();
                let actual_col = projected.sector_marker_col(x);
                let actual_row = projected.sector_marker_row(y);
                assert_eq!(
                    actual_col, expected_col,
                    "col marker at x={x}: expected {expected_col}, got {actual_col}"
                );
                assert_eq!(
                    actual_row, expected_row,
                    "row marker at y={y}: expected {expected_row}, got {actual_row}"
                );
            }
        }
    }

    #[test]
    fn initial_sentinel_centers_viewport_on_crosshair() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(93, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        assert_eq!(app.starmap_viewport_x_min, 0);
        assert_eq!(app.starmap_viewport_y_min, 0);

        advance_starmap_viewport(&mut app);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 45);

        assert!(projected.x_min <= app.crosshair_x && projected.x_max >= app.crosshair_x);
        assert!(projected.y_min <= app.crosshair_y && projected.y_max >= app.crosshair_y);

        // With the sentinel the viewport should be centred on the crosshair.
        let expected_x_min = app
            .crosshair_x
            .saturating_sub(projected.visible_x / 2)
            .max(1);
        let expected_y_min = app
            .crosshair_y
            .saturating_sub(projected.visible_y / 2)
            .max(1);
        assert_eq!(projected.x_min, expected_x_min);
        assert_eq!(projected.y_min, expected_y_min);
    }

    #[test]
    fn crosshair_move_inside_dead_zone_does_not_pan() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(93, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        advance_starmap_viewport(&mut app);
        let initial_x_min = app.starmap_viewport_x_min;
        let initial_y_min = app.starmap_viewport_y_min;
        // Pick a target well inside the dead-zone interior.
        let x_dead = app.starmap_viewport_x_min + STARMAP_SCROLL_MARGIN + 1;
        let y_dead = app.starmap_viewport_y_min + STARMAP_SCROLL_MARGIN + 1;

        // Move crosshair to the dead-zone interior — viewport should stay put.
        app.crosshair_x = x_dead;
        advance_starmap_viewport(&mut app);
        assert_eq!(app.starmap_viewport_x_min, initial_x_min);

        app.crosshair_y = y_dead;
        advance_starmap_viewport(&mut app);
        assert_eq!(app.starmap_viewport_y_min, initial_y_min);
    }

    #[test]
    fn crosshair_crossing_margin_trigger_pans_one_sector() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(93, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        advance_starmap_viewport(&mut app);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 45);
        let visible_x = projected.visible_x;
        let visible_y = projected.visible_y;

        // Move crosshair to just past the right margin boundary.
        let right_boundary = app.starmap_viewport_x_min + visible_x - 1 - STARMAP_SCROLL_MARGIN;
        app.crosshair_x = right_boundary + 1;
        let old_x_min = app.starmap_viewport_x_min;
        advance_starmap_viewport(&mut app);
        assert_eq!(app.starmap_viewport_x_min, old_x_min + 1);

        // Same for bottom margin.
        let bottom_boundary = app.starmap_viewport_y_min + visible_y - 1 - STARMAP_SCROLL_MARGIN;
        app.crosshair_y = bottom_boundary + 1;
        let old_y_min = app.starmap_viewport_y_min;
        advance_starmap_viewport(&mut app);
        assert_eq!(app.starmap_viewport_y_min, old_y_min + 1);
    }

    #[test]
    fn small_map_falls_back_to_clamp_only_behavior() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        // 18-map on 160x45: full map fits (visible == map_size), so the
        // margin fallback disables dead-zone scrolling.
        app.crosshair_x = 5;
        app.crosshair_y = 5;
        advance_starmap_viewport(&mut app);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        if projected.visible_x >= 18 {
            assert_eq!(app.starmap_viewport_x_min, 1);
        }
        if projected.visible_y >= 18 {
            assert_eq!(app.starmap_viewport_y_min, 1);
        }
    }

    #[test]
    fn jump_outside_viewport_pans_until_crosshair_within_margin() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(93, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        advance_starmap_viewport(&mut app);
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 45);
        let visible_x = projected.visible_x;
        let visible_y = projected.visible_y;

        // Jump far to the bottom-right corner of the map.
        app.crosshair_x = 43;
        app.crosshair_y = 43;
        advance_starmap_viewport(&mut app);

        // The viewport should pan so the crosshair is inside the margin.
        let x_max = app.starmap_viewport_x_min + visible_x - 1;
        let y_max = app.starmap_viewport_y_min + visible_y - 1;
        assert!(app.crosshair_x <= x_max);
        assert!(app.crosshair_x >= app.starmap_viewport_x_min + STARMAP_SCROLL_MARGIN);
        assert!(app.crosshair_y <= y_max);
        assert!(app.crosshair_y >= app.starmap_viewport_y_min + STARMAP_SCROLL_MARGIN);
    }

    fn make_world(coords: [u8; 2]) -> PlayerStarmapWorld {
        PlayerStarmapWorld {
            planet_record_index_1_based: 1,
            coords,
            intel_tier: IntelTier::Unknown,
            known_name: None,
            known_owner_empire_id: None,
            known_owner_empire_name: None,
            known_potential_production: None,
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: None,
            known_current_production: None,
            known_stored_points: None,
            known_docked_summary: None,
            known_orbit_summary: None,
        }
    }

    fn place_viewer_fleet(app: &mut DashApp, coords: [u8; 2]) {
        place_viewer_fleet_at_record(app, 0, coords);
    }

    fn place_viewer_fleet_at_record(app: &mut DashApp, fleet_idx: usize, coords: [u8; 2]) {
        let fleet = &mut app.game_data.fleets[fleet_idx];
        fleet.owner.0 = app.player_record_index_1_based as u8;
        fleet.sector = nc_data::Coords::from_array(coords);
        fleet.ships.destroyers = 1;
        app.game_data_revision += 1;
    }

    fn visible_empty_sector(app: &DashApp, frame: MapWidgetFrame) -> [u8; 2] {
        visible_empty_sector_matching(app, frame, |_| true)
    }

    fn visible_empty_sector_matching<F>(
        app: &DashApp,
        frame: MapWidgetFrame,
        predicate: F,
    ) -> [u8; 2]
    where
        F: Fn([u8; 2]) -> bool,
    {
        let snapshot_map = snapshot_map_for_app(app);
        let projection = projection_for_snapshot_map(app, &snapshot_map);
        let projected = projected_map_geometry(app, frame, 18);
        for row_y in (projected.y_min..=projected.y_max).rev() {
            for col_x in projected.x_min..=projected.x_max {
                let coords = [col_x, row_y];
                if predicate(coords) && projection_world_at(&projection, coords).is_none() {
                    return coords;
                }
            }
        }
        panic!("expected an empty visible sector");
    }

    fn adjacent_empty_sector_pair(app: &DashApp, frame: MapWidgetFrame) -> ([u8; 2], [u8; 2]) {
        let snapshot_map = snapshot_map_for_app(app);
        let projection = projection_for_snapshot_map(app, &snapshot_map);
        let projected = projected_map_geometry(app, frame, 18);
        for row_y in (projected.y_min..=projected.y_max).rev() {
            for col_x in projected.x_min..projected.x_max {
                let left = [col_x, row_y];
                let right = [col_x + 1, row_y];
                if projection_world_at(&projection, left).is_none()
                    && projection_world_at(&projection, right).is_none()
                {
                    return (left, right);
                }
            }
        }
        panic!("expected adjacent empty sectors");
    }

    fn count_char_in_rect(buffer: &PlayfieldBuffer, rect: SectorRect, expected: char) -> usize {
        (rect.row..rect.row + rect.height)
            .map(|row| {
                (rect.col..rect.col + rect.width)
                    .filter(|&col| buffer.row(row)[col].ch == expected)
                    .count()
            })
            .sum()
    }

    fn visible_world_coords_matching_marker_kind(
        app: &DashApp,
        frame: MapWidgetFrame,
        expected_kind: StarmapMarkerKind,
    ) -> [u8; 2] {
        visible_world_coords_matching(app, frame, |_, world| {
            marker_kind_for_world(app, app.player_record_index_1_based as u8, world)
                == expected_kind
        })
    }

    fn visible_world_coords_matching<F>(
        app: &DashApp,
        frame: MapWidgetFrame,
        predicate: F,
    ) -> [u8; 2]
    where
        F: Fn([u8; 2], &PlayerStarmapWorld) -> bool,
    {
        let snapshot_map = snapshot_map_for_app(app);
        let projection = projection_for_snapshot_map(app, &snapshot_map);
        let projected = projected_map_geometry(app, frame, 18);
        for row_y in (projected.y_min..=projected.y_max).rev() {
            for col_x in projected.x_min..=projected.x_max {
                let coords = [col_x, row_y];
                let Some(world) = projection_world_at(&projection, coords) else {
                    continue;
                };
                if predicate(coords, world) {
                    return coords;
                }
            }
        }
        panic!("expected a visible world matching the predicate");
    }
}

#[cfg(test)]
fn owner_label(game_data: &CoreGameData, known_owner_empire_id: Option<u8>) -> String {
    match known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => game_data
            .players
            .get(owner.saturating_sub(1) as usize)
            .map(|player| {
                if player.mode == nc_data::PlayerMode::CivilDisorder {
                    String::from("ICD")
                } else {
                    format!("#{owner}")
                }
            })
            .unwrap_or_else(|| format!("#{owner}")),
        None => String::from("?"),
    }
}

#[cfg(test)]
fn known_u8(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}

#[cfg(test)]
fn known_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}
