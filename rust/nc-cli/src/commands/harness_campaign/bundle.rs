use std::fs;

use nc_data::{
    CampaignRuntimeState, Fleet, PlayerStarmapProjection, PlayerStarmapWorld, ProductionItemKind,
    build_player_starmap_projection_from_snapshots, order_summary, present_production_points,
    ship_composition_summary, stardock_slot_kind, stored_production_points, troop_transports,
};
use nc_engine::{FleetEtaEstimate, estimate_fleet_eta_to_destination};

use super::{
    BundleProfile, CampaignManifest, PlayerAssignment, PlayerTurnStatus, kdl_escape,
    load_snapshots_for_viewer, player_bundle_dir, player_notes_path, player_status_path,
    player_turn_path, player_workspace_dir,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisiblePlanetHint {
    record_index_1_based: usize,
    coords: [u8; 2],
    name: String,
    owner_empire_id: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FleetCapabilities {
    has_combat: bool,
    has_scout: bool,
    has_etac: bool,
    has_loaded_troops: bool,
}

pub(super) fn ensure_player_bundle(
    manifest: &CampaignManifest,
    state: &CampaignRuntimeState,
    assignment: &PlayerAssignment,
    current_status: Option<&PlayerTurnStatus>,
    turn_index_1_based: u16,
    year: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_dir = player_workspace_dir(manifest, assignment.record_index_1_based);
    let bundle_dir = player_bundle_dir(
        manifest,
        assignment.record_index_1_based,
        turn_index_1_based,
    );
    fs::create_dir_all(&player_dir)?;
    fs::create_dir_all(&bundle_dir)?;

    let snapshots =
        load_snapshots_for_viewer(&manifest.campaign_dir, assignment.record_index_1_based)?;
    let projection = build_player_starmap_projection_from_snapshots(
        &state.game_data,
        &snapshots,
        assignment.record_index_1_based as u8,
    );

    fs::write(
        bundle_dir.join("starmap.txt"),
        projection.render_ascii_export(),
    )?;
    fs::write(
        bundle_dir.join("starmap.csv"),
        projection.render_csv_export(),
    )?;
    fs::write(
        bundle_dir.join("starmap-DETAILS.csv"),
        projection.render_csv_details_export(),
    )?;

    let readme = render_player_bundle_readme(
        manifest,
        state,
        assignment,
        &projection,
        current_status,
        turn_index_1_based,
        year,
    );
    fs::write(bundle_dir.join("README.md"), readme)?;
    sync_hidden_llm_spatial_bundle(
        manifest,
        state,
        assignment,
        &projection,
        turn_index_1_based,
        year,
        &bundle_dir,
    )?;
    Ok(())
}

fn sync_hidden_llm_spatial_bundle(
    manifest: &CampaignManifest,
    state: &CampaignRuntimeState,
    assignment: &PlayerAssignment,
    projection: &PlayerStarmapProjection,
    turn_index_1_based: u16,
    year: u16,
    bundle_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_dir = bundle_dir.join(".llm");
    if manifest.bundle_profile == BundleProfile::Llm {
        fs::create_dir_all(&llm_dir)?;
        fs::write(
            llm_dir.join("spatial.kdl"),
            render_llm_spatial_kdl(state, assignment, projection, turn_index_1_based, year),
        )?;
    } else if llm_dir.exists() {
        fs::remove_dir_all(llm_dir)?;
    }
    Ok(())
}

fn render_player_bundle_readme(
    manifest: &CampaignManifest,
    state: &CampaignRuntimeState,
    assignment: &PlayerAssignment,
    projection: &PlayerStarmapProjection,
    current_status: Option<&PlayerTurnStatus>,
    turn_index_1_based: u16,
    year: u16,
) -> String {
    let player = &state.game_data.players[assignment.record_index_1_based - 1];
    let empire_name = player.empire_name.trim().to_string();
    let handle = player.handle.trim().to_string();
    let economy = state
        .game_data
        .empire_economy_summary(assignment.record_index_1_based);
    let active_duty = state
        .game_data
        .empire_active_duty_summary(assignment.record_index_1_based);
    let stardock = state
        .game_data
        .empire_stardock_summary(assignment.record_index_1_based);
    let visible_planets =
        visible_planet_hints_from_projection(projection, assignment.record_index_1_based as u8);

    let incoming_messages = state
        .queued_mail
        .iter()
        .filter(|mail| {
            mail.is_visible_to_recipient(assignment.record_index_1_based as u8)
                && mail.year.saturating_add(1) == year
        })
        .collect::<Vec<_>>();

    let player_dir = player_workspace_dir(manifest, assignment.record_index_1_based);
    let status_path = player_status_path(
        manifest,
        assignment.record_index_1_based,
        turn_index_1_based,
    );
    let turn_path = player_turn_path(
        manifest,
        assignment.record_index_1_based,
        turn_index_1_based,
    );
    let notes_path = player_notes_path(
        manifest,
        assignment.record_index_1_based,
        turn_index_1_based,
    );

    let mut out = String::new();
    out.push_str("# EC Bot Turn Bundle\n\n");
    out.push_str(&format!("- game_id: `{}`\n", manifest.game_id));
    out.push_str(&format!(
        "- player: `{}`\n",
        assignment.record_index_1_based
    ));
    out.push_str(&format!("- handle: `{}`\n", handle));
    out.push_str(&format!("- empire: `{}`\n", empire_name));
    out.push_str(&format!("- turn: `{}`\n", turn_index_1_based));
    out.push_str(&format!("- year: `{}`\n", year));
    out.push_str(&format!("- doctrine: `{}`\n", assignment.doctrine));
    out.push_str(&format!("- status_file: `{}`\n", status_path.display()));
    out.push_str(&format!("- turn_file: `{}`\n", turn_path.display()));
    out.push_str(&format!("- notes_file: `{}`\n", notes_path.display()));
    out.push_str(&format!("- workspace_dir: `{}`\n\n", player_dir.display()));

    out.push_str(
        "Use only this bundle, the player manuals, and your own prior notes. Do not inspect hidden state.\n\n",
    );

    out.push_str("## Current Turn Status\n\n");
    if let Some(status) = current_status {
        out.push_str(&format!("- state: `{}`\n", status.state.as_str()));
        if let Some(error) = &status.error {
            out.push_str(&format!("- validation_error: `{}`\n", error));
            out.push_str(
                "- fix only the cited issue, keep the rest of the turn stable if possible\n",
            );
        }
    } else {
        out.push_str("- state: `ready`\n");
    }
    out.push('\n');

    out.push_str("## Economy Summary\n\n");
    out.push_str(&format!(
        "- planets_owned: `{}`\n- present_production: `{}`\n- potential_production: `{}`\n- available_points: `{}`\n- efficiency_percent: `{}`\n- tax_rate: `{}`\n- rank_by_planets: `{}`\n- rank_by_present_production: `{}`\n\n",
        economy.owned_planets,
        economy.present_production,
        economy.potential_production,
        economy.total_available_points,
        economy.efficiency_percent,
        economy.tax_rate,
        economy.rank_by_planets,
        economy.rank_by_present_production
    ));

    out.push_str("## Active Duty Summary\n\n");
    out.push_str(&format!(
        "- battleships: `{}`\n- cruisers: `{}`\n- destroyers: `{}`\n- scouts: `{}`\n- transports: `{}`\n- etacs: `{}`\n- starbases: `{}`\n- armies: `{}`\n- ground_batteries: `{}`\n\n",
        active_duty.battleships,
        active_duty.cruisers,
        active_duty.destroyers,
        active_duty.scouts,
        active_duty.transports,
        active_duty.etacs,
        active_duty.starbases,
        active_duty.armies,
        active_duty.ground_batteries
    ));

    out.push_str("## Stardock Summary\n\n");
    out.push_str(&format!(
        "- battleships: `{}`\n- cruisers: `{}`\n- destroyers: `{}`\n- scouts: `{}`\n- transports: `{}`\n- etacs: `{}`\n- starbases: `{}`\n- armies: `{}`\n- ground_batteries: `{}`\n\n",
        stardock.battleships,
        stardock.cruisers,
        stardock.destroyers,
        stardock.scouts,
        stardock.transports,
        stardock.etacs,
        stardock.starbases,
        stardock.armies,
        stardock.ground_batteries
    ));

    out.push_str("## Diplomacy\n\n");
    for other in 1..=state.game_data.conquest.player_count as usize {
        if other == assignment.record_index_1_based {
            continue;
        }
        let relation = state
            .game_data
            .stored_diplomatic_relation(assignment.record_index_1_based as u8, other as u8)
            .map(|value| match value {
                nc_data::DiplomaticRelation::Neutral => "neutral",
                nc_data::DiplomaticRelation::Enemy => "enemy",
                nc_data::DiplomaticRelation::Unknown(_) => "unknown",
            })
            .unwrap_or("unknown");
        let other_empire = state.game_data.players[other - 1]
            .empire_name
            .trim()
            .to_string();
        out.push_str(&format!(
            "- empire `{other}` (`{other_empire}`): `{relation}`\n"
        ));
    }
    out.push('\n');

    out.push_str("## Owned Planets\n\n");
    for planet in state
        .game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 as usize == assignment.record_index_1_based)
    {
        let [x, y] = planet.sector.to_array();
        let present = Some(present_production_points(planet))
            .map(|value| value.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());
        out.push_str(&format!(
            "- `{}` at `({x},{y})`: potential `{}`, present `{}`, stored `{}`, armies `{}`, batteries `{}`\n",
            planet.name.clone(),
            u16::from(planet.potential_production),
            present,
            stored_production_points(planet),
            u8::try_from(planet.armies).unwrap_or(u8::MAX),
            u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)
        ));
        let build_slots = (0..10)
            .filter_map(|slot| {
                let count = planet.build_queue[slot].points_owed;
                let kind = planet.build_queue[slot].kind.to_raw();
                (count > 0 || kind > 0).then_some(format!(
                    "slot {}: points={} kind_raw={}",
                    slot + 1,
                    count,
                    kind
                ))
            })
            .collect::<Vec<_>>();
        if !build_slots.is_empty() {
            out.push_str("  - build_queue:\n");
            for slot in build_slots {
                out.push_str(&format!("    - {slot}\n"));
            }
        }
        let docked = (0..nc_data::STARDOCK_SLOT_COUNT)
            .filter_map(|slot| {
                let count = u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX);
                (count > 0).then_some(format!(
                    "slot {}: {} x {}",
                    slot + 1,
                    count,
                    production_kind_label(stardock_slot_kind(planet, slot))
                ))
            })
            .collect::<Vec<_>>();
        if !docked.is_empty() {
            out.push_str("  - stardock:\n");
            for line in docked {
                out.push_str(&format!("    - {line}\n"));
            }
        }
    }
    out.push('\n');

    out.push_str("## Owned Fleets\n\n");
    for (idx, fleet) in state.game_data.fleets.iter().enumerate() {
        if fleet.owner.0 as usize != assignment.record_index_1_based {
            continue;
        }
        let [x, y] = fleet.sector.to_array();
        out.push_str(&format!(
            "- record `{}` / fleet_id `{}` at `({x},{y})`: ships `{}` speed `{}` / max `{}` roe `{}` order `{}`\n",
            idx + 1,
            fleet.record.0,
            ship_composition_summary(fleet),
            fleet.speed,
            fleet.max_speed,
            fleet.rules_of_engagement,
            order_summary(fleet)
        ));
    }
    out.push('\n');

    out.push_str("## Legal Action Hints\n\n");
    out.push_str(
        "Use these as hard guardrails. If an order family is listed as unavailable, do not submit it this turn.\n\n",
    );
    for (idx, fleet) in state.game_data.fleets.iter().enumerate() {
        if fleet.owner.0 as usize != assignment.record_index_1_based {
            continue;
        }
        let [x, y] = fleet.sector.to_array();
        let capabilities = fleet_capabilities(fleet);
        out.push_str(&format!(
            "- fleet `{}` / fleet_id `{}` at `({x},{y})`\n",
            idx + 1,
            fleet.record.0
        ));
        out.push_str("  - always safe families: `hold`, `move`, `seek_home`\n");

        let view_targets = targets_for_family(
            &visible_planets,
            capabilities,
            assignment.record_index_1_based as u8,
            "view",
        );
        if view_targets.is_empty() {
            out.push_str("  - `view`: unavailable, no visible planet targets in this bundle\n");
        } else {
            out.push_str(&format!(
                "  - `view`: visible planet targets {}\n",
                format_target_list(&view_targets)
            ));
        }

        if capabilities.has_scout {
            out.push_str(
                "  - `scout_sector`: legal, but still use only player-visible reasoning\n",
            );
            let scout_targets = targets_for_family(
                &visible_planets,
                capabilities,
                assignment.record_index_1_based as u8,
                "scout_system",
            );
            if scout_targets.is_empty() {
                out.push_str(
                    "  - `scout_system`: unavailable, no visible planet targets in this bundle\n",
                );
            } else {
                out.push_str(&format!(
                    "  - `scout_system`: visible planet targets {}\n",
                    format_target_list(&scout_targets)
                ));
            }
        } else {
            out.push_str(
                "  - `scout_sector` / `scout_system`: unavailable, no scout ships in this fleet\n",
            );
        }

        if capabilities.has_etac {
            let colonize_targets = targets_for_family(
                &visible_planets,
                capabilities,
                assignment.record_index_1_based as u8,
                "colonize",
            );
            if colonize_targets.is_empty() {
                out.push_str(
                    "  - `colonize`: unavailable, no visible unowned planet targets this turn\n",
                );
            } else {
                out.push_str(&format!(
                    "  - `colonize`: visible unowned planet targets {}\n",
                    format_target_list(&colonize_targets)
                ));
            }
        } else {
            out.push_str("  - `colonize`: unavailable, no ETAC ships in this fleet\n");
        }

        if capabilities.has_combat {
            let blockade_targets = targets_for_family(
                &visible_planets,
                capabilities,
                assignment.record_index_1_based as u8,
                "guard_blockade",
            );
            if blockade_targets.is_empty() {
                out.push_str(
                    "  - `guard_blockade`: unavailable, no visible planet targets in this bundle\n",
                );
            } else {
                out.push_str(&format!(
                    "  - `guard_blockade`: visible planet targets {}\n",
                    format_target_list(&blockade_targets)
                ));
            }

            let bombard_targets = targets_for_family(
                &visible_planets,
                capabilities,
                assignment.record_index_1_based as u8,
                "bombard",
            );
            if bombard_targets.is_empty() {
                out.push_str(
                    "  - `bombard`: unavailable, no visible foreign planet targets this turn\n",
                );
            } else {
                out.push_str(&format!(
                    "  - `bombard`: visible foreign planet targets {}\n",
                    format_target_list(&bombard_targets)
                ));
            }
        } else {
            out.push_str(
                "  - `guard_blockade` / `bombard`: unavailable, no combat ships in this fleet\n",
            );
        }

        if capabilities.has_combat && capabilities.has_loaded_troops {
            let invade_targets = targets_for_family(
                &visible_planets,
                capabilities,
                assignment.record_index_1_based as u8,
                "invade",
            );
            if invade_targets.is_empty() {
                out.push_str(
                    "  - `invade` / `blitz`: unavailable, no visible foreign planet targets this turn\n",
                );
            } else {
                out.push_str(&format!(
                    "  - `invade` / `blitz`: visible foreign planet targets {}\n",
                    format_target_list(&invade_targets)
                ));
            }
        } else if !capabilities.has_loaded_troops {
            out.push_str(
                "  - `invade` / `blitz`: unavailable, no loaded troop transports in this fleet\n",
            );
        } else {
            out.push_str("  - `invade` / `blitz`: unavailable, no combat ships in this fleet\n");
        }
    }
    out.push('\n');

    out.push_str("## Mandatory Pre-Submit Checks\n\n");
    out.push_str("- Every `colonize`, `view`, `guard_blockade`, `bombard`, `invade`, or `blitz` target must appear in the legal action hints above.\n");
    out.push_str("- Do not use `scout_sector` or `scout_system` unless the fleet actually has scout ships.\n");
    out.push_str("- Do not use `colonize` unless the fleet actually has ETAC ships.\n");
    out.push_str(
        "- Do not use `invade` or `blitz` unless the fleet has loaded troop transports.\n",
    );
    out.push_str("- If a target is not clearly legal from this bundle, prefer `hold`, `move`, `seek_home`, or a message/diplomacy action instead.\n\n");

    out.push_str("## Incoming Player Mail\n\n");
    if incoming_messages.is_empty() {
        out.push_str("- none from the immediately completed turn\n");
    } else {
        for mail in incoming_messages {
            let sender_name = state.game_data.players[mail.sender_empire_id as usize - 1]
                .empire_name
                .trim()
                .to_string();
            out.push_str(&format!(
                "- from empire `{}` (`{}`), year `{}`\n",
                mail.sender_empire_id, sender_name, mail.year
            ));
            if !mail.subject.trim().is_empty() {
                out.push_str(&format!("  - subject: `{}`\n", mail.subject.trim()));
            }
            out.push_str(&format!("  - body: `{}`\n", mail.body.trim()));
        }
    }
    out.push('\n');

    out.push_str("## Review Flags\n\n");
    out.push_str(&format!(
        "- reports_pending_flag: `{}`\n- messages_pending_flag: `{}`\n\n",
        player.messages_last as u8, player.messages_first as u8
    ));

    out.push_str("## Files In This Bundle\n\n");
    out.push_str("- `README.md`: this summary\n");
    out.push_str("- `starmap.txt`: player-visible map projection\n");
    out.push_str("- `starmap.csv`: map grid export\n");
    out.push_str("- `starmap-DETAILS.csv`: known world details export\n");
    out
}

