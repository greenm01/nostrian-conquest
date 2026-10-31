//! Plain event and type definitions shared by the maintenance engine.

use crate::{
    DiplomaticRelation, EmpireUnitSummary, FleetOrderValidationError,
    FleetPlayerInputValidationError, PlanetIntelSnapshot, PlanetPlayerInputValidationError,
    PlayerDiplomacyValidationError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ShipLosses {
    pub destroyers: u32,
    pub cruisers: u32,
    pub battleships: u32,
    pub scouts: u32,
    pub transports: u32,
    pub etacs: u32,
}

/// A bombardment event: an empire's fleets executed BombardWorld against one
/// planet, together (t202).
#[derive(Debug, Clone)]
pub struct BombardEvent {
    /// Planet index (into PLANETS.DAT records) that was bombarded.
    pub planet_idx: usize,
    /// Attacking fleet's owner_empire (1-based player index).
    pub attacker_empire: u8,
    /// Attacking fleet number when one specific fleet can be named.
    pub attacker_fleet_number: Option<u8>,
    /// Defending empire that should receive the bombardment report, if any.
    pub defender_empire: u8,
    /// Initial attacking fleet composition observed by both sides.
    pub attacker_initial: ShipLosses,
    /// Initial loaded armies aboard the attacking force.
    pub attacker_loaded_armies_initial: u32,
    /// Initial defender ground batteries.
    pub defender_batteries_initial: u8,
    /// Initial defender armies.
    pub defender_armies_initial: u8,
    /// Exact attacker fleet losses during the bombardment exchange.
    pub attacker_losses: ShipLosses,
    /// Observed defender ground battery losses.
    pub defender_battery_losses: u8,
    /// The defender's batteries left crippled after the bombardment (t202).
    pub defender_batteries_crippled: u8,
    /// Observed defender army losses.
    pub defender_army_losses: u8,
    /// Whether the bombardment broke through to armies/production (batteries cleared).
    pub breakthrough: bool,
    /// Typed docked-unit losses caused by the bombardment.
    pub docked_losses: EmpireUnitSummary,
    /// Stardock items destroyed by bombardment.
    pub stardock_items_destroyed: u32,
    /// Stored goods destroyed by bombardment spillover.
    pub stored_goods_destroyed: u32,
    /// Factories destroyed by bombardment spillover.
    pub factories_destroyed: u16,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// What a captured world passed to its new owner, as the capture left it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InheritedAssets {
    /// Stored goods the new owner can spend, in whole points (the planet
    /// keeps the fraction); debt is not an asset.
    pub stored_goods: u32,
    /// Ships and starbases waiting in the stardock.
    pub docked: EmpireUnitSummary,
    /// Production points still owed on the build queue.
    pub queued_work: u32,
    /// The defenders had set Scorch Earth, so the capture halved the world.
    pub scorched: bool,
    /// Armies the attackers left holding the world.
    pub garrison: u16,
}

/// A ground-assault event for invade/blitz mission reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssaultReportEvent {
    /// Fleet mission kind that produced the assault.
    pub kind: Mission,
    /// Attacking fleet number when one specific fleet can be named.
    pub attacker_fleet_number: Option<u8>,
    /// Planet index (into PLANETS.DAT records) that was attacked.
    pub planet_idx: usize,
    /// Acting empire that should receive the attacker-side report.
    pub attacker_empire: u8,
    /// Defending empire that was attacked, if any.
    pub defender_empire: u8,
    /// Initial attacking fleet composition observed by both sides.
    pub attacker_initial: ShipLosses,
    /// Initial loaded armies aboard the attacking force.
    pub attacker_loaded_armies_initial: u32,
    /// Initial defender ground batteries.
    pub defender_batteries_initial: u8,
    /// Initial defender armies.
    pub defender_armies_initial: u8,
    /// Exact attacker fleet losses during the orbital/landing exchange.
    pub attacker_ship_losses: ShipLosses,
    /// Attacker ground losses.
    pub attacker_army_losses: u32,
    /// Attacker troop losses suffered in destroyed transports during landing.
    pub transport_army_losses: u32,
    /// Defender battery losses.
    pub defender_battery_losses: u8,
    /// Defender army losses caused by invasion softening fire before landing.
    pub defender_army_losses_softening: u8,
    /// Defender army losses.
    pub defender_army_losses: u8,
    /// What the attacker inherited, when the assault took the world.
    pub inherited: Option<InheritedAssets>,
    /// Final mission outcome.
    pub outcome: MissionOutcome,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// Source family for one player's classic planet-database refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetIntelSource {
    ScoutSolarSystem,
    ViewWorld,
    ColonizeBlockedByOwner,
    CivilDisorderContact,
    AssaultSuccess,
    AssaultFailure,
    /// The attacker's view of a world it bombarded: the figures after the
    /// attack (t194), as ECMAINT writes it.
    Bombardment,
}

