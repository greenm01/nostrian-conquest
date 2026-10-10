use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatReservation {
    pub player_record_index_1_based: usize,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BbsGameConfig {
    pub players: u8,
    pub reservations: Vec<SeatReservation>,
}

#[derive(Debug)]
pub enum BbsGameConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for BbsGameConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for BbsGameConfigError {}

impl From<std::io::Error> for BbsGameConfigError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl BbsGameConfig {
    pub fn parse_kdl_str(input: &str) -> Result<Self, BbsGameConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| BbsGameConfigError::Parse(format!("invalid KDL: {err}")))?;

        for node in document.nodes() {
            match node.name().value() {
                "players" | "reservations" => {}
                other => {
                    return Err(BbsGameConfigError::Parse(format!(
                        "unsupported BBS config field '{other}'; expected players or reservations"
                    )));
                }
            }
        }

        let players = document
            .get("players")
            .and_then(|node| node.get(0))
            .and_then(|value| value.as_integer())
            .ok_or_else(|| {
                BbsGameConfigError::Parse("players requires an integer argument".to_string())
            })
            .and_then(|value| {
                let players = u8::try_from(value).map_err(|_| {
                    BbsGameConfigError::Parse(format!("players out of u8 range: {value}"))
                })?;
                if !crate::is_valid_player_count(players) {
                    return Err(BbsGameConfigError::Parse(format!(
                        "players must be in 1..=25, got {players}"
                    )));
                }
                Ok(players)
            })?;

        let reservations = if let Some(node) = document.get("reservations") {
            let mut reservations = Vec::new();
            if let Some(children) = node.children() {
                for child in children.nodes() {
                    if child.name().value() != "seat" {
                        return Err(BbsGameConfigError::Parse(format!(
                            "reservations only accepts seat children, found '{}'",
                            child.name().value()
                        )));
                    }
                    let player_record_index_1_based = prop_usize(child, "player")?;
                    let alias = prop_string(child, "alias")?.trim().to_string();
                    if alias.is_empty() {
                        return Err(BbsGameConfigError::Parse(
                            "reservation alias must contain at least one visible character"
                                .to_string(),
                        ));
                    }
                    reservations.push(SeatReservation {
                        player_record_index_1_based,
                        alias,
                    });
                }
            }
            reservations
        } else {
            Vec::new()
        };

        Ok(Self {
            players,
            reservations,
        }
        .validate()?)
    }

    pub fn load_kdl(path: &Path) -> Result<Self, BbsGameConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    pub fn to_kdl_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("players {}\n", self.players));
        if !self.reservations.is_empty() {
            out.push_str("reservations {\n");
            for reservation in &self.reservations {
                out.push_str(&format!(
                    "    seat player={} alias=\"{}\"\n",
                    reservation.player_record_index_1_based,
                    kdl_escape(&reservation.alias),
                ));
            }
            out.push_str("}\n");
        }
        out
    }

    pub fn save_kdl(&self, path: &Path) -> Result<(), BbsGameConfigError> {
        fs::write(path, self.to_kdl_string())?;
        Ok(())
    }

    pub fn validate(self) -> Result<Self, BbsGameConfigError> {
        validate_reservations_with_error(&self.reservations, BbsGameConfigError::Parse)?;
        self.validate_reservations_for_player_count(self.players as usize)?;
        Ok(self)
    }

    pub fn reservation_for_alias(&self, alias: &str) -> Option<&SeatReservation> {
        let alias = alias.trim();
        self.reservations
            .iter()
            .find(|reservation| reservation.alias.eq_ignore_ascii_case(alias))
    }

    pub fn reservation_for_player(
        &self,
        player_record_index_1_based: usize,
    ) -> Option<&SeatReservation> {
        self.reservations.iter().find(|reservation| {
            reservation.player_record_index_1_based == player_record_index_1_based
        })
    }

    pub fn validate_reservations_for_player_count(
        &self,
        player_count: usize,
    ) -> Result<(), BbsGameConfigError> {
        validate_reservation_player_count(
            &self.reservations,
            player_count,
            BbsGameConfigError::Parse,
        )
    }
}

pub(crate) fn validate_reservations(reservations: &[SeatReservation]) -> Result<(), String> {
    validate_reservations_with_error(reservations, |message| message)
}

pub(crate) fn validate_reservation_player_count(
    reservations: &[SeatReservation],
    player_count: usize,
    wrap: impl Fn(String) -> BbsGameConfigError,
) -> Result<(), BbsGameConfigError> {
    for reservation in reservations {
        if reservation.player_record_index_1_based > player_count {
            return Err(wrap(format!(
                "reservation player {} exceeds player count {}",
                reservation.player_record_index_1_based, player_count
            )));
        }
    }
    Ok(())
}

fn validate_reservations_with_error<E>(
    reservations: &[SeatReservation],
    wrap: impl Fn(String) -> E,
) -> Result<(), E> {
    let mut seen_players = std::collections::BTreeSet::new();
    let mut seen_aliases = std::collections::BTreeSet::new();
    for reservation in reservations {
        if reservation.player_record_index_1_based == 0 {
            return Err(wrap("reservation player must be >= 1".to_string()));
        }
        if !seen_players.insert(reservation.player_record_index_1_based) {
            return Err(wrap(format!(
                "duplicate reservation for player {}",
                reservation.player_record_index_1_based
            )));
        }
        let alias_key = reservation.alias.to_ascii_lowercase();
        if !seen_aliases.insert(alias_key) {
            return Err(wrap(format!(
                "duplicate reservation alias '{}'",
                reservation.alias
            )));
        }
    }
    Ok(())
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, BbsGameConfigError> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires a string property")))
}

fn prop_usize(node: &kdl::KdlNode, name: &str) -> Result<usize, BbsGameConfigError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires an integer property")))?;
    usize::try_from(value)
        .map_err(|_| BbsGameConfigError::Parse(format!("{name} out of usize range: {value}")))
}

fn kdl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
