use nc_data::{GameStateMutationError, name_unnamed_worlds, set_planet_name};

#[test]
fn ai_worlds_get_names_no_other_planet_has() {
    let mut game_data = nc_engine::build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    // Empire 2 holds ten unnamed worlds; another empire already took "Basil",
    // the head of the list.
    for planet in game_data.planets.iter_mut().take(10) {
        planet.owner.0 = 2;
        set_planet_name(planet, "Not Named Yet");
    }
    set_planet_name(&mut game_data.planets[10], "BASIL");
    name_unnamed_worlds(&mut game_data, 2);

    let names: Vec<String> = game_data
        .planets
        .iter()
        .map(|planet| planet.name.trim().to_ascii_lowercase())
        .collect();
    assert!(
        names[..10].iter().all(|name| name != "not named yet"),
        "{names:?}"
    );
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "a name repeats: {names:?}");
}

#[test]
fn a_player_cannot_reuse_a_planet_name() {
    let mut game_data = nc_engine::build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    game_data.planets[0].owner.0 = 1;
    set_planet_name(&mut game_data.planets[1], "Polycarp");
    let err = game_data
        .rename_owned_planet(1, 1, "polycarp")
        .expect_err("another planet has the name");
    assert!(matches!(
        err,
        GameStateMutationError::DuplicatePlanetName {
            planet_index_1_based: 1
        }
    ));
    game_data
        .rename_owned_planet(1, 1, "Ephrem")
        .expect("an unused name");
    game_data
        .rename_owned_planet(1, 1, "Ephrem")
        .expect("a world keeps its own name");
}
