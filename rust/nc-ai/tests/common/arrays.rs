//! Batch action arrays for tests: owned arrays, and seeded random choices
//! within the masks.
#![allow(dead_code)]

use nc_ai::{
    BUILD_AMOUNT_CHOICES, BUILD_CHOICES, BUILD_SLOTS, BatchActions, BatchEnv, BatchMasks,
    DETACH_CHOICES, MISSION_CHOICES, ROE_CHOICES, SPEED_CHOICES, SeatAction, TAX_CHOICES,
};

/// Owned action arrays, borrowed as [`BatchActions`].
#[derive(Clone)]
pub struct Arrays {
    pub tax: Vec<i64>,
    pub diplomacy: Vec<i64>,
    pub build: Vec<i64>,
    pub amount: Vec<i64>,
    pub mission: Vec<i64>,
    pub target: Vec<i64>,
    pub speed: Vec<i64>,
    pub roe: Vec<i64>,
    pub load: Vec<i64>,
    pub detach: Vec<i64>,
}

impl Arrays {
    pub fn zeros(env: &BatchEnv) -> Self {
        let seats = env.games() * usize::from(env.players());
        let fleets = vec![0; seats * env.fleets()];
        Arrays {
            tax: vec![0; seats],
            diplomacy: vec![0; seats * usize::from(env.players())],
            build: vec![0; seats * env.worlds() * BUILD_SLOTS],
            amount: vec![0; seats * env.worlds() * BUILD_SLOTS],
            mission: fleets.clone(),
            target: fleets.clone(),
            speed: fleets.clone(),
            roe: fleets.clone(),
            load: fleets.clone(),
            detach: fleets,
        }
    }

    pub fn view(&self) -> BatchActions<'_> {
        BatchActions {
            tax: &self.tax,
            diplomacy: &self.diplomacy,
            build: &self.build,
            amount: &self.amount,
            mission: &self.mission,
            target: &self.target,
            speed: &self.speed,
            roe: &self.roe,
            load: &self.load,
            detach: &self.detach,
        }
    }

    /// Write `action` for flat seat index `at`.
    pub fn set(&mut self, env: &BatchEnv, at: usize, action: &SeatAction) {
        let (seats, worlds, fleets) = (usize::from(env.players()), env.worlds(), env.fleets());
        self.tax[at] = action.tax as i64;
        for (other, &choice) in action.diplomacy.iter().enumerate() {
            self.diplomacy[at * seats + other] = choice as i64;
        }
        for planet in &action.planets {
            let world = (at * worlds + planet.record - 1) * BUILD_SLOTS;
            for slot in 0..BUILD_SLOTS {
                self.build[world + slot] = planet.build[slot] as i64;
                self.amount[world + slot] = planet.amount[slot] as i64;
            }
        }
        for (row, fleet) in action.fleets.iter().enumerate() {
            let idx = at * fleets + row;
            self.mission[idx] = fleet.mission as i64;
            self.target[idx] = fleet.target as i64;
            self.speed[idx] = fleet.speed as i64;
            self.roe[idx] = fleet.roe as i64;
            self.load[idx] = i64::from(fleet.load);
            self.detach[idx] = fleet.detach as i64;
        }
    }
}

/// Seeded splitmix64.
pub struct Rng(pub u64);

impl Rng {
    pub fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }

    pub fn allowed(&mut self, mask: &[bool]) -> i64 {
        let allowed: Vec<usize> = (0..mask.len()).filter(|&idx| mask[idx]).collect();
        assert!(!allowed.is_empty(), "every mask row allows something");
        allowed[self.below(allowed.len())] as i64
    }
}

/// Uniform random choices within `masks`, padding rows included.
pub fn random_arrays(env: &BatchEnv, masks: &BatchMasks, rng: &mut Rng) -> Arrays {
    let mut arrays = Arrays::zeros(env);
    let rows = |mask: &[bool], width: usize| mask.chunks(width).map(<[bool]>::to_vec).collect();
    let diplomacy: Vec<Vec<bool>> = rows(&masks.diplomacy.data, 3);
    let build: Vec<Vec<bool>> = rows(&masks.build.data, BUILD_CHOICES);
    let mission: Vec<Vec<bool>> = rows(&masks.mission.data, MISSION_CHOICES);
    let target: Vec<Vec<bool>> = rows(&masks.target.data, env.worlds());
    let roe: Vec<Vec<bool>> = rows(&masks.roe.data, ROE_CHOICES);
    let detach: Vec<Vec<bool>> = rows(&masks.detach.data, DETACH_CHOICES);
    for at in 0..arrays.tax.len() {
        arrays.tax[at] = rng.below(TAX_CHOICES) as i64;
    }
    for (idx, row) in diplomacy.iter().enumerate() {
        // Mostly keep relations, so games are not all-out war.
        arrays.diplomacy[idx] = if rng.below(8) == 0 {
            rng.allowed(row)
        } else {
            0
        };
    }
    for (idx, row) in build.iter().enumerate() {
        for slot in 0..BUILD_SLOTS {
            arrays.build[idx * BUILD_SLOTS + slot] = rng.allowed(row);
            arrays.amount[idx * BUILD_SLOTS + slot] = rng.below(BUILD_AMOUNT_CHOICES) as i64;
        }
    }
    for (idx, row) in mission.iter().enumerate() {
        let chosen = rng.allowed(row);
        arrays.mission[idx] = chosen;
        arrays.target[idx] = rng.allowed(&target[idx * MISSION_CHOICES + chosen as usize]);
        arrays.speed[idx] = rng.below(SPEED_CHOICES) as i64;
        arrays.roe[idx] = rng.allowed(&roe[idx]);
        arrays.load[idx] = i64::from(masks.load.data[idx] && rng.below(2) == 0);
        arrays.detach[idx] = if rng.below(8) == 0 {
            rng.allowed(&detach[idx])
        } else {
            0
        };
    }
    arrays
}
