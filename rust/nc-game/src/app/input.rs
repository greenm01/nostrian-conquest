use super::Action;
use super::state::App;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::startup::StartupAction;
use crate::domains::startup::state::FirstTimeOnboardingMode;
use crate::screen::{
    FIRST_TIME_INTRO_PAGE_COUNT, PlanetListMode, STARTUP_SPLASH_PAGE_COUNT, Screen, ScreenId,
};
use crate::startup::StartupPhase;

impl App {
    /// Screens that take free text, where `?` is a character and Ctrl+G
    /// opens help (Ctrl+H is Backspace on BBS terminals).
    fn free_text_entry_active(&self) -> bool {
        matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName
                | ScreenId::FirstTimeHomeworldName
                | ScreenId::ColonyWorldName
                | ScreenId::ComposeMessageSubject
                | ScreenId::ComposeMessageBody
        )
    }

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> Action {
        let key = self.normalize_navigation_hotkeys(key);
        if key.code == crossterm::event::KeyCode::Char('c')
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            return Action::Quit;
        }
        if self.popup_help.is_some() {
            if !key.modifiers.intersects(
                crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT,
            ) {
                return Action::DismissPopupHelp;
            }
            return Action::Noop;
        }
        if let Some(action) = self.handle_fleet_list_dismiss_latch(key) {
            return action;
        }
        if let Some(action) = self.handle_table_filter_prompt_dismiss_latch(key) {
            return action;
        }
        if self.free_text_entry_active() {
            if key.code == crossterm::event::KeyCode::Char('g')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                return Action::OpenPopupHelp;
            }
        } else if matches!(
            key,
            crossterm::event::KeyEvent {
                code: crossterm::event::KeyCode::Char('?'),
                modifiers: crossterm::event::KeyModifiers::NONE
                    | crossterm::event::KeyModifiers::SHIFT,
                ..
            }
        ) {
            return Action::OpenPopupHelp;
        }
        if self.quit_confirm_open {
            return self.handle_quit_confirm_key(key);
        }
        if let Some(action) = self.handle_planet_commission_dismiss_latch(key) {
            return action;
        }
        if self.inline_planet_transport_prompt_active_on_current_screen() {
            return self.handle_planet_transport_prompt_key(key);
        }
        if self.inline_planet_tax_active_on_current_screen() {
            return self.planet_tax.handle_inline_key(key);
        }
        if self.inline_planet_auto_commission_active_on_current_screen() {
            return self.handle_planet_auto_commission_prompt_key(key);
        }
        if self.inline_planet_scorch_prompt_active_on_current_screen() {
            return self.handle_planet_scorch_prompt_key(key);
        }
        if self.inline_planet_build_abort_active_on_current_screen() {
            return self.handle_planet_build_abort_prompt_key(key);
        }
        if self.inline_delete_reviewables_active_on_current_screen() {
            return self.handle_delete_reviewables_prompt_key(key);
        }
        if self.inline_planet_info_active_on_current_screen() {
            return self.handle_planet_info_prompt_key(key);
        }
        if self.inline_fleet_menu_prompt_active_on_current_screen() {
            return self.handle_fleet_menu_prompt_key(key);
        }
        if self.inline_starbase_move_prompt_active_on_current_screen() {
            return self.handle_starbase_move_prompt_key(key);
        }
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Splash)
                if self.startup_state.splash_page > 0
                    && self.startup_state.splash_page + 1 < STARTUP_SPLASH_PAGE_COUNT =>
            {
                Action::Startup(StartupAction::Advance)
            }
            ScreenId::Startup(phase) => self.handle_startup_key(phase, key),
            ScreenId::FirstTimeMenu => self
                .first_time_menu
                .handle_key_for_mode(key, self.door_mode),
            ScreenId::FirstTimeEmpires => self.first_time_empires.handle_key(key),
            ScreenId::FirstTimeReservedPrompt => match key.code {
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('y')
                | crossterm::event::KeyCode::Char('Y') => {
                    Action::Startup(StartupAction::AcceptFirstTimePrompt)
                }
                crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => {
                    Action::Startup(StartupAction::RejectFirstTimePrompt)
                }
                _ => Action::Noop,
            },
            ScreenId::FirstTimePreloadedRenamePrompt => match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                    Action::Startup(StartupAction::AcceptFirstTimePrompt)
                }
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => {
                    Action::Startup(StartupAction::RejectFirstTimePrompt)
                }
                _ => Action::Noop,
            },
            ScreenId::FirstTimeIntro
                if self.startup_state.first_time_intro_page + 1 < FIRST_TIME_INTRO_PAGE_COUNT =>
            {
                Action::Startup(StartupAction::Advance)
            }
            ScreenId::FirstTimeIntro => self.first_time_intro.handle_key(key),
            ScreenId::ThemePicker => self.theme_picker.handle_key(key),
            ScreenId::TerminalNotice => Action::Quit,
            ScreenId::FleetMessage => {
                Action::Fleet(crate::domains::fleet::FleetAction::DismissMessage)
            }
            ScreenId::FirstTimeJoinEmpireName | ScreenId::FirstTimeHomeworldName => {
                match key.code {
                    crossterm::event::KeyCode::Char(ch) => {
                        Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
                    }
                    crossterm::event::KeyCode::Backspace => {
                        Action::Startup(StartupAction::BackspaceFirstTimeInput)
                    }
                    crossterm::event::KeyCode::Enter => {
                        Action::Startup(StartupAction::SubmitFirstTimeInput)
                    }
                    crossterm::event::KeyCode::Esc => {
                        if self.startup_state.first_time_rename_preloaded_empire {
                            Action::Startup(StartupAction::RejectFirstTimePrompt)
                        } else if self.startup_state.first_time_onboarding_mode
                            == FirstTimeOnboardingMode::BbsReserved
                        {
                            Action::Startup(StartupAction::RejectFirstTimePrompt)
                        } else {
                            Action::Startup(StartupAction::OpenFirstTimeMenu)
                        }
                    }
                    _ => Action::Noop,
                }
            }
            ScreenId::ColonyWorldName => match key.code {
                crossterm::event::KeyCode::Char(ch) => {
                    Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
                }
                crossterm::event::KeyCode::Backspace => {
                    Action::Startup(StartupAction::BackspaceFirstTimeInput)
                }
                crossterm::event::KeyCode::Enter => {
                    Action::Startup(StartupAction::SubmitFirstTimeInput)
                }
                crossterm::event::KeyCode::Esc => {
                    Action::Startup(StartupAction::RejectFirstTimePrompt)
                }
                _ => Action::Noop,
            },
            ScreenId::FirstTimeJoinEmpireConfirm => {
                if self.startup_state.first_time_rename_preloaded_empire {
                    match key.code {
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::AcceptFirstTimePrompt)
                        }
                        crossterm::event::KeyCode::Enter
                        | crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N')
                        | crossterm::event::KeyCode::Esc => {
                            Action::Startup(StartupAction::RejectFirstTimePrompt)
                        }
                        _ => Action::Noop,
                    }
                } else {
                    match key.code {
                        crossterm::event::KeyCode::Enter
                        | crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::AcceptFirstTimePrompt)
                        }
                        crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N')
                        | crossterm::event::KeyCode::Esc => {
                            Action::Startup(StartupAction::RejectFirstTimePrompt)
                        }
                        _ => Action::Noop,
                    }
                }
            }
            ScreenId::FirstTimeJoinSummary | ScreenId::FirstTimeJoinNoPending => {
                Action::Startup(StartupAction::AcceptFirstTimePrompt)
            }
            ScreenId::FirstTimeHomeworldConfirm => match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                    Action::Startup(StartupAction::AcceptFirstTimePrompt)
                }
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => {
                    Action::Startup(StartupAction::RejectFirstTimePrompt)
                }
                _ => Action::Noop,
            },
            ScreenId::ColonyWorldConfirm => match key.code {
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('y')
                | crossterm::event::KeyCode::Char('Y') => {
                    Action::Startup(StartupAction::AcceptFirstTimePrompt)
                }
                crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => {
                    Action::Startup(StartupAction::RejectFirstTimePrompt)
                }
                _ => Action::Noop,
            },
            ScreenId::MainMenu => self.main_menu.handle_key_for_mode(key, self.door_mode),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::StarbaseMenu => self.starbase_menu.handle_key(key),
            ScreenId::StarbaseList => self.starbase_list.handle_key(key),
            ScreenId::StarbaseReviewSelect => self.handle_starbase_review_select_key(key),
            ScreenId::StarbaseReview => Action::Starbase(StarbaseAction::OpenReviewSelect),
            ScreenId::FleetMenu => {
                if self.inline_fleet_menu_prompt_active_on_current_screen() {
                    self.handle_fleet_menu_prompt_key(key)
                } else {
                    self.fleet_menu.handle_key(key)
                }
            }
            ScreenId::FleetList => {
                if self.inline_fleet_menu_prompt_active_on_current_screen() {
                    self.handle_fleet_menu_prompt_key(key)
                } else {
                    self.fleet_list.handle_key(key)
                }
            }
            ScreenId::FleetListFilterPrompt => self
                .fleet_list
                .handle_filter_prompt_key(key, self.fleet.list_filter_prompt_mode),
            ScreenId::FleetListSortPrompt => self.fleet_list.handle_sort_prompt_key(key),
            ScreenId::FleetReview => self.fleet_review.handle_key(key),
            ScreenId::FleetOrder => self.handle_fleet_order_key(key),
            ScreenId::FleetGroupOrder => self.handle_fleet_group_order_key(key),
            ScreenId::FleetMissionPicker => self.handle_fleet_mission_picker_key(key),
            ScreenId::FleetTransfer => self.handle_fleet_transfer_key(key),
            ScreenId::FleetDetach => self.handle_fleet_detach_key(key),
            ScreenId::FleetEta => self.handle_fleet_eta_key(key),
            ScreenId::PlanetMenu => self.planet_menu.handle_key(key),
            ScreenId::PlanetCommissionPicker => self.planet_commission.handle_picker_key(key),
            ScreenId::PlanetCommissionMenu => self.planet_commission.handle_detail_key(key),
            ScreenId::PlanetCommissionDraft => self.planet_commission.handle_draft_key(key),
            ScreenId::PlanetCommissionResult => self.planet_commission.handle_result_key(key),
            ScreenId::PlanetAutoCommissionReport => self
                .planet_commission
                .handle_auto_commission_report_key(key),
            ScreenId::PlanetTransportPlanetSelect(_) => {
                self.planet_transport.handle_planet_key(key)
            }
            ScreenId::PlanetTransportFleetSelect(_) => self.planet_transport.handle_fleet_key(key),
            ScreenId::PlanetTransportQuantityPrompt(_) => {
                self.planet_transport.handle_quantity_key(key)
            }
            ScreenId::PlanetTransportDone(_) => Action::Planet(PlanetAction::OpenMenu),
            ScreenId::PlanetBuildMenu => self.planet_build.handle_menu_key(key),
            ScreenId::PlanetBuildList => self.planet_build.handle_list_key(
                key,
                self.planet.build_list_confirming,
                self.planet.build_list_delete_qty_prompt_active,
            ),
            ScreenId::PlanetBuildChange => self.planet_build.handle_change_key(key),
            ScreenId::PlanetBuildSpecify => self.planet_build.handle_specify_key(key),
            ScreenId::PlanetBuildQuantity => self.planet_build.handle_quantity_key(key),
            ScreenId::PlanetListSortPrompt(PlanetListMode::Stub(_)) => {
                Action::Planet(PlanetAction::OpenMenu)
            }
            ScreenId::PlanetListFilterPrompt(mode) => self.planet_list.handle_filter_prompt_key(
                key,
                mode,
                self.planet.list_filter_prompt_mode,
            ),
            ScreenId::PlanetListSortPrompt(mode) => {
                self.planet_list.handle_sort_prompt_key(key, mode)
            }
            ScreenId::PlanetList(mode, _) => self.planet_list.handle_brief_key(key, mode),
            ScreenId::Starmap if self.starmap_state.capture_complete => {
                self.starmap.handle_complete_key(key)
            }
            ScreenId::Starmap if self.starmap_state.dump_active => {
                self.starmap.handle_dump_key(key)
            }
            ScreenId::Starmap => self.starmap.handle_prompt_key(key),
            ScreenId::PartialStarmapView => self.partial_starmap.handle_view_key(key),
            ScreenId::PlanetDatabaseList => self.planet_database.handle_list_key(key),
            ScreenId::PlanetDatabaseFilterPrompt => self
                .planet_database
                .handle_filter_prompt_key_for_mode(key, self.planet.database_prompt_mode),
            ScreenId::PlanetDatabaseSortPrompt => self
                .planet_database
                .handle_filter_prompt_key_for_mode(key, self.planet.database_prompt_mode),
            ScreenId::PlanetInfoDetail => self.planet_info.handle_detail_key(key),
            ScreenId::Enemies => self.enemies.handle_key(key),
            ScreenId::ComposeMessageRecipient => self.message_compose.handle_recipient_key(key),
            ScreenId::ComposeMessageSubject => self.message_compose.handle_subject_key(key),
            ScreenId::ComposeMessageBody => self.message_compose.handle_body_key(key),
            ScreenId::ComposeMessageOutbox => self
                .message_compose
                .handle_outbox_key(key, self.messaging.compose_outbox_mode),
            ScreenId::ComposeMessageDiscardConfirm => {
                self.message_compose.handle_discard_confirm_key(key)
            }
            ScreenId::ComposeMessageSendConfirm => {
                self.message_compose.handle_send_confirm_key(key)
            }
            ScreenId::ComposeMessageSent => self.message_compose.handle_sent_key(key),
            ScreenId::EmpireStatus => self.empire_status.handle_key(key),
            ScreenId::EmpireProfile => self.empire_profile.handle_key(key),
            ScreenId::Rankings(_) => self.rankings.handle_key(key),
            ScreenId::Reports => self.handle_reports_key(key),
        }
    }

    fn normalize_navigation_hotkeys(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crossterm::event::KeyEvent {
        match (key.code, key.modifiers) {
            (
                crossterm::event::KeyCode::Char('u') | crossterm::event::KeyCode::Char('U'),
                modifiers,
            ) if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::PageUp,
                    crossterm::event::KeyModifiers::NONE,
                )
            }
            (
                crossterm::event::KeyCode::Char('d') | crossterm::event::KeyCode::Char('D'),
                modifiers,
            ) if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::PageDown,
                    crossterm::event::KeyModifiers::NONE,
                )
            }
            _ => key,
        }
    }

    fn handle_planet_commission_dismiss_latch(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> Option<Action> {
        let latched = self.planet.commission_result_dismiss_key?;
        let commission_screen = matches!(
            self.current_screen,
            ScreenId::PlanetCommissionPicker
                | ScreenId::PlanetCommissionMenu
                | ScreenId::PlanetCommissionDraft
        );
        if !commission_screen || key.code != latched {
            return None;
        }
        Some(Action::Planet(PlanetAction::ClearCommissionDismissKey))
    }

    fn handle_fleet_list_dismiss_latch(&self, key: crossterm::event::KeyEvent) -> Option<Action> {
        if self.current_screen != ScreenId::FleetList || self.fleet.list_dismiss_message.is_none() {
            return None;
        }
        match key.code {
            crossterm::event::KeyCode::Modifier(_)
            | crossterm::event::KeyCode::CapsLock
            | crossterm::event::KeyCode::NumLock
            | crossterm::event::KeyCode::ScrollLock
            | crossterm::event::KeyCode::Null => None,
            _ => Some(Action::Fleet(
                crate::domains::fleet::FleetAction::DismissMessage,
            )),
        }
    }

    fn handle_table_filter_prompt_dismiss_latch(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> Option<Action> {
        let action = match self.current_screen {
            ScreenId::FleetListFilterPrompt
                if self.fleet.list_filter_prompt_dismiss_message.is_some() =>
            {
                Some(Action::Fleet(
                    crate::domains::fleet::FleetAction::DismissListFilterPromptNotice,
                ))
            }
            ScreenId::PlanetListFilterPrompt(_)
                if self.planet.list_prompt_dismiss_message.is_some() =>
            {
                Some(Action::Planet(
                    crate::domains::planet::PlanetAction::DismissListFilterPromptNotice,
                ))
            }
            ScreenId::PlanetDatabaseFilterPrompt
                if self.planet.database_prompt_dismiss_message.is_some() =>
            {
                Some(Action::Planet(
                    crate::domains::planet::PlanetAction::DismissDatabaseFilterPromptNotice,
                ))
            }
            _ => None,
        }?;
        match key.code {
            crossterm::event::KeyCode::Modifier(_)
            | crossterm::event::KeyCode::CapsLock
            | crossterm::event::KeyCode::NumLock
            | crossterm::event::KeyCode::ScrollLock
            | crossterm::event::KeyCode::Null => None,
            _ => Some(action),
        }
    }

    pub(crate) fn active_navigation_guards(&self) -> Vec<&'static str> {
        let mut guards = Vec::new();
        if self.popup_help.is_some() {
            guards.push("popup_help");
        }
        if self.quit_confirm_open {
            guards.push("quit_confirm");
        }
        if self.planet.commission_result_dismiss_key.is_some() {
            guards.push("commission_result_dismiss_latch");
        }
        if self.fleet.list_dismiss_message.is_some() {
            guards.push("fleet_list_dismiss_latch");
        }
        if self.inline_planet_transport_prompt_active_on_current_screen() {
            guards.push("planet_transport_prompt");
        }
        if self.inline_planet_tax_active_on_current_screen() {
            guards.push("planet_tax_prompt");
        }
        if self.inline_planet_auto_commission_active_on_current_screen() {
            guards.push("planet_auto_commission_prompt");
        }
        if self.inline_planet_scorch_prompt_active_on_current_screen() {
            guards.push("planet_scorch_prompt");
        }
        if self.inline_planet_build_abort_active_on_current_screen() {
            guards.push("planet_build_abort_prompt");
        }
        if self.inline_delete_reviewables_active_on_current_screen() {
            guards.push("delete_reviewables_prompt");
        }
        if self.inline_planet_info_active_on_current_screen() {
            guards.push("planet_info_prompt");
        }
        if self.inline_fleet_menu_prompt_active_on_current_screen() {
            guards.push("fleet_menu_prompt");
        }
        if self.inline_starbase_move_prompt_active_on_current_screen() {
            guards.push("starbase_move_prompt");
        }
        guards
    }
}
