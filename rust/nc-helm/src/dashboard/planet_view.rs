use std::cell::Ref;
use std::collections::BTreeMap;

use nc_data::{
    CampaignState, CompactUnitSummaryStyle, IntelTier, OwnedPlanetStatus, Planet,
    PlanetIntelSnapshot, PlayerStarmapWorld, active_starbase_count_at,
    format_build_queue_summary as shared_build_queue_summary,
    format_owned_orbit_summary as shared_owned_orbit_summary,
    format_stardock_summary as shared_stardock_summary, owned_orbit_presence, owned_planet_status,
    yearly_tax_revenue,
};

use crate::dashboard::app::panel_cache::CachedSectorDetails;
use crate::dashboard::app::state::DashApp;
use crate::dashboard::panels::starmap::cached_projection_for_app;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetailLine {
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectedPlanetDetail {
    pub planet_record_index_1_based: usize,
    pub widget_fields: Vec<DetailLine>,
    pub popup_lines: Vec<DetailLine>,
}

pub(crate) const EMPTY_SECTOR_LABEL: &str = "empty sector";
pub(crate) const PREFERRED_BODY_WIDTH_CAP: usize = 24;
pub(crate) const MAX_BODY_ROWS: usize = 16;
pub(crate) const MIN_BODY_ROWS: usize = 8;

pub(crate) fn selected_planet_detail(app: &DashApp) -> Option<SelectedPlanetDetail> {
    let planet_record_index_1_based = selected_planet_record_index(app);
    if planet_record_index_1_based == 0 {
        return None;
    }
    planet_detail_for_record(app, planet_record_index_1_based)
}

pub(crate) fn planet_detail_for_record(
    app: &DashApp,
    planet_record_index_1_based: usize,
) -> Option<SelectedPlanetDetail> {
    cached_sector_details_for_app(app)
        .details_by_planet_index
        .get(&planet_record_index_1_based)
        .cloned()
}

pub(crate) fn preferred_sector_detail_body_width(app: &DashApp) -> usize {
    cached_sector_details_for_app(app).preferred_body_width
}

pub(crate) fn preferred_sector_detail_body_rows(app: &DashApp) -> usize {
    cached_sector_details_for_app(app).preferred_body_rows
}

fn owned_planet_detail(
    app: &DashApp,
    planet_index_0_based: usize,
    planet: &Planet,
    budget: u32,
) -> SelectedPlanetDetail {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let coords = planet.sector.to_array();
    let present = nc_data::present_production_points(&planet);
    let potential = u16::from(planet.potential_production);
    let tax_rate = app
        .game_data
        .players
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|player| player.tax_rate)
        .unwrap_or(0);
    let owned_since = app
        .owned_planet_years
        .get(&(planet_index_0_based + 1))
        .map(|year| format!("Y{year}"))
        .unwrap_or_else(|| String::from("?"));
    let popup_lines = vec![
        detail_line("Coordinates", coords_label(coords)),
        detail_line("Planet", planet.name.trim().to_string()),
        detail_line(
            "Owner",
            owned_owner_detail_label(&app.game_data, viewer_empire_id),
        ),
        detail_line(
            "State",
            owner_state_detail_label(&app.game_data, viewer_empire_id),
        ),
        detail_line("Owned Since", owned_since),
        detail_line("Production", present.to_string()),
        detail_line("Potential Production", potential.to_string()),
        detail_line(
            "Treasury",
            nc_data::stored_production_points(&planet).to_string(),
        ),
        detail_line("Budget", budget.to_string()),
        detail_line("Efficiency", owned_efficiency_label(present, potential)),
        detail_line(
            "Expected Revenue",
            format!("{} points", yearly_tax_revenue(present, tax_rate)),
        ),
        detail_line(
            "Armies",
            (planet.armies.min(u16::from(u8::MAX)) as u8).to_string(),
        ),
        detail_line(
            "Ground Batteries",
            (planet.ground_batteries.min(u16::from(u8::MAX)) as u8).to_string(),
        ),
        detail_line(
            "Space Forces",
            format_owned_orbit_summary(app, coords, viewer_empire_id),
        ),
        detail_line(
            "Status",
            owned_status_detail_label(app, planet_index_0_based, planet),
        ),
        detail_line("Building", format_build_queue_summary(planet)),
        detail_line("Docked", format_stardock_summary(planet)),
    ];
    let widget_fields = vec![
        widget_field("Planet", planet.name.trim().to_string()),
        widget_field("Owner", String::from("You")),
        widget_field(
            "State",
            owned_status_widget_label(app, planet_index_0_based, planet),
        ),
        widget_field("Production", present.to_string()),
        widget_field("Potential Production", potential.to_string()),
        widget_field(
            "Treasury",
            nc_data::stored_production_points(&planet).to_string(),
        ),
        widget_field("Budget", budget.to_string()),
        widget_field(
            "Armies",
            (planet.armies.min(u16::from(u8::MAX)) as u8).to_string(),
        ),
        widget_field(
            "Ground Batteries",
            (planet.ground_batteries.min(u16::from(u8::MAX)) as u8).to_string(),
        ),
        widget_field(
            "Starbases",
            active_starbase_count_at(&app.game_data, coords).to_string(),
        ),
        widget_field(
            "Orbit",
            format_owned_orbit_summary(app, coords, viewer_empire_id),
        ),
        widget_field("Building", format_build_queue_summary(planet)),
        widget_field("Docked", format_stardock_summary(planet)),
    ];

    SelectedPlanetDetail {
        planet_record_index_1_based: planet_index_0_based + 1,
        widget_fields,
        popup_lines,
    }
}

