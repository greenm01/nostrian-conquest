//! The compact ship notation (`docs/dev/ship-notation.md`): `CODEtotal!crippled`
//! tokens, separated by single spaces. Every fleet composition on every
//! screen comes from here, so the door, the helm, and the reports cannot
//! drift into different dialects.
//!
//! The total always includes the crippled ships, and `!crippled` is left
//! off when it is zero. `TT` is an empty transport and `TT*` a loaded one,
//! two categories that each get their own token.

use crate::{AutoCommissionFleetEntry, Fleet, HullClass};

/// The short legend the door's help and detail screens repeat.
pub const NOTATION_LEGEND: &str = "!n = crippled ships included in total; TT* = loaded transports";

/// A kind of ship a fleet list, a detach, or a transfer tells apart. Loaded
/// and empty transports are separate categories, as in every composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipCategory {
    Scouts,
    Battleships,
    Cruisers,
    Destroyers,
    FullTransports,
    EmptyTransports,
    Etacs,
}

impl ShipCategory {
    pub const COUNT: usize = 7;

    /// The fleet screens' class order.
    pub const ALL: [ShipCategory; Self::COUNT] = [
        ShipCategory::Scouts,
        ShipCategory::Battleships,
        ShipCategory::Cruisers,
        ShipCategory::Destroyers,
        ShipCategory::FullTransports,
        ShipCategory::EmptyTransports,
        ShipCategory::Etacs,
    ];

    /// The category's place in [`ShipCategory::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// The code the notation writes: `SC`, `BB`, `CA`, `DD`, `TT*`, `TT`,
    /// `ETAC`.
    pub fn code(self) -> &'static str {
        match self {
            Self::Scouts => "SC",
            Self::Battleships => "BB",
            Self::Cruisers => "CA",
            Self::Destroyers => "DD",
            Self::FullTransports => "TT*",
            Self::EmptyTransports => "TT",
            Self::Etacs => "ETAC",
        }
    }

    /// The category a typed code names. `ET` still works for `ETAC`.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "ET" => Some(Self::Etacs),
            code => Self::ALL
                .into_iter()
                .find(|category| category.code() == code),
        }
    }

    /// The hull class whose crippled counter covers this category.
    pub fn hull_class(self) -> HullClass {
        match self {
            Self::Scouts => HullClass::Scout,
            Self::Battleships => HullClass::Battleship,
            Self::Cruisers => HullClass::Cruiser,
            Self::Destroyers => HullClass::Destroyer,
            Self::FullTransports | Self::EmptyTransports => HullClass::Transport,
            Self::Etacs => HullClass::Etac,
        }
    }

    /// How many ships of this category the fleet has.
    pub fn ships_in(self, fleet: &Fleet) -> u32 {
        let ships = &fleet.ships;
        u32::from(match self {
            Self::Scouts => ships.scouts,
            Self::Battleships => ships.battleships,
            Self::Cruisers => ships.cruisers,
            Self::Destroyers => ships.destroyers,
            Self::FullTransports => ships.loaded_transports,
            Self::EmptyTransports => ships.empty_transports,
            Self::Etacs => ships.etacs,
        })
    }
}

/// One token: `CA12!3`, `CA12`. Nothing for a class with no ships. The
/// crippled ships are part of the total, so they never exceed it.
pub fn ship_token(code: &str, total: u32, crippled: u32) -> Option<String> {
    match (total, crippled.min(total)) {
        (0, _) => None,
        (total, 0) => Some(format!("{code}{total}")),
        (total, crippled) => Some(format!("{code}{total}!{crippled}")),
    }
}

/// Tokens in the order given, one space apart.
pub fn join_tokens(tokens: impl IntoIterator<Item = Option<String>>) -> String {
    tokens.into_iter().flatten().collect::<Vec<_>>().join(" ")
}

/// A fleet's crippled ships per category, indexed like [`ShipCategory::ALL`].
///
/// A fleet keeps one crippled counter for all its troop transports, loaded
/// or empty, so the counter alone cannot say which transports are crippled.
/// The rule is the one the engine already follows for transports lost in a
/// battle: loaded transports come first. Every screen, every selection, and
/// every preview reads the split from this function, so what a player sees
/// is what a detach or transfer takes.
pub fn crippled_by_category(fleet: &Fleet) -> [u32; ShipCategory::COUNT] {
    let mut crippled = [0; ShipCategory::COUNT];
    for category in ShipCategory::ALL {
        let ships = category.ships_in(fleet);
        let hulls = fleet.crippled.get(category.hull_class()).total();
        crippled[category.index()] = match category {
            ShipCategory::FullTransports => hulls.min(ships),
            ShipCategory::EmptyTransports => {
                let loaded = ShipCategory::FullTransports.ships_in(fleet);
                hulls.saturating_sub(loaded).min(ships)
            }
            _ => hulls.min(ships),
        };
    }
    crippled
}

/// The fleet's ships in notation: `SC1 BB2!1 CA12!3 DD8 TT*4!1 TT5 ETAC2`.
/// Empty when the fleet has no ships.
pub fn fleet_notation(fleet: &Fleet) -> String {
    let crippled = crippled_by_category(fleet);
    join_tokens(ShipCategory::ALL.into_iter().map(|category| {
        ship_token(
            category.code(),
            category.ships_in(fleet),
            crippled[category.index()],
        )
    }))
}

/// A starbase count in notation: `SB1`, `SB1!1`.
pub fn starbase_token(total: u32, crippled: u32) -> Option<String> {
    ship_token("SB", total, crippled)
}

/// The new ships of an auto-commission entry in notation, as the commission
/// notice lists them (`DD4 CA2`). New ships are undamaged and their
/// transports empty.
pub fn auto_commission_notation(entry: &AutoCommissionFleetEntry) -> String {
    join_tokens([
        ship_token("DD", entry.destroyers, 0),
        ship_token("CA", entry.cruisers, 0),
        ship_token("BB", entry.battleships, 0),
        ship_token("SC", entry.scouts, 0),
        ship_token("TT", entry.transports, 0),
        ship_token("ETAC", entry.etacs, 0),
    ])
}

/// A composition cut to `width` characters without splitting a token: the
/// tokens that fit, then ` +` for the rest. For a fleet with crippled ships
/// the cut ends in ` +!`, so a lone `!` still says the fleet is damaged
/// when the damaged classes are among the tokens cut away. A composition that
/// fits is returned whole.
pub fn fit_notation(summary: &str, width: usize, damaged: bool) -> String {
    if summary.chars().count() <= width {
        return summary.to_string();
    }
    let marker = if damaged { "+!" } else { "+" };
    let mut kept = Vec::new();
    let mut used = 0;
    for token in summary.split_whitespace() {
        let separator = usize::from(!kept.is_empty());
        let needed = used + separator + token.chars().count() + 1 + marker.len();
        if needed > width {
            break;
        }
        used += separator + token.chars().count();
        kept.push(token);
    }
    let cut = if kept.is_empty() {
        marker.to_string()
    } else {
        format!("{} {marker}", kept.join(" "))
    };
    // Whatever else is cut, the damage mark stays.
    match cut.chars().count() > width {
        false => cut,
        true if damaged => "!".repeat(width.min(1)),
        true => cut.chars().take(width).collect(),
    }
}
