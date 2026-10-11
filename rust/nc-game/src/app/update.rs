use crate::app::action::Action;
use crate::app::state::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOutcome {
    Continue,
    Quit,
}

pub fn apply_action(app: &mut App, action: Action) -> AppOutcome {
    let outcome = dispatch_action(app, action);
    app.show_stale_session_notice_if_due();
    outcome
}

fn dispatch_action(app: &mut App, action: Action) -> AppOutcome {
    match action {
        // Domain dispatch
        Action::Startup(act) => {
            crate::domains::startup::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Fleet(act) => {
            crate::domains::fleet::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Planet(act) => {
            crate::domains::planet::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Starbase(act) => {
            crate::domains::starbase::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Empire(act) => {
            crate::domains::empire::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Messaging(act) => {
            crate::domains::messaging::update::update(app, act);
            AppOutcome::Continue
        }
        Action::Starmap(act) => {
            crate::domains::starmap::update::update(app, act);
            AppOutcome::Continue
        }

        // Top-level app actions
        Action::OpenMainMenu => {
            app.open_main_menu();
            AppOutcome::Continue
        }
        Action::OpenGeneralMenu => {
            app.open_general_menu();
            AppOutcome::Continue
        }
        Action::OpenPopupHelp => {
            app.open_popup_help();
            AppOutcome::Continue
        }
        Action::DismissPopupHelp => {
            app.dismiss_popup_help();
            AppOutcome::Continue
        }
        Action::ToggleAnsiMode => {
            let _ = crate::theme::toggle_ansi_mode();
            AppOutcome::Continue
        }
        Action::ToggleExpertMode => {
            app.toggle_expert_mode();
            AppOutcome::Continue
        }
        Action::ReturnToCommandMenu => {
            app.return_to_command_menu();
            AppOutcome::Continue
        }
        Action::ToggleAutopilot => match app.toggle_autopilot() {
            Ok(()) | Err(_) => AppOutcome::Continue,
        },
        Action::RequestQuit => {
            app.request_quit();
            AppOutcome::Continue
        }
        Action::CancelQuitPrompt => {
            app.cancel_quit_prompt();
            AppOutcome::Continue
        }
        Action::Quit => AppOutcome::Quit,
        Action::Noop => AppOutcome::Continue,
    }
}
