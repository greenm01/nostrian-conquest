use crossterm::event::KeyEvent;
use nc_data::{
    CompactUnitSummaryStyle, IntelTier, OwnedPlanetStatus, PlanetIntelSnapshot,
    build_player_starmap_projection_from_snapshots,
    format_build_queue_summary as shared_build_queue_summary,
    format_owned_orbit_summary as shared_owned_orbit_summary,
    format_stardock_summary as shared_stardock_summary, owned_orbit_presence, owned_planet_status,
};

use crate::app::Action;
use crate::domains::planet::{KnownOwnerLabelStyle, known_owner_label};
use crate::screen::layout::{
    aligned_label_width, dismiss_prompt_row, draw_aligned_detail_line, draw_aligned_status_line,
    draw_dismiss_prompt_padded, draw_title_bar_padded, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, ScreenFrame, format_sector_coords_zero_padded};

pub struct PlanetInfoScreen;

impl PlanetInfoScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        planet_idx: usize,
        _menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let planet = frame
            .game_data
            .planets
            .get(planet_idx)
            .ok_or("planet detail missing")?;
        if planet.owner.0 as usize != frame.player.record_index_1_based {
            return self.render_intel_detail(frame, planet_idx);
        }

        let [x, y] = planet.sector.to_array();
        let owner_empire_id = planet.owner.0;
        let owner_label = owner_summary(frame, owner_empire_id);
        let state_label = owner_state_summary(frame, owner_empire_id);
        let owned_since_label = frame
            .owned_planet_years
            .get(&(planet_idx + 1))
            .map(|year| format!("Y{year}"))
            .unwrap_or_else(|| "?".to_string());
        let present = nc_data::present_production_points(&planet);
        let potential = u16::from(planet.potential_production);
        let efficiency = if potential == 0 {
            0.0
        } else {
            (present as f64 / potential as f64) * 100.0
        };
        let expected_revenue = frame.game_data.planet_forecast_revenue(planet);
        let budget = frame
            .game_data
            .empire_planet_economy_rows(frame.player.record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_idx + 1)
            .map(|row| u32::from(row.build_capacity).min(row.stored_production_points))
            .unwrap_or_else(|| nc_data::stored_production_points(&planet).min(u32::from(present)));
        let info_label_width = aligned_label_width([
            "Coordinates",
            "Planet",
            "Owner",
            "State",
            "Owned Since",
            "Production",
            "Potential Production",
            "Treasury",
            "Budget",
            "Efficiency",
            "Expected Revenue",
            "Armies",
            "Ground Batteries",
            "Space Forces",
            "Status",
        ]);
        let bottom_label_width = aligned_label_width(["Building", "Docked"]);

        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "INFO ABOUT A PLANET:");
        draw_aligned_status_line(
            &mut buffer,
            2,
            info_label_width,
            "Coordinates",
            &format_sector_coords_zero_padded([x, y]),
        );
        draw_aligned_status_line(
            &mut buffer,
            3,
            info_label_width,
            "Planet",
            &planet.name.trim().to_string(),
        );
        draw_aligned_status_line(&mut buffer, 4, info_label_width, "Owner", &owner_label);
        draw_aligned_status_line(&mut buffer, 5, info_label_width, "State", &state_label);
        draw_aligned_status_line(
            &mut buffer,
            6,
            info_label_width,
            "Owned Since",
            &owned_since_label,
        );
        draw_aligned_status_line(
            &mut buffer,
            7,
            info_label_width,
            "Production",
            &present.to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            8,
            info_label_width,
            "Potential Production",
            &potential.to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            9,
            info_label_width,
            "Treasury",
            &nc_data::stored_production_points(&planet).to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            10,
            info_label_width,
            "Budget",
            &budget.to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            11,
            info_label_width,
            "Efficiency",
            &format!("{efficiency:.1}%"),
        );
        draw_aligned_status_line(
            &mut buffer,
            12,
            info_label_width,
            "Expected Revenue",
            &format!("{expected_revenue} points"),
        );
        draw_aligned_status_line(
            &mut buffer,
            13,
            info_label_width,
            "Armies",
            &(planet.armies.min(u16::from(u8::MAX)) as u8).to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            14,
            info_label_width,
            "Ground Batteries",
            &(planet.ground_batteries.min(u16::from(u8::MAX)) as u8).to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            15,
            info_label_width,
            "Space Forces",
            &format_owned_orbit_summary(frame, [x, y]),
        );
        draw_aligned_status_line(
            &mut buffer,
            16,
            info_label_width,
            "Status",
            &owned_status_summary(frame, planet_idx, [x, y]),
        );
        draw_aligned_detail_line(
            &mut buffer,
            18,
            bottom_label_width,
            "Building",
            ": ",
            &format_build_queue_summary(planet),
        );
        draw_aligned_detail_line(
            &mut buffer,
            19,
            bottom_label_width,
            "Docked",
            ": ",
            &format_stardock_summary(planet),
        );
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(19));
        Ok(buffer)
    }

    fn render_intel_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        planet_idx: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection_from_snapshots(
            frame.game_data,
            frame.planet_intel_snapshots,
            frame.player.record_index_1_based as u8,
        );
        let world = projection
            .worlds
            .into_iter()
            .find(|world| world.planet_record_index_1_based == planet_idx + 1)
            .ok_or("planet intel detail missing")?;
        let owner_label = known_owner_label(
            world.known_owner_empire_id,
            world.known_owner_empire_name.as_deref(),
            KnownOwnerLabelStyle::Detail,
            world
                .known_owner_empire_id
                .and_then(|id| frame.game_data.empire_campaign_state(id)),
        );
        let info_label_width = aligned_label_width([
            "Coordinates",
            "Planet",
            "Owner",
            "State",
            "Last Viewed/Scouted",
            "Production",
            "Potential Production",
            "Efficiency",
            "Treasury",
            "Armies",
            "Ground Batteries",
            "Space Forces",
            "Intel Tier",
            "Docked",
        ]);

        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "INFO ABOUT A PLANET:");
        draw_aligned_status_line(
            &mut buffer,
            2,
            info_label_width,
            "Coordinates",
            &format_sector_coords_zero_padded(world.coords),
        );
        draw_aligned_status_line(
            &mut buffer,
            3,
            info_label_width,
            "Planet",
            world.known_name.as_deref().unwrap_or("?"),
        );
        draw_aligned_status_line(&mut buffer, 4, info_label_width, "Owner", &owner_label);
        draw_aligned_status_line(&mut buffer, 5, info_label_width, "State", "?");
        let intel_snapshot = frame.planet_intel_snapshots.get(&(planet_idx + 1));
        draw_aligned_status_line(
            &mut buffer,
            6,
            info_label_width,
            "Last Viewed/Scouted",
            &intel_snapshot
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|year| format!("Y{year}"))
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            7,
            info_label_width,
            "Production",
            &world
                .known_current_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            8,
            info_label_width,
            "Potential Production",
            &world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            9,
            info_label_width,
            "Efficiency",
            &intel_efficiency_label(
                world.known_current_production,
                world.known_potential_production,
            ),
        );
        draw_aligned_status_line(
            &mut buffer,
            10,
            info_label_width,
            "Treasury",
            &world
                .known_stored_points
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            12,
            info_label_width,
            "Armies",
            &world
                .known_armies
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            13,
            info_label_width,
            "Ground Batteries",
            &world
                .known_ground_batteries
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            14,
            info_label_width,
            "Space Forces",
            world.known_orbit_summary.as_deref().unwrap_or("?"),
        );
        draw_aligned_status_line(
            &mut buffer,
            15,
            info_label_width,
            "Intel Tier",
            intel_tier_label(intel_snapshot, &world),
        );
        draw_aligned_detail_line(
            &mut buffer,
            17,
            info_label_width,
            "Docked",
            ": ",
            world.known_docked_summary.as_deref().unwrap_or("?"),
        );
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(17));
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }

    pub fn handle_detail_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
}

