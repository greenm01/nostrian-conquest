//! Invariants checked after every fuzz turn, returned as typed violations.

mod economy;
mod fleets;
mod map;
mod ownership;
mod reports;
mod ships;

use std::fmt;

use nc_data::{CoreGameData, MaintenanceEvents, PlayerLifecycleState, ReportBlockRow};

pub use economy::check_submitted_spending;

/// Everything one turn's checks look at.
pub struct TurnObservation<'a> {
    /// State after every empire submitted, before maintenance.
    pub before: &'a CoreGameData,
    /// State after maintenance.
    pub after: &'a CoreGameData,
    pub events: &'a MaintenanceEvents,
    pub report_rows: &'a [ReportBlockRow],
    /// `after` with the results reviewable flags applied for `report_rows`.
    pub flagged: &'a CoreGameData,
    /// Player lifecycle after maintenance.
    pub lifecycle: &'a [PlayerLifecycleState],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// An action's turn KDL parsed back to a different submission.
    TurnKdlRoundTrip {
        empire: u8,
        kdl: String,
    },
    /// A finished submission was rejected when submitted for real.
    SubmissionRejected {
        empire: u8,
        error: String,
    },
    /// A submission panicked inside the submission path.
    SubmissionPanic {
        empire: u8,
        message: String,
    },
    /// Accepted submissions left a state maintenance refuses to run.
    InvalidAfterSubmission {
        issues: Vec<String>,
    },
    MaintenanceError(String),
    MaintenancePanic(String),
    /// Two maintenance runs from the same state disagreed.
    Nondeterministic(String),
    CoordinateOutOfMap {
        what: String,
        coords: [u8; 2],
        map_size: u8,
    },
    PlanetsShareSector {
        first_record: usize,
        second_record: usize,
        coords: [u8; 2],
    },
    /// An event's fleet index names a different fleet than the one it
    /// described.
    FleetIdentity {
        event: String,
        detail: String,
    },
    ShipConservation {
        before: [u32; 6],
        after: [u32; 6],
        expected: [u32; 6],
    },
    OverBudget {
        planet_record: usize,
        spent: u32,
        budget: u32,
    },
    NegativeTreasury {
        planet_record: usize,
        stored: u32,
    },
    UnexplainedOwnershipChange {
        planet_record: usize,
        from: u8,
        to: u8,
    },
    ReportLine {
        viewer: u8,
        line: String,
    },
    ReportRecipient {
        viewer: u8,
    },
    OutlookMissed {
        empire: u8,
    },
    StorageRoundTrip(String),
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TurnKdlRoundTrip { empire, kdl } => {
                write!(f, "empire {empire} turn KDL does not round trip: {kdl:?}")
            }
            Self::SubmissionRejected { empire, error } => {
                write!(f, "empire {empire} submission rejected: {error}")
            }
            Self::SubmissionPanic { empire, message } => {
                write!(f, "empire {empire} submission panicked: {message}")
            }
            Self::InvalidAfterSubmission { issues } => {
                write!(
                    f,
                    "submissions left an invalid state: {}",
                    issues.join("; ")
                )
            }
            Self::MaintenanceError(error) => write!(f, "maintenance failed: {error}"),
            Self::MaintenancePanic(message) => write!(f, "maintenance panicked: {message}"),
            Self::Nondeterministic(detail) => {
                write!(f, "maintenance is nondeterministic: {detail}")
            }
            Self::CoordinateOutOfMap {
                what,
                coords,
                map_size,
            } => write!(f, "{what} at {coords:?} is outside 1..={map_size}"),
            Self::PlanetsShareSector {
                first_record,
                second_record,
                coords,
            } => write!(
                f,
                "planets {first_record} and {second_record} share sector {coords:?}"
            ),
            Self::FleetIdentity { event, detail } => {
                write!(f, "event names the wrong fleet: {detail} in {event}")
            }
            Self::ShipConservation {
                before,
                after,
                expected,
            } => write!(
                f,
                "ships [dd ca bb sc tt et] not conserved: before {before:?}, after {after:?}, events explain {expected:?}"
            ),
            Self::OverBudget {
                planet_record,
                spent,
                budget,
            } => write!(
                f,
                "planet {planet_record} spent {spent} points against a budget of {budget}"
            ),
            Self::NegativeTreasury {
                planet_record,
                stored,
            } => write!(f, "planet {planet_record} treasury wrapped to {stored}"),
            Self::UnexplainedOwnershipChange {
                planet_record,
                from,
                to,
            } => write!(
                f,
                "planet {planet_record} changed owner {from} -> {to} without an event"
            ),
            Self::ReportLine { viewer, line } => {
                write!(
                    f,
                    "report line for viewer {viewer} is not printable within 72: {line:?}"
                )
            }
            Self::ReportRecipient { viewer } => write!(
                f,
                "report sent to empire {viewer}, which has neither an active player nor a terminal review"
            ),
            Self::OutlookMissed { empire } => {
                write!(
                    f,
                    "campaign outlook not reported to and flagged for active empire {empire}"
                )
            }
            Self::StorageRoundTrip(detail) => write!(f, "SQLite round trip differs: {detail}"),
        }
    }
}

/// Every per-turn invariant over one maintenance turn.
pub fn check_turn(observation: &TurnObservation) -> Vec<Violation> {
    let mut violations = map::check(observation.after);
    violations.extend(fleets::check(observation));
    violations.extend(ships::check(observation));
    violations.extend(economy::check_maintenance_spending(observation));
    violations.extend(ownership::check(observation));
    violations.extend(reports::check(observation));
    violations
}