/// A classic planet-database refresh for one player's view of one planet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetIntelEvent {
    /// Planet index (into PLANETS.DAT records) whose intel should be refreshed.
    pub planet_idx: usize,
    /// Viewer empire (1-based player index) whose DATABASE record should be updated.
    pub viewer_empire: u8,
    /// Mission/combat family that produced this refresh.
    pub source: PlanetIntelSource,
    /// Source fleet index when one specific fleet observation produced this refresh.
    pub source_fleet_idx: Option<usize>,
    /// Observed snapshot captured when the intel was earned.
    pub observed_snapshot: Option<PlanetIntelSnapshot>,
    /// Week of year (1–52) when this intel was acquired; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A combat-triggered planet ownership change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetOwnershipChangeEvent {
    /// Planet index (into PLANETS.DAT records) that changed owner.
    pub planet_idx: usize,
    /// Empire that should receive the "we were captured" report.
    pub reporting_empire: u8,
    /// Previous owner empire slot (1-based, or 0 if unowned).
    pub previous_owner_empire: u8,
    /// New owner empire slot (1-based).
    pub new_owner_empire: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A fleet battle resolved at one location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetBattlePerspective {
    Attacked,
    Intercepted,
}

/// A fleet battle resolved at one location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetBattleEvent {
    /// Empire that should receive this battle report.
    pub reporting_empire: u8,
    /// Reporting fleet number when one specific fleet can be named.
    pub reporting_fleet_number: Option<u8>,
    /// Reporting fleet mission context when one classic mission-family label applies.
    pub reporting_mission: Option<Mission>,
    /// Whether the report should read as "we were attacked" or "we intercepted".
    pub perspective: FleetBattlePerspective,
    /// Coordinates where the battle took place.
    pub coords: [u8; 2],
    /// Hostile empires this side encountered.
    pub enemy_empires: Vec<u8>,
    /// Primary hostile fleet number when one specific enemy fleet can be named.
    pub primary_enemy_fleet_number: Option<u8>,
    /// Whether the reporting empire held the field after the battle.
    pub held_field: bool,
    /// Initial composition of the reporting force.
    pub friendly_initial: ShipLosses,
    /// Initial friendly starbases participating in the battle.
    pub friendly_initial_starbases: u32,
    /// Loaded armies aboard the reporting force at battle start.
    pub friendly_loaded_armies_initial: u32,
    /// Exact losses suffered by the reporting empire.
    pub friendly_losses: ShipLosses,
    /// Friendly starbases destroyed in this battle.
    pub friendly_starbases_lost: u32,
    /// Initial observed hostile composition across opposing forces.
    pub enemy_initial: ShipLosses,
    /// Initial enemy starbases participating in the battle.
    pub enemy_initial_starbases: u32,
    /// Loaded armies aboard the opposing force at battle start.
    pub enemy_loaded_armies_initial: u32,
    /// Observed hostile losses across the opposing forces.
    pub enemy_losses: ShipLosses,
    /// Enemy starbases destroyed in this battle.
    pub enemy_starbases_destroyed: u32,
    /// Whether the attacking side struck by surprise: the defending empire
    /// had no starbase in this sector and had not sighted the attacker
    /// there. The `Intercepted` side is the attacker.
    pub surprise: bool,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A fleet was completely destroyed and command lost all contact with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetDestroyedEvent {
    /// Empire that receives the destruction report.
    pub reporting_empire: u8,
    /// Player-visible fleet number that was lost.
    pub fleet_number: u8,
    /// Coordinates of the loss.
    pub coords: [u8; 2],
    /// Whether the fleet was attacking/intercepting or was attacked.
    pub was_intercepting: bool,
    /// Initial composition of the lost fleet.
    pub friendly_initial: ShipLosses,
    /// Loaded armies aboard the lost fleet at battle start.
    pub friendly_loaded_armies_initial: u32,
    /// Initial observed hostile composition.
    pub enemy_initial: ShipLosses,
    /// Initial hostile starbases participating in the battle.
    pub enemy_initial_starbases: u32,
    /// Loaded armies aboard the hostile force at battle start.
    pub enemy_loaded_armies_initial: u32,
    /// Initial hostile ground batteries observed at contact, if any.
    pub enemy_ground_batteries_initial: u8,
    /// Initial hostile armies observed at contact, if any.
    pub enemy_ground_armies_initial: u8,
    /// Observed hostile losses before contact was lost.
    pub enemy_losses: ShipLosses,
    /// Hostile starbases destroyed before contact was lost.
    pub enemy_starbases_destroyed: u32,
    /// Hostile ground batteries destroyed before contact was lost.
    pub enemy_ground_battery_losses: u8,
    /// Hostile armies destroyed before contact was lost.
    pub enemy_ground_army_losses: u8,
    /// Hostile empire if a primary enemy can be named.
    pub primary_enemy_empire: Option<u8>,
    /// Hostile fleet number if one specific enemy fleet can be named.
    pub primary_enemy_fleet_number: Option<u8>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A starbase was completely destroyed and command lost all contact with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarbaseDestroyedEvent {
    /// Empire that receives the destruction report.
    pub reporting_empire: u8,
    /// Starbase ID that was lost.
    pub starbase_id: u16,
    /// Coordinates of the loss.
    pub coords: [u8; 2],
    /// Initial observed hostile composition.
    pub enemy_initial: ShipLosses,
    /// Loaded armies aboard the hostile force at contact.
    pub enemy_loaded_armies_initial: u32,
    /// Observed hostile losses before contact was lost.
    pub enemy_losses: ShipLosses,
    /// Hostile empire if a primary enemy can be named.
    pub primary_enemy_empire: Option<u8>,
    /// Hostile fleet number if one specific enemy fleet can be named.
    pub primary_enemy_fleet_number: Option<u8>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A starbase's own report from the year's ticks (ECMAINT `0b5d:5793`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarbaseReportEvent {
    /// Empire that owns the base and receives the report.
    pub owner_empire: u8,
    pub starbase_id: u16,
    /// The base's sector when it reported.
    pub coords: [u8; 2],
    /// Whether the base was in a world's system (depth layer 1).
    pub in_system: bool,
    pub report: StarbaseReport,
    /// Week of year (1-52) when this event occurred.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarbaseReport {
    /// The base reached its destination: in orbit (`orbit`) or in deep
    /// space.
    Arrived { orbit: bool },
    /// The world the base relies on is no longer its owner's. `system` is
    /// the base's target sector; `planet_idx` names the world, if any.
    LostWorld {
        planet_idx: Option<usize>,
        system: [u8; 2],
        response: LostWorldResponse,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostWorldResponse {
    /// It stopped where it is and awaits orders.
    Holding,
    /// It is leaving the system for deep space in its own sector.
    Leaving,
}

/// What a starbase tells its owner about the alien fleet it identified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarbaseAlert {
    /// The fleet is in, or on an assault bound for, the owner's world here:
    /// "The aliens appear to be headed at our planet X! We are alerting all
    /// fleets in the area."
    HeadedAtOurWorld,
    /// The owner has declared the fleet's empire an enemy: "We are alerting
    /// all fleets in the area."
    Enemy,
    /// Neither: "Continuing with scanning assignment...".
    Scanning,
}

/// A starbase identified an alien starbase sharing its sector and depth
/// layer (ECMAINT `18ea:1fea`). It only reports; a base never attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarbaseSightingEvent {
    /// Empire that owns the reporting base and receives the report.
    pub viewer_empire: u8,
    pub starbase_id: u16,
    /// The sector both bases share.
    pub coords: [u8; 2],
    /// Whether they are in a world's system (depth layer 1).
    pub in_system: bool,
    pub target_empire: u8,
    pub target_starbase_id: u16,
    /// The viewer has declared the target's empire an enemy: "We are
    /// alerting all fleets in the area." Otherwise "Continuing with
    /// scanning assignment...".
    pub alerting: bool,
    /// Week of year (1-52) when this event occurred.
    pub stardate_week: Option<u8>,
}

/// A scout-style hostile contact report resolved during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactReportSource {
    FleetMission(Mission),
    Fleet(u8),
    Starbase { id: u16, alert: StarbaseAlert },
}

/// A scout-style hostile contact report resolved during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoutContactEvent {
    /// Empire that owns the observing fleet.
    pub viewer_empire: u8,
    /// Fleet or starbase source that made the contact.
    pub source: ContactReportSource,
    /// Fleet number of the reporting fleet when the source is a fleet.
    pub reporting_fleet_number: Option<u8>,
    /// Initial composition of the reporting fleet when the source is a fleet.
    pub reporting_initial: ShipLosses,
    /// Loaded armies aboard the reporting fleet when the source is a fleet.
    pub reporting_loaded_armies_initial: u32,
    /// Coordinates where the contact occurred.
    pub coords: [u8; 2],
    /// Empire that was detected.
    pub target_empire: u8,
    /// Target fleet number when one specific hostile fleet can be named.
    pub target_fleet_number: Option<u8>,
    /// Aggregate "small vessel" count in the detected force.
    pub small_vessels: u32,
    /// Aggregate "medium vessel" count in the detected force.
    pub medium_vessels: u32,
    /// Aggregate "large vessel" count in the detected force.
    pub large_vessels: u32,
    /// The detected fleet's speed at the sighting; 0 is sublight.
    pub target_speed: u8,
    /// The observer was inside the world's system, not in deep space.
    pub in_system: bool,
    /// The observer identified the fleet. A fleet that is not closing only
    /// reaches contact; a later sighting may identify it.
    pub identified: bool,
    /// What the identifying fleet decided, when the report states it.
    pub decision: Option<ContactDecision>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// What a fleet decided about an alien fleet it identified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactDecision {
    /// Not hostile: "Ignoring alien fleet".
    Ignore,
    /// Closing to attack.
    Engage,
}

/// A friendly merge result for join/rendezvous style orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetMergeEvent {
    /// Fleet index in FLEETS.DAT that merged away.
    pub fleet_idx: usize,
    /// Empire that owned the merging fleet.
    pub owner_empire: u8,
    /// Kind of merge-producing mission.
    pub kind: Mission,
    /// Structural fleet ID of the surviving host, for internal remap logic.
    pub host_fleet_id: u16,
    /// Structural fleet ID of the absorbed fleet, for internal remap logic.
    pub absorbed_fleet_id: u16,
    /// Host fleet number that remained after the merge.
    pub host_fleet_number: u8,
    /// Fleet number that was absorbed/merged away.
    pub absorbed_fleet_number: u8,
    /// Coordinates where the merge occurred.
    pub coords: [u8; 2],
    /// Whether this is the survivor-side "absorbing" report.
    pub survivor_side: bool,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A join mission whose host fleet changed or was lost during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinMissionHostEvent {
    /// The intended host merged into another surviving fleet.
    Retargeted {
        /// Joining fleet index in FLEETS.DAT.
        fleet_idx: usize,
        /// Empire that owned the joining fleet.
        owner_empire: u8,
        /// Previous host fleet number, when still known.
        previous_host_fleet_number: Option<u8>,
        /// New surviving host fleet number, when one specific fleet can be named.
        new_host_fleet_number: Option<u8>,
        /// Current location of the joining fleet.
        coords: [u8; 2],
    },
    /// The intended host was destroyed and the joining fleet abandoned its mission.
    HostDestroyed {
        /// Joining fleet index in FLEETS.DAT.
        fleet_idx: usize,
        /// Empire that owned the joining fleet.
        owner_empire: u8,
        /// Destroyed host fleet number, when still known.
        destroyed_host_fleet_number: Option<u8>,
        /// Current location of the joining fleet.
        coords: [u8; 2],
    },
}

/// A fleet mission whose semantic target changed or disappeared during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRetargetEvent {
    Retargeted {
        fleet_idx: usize,
        /// Player-visible fleet number captured when the event was created.
        reporting_fleet_number: Option<u8>,
        owner_empire: u8,
        mission: Mission,
        current_coords: [u8; 2],
        /// The fleet is in a world's system (depth 0.5 or more); its report
        /// is then headed "System(x,y)" instead of "Sector(x,y)".
        in_system: bool,
        previous_target_coords: [u8; 2],
        new_target_coords: [u8; 2],
        /// Tick (1-52) when this event occurred; None for events from before the year's ticks.
        stardate_week: Option<u8>,
    },
    Abandoned {
        fleet_idx: usize,
        /// Player-visible fleet number captured when the event was created.
        reporting_fleet_number: Option<u8>,
        owner_empire: u8,
        mission: Mission,
        previous_target_coords: [u8; 2],
        coords: [u8; 2],
        /// As for `Retargeted`.
        in_system: bool,
        /// Tick (1-52) when this event occurred; None for events from before the year's ticks.
        stardate_week: Option<u8>,
    },
}

