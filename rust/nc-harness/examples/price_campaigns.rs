//! Price campaigns (t090): whole campaigns on generated maps, every empire at
//! war from the first year and run by one scripted policy, to see what the
//! unit prices and the opening change once the economy and combat decisions
//! are in the engine.
//!
//! The policy, the same for every empire, with its opening and hull doctrine
//! as the two choices:
//! - Opening (year 0): `bare` buys two extra ETACs; `escort` buys two
//!   ETAC-plus-cruiser pairs as the goods allow; `base` buys a starbase;
//!   `pressure` buys a battleship.
//! - Expansion: every ETAC fleet colonizes the nearest unclaimed raw world,
//!   and ETACs are replaced while raw worlds remain. Under `escort` a bare
//!   ETAC joins a lone cruiser at home before it leaves.
//! - Builds, each world within its budget: the garrison floor (home 10
//!   armies and 4 batteries, colonies 2 armies, captured worlds 3), the
//!   transports and armies the chosen assault needs (and from year 10 those
//!   the nearest enemy homeworld would need), a starbase at home once two
//!   colonies exist or from year 6, then hulls. The `value` doctrine buys
//!   the affordable hull with the most attack plus defense per point; the
//!   `fixed` doctrine buys BB, CA, CA, DD, DD in turn and waits for the next
//!   one; the `cruisers` doctrine never buys a battleship. Colonies follow
//!   the empire's doctrine, except that `fixed` colonies use `value`. Tax is
//!   50% throughout.
//! - War: the two starting lone destroyers guard home. Everything else built
//!   at home joins one home fleet on Guard/Blockade. The victim is the
//!   enemy with the least navy, batteries, and bases, weighted by distance;
//!   the goal is the victim's world worth most (present production over
//!   the armies, strength, and distance it needs). A strike force sized to
//!   150% of the goal's defense, never more than 60% of the home fleet,
//!   with twice the defending armies plus one aboard, detaches and invades
//!   at ROE 8. Fleets away from home return; a fleet at an enemy world with
//!   troops aboard invades again. The policy sees the whole map.
//!
//! Each empire's turn goes through the real submission path in three
//! submissions a year (commissions and builds; joins, loads, and the strike
//! detach; fleet orders), because the submission path checks fleet orders
//! against the turn-start state, so a fleet commissioned this year cannot be
//! ordered in the same submission. Then one maintenance turn runs. A game
//! ends at the year limit or when the engine declares a winner.
//!
//! The prices are the engine's compiled constants; an alternative price is a
//! separate build. Every row repeats the prices the binary was built with.
//!
//! Run in release and keep the output in /tmp:
//!
//! ```text
//! cargo run --release -p nc-harness --example price_campaigns -- \
//!     --label base --players 4 --seeds 0..60 --years 80 \
//!     --opening escort --doctrine value --goods 50 > /tmp/base.tsv
//! ```
//!
//! `--opening mixed` gives the four openings to the empires of each game,
//! rotated by seed; `--doctrine mixed` seats `value` and `cruisers` empires
//! alternately. The `destroyers` doctrine buys only destroyers, and
//! `--doctrine-mix destroyers,value,value,value` seats the listed doctrines
//! in turn, rotated by seed (t146). `tools/price-campaigns/summarize.py`
//! makes the tables.

use std::thread;

use nc_data::maintenance_types::FleetBattlePerspective;
use nc_data::{
    CoreGameData, DiplomaticRelation, Fleet, FleetDetachSelection, FleetTurnAction, FleetTurnBlock,
    MaintenanceEvents, Order, PlanetTurnAction, PlanetTurnBlock, PlayerMode, ProductionItemKind,
    STARDOCK_SLOT_COUNT, ShipCounts, ShipLosses, TurnSubmission, has_combat_ships,
    stardock_slot_kind,
};
use nc_harness::{
    DiplomacySpec, HouseSpec, PlanetSpec, ScenarioBaseline, ScenarioMetadata, ScenarioPlay,
    ScenarioSpec, build_scenario,
};

const START_YEAR: u16 = 3000;
const TAX_RATE: u8 = 50;
const HOME_ARMIES: u16 = 10;
const HOME_BATTERIES: u16 = 4;
const COLONY_ARMIES: u16 = 2;
const CAPTURED_ARMIES: u16 = 3;
/// Armies kept at home when a strike fleet loads.
const HOME_ARMY_RESERVE: u16 = 4;
const GUARD_ROE: u8 = 6;
/// A strike force is sized to this share of the defense it expects at the
/// target, and leaves only when the home fleet can spare that much.
const STRIKE_SIZE_PERCENT: u32 = 150;
/// The most of the home fleet's strength a strike may take.
const STRIKE_SHARE_PERCENT: u32 = 60;
/// From this year the troops for the nearest enemy homeworld are kept ready.
const HOMEWORLD_TROOPS_FROM_YEAR: u16 = 10;
const STRIKE_ROE: u8 = 8;
/// Milestone years for the colony counts.
const MILESTONES: [u16; 3] = [5, 10, 20];

// ── Configuration ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opening {
    /// Two extra bare ETACs with the opening goods.
    Bare,
    /// Two extra cruiser-escorted ETACs, the second pair when affordable.
    Escort,
    /// A starbase at home first.
    Base,
    /// A battleship first.
    Pressure,
}

impl Opening {
    const ALL: [Opening; 4] = [
        Opening::Bare,
        Opening::Escort,
        Opening::Base,
        Opening::Pressure,
    ];

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "bare" => Opening::Bare,
            "escort" => Opening::Escort,
            "base" => Opening::Base,
            "pressure" => Opening::Pressure,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Opening::Bare => "bare",
            Opening::Escort => "escort",
            Opening::Base => "base",
            Opening::Pressure => "pressure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Doctrine {
    /// Buys the affordable hull with the most attack plus defense per point.
    Value,
    /// Buys BB, CA, CA, DD, DD in turn and waits for the next one.
    Fixed,
    /// Buys cruisers, or destroyers when a cruiser is not affordable.
    Cruisers,
    /// Buys only destroyers (t146).
    Destroyers,
}

impl Doctrine {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "value" => Doctrine::Value,
            "fixed" => Doctrine::Fixed,
            "cruisers" => Doctrine::Cruisers,
            "destroyers" => Doctrine::Destroyers,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Doctrine::Value => "value",
            Doctrine::Fixed => "fixed",
            Doctrine::Cruisers => "cruisers",
            Doctrine::Destroyers => "destroyers",
        }
    }
}

const FIXED_CYCLE: [ProductionItemKind; 5] = [
    ProductionItemKind::Battleship,
    ProductionItemKind::Cruiser,
    ProductionItemKind::Cruiser,
    ProductionItemKind::Destroyer,
    ProductionItemKind::Destroyer,
];

