use nc_data::{CoreGameData, EmpireProductionRankingSort};

pub fn build_rankings_text(game_data: &CoreGameData) -> String {
    let year = game_data.conquest.game_year();
    let stardate = crate::maint::timing::format_rankings_stardate(year);
    let rows = game_data.empire_production_ranking_rows(EmpireProductionRankingSort::Production);

    let mut out = String::new();
    out.push_str(&stardate);
    out.push('\n');
    out.push('\n');
    out.push_str("Empire Rankings (by production):\n");
    for (rank, row) in rows.iter().enumerate() {
        let name = if row.empire_name.is_empty() {
            format!("Empire #{}", row.empire_id)
        } else {
            format!("Empire #{} \"{}\"", row.empire_id, row.empire_name)
        };
        out.push_str(&format!(
            "  {}. {}  -- {} planet(s), {} production\n",
            rank + 1,
            name,
            row.planets_owned,
            row.current_production,
        ));
    }
    out
}
