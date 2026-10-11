use crate::app::state::App;
use crate::domains::messaging::MessagingAction;
use crate::screen::layout::PromptFeedback;
use crate::screen::{CommandMenu, ScreenId};

pub fn update(app: &mut App, action: MessagingAction) {
    match action {
        MessagingAction::SetInboxFolderSent => app.set_inbox_folder_sent(),
        MessagingAction::SetInboxTypeFilterMessages => app.set_inbox_type_filter_messages(),
        MessagingAction::SetInboxTypeFilterReports => app.set_inbox_type_filter_reports(),
        MessagingAction::OpenInboxYearPrompt => app.open_inbox_year_prompt(),
        MessagingAction::ClearInboxYearFilter => app.clear_inbox_year_filter(),
        MessagingAction::AppendInboxYearChar(ch) => app.append_inbox_year_char(ch),
        MessagingAction::BackspaceInboxYearInput => app.backspace_inbox_year_input(),
        MessagingAction::SubmitInboxYearInput => app.submit_inbox_year_input(),
        MessagingAction::MoveInboxCursor(delta) => app.move_inbox_cursor(delta),
        MessagingAction::PageInboxCursor(delta) => app.page_inbox_cursor(delta),
        MessagingAction::ScrollInboxPreview(delta) => app.scroll_inbox_preview(delta),
        MessagingAction::ToggleInboxFocus => app.toggle_inbox_focus(),
        MessagingAction::AppendInboxIdChar(ch) => app.append_inbox_id_char(ch),
        MessagingAction::BackspaceInboxIdInput => app.backspace_inbox_id_input(),
        MessagingAction::SubmitInboxIdInput => app.submit_inbox_id_input(),
        MessagingAction::OpenInboxDeleteConfirm => app.open_inbox_delete_confirm(),
        MessagingAction::CancelInboxPrompt => app.cancel_inbox_prompt(),
        MessagingAction::ConfirmDeleteInboxItem => {
            if let Err(err) = app.confirm_delete_inbox_item() {
                app.log_action_error("confirm_delete_inbox_item", err.as_ref());
                app.messaging.inbox_feedback = Some(PromptFeedback::error(
                    "Unable to delete this inbox item right now. Please try again.",
                ));
            }
        }
        MessagingAction::OpenDeleteReviewables => app.open_delete_reviewables(),
        MessagingAction::CloseDeleteReviewables => app.close_delete_reviewables_prompt(),
        MessagingAction::OpenComposeRecipient => app.open_compose_message_recipient(),
        MessagingAction::OpenComposeSubject => app.open_compose_message_subject(),
        MessagingAction::OpenComposeBody => app.open_compose_message_body(),
        MessagingAction::OpenComposeOutbox => app.open_compose_message_outbox(),
        MessagingAction::OpenComposeDiscardConfirm => app.open_compose_message_discard_confirm(),
        MessagingAction::OpenComposeSendConfirm => app.open_compose_message_send_confirm(),
        MessagingAction::ScrollComposeRecipients(delta) => app.scroll_compose_recipients(delta),
        MessagingAction::MoveComposeRecipient(delta) => app.move_compose_recipient_cursor(delta),
        MessagingAction::ScrollComposeOutbox(delta) => app.scroll_compose_outbox(delta),
        MessagingAction::MoveComposeOutbox(delta) => app.move_compose_outbox_cursor(delta),
        MessagingAction::ConfirmDeleteReviewables => {
            if let Err(err) = app.delete_reviewables() {
                app.log_action_error("delete_reviewables", err.as_ref());
                app.show_command_menu_notice(
                    CommandMenu::General,
                    "Unable to delete messages and results right now. Please try again.",
                );
                app.messaging.delete_reviewables_prompt_active = true;
            }
        }
        MessagingAction::AppendComposeRecipientChar(ch) => app.append_compose_recipient_char(ch),
        MessagingAction::BackspaceComposeRecipient => app.backspace_compose_recipient(),
        MessagingAction::SubmitComposeRecipient => app.submit_compose_recipient(),
        MessagingAction::AppendComposeSubjectChar(ch) => app.append_compose_subject_char(ch),
        MessagingAction::BackspaceComposeSubject => app.backspace_compose_subject(),
        MessagingAction::SubmitComposeSubject => app.submit_compose_subject(),
        MessagingAction::AppendComposeBodyChar(ch) => app.append_compose_body_char(ch),
        MessagingAction::InsertComposeTab => app.insert_compose_tab(),
        MessagingAction::BackspaceComposeBody => app.backspace_compose_body(),
        MessagingAction::DeleteComposeBodyChar => app.delete_compose_body_char(),
        MessagingAction::InsertComposeNewline => app.insert_compose_newline(),
        MessagingAction::MoveComposeBodyCursorLeft => app.move_compose_body_cursor_left(),
        MessagingAction::MoveComposeBodyCursorRight => app.move_compose_body_cursor_right(),
        MessagingAction::MoveComposeBodyCursorUp => app.move_compose_body_cursor_up(),
        MessagingAction::MoveComposeBodyCursorDown => app.move_compose_body_cursor_down(),
        MessagingAction::MoveComposeBodyCursorHome => app.move_compose_body_cursor_home(),
        MessagingAction::MoveComposeBodyCursorEnd => app.move_compose_body_cursor_end(),
        MessagingAction::SendComposedMessage => {
            if let Err(err) = app.send_composed_message() {
                set_compose_send_failure(app, "send_composed_message", err.as_ref());
            }
        }
        MessagingAction::AppendComposeOutboxChar(ch) => app.append_compose_outbox_char(ch),
        MessagingAction::BackspaceComposeOutboxInput => app.backspace_compose_outbox_input(),
        MessagingAction::OpenComposeOutboxMessage => app.open_compose_outbox_message(),
        MessagingAction::ScrollComposeOutboxMessage(delta) => {
            app.scroll_compose_outbox_message(delta)
        }
        MessagingAction::OpenComposeOutboxDeleteConfirm => app.open_compose_outbox_delete_confirm(),
        MessagingAction::ReturnToComposeOutboxList => app.return_to_compose_outbox_list(),
        MessagingAction::DeleteQueuedComposeMessage => {
            if let Err(err) = app.delete_queued_compose_message() {
                app.log_action_error("delete_queued_compose_message", err.as_ref());
                app.messaging.compose_outbox_status = Some(
                    "Unable to delete this queued message right now. Please try again.".to_string(),
                );
            }
        }
        MessagingAction::ConfirmDiscardComposedMessage => app.confirm_discard_composed_message(),
        MessagingAction::ConfirmSendComposedMessage => {
            if let Err(err) = app.send_composed_message() {
                set_compose_send_failure(app, "confirm_send_composed_message", err.as_ref());
            }
        }
    }
}

fn set_compose_send_failure(app: &mut App, action: &'static str, err: &dyn std::error::Error) {
    app.log_action_error(action, err);
    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body_status =
        Some("Unable to queue this message right now. Please try again.".to_string());
}