fn intel_planet_detail(
    app: &DashApp,
    world: &PlayerStarmapWorld,
    snapshot: Option<&PlanetIntelSnapshot>,
) -> SelectedPlanetDetail {
    let popup_lines = vec![
        detail_line("Coordinates", coords_label(world.coords)),
        detail_line(
            "Planet",
            world
                .known_name
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        detail_line("Owner", intel_owner_detail_label(&app.game_data, world)),
        detail_line("State", String::from("?")),
        detail_line(
            "Last Viewed/Scouted",
            snapshot
                .and_then(|row| row.last_intel_year)
                .map(|year| format!("Y{year}"))
                .unwrap_or_else(|| String::from("?")),
        ),
        detail_line("Production", known_u8(world.known_current_production)),
        detail_line(
            "Potential Production",
            known_u16(world.known_potential_production),
        ),
        detail_line(
            "Efficiency",
            efficiency_label(
                world.known_current_production,
                world.known_potential_production,
            ),
        ),
        detail_line("Treasury", known_u16(world.known_stored_points)),
        detail_line("Armies", known_u8(world.known_armies)),
        detail_line("Ground Batteries", known_u8(world.known_ground_batteries)),
        detail_line(
            "Space Forces",
            world
                .known_orbit_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        detail_line("Intel Tier", intel_tier_label(snapshot, world).to_string()),
        detail_line(
            "Docked",
            world
                .known_docked_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
    ];
    let intel_year = snapshot
        .and_then(|row| row.last_intel_year)
        .map(|year| format!("Y{year}"))
        .unwrap_or_else(|| String::from("Y?"));
    let widget_fields = vec![
        widget_field(
            "Planet",
            world
                .known_name
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        widget_field("Owner", intel_owner_widget_label(&app.game_data, world)),
        widget_field("State", String::from("?")),
        widget_field(
            "Intel",
            format!("{intel_year} {}", intel_tier_code(snapshot, world)),
        ),
        widget_field("Production", known_u8(world.known_current_production)),
        widget_field(
            "Potential Production",
            known_u16(world.known_potential_production),
        ),
        widget_field("Treasury", known_u16(world.known_stored_points)),
        widget_field("Armies", known_u8(world.known_armies)),
        widget_field("Ground Batteries", known_u8(world.known_ground_batteries)),
        widget_field("Starbases", known_u8(world.known_starbase_count)),
        widget_field(
            "Orbit",
            world
                .known_orbit_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        widget_field(
            "Docked",
            world
                .known_docked_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
    ];

    SelectedPlanetDetail {
        planet_record_index_1_based: world.planet_record_index_1_based,
        widget_fields,
        popup_lines,
    }
}

fn detail_line(label: &'static str, value: String) -> DetailLine {
    DetailLine { label, value }
}

fn widget_field(label: &'static str, value: String) -> DetailLine {
    DetailLine { label, value }
}

pub(crate) fn widget_label_for_width(field: &DetailLine, body_width: usize) -> &'static str {
    let Some(variants) = widget_label_variants(field.label) else {
        return field.label;
    };

    variants
        .iter()
        .copied()
        .find(|label| label.chars().count() + 2 + field.value.chars().count() <= body_width)
        .unwrap_or_else(|| variants.last().copied().unwrap_or(field.label))
}

fn preferred_widget_field_width(field: &DetailLine) -> usize {
    widget_label_variants(field.label)
        .and_then(|variants| variants.first().copied())
        .unwrap_or(field.label)
        .chars()
        .count()
        + 2
        + field.value.chars().count()
}

pub(crate) fn rendered_widget_lines(
    rows: &[DetailLine],
    body_width: usize,
    max_rows: usize,
) -> Vec<String> {
    if max_rows == 0 {
        return Vec::new();
    }

    let mut indexed = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            (
                row_priority(row.label),
                idx,
                render_widget_field_lines(row, body_width),
            )
        })
        .collect::<Vec<_>>();
    indexed.sort_by_key(|(priority, idx, _)| (*priority, *idx));

    let mut used_rows = 0usize;
    let mut kept = indexed
        .into_iter()
        .filter_map(|(_, idx, lines)| {
            if used_rows + lines.len() > max_rows {
                return None;
            }
            used_rows += lines.len();
            Some((idx, lines))
        })
        .collect::<Vec<_>>();
    kept.sort_by_key(|(idx, _)| *idx);
    kept.into_iter().flat_map(|(_, lines)| lines).collect()
}

fn render_widget_field_lines(field: &DetailLine, body_width: usize) -> Vec<String> {
    let label = widget_label_for_width(field, body_width);
    let prefix = format!("{label}: ");

    if !field_is_wrappable(field.label) {
        return vec![format!("{prefix}{}", field.value)];
    }

    wrap_field_value_lines(&prefix, &field.value, body_width)
}

fn wrap_field_value_lines(prefix: &str, value: &str, body_width: usize) -> Vec<String> {
    let prefix_width = prefix.chars().count();
    let available = body_width.saturating_sub(prefix_width);
    if available == 0 {
        return vec![prefix.trim_end().to_string()];
    }

    let wrapped = wrap_tokens(value, available);
    let continuation = " ".repeat(prefix_width);

    wrapped
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                format!("{prefix}{line}")
            } else {
                format!("{continuation}{line}")
            }
        })
        .collect()
}

