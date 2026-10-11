//! Classic ECMAINT's encounter rules as pure functions: sighting, strength,
//! ROE, intercept, and the attack decision (`docs/dev/ec-encounter-spec.md`
//! sections 2-4). Randomness comes from the caller's `GameRng`, so Rust
//! matches classic's odds, not its random sequence. NC departs in what a
//! fleet weighs (combat decisions 2 and 6): the sides as they would fight,
//! each empire's fleets in the sector and layer at their NC attack strength,
//! not classic's 21/7/2 for the two fleets alone, and no attack skips the
//! ROE test because the attacker identified its enemy first.

use nc_data::{CrippledShips, GameRng};

use crate::maint::combat::{STARBASE_ATTACK_STRENGTH, warship_attack_strength};

/// Order codes as classic stores them in FLEETS.DAT `+0x1f`.
pub mod order {
    pub const HOLD: u8 = 0;
    pub const PATROL: u8 = 3;
    pub const GUARD_STARBASE: u8 = 4;
    pub const GUARD_BLOCKADE: u8 = 5;
    pub const BOMBARD: u8 = 6;
    pub const INVADE: u8 = 7;
    pub const BLITZ: u8 = 8;
    pub const VIEW: u8 = 9;
    pub const SCOUT_SECTOR: u8 = 10;
    pub const SCOUT_SYSTEM: u8 = 11;
    pub const COLONIZE: u8 = 12;
}

/// What the encounter rules read from a fleet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FleetFacts {
    pub owner: u8,
    /// Classic order code (`order::*`).
    pub order: u8,
    pub sector: [u8; 2],
    /// 1 inside a world's system (depth at least 0.5), else 0.
    pub layer: u8,
    pub target: [u8; 2],
    /// The order's target layer (`+0x22`, capped at 1).
    pub target_layer: u8,
    pub speed: u8,
    pub max_speed: u8,
    pub roe: u8,
    pub battleships: u16,
    pub cruisers: u16,
    pub destroyers: u16,
    /// Troop transports, empty and loaded.
    pub transports: u16,
    pub etacs: u16,
    pub scouts: u16,
    /// The fleet's crippled ships, which count at their crippled AS.
    pub crippled: CrippledShips,
}

impl FleetFacts {
    /// In the target sector at the target layer.
    pub fn at_target(&self) -> bool {
        self.sector == self.target && self.layer == self.target_layer
    }

    /// On Scout Sector or Scout System; scouts identify on first sighting.
    pub fn scouting(&self) -> bool {
        matches!(self.order, order::SCOUT_SECTOR | order::SCOUT_SYSTEM)
    }
}

/// How an observer scans: its sensor factor and whether it closes on
/// contacts to identify them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Posture {
    pub factor: f64,
    pub closing: bool,
}

/// A fleet observer's posture (`18ea:13cf`). The order factors apply only
/// to a fleet that is stopped or at its target.
pub fn fleet_posture(observer: &FleetFacts) -> Posture {
    let mut posture = Posture {
        factor: 1.0,
        closing: observer.roe != 0,
    };
    if observer.speed == 0 || observer.at_target() {
        match observer.order {
            order::PATROL => {
                posture = Posture {
                    factor: 10.0,
                    closing: true,
                }
            }
            order::SCOUT_SECTOR | order::SCOUT_SYSTEM | order::GUARD_BLOCKADE => {
                posture = Posture {
                    factor: 5.0,
                    closing: true,
                }
            }
            order::GUARD_STARBASE => posture.closing = false,
            _ => {}
        }
    }
    posture
}

/// A starbase's sensor factor: 25 in a world's system, 30 in deep space.
pub fn base_factor(layer: u8) -> f64 {
    if layer == 1 { 25.0 } else { 30.0 }
}

/// Every starbase shows the same visibility, `90 / 52`.
pub const BASE_VISIBILITY: f64 = 90.0 / 52.0;

fn order_visibility(order: u8) -> f64 {
    match order {
        order::PATROL | order::SCOUT_SYSTEM | order::COLONIZE => 2.0,
        order::GUARD_BLOCKADE => 1.1,
        order::BOMBARD | order::INVADE | order::BLITZ => 20.0,
        order::VIEW => 1.2,
        order::SCOUT_SECTOR => 1.5,
        _ => 1.0,
    }
}

/// How visible a fleet is (`18ea:007a`). Each class product wraps at 16
/// bits, as in classic.
pub fn fleet_visibility(target: &FleetFacts) -> f64 {
    let heavy = 45u16.wrapping_mul(target.battleships.wrapping_add(target.etacs));
    let medium = 30u16.wrapping_mul(target.cruisers.wrapping_add(target.transports));
    let light = 15u16.wrapping_mul(target.destroyers);
    let mut v = (f64::from(heavy) + f64::from(medium) + f64::from(light)).sqrt();
    if target.layer == 1 {
        v *= 5.0;
    }
    v += (3.0 * f64::from(target.scouts)).sqrt();
    if target.at_target() {
        v *= order_visibility(target.order);
    }
    v *= 1.0 + f64::from(target.speed) / 10.0;
    v / 52.0
}

