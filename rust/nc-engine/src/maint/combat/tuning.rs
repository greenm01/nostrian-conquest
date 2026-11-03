//! t255 experiment only (branch `t255-combat-search`, never merged): combat
//! settings a search harness can set once per process. The default is the
//! current rules, so a process that never sets them plays the real game.

use std::sync::OnceLock;

/// Hull indices DD, CA, BB, as in `state.rs`.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatTuning {
    /// Attack strength of DD, CA, BB.
    pub attack: [u32; 3],
    /// Defense strength of DD, CA, BB.
    pub defense: [u32; 3],
    /// The engagement count's exponent (`y = (A/B)^(1/exponent)`).
    pub engage_exponent: f64,
    /// Per-hull fire: each hull class spends its own share of the volley.
    pub per_hull: bool,
    /// Per-hull fire: the class each shooter (DD, CA, BB) aims at first;
    /// `None` keeps the line order (BB, CA, DD).
    pub first_target: [Option<usize>; 3],
    /// Per-hull fire: a shooter's hits against its first target count this
    /// many times (its DS costs are divided by it).
    pub bonus: [f64; 3],
}

impl CombatTuning {
    pub const CURRENT: Self = Self {
        attack: [12, 33, 90],
        defense: [7, 30, 110],
        engage_exponent: 3.3,
        per_hull: false,
        first_target: [None; 3],
        bonus: [1.0; 3],
    };

    /// Parse `key=value` pairs separated by commas: `as=12/33/90`,
    /// `ds=7/30/110`, `exp=3.3`, `per_hull=1`, `first=BB/DD/CA` (`-` for
    /// none), `bonus=1.5/1/1`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut tuning = Self::CURRENT;
        for pair in text.split(',').filter(|p| !p.is_empty()) {
            let (key, value) = pair.split_once('=').ok_or(format!("bad pair {pair}"))?;
            let parts: Vec<&str> = value.split('/').collect();
            let three = |parts: &[&str]| -> Result<[f64; 3], String> {
                if parts.len() != 3 {
                    return Err(format!("{key} needs three values"));
                }
                let mut out = [0.0; 3];
                for (slot, part) in out.iter_mut().zip(parts) {
                    *slot = part.parse().map_err(|_| format!("bad number {part}"))?;
                }
                Ok(out)
            };
            match key {
                "as" => tuning.attack = three(&parts)?.map(|v| v.round() as u32),
                "ds" => tuning.defense = three(&parts)?.map(|v| v.round() as u32),
                "exp" => tuning.engage_exponent = value.parse().map_err(|_| "bad exp")?,
                "per_hull" => tuning.per_hull = value == "1",
                "bonus" => tuning.bonus = three(&parts)?,
                "first" => {
                    if parts.len() != 3 {
                        return Err("first needs three values".into());
                    }
                    for (slot, part) in tuning.first_target.iter_mut().zip(&parts) {
                        *slot = match *part {
                            "DD" => Some(0),
                            "CA" => Some(1),
                            "BB" => Some(2),
                            "-" => None,
                            other => return Err(format!("bad hull {other}")),
                        };
                    }
                }
                other => return Err(format!("unknown key {other}")),
            }
        }
        Ok(tuning)
    }
}

static TUNING: OnceLock<CombatTuning> = OnceLock::new();

/// Set the process's combat settings; only the first call counts.
pub fn set_combat_tuning(tuning: CombatTuning) {
    let _ = TUNING.set(tuning);
}

pub(super) fn tuning() -> &'static CombatTuning {
    TUNING.get_or_init(|| CombatTuning::CURRENT)
}