fn wrap_tokens(value: &str, max_width: usize) -> Vec<String> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for token in tokens {
        let token_width = token.chars().count();
        if current.is_empty() {
            current.push_str(token);
            continue;
        }

        let next_width = current.chars().count() + 1 + token_width;
        if next_width <= max_width {
            current.push(' ');
            current.push_str(token);
        } else {
            lines.push(current);
            current = token.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn field_is_wrappable(label: &str) -> bool {
    matches!(label, "Building" | "Docked")
}

fn row_priority(label: &str) -> usize {
    match label {
        "Planet" => 0,
        "Owner" => 1,
        "State" => 2,
        "Intel" => 3,
        "Production" => 4,
        "Potential Production" => 5,
        "Treasury" => 6,
        "Armies" => 7,
        "Ground Batteries" => 8,
        "Starbases" => 9,
        "Building" => 10,
        "Docked" => 11,
        "Orbit" => 12,
        _ => 13,
    }
}

fn widget_label_variants(label: &'static str) -> Option<&'static [&'static str]> {
    Some(match label {
        "Potential Production" => &["Pot Prod"],
        "Production" => &["Production", "Prod"],
        "Treasury" => &["Treasury", "Trsry"],
        "Budget" => &["Budget", "Bdgt"],
        "Ground Batteries" => &["Grnd Batt", "GBs"],
        "Armies" => &["Armies", "ARs"],
        "Starbases" => &["Starbases", "SBs"],
        _ => return None,
    })
}

fn coords_label(coords: [u8; 2]) -> String {
    format!("({:02},{:02})", coords[0], coords[1])
}

fn cached_sector_details_for_app(app: &DashApp) -> Ref<'_, CachedSectorDetails> {
    let revision = app.game_data_revision;
    let player = app.player_record_index_1_based;
    let needs_update = !app
        .sector_detail_cache
        .borrow()
        .as_ref()
        .is_some_and(|cache| cache.revision == revision && cache.player == player);

    if needs_update {
        *app.sector_detail_cache.borrow_mut() = Some(build_cached_sector_details(app));
    }

    Ref::map(app.sector_detail_cache.borrow(), |opt| {
        opt.as_ref()
            .expect("sector detail cache should be populated")
    })
}

fn build_cached_sector_details(app: &DashApp) -> CachedSectorDetails {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let budget_by_planet_index = owned_planet_budget_map(app);
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = cached_projection_for_app(app);
    let details_by_planet_index = projection
        .projection
        .worlds
        .iter()
        .filter_map(|world| {
            let planet_index_0_based = world.planet_record_index_1_based.checked_sub(1)?;
            let planet = app.game_data.planets.get(planet_index_0_based)?;
            let detail = if planet.owner.0 == viewer_empire_id {
                let budget = budget_by_planet_index
                    .get(&world.planet_record_index_1_based)
                    .copied()
                    .unwrap_or_else(|| {
                        nc_data::stored_production_points(&planet)
                            .min(u32::from(nc_data::present_production_points(&planet)))
                    });
                owned_planet_detail(app, planet_index_0_based, planet, budget)
            } else {
                intel_planet_detail(
                    app,
                    world,
                    snapshot_map.get(&world.planet_record_index_1_based),
                )
            };
            Some((detail.planet_record_index_1_based, detail))
        })
        .collect::<BTreeMap<_, _>>();
    let preferred_body_width = details_by_planet_index
        .values()
        .flat_map(|detail| detail.widget_fields.iter())
        .map(preferred_widget_field_width)
        .max()
        .unwrap_or_else(|| EMPTY_SECTOR_LABEL.chars().count())
        .max(EMPTY_SECTOR_LABEL.chars().count())
        .min(PREFERRED_BODY_WIDTH_CAP);
    let preferred_body_rows = details_by_planet_index
        .values()
        .map(|detail| {
            rendered_widget_lines(&detail.widget_fields, preferred_body_width, MAX_BODY_ROWS).len()
        })
        .max()
        .unwrap_or(1)
        .clamp(MIN_BODY_ROWS, MAX_BODY_ROWS);

    CachedSectorDetails {
        revision: app.game_data_revision,
        player: app.player_record_index_1_based,
        details_by_planet_index,
        preferred_body_width,
        preferred_body_rows,
    }
}

fn owned_planet_budget_map(app: &DashApp) -> BTreeMap<usize, u32> {
    app.game_data
        .empire_planet_economy_rows(app.player_record_index_1_based)
        .into_iter()
        .map(|row| {
            (
                row.planet_record_index_1_based,
                u32::from(row.build_capacity).min(row.stored_production_points),
            )
        })
        .collect()
}

fn selected_planet_record_index(app: &DashApp) -> usize {
    let cache = cached_projection_for_app(app);
    cache
        .world_index
        .get(&[app.crosshair_x, app.crosshair_y])
        .map(|&i| cache.projection.worlds[i].planet_record_index_1_based)
        .unwrap_or(0)
}

fn known_u8(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}

fn known_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}

fn efficiency_label(current: Option<u8>, potential: Option<u16>) -> String {
    match (current, potential) {
        (Some(current), Some(potential)) if potential != 0 => {
            format!(
                "{:.1}%",
                (f64::from(current) / f64::from(potential)) * 100.0
            )
        }
        _ => String::from("?"),
    }
}

fn owned_efficiency_label(current: u16, potential: u16) -> String {
    if potential == 0 {
        String::from("?")
    } else {
        format!(
            "{:.1}%",
            (f64::from(current) / f64::from(potential)) * 100.0
        )
    }
}

fn owned_owner_detail_label(game_data: &nc_data::CoreGameData, owner_empire_id: u8) -> String {
    if owner_empire_id == 0 {
        return String::from("Unowned");
    }

    let Some(player) = game_data
        .players
        .get(owner_empire_id.saturating_sub(1) as usize)
    else {
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

fn owner_state_detail_label(game_data: &nc_data::CoreGameData, owner_empire_id: u8) -> String {
    match game_data.empire_campaign_state(owner_empire_id) {
        Some(CampaignState::Stable) => String::from("Stable"),
        Some(CampaignState::MarginalExistence) => String::from("Marginal Existence"),
        Some(CampaignState::DefectionRisk) => String::from("Defection Risk"),
        Some(CampaignState::Defeated) => String::from("Defeated"),
        Some(CampaignState::CivilDisorder) => String::from("In Civil Disorder"),
        Some(CampaignState::Rogue) => String::from("Rogue"),
        None if owner_empire_id == 0 => String::from("Unowned"),
        None => String::from("Unknown"),
    }
}

fn intel_owner_detail_label(
    game_data: &nc_data::CoreGameData,
    world: &PlayerStarmapWorld,
) -> String {
    match world.known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => {
            if game_data.empire_campaign_state(owner) == Some(CampaignState::CivilDisorder) {
                String::from("In Civil Disorder")
            } else {
                world
                    .known_owner_empire_name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Empire #{owner}"))
            }
        }
        None => String::from("?"),
    }
}

fn intel_owner_widget_label(
    game_data: &nc_data::CoreGameData,
    world: &PlayerStarmapWorld,
) -> String {
    match world.known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => {
            if game_data.empire_campaign_state(owner) == Some(CampaignState::CivilDisorder) {
                String::from("ICD")
            } else {
                format!("#{owner}")
            }
        }
        None => String::from("?"),
    }
}

fn intel_tier_label<'a>(
    snapshot: Option<&'a PlanetIntelSnapshot>,
    world: &'a PlayerStarmapWorld,
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

fn intel_tier_code(
    snapshot: Option<&PlanetIntelSnapshot>,
    world: &PlayerStarmapWorld,
) -> &'static str {
    match intel_tier_label(snapshot, world) {
        "owned" => "own",
        "full" => "full",
        "partial" => "part",
        _ => "unk",
    }
}

