use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, ScreenFrame, ScreenId, format_sector_coords};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    app.enforce_valid_fleet_filter();
    if matches!(
        app.current_screen,
        ScreenId::FleetList | ScreenId::FleetListFilterPrompt | ScreenId::FleetListSortPrompt
    ) {
        app.normalize_fleet_list_selection();
    }
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        player_activity_states: &app.player_activity_states,
        player_lifecycle_states: &app.player_lifecycle_states,
        winner_state: app.winner_state,
        planet_intel_snapshots: &app.planet_intel_snapshots,
        owned_planet_years: &app.owned_planet_years,
        geometry: app.screen_geometry,
    };
    let inline_transport = match app.fleet.menu_prompt_mode {
        Some(crate::domains::fleet::state::FleetMenuPromptMode::TransportQuantity(mode)) => {
            let planet = app.current_planet_transport_planet_row(mode)?;
            let fleet = app.current_planet_transport_fleet_row(mode)?;
            Some((
                mode,
                format!(
                    "Planet: {} {}   Fleet {:02}",
                    planet.planet_name,
                    format_sector_coords(planet.coords),
                    fleet.fleet_number
                ),
            ))
        }
        _ => None,
    };
    match app.current_screen {
        ScreenId::FleetMenu => app.fleet_menu.render_with_notice(
            app.command_menu_notice.as_deref(),
            app.expert_mode,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::Fleet,
            app.fleet.menu_prompt_mode,
            app.fleet_menu_prompt_label().as_deref(),
            &app.fleet.menu_prompt_default_value,
            &app.fleet.menu_prompt_input,
            app.fleet.menu_prompt_status.as_ref(),
            inline_transport.as_ref().map(|(mode, _)| *mode),
            inline_transport
                .as_ref()
                .map(|(_, summary)| summary.as_str()),
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
        ),
        ScreenId::FleetList => app.fleet_list.render_with_filter_clause(
            frame.geometry,
            &app.fleet_list_rows(),
            &app.fleet.group_selected_fleets,
            app.fleet.list_sort,
            app.fleet.list_sort_direction,
            app.fleet.list_filter,
            app.fleet.list_filter_clause.as_ref(),
            app.fleet.scroll_offset,
            app.fleet.cursor,
            &app.fleet.list_input,
            app.fleet.list_status.as_deref(),
            app.fleet.list_dismiss_message.as_deref(),
            app.fleet_menu_prompt_label().as_deref(),
            &app.fleet.menu_prompt_default_value,
            &app.fleet.menu_prompt_input,
            app.fleet.menu_prompt_status.as_ref(),
        ),
        ScreenId::FleetListFilterPrompt => app
            .fleet_list
            .render_filter_prompt_with_selected_filter_clause(
                frame.geometry,
                &app.fleet_list_rows(),
                &app.fleet.group_selected_fleets,
                app.fleet.list_sort,
                app.fleet.list_sort_direction,
                app.fleet.list_filter,
                app.fleet.list_filter_clause.as_ref(),
                app.fleet.scroll_offset,
                app.fleet.cursor,
                app.fleet.list_filter_prompt_mode,
                &app.fleet.list_filter_prompt_default_value,
                &app.fleet.list_filter_prompt_input,
                app.fleet.list_filter_prompt_status.as_deref(),
                app.fleet.list_filter_prompt_dismiss_message.as_deref(),
                app.fleet
                    .list_filter_pending_column
                    .map(|column| column.code),
            ),
        ScreenId::FleetListSortPrompt => app.fleet_list.render_sort_prompt_with_filter_clause(
            frame.geometry,
            &app.fleet_list_rows(),
            &app.fleet.group_selected_fleets,
            app.fleet.list_sort,
            app.fleet.list_sort_direction,
            app.fleet.list_filter,
            app.fleet.list_filter_clause.as_ref(),
            app.fleet.scroll_offset,
            app.fleet.cursor,
            &app.fleet.list_filter_prompt_default_value,
            &app.fleet.list_filter_prompt_input,
            app.fleet.list_filter_prompt_status.as_deref(),
            app.fleet.list_filter_prompt_dismiss_message.as_deref(),
        ),
        ScreenId::FleetReview => {
            let rows = if app.fleet.review_return_to_list {
                app.fleet_list_rows()
            } else {
                app.fleet_rows()
            };
            let row = rows
                .get(app.fleet.review_index)
                .ok_or("fleet review row missing")?;
            app.fleet_review.render(
                row,
                app.fleet.review_index,
                rows.len(),
                app.fleet.review_return_to_list,
            )
        }
        ScreenId::FleetOrder => {
            let row = app
                .fleet_order_selected_row()
                .ok_or("fleet order row missing")?;
            let current_order_label = app.fleet_order_current_order_label();
            let new_order_label = app.fleet_order_new_order_label();
            let current_year = app.game_data.conquest.game_year;
            let status_line = app.fleet_order_target_status_line();
            let target_prompt = app.fleet_order_target_prompt();
            let target_default = app.fleet_order_target_default();
            let target_x_default = app.fleet_order_target_x_default();
            let target_x_input = app.fleet_order_target_x_display_input();
            let target_y_default = app.fleet_order_target_y_default();
            let target_y_input = app.fleet_order_target_y_display_input();
            app.fleet_order.render(
                &row,
                &current_order_label,
                &new_order_label,
                app.fleet.order_mode,
                &status_line,
                &target_prompt,
                &target_default,
                &app.fleet.order_input,
                &target_x_default,
                &target_x_input,
                &target_y_default,
                &target_y_input,
                &app.fleet.order_confirm_input,
                current_year,
                app.fleet.order_status.as_deref(),
            )
        }
        ScreenId::FleetGroupOrder => {
            let rows = app.fleet_rows();
            let status_line = app.fleet_group_target_status_line();
            let new_order_label = app
                .fleet
                .group_mission_code
                .map(crate::domains::fleet::screens::fleet::fleet_order_label)
                .unwrap_or("Unknown")
                .to_string();
            let current_year = app.game_data.conquest.game_year;
            let target_prompt = app.fleet_group_target_prompt();
            let target_default = app.fleet_group_target_default();
            let target_x_default = app.fleet_group_target_x_default_value();
            let target_x_input = app.fleet_group_target_x_display_input();
            let target_y_default = app.fleet_group_target_y_default_value();
            let target_y_input = app.fleet_group_target_y_display_input();
            app.fleet_group.render(
                frame.geometry,
                &rows,
                app.fleet.group_scroll_offset,
                app.fleet.group_cursor,
                &app.fleet.group_selected_fleets,
                app.fleet.group_mode,
                &status_line,
                &new_order_label,
                &target_prompt,
                &target_default,
                &app.fleet.group_input,
                &target_x_default,
                &target_x_input,
                &target_y_default,
                &target_y_input,
                &app.fleet.group_confirm_input,
                current_year,
                app.fleet.group_status.as_deref(),
            )
        }
        ScreenId::FleetMissionPicker => app.fleet_mission_picker.render(
            app.fleet.mission_picker_cursor,
            &app.fleet.mission_picker_input,
            &app.fleet_mission_picker_enabled_flags(),
            app.fleet.mission_picker_status.as_deref(),
        ),
        ScreenId::FleetTransfer => {
            let donor_row = app
                .fleet_transfer_donor_row()
                .ok_or("fleet transfer donor row missing")?;
            let host_row = app
                .fleet_transfer_host_row()
                .ok_or("fleet transfer host row missing")?;
            let status = app.fleet.transfer_status.as_deref();
            let (prompt, default) = app.fleet_transfer_prompt_and_default();
            let crippled_quantity_hint = match app.fleet.transfer_mode {
                crate::screen::FleetTransferMode::EnteringQuantity(class) => app
                    .fleet
                    .transfer_donor_record_index_1_based
                    .and_then(|idx| app.game_data.fleets.get(idx - 1))
                    .is_some_and(|donor| {
                        nc_data::crippled_left_to_stage(
                            donor,
                            &app.fleet.transfer_selection,
                            nc_data::ShipCategory::from(class),
                        ) > 0
                    }),
                _ => false,
            };
            let donor_ships = app.fleet_transfer_source_summary();
            let host_ships = app.fleet_transfer_destination_summary();
            let staged_summary = app.fleet_transfer_staged_summary();
            let remaining_summary = app.fleet_transfer_remaining_summary();
            let projected_destination_summary = app.fleet_transfer_projected_destination_summary();
            app.fleet_transfer.render(
                &donor_row,
                &host_row,
                app.fleet.transfer_mode,
                crippled_quantity_hint,
                &app.fleet.transfer_input,
                status,
                &prompt,
                &default,
                &donor_ships,
                &host_ships,
                &staged_summary,
                &remaining_summary,
                &projected_destination_summary,
            )
        }
        ScreenId::FleetDetach => {
            let donor_row = app
                .fleet_detach_donor_row()
                .ok_or("fleet detach donor row missing")?;
            let (prompt, default) = app.fleet_detach_prompt_and_default();
            let crippled_quantity_hint = match app.fleet.detach_mode {
                crate::screen::FleetDetachMode::EnteringQuantity(class) => app
                    .fleet
                    .detach_donor_record_index_1_based
                    .and_then(|idx| app.game_data.fleets.get(idx - 1))
                    .is_some_and(|donor| {
                        nc_data::crippled_left_to_stage(
                            donor,
                            &app.fleet.detach_selection,
                            nc_data::ShipCategory::from(class),
                        ) > 0
                    }),
                _ => false,
            };
            let staged_summary = app.fleet_detach_staged_summary();
            let remaining_summary = app.fleet_detach_remaining_summary();
            let status = app.fleet.detach_status.as_deref();
            let last_commissioned = app.fleet.detach_last_commissioned.as_deref();
            app.fleet_detach.render(
                &donor_row,
                app.fleet.detach_mode,
                crippled_quantity_hint,
                &prompt,
                &default,
                &app.fleet.detach_input,
                &staged_summary,
                &remaining_summary,
                status,
                last_commissioned,
            )
        }
        ScreenId::FleetEta => {
            let row = app
                .fleet_eta_selected_row()
                .ok_or("fleet eta row missing")?;
            app.fleet_eta.render(
                &row,
                app.fleet.eta_mode,
                app.fleet_eta_default_destination(),
                &app.fleet.eta_destination_input,
                &app.fleet.eta_include_system_input,
                app.fleet.eta_status.as_deref(),
            )
        }
        ScreenId::FleetMessage => app.fleet_message.render(
            app.fleet
                .dismiss_message
                .as_deref()
                .unwrap_or("Fleet command completed."),
        ),
        _ => unreachable!("fleet views called for non-fleet screen"),
    }
}
