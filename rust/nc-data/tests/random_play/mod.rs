//! Random player changes of every kind, for the storage tests and the
//! stress writer (t204, t205). A change is plain data, so the same change
//! can run against two stores or be logged by a writer process.

use nc_data::{
    CampaignStore, FleetDetachSelection, Order, QueuedPlayerMail, ReportBlockRow, RuntimeChange,
    delete_report_block_for_viewer,
};
use nc_engine::build_seeded_initialized_game;

/// One player's change.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    Commission(u8),
    Detach(u8, usize),
    Transfer(u8, usize, usize),
    Join(u8, usize, usize),
    Move(u8, usize, [u8; 2]),
    Roe(u8, usize, u8),
    Speed(u8, usize, u8),
    Build(u8, usize, u32),
    Scorch(u8, usize, bool),
    Rename(u8, usize, usize),
    MoveStarbase(u8, usize, [u8; 2]),
    Tax(u8, u8),
    Mail(u8, u8),
    DeleteReport(u8),
    CancelMail(usize),
    Handle(u8, usize),
}

impl Step {
    pub fn empire(self) -> Option<u8> {
        match self {
            Step::CancelMail(_) => None,
            Step::Commission(e)
            | Step::Detach(e, _)
            | Step::Transfer(e, ..)
            | Step::Join(e, ..)
            | Step::Move(e, ..)
            | Step::Roe(e, ..)
            | Step::Speed(e, ..)
            | Step::Build(e, ..)
            | Step::Scorch(e, ..)
            | Step::Rename(e, ..)
            | Step::MoveStarbase(e, ..)
            | Step::Tax(e, _)
            | Step::Mail(e, _)
            | Step::DeleteReport(e)
            | Step::Handle(e, _) => Some(e),
        }
    }
}

/// A small xorshift, so a run's random play is repeatable.
pub struct Rng(pub u64);

impl Rng {
    pub fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }

    /// A random change by `empire`, or by any empire when `None`.
    pub fn step(&mut self, empire: Option<u8>, players: u8) -> Step {
        let e = empire.unwrap_or_else(|| self.below(usize::from(players)) as u8 + 1);
        let (a, b) = (self.below(1000), self.below(1000));
        let sector = [self.below(40) as u8 + 1, self.below(40) as u8 + 1];
        match self.below(16) {
            0 => Step::Commission(e),
            1 => Step::Detach(e, a),
            2 => Step::Transfer(e, a, b),
            3 => Step::Join(e, a, b),
            4 => Step::Move(e, a, sector),
            5 => Step::Roe(e, a, self.below(11) as u8),
            6 => Step::Speed(e, a, self.below(9) as u8 + 1),
            7 => Step::Build(e, a, (self.below(5) as u32 + 1) * 5),
            8 => Step::Scorch(e, a, self.below(2) == 0),
            9 => Step::Rename(e, a, b),
            10 => Step::MoveStarbase(e, a, sector),
            11 => Step::Tax(e, self.below(101) as u8),
            12 => Step::Mail(e, e % players + 1),
            13 => Step::DeleteReport(e),
            14 => Step::CancelMail(a),
            _ => Step::Handle(e, a),
        }
    }
}

pub fn mail(sender_empire_id: u8, recipient_empire_id: u8, subject: String) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id,
        recipient_empire_id,
        year: 0,
        subject,
        body: "hello".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }
}

fn own_fleets(state: &RuntimeChange, empire: u8) -> Vec<usize> {
    state
        .game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == empire)
        .map(|(idx, _)| idx + 1)
        .collect()
}

fn own_planets(state: &RuntimeChange, empire: u8) -> Vec<usize> {
    state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == empire)
        .map(|(idx, _)| idx + 1)
        .collect()
}

fn pick(records: &[usize], at: usize) -> Option<usize> {
    (!records.is_empty()).then(|| records[at % records.len()])
}