#[derive(Debug, Clone)]
struct Config {
    label: String,
    players: u8,
    seeds: std::ops::Range<u64>,
    years: u16,
    /// `None` is the mixed opening: the four openings rotated by seed.
    opening: Option<Opening>,
    /// `None` is the mixed doctrine: `value` and `cruisers` seated in turn.
    doctrine: Option<Doctrine>,
    /// When set, seats take these doctrines in turn, rotated by seed (t146).
    doctrine_mix: Vec<Doctrine>,
    goods: u32,
    threads: usize,
}

fn parse_args() -> Config {
    let mut config = Config {
        label: "base".to_string(),
        players: 4,
        seeds: 0..20,
        years: 40,
        opening: Some(Opening::Escort),
        doctrine: Some(Doctrine::Value),
        doctrine_mix: Vec::new(),
        goods: 50,
        threads: thread::available_parallelism().map_or(1, |n| n.get()),
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut idx = 0;
    while idx + 1 < args.len() {
        let (flag, value) = (args[idx].as_str(), args[idx + 1].as_str());
        match flag {
            "--label" => config.label = value.to_string(),
            "--players" => config.players = value.parse().expect("--players"),
            "--seeds" => {
                let (lo, hi) = value.split_once("..").expect("--seeds lo..hi");
                config.seeds = lo.parse().expect("seed")..hi.parse().expect("seed");
            }
            "--years" => config.years = value.parse().expect("--years"),
            "--opening" => {
                config.opening = if value == "mixed" {
                    None
                } else {
                    Some(Opening::parse(value).expect("--opening bare|escort|base|pressure|mixed"))
                }
            }
            "--doctrine" => {
                config.doctrine = if value == "mixed" {
                    None
                } else {
                    Some(
                        Doctrine::parse(value)
                            .expect("--doctrine value|fixed|cruisers|destroyers|mixed"),
                    )
                }
            }
            "--doctrine-mix" => {
                config.doctrine_mix = value
                    .split(',')
                    .map(|name| Doctrine::parse(name).expect("--doctrine-mix doctrine,..."))
                    .collect()
            }
            "--goods" => config.goods = value.parse().expect("--goods"),
            "--threads" => config.threads = value.parse().expect("--threads"),
            other => panic!("unknown flag {other}"),
        }
        idx += 2;
    }
    config
}

fn doctrine_label(config: &Config) -> String {
    if config.doctrine_mix.is_empty() {
        config.doctrine.map_or("mixed", Doctrine::name).to_string()
    } else {
        let names: Vec<_> = config.doctrine_mix.iter().map(|d| d.name()).collect();
        format!("mix:{}", names.join(","))
    }
}

fn opening_for(config: &Config, seed: u64, empire: u8) -> Opening {
    config.opening.unwrap_or_else(|| {
        Opening::ALL[(seed as usize + usize::from(empire) - 1) % Opening::ALL.len()]
    })
}

fn doctrine_for(config: &Config, seed: u64, empire: u8) -> Doctrine {
    if !config.doctrine_mix.is_empty() {
        let mix = &config.doctrine_mix;
        return mix[(seed as usize + usize::from(empire) - 1) % mix.len()];
    }
    config.doctrine.unwrap_or_else(|| {
        if (seed as usize + usize::from(empire)) % 2 == 0 {
            Doctrine::Value
        } else {
            Doctrine::Cruisers
        }
    })
}

fn cost(kind: ProductionItemKind) -> u32 {
    kind.build_cost().expect("priced unit")
}

fn prices() -> String {
    [
        ProductionItemKind::Destroyer,
        ProductionItemKind::Cruiser,
        ProductionItemKind::Battleship,
        ProductionItemKind::Scout,
        ProductionItemKind::Transport,
        ProductionItemKind::Etac,
        ProductionItemKind::GroundBattery,
        ProductionItemKind::Army,
        ProductionItemKind::Starbase,
    ]
    .iter()
    .map(|kind| cost(*kind).to_string())
    .collect::<Vec<_>>()
    .join("/")
}

// ── One submission phase through the real path ──────────────────────────

/// Actions accepted one at a time: each is tried with everything accepted so
/// far on a copy of the phase-start state, as the finished file would apply.
struct Phase {
    start: CoreGameData,
    working: CoreGameData,
    submission: TurnSubmission,
}

impl Phase {
    fn new(state: &CoreGameData, empire: u8) -> Self {
        Self {
            start: state.clone(),
            working: state.clone(),
            submission: TurnSubmission {
                player_record_index_1_based: usize::from(empire),
                year: state.conquest.game_year,
                tax_rate: None,
                diplomacy: Vec::new(),
                planets: Vec::new(),
                fleets: Vec::new(),
                messages: Vec::new(),
            },
        }
    }

    fn planet(&mut self, record: usize, action: PlanetTurnAction) -> bool {
        let mut trial = self.submission.clone();
        trial.planets.push(PlanetTurnBlock {
            planet_record_index_1_based: record,
            actions: vec![action],
        });
        self.try_submission(trial)
    }

    fn fleet(&mut self, record: usize, action: FleetTurnAction) -> bool {
        let mut trial = self.submission.clone();
        trial.fleets.push(FleetTurnBlock {
            fleet_record_index_1_based: record,
            actions: vec![action],
        });
        self.try_submission(trial)
    }

    fn try_submission(&mut self, trial: TurnSubmission) -> bool {
        let mut state = self.start.clone();
        if trial.apply_to(&mut state, &mut Vec::new()).is_ok() {
            self.submission = trial;
            self.working = state;
            true
        } else {
            false
        }
    }

    fn commit(self, play: &mut ScenarioPlay) {
        if self.submission.planets.is_empty()
            && self.submission.fleets.is_empty()
            && self.submission.tax_rate.is_none()
        {
            return;
        }
        play.submit_turn(&self.submission)
            .expect("a submission accepted on the copy is accepted by the game");
    }
}

// ── Views ───────────────────────────────────────────────────────────────

fn distance(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = f32::from(a[0]) - f32::from(b[0]);
    let dy = f32::from(a[1]) - f32::from(b[1]);
    (dx * dx + dy * dy).sqrt()
}

fn attack_strength(ships: &ShipCounts) -> u32 {
    u32::from(ships.battleships) * 90
        + u32::from(ships.cruisers) * 30
        + u32::from(ships.destroyers) * 10
}

fn planet_at(state: &CoreGameData, coords: [u8; 2]) -> Option<usize> {
    state
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == coords)
}

/// 0-based planet indexes the empire owns.
fn owned_planets(state: &CoreGameData, empire: u8) -> Vec<usize> {
    (0..state.planets.len())
        .filter(|&idx| state.planets[idx].owner.0 == empire)
        .collect()
}

