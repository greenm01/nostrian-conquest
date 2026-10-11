use crate::Planet;

pub fn yearly_tax_revenue(present_production: u16, empire_tax_rate: u8) -> u32 {
    (u32::from(present_production) * u32::from(empire_tax_rate)) / 100
}

pub fn yearly_growth_delta(
    present_production: u16,
    potential_production: u16,
    empire_tax_rate: u8,
    has_friendly_starbase: bool,
) -> u16 {
    if present_production >= potential_production {
        return 0;
    }

    let gap = potential_production - present_production;
    let tax_headroom = 100u16.saturating_sub(u16::from(empire_tax_rate.min(95)));
    let base_growth = ((u32::from(gap) * u32::from(tax_headroom)) + 399) / 400;
    let mut growth = base_growth;
    if has_friendly_starbase {
        let bonus_percent = starbase_growth_bonus_percent(empire_tax_rate);
        if bonus_percent > 0 {
            growth += (base_growth * u32::from(bonus_percent)).div_ceil(100);
        }
    }
    growth.max(1).min(u32::from(gap)) as u16
}

pub fn starbase_growth_bonus_percent(empire_tax_rate: u8) -> u16 {
    if empire_tax_rate <= 50 {
        50
    } else if empire_tax_rate >= 65 {
        0
    } else {
        (u16::from(65 - empire_tax_rate) * 50) / 15
    }
}

pub fn yearly_high_tax_penalty(present_production: u16, empire_tax_rate: u8) -> u16 {
    if present_production == 0 {
        return 0;
    }

    if empire_tax_rate <= 65 {
        return 0;
    }

    let overtax = u16::from(empire_tax_rate - 65);
    let penalty = ((u32::from(present_production) * u32::from(overtax)) + 499) / 500;
    penalty.max(1).min(u32::from(present_production)) as u16
}

pub fn build_capacity(present_production: u16, has_friendly_starbase: bool) -> u16 {
    if has_friendly_starbase {
        present_production.saturating_mul(5)
    } else {
        present_production
    }
}

/// Present production rounded to whole points.
pub fn present_production_points(planet: &Planet) -> u16 {
    planet
        .production
        .to_f64()
        .round()
        .clamp(0.0, f64::from(u16::MAX)) as u16
}

/// Stored goods rounded to whole points.
pub fn stored_production_points(planet: &Planet) -> u32 {
    planet
        .stored_goods
        .to_f64()
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}
