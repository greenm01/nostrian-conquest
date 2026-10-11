//! Left panel: active fleet summary stats.

use crate::dashboard::app::state::DashApp;
use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::theme::classic::status_value_style;
use nc_data::{Order, STARDOCK_SLOT_COUNT};

pub(crate) const TITLE: &str = "MY FLEETS";
pub(crate) const MIN_BODY_ROWS: usize = 7;

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

    let mut total_fleets = 0;
    let mut total_ships = 0;
    let mut docked = 0;
    let mut moving = 0;
    let mut combat = 0;
    let mut guarding = 0;
    let mut holding = 0;

    for planet in &app.game_data.planets {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }

        for slot in 0..STARDOCK_SLOT_COUNT {
            docked += u32::from(planet.stardock_count_raw(slot));
        }
    }

    for fleet in &app.game_data.fleets {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }

        total_fleets += 1;
        total_ships += fleet.battleship_count() as u32
            + fleet.cruiser_count() as u32
            + fleet.destroyer_count() as u32
            + fleet.scout_count() as u32
            + fleet.troop_transport_count() as u32
            + fleet.etac_count() as u32;

        match fleet.standing_order_kind() {
            Order::MoveOnly
            | Order::SeekHome
            | Order::JoinAnotherFleet
            | Order::RendezvousSector => {
                moving += 1;
            }
            Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld => {
                combat += 1;
            }
            Order::PatrolSector | Order::GuardStarbase | Order::GuardBlockadeWorld => {
                guarding += 1;
            }
            Order::HoldPosition => {
                holding += 1;
            }
            _ => {}
        }
    }

    let mut summary_rows = vec![
        (
            layout::format_left_column_value("Tot Fleets", &total_fleets.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Tot Ships", &total_ships.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Docked", &docked.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Moving", &moving.to_string()),
            crate::dashboard::theme::dim_style(),
        ),
        (
            layout::format_left_column_value("Combat", &combat.to_string()),
            if combat > 0 {
                crate::dashboard::theme::enemy_style()
            } else {
                crate::dashboard::theme::dim_style()
            },
        ),
        (
            layout::format_left_column_value("Guarding", &guarding.to_string()),
            if guarding > 0 {
                crate::dashboard::theme::friendly_style()
            } else {
                crate::dashboard::theme::dim_style()
            },
        ),
        (
            layout::format_left_column_value("Holding", &holding.to_string()),
            crate::dashboard::theme::dim_style(),
        ),
    ];

    let active = app
        .game_data
        .empire_active_duty_summary(app.player_record_index_1_based);
    if active.battleships > 0 {
        summary_rows.push((
            layout::format_left_column_value("BBs", &active.battleships.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if active.cruisers > 0 {
        summary_rows.push((
            layout::format_left_column_value("CAs", &active.cruisers.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if active.destroyers > 0 {
        summary_rows.push((
            layout::format_left_column_value("DDs", &active.destroyers.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if active.scouts > 0 {
        summary_rows.push((
            layout::format_left_column_value("SCs", &active.scouts.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if active.transports > 0 {
        summary_rows.push((
            layout::format_left_column_value("TTs", &active.transports.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if active.etacs > 0 {
        summary_rows.push((
            layout::format_left_column_value("ETs", &active.etacs.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }
    if active.starbases > 0 {
        summary_rows.push((
            layout::format_left_column_value("SBs", &active.starbases.to_string()),
            crate::dashboard::theme::dim_style(),
        ));
    }

    summary_rows
}