/// 0-based fleet indexes of the empire's fleets that carry ships.
fn owned_fleets(state: &CoreGameData, empire: u8) -> Vec<usize> {
    (0..state.fleets.len())
        .filter(|&idx| {
            let fleet = &state.fleets[idx];
            fleet.owner.0 == empire && nc_data::has_any_force(fleet)
        })
        .collect()
}

fn unowned_planets(state: &CoreGameData) -> Vec<usize> {
    (0..state.planets.len())
        .filter(|&idx| {
            let planet = &state.planets[idx];
            planet.owner.0 == 0 && planet.sector.to_array() != [0, 0]
        })
        .collect()
}

/// Worlds of other empires, civil-disorder worlds included.
fn enemy_planets(state: &CoreGameData, empire: u8) -> Vec<usize> {
    (0..state.planets.len())
        .filter(|&idx| {
            let owner = state.planets[idx].owner.0;
            owner != 0 && owner != empire
        })
        .collect()
}

/// Attack strength of other empires' fleets and bases in the sector.
fn hostile_strength_at(state: &CoreGameData, empire: u8, coords: [u8; 2]) -> u32 {
    let fleets: u32 = state
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 != empire && fleet.sector.to_array() == coords)
        .map(|fleet| attack_strength(&fleet.ships))
        .sum();
    let bases = state
        .bases
        .iter()
        .filter(|base| {
            base.owner.0 != empire && base.base_id.0 != 0 && base.sector.to_array() == coords
        })
        .count() as u32;
    fleets + bases * 100
}

fn has_base_at(state: &CoreGameData, empire: u8, coords: [u8; 2]) -> bool {
    state.bases.iter().any(|base| {
        base.owner.0 == empire && base.base_id.0 != 0 && base.sector.to_array() == coords
    })
}

fn queued_units(state: &CoreGameData, planet_idx: usize, kind: ProductionItemKind) -> u32 {
    state.planets[planet_idx]
        .build_queue
        .iter()
        .filter(|slot| slot.kind == kind)
        .map(|slot| slot.points_owed.div_ceil(cost(kind)))
        .sum()
}

fn docked_units(state: &CoreGameData, planet_idx: usize, kind: ProductionItemKind) -> u32 {
    nc_data::stardock_slot_for_kind(kind.code())
        .map_or(0, |slot| state.planets[planet_idx].stardock[slot])
}

fn transports(fleet: &Fleet) -> u16 {
    fleet.ships.empty_transports + fleet.ships.loaded_transports
}

// ── The policy ──────────────────────────────────────────────────────────

/// What an assault on the chosen target needs.
#[derive(Debug, Clone, Copy)]
struct Goal {
    coords: [u8; 2],
    armies_needed: u16,
    strength_needed: u32,
}

#[derive(Debug, Default, Clone)]
struct EmpireStats {
    built: [u32; 10],
    built_home: [u32; 10],
    first_bb_order: Option<u16>,
    first_base: Option<u16>,
    first_colony: Option<u16>,
    colonies_by: [u16; 3],
    owned_by: [u16; 3],
    /// Years the homeworld could not place any order: no unit affordable.
    exhausted_years: u16,
    /// Years the fixed doctrine's next hull was unaffordable at home.
    wait_years: u16,
    etacs_bought: u32,
    losses: ShipLosses,
    eliminated: Option<u16>,
    home_lost: Option<u16>,
    assaults: u32,
    captures: u32,
}

struct Brain {
    empire: u8,
    opening: Opening,
    doctrine: Doctrine,
    home: [u8; 2],
    home_record: usize,
    escort_pairs: u8,
    fixed_pos: usize,
    /// Fleet numbers of the two starting lone destroyers, kept as pickets.
    pickets: Vec<u16>,
    stats: EmpireStats,
}

impl Brain {
    fn new(state: &CoreGameData, empire: u8, opening: Opening, doctrine: Doctrine) -> Self {
        let home_record = usize::from(empire);
        let home = state.planets[home_record - 1].sector.to_array();
        let pickets = state
            .fleets
            .iter()
            .filter(|fleet| {
                fleet.owner.0 == empire
                    && fleet.ships.destroyers == 1
                    && fleet.ships.etacs == 0
                    && fleet.ships.cruisers == 0
            })
            .map(|fleet| fleet.fleet_number.0)
            .collect();
        Self {
            empire,
            opening,
            doctrine,
            home,
            home_record,
            escort_pairs: 0,
            fixed_pos: 0,
            pickets,
            stats: EmpireStats::default(),
        }
    }

    fn alive(&self, state: &CoreGameData) -> bool {
        let player = &state.players[usize::from(self.empire) - 1];
        player.mode == PlayerMode::Human
            && (!owned_planets(state, self.empire).is_empty()
                || !owned_fleets(state, self.empire).is_empty())
    }

    /// The enemy empire to wear down: the one with the least to defend
    /// itself, weighted by how far its homeworld is.
    fn victim(&self, state: &CoreGameData) -> Option<u8> {
        (1..=state.conquest.player_count)
            .filter(|&other| other != self.empire && !owned_planets(state, other).is_empty())
            .map(|other| {
                let navy: u32 = state
                    .fleets
                    .iter()
                    .filter(|fleet| fleet.owner.0 == other)
                    .map(|fleet| attack_strength(&fleet.ships))
                    .sum();
                let ground: u32 = owned_planets(state, other)
                    .into_iter()
                    .map(|idx| u32::from(state.planets[idx].ground_batteries) * 90)
                    .sum();
                let bases = state
                    .bases
                    .iter()
                    .filter(|base| base.owner.0 == other && base.base_id.0 != 0)
                    .count() as u32
                    * 100;
                let reach = distance(
                    self.home,
                    state.planets[usize::from(other) - 1].sector.to_array(),
                );
                (navy + ground + bases + (reach * 100.0) as u32, other)
            })
            .min()
            .map(|(_, other)| other)
    }

    /// The victim's world worth most for what taking it needs: present
    /// production over the armies, the strength, and the distance.
    fn goal(&self, state: &CoreGameData) -> Option<Goal> {
        let victim = self.victim(state)?;
        enemy_planets(state, self.empire)
            .into_iter()
            .filter(|&idx| state.planets[idx].owner.0 == victim)
            .map(|idx| {
                let planet = &state.planets[idx];
                let coords = planet.sector.to_array();
                let armies_needed = (planet.armies * 2 + 1).max(3);
                let batteries = u32::from(planet.ground_batteries) * 90;
                let strength_needed =
                    batteries + hostile_strength_at(state, self.empire, coords) + 1;
                let worth = planet.production.to_f64().max(1.0)
                    / (1.0
                        + f64::from(armies_needed)
                        + f64::from(strength_needed) / 50.0
                        + f64::from(distance(self.home, coords)));
                (
                    worth,
                    Goal {
                        coords,
                        armies_needed,
                        strength_needed,
                    },
                )
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).expect("finite"))
            .map(|(_, goal)| goal)
    }

