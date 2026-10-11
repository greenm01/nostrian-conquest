use crate::app::state::App;
use crate::domains::planet::state::{PlanetMenuTransportPromptMode, PlanetScorchPromptMode};
use crate::screen::{
    PlayfieldBuffer, ScreenFrame, ScreenId, build_unit_spec_by_kind, format_sector_coords,
};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    app.enforce_valid_planet_list_filter();
    app.enforce_valid_planet_database_filter();
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
    let inline_transport = match app.planet.transport_prompt_mode {
        Some(PlanetMenuTransportPromptMode::Quantity(mode)) => {
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
    let transport_prompt_label = app.planet_transport_prompt_label();
    let list_transport_prompt_label = app.planet_list_transport_prompt_label();
    let scorch_planet_prompt_active =
        app.planet.scorch_prompt_mode == Some(PlanetScorchPromptMode::Planet);
    let inline_list_transport = if matches!(app.current_screen, ScreenId::PlanetList(_, _)) {
        inline_transport.as_ref()
    } else {
        None
    };
    let inline_list_scorch_summary = if matches!(app.current_screen, ScreenId::PlanetList(_, _))
        && matches!(
            app.planet.scorch_prompt_mode,
            Some(PlanetScorchPromptMode::Confirm1)
                | Some(PlanetScorchPromptMode::Confirm2)
                | Some(PlanetScorchPromptMode::Confirm3)
        ) {
        app.selected_planet_scorch_name_and_coords()
            .map(|(name, coords)| {
                let verb = if app.selected_planet_scorched() {
                    "Rescind Scorch-Earth on"
                } else {
                    "Scorch"
                };
                format!("{verb} \"{}\" at {}.", name, format_sector_coords(coords))
            })
    } else {
        None
    };
    let list_scorch_confirm_prompt = app.planet_list_scorch_confirm_prompt();
    let menu_prompt_label = if scorch_planet_prompt_active {
        Some("Scorch Planet XX ")
    } else if matches!(
        app.planet.scorch_prompt_mode,
        Some(PlanetScorchPromptMode::Confirm1)
            | Some(PlanetScorchPromptMode::Confirm2)
            | Some(PlanetScorchPromptMode::Confirm3)
    ) {
        app.planet_scorch_confirm_prompt()
    } else {
        transport_prompt_label.as_deref()
    };
    let menu_prompt_default = if scorch_planet_prompt_active {
        app.planet.scorch_prompt_default_value.as_str()
    } else {
        &app.planet.transport_prompt_default_value
    };
    let menu_prompt_input = if scorch_planet_prompt_active {
        app.planet.scorch_prompt_input.as_str()
    } else {
        &app.planet.transport_prompt_input
    };
    let menu_prompt_status = if scorch_planet_prompt_active {
        app.planet.scorch_prompt_status.as_deref()
    } else {
        app.planet.transport_status.as_deref()
    };
    match app.current_screen {
        ScreenId::PlanetMenu => app.planet_menu.render_with_notice(
            app.command_menu_notice.as_deref(),
            app.expert_mode,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::Planet,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
            app.planet.tax_prompt_active,
            &app.game_data.players[app.player.record_index_1_based - 1]
                .tax_rate
                .to_string(),
            &app.planet.tax_input,
            app.planet.tax_error.as_deref(),
            app.planet.tax_notice.as_deref(),
            app.planet.auto_commission_prompt_active,
            app.planet.scorch_prompt_mode,
            &app.planet_scorch_warning_lines(),
            menu_prompt_label,
            menu_prompt_default,
            menu_prompt_input,
            menu_prompt_status,
            inline_transport.as_ref().map(|(mode, _)| *mode),
            inline_transport
                .as_ref()
                .map(|(_, summary)| summary.as_str()),
        ),
        ScreenId::PlanetTransportPlanetSelect(mode) => app.planet_transport.render_planet_select(
            frame.geometry,
            "COMMAND",
            mode,
            &app.planet_transport_planet_rows(mode),
            app.planet.transport_planet_scroll_offset,
            app.planet.transport_planet_cursor,
            &app.planet.transport_planet_input,
            app.planet_transport_planet_default_coords(mode),
            app.planet.transport_status.as_deref(),
        ),
        ScreenId::PlanetTransportFleetSelect(mode) => app.planet_transport.render_fleet_select(
            frame.geometry,
            "COMMAND",
            mode,
            &app.current_planet_transport_planet_row(mode)?,
            &app.current_planet_transport_fleet_rows(mode)?,
            app.planet.transport_fleet_scroll_offset,
            app.planet.transport_fleet_cursor,
            &app.planet.transport_qty_input,
            app.planet.transport_status.as_deref(),
        ),
        ScreenId::PlanetTransportQuantityPrompt(mode) => {
            app.planet_transport.render_quantity_prompt(
                "COMMAND",
                mode,
                &app.current_planet_transport_planet_row(mode)?,
                &app.current_planet_transport_fleet_row(mode)?,
                &app.planet.transport_qty_input,
                app.planet.transport_status.as_deref(),
            )
        }
        ScreenId::PlanetTransportDone(mode) => app.planet_transport.render_done(
            crate::screen::command_menu_label(app.command_return_menu),
            mode,
            app.planet
                .transport_status
                .as_deref()
                .unwrap_or("Transport order completed."),
        ),
        ScreenId::PlanetCommissionPicker => {
            let rows = app.planet_commission_picker_rows();
            if rows.is_empty() {
                app.open_planet_menu();
                return render(app);
            }
            app.planet_commission.render_picker(
                frame.geometry,
                &rows,
                app.planet.commission_picker_scroll_offset,
                app.planet.commission_index,
            )
        }
        ScreenId::PlanetCommissionMenu => app.planet_commission.render_menu(
            frame.geometry,
            &app.current_planet_commission_view()?,
            app.planet.commission_scroll_offset,
            app.planet.commission_cursor,
            &app.planet.commission_selected_slots,
            app.planet.commission_status.as_deref(),
        ),
        ScreenId::PlanetCommissionDraft => app.planet_commission.render_draft(
            frame.geometry,
            &app.current_planet_commission_draft_title()
                .unwrap_or_else(|_| "DRAFT COMMISSION FLEET:".to_string()),
            &app.planet.commission_draft_rows,
            app.planet.commission_draft_cursor,
            &app.planet.commission_draft_input,
            app.planet.commission_draft_status.as_deref(),
            app.planet.commission_draft_notice.as_deref(),
        ),
        ScreenId::PlanetCommissionResult => app.planet_commission.render_result(
            app.planet
                .commission_result_title
                .as_deref()
                .unwrap_or("COMMISSION SHIPS:"),
            app.planet
                .commission_result_notice
                .as_deref()
                .unwrap_or("Commission completed."),
        ),
        ScreenId::PlanetAutoCommissionReport => {
            if app.planet.auto_commission_report_rows.is_empty() {
                app.open_planet_menu();
                return render(app);
            }
            app.planet_commission.render_auto_commission_report(
                frame.geometry,
                &app.planet.auto_commission_report_rows,
                app.planet.auto_commission_report_revealed_rows,
            )
        }
        ScreenId::PlanetBuildMenu => app.planet_build.render_menu(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
            app.planet.build_status.as_deref(),
            app.expert_mode,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::PlanetBuild,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
            app.planet.build_abort_prompt_active,
        ),
        ScreenId::PlanetBuildList => app.planet_build.render_list(
            frame.geometry,
            &app.current_planet_build_view()?,
            &app.planet_build_list_rows(),
            app.planet.build_list_scroll_offset,
            app.planet.build_list_cursor,
            app.planet.build_list_confirming,
            app.planet.build_list_delete_qty_prompt_active,
            &app.planet.build_list_delete_qty_input,
            app.planet.build_list_delete_qty_status.as_deref(),
            app.planet.build_list_delete_qty_pending,
        ),
        ScreenId::PlanetBuildChange => app.planet_build.render_change(
            frame.geometry,
            &app.build_change_rows(),
            app.planet.build_change_scroll_offset,
            app.planet.build_change_cursor,
        ),
        ScreenId::PlanetBuildSpecify => app.planet_build.render_specify(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
            &app.planet.build_unit_input,
            app.planet.build_unit_status.as_deref(),
            app.planet.build_unit_notice.as_deref(),
        ),
        ScreenId::PlanetBuildQuantity => app.planet_build.render_quantity_prompt(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
            build_unit_spec_by_kind(
                app.planet
                    .build_selected_kind
                    .ok_or("planet build kind not selected")?,
            )
            .ok_or("planet build unit missing")?,
            app.current_planet_build_max_quantity()?,
            &app.planet.build_quantity_input,
            app.planet.build_quantity_status.as_deref(),
        ),
        ScreenId::PlanetListSortPrompt(mode) => app.planet_list.render_sort_prompt(
            &frame,
            mode,
            &app.planet_list_rows(mode, app.planet.list_sort),
            app.planet.list_sort,
            app.planet.list_sort_direction,
            app.planet.list_filter,
            app.planet.brief_scroll_offset,
            app.planet.brief_cursor,
            &app.planet.list_prompt_default_value,
            &app.planet.list_prompt_input,
            app.planet.list_prompt_status.as_deref(),
            app.planet.list_prompt_dismiss_message.as_deref(),
        ),
        ScreenId::PlanetListFilterPrompt(mode) => {
            app.planet_list.render_filter_prompt_with_filter_clause(
                &frame,
                mode,
                &app.planet_list_rows(mode, app.planet.list_sort),
                app.planet.list_sort,
                app.planet.list_sort_direction,
                app.planet.list_filter,
                app.planet.list_filter_clause.as_ref(),
                app.planet.brief_scroll_offset,
                app.planet.brief_cursor,
                app.planet.list_filter_prompt_mode,
                &app.planet.list_prompt_default_value,
                &app.planet.list_prompt_input,
                app.planet.list_prompt_status.as_deref(),
                app.planet.list_prompt_dismiss_message.as_deref(),
                app.planet
                    .list_filter_pending_column
                    .map(|column| column.code),
            )
        }
        ScreenId::PlanetList(mode, sort) => app.planet_list.render_brief_list_with_filter_clause(
            &frame,
            mode,
            &app.planet_list_rows(mode, sort),
            sort,
            app.planet.list_sort_direction,
            app.planet.list_filter,
            app.planet.list_filter_clause.as_ref(),
            app.planet.brief_scroll_offset,
            app.planet.brief_cursor,
            &app.planet.brief_input,
            app.planet.list_prompt_status.as_deref(),
            app.planet.auto_commission_prompt_active
                && mode == crate::screen::PlanetListMode::Brief,
            app.planet.build_abort_prompt_active && mode == crate::screen::PlanetListMode::Brief,
            if mode == crate::screen::PlanetListMode::Brief {
                list_transport_prompt_label.as_deref()
            } else {
                None
            },
            &app.planet.transport_prompt_default_value,
            &app.planet.transport_prompt_input,
            inline_list_transport.map(|(_, summary)| summary.as_str()),
            if mode == crate::screen::PlanetListMode::Brief && !scorch_planet_prompt_active {
                list_scorch_confirm_prompt
            } else {
                None
            },
            inline_list_scorch_summary.as_deref(),
        ),
        ScreenId::PlanetDatabaseList => app.planet_database.render_list_with_filter_clause(
            frame.geometry,
            &app.planet_database_rows(),
            app.planet.database_sort,
            app.planet.database_sort_direction,
            app.planet.database_filter,
            app.planet.database_filter_clause.as_ref(),
            app.planet.database_scroll_offset,
            app.planet.database_cursor,
            app.default_planet_prompt_coords(),
            &app.planet.database_input,
            app.planet.database_status.as_deref(),
            app.command_return_menu,
        ),
        ScreenId::PlanetDatabaseFilterPrompt | ScreenId::PlanetDatabaseSortPrompt => {
            app.planet_database.render_filter_prompt_with_filter_clause(
                frame.geometry,
                &app.planet_database_rows(),
                app.planet.database_sort,
                app.planet.database_sort_direction,
                app.planet.database_filter,
                app.planet.database_filter_clause.as_ref(),
                app.planet.database_scroll_offset,
                app.planet.database_cursor,
                app.planet.database_prompt_mode,
                &app.planet.database_prompt_default_value,
                &app.planet.database_input,
                app.planet.database_status.as_deref(),
                app.command_return_menu,
                app.planet.database_prompt_dismiss_message.as_deref(),
                app.planet.database_pending_column.map(|column| column.code),
            )
        }
        ScreenId::PlanetInfoDetail => app.planet_info.render_detail(
            &frame,
            app.planet
                .info_selected
                .ok_or("planet info detail not selected")?,
            app.command_return_menu,
        ),
        _ => unreachable!("planet views called for non-planet screen"),
    }
}