/// Percent chance per tick that an observer sights a unit it knows at
/// `awareness` (0 unknown, 1 contact, 2 identified, 3 fought).
pub fn sighting_percent(awareness: u8, visibility: f64, factor: f64) -> f64 {
    10.0 * f64::from(awareness) + visibility * factor
}

/// Classic's `Roll(p)`: `Random * 100 <= p`.
pub fn roll(rng: &mut GameRng, percent: f64) -> bool {
    rng.next_real() * 100.0 <= percent
}

/// A fleet's strength for the decision to fight: its NC attack strength,
/// each warship at its current AS (destroyer 10, cruiser 30, battleship 90,
/// a crippled one at half), the yardstick the battle itself uses (combat
/// decisions 2 and 3). Scouts, transports, and ETACs count 0. Classic weighed `21 BB + 7 CA + 2 DD` plus 30 for a
/// guard at rest with its base (`24a3:02b4`); NC does not.
pub fn strength(fleet: &FleetFacts) -> u32 {
    warship_attack_strength(
        fleet.destroyers,
        fleet.cruisers,
        fleet.battleships,
        &fleet.crippled,
    )
}

/// A starbase counts its attack strength, 100 (50 while crippled), once;
/// its guard fleet adds its own. This is the full-strength value; a base's
/// own is `base_attack_strength`.
pub const BASE_STRENGTH: u32 = STARBASE_ATTACK_STRENGTH;

/// The ROE test (`1b05:0706`): would a force of strength `own` take on one of
/// strength `enemy`? ROE above 10 counts as 10.
pub fn roe_permits(roe: u8, own: u32, enemy: u32) -> bool {
    let (own, enemy) = (f64::from(own), f64::from(enemy));
    let at_least = |ratio: f64| enemy == 0.0 || own / enemy >= ratio;
    let at_most = |ratio: f64| own > 0.0 && enemy / own <= ratio;
    match roe {
        0 => false,
        1 => enemy == 0.0,
        2 => at_least(4.0),
        3 => at_least(3.0),
        4 => at_least(2.0),
        5 => at_least(1.5),
        6 => at_least(1.0),
        7 => at_most(1.5),
        8 => at_most(2.0),
        9 => at_most(3.0),
        _ => true,
    }
}

/// Whether an attacker catches its target (`1b05:03b1`). A faster attacker
/// always catches; otherwise the target notices with `100 - 33 * awareness`
/// percent (the target empire's awareness of the attacker) and runs, and an
/// unnoticed target fights only if its own ROE accepts the odds of the sides
/// as they would fight.
pub fn intercept(
    rng: &mut GameRng,
    attacker: &FleetFacts,
    target: &FleetFacts,
    situation: Situation,
) -> bool {
    if attacker.max_speed > target.max_speed {
        return true;
    }
    if roll(rng, 100.0 - 33.0 * f64::from(situation.target_awareness)) {
        let arc = rng.next_real();
        let roll = rng.next_real();
        return speed_roll_catches(attacker.max_speed, target.speed, arc, roll);
    }
    roe_permits(target.roe, situation.enemy_strength, situation.own_strength)
}

/// Which of `candidates`, each a fleet's ROE and strength, volunteer to
/// attack a side of strength `enemy`: a fleet with ROE above 0 and warships
/// whose ROE accepts the volunteers' assembled strength against the enemy's,
/// recomputed with every refusal until none refuses. A fleet dropped is not
/// re-admitted.
pub fn admitted(candidates: &[(u8, u32)], enemy: u32) -> Vec<bool> {
    let mut admitted: Vec<bool> = candidates
        .iter()
        .map(|&(roe, strength)| roe > 0 && strength > 0)
        .collect();
    loop {
        let own: u32 = candidates
            .iter()
            .zip(&admitted)
            .filter(|(_, stays)| **stays)
            .map(|(&(_, strength), _)| strength)
            .sum();
        let mut dropped = false;
        for (&(roe, _), stays) in candidates.iter().zip(&mut admitted) {
            if *stays && !roe_permits(roe, own, enemy) {
                *stays = false;
                dropped = true;
            }
        }
        if !dropped {
            return admitted;
        }
    }
}

/// The intercept's speed roll, with `arc` and `roll` uniform in [0, 1): a
/// pursuer of maximum speed `pursuer_max` catches a target running at
/// `speed` when `roll * 100 <= 100 * sin(pi * arc) * pursuer_max / speed`.
/// A stopped target is caught.
pub fn speed_roll_catches(pursuer_max: u8, speed: u8, arc: f64, roll: f64) -> bool {
    if speed == 0 {
        return true;
    }
    let chance =
        100.0 * (std::f64::consts::PI * arc).sin() * f64::from(pursuer_max) / f64::from(speed);
    roll * 100.0 <= chance
}

