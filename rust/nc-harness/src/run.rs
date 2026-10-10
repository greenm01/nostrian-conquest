use std::time::Instant;

use nc_data::CoreGameData;

use crate::build::{BuiltScenario, ScenarioBuildReport, apply_turn_files, build_scenario};
use crate::error::HarnessError;
use crate::play::ScenarioPlay;
use crate::spec::{
    CombatScenarioSpec, CombatSweepSpec, DiplomacySpec, FleetShipsSpec, PlanetSpec, ScenarioSpec,
    SweepDimension,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpireCombatSummary {
    pub empire_raw: u8,
    pub planets_before: usize,
    pub planets_after: usize,
    pub fleets_before: usize,
    pub fleets_after: usize,
    pub ships_before: u32,
    pub ships_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatRunReport {
    pub scenario: ScenarioBuildReport,
    pub maintenance_turns: u16,
    pub final_year: u16,
    pub fleet_battle_events: usize,
    pub bombard_events: usize,
    pub assault_report_events: usize,
    pub ownership_changes: usize,
    pub elapsed_millis: u128,
    pub empires: Vec<EmpireCombatSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatRun {
    pub built: BuiltScenario,
    pub report: CombatRunReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepCaseReport {
    pub case_index: usize,
    pub label: String,
    pub elapsed_millis: u128,
    pub fleet_battle_events: usize,
    pub bombard_events: usize,
    pub assault_report_events: usize,
    pub ownership_changes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatSweepReport {
    pub scenario_path: std::path::PathBuf,
    pub seed: u64,
    pub requested_max_cases: usize,
    pub executed_cases: usize,
    pub total_possible_cases: usize,
    pub mean_millis: u128,
    pub median_millis: u128,
    pub p95_millis: u128,
    pub cases: Vec<SweepCaseReport>,
}

pub fn run_combat_scenario(spec: &CombatScenarioSpec) -> Result<CombatRun, HarnessError> {
    let mut play = ScenarioPlay::new(build_scenario(&spec.scenario)?);
    let before = snapshot_empire_summaries(&play.built.game_data);
    let started = Instant::now();

    let mut fleet_battle_events = 0usize;
    let mut bombard_events = 0usize;
    let mut assault_report_events = 0usize;
    let mut ownership_changes = 0usize;

    for turn in 1..=spec.maintenance_turns {
        if turn > 1 {
            let built = &mut play.built;
            apply_turn_files(
                &spec.scenario,
                turn,
                &mut built.game_data,
                &mut built.queued_mail,
            )?;
        }
        let events = play.run_maintenance()?;
        fleet_battle_events += events.fleet_battle_events.len();
        bombard_events += events.bombard_events.len();
        assault_report_events += events.assault_report_events.len();
        ownership_changes += events.ownership_change_events.len();
    }

    let elapsed_millis = started.elapsed().as_millis();
    let built = play.built;
    let final_year = built.game_data.conquest.game_year();
    let after = snapshot_empire_summaries(&built.game_data);
    let empires = (1..=built.game_data.conquest.player_count())
        .map(|empire_raw| EmpireCombatSummary {
            empire_raw,
            planets_before: before[(empire_raw - 1) as usize].0,
            planets_after: after[(empire_raw - 1) as usize].0,
            fleets_before: before[(empire_raw - 1) as usize].1,
            fleets_after: after[(empire_raw - 1) as usize].1,
            ships_before: before[(empire_raw - 1) as usize].2,
            ships_after: after[(empire_raw - 1) as usize].2,
        })
        .collect::<Vec<_>>();

    let report = CombatRunReport {
        scenario: built.report.clone(),
        maintenance_turns: spec.maintenance_turns,
        final_year,
        fleet_battle_events,
        bombard_events,
        assault_report_events,
        ownership_changes,
        elapsed_millis,
        empires,
    };

    Ok(CombatRun { built, report })
}

pub fn run_combat_sweep(spec: &CombatSweepSpec) -> Result<CombatSweepReport, HarnessError> {
    let base = CombatScenarioSpec::load_kdl(&spec.scenario_path)?;
    let total_possible_cases = case_count(&spec.dimensions);
    let case_specs =
        expand_case_specs(&base.scenario, &spec.dimensions, spec.seed, spec.max_cases)?;
    let mut cases = Vec::with_capacity(case_specs.len());

    for (case_index, (label, scenario)) in case_specs.into_iter().enumerate() {
        let run = run_combat_scenario(&CombatScenarioSpec {
            scenario,
            maintenance_turns: spec.maintenance_turns.unwrap_or(base.maintenance_turns),
        })?;
        cases.push(SweepCaseReport {
            case_index: case_index + 1,
            label,
            elapsed_millis: run.report.elapsed_millis,
            fleet_battle_events: run.report.fleet_battle_events,
            bombard_events: run.report.bombard_events,
            assault_report_events: run.report.assault_report_events,
            ownership_changes: run.report.ownership_changes,
        });
    }

    let mut timings = cases
        .iter()
        .map(|case| case.elapsed_millis)
        .collect::<Vec<_>>();
    timings.sort_unstable();
    let executed_cases = cases.len();
    let mean_millis = if timings.is_empty() {
        0
    } else {
        timings.iter().sum::<u128>() / timings.len() as u128
    };
    let median_millis = percentile(&timings, 50);
    let p95_millis = percentile(&timings, 95);

    Ok(CombatSweepReport {
        scenario_path: spec.scenario_path.clone(),
        seed: spec.seed,
        requested_max_cases: spec.max_cases,
        executed_cases,
        total_possible_cases,
        mean_millis,
        median_millis,
        p95_millis,
        cases,
    })
}

fn snapshot_empire_summaries(game_data: &CoreGameData) -> Vec<(usize, usize, u32)> {
    (1..=game_data.conquest.player_count())
        .map(|empire_raw| {
            let planets = game_data
                .planets
                .records
                .iter()
                .filter(|planet| planet.owner_empire_slot_raw() == empire_raw)
                .count();
            let fleets = game_data
                .fleets
                .records
                .iter()
                .filter(|fleet| fleet.owner_empire_raw() == empire_raw && fleet_has_units(fleet))
                .count();
            let ships = game_data
                .fleets
                .records
                .iter()
                .filter(|fleet| fleet.owner_empire_raw() == empire_raw)
                .map(total_fleet_units)
                .sum::<u32>();
            (planets, fleets, ships)
        })
        .collect()
}

fn fleet_has_units(fleet: &nc_data::FleetRecord) -> bool {
    total_fleet_units(fleet) > 0
}

fn total_fleet_units(fleet: &nc_data::FleetRecord) -> u32 {
    u32::from(fleet.battleship_count())
        + u32::from(fleet.cruiser_count())
        + u32::from(fleet.destroyer_count())
        + u32::from(fleet.scout_count())
        + u32::from(fleet.troop_transport_count())
        + u32::from(fleet.etac_count())
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let idx = ((values.len() - 1) * percentile) / 100;
    values[idx]
}

fn case_count(dimensions: &[SweepDimension]) -> usize {
    dimensions.iter().fold(1usize, |acc, dimension| {
        acc.saturating_mul(match dimension {
            SweepDimension::FleetShips { values, .. } => values.len(),
            SweepDimension::FleetRoe { values, .. } => values.len(),
            SweepDimension::PlanetStat { values, .. } => values.len(),
            SweepDimension::DiplomaticRelation { values, .. } => values.len(),
        })
    })
}

fn expand_case_specs(
    base: &ScenarioSpec,
    dimensions: &[SweepDimension],
    seed: u64,
    max_cases: usize,
) -> Result<Vec<(String, ScenarioSpec)>, HarnessError> {
    let dimensions = rotated_dimensions(dimensions, seed);
    let mut out = Vec::new();
    let mut current = base.clone();
    let mut labels = Vec::new();
    expand_dimension_recursive(
        &dimensions,
        0,
        &mut current,
        &mut labels,
        &mut out,
        max_cases,
    )?;
    Ok(out)
}

fn rotated_dimensions(dimensions: &[SweepDimension], seed: u64) -> Vec<SweepDimension> {
    dimensions
        .iter()
        .enumerate()
        .map(|(idx, dimension)| {
            let salt = ((seed as usize) ^ idx).wrapping_mul(31);
            match dimension {
                SweepDimension::FleetShips {
                    fleet_record_index_1_based,
                    kind,
                    values,
                } => {
                    let mut rotated = values.clone();
                    rotate_left(&mut rotated, salt);
                    SweepDimension::FleetShips {
                        fleet_record_index_1_based: *fleet_record_index_1_based,
                        kind: *kind,
                        values: rotated,
                    }
                }
                SweepDimension::FleetRoe {
                    fleet_record_index_1_based,
                    values,
                } => {
                    let mut rotated = values.clone();
                    rotate_left(&mut rotated, salt);
                    SweepDimension::FleetRoe {
                        fleet_record_index_1_based: *fleet_record_index_1_based,
                        values: rotated,
                    }
                }
                SweepDimension::PlanetStat {
                    planet_record_index_1_based,
                    field,
                    values,
                } => {
                    let mut rotated = values.clone();
                    rotate_left(&mut rotated, salt);
                    SweepDimension::PlanetStat {
                        planet_record_index_1_based: *planet_record_index_1_based,
                        field: *field,
                        values: rotated,
                    }
                }
                SweepDimension::DiplomaticRelation {
                    from_empire_raw,
                    to_empire_raw,
                    values,
                } => {
                    let mut rotated = values.clone();
                    rotate_left(&mut rotated, salt);
                    SweepDimension::DiplomaticRelation {
                        from_empire_raw: *from_empire_raw,
                        to_empire_raw: *to_empire_raw,
                        values: rotated,
                    }
                }
            }
        })
        .collect()
}

fn rotate_left<T>(values: &mut [T], salt: usize) {
    if values.is_empty() {
        return;
    }
    values.rotate_left(salt % values.len());
}

fn expand_dimension_recursive(
    dimensions: &[SweepDimension],
    idx: usize,
    current: &mut ScenarioSpec,
    labels: &mut Vec<String>,
    out: &mut Vec<(String, ScenarioSpec)>,
    max_cases: usize,
) -> Result<(), HarnessError> {
    if out.len() >= max_cases {
        return Ok(());
    }
    if idx == dimensions.len() {
        out.push((labels.join(", "), current.clone()));
        return Ok(());
    }

    match &dimensions[idx] {
        SweepDimension::FleetShips {
            fleet_record_index_1_based,
            kind,
            values,
        } => {
            for value in values {
                let original = fleet_ships_mut(current, *fleet_record_index_1_based)?.clone();
                apply_fleet_ship_value(current, *fleet_record_index_1_based, *kind, *value)?;
                labels.push(format!(
                    "fleet{}:{}={}",
                    fleet_record_index_1_based,
                    ship_kind_label(*kind),
                    value
                ));
                expand_dimension_recursive(dimensions, idx + 1, current, labels, out, max_cases)?;
                labels.pop();
                *fleet_ships_mut(current, *fleet_record_index_1_based)? = original;
                if out.len() >= max_cases {
                    break;
                }
            }
        }
        SweepDimension::FleetRoe {
            fleet_record_index_1_based,
            values,
        } => {
            let original =
                fleet_spec_mut(current, *fleet_record_index_1_based)?.rules_of_engagement;
            for value in values {
                fleet_spec_mut(current, *fleet_record_index_1_based)?.rules_of_engagement =
                    Some(*value);
                labels.push(format!("fleet{}:roe={value}", fleet_record_index_1_based));
                expand_dimension_recursive(dimensions, idx + 1, current, labels, out, max_cases)?;
                labels.pop();
                if out.len() >= max_cases {
                    break;
                }
            }
            fleet_spec_mut(current, *fleet_record_index_1_based)?.rules_of_engagement = original;
        }
        SweepDimension::PlanetStat {
            planet_record_index_1_based,
            field,
            values,
        } => {
            let original_armies = planet_spec_mut(current, *planet_record_index_1_based)?.armies;
            let original_batteries =
                planet_spec_mut(current, *planet_record_index_1_based)?.ground_batteries;
            for value in values {
                match field {
                    crate::spec::PlanetStatField::Armies => {
                        planet_spec_mut(current, *planet_record_index_1_based)?.armies =
                            Some(u8::try_from(*value).map_err(|_| {
                                HarnessError::Validation(format!(
                                    "planet-stat armies value out of range: {value}"
                                ))
                            })?);
                    }
                    crate::spec::PlanetStatField::GroundBatteries => {
                        planet_spec_mut(current, *planet_record_index_1_based)?.ground_batteries =
                            Some(u8::try_from(*value).map_err(|_| {
                                HarnessError::Validation(format!(
                                    "planet-stat batteries value out of range: {value}"
                                ))
                            })?);
                    }
                }
                labels.push(format!(
                    "planet{}:{}={}",
                    planet_record_index_1_based,
                    planet_field_label(*field),
                    value
                ));
                expand_dimension_recursive(dimensions, idx + 1, current, labels, out, max_cases)?;
                labels.pop();
                if out.len() >= max_cases {
                    break;
                }
            }
            let planet = planet_spec_mut(current, *planet_record_index_1_based)?;
            planet.armies = original_armies;
            planet.ground_batteries = original_batteries;
        }
        SweepDimension::DiplomaticRelation {
            from_empire_raw,
            to_empire_raw,
            values,
        } => {
            let relation = relation_spec_mut(current, *from_empire_raw, *to_empire_raw);
            let original = relation.map(|existing| existing.relation);
            for value in values {
                if let Some(existing) = relation_spec_mut(current, *from_empire_raw, *to_empire_raw)
                {
                    existing.relation = *value;
                } else {
                    current.diplomacy.push(DiplomacySpec {
                        from_empire_raw: *from_empire_raw,
                        to_empire_raw: *to_empire_raw,
                        relation: *value,
                    });
                }
                labels.push(format!(
                    "relation{}-{}={}",
                    from_empire_raw,
                    to_empire_raw,
                    relation_label(*value)
                ));
                expand_dimension_recursive(dimensions, idx + 1, current, labels, out, max_cases)?;
                labels.pop();
                if out.len() >= max_cases {
                    break;
                }
            }
            if let Some(existing) = relation_spec_mut(current, *from_empire_raw, *to_empire_raw) {
                if let Some(original) = original {
                    existing.relation = original;
                } else {
                    current.diplomacy.retain(|entry| {
                        !(entry.from_empire_raw == *from_empire_raw
                            && entry.to_empire_raw == *to_empire_raw)
                    });
                }
            }
        }
    }

    Ok(())
}

fn fleet_spec_mut(
    scenario: &mut ScenarioSpec,
    record_index_1_based: usize,
) -> Result<&mut crate::spec::FleetSpec, HarnessError> {
    scenario
        .fleets
        .iter_mut()
        .find(|fleet| fleet.record_index_1_based == record_index_1_based)
        .ok_or_else(|| {
            HarnessError::Validation(format!(
                "sweep references fleet {} but base combat scenario does not define it",
                record_index_1_based
            ))
        })
}

fn fleet_ships_mut(
    scenario: &mut ScenarioSpec,
    record_index_1_based: usize,
) -> Result<&mut FleetShipsSpec, HarnessError> {
    let fleet = fleet_spec_mut(scenario, record_index_1_based)?;
    if fleet.ships.is_none() {
        fleet.ships = Some(FleetShipsSpec::default());
    }
    Ok(fleet.ships.as_mut().expect("fleet ships should exist"))
}

fn planet_spec_mut(
    scenario: &mut ScenarioSpec,
    record_index_1_based: usize,
) -> Result<&mut PlanetSpec, HarnessError> {
    scenario
        .planets
        .iter_mut()
        .find(|planet| planet.record_index_1_based == record_index_1_based)
        .ok_or_else(|| {
            HarnessError::Validation(format!(
                "sweep references planet {} but base combat scenario does not define it",
                record_index_1_based
            ))
        })
}

fn relation_spec_mut(
    scenario: &mut ScenarioSpec,
    from_empire_raw: u8,
    to_empire_raw: u8,
) -> Option<&mut DiplomacySpec> {
    scenario.diplomacy.iter_mut().find(|relation| {
        relation.from_empire_raw == from_empire_raw && relation.to_empire_raw == to_empire_raw
    })
}

fn apply_fleet_ship_value(
    scenario: &mut ScenarioSpec,
    fleet_record_index_1_based: usize,
    kind: crate::spec::ShipDimensionKind,
    value: u16,
) -> Result<(), HarnessError> {
    let ships = fleet_ships_mut(scenario, fleet_record_index_1_based)?;
    match kind {
        crate::spec::ShipDimensionKind::Battleships => ships.battleships = value,
        crate::spec::ShipDimensionKind::Cruisers => ships.cruisers = value,
        crate::spec::ShipDimensionKind::Destroyers => ships.destroyers = value,
        crate::spec::ShipDimensionKind::Scouts => {
            ships.scouts = u8::try_from(value).map_err(|_| {
                HarnessError::Validation(format!("scout value out of range: {value}"))
            })?
        }
        crate::spec::ShipDimensionKind::Transports => ships.transports = value,
        crate::spec::ShipDimensionKind::LoadedArmies => ships.loaded_armies = value,
        crate::spec::ShipDimensionKind::Etacs => ships.etacs = value,
    }
    Ok(())
}

fn ship_kind_label(kind: crate::spec::ShipDimensionKind) -> &'static str {
    match kind {
        crate::spec::ShipDimensionKind::Battleships => "bb",
        crate::spec::ShipDimensionKind::Cruisers => "ca",
        crate::spec::ShipDimensionKind::Destroyers => "dd",
        crate::spec::ShipDimensionKind::Scouts => "sc",
        crate::spec::ShipDimensionKind::Transports => "tt",
        crate::spec::ShipDimensionKind::LoadedArmies => "armies",
        crate::spec::ShipDimensionKind::Etacs => "etac",
    }
}

fn planet_field_label(field: crate::spec::PlanetStatField) -> &'static str {
    match field {
        crate::spec::PlanetStatField::Armies => "armies",
        crate::spec::PlanetStatField::GroundBatteries => "batteries",
    }
}

fn relation_label(value: nc_data::DiplomaticRelation) -> &'static str {
    match value {
        nc_data::DiplomaticRelation::Neutral => "neutral",
        nc_data::DiplomaticRelation::Enemy => "enemy",
    }
}