    fn plan_year(&mut self, play: &mut ScenarioPlay, year: u16) {
        let goal = self.goal(&play.built.game_data);

        // Phase 1: commission everything docked, then buy.
        let mut phase = Phase::new(&play.built.game_data, self.empire);
        self.commission(&mut phase);
        self.build(&mut phase, year, goal);
        phase.commit(play);
        if self.stats.first_base.is_none()
            && has_base_at(&play.built.game_data, self.empire, self.home)
        {
            self.stats.first_base = Some(year);
        }

        // Phase 2: escorts, the home fleet, and loading for an assault.
        let mut phase = Phase::new(&play.built.game_data, self.empire);
        let reserved = self.escort_colonizers(&mut phase);
        let strike = self.gather_home_fleet(&mut phase, goal, &reserved);
        phase.commit(play);

        // Phase 3: orders.
        let mut phase = Phase::new(&play.built.game_data, self.empire);
        self.order_colonizers(&mut phase);
        self.order_warships(&mut phase, goal, strike);
        phase.commit(play);
    }

    fn commission(&mut self, phase: &mut Phase) {
        for planet_idx in owned_planets(&phase.working, self.empire) {
            for slot in 0..STARDOCK_SLOT_COUNT {
                let planet = &phase.working.planets[planet_idx];
                if planet.stardock[slot] == 0 {
                    continue;
                }
                if stardock_slot_kind(planet, slot) == ProductionItemKind::Unknown(0) {
                    continue;
                }
                phase.planet(
                    planet_idx + 1,
                    PlanetTurnAction::Commission { slot_0_based: slot },
                );
            }
        }
    }

    fn order(
        &mut self,
        phase: &mut Phase,
        planet_idx: usize,
        kind: ProductionItemKind,
        qty: u32,
        year: u16,
    ) -> bool {
        if qty == 0 {
            return false;
        }
        let accepted = phase.planet(
            planet_idx + 1,
            PlanetTurnAction::Build {
                points_remaining: qty * cost(kind),
                kind: kind.code(),
            },
        );
        if accepted {
            let code = usize::from(kind.code());
            self.stats.built[code] += qty;
            if planet_idx + 1 == self.home_record {
                self.stats.built_home[code] += qty;
            }
            if kind == ProductionItemKind::Battleship && self.stats.first_bb_order.is_none() {
                self.stats.first_bb_order = Some(year);
            }
            if kind == ProductionItemKind::Etac {
                self.stats.etacs_bought += qty;
            }
        }
        accepted
    }

    fn budget(phase: &Phase, planet_idx: usize) -> u32 {
        phase
            .working
            .planet_build_points_left(planet_idx + 1)
            .unwrap_or(0)
    }

    fn build(&mut self, phase: &mut Phase, year: u16, goal: Option<Goal>) {
        let home_idx = self.home_record - 1;
        let home_owned = phase.working.planets[home_idx].owner.0 == self.empire;
        if home_owned {
            self.build_home(phase, year, goal);
        }
        for planet_idx in owned_planets(&phase.working, self.empire) {
            if planet_idx == home_idx {
                continue;
            }
            self.build_colony(phase, planet_idx, year);
        }
    }

    fn build_home(&mut self, phase: &mut Phase, year: u16, goal: Option<Goal>) {
        let home_idx = self.home_record - 1;
        if Self::budget(phase, home_idx) < cost(ProductionItemKind::Army) {
            self.stats.exhausted_years += 1;
        }

        // The opening.
        if year == 0 {
            match self.opening {
                Opening::Bare => {
                    let qty =
                        (Self::budget(phase, home_idx) / cost(ProductionItemKind::Etac)).min(2);
                    self.order(phase, home_idx, ProductionItemKind::Etac, qty, year);
                }
                Opening::Escort => {}
                Opening::Base => {
                    self.order(phase, home_idx, ProductionItemKind::Starbase, 1, year);
                }
                Opening::Pressure => {
                    self.order(phase, home_idx, ProductionItemKind::Battleship, 1, year);
                }
            }
        }
        if self.opening == Opening::Escort && self.escort_pairs < 2 {
            let pair = cost(ProductionItemKind::Etac) + cost(ProductionItemKind::Cruiser);
            while self.escort_pairs < 2 && Self::budget(phase, home_idx) >= pair {
                self.order(phase, home_idx, ProductionItemKind::Etac, 1, year);
                self.order(phase, home_idx, ProductionItemKind::Cruiser, 1, year);
                self.escort_pairs += 1;
            }
        }

        // Defense floor.
        let home = &phase.working.planets[home_idx];
        let armies_short = HOME_ARMIES.saturating_sub(home.armies) as u32;
        let batteries_short = HOME_BATTERIES.saturating_sub(home.ground_batteries) as u32;
        self.order_up_to(
            phase,
            home_idx,
            ProductionItemKind::Army,
            armies_short,
            year,
        );
        self.order_up_to(
            phase,
            home_idx,
            ProductionItemKind::GroundBattery,
            batteries_short,
            year,
        );

        // Expansion: keep colonizers at work while raw worlds remain.
        let raw = unowned_planets(&phase.working).len() as u32;
        if raw > 0 {
            let colonizers = owned_fleets(&phase.working, self.empire)
                .into_iter()
                .filter(|&idx| phase.working.fleets[idx].ships.etacs > 0)
                .count() as u32
                + docked_units(&phase.working, home_idx, ProductionItemKind::Etac)
                + queued_units(&phase.working, home_idx, ProductionItemKind::Etac);
            let wanted = match self.opening {
                Opening::Bare | Opening::Escort => raw.min(4),
                Opening::Base | Opening::Pressure => raw.min(2),
            };
            let short = wanted.saturating_sub(colonizers);
            if short > 0 && year > 0 {
                self.order_up_to(
                    phase,
                    home_idx,
                    ProductionItemKind::Etac,
                    short.min(1),
                    year,
                );
            }
        }

        // The troops the chosen assault needs, and from year 10 those the
        // nearest enemy homeworld would need.
        let mut troops = goal.map_or(0, |goal| goal.armies_needed);
        if year >= HOMEWORLD_TROOPS_FROM_YEAR {
            let nearest_home = (0..phase.working.conquest.player_count as usize)
                .filter(|&idx| {
                    let owner = phase.working.planets[idx].owner.0;
                    owner != 0 && owner != self.empire
                })
                .min_by(|&a, &b| {
                    let da = distance(self.home, phase.working.planets[a].sector.to_array());
                    let db = distance(self.home, phase.working.planets[b].sector.to_array());
                    da.partial_cmp(&db).expect("finite")
                });
            if let Some(idx) = nearest_home {
                troops = troops.max((phase.working.planets[idx].armies * 2 + 1).max(3));
            }
        }
        if troops > 0 {
            let transports_have: u32 = owned_fleets(&phase.working, self.empire)
                .into_iter()
                .map(|idx| u32::from(transports(&phase.working.fleets[idx])))
                .sum::<u32>()
                + docked_units(&phase.working, home_idx, ProductionItemKind::Transport)
                + queued_units(&phase.working, home_idx, ProductionItemKind::Transport);
            let transports_short = u32::from(troops).saturating_sub(transports_have);
            self.order_up_to(
                phase,
                home_idx,
                ProductionItemKind::Transport,
                transports_short,
                year,
            );
            let armies_wanted = troops + HOME_ARMY_RESERVE;
            let home = &phase.working.planets[home_idx];
            let armies_short = u32::from(armies_wanted.saturating_sub(home.armies)).saturating_sub(
                queued_units(&phase.working, home_idx, ProductionItemKind::Army),
            );
            self.order_up_to(
                phase,
                home_idx,
                ProductionItemKind::Army,
                armies_short,
                year,
            );
        }

        // A base at home once the opening is over.
        let colonies = owned_planets(&phase.working, self.empire)
            .len()
            .saturating_sub(1);
        if !has_base_at(&phase.working, self.empire, self.home)
            && docked_units(&phase.working, home_idx, ProductionItemKind::Starbase) == 0
            && queued_units(&phase.working, home_idx, ProductionItemKind::Starbase) == 0
            && (colonies >= 2 || year >= 6)
        {
            self.order_up_to(phase, home_idx, ProductionItemKind::Starbase, 1, year);
        }

        // Hulls with what is left.
        self.build_hulls(phase, home_idx, year, true);
    }

