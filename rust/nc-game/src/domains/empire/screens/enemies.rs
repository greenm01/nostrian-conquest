use crossterm::event::{KeyCode, KeyEvent};
use nc_data::DiplomaticRelation;

use crate::app::Action;
use crate::domains::empire::EmpireAction;
use crate::screen::layout::{ScreenGeometry, new_playfield_for, standard_table_visible_rows_for};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, format_empire_id, layout_standard_table_block,
    resolve_table_columns_for_widget, write_table_window_with_cursor_at,
};
use crate::screen::{COMMAND_LABEL, PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;

pub struct EnemiesScreen;

pub fn enemies_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

const ENEMIES_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::right("ID", 3),
    TableColumn::left("Empire", 26),
    TableColumn::left("Status", 8),
];

impl EnemiesScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
        input: &str,
        _status: Option<&str>,
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(frame.geometry);

        let viewer_empire = frame.player.record_index_1_based as u8;
        let mut others = frame
            .game_data
            .players
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx + 1 != frame.player.record_index_1_based)
            .map(|(idx, player)| {
                let empire_id = (idx + 1) as u8;
                let name = player.controlled_empire_name_summary();
                let fallback = player.legacy_status_name_summary();
                let display = if !name.is_empty() { name } else { fallback };
                let relation = frame
                    .game_data
                    .stored_diplomatic_relation(viewer_empire, empire_id)
                    .unwrap_or(DiplomaticRelation::Neutral);
                (empire_id, display, relation)
            })
            .collect::<Vec<_>>();
        others.sort_by_key(|(empire_id, _, _)| *empire_id);

        let rows = others
            .into_iter()
            .map(|(empire_id, name, relation)| {
                vec![
                    format_empire_id(empire_id),
                    name,
                    match relation {
                        DiplomaticRelation::Enemy => "ENEMY".to_string(),
                        DiplomaticRelation::Neutral | DiplomaticRelation::Unknown(_) => {
                            "NEUTRAL".to_string()
                        }
                    },
                ]
            })
            .collect::<Vec<_>>();

        let visible_rows = enemies_visible_rows(frame.geometry);
        let displayed_rows = rows.len().saturating_sub(scroll_offset).min(visible_rows);
        let scrollable = rows.len() > visible_rows;
        let footer = if rows.is_empty() {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: "No empires found.",
            }
        } else {
            let default_empire = rows
                .get(cursor)
                .and_then(|row| row.first())
                .map(String::as_str)
                .unwrap_or("");
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: Some(default_empire),
                input,
            }
        };
        let columns = resolve_table_columns_for_widget(
            &ENEMIES_COLUMNS,
            &rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("ENEMIES, DECLARE OR LIST:"),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some("ENEMIES, DECLARE OR LIST:"),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "ENEMIES, DECLARE OR LIST:",
        );
        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );

        draw_table_footer(
            &mut buffer,
            frame.geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Empire(EmpireAction::MoveEnemies(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Empire(EmpireAction::MoveEnemies(1))
            }
            KeyCode::PageUp => Action::Empire(EmpireAction::MoveEnemies(-8)),
            KeyCode::PageDown => Action::Empire(EmpireAction::MoveEnemies(8)),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Enter => Action::Empire(EmpireAction::SubmitEnemiesInput),
            KeyCode::Backspace => Action::Empire(EmpireAction::BackspaceEnemiesInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Empire(EmpireAction::AppendEnemiesChar(ch))
            }
            _ => Action::Noop,
        }
    }
}
