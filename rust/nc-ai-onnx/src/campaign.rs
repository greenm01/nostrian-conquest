//! AI seats in a campaign's maintenance (t165).
//!
//! A sysop marks seats as AI in the campaign settings. An AI seat joins its
//! empire, as "AI Empire N", the first time it plays. Each maintenance year
//! every AI seat that may still play commissions its finished ships and
//! submits its turn, as a player would, before the year runs; after the
//! year it reads its reports into its memory. Its memory is saved with the
//! campaign and restored on the next run. A memory saved for another year
//! than the campaign's (after a restore, say) is replaced by the stored
//! planet intel, without remembered contacts.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use nc_ai::{Families, Personality, SeatMemory, commission_submission, personality};
use nc_data::{
    AiDifficulty, AiSeat, CampaignStore, PlayerAccessMode, TurnSubmission, player_access_mode,
};
use nc_engine::CampaignMaintenance;

use crate::NetworkSeat;

/// The players the exported agent's graph is built for: its diplomacy
/// inputs are fixed at four empires.
pub const AGENT_PLAYERS: usize = 4;

/// The environment variable that names the agent a host's AI seats play.
pub const MODEL_ENV: &str = "NC_AI_MODEL";

/// The agent named by [`MODEL_ENV`], if set.
pub fn model_from_env() -> Option<std::path::PathBuf> {
    std::env::var_os(MODEL_ENV).map(std::path::PathBuf::from)
}

/// The temperature each difficulty plays at. Greedy is the agent at full
/// strength.
pub fn temperature(difficulty: AiDifficulty) -> f32 {
    match difficulty {
        AiDifficulty::Hard => 0.0,
        AiDifficulty::Normal => 1.0,
        AiDifficulty::Easy => 2.0,
    }
}

/// The families the shipped agent orders itself; the executor orders the
/// rest (t157).
pub const AGENT_FAMILIES: Families = Families {
    colonize: false,
    scout: false,
    strike: true,
};

/// A campaign's AI seats during one maintenance run: each seat with the
/// network seat of its personality, and its memory.
pub struct CampaignAi {
    networks: BTreeMap<String, NetworkSeat>,
    seats: Vec<(AiSeat, SeatMemory)>,
    stale: Vec<(usize, u16)>,
}

impl CampaignAi {
    /// The campaign's AI seats, playing the agent at `model`, or `None` when
    /// it has none. A campaign with AI seats needs a model.
    pub fn load(
        store: &CampaignStore,
        maintenance: &CampaignMaintenance,
        model: Option<&Path>,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        let Some(settings) = store.campaign_settings()? else {
            return Ok(None);
        };
        if settings.ai_seats.is_empty() {
            return Ok(None);
        }
        let players = maintenance.game_data.players.len();
        if players != AGENT_PLAYERS {
            return Err(format!(
                "AI seats play {AGENT_PLAYERS}-player campaigns only; this one has {players} players"
            )
            .into());
        }
        let model = model.ok_or(format!(
            "this campaign has AI seats; set {MODEL_ENV} to the agent's .onnx file"
        ))?;
        let mut networks = BTreeMap::new();
        for seat in &settings.ai_seats {
            if networks.contains_key(&seat.personality) {
                continue;
            }
            let style: Personality = personality(&seat.personality).ok_or(format!(
                "AI seat {} has unknown personality {:?}",
                seat.player_record_index_1_based, seat.personality
            ))?;
            let network = NetworkSeat::load(model, AGENT_FAMILIES, style)
                .map_err(|err| format!("AI model {}: {err}", model.display()))?;
            networks.insert(seat.personality.clone(), network);
        }
        let year = maintenance.game_data.conquest.game_year;
        let mut stale = Vec::new();
        let seats = settings
            .ai_seats
            .into_iter()
            .map(|seat| {
                let player = seat.player_record_index_1_based;
                let memory = match store.load_ai_seat_memory(player)? {
                    Some((saved, text)) if saved == year => serde_json::from_str(&text)?,
                    saved => {
                        if let Some((saved, _)) = saved {
                            stale.push((player, saved));
                        }
                        SeatMemory {
                            empire: player as u8,
                            planet_intel: store
                                .latest_planet_intel_for_viewer(player as u8)?
                                .into_iter()
                                .map(|intel| (intel.planet_record_index_1_based, intel))
                                .collect(),
                            contacts: Vec::new(),
                        }
                    }
                };
                Ok((seat, memory))
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
        Ok(Some(Self {
            networks,
            seats,
            stale,
        }))
    }

    /// The seats whose saved memory was learned up to another year than the
    /// campaign's, as when a run stopped between saving the campaign and
    /// saving the memory (t194), with that year. They start again from their
    /// stored planet intel, without remembered contacts.
    pub fn stale_seats(&self) -> &[(usize, u16)] {
        &self.stale
    }

    /// Before a year runs: every AI seat that may play commissions its
    /// ships and submits its turn.
    pub fn play(&self, maintenance: &mut CampaignMaintenance) -> Result<(), Box<dyn Error>> {
        for (seat, memory) in &self.seats {
            let player = seat.player_record_index_1_based;
            let access = player_access_mode(
                player,
                &maintenance.player_lifecycle_states,
                maintenance.winner_state,
            );
            if access != PlayerAccessMode::Normal {
                continue;
            }
            // A new game's seats wait in civil disorder, on autopilot,
            // until their player joins; an AI seat joins when it first
            // plays.
            if maintenance
                .game_data
                .player_slot_is_open_for_first_join(player)
            {
                maintenance
                    .game_data
                    .join_player(player, &format!("AI Empire {player}"))
                    .map_err(|err| format!("AI seat {player} cannot join: {err}"))?;
            }
            if let Some(commission) = commission_submission(&maintenance.game_data, memory.empire) {
                apply(maintenance, &commission)?;
            }
            let year = maintenance.game_data.conquest.game_year;
            let turn = self.networks[&seat.personality].turn(
                &maintenance.game_data,
                memory,
                temperature(seat.difficulty),
                (u64::from(year) << 8) ^ player as u64,
            )?;
            apply(maintenance, &turn)?;
        }
        Ok(())
    }

    /// After a year runs: every AI seat reads that year's reports.
    pub fn learn(&mut self, maintenance: &CampaignMaintenance) {
        for (_, memory) in &mut self.seats {
            memory.learn_year(&maintenance.game_data, maintenance.last_year_events());
        }
    }

    /// Save every AI seat's memory, learned up to `game_year`. Call after
    /// the maintenance run is saved.
    pub fn save(&self, store: &CampaignStore, game_year: u16) -> Result<(), Box<dyn Error>> {
        for (seat, memory) in &self.seats {
            store.save_ai_seat_memory(
                seat.player_record_index_1_based,
                game_year,
                &serde_json::to_string(memory)?,
            )?;
        }
        Ok(())
    }

    /// The seats' memories, in seat order.
    pub fn memories(&self) -> impl Iterator<Item = &SeatMemory> {
        self.seats.iter().map(|(_, memory)| memory)
    }
}

fn apply(
    maintenance: &mut CampaignMaintenance,
    turn: &TurnSubmission,
) -> Result<(), Box<dyn Error>> {
    turn.apply_to_runtime_state(
        &mut maintenance.game_data,
        &mut maintenance.queued_mail,
        &maintenance.player_lifecycle_states,
        maintenance.winner_state,
        &mut maintenance.player_activity_states,
    )
    .map_err(|err| format!("AI seat {}: {err}", turn.player_record_index_1_based))?;
    Ok(())
}
