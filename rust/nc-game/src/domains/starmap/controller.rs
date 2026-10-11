use crate::app::state::App;
use crate::screen::{CommandMenu, ScreenId};
use nc_data::build_player_starmap_projection_from_snapshots;
use nc_engine::map_size_for_player_count;

impl App {
    fn starmap_dump_page_lines(&self) -> usize {
        crate::domains::starmap::screens::starmap::starmap_dump_page_lines(self.screen_geometry)
    }

    pub fn open_partial_starmap_view(&mut self, menu: CommandMenu) {
        self.command_return_menu = menu;
        self.return_screen = None;
        let default = self.default_planet_prompt_coords();
        self.starmap_state.partial_status = None;
        self.starmap_state.partial_center = default;
        self.current_screen = ScreenId::PartialStarmapView;
    }

    pub fn open_starmap(&mut self) {
        self.starmap_state.view_x = 1;
        self.starmap_state.view_y = 1;
        self.starmap_state.status = None;
        self.starmap_state.dump_lines.clear();
        self.starmap_state.dump_offset = 0;
        self.starmap_state.dump_active = false;
        self.starmap_state.capture_complete = false;
        self.current_screen = ScreenId::Starmap;
    }

    pub fn move_partial_starmap(&mut self, dx: i8, dy: i8) {
        let map_size = map_size_for_player_count(self.game_data.conquest.player_count);
        self.starmap_state.partial_center[0] = self.starmap_state.partial_center[0]
            .saturating_add_signed(dx)
            .clamp(1, map_size);
        self.starmap_state.partial_center[1] = self.starmap_state.partial_center[1]
            .saturating_add_signed(dy)
            .clamp(1, map_size);
        self.starmap_state.partial_status = None;
    }

    pub fn open_partial_starmap_planet_info(&mut self) {
        let coords = self.starmap_state.partial_center;
        let _ = self.open_planet_info_detail_at_coords(coords, Some(ScreenId::PartialStarmapView));
    }

    pub fn export_starmap(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        );
        std::fs::create_dir_all(&self.export_root)?;
        let filename = format!(
            "ECMAP-P{}-Y{}.TXT",
            self.player.record_index_1_based, self.game_data.conquest.game_year
        );
        let export_path = self.export_root.join(&filename);
        let csv_path = self.export_root.join(filename.replace(".TXT", ".CSV"));
        let details_csv_path = self
            .export_root
            .join(filename.replace(".TXT", "-DETAILS.CSV"));
        std::fs::write(&export_path, projection.render_ascii_export())?;
        std::fs::write(&csv_path, projection.render_csv_export())?;
        std::fs::write(&details_csv_path, projection.render_csv_details_export())?;
        if let Some(queue_dir) = &self.queue_dir {
            std::fs::create_dir_all(queue_dir)?;
            std::fs::copy(&export_path, queue_dir.join(&filename))?;
            std::fs::copy(&csv_path, queue_dir.join(csv_path.file_name().unwrap()))?;
            std::fs::copy(
                &details_csv_path,
                queue_dir.join(details_csv_path.file_name().unwrap()),
            )?;
            self.starmap_state.status = Some(format!(
                "Exported TXT + grid CSV + details CSV and queued copies in {}",
                queue_dir.display()
            ));
        } else {
            self.starmap_state.status = Some(format!(
                "Exported {}, {}, and {}",
                export_path.display(),
                csv_path.display(),
                details_csv_path.display()
            ));
        }
        Ok(())
    }

    pub fn starmap_dump_text(&self) -> String {
        build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        )
        .render_ascii_map()
    }

    pub fn begin_starmap_dump(&mut self) {
        self.starmap_state.dump_lines = self
            .starmap_dump_text()
            .lines()
            .map(|line| line.to_string())
            .collect();
        self.starmap_state.dump_offset = 0;
        self.starmap_state.dump_active = true;
        self.starmap_state.capture_complete = false;
    }

    pub fn advance_starmap_page(&mut self) {
        if !self.starmap_state.dump_active {
            return;
        }
        let next_offset = self
            .starmap_state
            .dump_offset
            .saturating_add(self.starmap_dump_page_lines());
        if next_offset >= self.starmap_state.dump_lines.len() {
            self.starmap_state.dump_active = false;
            self.starmap_state.capture_complete = true;
        } else {
            self.starmap_state.dump_offset = next_offset;
        }
    }
}
