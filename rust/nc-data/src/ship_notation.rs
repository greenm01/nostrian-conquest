//! The compact ship notation (`docs/dev/ship-notation.md`): `CODEtotal!crippled`
//! tokens, separated by single spaces. Every fleet composition on every
//! screen comes from here, so the door, the helm, and the reports cannot
//! drift into different dialects.
//!
//! The total always includes the crippled ships, and `!crippled` is left
//! off when it is zero. `TT` is an empty transport and `TT*` a loaded one,
//! two categories that each get their own token.

use crate::{AutoCommissionFleetEntry, Fleet, HullClass};

/// The code a starbase count is written with. It follows the ship classes.
pub const STARBASE_CODE: &str = "SB";

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
            Self::FullTransports => HullClass::LoadedTransport,
            Self::EmptyTransports => HullClass::EmptyTransport,
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
/// Every category has its own counter, loaded and empty transports each
/// theirs, so this is the record's state, capped at the ships the category
/// has. Every screen, every selection, and every preview reads it, so what a
/// player sees is what a detach or transfer takes.
pub fn crippled_by_category(fleet: &Fleet) -> [u32; ShipCategory::COUNT] {
    let mut crippled = [0; ShipCategory::COUNT];
    for category in ShipCategory::ALL {
        let hulls = fleet.crippled.get(category.hull_class()).total();
        crippled[category.index()] = hulls.min(category.ships_in(fleet));
    }
    crippled
}

/// Ships and starbases to be written in the notation. Every compact
/// composition on every screen is filled in here and written by
/// [`Composition::notation`], so the one class order (`SC BB CA DD TT* TT
/// ETAC SB`) lives in this file alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Composition {
    /// Per category, indexed like [`ShipCategory::ALL`]: the ships and how
    /// many of them are crippled (included in the total).
    pub ships: [(u32, u32); ShipCategory::COUNT],
    /// The starbases and how many of them are crippled.
    pub starbases: (u32, u32),
}

impl Composition {
    /// The ships of a fleet, with the crippled split of
    /// [`crippled_by_category`].
    pub fn of_fleet(fleet: &Fleet) -> Self {
        let crippled = crippled_by_category(fleet);
        let mut composition = Self::default();
        for category in ShipCategory::ALL {
            composition.ships[category.index()] =
                (category.ships_in(fleet), crippled[category.index()]);
        }
        composition
    }

    /// Undamaged ships of the given categories.
    pub fn undamaged(counts: impl IntoIterator<Item = (ShipCategory, u32)>) -> Self {
        let mut composition = Self::default();
        for (category, total) in counts {
            composition.ships[category.index()] = (total, 0);
        }
        composition
    }

    /// The same composition with undamaged starbases.
    pub fn with_starbases(mut self, total: u32) -> Self {
        self.starbases = (total, 0);
        self
    }

    /// The tokens in the one class order, one space apart. Empty when there
    /// is nothing to show.
    pub fn notation(&self) -> String {
        join_tokens(
            ShipCategory::ALL
                .into_iter()
                .map(|category| {
                    let (total, crippled) = self.ships[category.index()];
                    ship_token(category.code(), total, crippled)
                })
                .chain([ship_token(
                    STARBASE_CODE,
                    self.starbases.0,
                    self.starbases.1,
                )]),
        )
    }
}

/// The fleet's ships in notation: `SC1 BB2!1 CA12!3 DD8 TT*4!1 TT5 ETAC2`.
/// Empty when the fleet has no ships.
pub fn fleet_notation(fleet: &Fleet) -> String {
    Composition::of_fleet(fleet).notation()
}

/// The new ships of an auto-commission entry in notation, as the commission
/// notice lists them (`CA2 DD4`). New ships are undamaged and their
/// transports empty.
pub fn auto_commission_notation(entry: &AutoCommissionFleetEntry) -> String {
    Composition::undamaged([
        (ShipCategory::Scouts, entry.scouts),
        (ShipCategory::Battleships, entry.battleships),
        (ShipCategory::Cruisers, entry.cruisers),
        (ShipCategory::Destroyers, entry.destroyers),
        (ShipCategory::EmptyTransports, entry.transports),
        (ShipCategory::Etacs, entry.etacs),
    ])
    .notation()
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
