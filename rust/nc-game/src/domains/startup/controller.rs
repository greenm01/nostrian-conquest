use crate::app::action::Action;
use crate::app::state::App;
use crate::domains::startup::StartupAction;
use crate::domains::startup::screens::startup::{
    block_review_rows, completed_block_transcript_rows,
};
use crate::domains::startup::state::FirstTimeOnboardingMode;
use crate::model::{ClassicLoginState, MainMenuSummary};
use crate::reports::{ReportsPreview, has_visible_runtime_messages};
use crate::screen::{
    CommandMenu, FIRST_TIME_INTRO_PAGE_COUNT, STARTUP_SPLASH_PAGE_COUNT, ScreenId,
    StartupReviewMode,
};
use crate::startup::{StartupPhase, StartupSummary};
use nc_data::{
    PlayerAccessMode, TerminalOutcome, delete_report_block_for_viewer, layout_classic_results,
};
use nc_session::onboarding::first_time_onboarding_mode as shared_first_time_onboarding_mode;

impl App {
    fn record_returning_player_participation_once(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.player.classic_login_state != ClassicLoginState::ReturningPlayer {
            return Ok(());
        }
        if !matches!(self.player_access_mode, PlayerAccessMode::Normal) {
            return Ok(());
        }
        let player_idx = self.player.record_index_1_based.saturating_sub(1);
        let current_year = self.game_data.conquest.game_year();
        let Some(activity_state) = self.player_activity_states.get(player_idx).copied() else {
            return Ok(());
        };
        let last_run_year = self
            .game_data
            .player
            .records
            .get(player_idx)
            .map(|player| player.last_run_year_raw())
            .unwrap_or(0);
        if last_run_year == current_year
            && activity_state.last_participation_year == current_year
            && !activity_state.inactivity_autopilot_pending_clear
        {
            return Ok(());
        }
        nc_data::record_interactive_participation(
            &mut self.game_data,
            self.player.record_index_1_based,
            &mut self.player_activity_states,
        );
        if matches!(self.player_access_mode, PlayerAccessMode::Normal) {
            self.save_game_data()?;
        } else {
            self.save_terminal_access_state()?;
        }
        Ok(())
    }

    pub fn enter_unbound_bbs_first_time_mode(&mut self) {
        self.startup_state.unbound_bbs_caller = true;
        self.startup_state.fixed_player_launch = false;
        self.startup_state.first_time_onboarding_mode = FirstTimeOnboardingMode::Generic;
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_input.clear();
        self.current_screen = ScreenId::FirstTimeMenu;
    }

    fn has_bbs_reserved_seat(&self) -> bool {
        self.door_mode && self.startup_state.reserved_seat_alias.is_some()
    }

    fn first_time_onboarding_mode(&self) -> FirstTimeOnboardingMode {
        shared_first_time_onboarding_mode(self.has_bbs_reserved_seat())
    }

    fn is_bbs_reserved_first_time_login(&self) -> bool {
        self.player.classic_login_state == ClassicLoginState::FirstTimeMenu
            && self.has_bbs_reserved_seat()
    }

    fn first_time_join_uses_reserved_prompt(&self) -> bool {
        self.startup_state.first_time_onboarding_mode == FirstTimeOnboardingMode::BbsReserved
    }

    fn theme_picker_visible_rows(&self) -> usize {
        crate::domains::startup::screens::theme_picker::theme_picker_visible_rows(
            self.screen_geometry,
        )
    }

    fn startup_review_visible_lines(&self) -> usize {
        crate::domains::startup::screens::startup::startup_review_visible_lines(
            self.screen_geometry,
        )
    }

    fn startup_review_row_count(&self, is_results: bool) -> usize {
        if is_results {
            self.startup
                .results_block_row_count(self.startup_state.results_block)
        } else {
            self.startup
                .messages_block_row_count(self.startup_state.messages_block)
        }
    }

    fn startup_review_scroll_offset(&self, is_results: bool) -> usize {
        if is_results {
            self.startup_state.results_page
        } else {
            self.startup_state.messages_page
        }
    }

    fn startup_review_max_scroll_offset(&self, is_results: bool) -> usize {
        self.startup_review_row_count(is_results)
            .saturating_sub(self.startup_review_visible_lines())
    }

    fn startup_review_is_at_end(&self, is_results: bool) -> bool {
        self.startup_review_scroll_offset(is_results)
            >= self.startup_review_max_scroll_offset(is_results)
    }

    fn startup_review_nonstop(&self, is_results: bool) -> bool {
        if is_results {
            self.startup_state.results_nonstop
        } else {
            self.startup_state.messages_nonstop
        }
    }

    fn delete_current_startup_review_block(
        &mut self,
        is_results: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.append_current_startup_review_block_to_history(is_results, false);
        if is_results {
            let block_idx = self.startup_state.results_block;
            let row_idx = self
                .startup
                .result_blocks()
                .get(block_idx)
                .and_then(|block| block.runtime_report_index);
            if let Some(row_idx) = row_idx {
                delete_report_block_for_viewer(
                    &mut self.report_block_rows,
                    &self.game_data.player.records,
                    row_idx,
                    self.player.record_index_1_based as u8,
                );
            }
            self.sync_player_review_flags();
            if matches!(self.player_access_mode, PlayerAccessMode::Normal) {
                self.save_game_data()?;
            } else {
                self.save_terminal_access_state()?;
            }
            self.refresh_review_context()?;
            self.startup_state.results_deleted_any = true;
            self.startup_state.results_page = 0;
            if self.startup_state.results_block < self.startup.result_block_count() {
                self.startup_state.results_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.results_mode = StartupReviewMode::EndStatus;
            }
        } else {
            let block_idx = self.startup_state.messages_block;
            let mail_index = self
                .startup
                .message_blocks()
                .get(block_idx)
                .and_then(|block| block.runtime_mail_index);
            if let Some(mail) = mail_index.and_then(|idx| self.queued_mail.get_mut(idx)) {
                mail.mark_deleted_by_recipient();
            }
            self.sync_player_review_flags();
            if matches!(self.player_access_mode, PlayerAccessMode::Normal) {
                self.save_game_data()?;
            } else {
                self.save_terminal_access_state()?;
            }
            self.refresh_review_context()?;
            self.startup_state.messages_deleted_any = true;
            self.startup_state.messages_page = 0;
            if self.startup_state.messages_block < self.startup.message_block_count() {
                self.startup_state.messages_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.messages_mode = StartupReviewMode::EndStatus;
            }
        }
        Ok(())
    }

