use std::collections::BTreeMap;

use crate::{
    CoreGameData, IntelTier, PlanetIntelSnapshot, active_starbase_count_at,
    map_size_for_player_count, present_production_points, stored_production_points,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStarmapWorld {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub intel_tier: IntelTier,
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_owner_empire_name: Option<String>,
    pub known_potential_production: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_ground_batteries: Option<u8>,
    pub known_starbase_count: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_stored_points: Option<u16>,
    pub known_docked_summary: Option<String>,
    pub known_orbit_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStarmapProjection {
    pub map_width: u8,
    pub map_height: u8,
    pub year: u16,
    pub viewer_empire_id: u8,
    pub worlds: Vec<PlayerStarmapWorld>,
}

impl PlayerStarmapProjection {
    pub fn render_ascii_map(&self) -> String {
        let height = self.map_height as usize;
        let width = self.map_width as usize;
        let mut occupied = vec![vec![false; width + 1]; height + 1];
        for world in &self.worlds {
            let x = world.coords[0] as usize;
            let y = world.coords[1] as usize;
            if x <= width && y <= height {
                occupied[y][x] = true;
            }
        }

        let mut pages = Vec::new();
        for start_x in (1..=width).step_by(18) {
            let end_x = usize::min(start_x + 17, width);
            let mut rows = Vec::new();
            rows.push(render_header_row(start_x, end_x));
            rows.push(render_border_row(start_x, end_x, true));

            for y in (1..=height).rev() {
                let mut row = format!("{y:02} |");
                for x in start_x..=end_x {
                    let ch = if occupied[y][x] { '*' } else { ' ' };
                    if x == end_x {
                        row.push_str(&format!(" {ch} "));
                    } else {
                        row.push_str(&format!(" {ch}  "));
                    }
                }
                row.push_str(&format!("| {y:02}"));
                rows.push(row);
                rows.push(render_border_row(start_x, end_x, y == 1));
            }
            rows.push(render_header_row(start_x, end_x));
            pages.push(rows.join("\n"));
        }

        pages.join("\n\n\x0c\n")
    }

    pub fn render_ascii_export(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "NOSTRIAN CONQUEST STARMAP  YEAR {}  EMPIRE {}\n\n",
            self.year, self.viewer_empire_id
        ));
        out.push_str(&self.render_ascii_map());
        let mut worlds = self.worlds.iter().collect::<Vec<_>>();
        worlds.sort_by_key(|world| (world.coords[1], world.coords[0]));
        let known_worlds = worlds
            .into_iter()
            .filter(|world| {
                world.known_name.is_some()
                    || world.known_owner_empire_name.is_some()
                    || world.known_potential_production.is_some()
                    || world.known_armies.is_some()
                    || world.known_ground_batteries.is_some()
            })
            .collect::<Vec<_>>();

        if !known_worlds.is_empty() {
            out.push_str("\n\nKNOWN WORLD DETAILS\n");
            out.push_str("-------------------\n");
        }

        for world in known_worlds {
            let known_name = world.known_name.as_deref().unwrap_or("UNKNOWN");
            let owner = world
                .known_owner_empire_name
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| "UNKNOWN".to_string());
            let prod = world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let armies = world
                .known_armies
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let batteries = world
                .known_ground_batteries
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());

            out.push_str(&format!(
                "({:02},{:02})  {:<14}  OWNER {:<20}  PROD {:>3}  ARM {:>3}  BAT {:>3}\n",
                world.coords[0], world.coords[1], known_name, owner, prod, armies, batteries
            ));
        }

        out
    }

    pub fn render_csv_export(&self) -> String {
        let width = self.map_width as usize;
        let height = self.map_height as usize;
        let mut occupied = vec![vec![false; width + 1]; height + 1];
        for world in &self.worlds {
            let x = world.coords[0] as usize;
            let y = world.coords[1] as usize;
            if x <= width && y <= height {
                occupied[y][x] = true;
            }
        }

        let mut out = String::new();
        for start_x in (1..=width).step_by(18) {
            let end_x = usize::min(start_x + 17, width);
            out.push(',');
            for x in start_x..=end_x {
                out.push_str(&format!("{x:02}"));
                if x != end_x {
                    out.push(',');
                }
            }
            out.push('\n');

            for y in (1..=height).rev() {
                out.push_str(&format!("{y:02}"));
                out.push(',');
                for x in start_x..=end_x {
                    let cell = if occupied[y][x] { "*" } else { "" };
                    out.push_str(cell);
                    if x != end_x {
                        out.push(',');
                    }
                }
                out.push('\n');
            }

            if end_x != width {
                out.push('\n');
            }
        }

        out
    }

    pub fn render_csv_details_export(&self) -> String {
        let mut out = String::from(
            "x,y,known_name,known_owner_empire_id,known_owner_empire_name,known_potential_production,known_armies,known_ground_batteries\n",
        );
        let mut worlds = self.worlds.iter().collect::<Vec<_>>();
        worlds.sort_by_key(|world| (world.coords[1], world.coords[0]));
        for world in worlds.into_iter().filter(|world| {
            world.known_name.is_some()
                || world.known_owner_empire_name.is_some()
                || world.known_potential_production.is_some()
                || world.known_armies.is_some()
                || world.known_ground_batteries.is_some()
        }) {
            out.push_str(&format!(
                "{:02},{:02},{},{},{},{},{},{}\n",
                world.coords[0],
                world.coords[1],
                csv_field(world.known_name.as_deref()),
                world
                    .known_owner_empire_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                csv_field(world.known_owner_empire_name.as_deref()),
                world
                    .known_potential_production
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                world
                    .known_armies
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                world
                    .known_ground_batteries
                    .map(|value| value.to_string())
                    .unwrap_or_default()
            ));
        }
        out
    }
}

