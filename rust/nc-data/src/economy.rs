//! The yearly economy rules.
//!
//! Production and stored goods are Real48 values that carry fractions from
//! pass to pass; nothing in the yearly pass rounds them. Only what a player
//! reads or spends is a whole number: spending takes the whole points of the
//! non-negative part of the treasury, build capacity takes production
//! rounded to the nearest point, and the screens show next pass's growth and
//! revenue rounded to the nearest point.
//!
//! The arithmetic is `tools/economy-model/accounting_review.py`'s `smooth`
//! rule and `starbase_review.py`'s fade, in `f64`. The engine stores results
//! back into Real48 (about 12 significant digits), so a value agrees with the
//! models' exact fractions to roughly 1e-9.

use crate::Planet;

/// Real48 keeps 40 bits of mantissa; an exact whole number can come back a
/// hair below itself after a few operations. Flooring ignores that hair.
const WHOLE_POINT_GUARD: f64 = 1e-9;

/// The starbase's extra share of a year's growth: half again at taxes of 50%
/// or less, fading in a straight line to nothing at 65%.
pub fn starbase_growth_bonus(tax_rate: u8) -> f64 {
    if tax_rate <= 50 {
        0.5
    } else {
        f64::from(65u8.saturating_sub(tax_rate)) / 30.0
    }
}

/// This year's growth: the share of the gap to potential that the tax
/// leaves room for, with the starbase bonus when one counts. Not rounded and
/// with no yearly minimum.
pub fn yearly_growth(production: f64, potential: f64, tax_rate: u8, has_starbase: bool) -> f64 {
    let gap = (potential - production).max(0.0);
    let headroom = f64::from(100 - tax_rate.min(95));
    let mut growth = gap * headroom / 400.0;
    if has_starbase {
        growth *= 1.0 + starbase_growth_bonus(tax_rate);
    }
    growth.min(gap)
}

/// The high-tax penalty: above 65% a world loses `(tax - 65) / 500` of its
/// production a year.
pub fn yearly_high_tax_penalty(production: f64, tax_rate: u8) -> f64 {
    production * f64::from(tax_rate.saturating_sub(65)) / 500.0
}

/// Production after this year's growth and penalty, never past potential.
pub fn next_production(production: f64, potential: f64, tax_rate: u8, has_starbase: bool) -> f64 {
    let growth = yearly_growth(production, potential, tax_rate, has_starbase);
    let penalty = yearly_high_tax_penalty(production, tax_rate);
    (production + growth - penalty).clamp(0.0, potential.max(0.0))
}

/// Tax income at a production level, credited after the year's growth.
pub fn tax_income(production: f64, tax_rate: u8) -> f64 {
    production * f64::from(tax_rate) / 100.0
}

/// What the next yearly pass does to a world, from its state now and
/// assuming its tax rate, owner, and starbase stay as they are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NextPass {
    /// Production gained from growth, before any high-tax penalty.
    pub growth: f64,
    /// Tax revenue credited to the world's treasury.
    pub revenue: f64,
}

pub fn next_pass(planet: &Planet, tax_rate: u8, has_starbase: bool) -> NextPass {
    // A scorched world neither grows nor earns, and a captured one pays
    // nothing until its second pass under the new owner has gone by.
    if planet.scorch_earth || planet.years_under_owner < 2 {
        return NextPass {
            growth: 0.0,
            revenue: 0.0,
        };
    }
    let production = planet.production.to_f64();
    let potential = f64::from(planet.potential_production);
    NextPass {
        growth: yearly_growth(production, potential, tax_rate, has_starbase),
        revenue: tax_income(
            next_production(production, potential, tax_rate, has_starbase),
            tax_rate,
        ),
    }
}

/// A fractional amount as the whole number a screen shows: the nearest point.
pub fn whole_points(value: f64) -> u32 {
    value.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

pub fn build_capacity(present_production: u16, has_friendly_starbase: bool) -> u16 {
    if has_friendly_starbase {
        present_production.saturating_mul(5)
    } else {
        present_production
    }
}

/// Present production rounded to the nearest whole point.
pub fn present_production_points(planet: &Planet) -> u16 {
    planet
        .production
        .to_f64()
        .round()
        .clamp(0.0, f64::from(u16::MAX)) as u16
}

/// The whole points the treasury can spend: its non-negative part, floored.
/// A debt reads as 0 here, but the treasury keeps it and income pays it down.
pub fn stored_production_points(planet: &Planet) -> u32 {
    (planet.stored_goods.to_f64() + WHOLE_POINT_GUARD)
        .floor()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}
