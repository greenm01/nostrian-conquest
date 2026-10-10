use std::fmt;
use std::fs;
use std::path::Path;

use crate::DiplomaticRelation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiplomacyDirective {
    pub from_empire_raw: u8,
    pub to_empire_raw: u8,
    pub relation: DiplomaticRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiplomacyConfig {
    pub directives: Vec<DiplomacyDirective>,
}

#[derive(Debug)]
pub enum SetupConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for SetupConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SetupConfigError {}

impl From<std::io::Error> for SetupConfigError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl DiplomacyConfig {
    pub fn parse_kdl_str(input: &str) -> Result<Self, SetupConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| SetupConfigError::Parse(format!("invalid KDL: {err}")))?;

        let mut directives = Vec::new();
        for node in document.nodes() {
            if node.name().value() != "relation" {
                continue;
            }

            let relation = match prop_string(node, "status")?.as_str() {
                "enemy" => DiplomaticRelation::Enemy,
                "neutral" => DiplomaticRelation::Neutral,
                other => {
                    return Err(SetupConfigError::Parse(format!(
                        "unknown diplomacy status: {other}"
                    )));
                }
            };

            directives.push(DiplomacyDirective {
                from_empire_raw: prop_u8(node, "from")?,
                to_empire_raw: prop_u8(node, "to")?,
                relation,
            });
        }

        Ok(Self { directives })
    }

    pub fn load_kdl(path: &Path) -> Result<Self, SetupConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    pub fn to_kdl_string(&self) -> String {
        let mut text = String::new();
        for directive in &self.directives {
            let status = match directive.relation {
                DiplomaticRelation::Neutral | DiplomaticRelation::Unknown(_) => "neutral",
                DiplomaticRelation::Enemy => "enemy",
            };
            text.push_str(&format!(
                "relation from={} to={} status=\"{}\"\n",
                directive.from_empire_raw, directive.to_empire_raw, status
            ));
        }
        text
    }

    pub fn validate_for_player_count(self, player_count: u8) -> Result<Self, SetupConfigError> {
        let mut seen = std::collections::BTreeSet::new();
        for directive in &self.directives {
            if directive.from_empire_raw == 0 || directive.from_empire_raw > player_count {
                return Err(SetupConfigError::Parse(format!(
                    "diplomacy relation 'from' must be in 1..={player_count}, got {}",
                    directive.from_empire_raw
                )));
            }
            if directive.to_empire_raw == 0 || directive.to_empire_raw > player_count {
                return Err(SetupConfigError::Parse(format!(
                    "diplomacy relation 'to' must be in 1..={player_count}, got {}",
                    directive.to_empire_raw
                )));
            }
            if directive.from_empire_raw == directive.to_empire_raw {
                return Err(SetupConfigError::Parse(
                    "diplomacy relation cannot target the same empire".to_string(),
                ));
            }
            if !seen.insert((directive.from_empire_raw, directive.to_empire_raw)) {
                return Err(SetupConfigError::Parse(format!(
                    "duplicate diplomacy relation from empire {} to {}",
                    directive.from_empire_raw, directive.to_empire_raw
                )));
            }
        }

        Ok(self)
    }
}

fn prop_u8(node: &kdl::KdlNode, name: &str) -> Result<u8, SetupConfigError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            SetupConfigError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    u8::try_from(value)
        .map_err(|_| SetupConfigError::Parse(format!("property {name} out of u8 range: {value}")))
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, SetupConfigError> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| {
            SetupConfigError::Parse(format!("missing or invalid string property: {name}"))
        })
}
