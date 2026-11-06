use crate::app::state::App;
use crate::domains::planet::PlanetAction;

pub fn update(app: &mut App, action: PlanetAction) {
    match action {
        PlanetAction::OpenMenu => app.open_planet_menu(),
        PlanetAction::OpenAutoCommissionPrompt => app.open_planet_auto_commission_prompt(),
        PlanetAction::CloseAutoCommissionPrompt => app.close_planet_auto_commission_prompt(),
        PlanetAction::AdvanceAutoCommissionReport => app.advance_planet_auto_commission_report(),
        PlanetAction::ToggleBuildPlanetDefensive => {
            if let Err(err) = app.toggle_build_planet_defensive() {
                app.log_action_error("toggle_build_planet_defensive", err.as_ref());
                app.planet.build_status = Some(
                    "Unable to change defensive mode right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::AutoCommissionReportKey(key_code) => {
            app.answer_planet_auto_commission_merge(key_code)
        }
        PlanetAction::OpenScorchPrompt => app.open_planet_scorch_prompt(),
        PlanetAction::SubmitScorchPrompt => {
            if let Err(err) = app.submit_planet_scorch_prompt() {
                app.planet.scorch_prompt_status = Some(err);
            }
        }
        PlanetAction::AppendScorchPromptChar(ch) => app.append_planet_scorch_prompt_char(ch),
        PlanetAction::BackspaceScorchPromptInput => app.backspace_planet_scorch_prompt_input(),
        PlanetAction::CancelScorchPrompt => app.cancel_planet_scorch_prompt(),
        PlanetAction::OpenCommissionMenu => app.open_planet_commission_menu(),
        PlanetAction::OpenCommissionPlanet => app.open_planet_commission_planet(),
        PlanetAction::CloseCommissionPlanet => app.close_planet_commission_planet(),
        PlanetAction::MoveCommissionDraftRow(delta) => {
            if let Err(err) = app.move_planet_commission_draft_row(delta) {
                app.log_action_error("move_planet_commission_draft_row", err.as_ref());
                app.planet.commission_draft_status = Some(
                    "Unable to update this commission draft right now. Please try again."
                        .to_string(),
                );
            }
        }
        PlanetAction::AppendCommissionDraftChar(ch) => app.append_planet_commission_draft_char(ch),
        PlanetAction::BackspaceCommissionDraftInput => {
            app.backspace_planet_commission_draft_input()
        }
        PlanetAction::SubmitCommissionDraft => {
            if let Err(err) = app.submit_planet_commission_draft() {
                app.log_action_error("submit_planet_commission_draft", err.as_ref());
                app.planet.commission_draft_status = Some(
                    "Unable to save this commission draft right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::CloseCommissionDraft => app.close_planet_commission_draft(),
        PlanetAction::DismissCommissionResult(key_code) => {
            app.dismiss_planet_commission_result(key_code)
        }
        PlanetAction::ClearCommissionDismissKey => app.clear_planet_commission_dismiss_key(),
        PlanetAction::OpenTransportPrompt(mode) => app.open_planet_transport_prompt(mode),
        PlanetAction::SubmitTransportPrompt => app.submit_planet_transport_prompt(),
        PlanetAction::AppendTransportPromptChar(ch) => app.append_planet_transport_prompt_char(ch),
        PlanetAction::BackspaceTransportPromptInput => {
            app.backspace_planet_transport_prompt_input()
        }
        PlanetAction::CancelTransportPrompt => app.cancel_planet_transport_prompt(),
        PlanetAction::OpenBuildMenu => app.open_planet_build_menu(),
        PlanetAction::OpenCurrentBuildPlanetInfo => app.open_current_build_planet_info(),
        PlanetAction::OpenBuildList => app.open_planet_build_list(),
        PlanetAction::OpenBuildChange => app.open_planet_build_change(),
        PlanetAction::MoveBuildChange(delta) => app.move_planet_build_change_cursor(delta),
        PlanetAction::ConfirmBuildChange => app.confirm_planet_build_change(),
        PlanetAction::OpenBuildAbortPrompt => app.open_planet_build_abort_prompt(),
        PlanetAction::CloseBuildAbortPrompt => app.close_planet_build_abort_prompt(),
        PlanetAction::OpenBuildSpecify => app.open_planet_build_specify(),
        PlanetAction::OpenListFilterPrompt(mode) => app.open_planet_list_filter_prompt(mode),
        PlanetAction::DismissListFilterPromptNotice => {
            app.dismiss_planet_list_filter_prompt_notice()
        }
        PlanetAction::OpenListSortPrompt(mode) => app.open_planet_list_sort_prompt(mode),
        PlanetAction::SubmitListFilter(mode, filter) => app.submit_planet_list_filter(mode, filter),
        PlanetAction::SubmitListFilterPrompt(mode) => app.submit_planet_list_filter_prompt(mode),
        PlanetAction::SubmitListSort(mode, sort) => app.submit_planet_list_sort(mode, sort),
        PlanetAction::SubmitListSortPrompt(mode) => app.submit_planet_list_sort_prompt(mode),
        PlanetAction::CloseListFilterPrompt(mode) => app.close_planet_list_filter_prompt(mode),
        PlanetAction::CloseListSortPrompt(mode) => app.close_planet_list_sort_prompt(mode),
        PlanetAction::AppendListPromptChar(ch) => app.append_planet_list_prompt_char(ch),
        PlanetAction::BackspaceListPromptInput => app.backspace_planet_list_prompt_input(),
        PlanetAction::OpenTaxPrompt => app.open_planet_tax_prompt(),
        PlanetAction::CloseTaxPrompt => app.close_planet_tax_prompt(),
        PlanetAction::OpenDatabase => app.open_planet_database(),
        PlanetAction::OpenDatabaseFilterPrompt => app.open_planet_database_filter_prompt(),
        PlanetAction::DismissDatabaseFilterPromptNotice => {
            app.dismiss_planet_database_filter_prompt_notice()
        }
        PlanetAction::OpenDatabaseSortPrompt => app.open_planet_database_sort_prompt(),
        PlanetAction::SubmitDatabaseFilter(mode) => app.submit_planet_database_filter(mode),
        PlanetAction::SubmitDatabaseFilterPrompt => app.submit_planet_database_filter_prompt(),
        PlanetAction::SubmitDatabaseSort(mode) => app.submit_planet_database_sort(mode),
        PlanetAction::SubmitDatabaseSortPrompt => app.submit_planet_database_sort_prompt(),
        PlanetAction::ScrollBrief(delta) => app.scroll_planet_brief(delta),
        PlanetAction::MoveBrief(delta) => app.move_planet_brief_cursor(delta),
        PlanetAction::AppendBriefChar(ch) => app.append_planet_brief_char(ch),
        PlanetAction::BackspaceBriefInput => app.backspace_planet_brief_input(),
        PlanetAction::SubmitBriefInput => app.submit_planet_brief_input(),
        PlanetAction::ScrollBuildList(delta) => app.scroll_planet_build_list(delta),
        PlanetAction::MoveBuildList(delta) => app.move_planet_build_list_cursor(delta),
        PlanetAction::DeleteBuildSlotRequest => app.delete_planet_build_slot_request(),
        PlanetAction::AppendDeleteBuildQtyChar(ch) => app.append_delete_build_qty_char(ch),
        PlanetAction::BackspaceDeleteBuildQtyInput => app.backspace_delete_build_qty_input(),
        PlanetAction::SubmitDeleteBuildQty => {
            if let Err(err) = app.submit_delete_build_qty() {
                app.log_action_error("submit_delete_build_qty", err.as_ref());
                app.planet.build_list_delete_qty_status = Some(
                    "Unable to update this build order right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::ConfirmDeleteBuildSlot => {
            if let Err(err) = app.confirm_delete_planet_build_slot() {
                app.log_action_error("confirm_delete_planet_build_slot", err.as_ref());
                app.planet.build_list_delete_qty_status = Some(
                    "Unable to delete this build order right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::CancelDeleteBuildSlot => app.cancel_delete_planet_build_slot(),
        PlanetAction::MoveBuild(delta) => app.move_planet_build(delta),
        PlanetAction::MoveCommissionPlanet(delta) => app.move_planet_commission_planet(delta),
        PlanetAction::MoveCommissionRow(delta) => app.move_planet_commission_row(delta),
        PlanetAction::ToggleCommissionSelection => app.toggle_planet_commission_selection(),
        PlanetAction::CommissionStardockSelection => {
            if let Err(err) = app.commission_selected_stardock_row() {
                app.log_action_error("commission_selected_stardock_row", err.as_ref());
                app.planet.commission_status = Some(
                    "Unable to open that stardock selection right now. Please try again."
                        .to_string(),
                );
            }
        }
        PlanetAction::MoveTransportPlanet(delta) => app.move_planet_transport_planet(delta),
        PlanetAction::ConfirmTransportPlanet => app.confirm_planet_transport_planet(),
        PlanetAction::AppendTransportPlanetChar(ch) => app.append_planet_transport_planet_char(ch),
        PlanetAction::BackspaceTransportPlanetInput => {
            app.backspace_planet_transport_planet_input()
        }
        PlanetAction::SubmitTransportPlanet => app.submit_planet_transport_planet(),
        PlanetAction::MoveTransportFleet(delta) => app.move_planet_transport_fleet(delta),
        PlanetAction::ConfirmTransportFleet => app.confirm_planet_transport_fleet(),
        PlanetAction::AppendTransportQtyChar(ch) => app.append_planet_transport_qty_char(ch),
        PlanetAction::BackspaceTransportQty => app.backspace_planet_transport_qty(),
        PlanetAction::SubmitTransportQty => {
            if let Err(err) = app.submit_planet_transport_qty() {
                app.log_action_error("submit_planet_transport_qty", err.as_ref());
                app.planet.transport_status = Some(
                    "Unable to save this transport order right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::MoveDatabaseList(delta) => app.move_planet_database_list(delta),
        PlanetAction::PageDatabaseList(direction) => {
            let page = app.planet_database_visible_rows() as isize * direction as isize;
            app.move_planet_database_list_by(page);
        }
        PlanetAction::AppendDatabaseChar(ch) => app.append_planet_database_char(ch),
        PlanetAction::BackspaceDatabaseInput => app.backspace_planet_database_input(),
        PlanetAction::SubmitDatabaseLookup => app.submit_planet_database_lookup(),
        PlanetAction::AppendTaxChar(ch) => app.append_planet_tax_char(ch),
        PlanetAction::BackspaceTaxInput => app.backspace_planet_tax_input(),
        PlanetAction::SubmitTax => {
            if let Err(err) = app.submit_planet_tax() {
                app.log_action_error("submit_planet_tax", err.as_ref());
                app.planet.tax_notice =
                    Some("Unable to save this tax change right now. Please try again.".to_string());
            }
        }
        PlanetAction::AppendBuildUnitChar(ch) => app.append_planet_build_unit_char(ch),
        PlanetAction::BackspaceBuildUnitInput => app.backspace_planet_build_unit_input(),
        PlanetAction::SubmitBuildUnit => app.submit_planet_build_unit(),
        PlanetAction::AppendBuildQuantityChar(ch) => app.append_planet_build_quantity_char(ch),
        PlanetAction::BackspaceBuildQuantityInput => app.backspace_planet_build_quantity_input(),
        PlanetAction::SubmitBuildQuantity => {
            if let Err(err) = app.submit_planet_build_quantity() {
                app.log_action_error("submit_planet_build_quantity", err.as_ref());
                app.planet.build_quantity_status = Some(
                    "Unable to save this build order right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::ConfirmBuildAbort => {
            if let Err(err) = app.abort_current_planet_builds() {
                app.log_action_error("abort_current_planet_builds", err.as_ref());
                app.planet.build_status = Some(
                    "Unable to abort these build orders right now. Please try again.".to_string(),
                );
            }
        }
        PlanetAction::ConfirmAutoCommission => {
            if let Err(err) = app.confirm_planet_auto_commission() {
                app.log_action_error("confirm_planet_auto_commission", err.as_ref());
                app.show_command_menu_notice(
                    crate::screen::CommandMenu::Planet,
                    "Unable to auto-commission these stardock units right now. Please try again.",
                );
                app.planet.auto_commission_prompt_active = true;
            }
        }
        PlanetAction::OpenInfoPrompt(menu) => app.open_planet_info_prompt(menu),
        PlanetAction::CloseInfoPrompt => app.close_planet_info_prompt(),
        PlanetAction::AppendInfoChar(ch) => app.append_planet_info_char(ch),
        PlanetAction::BackspaceInfoInput => app.backspace_planet_info_input(),
        PlanetAction::SubmitInfoPrompt => app.submit_planet_info_prompt(),
    }
}
