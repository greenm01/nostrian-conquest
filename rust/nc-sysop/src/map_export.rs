//! `map-export`: a player's star map, as the in-game export writes it.

use std::fs;
use std::path::Path;

use nc_data::{CampaignStore, build_player_map_export_data};

pub fn export_player_map(
    dir: &Path,
    player: usize,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(runtime) = CampaignStore::open_default_in_dir(dir)?.load_latest_runtime_state()?
    else {
        return Err(format!("{} has no campaign", dir.display()).into());
    };
    let players = runtime.game_data.players.len();
    if player == 0 || player > players {
        return Err(format!("--player must be 1-{players}").into());
    }
    let export = build_player_map_export_data(dir, player)?;
    let stem = format!(
        "ECMAP-P{}-Y{}",
        player, runtime.game_data.conquest.game_year
    );
    fs::create_dir_all(out_dir)?;
    let paths = [
        (out_dir.join(format!("{stem}.TXT")), export.ascii_export),
        (out_dir.join(format!("{stem}.CSV")), export.csv_export),
        (
            out_dir.join(format!("{stem}-DETAILS.CSV")),
            export.csv_details_export,
        ),
    ];
    for (path, contents) in &paths {
        fs::write(path, contents)?;
    }
    println!(
        "Exported player {} star map to {}, {}, and {}",
        player,
        paths[0].0.display(),
        paths[1].0.display(),
        paths[2].0.display()
    );
    Ok(())
}