fn render_llm_spatial_kdl(
    state: &CampaignRuntimeState,
    assignment: &PlayerAssignment,
    projection: &PlayerStarmapProjection,
    turn_index_1_based: u16,
    year: u16,
) -> String {
    let viewer = assignment.record_index_1_based as u8;
    let visible_planets = visible_planet_hints_from_projection(projection, viewer);
    let mut worlds = projection.worlds.iter().collect::<Vec<_>>();
    worlds.sort_by_key(|world| {
        (
            world.coords[1],
            world.coords[0],
            world.planet_record_index_1_based,
        )
    });

    let mut out = String::new();
    out.push_str(&format!(
        "bundle-spatial player={} turn={} year={} map_width={} map_height={}\n",
        assignment.record_index_1_based,
        turn_index_1_based,
        year,
        projection.map_width,
        projection.map_height
    ));

    for world in worlds {
        out.push_str(&render_world_node(viewer, world));
    }

    for (idx, fleet) in state.game_data.fleets.iter().enumerate() {
        if fleet.owner.0 as usize != assignment.record_index_1_based {
            continue;
        }
        let [x, y] = fleet.sector.to_array();
        let capabilities = fleet_capabilities(fleet);
        out.push_str(&format!(
            "fleet record={} fleet_id={} x={} y={} speed={} max_speed={} roe={} order=\"{}\" order_label=\"{}\" {{\n",
            idx + 1,
            fleet.record.0,
            x,
            y,
            fleet.speed,
            fleet.max_speed,
            fleet.rules_of_engagement,
            kdl_escape(fleet.order.as_str()),
            kdl_escape(fleet.order.display_label())
        ));
        for family in non_target_families(capabilities) {
            out.push_str(&format!("  non_target_family \"{}\"\n", kdl_escape(family)));
        }
        for target in &visible_planets {
            let legal_families = legal_target_families(capabilities, viewer, target);
            if legal_families.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "  target planet_record={} x={} y={} distance={}",
                target.record_index_1_based,
                target.coords[0],
                target.coords[1],
                chebyshev_distance([x, y], target.coords)
            ));
            push_eta_properties(
                &mut out,
                estimate_fleet_eta_to_destination(
                    &state.game_data,
                    idx,
                    target.coords,
                    false,
                    true,
                ),
                year,
            );
            push_string_property(&mut out, "name", &target.name);
            if let Some(owner_empire_id) = target.owner_empire_id {
                out.push_str(&format!(" owner_empire_id={owner_empire_id}"));
                if owner_empire_id >= 1 {
                    push_string_property(
                        &mut out,
                        "owner_empire_name",
                        &state.game_data.players[owner_empire_id as usize - 1]
                            .empire_name
                            .trim()
                            .to_string(),
                    );
                }
            }
            out.push_str(" {\n");
            for family in legal_families {
                out.push_str(&format!("    legal_family \"{}\"\n", kdl_escape(family)));
            }
            out.push_str("  }\n");
        }
        out.push_str("}\n");
    }

    out
}

