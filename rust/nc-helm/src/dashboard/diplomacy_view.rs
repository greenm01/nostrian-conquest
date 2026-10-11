use crate::dashboard::buffer::{CellStyle, GameColor};
use nc_data::{
    DiplomaticRelation, Player, PlayerActivityState, PlayerLifecycleState, PublicEmpireStatus,
    player_public_status,
};

use crate::dashboard::theme;

pub(crate) fn display_name(player: &Player, empire_slot: u8) -> String {
    let name = player.empire_name.trim().to_string();
    let fallback = player.empire_name.trim().to_string();
    if !name.is_empty() {
        name
    } else if !fallback.is_empty() {
        fallback
    } else {
        format!("Empire #{empire_slot}")
    }
}

pub(crate) fn state_label_and_style(
    game_data: &nc_data::CoreGameData,
    player: &Player,
    player_activity_states: &[PlayerActivityState],
    player_lifecycle_states: &[PlayerLifecycleState],
    viewer_slot: u8,
    empire_slot: u8,
) -> (&'static str, CellStyle) {
    if player.mode == nc_data::PlayerMode::Rogue {
        ("Rogue", theme::alert_style())
    } else {
        match player_public_status(
            game_data,
            empire_slot as usize,
            player_activity_states,
            player_lifecycle_states,
        ) {
            PublicEmpireStatus::Active if empire_slot == viewer_slot => {
                ("Active", theme::friendly_style())
            }
            PublicEmpireStatus::Active => ("Active", theme::dim_style()),
            PublicEmpireStatus::Mia => ("MIA", theme::alert_style()),
            PublicEmpireStatus::Defeated => ("Defeated", theme::icd_style()),
        }
    }
}

pub(crate) fn relation_label_and_style(
    viewer: Option<&Player>,
    viewer_slot: u8,
    empire_slot: u8,
) -> (&'static str, CellStyle) {
    if empire_slot == viewer_slot {
        ("—", theme::dim_style())
    } else if viewer.and_then(|row| nc_data::diplomatic_relation_toward(row, empire_slot))
        == Some(DiplomaticRelation::Enemy)
    {
        ("Enemy", theme::enemy_style())
    } else {
        ("Neutral", theme::dim_style())
    }
}

pub(crate) fn empire_name_style(empire_slot: u8, bg: GameColor, bold: bool) -> CellStyle {
    theme::empire_slot_style_on(empire_slot, bg, bold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::buffer::GameColor;

    #[test]
    fn blank_display_name_falls_back_to_empire_slot() {
        let player = Player::default();
        assert_eq!(display_name(&player, 4), "Empire #4");
    }

    #[test]
    fn empire_name_style_uses_theme_slot_color() {
        let style = empire_name_style(3, GameColor::Black, false);
        assert_eq!(
            style.fg,
            crate::dashboard::theme::classic::empire_slot_color(3)
        );
        assert_eq!(style.bg, GameColor::Black);
    }
}
