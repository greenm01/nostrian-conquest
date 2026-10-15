//! AI seats in campaign maintenance (t165): a campaign whose seats are all
//! AI, advanced one stored maintenance run per year, plays the same game as
//! the in-memory simulator on the same seeds, and each seat's memory, saved
//! and restored between runs, matches the simulator's.

use std::path::{Path, PathBuf};

use nc_ai::{Weights, commission_submission, match_game};
use nc_ai_onnx::{AGENT_FAMILIES, CampaignAi, NetworkSeat};
use nc_data::{AiDifficulty, AiSeat, CampaignSettings, CampaignStore};
use nc_engine::CampaignMaintenance;

const PLAYERS: usize = 4;

fn model() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.onnx")
}

#[test]
fn a_campaign_of_ai_seats_plays_the_simulators_game() {
    let (map, years) = (61, 20);
    let mut game = match_game(map, map, &[false; PLAYERS], years);

    let dir = std::env::temp_dir().join(format!("nc-ai-campaign-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    let start = &game.play.built.game_data;
    let intel: Vec<_> = game
        .seats
        .iter()
        .map(|seat| seat.planet_intel.clone())
        .collect();
    store
        .save_runtime_state_structured_with_intel_and_seed(
            start.conquest.game_year,
            start,
            &[],
            &game.play.built.queued_mail,
            &intel,
            Some(map),
        )
        .unwrap();
    let mut settings = CampaignSettings::new("ai-test", "AI test");
    settings.inactivity_autopilot_after_turns = start.setup.autopilot_after_turns as u8;
    settings.ai_seats = (1..=PLAYERS)
        .map(|player| AiSeat {
            player_record_index_1_based: player,
            difficulty: AiDifficulty::Hard,
        })
        .collect();
    store.save_campaign_settings(&settings).unwrap();

    let seat = NetworkSeat::load(&model(), AGENT_FAMILIES, Weights::TUNED).unwrap();
    for _ in 0..years {
        // The simulator: each seat commissions and plays in seat order.
        for empire in 1..=PLAYERS as u8 {
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission).unwrap();
            }
            let memory = &game.seats[usize::from(empire) - 1];
            let turn = seat
                .turn(&game.play.built.game_data, memory, 0.0, 0)
                .unwrap();
            game.submit_turn(&turn).unwrap();
        }
        game.run_maintenance().unwrap();

        // The campaign: one stored maintenance run.
        let mut maintenance = CampaignMaintenance::load(&store).unwrap();
        let mut ai = CampaignAi::load(&store, &maintenance, Some(&model()))
            .unwrap()
            .expect("the campaign has AI seats");
        ai.play(&mut maintenance).unwrap();
        maintenance.run_turn().unwrap();
        ai.learn(&maintenance);
        let year = maintenance.game_data.conquest.game_year;
        maintenance.save(&store).unwrap();
        ai.save(&store, year).unwrap();

        let stored = store.load_latest_runtime_state().unwrap().unwrap();
        // Saving writes the year's reports, which the simulator does not
        // keep; their record ranges are the only other difference.
        let mut campaign = stored.game_data;
        for (player, simulated) in campaign
            .players
            .iter_mut()
            .zip(&game.play.built.game_data.players)
        {
            player.messages_first = simulated.messages_first;
            player.messages_last = simulated.messages_last;
            player.results_first = simulated.results_first;
            player.results_last = simulated.results_last;
        }
        assert_eq!(
            campaign, game.play.built.game_data,
            "year {year}: the campaign and the simulator differ"
        );
        let restored = CampaignMaintenance::load(&store).unwrap();
        let ai = CampaignAi::load(&store, &restored, Some(&model()))
            .unwrap()
            .unwrap();
        for (memory, simulated) in ai.memories().zip(&game.seats) {
            assert_eq!(
                memory, simulated,
                "year {year}: seat {}'s restored memory differs",
                memory.empire
            );
        }
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
