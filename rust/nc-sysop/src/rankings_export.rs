//! `rankings-export`: the year's empire rankings as KDL, for a BBS bulletin
//! or a website. Classic ECMAINT wrote RANKINGS.TXT on every run; NC writes
//! the rankings only when the sysop asks.

use std::fs;
use std::path::Path;

use kdl::{KdlDocument, KdlEntry, KdlNode};
use nc_data::{CampaignStore, CoreGameData, EmpireProductionRankingSort};

pub fn export_rankings(dir: &Path, out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let Some(runtime) = CampaignStore::open_default_in_dir(dir)?.load_latest_runtime_state()?
    else {
        return Err(format!("{} has no campaign", dir.display()).into());
    };
    let path = out_dir.join(format!(
        "RANKINGS-Y{}.kdl",
        runtime.game_data.conquest.game_year
    ));
    fs::create_dir_all(out_dir)?;
    fs::write(&path, rankings_kdl(&runtime.game_data))?;
    println!("Exported rankings to {}", path.display());
    Ok(())
}

/// Empires by production, as the in-game rankings screen sorts them.
fn rankings_kdl(game_data: &CoreGameData) -> String {
    let mut empires = KdlDocument::new();
    let rows = game_data.empire_production_ranking_rows(EmpireProductionRankingSort::Production);
    for (rank, row) in rows.into_iter().enumerate() {
        let mut empire = KdlNode::new("empire");
        empire.push(KdlEntry::new(i128::from(row.empire_id)));
        empire.push(("rank", rank as i128 + 1));
        empire.push(("name", row.empire_name));
        empire.push(("planets", row.planets_owned as i128));
        empire.push(("production", i128::from(row.current_production)));
        empire.push(("prev-planets", i128::from(row.previous_planets)));
        empire.push(("prev-production", i128::from(row.previous_production)));
        empires.nodes_mut().push(empire);
    }
    let mut rankings = KdlNode::new("rankings");
    rankings.push(("year", i128::from(game_data.conquest.game_year)));
    rankings.set_children(empires);
    let mut doc = KdlDocument::new();
    doc.nodes_mut().push(rankings);
    doc.autoformat();
    doc.to_string()
}