/// The generic outcome class for a mission report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionOutcome {
    Arrived,
    Succeeded,
    Failed,
    Aborted,
}

/// Refines why a mission ended in the generic `Aborted` outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionAbortReason {
    CapabilityLost,
    OrbitBlocked,
    /// The fleet's ROE would not let it fight the force in its way.
    RoeDeclined(BlockingForce),
}

/// A force a fleet's ROE declined to fight: what the abort report tells
/// its owner about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockingForce {
    pub empire: u8,
    /// Set when the force is a single fleet.
    pub fleet_number: Option<u8>,
    pub ships: crate::ShipCounts,
    pub starbases: u32,
}

/// Mission kinds that currently participate in typed maintenance reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mission {
    MoveOnly,
    SeekHome,
    PatrolSector,
    ViewWorld,
    Salvage,
    GuardStarbase,
    GuardBlockadeWorld,
    JoinAnotherFleet,
    RendezvousSector,
    ColonizeWorld,
    BombardWorld,
    InvadeWorld,
    BlitzWorld,
    ScoutSector,
    ScoutSolarSystem,
}

/// A generic mission-resolution report event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionEvent {
    /// Fleet index in FLEETS.DAT that attempted the mission.
    pub fleet_idx: usize,
    /// Empire that owned the acting fleet (1-based player index).
    pub owner_empire: u8,
    /// Mission kind.
    pub kind: Mission,
    /// Resolved outcome class.
    pub outcome: MissionOutcome,
    /// More specific abort reason when the mission was aborted.
    pub abort_reason: Option<MissionAbortReason>,
    /// Target planet index when the mission is planet-directed.
    pub planet_idx: Option<usize>,
    /// Coordinates where the mission resolved, if known.
    pub location_coords: Option<[u8; 2]>,
    /// Original mission target coordinates, if known.
    pub target_coords: Option<[u8; 2]>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncounterDispositionReason {
    RoeDeclined,
    /// The fleet broke off a battle because its ROE no longer accepted the
    /// odds.
    RoeWithdrawal,
    /// The fleet broke off a battle because it had no armed ship left.
    UnarmedEscape,
    /// The fleet meant to attack but the intercept roll failed: "We are
    /// unable to intercept the enemy."
    InterceptFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterDispositionEvent {
    NoEngagement {
        fleet_idx: usize,
        owner_empire: u8,
        mission: Option<Mission>,
        coords: [u8; 2],
        friendly_initial: ShipLosses,
        friendly_loaded_armies_initial: u32,
        target_empire: u8,
        target_fleet_number: Option<u8>,
        small_vessels: u32,
        medium_vessels: u32,
        large_vessels: u32,
        reason: EncounterDispositionReason,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    Retreated {
        fleet_idx: usize,
        owner_empire: u8,
        mission: Option<Mission>,
        coords: [u8; 2],
        friendly_initial: ShipLosses,
        friendly_loaded_armies_initial: u32,
        target_empire: u8,
        target_fleet_number: Option<u8>,
        enemy_initial: ShipLosses,
        /// Where the fleet headed: home, or on to its own target when it
        /// broke off en route and carried on (`resumed`).
        retreat_target_coords: [u8; 2],
        /// The fleet kept its order and carried on toward its target.
        resumed: bool,
        losses_sustained: ShipLosses,
        enemy_losses_inflicted: ShipLosses,
        reason: EncounterDispositionReason,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    /// The fleet broke off a battle and the enemy caught it in a pursuit
    /// exchange before it got away.
    PursuitFire {
        fleet_idx: usize,
        owner_empire: u8,
        mission: Option<Mission>,
        coords: [u8; 2],
        friendly_initial: ShipLosses,
        friendly_loaded_armies_initial: u32,
        target_empire: u8,
        target_fleet_number: Option<u8>,
        enemy_initial: ShipLosses,
        /// Where the fleet headed: home, or on to its own target when it
        /// broke off en route and carried on (`resumed`).
        retreat_target_coords: [u8; 2],
        /// The fleet kept its order and carried on toward its target.
        resumed: bool,
        losses_sustained: ShipLosses, // What the withdrawing fleet lost
        enemy_losses_inflicted: ShipLosses, // What the withdrawing fleet inflicted on pursuer
        reason: EncounterDispositionReason,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidPlayerStateEvent {
    FleetMission {
        fleet_idx: usize,
        owner_empire: u8,
        order_code: u8,
        coords: [u8; 2],
        reason: FleetOrderValidationError,
    },
    FleetInput {
        fleet_idx: usize,
        owner_empire: u8,
        coords: [u8; 2],
        reason: FleetPlayerInputValidationError,
    },
    PlanetInput {
        planet_idx: usize,
        owner_empire: u8,
        coords: [u8; 2],
        reason: PlanetPlayerInputValidationError,
    },
    PlayerTaxRate {
        player_idx: usize,
        owner_empire: u8,
        tax_rate: u8,
    },
    DiplomacyInput {
        player_idx: usize,
        owner_empire: u8,
        reason: PlayerDiplomacyValidationError,
    },
}

/// A fleet scrapped at a world of its empire. Salvage checks its target
/// world's owner every tick, so a fleet only ever arrives at its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalvageResolvedEvent {
    Succeeded {
        fleet_idx: usize,
        /// Number of the salvaged fleet, which is removed before reporting.
        fleet_number: u8,
        owner_empire: u8,
        planet_idx: usize,
        coords: [u8; 2],
        recovered_points: u32,
        /// Armies the fleet's loaded transports landed on the world.
        armies_landed: u16,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiplomacyOverride {
    pub from_empire: u8,
    pub to_empire: u8,
    pub relation: DiplomaticRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiplomaticEscalationEvent {
    pub left_empire: u8,
    pub right_empire: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilDisorderEvent {
    pub reporting_empire: u8,
    pub prior_label: String,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignOutlookEvent {
    pub empire: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignOutcomeEvent {
    pub emperor_empire: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameVictoryNoticeEvent {
    pub recipient_empire: u8,
    pub winner_empire: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpireEliminationCause {
    LastPlanetLost,
    LastRecoveryForceDestroyed,
    RecoveryWindowExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpireEliminationEvent {
    pub defeated_empire: u8,
    pub victor_empire: Option<u8>,
    pub cause: EmpireEliminationCause,
    pub planet_idx: Option<usize>,
    pub coords: Option<[u8; 2]>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetDefectionEvent {
    pub reporting_empire: u8,
    pub fleet_number: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A colonization outcome resolved during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColonizationResolvedEvent {
    /// The fleet established a new colony.
    Succeeded {
        /// Fleet index in FLEETS.DAT that completed the mission.
        fleet_idx: usize,
        /// Planet index (into PLANETS.DAT records) that was colonized.
        planet_idx: usize,
        /// Empire that established the colony (1-based player index).
        colonizer_empire: u8,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    /// The fleet reached the target world but it was already occupied.
    BlockedByOwner {
        /// Fleet index in FLEETS.DAT that attempted the mission.
        fleet_idx: usize,
        /// Planet index (into PLANETS.DAT records) that blocked colonization.
        planet_idx: usize,
        /// Empire that attempted the colony mission (1-based player index).
        colonizer_empire: u8,
        /// Current owner of the world (1-based player index).
        owner_empire: u8,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    /// Hostile action or ship losses forced the fleet to abandon the colony attempt.
    Aborted {
        /// Fleet index in FLEETS.DAT that attempted the mission.
        fleet_idx: usize,
        /// Empire that attempted the colony mission (1-based player index).
        colonizer_empire: u8,
        /// Coordinates where the mission was aborted.
        coords: [u8; 2],
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
}

/// A world under Scorch-Earth, after the year's planet pass decayed it.
/// Amounts are the whole numbers a report shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetScorchEvent {
    /// Planet index (into PLANETS.DAT records) that was scorched.
    pub planet_idx: usize,
    /// Owner of the world (1-based player index).
    pub owner_empire: u8,
    pub production_before: u16,
    pub production_after: u16,
    /// Spendable stored points; a debt reads as 0.
    pub goods_before: u32,
    pub goods_after: u32,
}

/// What a recovery step gave back: a fleet's or a starbase's crippled hulls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredUnit {
    Fleet { fleet_number: u16 },
    Starbase { base_id: u16 },
}

/// Crippled hulls that recovered to full strength at the end of the pass.
/// The report says how many ships came back, and how many of them a
/// starbase's help brought back a pass early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipsRecoveredEvent {
    pub owner_empire: u8,
    pub unit: RecoveredUnit,
    pub coords: [u8; 2],
    /// Inside the world's system rather than in deep space.
    pub in_system: bool,
    pub hulls: u32,
    /// Of `hulls`, those a starbase's help brought back a pass early.
    pub early: u32,
}

/// Events produced by a single maintenance turn, for use by callers
/// (e.g. DATABASE.DAT regeneration in the CLI layer).
#[derive(Debug, Clone, Default)]
pub struct MaintenanceEvents {
    /// Bombardment events: each describes one fleet-vs-planet bombardment.
    pub bombard_events: Vec<BombardEvent>,
    /// Planet intel refresh events generated by combat.
    pub planet_intel_events: Vec<PlanetIntelEvent>,
    /// Ownership changes caused by combat.
    pub ownership_change_events: Vec<PlanetOwnershipChangeEvent>,
    /// Fleet battle summaries for later reporting layers.
    pub fleet_battle_events: Vec<FleetBattleEvent>,
    /// Command-center reports for fleets that were totally destroyed.
    pub fleet_destroyed_events: Vec<FleetDestroyedEvent>,
    /// Command-center reports for starbases that were totally destroyed.
    pub starbase_destroyed_events: Vec<StarbaseDestroyedEvent>,
    /// Starbase arrival and lost-world reports.
    pub starbase_report_events: Vec<StarbaseReportEvent>,
    /// Attacker-side invade/blitz reports with bilateral ground losses.
    pub assault_report_events: Vec<AssaultReportEvent>,
    /// Scout-style hostile contact reports.
    pub scout_contact_events: Vec<ScoutContactEvent>,
    /// Starbases that identified an alien starbase.
    pub starbase_sighting_events: Vec<StarbaseSightingEvent>,
    /// Contact/retreat outcomes driven by ROE and hostile encounters.
    pub encounter_disposition_events: Vec<EncounterDispositionEvent>,
    /// Sanitization reports for invalid player-authored state.
    pub invalid_player_state_events: Vec<InvalidPlayerStateEvent>,
    /// Friendly merge reports for join/rendezvous outcomes.
    pub fleet_merge_events: Vec<FleetMergeEvent>,
    /// Join mission host retarget/destruction reports.
    pub join_host_events: Vec<JoinMissionHostEvent>,
    /// Semantic mission target refresh/abandon reports.
    pub mission_retarget_events: Vec<MissionRetargetEvent>,
    /// Successful colonization outcomes.
    pub colonization_events: Vec<ColonizationResolvedEvent>,
    /// Generic mission outcomes for report generation.
    pub mission_events: Vec<MissionEvent>,
    /// Salvage mission outcomes with recovered production detail.
    pub salvage_events: Vec<SalvageResolvedEvent>,
    /// Diplomatic escalations caused by hostile action during maint.
    pub diplomatic_escalation_events: Vec<DiplomaticEscalationEvent>,
    /// Empires that fell into civil disorder this turn.
    pub civil_disorder_events: Vec<CivilDisorderEvent>,
    /// A new sole remaining serious contender emerged this turn.
    pub campaign_outlook_events: Vec<CampaignOutlookEvent>,
    /// A stable sole contender is now recognized as emperor.
    pub campaign_outcome_events: Vec<CampaignOutcomeEvent>,
    /// Targeted game-over notices emitted when a winner is declared.
    pub game_victory_notice_events: Vec<GameVictoryNoticeEvent>,
    /// Empires that were eliminated this turn.
    pub empire_elimination_events: Vec<EmpireEliminationEvent>,
    /// A civil-disorder empire lost another fleet to defection.
    pub fleet_defection_events: Vec<FleetDefectionEvent>,
    /// Worlds the yearly pass decayed under Scorch-Earth.
    pub planet_scorch_events: Vec<PlanetScorchEvent>,
    /// Crippled hulls that recovered when the pass ended.
    pub ship_recovery_events: Vec<ShipsRecoveredEvent>,
    /// Empires the inactivity rule put on autopilot before this turn.
    pub autopilot_engaged_empires: Vec<u8>,
}

impl MaintenanceEvents {
    /// Append another turn's events, so a multi-turn run reports every
    /// turn. The destructuring names every field: adding one to the struct
    /// fails here until it is carried too.
    pub fn extend(&mut self, other: MaintenanceEvents) {
        let MaintenanceEvents {
            bombard_events,
            planet_intel_events,
            ownership_change_events,
            fleet_battle_events,
            fleet_destroyed_events,
            starbase_destroyed_events,
            starbase_report_events,
            assault_report_events,
            scout_contact_events,
            starbase_sighting_events,
            encounter_disposition_events,
            invalid_player_state_events,
            fleet_merge_events,
            join_host_events,
            mission_retarget_events,
            colonization_events,
            mission_events,
            salvage_events,
            diplomatic_escalation_events,
            civil_disorder_events,
            campaign_outlook_events,
            campaign_outcome_events,
            game_victory_notice_events,
            empire_elimination_events,
            fleet_defection_events,
            planet_scorch_events,
            ship_recovery_events,
            autopilot_engaged_empires,
        } = other;
        self.bombard_events.extend(bombard_events);
        self.planet_intel_events.extend(planet_intel_events);
        self.ownership_change_events.extend(ownership_change_events);
        self.fleet_battle_events.extend(fleet_battle_events);
        self.fleet_destroyed_events.extend(fleet_destroyed_events);
        self.starbase_destroyed_events
            .extend(starbase_destroyed_events);
        self.starbase_report_events.extend(starbase_report_events);
        self.assault_report_events.extend(assault_report_events);
        self.scout_contact_events.extend(scout_contact_events);
        self.starbase_sighting_events
            .extend(starbase_sighting_events);
        self.encounter_disposition_events
            .extend(encounter_disposition_events);
        self.invalid_player_state_events
            .extend(invalid_player_state_events);
        self.fleet_merge_events.extend(fleet_merge_events);
        self.join_host_events.extend(join_host_events);
        self.mission_retarget_events.extend(mission_retarget_events);
        self.colonization_events.extend(colonization_events);
        self.mission_events.extend(mission_events);
        self.salvage_events.extend(salvage_events);
        self.diplomatic_escalation_events
            .extend(diplomatic_escalation_events);
        self.civil_disorder_events.extend(civil_disorder_events);
        self.campaign_outlook_events.extend(campaign_outlook_events);
        self.campaign_outcome_events.extend(campaign_outcome_events);
        self.game_victory_notice_events
            .extend(game_victory_notice_events);
        self.empire_elimination_events
            .extend(empire_elimination_events);
        self.fleet_defection_events.extend(fleet_defection_events);
        self.planet_scorch_events.extend(planet_scorch_events);
        self.ship_recovery_events.extend(ship_recovery_events);
        self.autopilot_engaged_empires
            .extend(autopilot_engaged_empires);
    }
}
