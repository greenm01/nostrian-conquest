//! `check`: whether a campaign is safe to play on: the file is intact, the
//! latest year loads, and maintenance would accept it.

use std::path::Path;

use nc_data::CampaignStore;

pub fn check_campaign(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !dir.join(nc_data::DEFAULT_CAMPAIGN_DB_NAME).exists() {
        return Err(format!("{} has no campaign", dir.display()).into());
    }
    // A badly damaged file fails to open before `quick_check` can run.
    let store = CampaignStore::open_default_in_dir(dir)
        .and_then(|store| store.check_integrity().map(|()| store))
        .map_err(|err| format!("the campaign file in {} is damaged: {err}", dir.display()))?;
    println!("File: intact");
    let Some(runtime) = store.load_latest_runtime_state()? else {
        return Err(format!("{} has no saved year", dir.display()).into());
    };
    let game = &runtime.game_data;
    println!(
        "Year {}: loads ({} players, {} planets, {} fleets)",
        game.conquest.game_year,
        game.players.len(),
        game.planets.len(),
        game.fleets.len()
    );
    let errors = game.ecmaint_structural_preflight_errors();
    if !errors.is_empty() {
        for error in &errors {
            println!("  {error}");
        }
        return Err(format!(
            "maintenance would refuse this campaign: {} problem(s)",
            errors.len()
        )
        .into());
    }
    println!("Maintenance: ready");
    Ok(())
}
