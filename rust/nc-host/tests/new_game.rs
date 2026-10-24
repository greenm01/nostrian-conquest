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

#[test]
fn new_game_writes_the_databases_and_no_classic_files() {
    let root = tempfile::Builder::new()
        .prefix("nc-host-new-game-files-")
        .tempdir()
        .expect("temp dir should create");
    let dir = root.path().join("files");
    let dir_arg = dir.to_str().expect("temp path should be UTF-8");

    new_game::run(&[dir_arg, "--players", "4", "--seed", "7"]).expect("new game should create");

    let mut names = std::fs::read_dir(&dir)
        .expect("game dir should list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("UTF-8")
        })
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, ["config.kdl", "hosted.db", "ncgame.db"]);
}