fn format_stardock_summary(planet: &Planet) -> String {
    shared_stardock_summary(planet, CompactUnitSummaryStyle::JoinedCodes)
}

fn format_build_queue_summary(planet: &Planet) -> String {
    shared_build_queue_summary(planet, CompactUnitSummaryStyle::JoinedCodes)
}

fn format_owned_orbit_summary(app: &DashApp, coords: [u8; 2], viewer_empire_id: u8) -> String {
    shared_owned_orbit_summary(owned_orbit_presence(
        &app.game_data,
        viewer_empire_id,
        coords,
    ))
}

fn owned_status_widget_label(
    app: &DashApp,
    planet_index_0_based: usize,
    _planet: &Planet,
) -> String {
    match owned_planet_status(
        &app.game_data,
        app.player_record_index_1_based as u8,
        planet_index_0_based,
        &app.planet_scorch_orders,
    ) {
        OwnedPlanetStatus::Scorched => String::from("Scorched"),
        OwnedPlanetStatus::Homeworld => String::from("Homeworld"),
        OwnedPlanetStatus::StarbasePresent => String::from("Starbase"),
        OwnedPlanetStatus::FactoriesDestroyed => String::from("Destroyed"),
        OwnedPlanetStatus::FactoriesDamaged => String::from("Damaged"),
        OwnedPlanetStatus::FactoriesFunctional => String::from("Normal"),
    }
}

