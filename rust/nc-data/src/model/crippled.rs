//! Battle damage that lasts: the hulls a fleet or starbase has crippled, and
//! how long each needs to recover. Not a classic field: a classic export
//! writes whole hulls and drops this.
//!
//! RT, Recovery Turns, is one game-wide rule: every class recovers in
//! [`RECOVERY_TURNS`] full passes, so a fleet keeps counts, not per-hull
//! records. A hull crippled in pass N is crippled through N+1, recovers at
//! the end of N+2, and fights whole from N+3.

/// Recovery Turns: the full passes a crippled hull needs to recover on its
/// own. The pass that cripples it does not count.
pub const RECOVERY_TURNS: usize = 2;

/// A ship class that can be crippled. Starbases are the lone hull of their
/// record and use [`CrippledHulls`] directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HullClass {
    Destroyer,
    Cruiser,
    Battleship,
    Scout,
    /// Troop transports, empty and loaded.
    Transport,
    Etac,
}

impl HullClass {
    pub const ALL: [HullClass; 6] = [
        HullClass::Destroyer,
        HullClass::Cruiser,
        HullClass::Battleship,
        HullClass::Scout,
        HullClass::Transport,
        HullClass::Etac,
    ];

    /// The class's number in the campaign store.
    pub fn code(self) -> u8 {
        match self {
            Self::Destroyer => 0,
            Self::Cruiser => 1,
            Self::Battleship => 2,
            Self::Scout => 3,
            Self::Transport => 4,
            Self::Etac => 5,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|class| class.code() == code)
    }
}

/// One class's crippled hulls, counted by how long they have left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CrippledHulls {
    /// `turns[i]` hulls need `i + 1` more recovery steps.
    pub turns: [u32; RECOVERY_TURNS],
    /// Hulls crippled during the current pass. They enter at
    /// [`RECOVERY_TURNS`] at the recovery step that ends the pass.
    pub fresh: u32,
}

/// What a recovery step gave back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Recovered {
    /// Hulls that are whole again.
    pub hulls: u32,
    /// Of those, hulls that starbase assistance brought back a pass early.
    pub early: u32,
}

impl CrippledHulls {
    /// Every crippled hull, whatever its time.
    pub fn total(&self) -> u32 {
        self.turns.iter().sum::<u32>() + self.fresh
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// `count` more hulls crippled in the current pass.
    pub fn cripple(&mut self, count: u32) {
        self.fresh += count;
    }

    /// Merging adds the counts, this pass's included.
    pub fn add(&mut self, other: &CrippledHulls) {
        for (turns, extra) in self.turns.iter_mut().zip(other.turns) {
            *turns += extra;
        }
        self.fresh += other.fresh;
    }

    /// Removes up to `count` hulls and returns them. Hulls crippled this
    /// pass go first, then the ones with the most turns left, so the hulls
    /// that stay are the closest to recovery. Finishing fire and detaching
    /// both take them in this order.
    pub fn take(&mut self, count: u32) -> CrippledHulls {
        let mut taken = CrippledHulls::default();
        let mut left = count;
        taken.fresh = left.min(self.fresh);
        self.fresh -= taken.fresh;
        left -= taken.fresh;
        for i in (0..RECOVERY_TURNS).rev() {
            taken.turns[i] = left.min(self.turns[i]);
            self.turns[i] -= taken.turns[i];
            left -= taken.turns[i];
        }
        taken
    }

    /// The hulls of `self` (an assisted set, `fresh` ignored) that `present`
    /// still has: for each turns-left count, the smaller of the two. Hulls
    /// that were never assisted are lost first, so a battle or a merge costs
    /// assistance only once it eats into the assisted hulls themselves.
    pub fn limited_to(&self, present: &CrippledHulls) -> CrippledHulls {
        let mut kept = CrippledHulls::default();
        for i in 0..RECOVERY_TURNS {
            kept.turns[i] = self.turns[i].min(present.turns[i]);
        }
        kept
    }

    /// The recovery step: every turns-remaining count drops by one, hulls
    /// that reach zero are whole again, and this pass's newly crippled hulls
    /// enter at [`RECOVERY_TURNS`]. `assisted` names hulls (by their turns
    /// left, `fresh` ignored) that get one extra turn from a starbase.
    pub fn recover(&mut self, assisted: &CrippledHulls) -> Recovered {
        let mut next = [0u32; RECOVERY_TURNS];
        let mut recovered = Recovered::default();
        for (i, &count) in self.turns.iter().enumerate() {
            let turns = i + 1;
            let helped = assisted.turns[i].min(count);
            for (hulls, left) in [
                (count - helped, turns - 1),
                (helped, turns.saturating_sub(2)),
            ] {
                match left.checked_sub(1) {
                    None => recovered.hulls += hulls,
                    Some(slot) => next[slot] += hulls,
                }
            }
            if turns == 2 {
                recovered.early += helped;
            }
        }
        next[RECOVERY_TURNS - 1] += self.fresh;
        self.turns = next;
        self.fresh = 0;
        recovered
    }
}

/// A fleet's crippled hulls by class. Each class's crippled hulls are a
/// subset of that class's ships.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CrippledShips {
    pub destroyers: CrippledHulls,
    pub cruisers: CrippledHulls,
    pub battleships: CrippledHulls,
    pub scouts: CrippledHulls,
    pub transports: CrippledHulls,
    pub etacs: CrippledHulls,
}

impl CrippledShips {
    pub fn get(&self, class: HullClass) -> &CrippledHulls {
        match class {
            HullClass::Destroyer => &self.destroyers,
            HullClass::Cruiser => &self.cruisers,
            HullClass::Battleship => &self.battleships,
            HullClass::Scout => &self.scouts,
            HullClass::Transport => &self.transports,
            HullClass::Etac => &self.etacs,
        }
    }

    pub fn get_mut(&mut self, class: HullClass) -> &mut CrippledHulls {
        match class {
            HullClass::Destroyer => &mut self.destroyers,
            HullClass::Cruiser => &mut self.cruisers,
            HullClass::Battleship => &mut self.battleships,
            HullClass::Scout => &mut self.scouts,
            HullClass::Transport => &mut self.transports,
            HullClass::Etac => &mut self.etacs,
        }
    }

    pub fn total(&self) -> u32 {
        HullClass::ALL
            .into_iter()
            .map(|class| self.get(class).total())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    pub fn add(&mut self, other: &CrippledShips) {
        for class in HullClass::ALL {
            self.get_mut(class).add(other.get(class));
        }
    }
}