    fn build_colony(&mut self, phase: &mut Phase, planet_idx: usize, year: u16) {
        let planet = &phase.working.planets[planet_idx];
        let floor = if planet.production.to_f64() > 3.5 && planet.years_under_owner < 2 {
            CAPTURED_ARMIES
        } else {
            COLONY_ARMIES
        };
        let short = u32::from(floor.saturating_sub(planet.armies)).saturating_sub(queued_units(
            &phase.working,
            planet_idx,
            ProductionItemKind::Army,
        ));
        self.order_up_to(phase, planet_idx, ProductionItemKind::Army, short, year);
        let coords = phase.working.planets[planet_idx].sector.to_array();
        if !has_base_at(&phase.working, self.empire, coords)
            && docked_units(&phase.working, planet_idx, ProductionItemKind::Starbase) == 0
            && queued_units(&phase.working, planet_idx, ProductionItemKind::Starbase) == 0
        {
            self.order_up_to(phase, planet_idx, ProductionItemKind::Starbase, 1, year);
        }
        self.build_hulls(phase, planet_idx, year, false);
    }

    /// Order up to `qty` units, as many as the budget admits.
    fn order_up_to(
        &mut self,
        phase: &mut Phase,
        planet_idx: usize,
        kind: ProductionItemKind,
        qty: u32,
        year: u16,
    ) {
        let affordable = Self::budget(phase, planet_idx) / cost(kind);
        self.order(phase, planet_idx, kind, qty.min(affordable), year);
    }

    fn build_hulls(&mut self, phase: &mut Phase, planet_idx: usize, year: u16, at_home: bool) {
        let mut counts = [0u32; 10];
        let mut budget = Self::budget(phase, planet_idx);
        let fixed = at_home && self.doctrine == Doctrine::Fixed;
        let no_battleships = self.doctrine == Doctrine::Cruisers;
        let only_destroyers = self.doctrine == Doctrine::Destroyers;
        loop {
            let kind = if fixed {
                let wanted = FIXED_CYCLE[self.fixed_pos % FIXED_CYCLE.len()];
                if cost(wanted) > budget {
                    if counts.iter().all(|&count| count == 0) {
                        self.stats.wait_years += 1;
                    }
                    break;
                }
                self.fixed_pos += 1;
                wanted
            } else if only_destroyers {
                if cost(ProductionItemKind::Destroyer) > budget {
                    break;
                }
                ProductionItemKind::Destroyer
            } else {
                let Some(kind) = best_value_hull(budget, no_battleships) else {
                    break;
                };
                kind
            };
            counts[usize::from(kind.code())] += 1;
            budget -= cost(kind);
        }
        for kind in [
            ProductionItemKind::Battleship,
            ProductionItemKind::Cruiser,
            ProductionItemKind::Destroyer,
        ] {
            let qty = counts[usize::from(kind.code())];
            self.order(phase, planet_idx, kind, qty, year);
        }
    }

    /// Send each bare ETAC fleet at home to join a lone cruiser fleet
    /// there; returns the cruiser fleets so reserved.
    fn escort_colonizers(&mut self, phase: &mut Phase) -> Vec<usize> {
        let mut reserved = Vec::new();
        if self.opening != Opening::Escort {
            return reserved;
        }
        let fleets = owned_fleets(&phase.working, self.empire);
        let bare: Vec<usize> = fleets
            .iter()
            .copied()
            .filter(|&idx| {
                let fleet = &phase.working.fleets[idx];
                fleet.sector.to_array() == self.home
                    && fleet.ships.etacs > 0
                    && !has_combat_ships(fleet)
                    && fleet.order != Order::JoinAnotherFleet
            })
            .collect();
        for colonizer in bare {
            let escort = fleets.iter().copied().find(|&idx| {
                let fleet = &phase.working.fleets[idx];
                idx != colonizer
                    && !reserved.contains(&idx)
                    && fleet.sector.to_array() == self.home
                    && fleet.ships.etacs == 0
                    && fleet.ships.cruisers > 0
                    && fleet.ships.battleships == 0
                    && transports(fleet) == 0
                    && !self.pickets.contains(&fleet.fleet_number.0)
            });
            if let Some(escort) = escort {
                if phase.fleet(
                    colonizer + 1,
                    FleetTurnAction::Join {
                        host_fleet_record_index_1_based: escort + 1,
                    },
                ) {
                    reserved.push(escort);
                }
            }
        }
        reserved
    }

    /// Fleets at home that are neither pickets, colonizers, nor reserved.
    fn home_pool(&self, state: &CoreGameData, reserved: &[usize]) -> Vec<usize> {
        owned_fleets(state, self.empire)
            .into_iter()
            .filter(|&idx| {
                let fleet = &state.fleets[idx];
                fleet.sector.to_array() == self.home
                    && fleet.depth.round() == 1
                    && fleet.ships.etacs == 0
                    && !reserved.contains(&idx)
                    && !self.pickets.contains(&fleet.fleet_number.0)
                    && matches!(
                        fleet.order,
                        Order::HoldPosition | Order::GuardBlockadeWorld | Order::JoinAnotherFleet
                    )
            })
            .collect()
    }

