//! Shared launch/session helpers for nc-game and other clients.

use nc_data::{CoreGameData, SeatReservation, player_handle_matches};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPlayerBindingSource {
    ExplicitPlayer,
    ReservedAlias,
    StoredHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPlayerBinding {
    Bound {
        player_record_index_1_based: usize,
        source: LaunchPlayerBindingSource,
    },
    UnboundDropfile,
}

impl LaunchPlayerBinding {
    pub fn player_record_index_1_based(self) -> Option<usize> {
        match self {
            Self::Bound {
                player_record_index_1_based,
                ..
            } => Some(player_record_index_1_based),
            Self::UnboundDropfile => None,
        }
    }

    pub fn source(self) -> Option<LaunchPlayerBindingSource> {
        match self {
            Self::Bound { source, .. } => Some(source),
            Self::UnboundDropfile => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LaunchBindingRequest<'a> {
    pub explicit_player_record_index_1_based: Option<usize>,
    pub dropfile_alias: Option<&'a str>,
    /// Seats whose stored handle the alias may not match. The DOOR.SYS
    /// real-name retry passes the seats bound by handle.
    pub skip_stored_handle_seats: &'a [usize],
    pub use_door_terminal: bool,
    pub reservations: &'a [SeatReservation],
    pub game_data: &'a CoreGameData,
}

pub fn resolve_launch_player_binding(
    request: LaunchBindingRequest<'_>,
) -> Result<LaunchPlayerBinding, String> {
    let player_count = request.game_data.players.len();

    if let Some(explicit_player) = request.explicit_player_record_index_1_based {
        if explicit_player > player_count {
            return Err(format!(
                "--player {} exceeds player count {}",
                explicit_player, player_count
            ));
        }
    }

    validate_reservations_for_player_count(request.reservations, player_count)?;

    let alias_reservation = request
        .dropfile_alias
        .and_then(|alias| reservation_for_alias(request.reservations, alias));

    if let Some(reservation) = alias_reservation {
        validate_reserved_seat_runtime(request.game_data, request.reservations, reservation)?;
        if let Some(explicit_player) = request.explicit_player_record_index_1_based {
            if explicit_player != reservation.player_record_index_1_based {
                return Err(format!(
                    "--player {} does not match reserved seat {} for alias '{}'",
                    explicit_player, reservation.player_record_index_1_based, reservation.alias
                ));
            }
        }
        return Ok(LaunchPlayerBinding::Bound {
            player_record_index_1_based: reservation.player_record_index_1_based,
            source: LaunchPlayerBindingSource::ReservedAlias,
        });
    }

    if let Some(alias) = request
        .dropfile_alias
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
    {
        let matching_players = request
            .game_data
            .players
            .iter()
            .enumerate()
            .filter(|(idx, _)| !request.skip_stored_handle_seats.contains(&(idx + 1)))
            .filter_map(|(idx, player)| {
                let handle = player.handle.trim().to_string();
                (!handle.is_empty() && player_handle_matches(player, alias)).then_some(idx + 1)
            })
            .collect::<Vec<_>>();

        if matching_players.len() > 1 {
            return Err(format!(
                "caller alias '{}' matches multiple joined empires; reserve the caller explicitly in ncgame.db",
                alias
            ));
        }

        if let Some(player_record_index_1_based) = matching_players.first().copied() {
            if let Some(reservation) =
                reservation_for_player(request.reservations, player_record_index_1_based)
            {
                if !reservation.alias.eq_ignore_ascii_case(alias) {
                    return Err(format!(
                        "caller alias '{}' conflicts with reserved alias '{}' for seat {}; reconcile ncgame.db settings or the campaign state",
                        alias, reservation.alias, player_record_index_1_based
                    ));
                }
            }
            if let Some(explicit_player) = request.explicit_player_record_index_1_based {
                if explicit_player != player_record_index_1_based {
                    return Err(format!(
                        "--player {} does not match stored handle seat {} for alias '{}'",
                        explicit_player, player_record_index_1_based, alias
                    ));
                }
            }
            return Ok(LaunchPlayerBinding::Bound {
                player_record_index_1_based,
                source: LaunchPlayerBindingSource::StoredHandle,
            });
        }
    }

    if let Some(explicit_player) = request.explicit_player_record_index_1_based {
        return Ok(LaunchPlayerBinding::Bound {
            player_record_index_1_based: explicit_player,
            source: LaunchPlayerBindingSource::ExplicitPlayer,
        });
    }

    if request.use_door_terminal {
        return Ok(LaunchPlayerBinding::UnboundDropfile);
    }

    Err(
        "usage: nc-game --dir <game_dir> --player <1-based empire index>\n       or use --dropfile for BBS/door mode"
            .to_string(),
    )
}

pub fn validate_reservations_for_player_count(
    reservations: &[SeatReservation],
    player_count: usize,
) -> Result<(), String> {
    for reservation in reservations {
        if reservation.player_record_index_1_based > player_count {
            return Err(format!(
                "reservation player {} exceeds player count {}",
                reservation.player_record_index_1_based, player_count
            ));
        }
    }
    Ok(())
}

pub fn validate_reserved_seat_runtime(
    game_data: &CoreGameData,
    reservations: &[SeatReservation],
    reservation: &SeatReservation,
) -> Result<(), String> {
    validate_reservations_for_player_count(reservations, game_data.players.len())?;
    let player = game_data
        .players
        .get(reservation.player_record_index_1_based - 1)
        .ok_or_else(|| {
            format!(
                "reserved player {} is missing from PLAYER.DAT",
                reservation.player_record_index_1_based
            )
        })?;
    let handle = player.handle.trim().to_string();
    if !handle.is_empty() && !player_handle_matches(player, &reservation.alias) {
        return Err(format!(
            "reserved alias '{}' conflicts with stored player handle '{}' for seat {}; reconcile ncgame.db settings or the campaign state",
            reservation.alias, handle, reservation.player_record_index_1_based
        ));
    }
    Ok(())
}

fn reservation_for_alias<'a>(
    reservations: &'a [SeatReservation],
    alias: &str,
) -> Option<&'a SeatReservation> {
    let alias = alias.trim();
    reservations
        .iter()
        .find(|reservation| reservation.alias.eq_ignore_ascii_case(alias))
}

fn reservation_for_player(
    reservations: &[SeatReservation],
    player_record_index_1_based: usize,
) -> Option<&SeatReservation> {
    reservations
        .iter()
        .find(|reservation| reservation.player_record_index_1_based == player_record_index_1_based)
}