    fn skip_current_startup_review_block(&mut self, is_results: bool) {
        self.append_current_startup_review_block_to_history(is_results, false);
        if is_results {
            let next_block = self.startup_state.results_block + 1;
            self.startup_state.results_block = next_block;
            self.startup_state.results_page = 0;
            if next_block < self.startup.result_block_count() {
                self.startup_state.results_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.results_mode = StartupReviewMode::EndStatus;
            }
        } else {
            let next_block = self.startup_state.messages_block + 1;
            self.startup_state.messages_block = next_block;
            self.startup_state.messages_page = 0;
            if next_block < self.startup.message_block_count() {
                self.startup_state.messages_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.messages_mode = StartupReviewMode::EndStatus;
            }
        }
    }

    pub fn scroll_startup_review(&mut self, delta: isize) {
        let is_results = match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => true,
            ScreenId::Startup(StartupPhase::Messages) => false,
            _ => return,
        };
        let mode = if is_results {
            self.startup_state.results_mode
        } else {
            self.startup_state.messages_mode
        };
        if mode != StartupReviewMode::ItemBody {
            return;
        }
        if self.startup_review_is_at_end(is_results) && !self.startup_review_nonstop(is_results) {
            return;
        }

        let current = self.startup_review_scroll_offset(is_results) as isize;
        let max = self.startup_review_max_scroll_offset(is_results) as isize;
        let next = (current + delta).clamp(0, max) as usize;
        self.set_startup_review_page(is_results, next);
    }

    pub fn advance_startup(&mut self) {
        if self.current_screen == ScreenId::FirstTimeIntro {
            if self.startup_state.first_time_intro_page + 1 < FIRST_TIME_INTRO_PAGE_COUNT {
                self.startup_state.first_time_intro_page += 1;
            } else {
                self.current_screen = ScreenId::FirstTimeMenu;
            }
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Splash)
            && self.startup_state.splash_page + 1 < STARTUP_SPLASH_PAGE_COUNT
        {
            self.startup_state.splash_page += 1;
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Splash) {
            let next = self.startup_sequence.advance();
            self.current_screen = self.startup_target_screen(next);
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Intro)
            && self.startup_state.intro_page + 1 < crate::screen::STARTUP_INTRO_PAGE_COUNT
        {
            self.startup_state.intro_page += 1;
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Results) {
            self.advance_startup_review_phase(true);
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Messages) {
            self.advance_startup_review_phase(false);
            return;
        }
        self.reset_startup_review_cursors_for_phase_exit();
        let next = self.startup_sequence.advance();
        self.current_screen = self.startup_target_screen(next);
    }

    fn advance_startup_review_phase(&mut self, is_results: bool) {
        let mode = if is_results {
            self.startup_state.results_mode
        } else {
            self.startup_state.messages_mode
        };
        let block = if is_results {
            self.startup_state.results_block
        } else {
            self.startup_state.messages_block
        };
        let page = if is_results {
            self.startup_state.results_page
        } else {
            self.startup_state.messages_page
        };
        let nonstop = if is_results {
            self.startup_state.results_nonstop
        } else {
            self.startup_state.messages_nonstop
        };
        let block_count = if is_results {
            self.startup.result_block_count()
        } else {
            self.startup.message_block_count()
        };
        let max_scroll = self.startup_review_max_scroll_offset(is_results);

        match mode {
            StartupReviewMode::ViewPrompt => {
                if block_count == 0 {
                    self.advance_startup_phase(is_results);
                } else {
                    self.set_startup_review_mode(is_results, StartupReviewMode::ItemBody);
                    self.set_startup_review_page(is_results, 0);
                }
            }
            StartupReviewMode::ItemBody => {
                if page < max_scroll {
                    self.set_startup_review_page(is_results, page + 1);
                } else if nonstop {
                    self.append_current_startup_review_block_to_history(is_results, false);
                    let next_block = block + 1;
                    if next_block < block_count {
                        self.set_startup_review_block(is_results, next_block);
                        self.set_startup_review_page(is_results, 0);
                    } else {
                        self.set_startup_review_mode(is_results, StartupReviewMode::EndStatus);
                    }
                } else {
                    self.skip_current_startup_review_block(is_results);
                }
            }
            StartupReviewMode::DeletePrompt => {
                let next_block = block + 1;
                self.set_startup_review_block(is_results, next_block);
                self.set_startup_review_page(is_results, 0);
                if next_block < block_count {
                    self.set_startup_review_mode(is_results, StartupReviewMode::ContinuePrompt);
                } else {
                    self.set_startup_review_mode(is_results, StartupReviewMode::EndStatus);
                }
            }
            StartupReviewMode::ContinuePrompt => {
                self.set_startup_review_mode(is_results, StartupReviewMode::ItemBody);
                self.set_startup_review_page(is_results, 0);
            }
            StartupReviewMode::EndStatus => {
                self.advance_startup_phase(is_results);
            }
        }
    }

    fn advance_startup_phase(&mut self, is_results: bool) {
        if is_results {
            self.startup_state.results_block = 0;
            self.startup_state.results_page = 0;
            self.startup_state.results_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.results_nonstop = false;
            self.startup_state.results_review_history_rows.clear();
        } else {
            self.startup_state.messages_block = 0;
            self.startup_state.messages_page = 0;
            self.startup_state.messages_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.messages_nonstop = false;
            self.startup_state.messages_review_history_rows.clear();
        }
        let next = self.startup_sequence.advance();
        self.current_screen = self.startup_target_screen(next);
    }

    fn set_startup_review_mode(&mut self, is_results: bool, mode: StartupReviewMode) {
        if is_results {
            self.startup_state.results_mode = mode;
        } else {
            self.startup_state.messages_mode = mode;
        }
    }

    fn set_startup_review_block(&mut self, is_results: bool, block: usize) {
        if is_results {
            self.startup_state.results_block = block;
        } else {
            self.startup_state.messages_block = block;
        }
    }

    fn set_startup_review_page(&mut self, is_results: bool, page: usize) {
        if is_results {
            self.startup_state.results_page = page;
        } else {
            self.startup_state.messages_page = page;
        }
    }

    fn append_current_startup_review_block_to_history(
        &mut self,
        is_results: bool,
        include_continue_prompt: bool,
    ) {
        let (
            block_idx,
            blocks,
            empty_notice,
            singular,
            plural,
            section_label,
            game_year,
            history_rows,
        ) = if is_results {
            (
                self.startup_state.results_block,
                self.startup.result_blocks(),
                "Reports are marked pending, but no review text is available yet.",
                "report",
                "reports",
                "Reports",
                self.game_data.conquest.game_year(),
                &mut self.startup_state.results_review_history_rows,
            )
        } else {
            (
                self.startup_state.messages_block,
                self.startup.message_blocks(),
                "Messages are marked pending, but no review text is available yet.",
                "message",
                "messages",
                "Messages",
                self.game_data.conquest.game_year(),
                &mut self.startup_state.messages_review_history_rows,
            )
        };

        let lines = blocks
            .get(block_idx)
            .map(|block| block.lines.as_slice())
            .unwrap_or(&[]);
        let block_rows = block_review_rows(lines, empty_notice);
        let completed_rows = completed_block_transcript_rows(
            singular,
            plural,
            block_rows,
            history_rows.is_empty(),
            section_label,
            game_year,
            include_continue_prompt,
        );
        history_rows.extend(completed_rows);
    }

    pub fn skip_startup_intro(&mut self) {
        let next = self.startup_sequence.skip_intro();
        self.current_screen = self.startup_target_screen(next);
    }

    pub fn startup_accept_default(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => match self.startup_state.results_mode {
                StartupReviewMode::ViewPrompt => {
                    self.startup_state.results_mode = StartupReviewMode::ItemBody;
                    self.startup_state.results_page = 0;
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(true)
                        && !self.startup_review_nonstop(true) =>
                {
                    self.delete_current_startup_review_block(true)?;
                }
                StartupReviewMode::DeletePrompt => {
                    self.delete_current_startup_review_block(true)?;
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.results_mode = StartupReviewMode::ItemBody;
                    self.startup_state.results_page = 0;
                }
                _ => {}
            },
            ScreenId::Startup(StartupPhase::Messages) => match self.startup_state.messages_mode {
                StartupReviewMode::ViewPrompt => {
                    self.startup_state.messages_mode = StartupReviewMode::ItemBody;
                    self.startup_state.messages_page = 0;
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(false)
                        && !self.startup_review_nonstop(false) =>
                {
                    self.delete_current_startup_review_block(false)?;
                }
                StartupReviewMode::DeletePrompt => {
                    self.delete_current_startup_review_block(false)?;
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.messages_mode = StartupReviewMode::ItemBody;
                    self.startup_state.messages_page = 0;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn startup_reject_choice(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => match self.startup_state.results_mode {
                StartupReviewMode::ViewPrompt => {
                    self.advance_startup_phase(true);
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(true)
                        && !self.startup_review_nonstop(true) =>
                {
                    self.skip_current_startup_review_block(true);
                }
                StartupReviewMode::DeletePrompt => {
                    self.skip_current_startup_review_block(true);
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.results_mode = StartupReviewMode::EndStatus;
                }
                _ => {}
            },
            ScreenId::Startup(StartupPhase::Messages) => match self.startup_state.messages_mode {
                StartupReviewMode::ViewPrompt => {
                    self.advance_startup_phase(false);
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(false)
                        && !self.startup_review_nonstop(false) =>
                {
                    self.skip_current_startup_review_block(false);
                }
                StartupReviewMode::DeletePrompt => {
                    self.skip_current_startup_review_block(false);
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.messages_mode = StartupReviewMode::EndStatus;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn startup_enable_nonstop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => match self.startup_state.results_mode {
                StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                    self.startup_state.results_nonstop = true;
                    self.startup_state.results_mode = StartupReviewMode::ItemBody;
                    self.startup_state.results_page = 0;
                }
                _ => {}
            },
            ScreenId::Startup(StartupPhase::Messages) => match self.startup_state.messages_mode {
                StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                    self.startup_state.messages_nonstop = true;
                    self.startup_state.messages_mode = StartupReviewMode::ItemBody;
                    self.startup_state.messages_page = 0;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn open_first_time_menu(&mut self) {
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_input.clear();
        self.startup_state.first_time_onboarding_mode = FirstTimeOnboardingMode::Generic;
        self.current_screen = ScreenId::FirstTimeMenu;
    }

    pub fn open_first_time_empires(&mut self) {
        self.startup_state.first_time_status = None;
        self.current_screen = ScreenId::FirstTimeEmpires;
    }

    pub fn open_first_time_intro(&mut self) {
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_intro_page = 0;
        self.current_screen = ScreenId::FirstTimeIntro;
    }

    pub fn open_theme_picker(&mut self) {
        if self.door_mode {
            match self.current_screen {
                ScreenId::FirstTimeMenu => {
                    self.startup_state.first_time_status =
                        Some("Theme picker unavailable in door mode.".to_string());
                }
                _ => {
                    self.show_command_menu_notice(
                        CommandMenu::Main,
                        "Theme picker unavailable in door mode.".to_string(),
                    );
                }
            }
            return;
        }
        let return_screen = match self.current_screen {
            ScreenId::FirstTimeMenu => ScreenId::FirstTimeMenu,
            _ => ScreenId::MainMenu,
        };
        match crate::theme::discover_theme_entries(&self.game_dir) {
            Ok(rows) => {
                let raw_current = crate::theme::current_theme_key();
                let default_key = raw_current
                    .clone()
                    .unwrap_or_else(|| "tokyo_night".to_string());
                let discovered_keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
                tracing::debug!(
                    raw_current_theme_key = ?raw_current,
                    resolved_default_key = %default_key,
                    discovered_count = discovered_keys.len(),
                    discovered_keys = ?discovered_keys,
                    "open_theme_picker: resolving initial cursor"
                );
                self.startup_state.theme_picker_rows = rows;
                let cursor = self.theme_picker_cursor_for_key(&default_key);
                let cursor_key = self
                    .startup_state
                    .theme_picker_rows
                    .get(cursor)
                    .map(|r| r.key.as_str())
                    .unwrap_or("<none>");
                tracing::debug!(
                    cursor = cursor,
                    cursor_key = %cursor_key,
                    "open_theme_picker: cursor set"
                );
                self.startup_state.theme_picker_cursor = cursor;
                self.startup_state.theme_picker_scroll_offset = 0;
                self.startup_state.theme_picker_input.clear();
                let visible_rows = self.theme_picker_visible_rows();
                crate::app::helpers::sync_scroll_to_cursor(
                    &mut self.startup_state.theme_picker_scroll_offset,
                    self.startup_state.theme_picker_cursor,
                    visible_rows,
                );
                self.startup_state.theme_picker_status = None;
                self.startup_state.theme_picker_return_screen = Some(return_screen);
                self.current_screen = ScreenId::ThemePicker;
            }
            Err(err) => match return_screen {
                ScreenId::FirstTimeMenu => {
                    self.startup_state.first_time_status =
                        Some(format!("Unable to load themes: {err}"));
                    self.current_screen = ScreenId::FirstTimeMenu;
                }
                _ => self.show_command_menu_notice(
                    CommandMenu::Main,
                    format!("Unable to load themes: {err}"),
                ),
            },
        }
    }

    pub fn move_theme_picker_cursor(&mut self, delta: isize) {
        if self.current_screen != ScreenId::ThemePicker {
            return;
        }
        let len = self.startup_state.theme_picker_rows.len();
        if len == 0 {
            self.startup_state.theme_picker_cursor = 0;
            return;
        }
        self.startup_state.theme_picker_cursor = crate::app::helpers::move_wrapped_cursor(
            self.startup_state.theme_picker_cursor,
            delta,
            len,
        );
        let visible_rows = self.theme_picker_visible_rows();
        crate::app::helpers::sync_scroll_to_cursor(
            &mut self.startup_state.theme_picker_scroll_offset,
            self.startup_state.theme_picker_cursor,
            visible_rows,
        );
    }

    pub fn append_theme_picker_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::ThemePicker {
            return;
        }
        if self.startup_state.theme_picker_input.len() >= 22 {
            return;
        }
        self.startup_state.theme_picker_input.push(ch);
        self.sync_theme_picker_cursor_to_input();
        self.startup_state.theme_picker_status = None;
    }

    pub fn backspace_theme_picker_input(&mut self) {
        if self.current_screen != ScreenId::ThemePicker {
            return;
        }
        self.startup_state.theme_picker_input.pop();
        self.sync_theme_picker_cursor_to_input();
        self.startup_state.theme_picker_status = None;
    }

    pub fn apply_theme_picker_selection(&mut self) {
        if self.current_screen != ScreenId::ThemePicker {
            return;
        }
        let Some(entry) = self
            .startup_state
            .theme_picker_rows
            .get(self.startup_state.theme_picker_cursor)
            .cloned()
        else {
            self.startup_state.theme_picker_status = Some("No themes are available.".to_string());
            return;
        };
        tracing::debug!(
            entry_key = %entry.key,
            entry_display = %entry.display_name,
            "apply_theme_picker_selection: applying entry"
        );
        match crate::theme::apply_theme_entry(&entry) {
            Ok(()) => {
                let post_key = crate::theme::current_theme_key();
                tracing::debug!(
                    entry_key = %entry.key,
                    current_theme_key_after = ?post_key,
                    "apply_theme_picker_selection: apply_theme_entry Ok"
                );
                if self.player.is_joined {
                    if let Err(err) = self
                        .planet
                        .campaign_store
                        .set_player_theme_preference(self.player.record_index_1_based, &entry.key)
                    {
                        self.startup_state.theme_picker_status = Some(format!(
                            "Applied theme: {}. Could not save preference: {}",
                            entry.display_name, err
                        ));
                    } else {
                        self.startup_state.theme_picker_status =
                            Some(format!("Applied theme: {}.", entry.display_name));
                    }
                } else {
                    self.startup_state.prejoin_theme_key = Some(entry.key.clone());
                    self.startup_state.theme_picker_status =
                        Some(format!("Applied theme: {}.", entry.display_name));
                }
                self.startup_state.theme_picker_cursor =
                    self.theme_picker_cursor_for_key(&entry.key);
                self.startup_state.theme_picker_input.clear();
                let visible_rows = self.theme_picker_visible_rows();
                crate::app::helpers::sync_scroll_to_cursor(
                    &mut self.startup_state.theme_picker_scroll_offset,
                    self.startup_state.theme_picker_cursor,
                    visible_rows,
                );
            }
            Err(ref err) => {
                tracing::debug!(
                    entry_key = %entry.key,
                    error = %err,
                    "apply_theme_picker_selection: apply_theme_entry Err — falling back to default"
                );
                crate::theme::apply_default_theme();
                let fallback_key = crate::theme::default_theme_key();
                if self.player.is_joined {
                    let _ = self.planet.campaign_store.set_player_theme_preference(
                        self.player.record_index_1_based,
                        fallback_key,
                    );
                } else {
                    self.startup_state.prejoin_theme_key = Some(fallback_key.to_string());
                }
                self.startup_state.theme_picker_status = Some(format!(
                    "Theme unavailable. Using {}.",
                    crate::theme::default_theme_display_name()
                ));
                self.startup_state.theme_picker_cursor =
                    self.theme_picker_cursor_for_key(fallback_key);
                self.startup_state.theme_picker_input.clear();
                let visible_rows = self.theme_picker_visible_rows();
                crate::app::helpers::sync_scroll_to_cursor(
                    &mut self.startup_state.theme_picker_scroll_offset,
                    self.startup_state.theme_picker_cursor,
                    visible_rows,
                );
            }
        }
    }

    pub fn exit_theme_picker(&mut self) {
        if self.current_screen != ScreenId::ThemePicker {
            return;
        }
        let current_key_at_exit = crate::theme::current_theme_key();
        tracing::debug!(
            current_theme_key_at_exit = ?current_key_at_exit,
            "exit_theme_picker: leaving picker"
        );
        self.startup_state.theme_picker_rows.clear();
        self.startup_state.theme_picker_cursor = 0;
        self.startup_state.theme_picker_scroll_offset = 0;
        self.startup_state.theme_picker_input.clear();
        self.startup_state.theme_picker_status = None;
        self.current_screen = self
            .startup_state
            .theme_picker_return_screen
            .take()
            .unwrap_or(ScreenId::MainMenu);
    }

    pub fn open_first_time_join_name(&mut self) {
        if !self.game_data.has_open_first_join_slot() {
            self.startup_state.first_time_status =
                Some("This game is already full. No open empires remain.".to_string());
            self.current_screen = ScreenId::FirstTimeMenu;
            return;
        }
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_input.clear();
        self.startup_state.first_time_rename_preloaded_empire = false;
        self.startup_state.first_time_onboarding_mode = self.first_time_onboarding_mode();
        self.current_screen = ScreenId::FirstTimeJoinEmpireName;
    }

    pub fn append_first_time_input_char(&mut self, ch: char) {
        if !matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName
                | ScreenId::FirstTimeHomeworldName
                | ScreenId::ColonyWorldName
        ) {
            return;
        }
        if !nc_data::is_classic_text_char(ch) {
            return;
        }
        if self.startup_state.first_time_input.chars().count() >= 20 {
            return;
        }
        self.startup_state.first_time_input.push(ch);
    }

    pub fn backspace_first_time_input(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName
                | ScreenId::FirstTimeHomeworldName
                | ScreenId::ColonyWorldName
        ) {
            return;
        }
        self.startup_state.first_time_input.pop();
    }

    pub fn submit_first_time_input(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeJoinEmpireName => {
                let value = self.startup_state.first_time_input.trim();
                if value.is_empty() {
                    self.startup_state.first_time_status =
                        Some("Empire names need at least one visible character.".to_string());
                    return;
                }
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_empire_name = value.to_string();
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeJoinEmpireConfirm;
            }
            ScreenId::FirstTimeHomeworldName => {
                let value = self.startup_state.first_time_input.trim();
                if value.is_empty() {
                    self.startup_state.first_time_status =
                        Some("Homeworld names need at least one visible character.".to_string());
                    return;
                }
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_homeworld_name = value.to_string();
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeHomeworldConfirm;
            }
            ScreenId::ColonyWorldName => {
                let value = self.startup_state.first_time_input.trim();
                if value.is_empty() {
                    self.startup_state.first_time_status =
                        Some("World names need at least one visible character.".to_string());
                    return;
                }
                self.startup_state.first_time_status = None;
                self.startup_state.colony_world_planet_record_index_1_based =
                    self.colony_world_target_planet_index();
                self.startup_state.colony_world_name = value.to_string();
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::ColonyWorldConfirm;
            }
            _ => {}
        }
    }

    pub fn accept_first_time_prompt(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeReservedPrompt => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.startup_state.first_time_onboarding_mode =
                    FirstTimeOnboardingMode::BbsReserved;
                self.current_screen = ScreenId::FirstTimeJoinEmpireName;
            }
            ScreenId::FirstTimePreloadedRenamePrompt => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input = self.player.empire_name.clone();
                self.startup_state.first_time_rename_preloaded_empire = true;
                self.current_screen = ScreenId::FirstTimeJoinEmpireName;
            }
            ScreenId::FirstTimeJoinEmpireConfirm => {
                if self.startup_state.first_time_rename_preloaded_empire {
                    match self.complete_preloaded_empire_rename() {
                        Ok(()) => {
                            self.startup_state.first_time_rename_preloaded_empire = false;
                            self.current_screen = ScreenId::FirstTimeJoinSummary;
                        }
                        Err(_) => {
                            self.restore_first_time_input_after_failure(
                                ScreenId::FirstTimeJoinEmpireName,
                                self.startup_state.first_time_empire_name.clone(),
                                "Unable to save your empire name right now. Please try again.",
                            );
                        }
                    }
                } else {
                    match self.complete_first_time_join() {
                        Ok(()) => {
                            self.startup_state.first_time_onboarding_mode =
                                FirstTimeOnboardingMode::Generic;
                            self.current_screen = ScreenId::FirstTimeJoinSummary;
                        }
                        Err(err) => {
                            let status = if let Some(reason) =
                                err.downcast_ref::<nc_data::GameStateMutationError>()
                            {
                                match reason {
                                    nc_data::GameStateMutationError::PlayerAlreadyJoined {
                                        ..
                                    } => {
                                        "That empire slot was just claimed by another player. Please try again."
                                    }
                                    _ => "Unable to join this empire right now. Please try again.",
                                }
                            } else if err.to_string()
                                == "This game is already full. No open empires remain."
                            {
                                "This game is already full. No open empires remain."
                            } else {
                                "Unable to join this empire right now. Please try again."
                            };
                            self.restore_first_time_input_after_failure(
                                ScreenId::FirstTimeJoinEmpireName,
                                self.startup_state.first_time_empire_name.clone(),
                                status,
                            );
                        }
                    }
                }
            }
            ScreenId::FirstTimeJoinSummary => {
                self.current_screen = ScreenId::FirstTimeJoinNoPending;
            }
            ScreenId::FirstTimeJoinNoPending => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.current_screen = self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                match self.complete_first_time_homeworld_name() {
                    Ok(()) => {
                        self.current_screen =
                            self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
                    }
                    Err(_) => {
                        self.restore_first_time_input_after_failure(
                            ScreenId::FirstTimeHomeworldName,
                            self.startup_state.first_time_homeworld_name.clone(),
                            "Unable to save the homeworld name right now. Please try again.",
                        );
                    }
                }
            }
            ScreenId::ColonyWorldConfirm => match self.complete_colony_world_name() {
                Ok(()) => {
                    self.current_screen =
                        self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
                }
                Err(_) => {
                    self.restore_first_time_input_after_failure(
                        ScreenId::ColonyWorldName,
                        self.startup_state.colony_world_name.clone(),
                        "Unable to save the world name right now. Please try again.",
                    );
                }
            },
            _ => {}
        }
    }

    pub fn reject_first_time_prompt(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeReservedPrompt => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.startup_state.first_time_onboarding_mode = FirstTimeOnboardingMode::Generic;
                self.current_screen = ScreenId::FirstTimeMenu;
            }
            ScreenId::FirstTimePreloadedRenamePrompt => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.startup_state.first_time_rename_preloaded_empire = false;
                self.current_screen = ScreenId::FirstTimeJoinSummary;
            }
            ScreenId::FirstTimeJoinEmpireName if self.first_time_join_uses_reserved_prompt() => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeReservedPrompt;
            }
            ScreenId::FirstTimeJoinEmpireName
                if self.startup_state.first_time_rename_preloaded_empire =>
            {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimePreloadedRenamePrompt;
            }
            ScreenId::FirstTimeJoinEmpireConfirm => {
                self.startup_state.first_time_input =
                    self.startup_state.first_time_empire_name.clone();
                self.current_screen = ScreenId::FirstTimeJoinEmpireName;
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                self.startup_state.first_time_input =
                    self.startup_state.first_time_homeworld_name.clone();
                self.current_screen = ScreenId::FirstTimeHomeworldName;
            }
            ScreenId::ColonyWorldName => {
                self.startup_state.first_time_status =
                    Some("You must name this newly colonized world before continuing.".to_string());
                self.current_screen = ScreenId::ColonyWorldName;
            }
            ScreenId::ColonyWorldConfirm => {
                self.startup_state.first_time_input = self.startup_state.colony_world_name.clone();
                self.current_screen = ScreenId::ColonyWorldName;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_startup_key(
        &self,
        phase: StartupPhase,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match phase {
            StartupPhase::Splash => {
                if self.startup_state.splash_page == 0 {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::Advance)
                        }
                        _ => Action::Startup(StartupAction::SkipIntro),
                    }
                } else {
                    Action::Startup(StartupAction::Advance)
                }
            }
            StartupPhase::Intro | StartupPhase::LoginSummary => {
                Action::Startup(StartupAction::Advance)
            }
            StartupPhase::Results => {
                if self.startup_state.results_mode == StartupReviewMode::ItemBody {
                    let review_page = self.startup_review_visible_lines() as isize;
                    if !self.startup_review_is_at_end(true) {
                        return match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                                Action::Startup(StartupAction::ScrollReview(-1))
                            }
                            KeyCode::PageUp => {
                                Action::Startup(StartupAction::ScrollReview(-review_page))
                            }
                            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                                Action::Startup(StartupAction::ScrollReview(1))
                            }
                            KeyCode::PageDown => {
                                Action::Startup(StartupAction::ScrollReview(review_page))
                            }
                            _ => Action::Startup(StartupAction::Advance),
                        };
                    }
                }
                match self.startup_state.results_mode {
                    StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                                Action::Startup(StartupAction::AcceptDefault)
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                Action::Startup(StartupAction::RejectChoice)
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                Action::Startup(StartupAction::EnableNonstop)
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                            _ => Action::Noop,
                        }
                    }
                    StartupReviewMode::ItemBody => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')
                            if self.startup_review_is_at_end(true)
                                && !self.startup_review_nonstop(true) =>
                        {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Char('n') | KeyCode::Char('N')
                            if self.startup_review_is_at_end(true)
                                && !self.startup_review_nonstop(true) =>
                        {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            Action::Startup(StartupAction::ScrollReview(-1))
                        }
                        KeyCode::PageUp => Action::Startup(StartupAction::ScrollReview(
                            -(self.startup_review_visible_lines() as isize),
                        )),
                        _ => Action::Startup(StartupAction::Advance),
                    },
                    StartupReviewMode::DeletePrompt => match key.code {
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                        _ => Action::Noop,
                    },
                    StartupReviewMode::EndStatus => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                        _ => Action::Startup(StartupAction::Advance),
                    },
                }
            }
            StartupPhase::Messages => {
                if self.startup_state.messages_mode == StartupReviewMode::ItemBody {
                    let review_page = self.startup_review_visible_lines() as isize;
                    if !self.startup_review_is_at_end(false) {
                        return match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                                Action::Startup(StartupAction::ScrollReview(-1))
                            }
                            KeyCode::PageUp => {
                                Action::Startup(StartupAction::ScrollReview(-review_page))
                            }
                            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                                Action::Startup(StartupAction::ScrollReview(1))
                            }
                            KeyCode::PageDown => {
                                Action::Startup(StartupAction::ScrollReview(review_page))
                            }
                            _ => Action::Startup(StartupAction::Advance),
                        };
                    }
                }
                match self.startup_state.messages_mode {
                    StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                                Action::Startup(StartupAction::AcceptDefault)
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                Action::Startup(StartupAction::RejectChoice)
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                Action::Startup(StartupAction::EnableNonstop)
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                            _ => Action::Noop,
                        }
                    }
                    StartupReviewMode::ItemBody => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')
                            if self.startup_review_is_at_end(false)
                                && !self.startup_review_nonstop(false) =>
                        {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Char('n') | KeyCode::Char('N')
                            if self.startup_review_is_at_end(false)
                                && !self.startup_review_nonstop(false) =>
                        {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            Action::Startup(StartupAction::ScrollReview(-1))
                        }
                        KeyCode::PageUp => Action::Startup(StartupAction::ScrollReview(
                            -(self.startup_review_visible_lines() as isize),
                        )),
                        _ => Action::Startup(StartupAction::Advance),
                    },
                    StartupReviewMode::DeletePrompt => match key.code {
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                        _ => Action::Noop,
                    },
                    StartupReviewMode::EndStatus => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::RequestQuit,
                        _ => Action::Startup(StartupAction::Advance),
                    },
                }
            }
            StartupPhase::Complete => Action::Noop,
        }
    }

    pub(crate) fn first_time_empire_rows(&self) -> Vec<String> {
        self.game_data
            .player
            .records
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                let slot = idx + 1;
                if player.occupied_flag() != 0 {
                    let handle = player.assigned_player_handle_summary();
                    let empire = player.controlled_empire_name_summary();
                    format!(
                        "Empire {:>2}: JOINED  {}{}",
                        slot,
                        if empire.is_empty() {
                            "Empire".to_string()
                        } else {
                            empire
                        },
                        if handle.is_empty() {
                            String::new()
                        } else {
                            format!(" [{handle}]")
                        }
                    )
                } else if self.game_data.player_slot_is_open_for_first_join(slot) {
                    format!(
                        "Empire {:>2}: OPEN    Available for a new Star Master",
                        slot
                    )
                } else {
                    format!("Empire {:>2}: CLOSED  No longer available", slot)
                }
            })
            .collect()
    }

    pub(crate) fn first_time_homeworld_summary(
        &self,
    ) -> Result<([u8; 2], u16, u16), Box<dyn std::error::Error>> {
        let planet_index = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
            .ok_or("player record missing for homeworld prompt")?
            .homeworld_planet_index_1_based_raw() as usize;
        let planet = self
            .game_data
            .planets
            .records
            .get(planet_index.saturating_sub(1))
            .ok_or("homeworld planet missing for first-time prompt")?;
        Ok((
            planet.coords_raw(),
            planet
                .present_production_points()
                .unwrap_or(planet.potential_production_points()),
            planet.potential_production_points(),
        ))
    }

    pub(crate) fn colony_world_summary(
        &self,
    ) -> Result<([u8; 2], u16, u16), Box<dyn std::error::Error>> {
        let planet_index = self
            .startup_state
            .colony_world_planet_record_index_1_based
            .or_else(|| self.colony_world_target_planet_index())
            .ok_or("colony world prompt missing target planet")?;
        let planet = self
            .game_data
            .planets
            .records
            .get(planet_index - 1)
            .ok_or("colony world missing for naming prompt")?;
        Ok((
            planet.coords_raw(),
            planet
                .present_production_points()
                .unwrap_or(planet.potential_production_points()),
            planet.potential_production_points(),
        ))
    }

    fn complete_first_time_join(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.startup_state.unbound_bbs_caller {
            let runtime_state = self
                .planet
                .campaign_store
                .load_latest_runtime_state()?
                .ok_or(
                    "campaign store has no snapshots; initialize the campaign with nc-sysop first",
                )?;
            let player_record_index_1_based = runtime_state
                .game_data
                .player
                .records
                .iter()
                .enumerate()
                .find_map(|(idx, _)| {
                    let seat = idx + 1;
                    (runtime_state
                        .game_data
                        .player_slot_is_open_for_first_join(seat)
                        && self.game_config.reservation_for_player(seat).is_none())
                    .then_some(seat)
                })
                .ok_or("This game is already full. No open empires remain.")?;
            self.reload_runtime_state_and_bind_player_record_index_1_based(
                player_record_index_1_based,
            )?;
        } else {
            self.reload_runtime_state_and_bind_player_record_index_1_based(
                self.player.record_index_1_based,
            )?;
        }
        self.game_data.join_player(
            self.player.record_index_1_based,
            &self.startup_state.first_time_empire_name,
        )?;
        if let Some(alias) = self.startup_state.caller_alias.as_deref() {
            if let Some(player) = self
                .game_data
                .player
                .records
                .get_mut(self.player.record_index_1_based - 1)
            {
                player.set_assigned_player_handle_raw(alias);
            }
        }
        self.startup_state.unbound_bbs_caller = false;
        self.save_game_data()?;
        self.refresh_player_context()?;
        if self.player.is_joined {
            if let Some(theme_key) = self.startup_state.prejoin_theme_key.take() {
                self.planet
                    .campaign_store
                    .set_player_theme_preference(self.player.record_index_1_based, &theme_key)?;
            }
        }
        Ok(())
    }

    fn complete_first_time_homeworld_name(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.game_data.rename_player_homeworld(
            self.player.record_index_1_based,
            &self.startup_state.first_time_homeworld_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn complete_colony_world_name(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let planet_index = self
            .startup_state
            .colony_world_planet_record_index_1_based
            .or_else(|| self.colony_world_target_planet_index())
            .ok_or("colony world prompt missing target planet")?;
        self.game_data.rename_owned_planet(
            self.player.record_index_1_based,
            planet_index,
            &self.startup_state.colony_world_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        self.startup_state.colony_world_planet_record_index_1_based = None;
        self.startup_state.colony_world_name.clear();
        Ok(())
    }

    fn complete_preloaded_empire_rename(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let player = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
            .ok_or("player record missing for pre-loaded rename")?;
        player.set_controlled_empire_name_raw(&self.startup_state.first_time_empire_name);
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn restore_first_time_input_after_failure(
        &mut self,
        screen: ScreenId,
        input: String,
        status: &str,
    ) {
        self.startup_state.first_time_input = input;
        self.startup_state.first_time_status = Some(status.to_string());
        self.current_screen = screen;
    }

    fn refresh_player_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.bind_player_record_index_1_based(self.player.record_index_1_based)
    }

    pub(crate) fn refresh_review_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let refreshed = ReportsPreview::from_block_rows(
            &self.game_data,
            self.player.record_index_1_based as u8,
            &self.report_block_rows,
            &self.queued_mail,
        );
        let has_results = self.has_active_report_blocks();
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            has_results,
            has_visible_runtime_messages(self.player.record_index_1_based as u8, &self.queued_mail),
        );
        let startup_summary = StartupSummary::from_reports(
            summary.game_year,
            self.player.classic_login_state,
            summary.pending_results,
            summary.pending_messages,
            &refreshed,
        );
        self.startup.replace(startup_summary, refreshed);
        Ok(())
    }

    fn reset_startup_review_cursors_for_phase_exit(&mut self) {
        if self.current_screen != ScreenId::Startup(StartupPhase::Results) {
            self.startup_state.results_block = 0;
            self.startup_state.results_page = 0;
            self.startup_state.results_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.results_nonstop = false;
            self.startup_state.results_deleted_any = false;
            self.startup_state.results_review_history_rows.clear();
        }
        if self.current_screen != ScreenId::Startup(StartupPhase::Messages) {
            self.startup_state.messages_block = 0;
            self.startup_state.messages_page = 0;
            self.startup_state.messages_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.messages_nonstop = false;
            self.startup_state.messages_deleted_any = false;
            self.startup_state.messages_review_history_rows.clear();
        }
    }

    fn sync_player_review_flags(&mut self) {
        let has_results = self.has_active_report_blocks();
        let results_chain =
            layout_classic_results(&self.game_data.player.records, &self.report_block_rows)
                .chain_for(self.player.record_index_1_based as u8);
        if let Some(player) = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
        {
            player.set_classic_login_reviewables_present(
                has_results
                    || has_visible_runtime_messages(
                        self.player.record_index_1_based as u8,
                        &self.queued_mail,
                    ),
            );
            player.set_classic_results_chain(results_chain);
        }
    }

    fn startup_target_screen(&mut self, phase: StartupPhase) -> ScreenId {
        match phase {
            StartupPhase::Complete => match self.player.classic_login_state {
                crate::model::ClassicLoginState::FirstTimeMenu => {
                    self.startup_state.first_time_onboarding_mode =
                        self.first_time_onboarding_mode();
                    if self.is_bbs_reserved_first_time_login() {
                        ScreenId::FirstTimeReservedPrompt
                    } else if self.startup_state.fixed_player_launch {
                        self.startup_state.first_time_status = None;
                        self.startup_state.first_time_input.clear();
                        ScreenId::FirstTimeJoinEmpireName
                    } else {
                        ScreenId::FirstTimeMenu
                    }
                }
                crate::model::ClassicLoginState::MatchedPreloadedFirstLogin => {
                    ScreenId::FirstTimePreloadedRenamePrompt
                }
                crate::model::ClassicLoginState::ReturningPlayer => {
                    if let Err(err) = self.record_returning_player_participation_once() {
                        tracing::error!(error = %err, "failed to record returning-player participation");
                    }
                    match self.player_access_mode {
                        PlayerAccessMode::Normal => {
                            self.pending_naming_screen().unwrap_or(ScreenId::MainMenu)
                        }
                        PlayerAccessMode::SurveyOnly => {
                            self.show_command_menu_notice(
                                CommandMenu::General,
                                "Survey mode: the campaign is over and no further orders will be accepted.",
                            );
                            self.pending_naming_screen().unwrap_or(ScreenId::MainMenu)
                        }
                        PlayerAccessMode::ReviewOnly => {
                            if let Err(err) = self.complete_terminal_review() {
                                tracing::error!(error = %err, "failed to persist terminal review state");
                            }
                            ScreenId::TerminalNotice
                        }
                        PlayerAccessMode::LockedOut => ScreenId::TerminalNotice,
                    }
                }
            },
            other => ScreenId::Startup(other),
        }
    }

    fn complete_terminal_review(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let player_idx = self.player.record_index_1_based.saturating_sub(1);
        let outcome = self
            .player_lifecycle_states
            .get(player_idx)
            .map(|state| state.terminal_outcome)
            .unwrap_or(TerminalOutcome::None);
        if let Some(state) = self.player_lifecycle_states.get_mut(player_idx) {
            state.terminal_review_consumed = true;
        }
        self.player_access_mode = PlayerAccessMode::LockedOut;
        self.terminal_notice_lines = match outcome {
            TerminalOutcome::Defeated => vec![
                "Your empire has been defeated.".to_string(),
                "Command access is now closed.".to_string(),
                "Press any key to exit.".to_string(),
            ],
            TerminalOutcome::LostGame => vec![
                "The campaign is over.".to_string(),
                "The victor has been declared and your final review is complete.".to_string(),
                "Press any key to exit.".to_string(),
            ],
            _ => vec![
                "This session is no longer playable.".to_string(),
                "Press any key to exit.".to_string(),
            ],
        };
        self.save_terminal_access_state()
    }

    fn pending_naming_screen(&self) -> Option<ScreenId> {
        let Some(player) = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
        else {
            return None;
        };
        let planet_index = player.homeworld_planet_index_1_based_raw() as usize;
        if planet_index == 0 {
            return self.pending_colony_world_naming_screen();
        }
        if self
            .game_data
            .planets
            .records
            .get(planet_index - 1)
            .filter(|planet| planet.is_named_homeworld_seed())
            .is_some()
        {
            return Some(ScreenId::FirstTimeHomeworldName);
        }
        self.pending_colony_world_naming_screen()
    }

    fn pending_colony_world_naming_screen(&self) -> Option<ScreenId> {
        self.colony_world_target_planet_index()
            .map(|_| ScreenId::ColonyWorldName)
    }

    fn theme_picker_cursor_for_key(&self, key: &str) -> usize {
        self.startup_state
            .theme_picker_rows
            .iter()
            .position(|entry| entry.key == key)
            .or_else(|| {
                self.startup_state
                    .theme_picker_rows
                    .iter()
                    .position(|entry| entry.key == "tokyo_night")
            })
            .unwrap_or(0)
    }

    fn sync_theme_picker_cursor_to_input(&mut self) {
        let rows = self
            .startup_state
            .theme_picker_rows
            .iter()
            .map(|row| vec![String::new(), row.display_name.clone()])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &rows,
            1,
            &self.startup_state.theme_picker_input,
        ) else {
            return;
        };
        self.startup_state.theme_picker_cursor = index;
        let visible_rows = self.theme_picker_visible_rows();
        crate::app::helpers::sync_scroll_to_cursor(
            &mut self.startup_state.theme_picker_scroll_offset,
            self.startup_state.theme_picker_cursor,
            visible_rows,
        );
    }

    fn colony_world_target_planet_index(&self) -> Option<usize> {
        let player_empire = self.player.record_index_1_based as u8;
        let homeworld_index = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
            .map(|player| player.homeworld_planet_index_1_based_raw() as usize)
            .unwrap_or(0);
        self.game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(idx, planet)| {
                planet.owner_empire_slot_raw() == player_empire
                    && planet.planet_name() == "Not Named Yet"
                    && *idx + 1 != homeworld_index
            })
            .map(|(idx, _)| idx + 1)
    }
}
