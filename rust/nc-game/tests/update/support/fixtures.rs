use super::*;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

pub(crate) struct TempGameDir {
    path: PathBuf,
}

impl TempGameDir {
    fn new(label: &str) -> Self {
        Self {
            path: temp_dir_path(label),
        }
    }
}

impl std::ops::Deref for TempGameDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Drop for TempGameDir {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "failed to remove temp game dir {}: {err}",
                self.path.display()
            );
        }
    }
}

pub(crate) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn temp_dir_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

pub(crate) fn temp_game_copy() -> TempGameDir {
    let root = TempGameDir::new("nc-game-update");
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let mut data = CoreGameData::load(&root).expect("load joinable fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player for standard client tests");
    data.rename_player_homeworld(1, "Codex Prime")
        .expect("name homeworld for standard client tests");
    data.save(&root).expect("save joined fixture");
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    root
}

pub(crate) fn temp_first_time_game_copy() -> TempGameDir {
    let root = TempGameDir::new("nc-game-first-time");
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    root
}

pub(crate) fn temp_joined_needs_homeworld_copy() -> TempGameDir {
    temp_joined_needs_homeworld_copy_for_player(1)
}

pub(crate) fn temp_joined_needs_homeworld_copy_for_player(
    player_record_index_1_based: usize,
) -> TempGameDir {
    let root = temp_first_time_game_copy();
    let mut data = CoreGameData::load(&root).expect("load joinable fixture");
    data.join_player(
        player_record_index_1_based,
        &format!("Empire {player_record_index_1_based}"),
    )
    .expect("join player without naming homeworld");
    data.save(&root).expect("save partially joined fixture");
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("refresh sqlite snapshot");
    root
}

pub(crate) fn reserved_game_config(player_record_index_1_based: usize, alias: &str) -> GameConfig {
    GameConfig {
        reservations: vec![SeatReservation {
            player_record_index_1_based,
            alias: alias.to_string(),
        }],
        ..GameConfig::default()
    }
}

pub(crate) fn temp_full_game_copy() -> TempGameDir {
    let root = temp_first_time_game_copy();
    let mut data = CoreGameData::load(&root).expect("load full-game fixture");
    for player in 1..=4 {
        data.join_player(player, &format!("Empire {player}"))
            .expect("join player for full-game fixture");
    }
    data.save(&root).expect("save full-game fixture");
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("refresh sqlite snapshot");
    root
}

pub(crate) fn temp_joined_no_assets_copy() -> TempGameDir {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    for planet in &mut state.game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    save_runtime_state(&root, &state);
    root
}

pub(crate) fn temp_joined_empty_empire_copy() -> TempGameDir {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    for planet in &mut state.game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut state.game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_owner_empire_raw(0);
        }
    }
    save_runtime_state(&root, &state);
    root
}

pub(crate) fn temp_game_with_starbase_copy() -> TempGameDir {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    state
        .game_data
        .set_guard_starbase(1, 1, [6, 5], 1, 1)
        .expect("seed guard starbase");
    save_runtime_state(&root, &state);
    root
}

pub(crate) fn first_empty_sector(game_data: &CoreGameData) -> [u8; 2] {
    let map_size = map_size_for_player_count(game_data.conquest.player_count());
    for x in 1..=map_size {
        for y in 1..=map_size {
            if game_data
                .planets
                .records
                .iter()
                .all(|planet| planet.coords_raw() != [x, y])
            {
                return [x, y];
            }
        }
    }
    panic!("fixture should contain at least one empty sector");
}

pub(crate) fn first_other_planet_coords(game_data: &CoreGameData, excluded: [u8; 2]) -> [u8; 2] {
    game_data
        .planets
        .records
        .iter()
        .map(|planet| planet.coords_raw())
        .find(|coords| *coords != excluded)
        .expect("fixture should contain another planet")
}

pub(crate) fn temp_game_with_auto_commission_copy() -> TempGameDir {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    let homeworld = state
        .game_data
        .planets
        .records
        .iter_mut()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("owned planet exists");
    homeworld.set_stardock_count_raw(0, 4);
    homeworld.set_stardock_count_raw(1, 2);
    homeworld.set_stardock_count_raw(6, 1);
    save_runtime_state(&root, &state);
    root
}

pub(crate) fn temp_game_with_same_sector_fleets_copy() -> TempGameDir {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    state.game_data.fleets.records[0].set_current_location_coords_raw([6, 5]);
    state.game_data.fleets.records[0].set_standing_order_target_coords_raw([6, 5]);
    state.game_data.fleets.records[1].set_current_location_coords_raw([6, 5]);
    state.game_data.fleets.records[1].set_standing_order_target_coords_raw([6, 5]);
    save_runtime_state(&root, &state);
    root
}