/// Apply `step`, as the game's own mutators apply a player's order. A step
/// the rules refuse fails, and so writes nothing.
pub fn apply(step: Step, state: &mut RuntimeChange) -> Result<(), Box<dyn std::error::Error>> {
    let year = state.game_data.conquest.game_year;
    match step {
        Step::Commission(e) => {
            state
                .game_data
                .auto_commission_all_stardock_units(usize::from(e))?;
        }
        Step::Detach(e, at) => {
            let donors: Vec<usize> = own_fleets(state, e)
                .into_iter()
                .filter(|&record| state.game_data.fleets[record - 1].ships.destroyers > 1)
                .collect();
            if let Some(donor) = pick(&donors, at) {
                state.game_data.detach_ships_to_new_fleet(
                    usize::from(e),
                    donor,
                    FleetDetachSelection {
                        destroyers: 1,
                        ..FleetDetachSelection::default()
                    },
                    None,
                    0,
                )?;
            }
        }
        Step::Transfer(e, a, b) => {
            let fleets = own_fleets(state, e);
            if let (Some(donor), Some(host)) = (pick(&fleets, a), pick(&fleets, b)) {
                state.game_data.transfer_ships_between_fleets(
                    usize::from(e),
                    donor,
                    host,
                    FleetDetachSelection {
                        destroyers: 1,
                        ..FleetDetachSelection::default()
                    },
                )?;
            }
        }
        Step::Join(e, a, b) => {
            let fleets = own_fleets(state, e);
            if let (Some(fleet), Some(host)) = (pick(&fleets, a), pick(&fleets, b)) {
                state
                    .game_data
                    .set_join_fleet_order(usize::from(e), fleet, host)?;
            }
        }
        Step::Move(e, at, target) => {
            if let Some(fleet) = pick(&own_fleets(state, e), at) {
                let speed = state.game_data.fleets[fleet - 1].max_speed.max(1);
                state.game_data.set_fleet_order(
                    fleet,
                    speed,
                    Order::MoveOnly.code(),
                    target,
                    None,
                    None,
                )?;
            }
        }
        Step::Roe(e, at, roe) => {
            if let Some(fleet) = pick(&own_fleets(state, e), at) {
                state
                    .game_data
                    .set_fleet_rules_of_engagement(usize::from(e), fleet, roe)?;
            }
        }
        Step::Speed(e, at, speed) => {
            if let Some(fleet) = pick(&own_fleets(state, e), at) {
                state
                    .game_data
                    .set_fleet_speed(usize::from(e), fleet, speed)?;
            }
        }
        Step::Build(e, at, points) => {
            if let Some(planet) = pick(&own_planets(state, e), at) {
                state
                    .game_data
                    .append_planet_build_order(planet, points, 1)?;
            }
        }
        Step::Scorch(e, at, on) => {
            if let Some(planet) = pick(&own_planets(state, e), at) {
                state.game_data.set_planet_scorch_earth(planet, on)?;
            }
        }
        Step::Rename(e, at, name) => {
            if let Some(planet) = pick(&own_planets(state, e), at) {
                state.game_data.rename_owned_planet(
                    usize::from(e),
                    planet,
                    &format!("World {name}"),
                )?;
            }
        }
        Step::MoveStarbase(e, at, target) => {
            let bases: Vec<usize> = state
                .game_data
                .bases
                .iter()
                .enumerate()
                .filter(|(_, base)| base.owner.0 == e)
                .map(|(idx, _)| idx + 1)
                .collect();
            if let Some(base) = pick(&bases, at) {
                state
                    .game_data
                    .set_starbase_destination(usize::from(e), base, target)?;
            }
        }
        Step::Tax(e, rate) => {
            state.game_data.set_player_tax_rate(usize::from(e), rate)?;
        }
        Step::Mail(sender, recipient) => {
            let mut message = mail(sender, recipient, format!("from {sender}"));
            message.year = year;
            state.queued_mail.push(message);
        }
        Step::DeleteReport(viewer) => {
            let players = state.game_data.players.clone();
            if let Some(idx) = state
                .report_block_rows
                .iter()
                .position(|row| row.is_visible_to_viewer(viewer) && !row.recipient_deleted)
            {
                delete_report_block_for_viewer(&mut state.report_block_rows, &players, idx, viewer);
            }
        }
        Step::CancelMail(at) => {
            let count = state.queued_mail.len();
            if count > 0 {
                state.queued_mail[at % count].cancel();
            }
        }
        Step::Handle(e, k) => nc_data::set_player_handle(
            &mut state.game_data.players[usize::from(e) - 1],
            &format!("h{e}-{k}"),
        ),
    }
    Ok(())
}

/// A campaign of `players` empires in year 3000, saved in `store`: each
/// home stardock holds destroyers and starbases, and there are reports for
/// every empire and some mail.
pub fn seed_campaign(store: &CampaignStore, players: u8) {
    let mut data = build_seeded_initialized_game(players, 3000, 1515).expect("seeded game");
    for empire in 1..=players {
        if let Some(home) = data
            .planets
            .iter()
            .position(|planet| planet.owner.0 == empire)
        {
            data.planets[home].stardock.destroyers = 30;
            data.planets[home].stardock.starbases = 2;
        }
    }
    let reports: Vec<ReportBlockRow> = (0..=players)
        .map(|viewer| ReportBlockRow {
            viewer_empire_id: viewer,
            block_index: usize::from(viewer),
            decoded_text: format!("report for {viewer}"),
            raw_bytes: None,
            recipient_deleted: false,
        })
        .collect();
    let mut first = mail(1, 2, "first".into());
    first.year = 3000;
    store
        .save_year(3000, &data, &reports, &[first])
        .expect("seed store");
}
