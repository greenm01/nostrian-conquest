//! ECMAINT's continuous fleet motion, stepped in 6-byte reals.
//!
//! A fleet sits at `X = x + fx`, `Y = y + fy` (0.5 is a sector centre) at
//! depth `Z` (0.0 deep space, 1.0 at rest in orbit). A year is 52 ticks;
//! each tick the fleet spends `speed / 52` climbing out to `Z = 0`, flying
//! straight at the target sector centre, then diving toward the target depth
//! (`step`, ECMAINT 0824:01c0). After each tick the order's arrival test runs
//! (dispatcher 0b5d:5581); the first tick that passes resolves the order.

use nc_data::{FleetRecord, Order, Real48};

pub(crate) const TICKS_PER_YEAR: u8 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Motion {
    pub x: Real48,
    pub y: Real48,
    pub z: Real48,
}

impl Motion {
    pub(crate) fn of_fleet(fleet: &FleetRecord) -> Self {
        let [x, y] = fleet.current_location_coords_raw();
        Self {
            x: Real48::from_int(i32::from(x)) + fleet.x_fraction(),
            y: Real48::from_int(i32::from(y)) + fleet.y_fraction(),
            z: fleet.depth(),
        }
    }

    /// At rest in orbit at a sector centre.
    pub(crate) fn at_rest(coords: [u8; 2]) -> Self {
        Self {
            x: sector_centre(coords[0]),
            y: sector_centre(coords[1]),
            z: Real48::ONE,
        }
    }

    pub(crate) fn sector(self) -> [u8; 2] {
        [self.x.trunc() as u8, self.y.trunc() as u8]
    }

    pub(crate) fn write_to(self, fleet: &mut FleetRecord) {
        let [x, y] = self.sector();
        fleet.set_current_location_coords_raw([x, y]);
        fleet.set_x_fraction(self.x - Real48::from_int(i32::from(x)));
        fleet.set_y_fraction(self.y - Real48::from_int(i32::from(y)));
        fleet.set_depth(self.z);
    }
}

fn sector_centre(coord: u8) -> Real48 {
    Real48::from_int(i32::from(coord)) + Real48::HALF
}

/// Target depth `TZ`: the 0x22 world-target flag, capped at 1.
pub(crate) fn target_depth(world_target_flag: u8) -> Real48 {
    Real48::from_int(i32::from(world_target_flag.min(1)))
}

/// The sector centres a fleet flies through; the last one is the order
/// target. Intermediate waypoints come from hazard-aware routing only.
#[derive(Debug, Clone)]
pub(crate) struct Course {
    waypoints: Vec<[u8; 2]>,
    next: usize,
}

impl Course {
    pub(crate) fn direct(target: [u8; 2]) -> Self {
        Self {
            waypoints: vec![target],
            next: 0,
        }
    }

    pub(crate) fn through(waypoints: Vec<[u8; 2]>, target: [u8; 2]) -> Self {
        let mut waypoints = waypoints;
        if waypoints.last() != Some(&target) {
            waypoints.push(target);
        }
        Self { waypoints, next: 0 }
    }

    fn aim(&self) -> [u8; 2] {
        self.waypoints[self.next]
    }

    fn on_final_leg(&self) -> bool {
        self.next + 1 >= self.waypoints.len()
    }
}