fn intel_efficiency_label(current: Option<u8>, potential: Option<u16>) -> String {
    match (current, potential) {
        (Some(current), Some(potential)) if potential != 0 => {
            format!(
                "{:.1}%",
                (f64::from(current) / f64::from(potential)) * 100.0
            )
        }
        _ => "?".to_string(),
    }
}

fn format_stardock_summary(planet: &nc_data::Planet) -> String {
    shared_stardock_summary(planet, CompactUnitSummaryStyle::Notation)
}

fn format_build_queue_summary(planet: &nc_data::Planet) -> String {
    shared_build_queue_summary(planet, CompactUnitSummaryStyle::Notation)
}

fn format_owned_orbit_summary(frame: &ScreenFrame<'_>, coords: [u8; 2]) -> String {
    shared_owned_orbit_summary(owned_orbit_presence(
        frame.game_data,
        frame.player.record_index_1_based as u8,
        coords,
    ))
}

fn owned_status_summary(frame: &ScreenFrame<'_>, planet_idx: usize, coords: [u8; 2]) -> String {
    let _ = coords;
    match owned_planet_status(
        frame.game_data,
        frame.player.record_index_1_based as u8,
        planet_idx,
    ) {
        OwnedPlanetStatus::Scorched => "Planet is scorched!".to_string(),
        OwnedPlanetStatus::Homeworld => "Homeworld - fully developed".to_string(),
        OwnedPlanetStatus::StarbasePresent => "Regular planet - starbase present".to_string(),
        OwnedPlanetStatus::FactoriesDestroyed => "Regular planet - industry destroyed".to_string(),
        OwnedPlanetStatus::FactoriesDamaged => "Regular planet - industry damaged".to_string(),
        OwnedPlanetStatus::FactoriesFunctional => "Regular planet - industry intact".to_string(),
    }
}

