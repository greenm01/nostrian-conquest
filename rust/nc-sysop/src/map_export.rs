//! `map-export`: a player's star map, as the in-game export writes it, or
//! the galaxy map with no empire's intel.

use std::fs;
use std::path::Path;

use nc_data::{CampaignStore, build_galaxy_map_export_data, build_player_map_export_data};

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

/// Where the worlds are, and nothing any empire knows about them. There is
/// no details file: with no intel it would list nothing.
pub fn export_galaxy_map(dir: &Path, out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let Some(runtime) = CampaignStore::open_default_in_dir(dir)?.load_latest_runtime_state()?
    else {
        return Err(format!("{} has no campaign", dir.display()).into());
    };
    let export = build_galaxy_map_export_data(dir)?;
    let stem = format!("ECMAP-GALAXY-Y{}", runtime.game_data.conquest.game_year);
    fs::create_dir_all(out_dir)?;
    let text_path = out_dir.join(format!("{stem}.TXT"));
    let csv_path = out_dir.join(format!("{stem}.CSV"));
    fs::write(&text_path, export.ascii_export)?;
    fs::write(&csv_path, export.csv_export)?;
    println!(
        "Exported the galaxy map to {} and {}",
        text_path.display(),
        csv_path.display()
    );
    Ok(())
}
