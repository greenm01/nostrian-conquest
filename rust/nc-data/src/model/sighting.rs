use std::collections::BTreeMap;

use super::fleet::empire_bit;
use super::ids::Coords;

/// What one empire has sighted in one sector and depth layer: the set of
/// opponent empires, kept while the empire has a fleet or starbase there.
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
/// keeps, per sector and depth layer, the opponent empires it has sighted
/// there. An actual sighting by any of the viewer's units, or a combat,
/// adds the opponent; the set is shared by all of the viewer's fleets and
/// starbases in that sector and layer. It persists across years while the
/// viewer has a unit there, and is cleared when that presence ends. Not a
/// classic field: an export drops it and an import starts with none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SightingMemory {
    /// By viewer, sector x, sector y, layer; an entry is never empty.
    entries: BTreeMap<(u8, u8, u8, u8), u32>,
}

impl SightingMemory {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many (viewer, sector, layer) places hold a memory.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether `viewer` has sighted `opponent` in `sector` and `layer`.
    pub fn knows(&self, viewer: u8, sector: Coords, layer: u8, opponent: u8) -> bool {
        empire_bit(opponent).is_some_and(|bit| {
            self.entries
                .get(&key(viewer, sector, layer))
                .is_some_and(|opponents| opponents & bit != 0)
        })
    }

    /// Whether `viewer` holds any memory in `sector` and `layer`.
    pub fn holds(&self, viewer: u8, sector: Coords, layer: u8) -> bool {
        self.entries.contains_key(&key(viewer, sector, layer))
    }

    /// `viewer` has sighted `opponent` in `sector` and `layer`.
    pub fn note(&mut self, viewer: u8, sector: Coords, layer: u8, opponent: u8) {
        if let Some(bit) = empire_bit(opponent) {
            *self.entries.entry(key(viewer, sector, layer)).or_default() |= bit;
        }
    }

    /// `viewer` has no unit in `sector` and `layer` any more.
    pub fn forget(&mut self, viewer: u8, sector: Coords, layer: u8) {
        self.entries.remove(&key(viewer, sector, layer));
    }

    /// Keep the memories for which `present(viewer, sector, layer)` holds.
    pub fn retain(&mut self, mut present: impl FnMut(u8, Coords, u8) -> bool) {
        self.entries
            .retain(|&(viewer, x, y, layer), _| present(viewer, Coords::new(x, y), layer));
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
            self.forget(viewer, sector, layer);
        } else {
            self.entries.insert(key(viewer, sector, layer), opponents);
        }
    }
}

fn key(viewer: u8, sector: Coords, layer: u8) -> (u8, u8, u8, u8) {
    (viewer, sector.x, sector.y, layer)
}