fn render_world_node(viewer: u8, world: &PlayerStarmapWorld) -> String {
    let mut out = format!(
        "world record={} x={} y={} visibility=\"{}\"",
        world.planet_record_index_1_based,
        world.coords[0],
        world.coords[1],
        world_visibility(viewer, world)
    );
    if let Some(value) = &world.known_name {
        push_string_property(&mut out, "name", value);
    }
    if let Some(value) = world.known_owner_empire_id {
        out.push_str(&format!(" owner_empire_id={value}"));
    }
    if let Some(value) = &world.known_owner_empire_name {
        push_string_property(&mut out, "owner_empire_name", value);
    }
    if let Some(value) = world.known_potential_production {
        out.push_str(&format!(" potential_production={value}"));
    }
    if let Some(value) = world.known_current_production {
        out.push_str(&format!(" current_production={value}"));
    }
    if let Some(value) = world.known_stored_points {
        out.push_str(&format!(" stored_points={value}"));
    }
    if let Some(value) = world.known_armies {
        out.push_str(&format!(" armies={value}"));
    }
    if let Some(value) = world.known_ground_batteries {
        out.push_str(&format!(" ground_batteries={value}"));
    }
    out.push('\n');
    out
}

fn visible_planet_hints_from_projection(
    projection: &PlayerStarmapProjection,
    viewer: u8,
) -> Vec<VisiblePlanetHint> {
    let mut planets = projection
        .worlds
        .iter()
        .filter_map(|world| {
            let has_visible_details = world.known_name.is_some()
                || world.known_owner_empire_id.is_some()
                || world.known_owner_empire_name.is_some()
                || world.known_potential_production.is_some()
                || world.known_armies.is_some()
                || world.known_ground_batteries.is_some()
                || world.known_current_production.is_some()
                || world.known_stored_points.is_some();
            if !has_visible_details {
                return None;
            }
            Some(VisiblePlanetHint {
                record_index_1_based: world.planet_record_index_1_based,
                coords: world.coords,
                name: world
                    .known_name
                    .clone()
                    .unwrap_or_else(|| format!("planet {}", world.planet_record_index_1_based)),
                owner_empire_id: world
                    .known_owner_empire_id
                    .filter(|id| *id != 0)
                    .or_else(|| (world.known_owner_empire_id == Some(viewer)).then_some(viewer)),
            })
        })
        .collect::<Vec<_>>();
    planets.sort_by_key(|planet| {
        (
            planet.coords[1],
            planet.coords[0],
            planet.record_index_1_based,
        )
    });
    planets
}