    /// Point the home pool at its strongest fleet with Join orders (the
    /// merge happens in maintenance). When that fleet can spare a strike
    /// force for the goal, load the troops and detach the force; returns
    /// the strike fleet.
    fn gather_home_fleet(
        &mut self,
        phase: &mut Phase,
        goal: Option<Goal>,
        reserved: &[usize],
    ) -> Option<usize> {
        let pool = self.home_pool(&phase.working, reserved);
        let host = pool.iter().copied().max_by_key(|&idx| {
            (
                attack_strength(&phase.working.fleets[idx].ships),
                std::cmp::Reverse(idx),
            )
        })?;
        for donor in pool.iter().copied().filter(|&idx| idx != host) {
            phase.fleet(
                donor + 1,
                FleetTurnAction::Join {
                    host_fleet_record_index_1_based: host + 1,
                },
            );
        }
        let goal = goal?;
        let fleet = &phase.working.fleets[host];
        let spare_strength = attack_strength(&fleet.ships) * STRIKE_SHARE_PERCENT / 100;
        if !has_combat_ships(fleet) || spare_strength < goal.strength_needed {
            return None;
        }
        let home_idx = self.home_record - 1;
        if phase.working.planets[home_idx].owner.0 != self.empire {
            return None;
        }
        let spare_armies = phase.working.planets[home_idx]
            .armies
            .saturating_sub(HOME_ARMY_RESERVE);
        let to_load = goal
            .armies_needed
            .saturating_sub(fleet.ships.loaded_transports)
            .min(fleet.ships.empty_transports)
            .min(spare_armies);
        if to_load > 0 {
            phase.fleet(
                host + 1,
                FleetTurnAction::LoadArmies {
                    planet_record_index_1_based: self.home_record,
                    qty: to_load,
                },
            );
        }
        let fleet = &phase.working.fleets[host];
        if fleet.ships.loaded_transports < goal.armies_needed {
            return None;
        }
        // Warships for the strike: the heaviest first, up to the sized
        // strength, within what the fleet can spare.
        let target = (goal.strength_needed * STRIKE_SIZE_PERCENT / 100).min(spare_strength);
        let mut selection = FleetDetachSelection {
            full_transports: goal.armies_needed,
            ..FleetDetachSelection::default()
        };
        let mut strength = 0;
        for (count, worth, slot) in [
            (fleet.ships.battleships, 90, 0usize),
            (fleet.ships.cruisers, 30, 1),
            (fleet.ships.destroyers, 10, 2),
        ] {
            let mut taken = 0;
            while taken < count && strength < target {
                taken += 1;
                strength += worth;
            }
            match slot {
                0 => selection.battleships = taken,
                1 => selection.cruisers = taken,
                _ => selection.destroyers = taken,
            }
        }
        if strength == 0 || u32::from(nc_data::starship_count(fleet)) <= selection.total_ships() {
            return None;
        }
        let before = phase.working.fleets.len();
        let detached = phase.fleet(
            host + 1,
            FleetTurnAction::Detach {
                selection,
                donor_speed: None,
                new_fleet_roe: STRIKE_ROE,
            },
        );
        (detached && phase.working.fleets.len() == before + 1).then_some(before)
    }

    fn order_colonizers(&mut self, phase: &mut Phase) {
        for idx in owned_fleets(&phase.working, self.empire) {
            let fleet = &phase.working.fleets[idx];
            if fleet.ships.etacs == 0 {
                continue;
            }
            let target_idx = planet_at(&phase.working, fleet.target.to_array());
            if fleet.order == Order::JoinAnotherFleet
                || (fleet.order == Order::ColonizeWorld
                    && target_idx.is_some_and(|target| phase.working.planets[target].owner.0 == 0))
            {
                continue;
            }
            let here = fleet.sector.to_array();
            let claimed: Vec<[u8; 2]> = owned_fleets(&phase.working, self.empire)
                .into_iter()
                .filter(|&other| {
                    other != idx && phase.working.fleets[other].order == Order::ColonizeWorld
                })
                .map(|other| phase.working.fleets[other].target.to_array())
                .collect();
            let target = unowned_planets(&phase.working)
                .into_iter()
                .map(|planet| phase.working.planets[planet].sector.to_array())
                .filter(|coords| !claimed.contains(coords))
                .min_by(|a, b| {
                    distance(here, *a)
                        .partial_cmp(&distance(here, *b))
                        .expect("finite")
                });
            let speed = fleet.max_speed;
            match target {
                Some(coords) => {
                    phase.fleet(idx + 1, order_action(Order::ColonizeWorld, speed, coords));
                }
                None if here != self.home => {
                    phase.fleet(idx + 1, order_action(Order::MoveOnly, speed, self.home));
                }
                None => {}
            }
        }
    }

    fn order_warships(&mut self, phase: &mut Phase, goal: Option<Goal>, strike: Option<usize>) {
        for idx in owned_fleets(&phase.working, self.empire) {
            let fleet = &phase.working.fleets[idx];
            if fleet.ships.etacs > 0 {
                continue;
            }
            let here = fleet.sector.to_array();
            let speed = fleet.max_speed;
            let armed = has_combat_ships(fleet);
            if self.pickets.contains(&fleet.fleet_number.0) {
                if fleet.order != Order::GuardBlockadeWorld || fleet.target.to_array() != self.home
                {
                    if here == self.home {
                        phase.fleet(
                            idx + 1,
                            FleetTurnAction::RulesOfEngagement { value: GUARD_ROE },
                        );
                        phase.fleet(
                            idx + 1,
                            order_action(Order::GuardBlockadeWorld, 0, self.home),
                        );
                    } else {
                        phase.fleet(idx + 1, order_action(Order::MoveOnly, speed, self.home));
                    }
                }
                continue;
            }
            if Some(idx) == strike {
                if let Some(goal) = goal {
                    if phase.fleet(
                        idx + 1,
                        order_action(Order::InvadeWorld, speed, goal.coords),
                    ) {
                        self.stats.assaults += 1;
                    }
                }
                continue;
            }
            if fleet.order == Order::JoinAnotherFleet {
                continue;
            }
            match fleet.order {
                Order::InvadeWorld | Order::BombardWorld | Order::BlitzWorld | Order::MoveOnly => {
                    let target = planet_at(&phase.working, fleet.target.to_array());
                    let still_enemy = target.is_some_and(|planet| {
                        let owner = phase.working.planets[planet].owner.0;
                        owner != 0 && owner != self.empire
                    });
                    if fleet.order == Order::MoveOnly || still_enemy {
                        continue;
                    }
                }
                _ => {}
            }
            if here == self.home {
                if armed
                    && (fleet.order != Order::GuardBlockadeWorld
                        || fleet.target.to_array() != self.home)
                {
                    phase.fleet(
                        idx + 1,
                        FleetTurnAction::RulesOfEngagement { value: GUARD_ROE },
                    );
                    phase.fleet(
                        idx + 1,
                        order_action(Order::GuardBlockadeWorld, 0, self.home),
                    );
                }
                continue;
            }
            let at_planet = planet_at(&phase.working, here);
            let at_enemy = at_planet.is_some_and(|planet| {
                let owner = phase.working.planets[planet].owner.0;
                owner != 0 && owner != self.empire
            });
            if at_enemy && armed && fleet.ships.loaded_transports > 0 {
                if phase.fleet(idx + 1, order_action(Order::InvadeWorld, speed, here)) {
                    self.stats.assaults += 1;
                }
                continue;
            }
            let home_owned = phase.working.planets[self.home_record - 1].owner.0 == self.empire;
            if home_owned {
                phase.fleet(idx + 1, order_action(Order::MoveOnly, speed, self.home));
            } else if armed
                && at_planet
                    .is_some_and(|planet| phase.working.planets[planet].owner.0 == self.empire)
            {
                phase.fleet(idx + 1, order_action(Order::GuardBlockadeWorld, 0, here));
            }
        }
    }
}

