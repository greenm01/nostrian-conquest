//! The seven classic core files of a game directory: PLAYER.DAT,
//! PLANETS.DAT, FLEETS.DAT, BASES.DAT, IPBM.DAT, SETUP.DAT, and CONQUEST.DAT.
//! The runtime keeps game state in the campaign store; only explicit classic
//! workflows (import, export, oracle setup, and inspection of classic
//! directories) read or write these files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::classic::{
    parse_bases_dat, parse_conquest_dat, parse_fleets_dat, parse_ipbm_dat, parse_planets_dat,
    parse_player_dat, parse_setup_dat, serialize_bases_dat, serialize_conquest_dat,
    serialize_fleets_dat, serialize_ipbm_dat, serialize_planets_dat, serialize_player_dat,
    serialize_setup_dat,
};
use nc_classic::ParseError;
use nc_data::{CampaignStoreError, CoreGameData};

use crate::classic_parse_error;

#[derive(Debug)]
pub enum GameDirectoryError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: ParseError,
    },
}

impl std::fmt::Display for GameDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Parse { path, source } => write!(f, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for GameDirectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

impl From<GameDirectoryError> for CampaignStoreError {
    fn from(value: GameDirectoryError) -> Self {
        match value {
            GameDirectoryError::Io { path, source } => Self::Io { path, source },
            GameDirectoryError::Parse { source, .. } => classic_parse_error(source),
        }
    }
}

/// Decode the core files in `dir`. A missing IPBM.DAT reads as no IPBMs.
pub fn load_classic_core_files(dir: &Path) -> Result<CoreGameData, GameDirectoryError> {
    Ok(CoreGameData {
        players: load_parsed(dir, "PLAYER.DAT", parse_player_dat)?,
        planets: load_parsed(dir, "PLANETS.DAT", parse_planets_dat)?,
        fleets: load_parsed(dir, "FLEETS.DAT", parse_fleets_dat)?,
        bases: load_parsed(dir, "BASES.DAT", parse_bases_dat)?,
        ipbms: load_optional_parsed(dir, "IPBM.DAT", parse_ipbm_dat)?.unwrap_or_default(),
        setup: load_parsed(dir, "SETUP.DAT", parse_setup_dat)?,
        conquest: load_parsed(dir, "CONQUEST.DAT", parse_conquest_dat)?,
        sightings: Default::default(),
    })
}

/// Encode `game_data` as the core files in `dir`.
pub fn save_classic_core_files(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<(), GameDirectoryError> {
    save_bytes(dir, "PLAYER.DAT", &serialize_player_dat(&game_data.players))?;
    save_bytes(
        dir,
        "PLANETS.DAT",
        &serialize_planets_dat(&game_data.planets),
    )?;
    save_bytes(dir, "FLEETS.DAT", &serialize_fleets_dat(&game_data.fleets))?;
    save_bytes(dir, "BASES.DAT", &serialize_bases_dat(&game_data.bases))?;
    save_bytes(dir, "IPBM.DAT", &serialize_ipbm_dat(&game_data.ipbms))?;
    save_bytes(dir, "SETUP.DAT", &serialize_setup_dat(&game_data.setup))?;
    save_bytes(
        dir,
        "CONQUEST.DAT",
        &serialize_conquest_dat(&game_data.conquest),
    )?;
    Ok(())
}

fn load_parsed<T>(
    dir: &Path,
    file_name: &'static str,
    parse: impl Fn(&[u8]) -> Result<T, ParseError>,
) -> Result<T, GameDirectoryError> {
    let path = dir.join(file_name);
    let bytes = fs::read(&path).map_err(|source| GameDirectoryError::Io {
        path: path.clone(),
        source,
    })?;
    parse(&bytes).map_err(|source| GameDirectoryError::Parse { path, source })
}

fn load_optional_parsed<T>(
    dir: &Path,
    file_name: &'static str,
    parse: impl Fn(&[u8]) -> Result<T, ParseError>,
) -> Result<Option<T>, GameDirectoryError> {
    let path = dir.join(file_name);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(GameDirectoryError::Io {
                path: path.clone(),
                source,
            });
        }
    };
    parse(&bytes)
        .map(Some)
        .map_err(|source| GameDirectoryError::Parse { path, source })
}

fn save_bytes(dir: &Path, file_name: &'static str, bytes: &[u8]) -> Result<(), GameDirectoryError> {
    let path = dir.join(file_name);
    fs::write(&path, bytes).map_err(|source| GameDirectoryError::Io { path, source })
}
