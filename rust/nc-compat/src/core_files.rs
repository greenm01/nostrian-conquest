//! The seven classic core files of a game directory: PLAYER.DAT,
//! PLANETS.DAT, FLEETS.DAT, BASES.DAT, IPBM.DAT, SETUP.DAT, and CONQUEST.DAT.
//! The runtime keeps game state in the campaign store; only explicit classic
//! workflows (import, export, oracle setup, and inspection of classic
//! directories) read or write these files.

use std::fs;
use std::path::{Path, PathBuf};

use nc_data::{
    BaseDat, CampaignStoreError, ConquestDat, CoreGameData, FleetDat, IpbmDat, ParseError,
    PlanetDat, PlayerDat, SetupDat,
};

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
            GameDirectoryError::Parse { source, .. } => Self::Parse(source),
        }
    }
}

/// Decode the core files in `dir`. A missing IPBM.DAT reads as no IPBMs.
pub fn load_classic_core_files(dir: &Path) -> Result<CoreGameData, GameDirectoryError> {
    Ok(CoreGameData {
        player: load_parsed(dir, "PLAYER.DAT", PlayerDat::parse)?,
        planets: load_parsed(dir, "PLANETS.DAT", PlanetDat::parse)?,
        fleets: load_parsed(dir, "FLEETS.DAT", FleetDat::parse)?,
        bases: load_parsed(dir, "BASES.DAT", BaseDat::parse)?,
        ipbm: load_optional_parsed(dir, "IPBM.DAT", IpbmDat::parse)?
            .unwrap_or_else(|| IpbmDat { records: vec![] }),
        setup: load_parsed(dir, "SETUP.DAT", SetupDat::parse)?,
        conquest: load_parsed(dir, "CONQUEST.DAT", ConquestDat::parse)?,
    })
}

/// Encode `game_data` as the core files in `dir`.
pub fn save_classic_core_files(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<(), GameDirectoryError> {
    save_bytes(dir, "PLAYER.DAT", &game_data.player.to_bytes())?;
    save_bytes(dir, "PLANETS.DAT", &game_data.planets.to_bytes())?;
    save_bytes(dir, "FLEETS.DAT", &game_data.fleets.to_bytes())?;
    save_bytes(dir, "BASES.DAT", &game_data.bases.to_bytes())?;
    save_bytes(dir, "IPBM.DAT", &game_data.ipbm.to_bytes())?;
    save_bytes(dir, "SETUP.DAT", &game_data.setup.to_bytes())?;
    save_bytes(dir, "CONQUEST.DAT", &game_data.conquest.to_bytes())?;
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