/// One tick of motion (ECMAINT 0824:01c0), in the runtime's real arithmetic.
pub(crate) fn step(mut motion: Motion, speed: u8, course: &mut Course, depth: Real48) -> Motion {
    let mut budget = Real48::from_int(i32::from(speed)) / Real48::from_int(52);
    let leg_depth = if course.on_final_leg() {
        depth
    } else {
        Real48::ZERO
    };
    let aim = course.aim();
    let (aim_x, aim_y) = (sector_centre(aim[0]), sector_centre(aim[1]));
    if aim_x.trunc() != motion.x.trunc()
        || aim_y.trunc() != motion.y.trunc()
        || leg_depth < motion.z
    {
        if budget > motion.z {
            budget = budget - motion.z;
            motion.z = Real48::ZERO;
        } else {
            motion.z = motion.z - budget;
            budget = Real48::ZERO;
        }
    }
    while budget > Real48::ZERO {
        let aim = course.aim();
        let (aim_x, aim_y) = (sector_centre(aim[0]), sector_centre(aim[1]));
        let dx = (aim_x - motion.x).abs();
        let dy = (aim_y - motion.y).abs();
        let distance = (dx * dx + dy * dy).sqrt();
        let travelled = if distance > budget {
            std::mem::replace(&mut budget, Real48::ZERO)
        } else {
            budget = budget - distance;
            distance
        };
        if budget > Real48::ZERO {
            motion.x = aim_x;
            motion.y = aim_y;
            if !course.on_final_leg() {
                course.next += 1;
                continue;
            }
            if depth > Real48::ZERO {
                let dived = budget + motion.z;
                motion.z = if dived > depth { depth } else { dived };
            }
        } else {
            let ratio = travelled / distance;
            motion.x = approach(motion.x, aim_x, ratio);
            motion.y = approach(motion.y, aim_y, ratio);
        }
        break;
    }
    motion
}

fn approach(from: Real48, to: Real48, ratio: Real48) -> Real48 {
    let shift = (from - to).abs() * ratio;
    if from < to {
        shift + from
    } else {
        from - shift
    }
}

/// The arrival test each order runs after every tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrivalRule {
    /// In the target sector at exactly the target depth.
    Settled,
    /// Settled, or at year end in the target sector at least half dived
    /// (Seek Home, Colonize).
    SettledOrHalfDivedAtYearEnd,
    /// In the guarded starbase's sector at least half dived, still under way.
    GuardStarbase,
    /// Bombard/Invade/Blitz: in the target sector, fully dived.
    Strike,
}

impl ArrivalRule {
    pub(crate) fn for_order(order: Order) -> Self {
        match order {
            Order::SeekHome | Order::ColonizeWorld => Self::SettledOrHalfDivedAtYearEnd,
            Order::GuardStarbase => Self::GuardStarbase,
            Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld => Self::Strike,
            _ => Self::Settled,
        }
    }

    pub(crate) fn passes(
        self,
        motion: Motion,
        target: [u8; 2],
        depth: Real48,
        speed: u8,
        last_tick: bool,
    ) -> bool {
        if motion.sector() != target {
            return false;
        }
        let settled = motion.z == depth;
        match self {
            Self::Settled => settled,
            Self::SettledOrHalfDivedAtYearEnd => settled || (last_tick && motion.z.round() == 1),
            Self::GuardStarbase => motion.z >= Real48::HALF && speed > 0,
            Self::Strike => motion.z.round() >= 1 && (motion.z >= Real48::ONE || settled),
        }
    }
}

/// The dispatcher prelude's "arrived" check; a fleet that has not arrived
/// and has speed 0 gets its maximum speed back.
fn prelude_arrived(order: Order, motion: Motion, target: [u8; 2], depth: Real48) -> bool {
    motion.sector() == target
        && motion.z.round() == depth.round()
        && (order == Order::GuardStarbase || motion.z >= depth)
}

/// What one year of ticks did to a fleet.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FleetYear {
    pub motion: Motion,
    pub speed: u8,
    /// Tick whose arrival test passed; the fleet stopped there.
    pub resolved_tick: Option<u8>,
    /// Motion and speed one tick before the resolving tick.
    pub before_resolution: (Motion, u8),
}

pub(crate) fn run_year(
    mut motion: Motion,
    mut speed: u8,
    max_speed: u8,
    order: Order,
    target: [u8; 2],
    depth: Real48,
    course: &mut Course,
) -> FleetYear {
    let rule = ArrivalRule::for_order(order);
    for tick in 1..=TICKS_PER_YEAR {
        let before = (motion, speed);
        motion = step(motion, speed, course, depth);
        if speed == 0 && !prelude_arrived(order, motion, target, depth) {
            speed = max_speed;
        }
        if rule.passes(motion, target, depth, speed, tick == TICKS_PER_YEAR) {
            return FleetYear {
                motion,
                speed,
                resolved_tick: Some(tick),
                before_resolution: before,
            };
        }
    }
    FleetYear {
        motion,
        speed,
        resolved_tick: None,
        before_resolution: (motion, speed),
    }
}
