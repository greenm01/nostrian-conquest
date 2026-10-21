//! AI personalities (t167): named executor styles, authored as typed KDL in
//! `nc-ai/personalities/`.
//!
//! A personality is a whole profile: every [`Weights`] field and every
//! [`Doctrine`] switch, each set exactly once, so no knob is left at a
//! default. Its `bounds` narrow [`Weights::RANGES`] for the weights that
//! make the style, so tuning cannot drift a rusher into a land grab.
//!
//! ```kdl
//! personality "rusher"
//! weights { tax 59.8; home-armies 19.6; /* ... every weight */ }
//! doctrine { land-grab #true; /* ... every switch */ }
//! bounds { war-year 0.0 3.0 }
//! goals { allocation 1 2 }
//! ```
//!
//! The optional `goals` block bounds the goals a learned seat of the style
//! may set, so its network cannot undo the personality: the strategist
//! decides when to fight, the personality biases how.

use kdl::{KdlDocument, KdlNode, KdlValue};

use crate::executor::{Doctrine, Weights};
use crate::goals::{ALLOCATIONS, Goals};

/// Weight names in [`Weights::to_array`] order.
pub const WEIGHT_NAMES: [&str; Weights::COUNT] = [
    "tax",
    "home-armies",
    "colony-armies",
    "batteries",
    "etacs",
    "scouts",
    "starbases",
    "transports",
    "battleship",
    "cruiser",
    "destroyer",
    "attack-ratio",
    "roe",
    "war-year",
];

const DOCTRINE_NAMES: [&str; 15] = [
    "land-grab",
    "strike-weakest",
    "suppress-first",
    "scaled-garrisons",
    "blockader",
    "allocate",
    "supply",
    "defend",
    "mixed-hulls",
    "keep-scouts",
    "watch",
    "forward-muster",
    "pickets",
    "ferry",
    "blitz",
];

/// The built-in personalities, the default first.
pub const PERSONALITIES: [&str; 5] = ["land-grab", "rusher", "turtle", "armada", "raider"];

const SOURCES: [&str; 5] = [
    include_str!("../personalities/land-grab.kdl"),
    include_str!("../personalities/rusher.kdl"),
    include_str!("../personalities/turtle.kdl"),
    include_str!("../personalities/armada.kdl"),
    include_str!("../personalities/raider.kdl"),
];

/// A named style: its weights and the ranges tuning keeps it within.
#[derive(Debug, Clone, PartialEq)]
pub struct Personality {
    pub name: String,
    pub weights: Weights,
    /// [`Weights::RANGES`], narrowed by the profile's bounds.
    pub ranges: [(f32, f32); Weights::COUNT],
    /// The fleet allocation goals (t175) the style allows, lowest and
    /// highest.
    pub allocation: (usize, usize),
}

impl Personality {
    /// `goals` within the style's goal bounds.
    pub fn bound_goals(&self, goals: &mut Goals) {
        goals.allocation = goals.allocation.clamp(self.allocation.0, self.allocation.1);
    }

    /// Weights from an array in field order, clamped to this style's
    /// ranges, with its doctrine.
    pub fn weights_from_array(&self, values: [f32; Weights::COUNT]) -> Weights {
        let clamped =
            std::array::from_fn(|idx| values[idx].clamp(self.ranges[idx].0, self.ranges[idx].1));
        Weights {
            doctrine: self.weights.doctrine,
            ..Weights::from_array(clamped)
        }
    }
}

/// The built-in personality called `name`.
pub fn personality(name: &str) -> Option<Personality> {
    PERSONALITIES
        .iter()
        .position(|known| *known == name)
        .map(|idx| parse_personality(SOURCES[idx]).expect("built-in personalities parse"))
}

/// A profile in KDL for `weights` under `name`, which [`parse_personality`]
/// reads back (no `bounds` or `goals`).
pub fn personality_kdl(name: &str, weights: &Weights) -> String {
    let d = weights.doctrine;
    let switches = [
        d.land_grab,
        d.strike_weakest,
        d.suppress_first,
        d.scaled_garrisons,
        d.blockader,
        d.allocate,
        d.supply,
        d.defend,
        d.mixed_hulls,
        d.keep_scouts,
        d.watch,
        d.forward_muster,
        d.pickets,
        d.ferry,
        d.blitz,
    ];
    let mut text = format!("personality \"{name}\"\nweights {{\n");
    for (weight, value) in WEIGHT_NAMES.iter().zip(weights.to_array()) {
        text.push_str(&format!("    {weight} {value:?}\n"));
    }
    text.push_str("}\ndoctrine {\n");
    for (switch, value) in DOCTRINE_NAMES.iter().zip(switches) {
        text.push_str(&format!("    {switch} #{value}\n"));
    }
    text.push_str("}\n");
    text
}

/// The built-in personality called `spec`, or the profile in the KDL file
/// `spec` names (a path ending in `.kdl`, as the tuner's `--out` writes).
pub fn personality_or_file(spec: &str) -> Result<Personality, String> {
    if spec.ends_with(".kdl") {
        let text = std::fs::read_to_string(spec).map_err(|err| format!("{spec}: {err}"))?;
        return parse_personality(&text);
    }
    personality(spec).ok_or(format!("unknown personality {spec}"))
}