fn owner_summary(frame: &ScreenFrame<'_>, owner_empire_id: u8) -> String {
    if owner_empire_id == 0 {
        return "Unowned".to_string();
    }

    let idx = owner_empire_id as usize - 1;
    let Some(player) = frame.game_data.players.get(idx) else {
        return format!("Empire #{owner_empire_id}");
    };

    let empire_name = player.empire_name.trim().to_string();
    let legacy_name = player.empire_name.trim().to_string();
    if !empire_name.is_empty() && !legacy_name.starts_with("In Civil Disorder") {
        format!("Empire #{owner_empire_id} ({empire_name})")
    } else if !legacy_name.is_empty() {
        format!("Empire #{owner_empire_id} ({legacy_name})")
    } else {
        format!("Empire #{owner_empire_id}")
    }
}

fn owner_state_summary(frame: &ScreenFrame<'_>, owner_empire_id: u8) -> String {
    match frame.game_data.empire_campaign_state(owner_empire_id) {
        Some(nc_data::CampaignState::Stable) => "Stable".to_string(),
        Some(nc_data::CampaignState::MarginalExistence) => "Marginal Existence".to_string(),
        Some(nc_data::CampaignState::DefectionRisk) => "Defection Risk".to_string(),
        Some(nc_data::CampaignState::Defeated) => "Defeated".to_string(),
        Some(nc_data::CampaignState::CivilDisorder) => "In Civil Disorder".to_string(),
        Some(nc_data::CampaignState::Rogue) => "Rogue".to_string(),
        None if owner_empire_id == 0 => "Unowned".to_string(),
        None => "Unknown".to_string(),
    }
}

fn intel_tier_label<'a>(
    snapshot: Option<&'a PlanetIntelSnapshot>,
    world: &'a nc_data::PlayerStarmapWorld,
) -> &'a str {
    match snapshot.map(|snapshot| snapshot.intel_tier) {
        Some(IntelTier::Owned) => "owned",
        Some(IntelTier::Full) => "full",
        Some(IntelTier::Partial) => "partial",
        Some(IntelTier::Unknown) => "unknown",
        None if world.known_armies.is_some() || world.known_ground_batteries.is_some() => "full",
        None if world.known_name.is_some()
            || world.known_owner_empire_id.is_some()
            || world.known_owner_empire_name.is_some()
            || world.known_potential_production.is_some() =>
        {
            "partial"
        }
        None => "unknown",
    }
}

pub fn parse_planet_coords(input: &str) -> Option<[u8; 2]> {
    let parts = input
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let x = parts[0].parse::<u8>().ok()?;
    let y = parts[1].parse::<u8>().ok()?;
    Some([x, y])
}
