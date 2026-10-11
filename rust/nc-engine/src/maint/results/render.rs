use crate::maint::results::format::owned_fleet_source_clause;

/// The header of a retarget report: the fleet's number and where it is,
/// named by system when it is in orbit and by sector when it is not.
pub(super) fn mission_retarget_source(
    reporting_fleet_number: Option<u8>,
    current_coords: [u8; 2],
    in_system: bool,
) -> String {
    let place = if in_system { "System" } else { "Sector" };
    owned_fleet_source_clause(
        reporting_fleet_number,
        &format!("{place}({},{})", current_coords[0], current_coords[1]),
    )
}
