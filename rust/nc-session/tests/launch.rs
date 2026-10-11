use nc_data::{GameStateBuilder, SeatReservation, set_player_handle};
use nc_session::launch::{
    LaunchBindingRequest, LaunchPlayerBinding, LaunchPlayerBindingSource,
    resolve_launch_player_binding, validate_reserved_seat_runtime,
};
use nc_session::onboarding::{FirstTimeOnboardingMode, first_time_onboarding_mode};

#[test]
fn resolve_launch_binding_prefers_reserved_alias() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");

    let binding = resolve_launch_player_binding(LaunchBindingRequest {
        explicit_player_record_index_1_based: None,
        dropfile_alias: Some("sysop"),
        skip_stored_handle_seats: &[],
        use_door_terminal: true,
        reservations: &[SeatReservation {
            player_record_index_1_based: 2,
            alias: "SYSOP".to_string(),
        }],
        game_data: &game_data,
    })
    .expect("resolve binding");

    assert_eq!(
        binding,
        LaunchPlayerBinding::Bound {
            player_record_index_1_based: 2,
            source: LaunchPlayerBindingSource::ReservedAlias,
        }
    );
}

#[test]
fn resolve_launch_binding_returns_unbound_for_new_door_caller() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");

    let binding = resolve_launch_player_binding(LaunchBindingRequest {
        explicit_player_record_index_1_based: None,
        dropfile_alias: Some("newcaller"),
        skip_stored_handle_seats: &[],
        use_door_terminal: true,
        reservations: &[],
        game_data: &game_data,
    })
    .expect("resolve binding");

    assert_eq!(binding, LaunchPlayerBinding::UnboundDropfile);
}

/// Handles are stored with `?` for characters outside classic text and cut
/// to 25 characters; the caller's full handle must still find its seat.
#[test]
fn resolve_launch_binding_finds_handles_stored_in_classic_form() {
    for alias in [
        "Zo\u{eb} \u{c5}ngstr\u{f6}m",
        "A Handle Well Over Twenty-Five Characters",
    ] {
        let mut game_data = GameStateBuilder::new()
            .with_player_count(4)
            .build_joinable_new_game_baseline()
            .expect("build game data");
        set_player_handle(&mut game_data.players[0], alias);

        let binding = resolve_launch_player_binding(LaunchBindingRequest {
            explicit_player_record_index_1_based: None,
            dropfile_alias: Some(alias),
            skip_stored_handle_seats: &[],
            use_door_terminal: true,
            reservations: &[],
            game_data: &game_data,
        })
        .expect("resolve binding");
        assert_eq!(
            binding,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 1,
                source: LaunchPlayerBindingSource::StoredHandle,
            },
            "{alias}"
        );

        let reservations = [SeatReservation {
            player_record_index_1_based: 1,
            alias: alias.to_string(),
        }];
        validate_reserved_seat_runtime(&game_data, &reservations, &reservations[0])
            .unwrap_or_else(|err| panic!("{alias}: {err}"));
    }
}

#[test]
fn resolve_launch_binding_skips_listed_stored_handle_seats() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_joinable_new_game_baseline()
        .expect("build game data");
    game_data.join_player(2, "Empire 2").expect("join player");
    set_player_handle(&mut game_data.players[1], "Tachyon");
    let request = |skip_stored_handle_seats| LaunchBindingRequest {
        explicit_player_record_index_1_based: None,
        dropfile_alias: Some("tachyon"),
        skip_stored_handle_seats,
        use_door_terminal: true,
        reservations: &[],
        game_data: &game_data,
    };

    assert_eq!(
        resolve_launch_player_binding(request(&[])).expect("resolve binding"),
        LaunchPlayerBinding::Bound {
            player_record_index_1_based: 2,
            source: LaunchPlayerBindingSource::StoredHandle,
        }
    );
    assert_eq!(
        resolve_launch_player_binding(request(&[2])).expect("resolve binding"),
        LaunchPlayerBinding::UnboundDropfile
    );
}

#[test]
fn onboarding_helpers_choose_expected_modes() {
    assert_eq!(
        first_time_onboarding_mode(true),
        FirstTimeOnboardingMode::BbsReserved
    );
    assert_eq!(
        first_time_onboarding_mode(false),
        FirstTimeOnboardingMode::Generic
    );
}
