use rand::random;

use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

const STREAM_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

pub const RNG_TAG_MAPGEN: u64 = 0xEC15_4D41_5047_454E;
pub const RNG_TAG_COMBAT: u64 = 0xEC15_434F_4D42_4154;
pub const RNG_TAG_ENCOUNTER: u64 = 0xEC15_454E_434F_554E;

#[derive(Debug, Clone)]
pub struct GameRng {
    state: u64,
}

impl GameRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            state: mix_seed(seed ^ 0xEC15_0000_0000_0001),
        }
    }

    pub fn from_context(seed: u64, tag: u64, context: &[u64]) -> Self {
        let mut mixed = mix_seed(seed ^ tag);
        for &value in context {
            mixed = mix_seed(mixed ^ value);
        }
        Self::from_seed(mixed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(STREAM_GAMMA);
        splitmix64(self.state)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }

    pub fn next_usize(&mut self) -> usize {
        (self.next_u64() >> 32) as usize
    }

    /// A real in [0, 1), like Turbo Pascal's `Random`.
    pub fn next_real(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// An integer in [0, n), like Turbo Pascal's `Random(n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    pub fn next_f32(&mut self) -> f32 {
        let value = self.next_u32() as f64 / u32::MAX as f64;
        value as f32
    }

    pub fn range_u8(&mut self, min: u8, max: u8) -> u8 {
        if min >= max {
            return min;
        }
        let span = max - min + 1;
        min + (self.next_u8() % span)
    }

    pub fn roll_d10(&mut self) -> u8 {
        self.next_u8() % 10
    }
}

pub fn generate_campaign_seed() -> u64 {
    let seed = random::<u64>() ^ 0xEC15_CAFE_5EED_0001;
    if seed == 0 {
        0xEC15_CAFE_5EED_0001
    } else {
        seed
    }
}

pub fn derive_campaign_seed_from_runtime(
    game_data: &CoreGameData,
    report_block_rows: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
) -> u64 {
    let mut hash = 0xEC15_0000_0000_0001u64;
    hash = hash_bytes(hash, &game_data.player.to_bytes());
    hash = hash_bytes(hash, &game_data.planets.to_bytes());
    hash = hash_bytes(hash, &game_data.fleets.to_bytes());
    hash = hash_bytes(hash, &game_data.bases.to_bytes());
    hash = hash_bytes(hash, &game_data.ipbm.to_bytes());
    hash = hash_bytes(hash, &game_data.setup.to_bytes());
    hash = hash_bytes(hash, &game_data.conquest.to_bytes());

    for row in report_block_rows {
        hash = hash_u64(hash, row.block_index as u64);
        hash = hash_bytes(hash, row.decoded_text.as_bytes());
        hash = hash_u64(hash, u64::from(row.recipient_deleted));
        if let Some(raw) = row.raw_bytes.as_ref() {
            hash = hash_bytes(hash, raw);
        }
    }

    for mail in queued_mail {
        hash = hash_u64(hash, u64::from(mail.sender_empire_id));
        hash = hash_u64(hash, u64::from(mail.recipient_empire_id));
        hash = hash_u64(hash, u64::from(mail.year));
        hash = hash_bytes(hash, mail.subject.as_bytes());
        hash = hash_bytes(hash, mail.body.as_bytes());
        hash = hash_u64(hash, u64::from(mail.recipient_deleted));
    }

    mix_seed(hash)
}

pub fn mix_seed(seed: u64) -> u64 {
    let mixed = splitmix64(seed ^ 0xA5A5_5A5A_D3C1_B4E7);
    if mixed == 0 {
        0xEC15_0000_0000_0001
    } else {
        mixed
    }
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    hash = hash_u64(hash, bytes.len() as u64);
    for &byte in bytes {
        hash = hash_u64(hash, u64::from(byte));
    }
    hash
}

fn hash_u64(hash: u64, value: u64) -> u64 {
    splitmix64(hash ^ value.wrapping_mul(0x1000_0000_01B3))
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
