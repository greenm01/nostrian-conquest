use std::fmt::Write;

use super::*;

impl CoreGameData {
    /// One empire's economy as text: its totals and ranks, then each planet
    /// it owns. `None` when the game has no such player.
    pub fn economy_report_text(&self, player_record_index_1_based: usize) -> Option<String> {
        let player = self
            .players
            .get(player_record_index_1_based.checked_sub(1)?)?;
        let economy = self.empire_economy_summary(player_record_index_1_based);
        let mut out = String::new();
        let _ = writeln!(
            out,
            "Economy report: player={} empire=\"{}\"",
            player_record_index_1_based,
            player.empire_name.trim()
        );
        let _ = writeln!(
            out,
            "  tax_rate={} present={} potential={} available={} last_run_year={}",
            economy.tax_rate,
            economy.present_production,
            economy.potential_production,
            economy.total_available_points,
            player.last_year_on
        );
        let _ = writeln!(
            out,
            "  planets={} rank_planets={} rank_production={} efficiency={:.3}%",
            economy.owned_planets,
            economy.rank_by_planets,
            economy.rank_by_present_production,
            economy.efficiency_percent
        );
        let _ = writeln!(out, "  planets:");
        let _ = writeln!(
            out,
            "    {:>3} {:>7} {:<13} {:>7} {:>9} {:>6} {:>6} {:>6} {:>5} {:>4} {:>4}",
            "rec",
            "coords",
            "name",
            "present",
            "potential",
            "stored",
            "rev",
            "grow",
            "cap",
            "army",
            "bat"
        );
        for row in self.empire_planet_economy_rows(player_record_index_1_based) {
            let [x, y] = row.coords;
            let _ = writeln!(
                out,
                "    {:>3} ({:02},{:02}) {:<13} {:>7} {:>9} {:>6} {:>6} {:>6} {:>5} {:>4} {:>4}{}{}",
                row.planet_record_index_1_based,
                x,
                y,
                row.planet_name,
                row.present_production,
                row.potential_production,
                row.stored_production_points,
                row.yearly_tax_revenue,
                row.yearly_growth_delta,
                row.build_capacity,
                row.armies,
                row.ground_batteries,
                if row.has_friendly_starbase {
                    " starbase"
                } else {
                    ""
                },
                if row.is_homeworld_seed {
                    " homeworld"
                } else {
                    ""
                }
            );
        }
        Some(out)
    }
}
