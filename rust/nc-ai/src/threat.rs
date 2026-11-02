//! What an empire knows about the danger to one of its worlds (t247): a
//! fixed row of numbers from its own memory and records, for learning a
//! threat score to replace the distance-and-recency rule
//! ([`crate::threatened`]). Nothing here reads another empire's records.

use nc_data::{CoreGameData, DiplomaticRelation};

use crate::memory::{Contact, SeatMemory};

/// Search radii (sectors) and report ages (years) of the strength features.
const RADII: [f32; 3] = [2.0, 4.0, 8.0];
const AGES: [u16; 2] = [1, 2];
/// Distance reported when no foreign world is known.
const FAR: f32 = 30.0;

pub const THREAT_FEATURE_NAMES: [&str; 18] = [
    "batteries",
    "armies",
    "potential",
    "production",
    "starbase",
    "converting",
    "enemy_world_distance",
    "at_war",
    "strength_r2_y1",
    "strength_r2_y2",
    "strength_r4_y1",
    "strength_r4_y2",
    "strength_r8_y1",
    "strength_r8_y2",
    "declared_strength_r4_y2",
    "largest_force_r8_y2",
    "contacts_r4_y2",
    "rule",
];
pub const THREAT_FEATURES: usize = THREAT_FEATURE_NAMES.len();

/// A contact's attack strength: its ships when a report counted them,
/// otherwise its size classes counted at their strongest hull (small as a
/// destroyer, medium a cruiser, large a battleship), and its starbases.
fn strength(contact: &Contact) -> f32 {
    let ships = match contact.ships {
        Some(ships) => {
            12.0 * ships.destroyers as f32
                + 33.0 * ships.cruisers as f32
                + 90.0 * ships.battleships as f32
        }
        None => {
            12.0 * contact.small as f32 + 33.0 * contact.medium as f32 + 90.0 * contact.large as f32
        }
    };
    ships + 100.0 * contact.starbases as f32
}

fn distance(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = f32::from(a[0]) - f32::from(b[0]);
    let dy = f32::from(a[1]) - f32::from(b[1]);
    (dx * dx + dy * dy).sqrt()
}

/// The features of world `planet_idx` (0-based) for its owner, `memory`'s
/// empire, in the order of [`THREAT_FEATURE_NAMES`].
pub fn threat_features(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    planet_idx: usize,
) -> [f32; THREAT_FEATURES] {
    let empire = memory.empire;
    let year = game_data.conquest.game_year;
    let planet = &game_data.planets[planet_idx];
    let sector = planet.sector.to_array();
    let declared = &game_data.players[usize::from(empire) - 1].diplomacy;
    let enemy =
        |other: u8| declared.get(usize::from(other) - 1) == Some(&DiplomaticRelation::Enemy);
    let foreign: Vec<(&Contact, f32, u16)> = memory
        .contacts
        .iter()
        .filter(|c| c.empire != 0 && c.empire != empire)
        .map(|c| (c, distance(c.coords, sector), year.saturating_sub(c.year)))
        .collect();
    let sum = |radius: f32, age: u16| -> f32 {
        foreign
            .iter()
            .filter(|(_, d, a)| *d <= radius && *a < age)
            .map(|(c, _, _)| strength(c))
            .sum()
    };
    let enemy_world_distance = memory
        .planet_intel
        .iter()
        .filter(|(_, intel)| {
            intel
                .known_owner_empire_id
                .is_some_and(|o| o != 0 && o != empire)
        })
        .filter_map(|(record, _)| game_data.planets.get(record - 1))
        .map(|p| distance(p.sector.to_array(), sector))
        .fold(FAR, f32::min);
    let mut row = [0.0; THREAT_FEATURES];
    row[0] = f32::from(planet.ground_batteries);
    row[1] = f32::from(planet.armies);
    row[2] = f32::from(planet.potential_production);
    row[3] = planet.production.to_f64() as f32;
    row[4] = f32::from(u8::from(
        game_data
            .bases
            .iter()
            .any(|b| b.owner.0 == empire && b.sector.to_array() == sector),
    ));
    row[5] = f32::from(u8::from(planet.years_under_owner < 2));
    row[6] = enemy_world_distance;
    row[7] = f32::from(u8::from(
        (1..=game_data.conquest.player_count).any(|other| other != empire && enemy(other)),
    ));
    for (r, &radius) in RADII.iter().enumerate() {
        for (a, &age) in AGES.iter().enumerate() {
            row[8 + 2 * r + a] = sum(radius, age);
        }
    }
    row[14] = foreign
        .iter()
        .filter(|(c, d, a)| enemy(c.empire) && *d <= 4.0 && *a < 2)
        .map(|(c, _, _)| strength(c))
        .sum();
    row[15] = foreign
        .iter()
        .filter(|(_, d, a)| *d <= 8.0 && *a < 2)
        .map(|(c, _, _)| strength(c))
        .fold(0.0, f32::max);
    row[16] = foreign
        .iter()
        .filter(|(_, d, a)| *d <= 4.0 && *a < 2)
        .count() as f32;
    row[17] = f32::from(u8::from(crate::executor::rule_threatened(
        game_data, memory, sector,
    )));
    row
}

