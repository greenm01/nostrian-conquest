use crate::app::state::App;
use crate::domains::messaging::controller::compose_recipient_label;
use crate::domains::messaging::state::OutboxMode;
use crate::reports::runtime_outgoing_message_lines;
use crate::screen::{PlayfieldBuffer, ScreenFrame, ScreenId};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
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
    match app.current_screen {
        ScreenId::ComposeMessageRecipient => app.message_compose.render_recipient(
            &frame,
            &app.messaging.compose_recipient_input,
            app.messaging.compose_recipient_status.as_deref(),
            app.messaging.compose_recipient_scroll_offset,
            app.messaging.compose_recipient_cursor,
        ),
        ScreenId::ComposeMessageSubject => app.message_compose.render_subject(
            &compose_recipient_label(&app.game_data, app.messaging.compose_recipient_empire),
            &app.messaging.compose_subject,
            app.messaging.compose_subject_status.as_deref(),
        ),
        ScreenId::ComposeMessageBody => app.message_compose.render_body(
            frame.geometry,
            &compose_recipient_label(&app.game_data, app.messaging.compose_recipient_empire),
            &app.messaging.compose_subject,
            &app.messaging.compose_body,
            app.messaging.compose_body_cursor_row,
            app.messaging.compose_body_cursor_col,
            app.messaging.compose_body_status.as_deref(),
        ),
        ScreenId::ComposeMessageOutbox => {
            let queue = app.compose_outbox_queue()?;
            match (
                app.messaging.compose_outbox_mode,
                queue.get(app.messaging.compose_outbox_cursor),
            ) {
                (OutboxMode::Read, Some(mail)) => app.message_compose.render_outbox_message(
                    frame.geometry,
                    &runtime_outgoing_message_lines(&app.game_data, mail),
                ),
                (mode, _) => app.message_compose.render_outbox(
                    frame.geometry,
                    &queue,
                    &app.messaging.compose_outbox_input,
                    app.messaging.compose_outbox_status.as_deref(),
                    app.messaging.compose_outbox_scroll_offset,
                    app.messaging.compose_outbox_cursor,
                    mode,
                    &app.game_data,
                ),
            }
        }
        ScreenId::ComposeMessageDiscardConfirm => app.message_compose.render_discard_confirm(
            frame.geometry,
            &compose_recipient_label(&app.game_data, app.messaging.compose_recipient_empire),
            &app.messaging.compose_subject,
            &app.messaging.compose_body,
        ),
        ScreenId::ComposeMessageSendConfirm => app.message_compose.render_send_confirm(
            frame.geometry,
            &compose_recipient_label(&app.game_data, app.messaging.compose_recipient_empire),
            &app.messaging.compose_subject,
            &app.messaging.compose_body,
        ),
        ScreenId::ComposeMessageSent => app.message_compose.render_sent(
            app.messaging
                .compose_sent_status
                .as_deref()
                .unwrap_or("Message queued."),
        ),
        _ => unreachable!("messaging views called for non-messaging screen"),
    }
}
