use std::collections::BTreeMap;

use super::fleet::empire_bit;
use super::ids::Coords;

/// What one empire has sighted in one sector, noted in the depth layer where
/// it saw them: the set of opponent empires. The empire knows them in the
/// whole sector while it has a fleet or starbase there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sighting {
    pub viewer: u8,
    pub sector: Coords,
    /// 1 inside the world's system, 0 in deep space.
    pub layer: u8,
    /// Bit `empire - 1` is set for each opponent the viewer has sighted or
    /// fought here.
    pub opponents: u32,
}

/// Local sighting memory (combat decision 6, surprise). Each viewing empire
/// keeps, per sector, the opponent empires it has sighted there. An actual
/// sighting by any of the viewer's units, or a combat, adds the opponent;
/// the set is shared by all of the viewer's fleets and starbases in the
/// sector, in deep space and inside the system alike (owner, 2026-10-30).
/// Entries stay keyed by the layer where they were noted, as stored. It
/// persists across years while the viewer has a unit in the sector, and is
/// cleared when that presence ends. Not a classic field: an export drops it
/// and an import starts with none.
///
/// It also keeps each empire's enemy contacts for routing: the sectors where
/// it has sighted or fought another empire's units, and the year it last
/// did. Unlike the surprise memory, a contact outlives the viewer's presence
/// but expires after [`CONTACT_MEMORY_YEARS`]: ships move on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SightingMemory {
    /// By viewer, sector x, sector y, layer; an entry is never empty.
    entries: BTreeMap<(u8, u8, u8, u8), u32>,
    /// By viewer, sector x, sector y: the year of the latest contact.
    contacts: BTreeMap<(u8, u8, u8), u16>,
}

/// How many years of movement a contact steers routes around.
pub const CONTACT_MEMORY_YEARS: u16 = 2;

/// One empire's latest enemy contact in one sector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact {
    pub viewer: u8,
    pub sector: Coords,
    pub year: u16,
}

impl SightingMemory {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many (viewer, sector, layer) places hold a memory.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether `viewer` has sighted `opponent` anywhere in `sector`.
    pub fn knows(&self, viewer: u8, sector: Coords, opponent: u8) -> bool {
        empire_bit(opponent).is_some_and(|bit| {
            [0, 1].into_iter().any(|layer| {
                self.entries
                    .get(&key(viewer, sector, layer))
                    .is_some_and(|opponents| opponents & bit != 0)
            })
        })
    }

    /// Whether `viewer` holds any memory in `sector`.
    pub fn holds(&self, viewer: u8, sector: Coords) -> bool {
        [0, 1]
            .into_iter()
            .any(|layer| self.entries.contains_key(&key(viewer, sector, layer)))
    }

    /// `viewer` has sighted `opponent` in `sector` and `layer`.
    pub fn note(&mut self, viewer: u8, sector: Coords, layer: u8, opponent: u8) {
        if let Some(bit) = empire_bit(opponent) {
            *self.entries.entry(key(viewer, sector, layer)).or_default() |= bit;
        }
    }

    /// `viewer` has no unit in `sector` any more.
    pub fn forget(&mut self, viewer: u8, sector: Coords) {
        for layer in [0, 1] {
            self.entries.remove(&key(viewer, sector, layer));
        }
    }

    /// Keep the memories for which `present(viewer, sector)` holds.
    pub fn retain(&mut self, mut present: impl FnMut(u8, Coords) -> bool) {
        self.entries
            .retain(|&(viewer, x, y, _), _| present(viewer, Coords::new(x, y)));
    }

    /// Every memory, by viewer, then sector, then layer.
    pub fn iter(&self) -> impl Iterator<Item = Sighting> + '_ {
        self.entries
            .iter()
            .map(|(&(viewer, x, y, layer), &opponents)| Sighting {
                viewer,
                sector: Coords::new(x, y),
                layer,
                opponents,
            })
    }

    /// `viewer` met an enemy unit in `sector` in `year`.
    pub fn note_contact(&mut self, viewer: u8, sector: Coords, year: u16) {
        let latest = self
            .contacts
            .entry((viewer, sector.x, sector.y))
            .or_default();
        *latest = (*latest).max(year);
    }

    /// The sectors where `viewer` met an enemy recently enough, in `year`,
    /// to route around them.
    pub fn recent_contacts(&self, viewer: u8, year: u16) -> impl Iterator<Item = Coords> + '_ {
        self.contacts
            .range((viewer, 0, 0)..=(viewer, u8::MAX, u8::MAX))
            .filter(move |(_, seen)| year.saturating_sub(**seen) < CONTACT_MEMORY_YEARS)
            .map(|(&(_, x, y), _)| Coords::new(x, y))
    }

    /// Drop the contacts too old, in `year`, to route around.
    pub fn expire_contacts(&mut self, year: u16) {
        self.contacts
            .retain(|_, seen| year.saturating_sub(*seen) < CONTACT_MEMORY_YEARS);
    }

    /// Every contact, by viewer, then sector.
    pub fn contacts(&self) -> impl Iterator<Item = Contact> + '_ {
        self.contacts
            .iter()
            .map(|(&(viewer, x, y), &year)| Contact {
                viewer,
                sector: Coords::new(x, y),
                year,
            })
    }

    /// Set one place's memory whole, as the store loads it; no opponents
    /// clears it.
    pub fn set(&mut self, sighting: Sighting) {
        let Sighting {
            viewer,
            sector,
            layer,
            opponents,
        } = sighting;
        if opponents == 0 {
            self.entries.remove(&key(viewer, sector, layer));
        } else {
            self.entries.insert(key(viewer, sector, layer), opponents);
        }
    }
}

fn key(viewer: u8, sector: Coords, layer: u8) -> (u8, u8, u8, u8) {
    (viewer, sector.x, sector.y, layer)
}