/// The fitted score (t247, `tools/ai/threat_fit.py` on 1,024 land-grab
/// executor games, attacked within a year): a logistic regression over every
/// feature but `rule`, each `log1p`'d where marked, then standardized.
/// Features, in order: batteries, armies, potential, production, starbase, converting, enemy_world_distance, at_war, strength_r2_y1, strength_r2_y2, strength_r4_y1, strength_r4_y2, strength_r8_y1, strength_r8_y2, declared_strength_r4_y2, largest_force_r8_y2, contacts_r4_y2.
const LOGGED: [bool; THREAT_FEATURES - 1] = [
    true, true, true, true, false, false, true, false, true, true, true, true, true, true, true,
    true, true,
];
const MEAN: [f32; THREAT_FEATURES - 1] = [
    1.40821, 3.29971, 4.446, 3.73866, 0.222053, 0.0365277, 1.45583, 0.853421, 1.39623, 2.11552,
    2.69003, 3.76624, 4.78787, 6.08387, 3.63584, 5.21881, 0.972798,
];
const SCALE: [f32; THREAT_FEATURES - 1] = [
    1.02778, 1.03389, 0.248461, 0.971485, 0.415626, 0.187599, 0.402625, 0.353686, 2.63917, 3.11707,
    3.33722, 3.64984, 3.49927, 3.42373, 3.70651, 3.04301, 0.918894,
];
const WEIGHT: [f32; THREAT_FEATURES - 1] = [
    0.0716086, -0.648791, 0.174802, -0.406636, 0.112178, -0.072041, -0.162142, 1.00242, 0.0778522,
    0.0646249, 0.0928596, 0.390371, -0.0371901, 0.283441, -0.16111, -0.199837, -0.24523,
];
const BIAS: f32 = -3.0468;
/// The logit at which [`crate::Doctrine::threat_score`] calls a world
/// threatened: on held-out maps, half the attacks a year ahead, flagging 12%
/// of world-years at 33% precision.
pub const THREAT_CUTOFF: f32 = -2.0229;

/// The log-odds that world `planet_idx` is attacked within a year, from its
/// owner's [`threat_features`].
pub fn threat_score(game_data: &CoreGameData, memory: &SeatMemory, planet_idx: usize) -> f32 {
    let row = threat_features(game_data, memory, planet_idx);
    (0..THREAT_FEATURES - 1).fold(BIAS, |sum, i| {
        let value = if LOGGED[i] {
            row[i].max(0.0).ln_1p()
        } else {
            row[i]
        };
        sum + WEIGHT[i] * (value - MEAN[i]) / SCALE[i]
    })
}
