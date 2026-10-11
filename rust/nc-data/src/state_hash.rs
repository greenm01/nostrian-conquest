//! A stable hash of the typed game state, for deriving a campaign seed.
//!
//! Each model struct feeds its fields in declaration order through an
//! exhaustive destructure, so a field added to the model fails to compile
//! here until the hash takes it. Values go in as explicit integers: enums as
//! their classic codes, booleans as 0 or 1, text as its UTF-8 bytes, and a
//! `Real48` as its six bytes. Lists feed their length first.

use crate::model::{
    BuildSlot, Conquest, Coords, Fleet, Ipbm, MaintenanceDays, Planet, Player, Setup, ShipCounts,
    Starbase, Stardock,
};
use crate::rng::splitmix64;
use crate::{CoreGameData, Real48};

pub(crate) struct StateHash(u64);

impl StateHash {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn finish(self) -> u64 {
        self.0
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.0 = splitmix64(self.0 ^ value.wrapping_mul(0x1000_0000_01B3));
    }

    /// The length, then each byte.
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        self.u64(bytes.len() as u64);
        for &byte in bytes {
            self.u64(u64::from(byte));
        }
    }

    fn bool(&mut self, value: bool) {
        self.u64(u64::from(value));
    }

    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn real(&mut self, value: Real48) {
        self.bytes(&value.to_bytes());
    }

    fn coords(&mut self, Coords { x, y }: Coords) {
        self.u64(u64::from(x));
        self.u64(u64::from(y));
    }

    pub(crate) fn game_data(&mut self, game_data: &CoreGameData) {
        let CoreGameData {
            players,
            planets,
            fleets,
            bases,
            ipbms,
            setup,
            conquest,
        } = game_data;
        self.u64(players.len() as u64);
        for player in players {
            self.player(player);
        }
        self.u64(planets.len() as u64);
        for planet in planets {
            self.planet(planet);
        }
        self.u64(fleets.len() as u64);
        for fleet in fleets {
            self.fleet(fleet);
        }
        self.u64(bases.len() as u64);
        for base in bases {
            self.starbase(base);
        }
        self.u64(ipbms.len() as u64);
        for ipbm in ipbms {
            self.ipbm(ipbm);
        }
        self.setup(setup);
        self.conquest(conquest);
    }

    fn player(&mut self, player: &Player) {
        let Player {
            mode,
            handle,
            page_lines,
            empire_name,
            messages_first,
            messages_last,
            results_first,
            results_last,
            fleets_first,
            fleets_last,
            bases_first,
            bases_last,
            ipbms_first,
            ipbms_last,
            homeworld,
            last_year_on,
            planet_count,
            tax_rate,
            production,
            diplomacy,
            autopilot,
        } = player;
        self.u64(u64::from(mode.to_raw()));
        self.text(handle);
        self.u64(u64::from(*page_lines));
        self.text(empire_name);
        for value in [messages_first, messages_last, results_first, results_last] {
            self.u64(*value as u32 as u64);
        }
        for record in [
            fleets_first,
            fleets_last,
            bases_first,
            bases_last,
            ipbms_first,
            ipbms_last,
        ] {
            self.u64(u64::from(record.0));
        }
        self.u64(u64::from(*homeworld));
        self.u64(u64::from(*last_year_on));
        self.u64(u64::from(*planet_count));
        self.u64(u64::from(*tax_rate));
        self.u64(u64::from(*production));
        for relation in diplomacy {
            self.u64(u64::from(relation.to_raw()));
        }
        self.bool(*autopilot);
    }

    fn planet(&mut self, planet: &Planet) {
        let Planet {
            sector,
            potential_production,
            production,
            stored_goods,
            name,
            build_queue,
            stardock,
            armies,
            ground_batteries,
            years_under_owner,
            owner,
            scorch_earth,
        } = planet;
        self.coords(*sector);
        self.u64(u64::from(*potential_production));
        self.real(*production);
        self.real(*stored_goods);
        self.text(name);
        for BuildSlot { points_owed, kind } in build_queue {
            self.u64(u64::from(*points_owed));
            self.u64(u64::from(kind.to_raw()));
        }
        let Stardock {
            destroyers,
            cruisers,
            battleships,
            scouts,
            transports,
            etacs,
            starbases,
            ipbms,
        } = stardock;
        for count in [
            destroyers,
            cruisers,
            battleships,
            scouts,
            transports,
            etacs,
            starbases,
            ipbms,
        ] {
            self.u64(u64::from(*count));
        }
        self.u64(u64::from(*armies));
        self.u64(u64::from(*ground_batteries));
        self.u64(u64::from(*years_under_owner));
        self.u64(u64::from(owner.0));
        self.bool(*scorch_earth);
    }

    fn fleet(&mut self, fleet: &Fleet) {
        let Fleet {
            fleet_number,
            owner,
            next,
            record,
            prev,
            max_speed,
            speed,
            sector,
            x_fraction,
            y_fraction,
            depth,
            order,
            target,
            target_depth,
            mission_target,
            rules_of_engagement,
            ships,
        } = fleet;
        self.u64(u64::from(fleet_number.0));
        self.u64(u64::from(owner.0));
        for link in [next, record, prev] {
            self.u64(u64::from(link.0));
        }
        self.u64(u64::from(*max_speed));
        self.u64(u64::from(*speed));
        self.coords(*sector);
        for real in [x_fraction, y_fraction, depth] {
            self.real(*real);
        }
        self.u64(u64::from(order.to_raw()));
        self.coords(*target);
        self.u64(u64::from(*target_depth));
        self.u64(u64::from(*mission_target));
        self.u64(u64::from(*rules_of_engagement));
        let ShipCounts {
            battleships,
            cruisers,
            destroyers,
            empty_transports,
            loaded_transports,
            etacs,
            scouts,
        } = ships;
        for count in [
            battleships,
            cruisers,
            destroyers,
            empty_transports,
            loaded_transports,
            etacs,
            scouts,
        ] {
            self.u64(u64::from(*count));
        }
    }

    fn starbase(&mut self, base: &Starbase) {
        let Starbase {
            escort,
            base_id,
            owner,
            next,
            record,
            prev,
            sector,
            x_fraction,
            y_fraction,
            depth,
            moving,
            target,
            target_depth,
        } = base;
        self.u64(u64::from(escort.0));
        self.u64(u64::from(base_id.0));
        self.u64(u64::from(owner.0));
        for link in [next, record, prev] {
            self.u64(u64::from(link.0));
        }
        self.coords(*sector);
        for real in [x_fraction, y_fraction, depth] {
            self.real(*real);
        }
        self.bool(*moving);
        self.coords(*target);
        self.u64(u64::from(*target_depth));
    }

    fn ipbm(&mut self, ipbm: &Ipbm) {
        let Ipbm {
            ipbm_number,
            owner,
            next,
            record,
            prev,
            sector,
            x_fraction,
            y_fraction,
            depth,
            target,
            target_depth,
        } = ipbm;
        self.u64(u64::from(*ipbm_number));
        self.u64(u64::from(owner.0));
        for link in [next, record, prev] {
            self.u64(u64::from(link.0));
        }
        self.coords(*sector);
        for real in [x_fraction, y_fraction, depth] {
            self.real(*real);
        }
        self.coords(*target);
        self.u64(u64::from(*target_depth));
    }

    fn setup(&mut self, setup: &Setup) {
        let Setup {
            version_tag,
            com_irq,
            com_hw_flow_control,
            snoop,
            max_key_gap_minutes,
            remote_timeout,
            local_timeout,
            minimum_time_minutes,
            purge_after_turns,
            autopilot_after_turns,
        } = setup;
        self.bytes(version_tag);
        self.bytes(com_irq);
        for flow_control in com_hw_flow_control {
            self.bool(*flow_control);
        }
        self.bool(*snoop);
        self.u64(u64::from(*max_key_gap_minutes));
        self.bool(*remote_timeout);
        self.bool(*local_timeout);
        self.u64(u64::from(*minimum_time_minutes));
        self.u64(u64::from(*purge_after_turns));
        self.u64(u64::from(*autopilot_after_turns));
    }

    fn conquest(&mut self, conquest: &Conquest) {
        let Conquest {
            game_year,
            player_count,
            maintenance_days: MaintenanceDays(days),
            previous_production,
            previous_planet_count,
        } = conquest;
        self.u64(u64::from(*game_year));
        self.u64(u64::from(*player_count));
        for day in days {
            self.bool(*day);
        }
        for production in previous_production {
            self.u64(u64::from(*production));
        }
        for count in previous_planet_count {
            self.u64(u64::from(*count));
        }
    }
}
