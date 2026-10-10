mod colonization;
mod consolidation;
mod helpers;
mod mission;

use super::{ColonizationEvent, ColonizationResolvedEvent, FleetMergeEvent, JoinMissionHostEvent};
use crate::{CoreGameData, maint::FleetRemovalRemapInfo};

pub(super) fn process_colonizations(
    game_data: &mut CoreGameData,
    events: &[ColonizationEvent],
) -> Result<Vec<ColonizationResolvedEvent>, Box<dyn std::error::Error>> {
    colonization::process_colonizations(game_data, events)
}

pub(super) fn process_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    consolidation::process_fleet_merging(game_data)
}

pub(super) fn process_mission_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    mission::process_mission_fleet_merging(game_data)
}

pub(super) fn process_join_host_updates(
    game_data: &mut CoreGameData,
    merge_events: &[FleetMergeEvent],
    destroyed_join_host_fleet_numbers: &std::collections::HashMap<u16, u8>,
    remap_info: &FleetRemovalRemapInfo,
) -> Vec<JoinMissionHostEvent> {
    mission::process_join_host_updates(
        game_data,
        merge_events,
        destroyed_join_host_fleet_numbers,
        remap_info,
    )
}