/// What a decision said, for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Not hostile: "Ignoring alien fleet".
    Ignored,
    /// Hostile, but ROE forbids the fight: "avoiding this enemy fleet".
    Avoided,
    /// Willing and caught it: attack.
    Engaged,
    /// Willing but it got away: "unable to intercept".
    Escaped,
    /// ROE forbids fighting a force that blocks the mission: abort to Seek
    /// Home (a fleet, `24a3:0876`).
    Aborted,
}

impl Verdict {
    /// The decision starts a battle.
    pub fn attacks(self) -> bool {
        self == Verdict::Engaged
    }
}

/// What the decision needs besides the two fleets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// The owner of the world in the observer's sector (0 unowned), looked up
    /// only when the observer is in the system. `None` in deep space and in a
    /// sector with no world: such a sector is nobody's system (a deliberate
    /// difference from classic, which reads a garbage owner there).
    pub world_owner: Option<u8>,
    /// The observer's empire has declared the target's owner an enemy.
    pub declared_enemy: bool,
    /// The target empire's awareness of the observer, for the intercept.
    pub target_awareness: u8,
    /// The strength of the observer's side as it would fight: the fleets of
    /// its empire in the sector and layer that `admitted` lets volunteer
    /// against `enemy_strength`.
    pub own_strength: u32,
    /// The strength of the target's side as it would fight: its empire's
    /// fleets in the sector and layer, with the starbases they bring.
    pub enemy_strength: u32,
}

impl Situation {
    /// The observer is among its side's volunteers: its ROE accepts the
    /// assembled odds. A fleet without warships never volunteers.
    fn willing(&self, o: &FleetFacts) -> bool {
        strength(o) > 0 && roe_permits(o.roe, self.own_strength, self.enemy_strength)
    }
}

fn engage(
    rng: &mut GameRng,
    willing: bool,
    o: &FleetFacts,
    t: &FleetFacts,
    situation: Situation,
) -> Verdict {
    if !willing {
        Verdict::Avoided
    } else if intercept(rng, o, t, situation) {
        Verdict::Engaged
    } else {
        Verdict::Escaped
    }
}

/// Should observer `o` attack fleet `t` (`1b05:1277`)? Classic lets any ROE
/// above 0 attack at first identification; NC does not (combat decision 6):
/// every attack passes the ROE test on the sides as they would fight, at
/// first identification and at every later re-check alike.
pub fn decide_vs_fleet(
    rng: &mut GameRng,
    o: &FleetFacts,
    t: &FleetFacts,
    situation: Situation,
) -> Verdict {
    let willing = situation.willing(o);
    let unowned_world = situation.world_owner == Some(0);
    // Hostile-mission block: both on world missions at this world.
    if o.layer == 1
        && matches!(
            o.order,
            order::GUARD_BLOCKADE | order::BOMBARD | order::INVADE | order::BLITZ | order::COLONIZE
        )
        && o.sector == o.target
        && t.at_target()
        && matches!(
            t.order,
            order::GUARD_STARBASE
                | order::GUARD_BLOCKADE
                | order::BOMBARD
                | order::INVADE
                | order::BLITZ
                | order::COLONIZE
        )
    {
        let assault = matches!(t.order, order::BOMBARD | order::INVADE | order::BLITZ);
        if o.order == order::GUARD_BLOCKADE
            && (assault || (t.order == order::COLONIZE && unowned_world))
        {
            return engage(rng, willing, o, t, situation);
        }
        if o.order == order::COLONIZE && t.order == order::COLONIZE && unowned_world {
            return engage(rng, willing, o, t, situation);
        }
        if o.order != order::GUARD_BLOCKADE
            && matches!(t.order, order::GUARD_STARBASE | order::GUARD_BLOCKADE)
            && t.layer == 1
        {
            if !willing {
                return Verdict::Aborted;
            }
            return engage(rng, willing, o, t, situation);
        }
    }
    if situation.declared_enemy {
        return engage(rng, willing, o, t, situation);
    }
    // A neutral fleet inside one of the observer's systems.
    if o.layer == 1 && o.owner != 0 && situation.world_owner == Some(o.owner) {
        return engage(rng, willing, o, t, situation);
    }
    Verdict::Ignored
}

/// Should observer `o` attack a starbase (`1b05:1d11`)? `own` is the
/// strength of the observer's side as it would fight and `enemy` the base's
/// side: the base, its guard, and its owner's other fleets there. Classic
/// weighed the base alone against a declared enemy; NC weighs the sides as
/// they would fight (combat decision 6). There is no intercept roll.
pub fn decide_vs_base(o: &FleetFacts, own: u32, enemy: u32, declared_enemy: bool) -> Verdict {
    let willing = strength(o) > 0 && roe_permits(o.roe, own, enemy);
    if o.layer == 1
        && matches!(o.order, order::BOMBARD | order::INVADE | order::BLITZ)
        && o.sector == o.target
        && o.target_layer == 1
    {
        // The base protects the assault's target world.
        if willing {
            Verdict::Engaged
        } else {
            Verdict::Aborted
        }
    } else if declared_enemy {
        if willing {
            Verdict::Engaged
        } else {
            Verdict::Avoided
        }
    } else {
        Verdict::Ignored
    }
}