fn render_header_row(start_x: usize, end_x: usize) -> String {
    let mut row = String::from("   ");
    for x in start_x..=end_x {
        let label = format!("{x:02}");
        row.push_str(&format!("{label:>4}"));
    }
    row
}

fn render_border_row(start_x: usize, end_x: usize, final_row: bool) -> String {
    let mut row = String::from("  -|");
    for _x in start_x..=end_x {
        if final_row {
            row.push_str("---|");
        } else {
            row.push_str("- -|");
        }
    }
    row.push('-');
    row
}

pub fn build_player_starmap_projection_from_snapshots(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire_id: u8,
) -> PlayerStarmapProjection {
    let map_size = map_size_for_player_count(game_data.conquest.player_count);
    let worlds = game_data
        .planets
        .iter()
        .enumerate()
        .map(|(planet_index, planet)| {
            let snapshot = snapshots.get(&(planet_index + 1));
            let actual_owner_empire_id = planet.owner.0;
            let is_owned_world = actual_owner_empire_id == viewer_empire_id;
            let known_name = if is_owned_world {
                Some(planet.name.trim().to_string())
            } else {
                snapshot.and_then(|row| row.known_name.clone())
            };
            let known_owner_empire_id = if is_owned_world {
                Some(viewer_empire_id)
            } else {
                snapshot.and_then(|row| row.known_owner_empire_id)
            };
            let known_owner_empire_name = known_owner_empire_id.and_then(|empire_id| {
                (empire_id >= 1).then(|| {
                    game_data.players[empire_id as usize - 1]
                        .empire_name
                        .trim()
                        .to_string()
                })
            });

            PlayerStarmapWorld {
                planet_record_index_1_based: planet_index + 1,
                coords: planet.sector.to_array(),
                intel_tier: if is_owned_world {
                    IntelTier::Owned
                } else {
                    IntelTier::infer_from_fields(
                        viewer_empire_id,
                        known_owner_empire_id,
                        known_name.as_deref(),
                        snapshot.and_then(|s| s.known_potential_production),
                        snapshot.and_then(|s| s.known_armies),
                        snapshot.and_then(|s| s.known_ground_batteries),
                    )
                },
                known_name,
                known_owner_empire_id,
                known_owner_empire_name,
                known_potential_production: if is_owned_world {
                    Some(u16::from(planet.potential_production))
                } else {
                    snapshot.and_then(|row| row.known_potential_production)
                },
                known_armies: if is_owned_world {
                    Some(u8::try_from(planet.armies).unwrap_or(u8::MAX))
                } else {
                    snapshot.and_then(|row| row.known_armies)
                },
                known_ground_batteries: if is_owned_world {
                    Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX))
                } else {
                    snapshot.and_then(|row| row.known_ground_batteries)
                },
                known_starbase_count: if is_owned_world {
                    Some(active_starbase_count_at(
                        game_data,
                        planet.sector.to_array(),
                    ))
                } else {
                    snapshot.and_then(|row| row.known_starbase_count)
                },
                known_current_production: if is_owned_world {
                    Some(present_production_points(planet)).map(|v| v as u8)
                } else {
                    snapshot.and_then(|row| row.known_current_production)
                },
                known_stored_points: if is_owned_world {
                    Some(stored_production_points(planet) as u16)
                } else {
                    snapshot.and_then(|row| row.known_stored_points)
                },
                known_docked_summary: if is_owned_world {
                    None
                } else {
                    snapshot.and_then(|row| row.known_docked_summary.clone())
                },
                known_orbit_summary: if is_owned_world {
                    None
                } else {
                    snapshot.and_then(|row| row.known_orbit_summary.clone())
                },
            }
        })
        .collect();

    PlayerStarmapProjection {
        map_width: map_size,
        map_height: map_size,
        year: game_data.conquest.game_year,
        viewer_empire_id,
        worlds,
    }
}

fn csv_field(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
