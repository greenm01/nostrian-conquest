use nc_host::commands::new_game;

#[test]
fn new_game_rejects_out_of_range_player_counts_before_creating_anything() {
    for players in ["0", "26", "256"] {
        let dir = std::env::temp_dir().join(format!(
            "nc-host-new-game-players-{players}-{}",
            std::process::id()
        ));
        let dir_arg = dir.to_str().expect("temp path should be UTF-8");

        let result = new_game::run(&[dir_arg, "--players", players]);

        assert!(result.is_err(), "--players {players} should be rejected");
        assert!(
            !dir.exists(),
            "--players {players} should not create {dir_arg}"
        );
    }
}