fn owned_status_detail_label(
    app: &DashApp,
    planet_index_0_based: usize,
    _planet: &Planet,
) -> String {
    match owned_planet_status(
        &app.game_data,
        app.player_record_index_1_based as u8,
        planet_index_0_based,
        &app.planet_scorch_orders,
    ) {
        OwnedPlanetStatus::Scorched => String::from("Planet is scorched!"),
        OwnedPlanetStatus::Homeworld => String::from("Homeworld - fully developed"),
        OwnedPlanetStatus::StarbasePresent => String::from("Regular planet - starbase present"),
        OwnedPlanetStatus::FactoriesDestroyed => {
            String::from("Regular planet - industry destroyed")
        }
        OwnedPlanetStatus::FactoriesDamaged => String::from("Regular planet - industry damaged"),
        OwnedPlanetStatus::FactoriesFunctional => String::from("Regular planet - industry intact"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_data::{GameStateBuilder, Planet};

    #[test]
    fn widget_rows_use_split_owned_world_fields() {
        let mut app = crate::dashboard::app::state::DashApp::new_for_tests(
            std::path::PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            std::collections::BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        );
        app.crosshair_x = app.game_data.planets[0].sector.to_array()[0];
        app.crosshair_y = app.game_data.planets[0].sector.to_array()[1];

        let detail = selected_planet_detail(&app).expect("selected world");
        assert_eq!(
            detail
                .widget_fields
                .iter()
                .map(|field| field.label)
                .collect::<Vec<_>>(),
            vec![
                "Planet",
                "Owner",
                "State",
                "Production",
                "Potential Production",
                "Treasury",
                "Budget",
                "Armies",
                "Ground Batteries",
                "Starbases",
                "Orbit",
                "Building",
                "Docked",
            ]
        );
    }

    #[test]
    fn widget_field_format_prefers_fuller_labels_when_they_fit() {
        let current = DetailLine {
            label: "Production",
            value: String::from("9"),
        };
        let potential = DetailLine {
            label: "Potential Production",
            value: String::from("10"),
        };
        let stored = DetailLine {
            label: "Treasury",
            value: String::from("125"),
        };
        let batteries = DetailLine {
            label: "Ground Batteries",
            value: String::from("4"),
        };

        assert_eq!(widget_label_for_width(&current, 19), "Production");
        assert_eq!(widget_label_for_width(&potential, 19), "Pot Prod");
        assert_eq!(widget_label_for_width(&stored, 19), "Treasury");
        assert_eq!(widget_label_for_width(&batteries, 19), "Grnd Batt");
        assert_eq!(widget_label_for_width(&current, 14), "Production");
        assert_eq!(widget_label_for_width(&stored, 14), "Treasury");
        assert_eq!(widget_label_for_width(&batteries, 14), "Grnd Batt");
        assert_eq!(widget_label_for_width(&batteries, 6), "GBs");
    }

    #[test]
    fn compact_unit_summary_uses_tui_style_without_dashes() {
        let mut planet = Planet::default();
        planet.build_queue[0].kind = nc_data::ProductionItemKind::from_raw(3);
        planet.build_queue[0].points_owed = 90;
        planet.stardock[1] = 5;

        assert_eq!(format_build_queue_summary(&planet), "2BB");
        assert_eq!(format_stardock_summary(&planet), "5CA");
    }

    #[test]
    fn preferred_sector_detail_metrics_are_cached() {
        let app = crate::dashboard::app::state::DashApp::new_for_tests(
            std::path::PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            std::collections::BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        );

        let width = preferred_sector_detail_body_width(&app);
        let rows = preferred_sector_detail_body_rows(&app);
        let cache = cached_sector_details_for_app(&app);

        assert_eq!(cache.preferred_body_width, width);
        assert_eq!(cache.preferred_body_rows, rows);
        assert!(!cache.details_by_planet_index.is_empty());
    }

    #[test]
    fn sector_detail_cache_invalidates_when_game_revision_changes() {
        let mut app = crate::dashboard::app::state::DashApp::new_for_tests(
            std::path::PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            std::collections::BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        );
        let before = preferred_sector_detail_body_width(&app);

        nc_data::set_planet_name(&mut app.game_data.planets[0], "Frontier Bastion Prime");
        app.game_data_revision += 1;

        let after = preferred_sector_detail_body_width(&app);

        assert!(after >= before);
        assert_eq!(
            cached_sector_details_for_app(&app).revision,
            app.game_data_revision
        );
    }
}