fn fleet_capabilities(fleet: &Fleet) -> FleetCapabilities {
    FleetCapabilities {
        has_combat: fleet.ships.destroyers > 0
            || fleet.ships.cruisers > 0
            || fleet.ships.battleships > 0,
        has_scout: fleet.ships.scouts > 0,
        has_etac: fleet.ships.etacs > 0,
        has_loaded_troops: troop_transports(fleet) > 0 && fleet.ships.loaded_transports > 0,
    }
}

fn non_target_families(capabilities: FleetCapabilities) -> Vec<&'static str> {
    let mut families = vec!["hold", "move", "seek_home"];
    if capabilities.has_scout {
        families.push("scout_sector");
    }
    families
}

fn targets_for_family<'a>(
    visible_planets: &'a [VisiblePlanetHint],
    capabilities: FleetCapabilities,
    viewer: u8,
    family: &'static str,
) -> Vec<&'a VisiblePlanetHint> {
    visible_planets
        .iter()
        .filter(|planet| legal_target_families(capabilities, viewer, planet).contains(&family))
        .collect()
}

fn legal_target_families(
    capabilities: FleetCapabilities,
    viewer: u8,
    planet: &VisiblePlanetHint,
) -> Vec<&'static str> {
    let mut families = vec!["view"];
    if capabilities.has_scout {
        families.push("scout_system");
    }
    if capabilities.has_etac && planet.owner_empire_id.is_none() {
        families.push("colonize");
    }
    if capabilities.has_combat {
        families.push("guard_blockade");
    }
    let is_foreign_target =
        planet.owner_empire_id.is_some() && planet.owner_empire_id != Some(viewer);
    if capabilities.has_combat && is_foreign_target {
        families.push("bombard");
    }
    if capabilities.has_combat && capabilities.has_loaded_troops && is_foreign_target {
        families.push("invade");
        families.push("blitz");
    }
    families
}