pub(crate) fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy file");
        }
    }
}
pub(crate) fn latest_runtime_state(root: &Path) -> CampaignRuntimeState {
    CampaignStore::open_default_in_dir(root)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load latest runtime state")
        .expect("campaign should have a latest runtime state")
}

pub(crate) fn save_runtime_state(root: &Path, state: &CampaignRuntimeState) {
    let player_count = state.game_data.conquest.player_count();
    let planet_intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(root)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    save_runtime_state_with_intel(root, state, &planet_intel_by_viewer);
}

pub(crate) fn save_runtime_state_with_intel(
    root: &Path,
    state: &CampaignRuntimeState,
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
) {
    let player_activity_states = CampaignStore::open_default_in_dir(root)
        .expect("open campaign store")
        .latest_player_activity_states(state.game_data.conquest.player_count())
        .expect("load player activity");
    save_runtime_state_with_intel_and_activity(
        root,
        state,
        planet_intel_by_viewer,
        &player_activity_states,
    );
}

pub(crate) fn save_runtime_state_with_intel_and_activity(
    root: &Path,
    state: &CampaignRuntimeState,
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    player_activity_states: &[nc_data::PlayerActivityState],
) {
    CampaignStore::open_default_in_dir(root)
        .expect("open campaign store")
        .save_runtime_state_structured_with_intel_and_activity(
            &state.game_data,
            &state.planet_scorch_orders,
            &state.report_block_rows,
            &state.queued_mail,
            planet_intel_by_viewer,
            player_activity_states,
        )
        .expect("save runtime state");
}

pub(crate) fn partial_known_world_snapshot(
    planet_record_index_1_based: usize,
    planet: &nc_data::PlanetRecord,
    owner_empire_id: u8,
    year: u16,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Partial,
        compat_is_orbit_seed: false,
        last_intel_year: Some(year),
        seen_year: Some(year),
        scout_year: Some(year),
        known_name: Some(planet.status_or_name_summary()),
        known_owner_empire_id: Some(owner_empire_id),
        known_potential_production: Some(planet.potential_production_points()),
        known_armies: None,
        known_ground_batteries: None,
        known_starbase_count: None,
        known_current_production: None,
        known_stored_points: None,
        known_docked_summary: None,
        known_orbit_summary: None,
        compat_word_1e: None,
    }
}

pub(crate) fn incoming_mail(
    sender_empire_id: u8,
    recipient_empire_id: u8,
    year: u16,
    subject: &str,
    body: &str,
) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id,
        recipient_empire_id,
        year,
        subject: subject.to_string(),
        body: body.to_string(),
        recipient_deleted: false,
    }
}

pub(crate) fn classic_chunked_report_bytes(text: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; 84];
    for (idx, byte) in text.bytes().take(75).enumerate() {
        bytes[idx + 1] = byte;
    }
    bytes
}

pub(crate) fn classic_chunked_report_blocks(texts: &[&str]) -> Vec<u8> {
    texts
        .iter()
        .flat_map(|text| classic_chunked_report_bytes(text))
        .collect()
}

pub(crate) fn length_prefixed_report_block(lines: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        let line_bytes = line.as_bytes();
        assert!(
            line_bytes.len() <= 72,
            "line too long for length-prefixed fixture"
        );
        let mut chunk = vec![0u8; 84];
        chunk[0] = 6;
        chunk[1] = line_bytes.len() as u8;
        chunk[2..2 + line_bytes.len()].copy_from_slice(line_bytes);
        bytes.extend_from_slice(&chunk);
    }
    bytes
}

pub(crate) fn set_runtime_report_blocks(state: &mut CampaignRuntimeState, bytes: impl AsRef<[u8]>) {
    state.report_block_rows = decode_report_block_rows(bytes.as_ref());
}

pub(crate) fn clear_runtime_report_blocks(state: &mut CampaignRuntimeState) {
    state.report_block_rows.clear();
}
pub(crate) fn make_runtime_db_read_only(root: &Path) {
    let db_path = root.join("ncgame.db");
    let mut permissions = fs::metadata(&db_path)
        .expect("runtime db metadata should load")
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&db_path, permissions).expect("runtime db should become read-only");
}

pub(crate) fn strongest_owned_fleet_number(root: &Path) -> u16 {
    latest_runtime_state(root)
        .game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1 && fleet.has_any_force())
        .max_by_key(|fleet| {
            (
                fleet.battleship_count(),
                fleet.cruiser_count(),
                fleet.destroyer_count(),
                fleet.troop_transport_count(),
                fleet.scout_count(),
                fleet.etac_count(),
                std::cmp::Reverse(fleet.local_slot_word_raw()),
            )
        })
        .expect("owned fleet exists")
        .local_slot_word_raw()
}