fn order_action(order: Order, speed: u8, target: [u8; 2]) -> FleetTurnAction {
    FleetTurnAction::Order {
        speed,
        order_code: order.code(),
        target: Some(target),
        aux0: None,
        aux1: None,
    }
}

/// The affordable hull with the most attack plus defense per point; the
/// larger hull wins a tie.
fn best_value_hull(budget: u32, no_battleships: bool) -> Option<ProductionItemKind> {
    [
        (ProductionItemKind::Battleship, 190.0),
        (ProductionItemKind::Cruiser, 60.0),
        (ProductionItemKind::Destroyer, 18.0),
    ]
    .into_iter()
    .filter(|(kind, _)| !(no_battleships && *kind == ProductionItemKind::Battleship))
    .filter(|(kind, _)| cost(*kind) <= budget)
    .map(|(kind, worth)| (kind, worth / cost(kind) as f64))
    .fold(
        None,
        |best: Option<(ProductionItemKind, f64)>, next| match best {
            Some((_, worth)) if worth >= next.1 => best,
            _ => Some(next),
        },
    )
    .map(|(kind, _)| kind)
}

// ── One game ────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct GameStats {
    years: u16,
    winner: Option<u8>,
    winner_year: Option<u16>,
    battles: u32,
    battle_reports: u32,
    assaults: u32,
    /// Assaults the engine resolved, and those against homeworlds.
    assaults_resolved: u32,
    home_assaults: u32,
    assault_failures: u32,
    captures: u32,
    bombards: u32,
    colonies: u32,
    eliminations: u32,
    first_battle: Option<u16>,
    first_capture: Option<u16>,
    losses: ShipLosses,
}

fn add_losses(total: &mut ShipLosses, losses: &ShipLosses) {
    total.destroyers += losses.destroyers;
    total.cruisers += losses.cruisers;
    total.battleships += losses.battleships;
    total.scouts += losses.scouts;
    total.transports += losses.transports;
    total.etacs += losses.etacs;
}

fn scenario(config: &Config, seed: u64) -> ScenarioSpec {
    let empires = 1..=config.players;
    ScenarioSpec {
        metadata: ScenarioMetadata {
            label: Some("price-campaigns".to_string()),
            player_count: config.players,
            year: START_YEAR,
            seed,
            baseline: ScenarioBaseline::BuilderCompatible,
        },
        houses: empires
            .clone()
            .map(|empire| HouseSpec {
                record_index_1_based: usize::from(empire),
                handle: None,
                empire_name: None,
                homeworld_name: None,
                tax_rate: Some(TAX_RATE),
                autopilot: false,
            })
            .collect(),
        diplomacy: empires
            .clone()
            .flat_map(|from| {
                empires
                    .clone()
                    .filter(move |&to| to != from)
                    .map(move |to| DiplomacySpec {
                        from_empire: from,
                        to_empire: to,
                        relation: DiplomaticRelation::Enemy,
                    })
            })
            .collect(),
        planets: empires
            .clone()
            .map(|empire| PlanetSpec {
                record_index_1_based: usize::from(empire),
                stored_production: Some(config.goods),
                ..PlanetSpec::default()
            })
            .collect(),
        fleets: Vec::new(),
        turn_files: Vec::new(),
        queued_mail: Vec::new(),
        results_blocks: Vec::new(),
        message_blocks: Vec::new(),
    }
}

fn record_events(
    game: &mut GameStats,
    brains: &mut [Brain],
    events: &MaintenanceEvents,
    year: u16,
) {
    for event in &events.fleet_battle_events {
        game.battle_reports += 1;
        if event.perspective == FleetBattlePerspective::Intercepted {
            game.battles += 1;
            game.first_battle.get_or_insert(year);
        }
        add_losses(&mut game.losses, &event.friendly_losses);
        if let Some(brain) = brains.get_mut(usize::from(event.reporting_empire) - 1) {
            add_losses(&mut brain.stats.losses, &event.friendly_losses);
        }
    }
    for event in &events.assault_report_events {
        game.assaults_resolved += 1;
        if event.planet_idx < brains.len() {
            game.home_assaults += 1;
        }
        if event.outcome != nc_data::MissionOutcome::Succeeded {
            game.assault_failures += 1;
        }
        add_losses(&mut game.losses, &event.attacker_ship_losses);
        if let Some(brain) = brains.get_mut(usize::from(event.attacker_empire) - 1) {
            add_losses(&mut brain.stats.losses, &event.attacker_ship_losses);
        }
    }
    for event in &events.bombard_events {
        game.bombards += 1;
        add_losses(&mut game.losses, &event.attacker_losses);
        if let Some(brain) = brains.get_mut(usize::from(event.attacker_empire) - 1) {
            add_losses(&mut brain.stats.losses, &event.attacker_losses);
        }
    }
    for event in &events.ownership_change_events {
        game.captures += 1;
        game.first_capture.get_or_insert(year);
        if let Some(brain) = brains.get_mut(usize::from(event.new_owner_empire) - 1) {
            brain.stats.captures += 1;
        }
    }
    for event in &events.colonization_events {
        if let nc_data::ColonizationResolvedEvent::Succeeded {
            colonizer_empire, ..
        } = event
        {
            game.colonies += 1;
            if let Some(brain) = brains.get_mut(usize::from(*colonizer_empire) - 1) {
                brain.stats.first_colony.get_or_insert(year);
                for (slot, milestone) in MILESTONES.iter().enumerate() {
                    if year <= *milestone {
                        brain.stats.colonies_by[slot] += 1;
                    }
                }
            }
        }
    }
    for event in &events.empire_elimination_events {
        game.eliminations += 1;
        if let Some(brain) = brains.get_mut(usize::from(event.defeated_empire) - 1) {
            brain.stats.eliminated.get_or_insert(year);
        }
    }
}