/// Parse a personality profile.
pub fn parse_personality(text: &str) -> Result<Personality, String> {
    let document: KdlDocument = text.parse().map_err(|err| format!("invalid KDL: {err}"))?;
    let node = |name: &str| -> Result<&KdlNode, String> {
        let mut found = document
            .nodes()
            .iter()
            .filter(|node| node.name().value() == name);
        match (found.next(), found.next()) {
            (Some(node), None) => Ok(node),
            (None, _) => Err(format!("missing {name}")),
            _ => Err(format!("{name} appears twice")),
        }
    };
    for top in document.nodes() {
        let name = top.name().value();
        if !["personality", "weights", "doctrine", "bounds", "goals"].contains(&name) {
            return Err(format!("unknown node {name}"));
        }
    }
    let name = node("personality")?
        .entries()
        .first()
        .and_then(|entry| entry.value().as_string())
        .ok_or("personality needs a name")?
        .to_string();
    let weights: [f32; Weights::COUNT] =
        settings(node("weights")?, &WEIGHT_NAMES, |args| match args {
            [value] => number(value),
            _ => None,
        })?;
    let [
        land_grab,
        strike_weakest,
        suppress_first,
        scaled_garrisons,
        blockader,
        allocate,
        supply,
        defend,
        mixed_hulls,
        keep_scouts,
        watch,
        forward_muster,
        pickets,
        ferry,
        blitz,
    ] = settings(node("doctrine")?, &DOCTRINE_NAMES, |args| match args {
        [value] => value.as_bool(),
        _ => None,
    })?;
    let mut ranges = Weights::RANGES;
    if let Ok(bounds) = node("bounds") {
        for child in children(bounds)? {
            let name = child.name().value();
            let idx = WEIGHT_NAMES
                .iter()
                .position(|known| *known == name)
                .ok_or(format!("bounds: unknown weight {name}"))?;
            let args: Vec<&KdlValue> = child.entries().iter().map(|e| e.value()).collect();
            let (Some(lo), Some(hi)) = (
                args.first().and_then(|v| number(v)),
                args.get(1).and_then(|v| number(v)),
            ) else {
                return Err(format!("bounds: {name} needs a low and a high number"));
            };
            let (min, max) = Weights::RANGES[idx];
            if args.len() != 2 || lo > hi || lo < min || hi > max {
                return Err(format!("bounds: {name} must lie within {min} to {max}"));
            }
            ranges[idx] = (lo, hi);
        }
    }
    for (idx, value) in weights.iter().enumerate() {
        let (lo, hi) = ranges[idx];
        if !(lo..=hi).contains(value) {
            return Err(format!(
                "{} {value} lies outside {lo} to {hi}",
                WEIGHT_NAMES[idx]
            ));
        }
    }
    let mut allocation = (0, ALLOCATIONS - 1);
    if let Ok(goals) = node("goals") {
        for child in children(goals)? {
            let args: Vec<i128> = child
                .entries()
                .iter()
                .filter_map(|entry| entry.value().as_integer())
                .collect();
            match (child.name().value(), args.as_slice()) {
                ("allocation", [lo, hi]) if 0 <= *lo && lo <= hi && *hi < ALLOCATIONS as i128 => {
                    allocation = (*lo as usize, *hi as usize);
                }
                (name, _) => return Err(format!("goals: bad setting {name}")),
            }
        }
    }
    Ok(Personality {
        name,
        allocation,
        weights: Weights {
            doctrine: Doctrine {
                land_grab,
                strike_weakest,
                suppress_first,
                scaled_garrisons,
                blockader,
                allocate,
                supply,
                defend,
                mixed_hulls,
                keep_scouts,
                watch,
                forward_muster,
                pickets,
                ferry,
                blitz,
                reinforce: false,
            },
            ..Weights::from_array(weights)
        },
        ranges,
    })
}

fn children(node: &KdlNode) -> Result<&[KdlNode], String> {
    node.children()
        .map(KdlDocument::nodes)
        .ok_or(format!("{} needs a block", node.name().value()))
}

/// One value per name, each named child set exactly once.
fn settings<T: Copy + Default, const N: usize>(
    node: &KdlNode,
    names: &[&str; N],
    value: impl Fn(&[&KdlValue]) -> Option<T>,
) -> Result<[T; N], String> {
    let block = node.name().value();
    let mut values: [Option<T>; N] = [None; N];
    for child in children(node)? {
        let name = child.name().value();
        let idx = names
            .iter()
            .position(|known| *known == name)
            .ok_or(format!("{block}: unknown setting {name}"))?;
        if values[idx].is_some() {
            return Err(format!("{block}: {name} is set twice"));
        }
        let args: Vec<&KdlValue> = child.entries().iter().map(|e| e.value()).collect();
        values[idx] = Some(value(&args).ok_or(format!("{block}: {name} has a bad value"))?);
    }
    let mut out = [T::default(); N];
    for (idx, slot) in values.iter().enumerate() {
        out[idx] = slot.ok_or(format!("{block}: {} is not set", names[idx]))?;
    }
    Ok(out)
}

fn number(value: &KdlValue) -> Option<f32> {
    value
        .as_float()
        .map(|v| v as f32)
        .or_else(|| value.as_integer().map(|v| v as f32))
}