fn format_target_list(planets: &[&VisiblePlanetHint]) -> String {
    planets
        .iter()
        .map(|planet| {
            format!(
                "`{} ({},{})`",
                planet.name, planet.coords[0], planet.coords[1]
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn chebyshev_distance(from: [u8; 2], to: [u8; 2]) -> u8 {
    from[0].abs_diff(to[0]).max(from[1].abs_diff(to[1]))
}

fn world_visibility(viewer: u8, world: &PlayerStarmapWorld) -> &'static str {
    if world.known_owner_empire_id == Some(viewer) {
        "owned"
    } else if world.known_name.is_some()
        || world.known_owner_empire_id.is_some()
        || world.known_owner_empire_name.is_some()
        || world.known_potential_production.is_some()
        || world.known_armies.is_some()
        || world.known_ground_batteries.is_some()
        || world.known_current_production.is_some()
        || world.known_stored_points.is_some()
    {
        "known"
    } else {
        "coords_only"
    }
}

fn push_string_property(out: &mut String, name: &str, value: &str) {
    out.push_str(&format!(" {name}=\"{}\"", kdl_escape(value)));
}

fn push_eta_properties(out: &mut String, eta: FleetEtaEstimate, current_year: u16) {
    match eta {
        FleetEtaEstimate::Arrived => {
            out.push_str(&format!(
                " eta_status=\"arrived\" eta_years=0 eta_arrival_year={current_year}"
            ));
        }
        FleetEtaEstimate::Years(years) => {
            out.push_str(&format!(
                " eta_status=\"years\" eta_years={} eta_arrival_year={}",
                years,
                current_year.saturating_add(years)
            ));
        }
        FleetEtaEstimate::Stopped => out.push_str(" eta_status=\"stopped\""),
        FleetEtaEstimate::Unreachable => out.push_str(" eta_status=\"unreachable\""),
    }
}

fn production_kind_label(kind: ProductionItemKind) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => "destroyer",
        ProductionItemKind::Cruiser => "cruiser",
        ProductionItemKind::Battleship => "battleship",
        ProductionItemKind::Scout => "scout",
        ProductionItemKind::Transport => "transport",
        ProductionItemKind::Etac => "etac",
        ProductionItemKind::GroundBattery => "ground_battery",
        ProductionItemKind::Army => "army",
        ProductionItemKind::Starbase => "starbase",
        ProductionItemKind::Unknown(_) => "unknown",
    }
}