fn play_game(config: &Config, seed: u64) -> (GameStats, Vec<Brain>, CoreGameData) {
    let built = build_scenario(&scenario(config, seed)).expect("generated map builds");
    let mut play = ScenarioPlay::new(built);
    let mut brains: Vec<Brain> = (1..=config.players)
        .map(|empire| {
            Brain::new(
                &play.built.game_data,
                empire,
                opening_for(config, seed, empire),
                doctrine_for(config, seed, empire),
            )
        })
        .collect();
    let mut game = GameStats::default();

    for year in 0..config.years {
        for brain in &mut brains {
            if brain.alive(&play.built.game_data) {
                brain.plan_year(&mut play, year);
            }
        }
        let events = play.run_maintenance().expect("maintenance runs");
        let played = year + 1;
        game.years = played;
        record_events(&mut game, &mut brains, &events, played);
        for brain in &mut brains {
            if brain.stats.home_lost.is_none()
                && play.built.game_data.planets[brain.home_record - 1].owner.0 != brain.empire
            {
                brain.stats.home_lost = Some(played);
            }
        }
        for (slot, milestone) in MILESTONES.iter().enumerate() {
            if played == *milestone {
                for brain in &mut brains {
                    brain.stats.owned_by[slot] =
                        owned_planets(&play.built.game_data, brain.empire).len() as u16;
                }
            }
        }
        if let Some(winner) = play.winner_state.winner_empire {
            game.winner = Some(winner);
            game.winner_year = Some(played);
            break;
        }
    }
    game.assaults = brains.iter().map(|brain| brain.stats.assaults).sum();
    (game, brains, play.built.game_data)
}

// ── Output ──────────────────────────────────────────────────────────────

fn opt(value: Option<u16>) -> String {
    value.map_or("-".to_string(), |v| v.to_string())
}

fn losses_columns(losses: &ShipLosses) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        losses.destroyers,
        losses.cruisers,
        losses.battleships,
        losses.scouts,
        losses.transports,
        losses.etacs
    )
}

fn game_rows(
    config: &Config,
    seed: u64,
    game: &GameStats,
    brains: &[Brain],
    state_end: &CoreGameData,
) -> String {
    let mut out = String::new();
    let opening = config.opening.map_or("mixed", Opening::name);
    let key = format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        config.label,
        prices(),
        config.goods,
        config.players,
        opening,
        doctrine_label(config),
        seed
    );
    out.push_str(&format!(
        "G\t{key}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        game.years,
        game.winner.map_or("-".to_string(), |w| w.to_string()),
        opt(game.winner_year),
        game.eliminations,
        game.battles,
        game.battle_reports,
        game.assaults,
        game.assaults_resolved,
        game.home_assaults,
        game.assault_failures,
        game.captures,
        game.bombards,
        game.colonies,
        opt(game.first_battle),
        opt(game.first_capture),
        losses_columns(&game.losses),
    ));
    for brain in brains {
        let stats = &brain.stats;
        let owned_end = owned_planets(state_end, brain.empire).len();
        let alive = brain.alive(state_end) && owned_end > 0;
        out.push_str(&format!(
            "E\t{key}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            brain.empire,
            brain.opening.name(),
            brain.doctrine.name(),
            opt(stats.first_colony),
            stats.colonies_by[0],
            stats.colonies_by[1],
            stats.colonies_by[2],
            stats.owned_by[0],
            stats.owned_by[1],
            stats.owned_by[2],
            owned_end,
            stats.built[1],
            stats.built[2],
            stats.built[3],
            stats.built[4],
            stats.built[5],
            stats.built[6],
            stats.built[7],
            stats.built[8],
            stats.built[9],
            stats.built_home[3],
            opt(stats.first_bb_order),
            opt(stats.first_base),
            stats.exhausted_years,
            stats.wait_years,
            stats.etacs_bought,
            stats.assaults,
            stats.captures,
            opt(stats.eliminated),
            opt(stats.home_lost),
            u8::from(alive),
            u8::from(game.winner == Some(brain.empire)),
            losses_columns(&stats.losses),
        ));
    }
    out
}

fn main() {
    let config = parse_args();
    println!(
        "# price_campaigns label={} prices(dd/ca/bb/sc/tt/etac/gb/army/sb)={} goods={} players={} seeds={}..{} years={} opening={} doctrine={}",
        config.label,
        prices(),
        config.goods,
        config.players,
        config.seeds.start,
        config.seeds.end,
        config.years,
        config.opening.map_or("mixed", Opening::name),
        doctrine_label(&config)
    );
    println!(
        "# G\tlabel\tprices\tgoods\tplayers\topening\tdoctrine\tseed\tyears\twinner\twinner_year\teliminations\tbattles\tbattle_reports\tassaults\tassaults_resolved\thome_assaults\tassault_failures\tcaptures\tbombards\tcolonies\tfirst_battle\tfirst_capture\tlost_dd\tlost_ca\tlost_bb\tlost_sc\tlost_tt\tlost_etac"
    );
    println!(
        "# E\tlabel\tprices\tgoods\tplayers\topening\tdoctrine\tseed\tempire\tempire_opening\tempire_doctrine\tfirst_colony\tcol_by5\tcol_by10\tcol_by20\towned5\towned10\towned20\towned_end\tbuilt_dd\tbuilt_ca\tbuilt_bb\tbuilt_sc\tbuilt_tt\tbuilt_etac\tbuilt_gb\tbuilt_army\tbuilt_sb\tbuilt_bb_home\tfirst_bb_order\tfirst_base\texhausted_years\twait_years\tetacs_bought\tassaults\tcaptures\teliminated\thome_lost\talive\twinner\tlost_dd\tlost_ca\tlost_bb\tlost_sc\tlost_tt\tlost_etac"
    );
    let seeds: Vec<u64> = config.seeds.clone().collect();
    let threads = config.threads.max(1).min(seeds.len().max(1));
    let mut rows: Vec<(u64, String)> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let config = &config;
                let seeds = &seeds;
                scope.spawn(move || {
                    seeds
                        .iter()
                        .copied()
                        .skip(worker)
                        .step_by(threads)
                        .map(|seed| {
                            let (game, brains, end) = play_game(config, seed);
                            (seed, game_rows(config, seed, &game, &brains, &end))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker finishes"))
            .collect()
    });
    rows.sort_by_key(|(seed, _)| *seed);
    for (_, row) in rows {
        print!("{row}");
    }
}
