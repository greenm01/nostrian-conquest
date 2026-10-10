//! Left panel: owned planet summary.

use crate::dashboard::app::state::DashApp;
use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::theme::classic::status_value_style;
use nc_data::{ProductionItemKind, build_queue_unit_counts};

pub(crate) const TITLE: &str = "MY PLANETS";
pub(crate) const MIN_BODY_ROWS: usize = 6;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(
        buf,
        frame,
        TITLE,
        crate::dashboard::theme::section_title_style(),
    );

    for (i, (row, style)) in body_rows(app).into_iter().enumerate() {
        if i >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, i, &row, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
    let owner_slot = app.player_record_index_1_based as u8;

    let mut owned_count = 0;
    let mut stardocks_active = 0;
    let mut starbases_built = 0;
    let mut armies_mustered = 0;
    let mut ground_batteries = 0;
    let mut building = 0u32;
    let mut vulnerable_worlds = 0;
    let mut build_bbs = 0u32;
    let mut build_cas = 0u32;
    let mut build_dds = 0u32;
    let mut build_scs = 0u32;
    let mut build_tts = 0u32;
    let mut build_ets = 0u32;
    let mut build_sbs = 0u32;

    let starbase_coords: std::collections::HashSet<[u8; 2]> = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.base_id_raw() != 0)
        .map(|b| b.coords_raw())
        .collect();

    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        owned_count += 1;

        let coords = planet.coords_raw();
        if starbase_coords.contains(&coords) {
            starbases_built += 1;
        }

        let armies = planet.army_count_raw();
        armies_mustered += armies as u32;

        let batteries = planet.ground_batteries_raw();
        ground_batteries += batteries as u32;

        for (kind_raw, qty) in build_queue_unit_counts(planet) {
            let kind = ProductionItemKind::from_raw(kind_raw);
            building += qty;
            match kind {
                ProductionItemKind::Battleship => build_bbs += qty,
                ProductionItemKind::Cruiser => build_cas += qty,
                ProductionItemKind::Destroyer => build_dds += qty,
                ProductionItemKind::Scout => build_scs += qty,
                ProductionItemKind::Transport => build_tts += qty,
                ProductionItemKind::Etac => build_ets += qty,
                ProductionItemKind::Starbase => build_sbs += qty,
                _ => {}
            }
        }

        if armies == 0 && batteries == 0 {
            vulnerable_worlds += 1;
        }

        let has_stardock_units = planet.stardock_count_raw(0) > 0
            || planet.stardock_count_raw(1) > 0
            || planet.stardock_count_raw(2) > 0
            || planet.stardock_count_raw(3) > 0
            || planet.stardock_count_raw(4) > 0
            || planet.stardock_count_raw(5) > 0
            || planet.stardock_count_raw(6) > 0;

        if has_stardock_units {
            stardocks_active += 1;
        }
    }

    let mut summary_rows = vec![
        (
            layout::format_left_column_value("Tot Worlds", &owned_count.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Stardocks", &stardocks_active.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Starbases", &starbases_built.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Tot Armies", &armies_mustered.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("GBs", &ground_batteries.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Building", &building.to_string()),
            status_value_style(),
        ),
    ];

    if vulnerable_worlds > 0 {
        summary_rows.push((
            layout::format_left_column_value("Vulnerable", &vulnerable_worlds.to_string()),
            crate::dashboard::theme::enemy_style(),
        ));
    }

    if build_bbs > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld BBs", &build_bbs.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if build_cas > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld CAs", &build_cas.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if build_dds > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld DDs", &build_dds.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if build_scs > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld SCs", &build_scs.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if build_tts > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld TTs", &build_tts.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if build_ets > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld ETs", &build_ets.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if build_sbs > 0 {
        summary_rows.push((
            layout::format_left_column_value("Bld SBs", &build_sbs.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }

    summary_rows
}

#[cfg(test)]
mod tests {
    use super::body_rows;
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn planet_summary_includes_total_building_row() {
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
        let homeworld = &mut app.game_data.planets.records[0];
        homeworld.set_build_kind_raw(0, 3);
        homeworld.set_build_count_raw(0, 90);

        assert!(
            body_rows(&app)
                .iter()
                .any(|(row, _)| row.contains("Building") && row.ends_with("2"))
        );
    }
}
