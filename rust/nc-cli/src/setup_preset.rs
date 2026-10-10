use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPresetConfig {
    pub player_count: u8,
    pub seed: Option<u64>,
}

#[derive(Debug)]
pub enum SetupPresetError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for SetupPresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SetupPresetError {}

impl From<std::io::Error> for SetupPresetError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl SetupPresetConfig {
    pub fn parse_kdl_str(input: &str) -> Result<Self, SetupPresetError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| SetupPresetError::Parse(format!("invalid KDL: {err}")))?;

        for node in document.nodes() {
            if node.name().value() != "game" {
                return Err(SetupPresetError::Parse(format!(
                    "unsupported setup.kdl node: {}",
                    node.name().value()
                )));
            }
        }

        let game = document
            .get("game")
            .ok_or_else(|| SetupPresetError::Parse("missing game node".to_string()))?;
        let player_count = prop_u8(game, "player_count")?;
        let seed = opt_prop_u64(game, "seed")?;

        Ok(Self { player_count, seed }.validate()?)
    }

    pub fn load_kdl(path: &Path) -> Result<Self, SetupPresetError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    pub fn with_player_count_override(
        mut self,
        player_count: u8,
    ) -> Result<Self, SetupPresetError> {
        self.player_count = player_count;
        self.validate()
    }

    pub fn validate(self) -> Result<Self, SetupPresetError> {
        if !nc_data::is_valid_player_count(self.player_count) {
            return Err(SetupPresetError::Parse(format!(
                "player_count must be in 1..=25, got {}",
                self.player_count
            )));
        }

        Ok(self)
    }
}

fn prop_u8(node: &kdl::KdlNode, name: &str) -> Result<u8, SetupPresetError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            SetupPresetError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    u8::try_from(value)
        .map_err(|_| SetupPresetError::Parse(format!("property {name} out of u8 range: {value}")))
}

fn opt_prop_u64(node: &kdl::KdlNode, name: &str) -> Result<Option<u64>, SetupPresetError> {
    let Some(value) = node.get(name) else {
        return Ok(None);
    };
    let integer = value.as_integer().ok_or_else(|| {
        SetupPresetError::Parse(format!("missing or invalid integer property: {name}"))
    })?;
    let integer = u64::try_from(integer).map_err(|_| {
        SetupPresetError::Parse(format!("property {name} out of u64 range: {integer}"))
    })?;
    Ok(Some(integer))
}
