//! Each empire's starbases form a doubly linked list in BASES.DAT, sorted
//! by starbase id. PLAYER.DAT `0x44` names the first record and `0x46` the
//! last; each record links the next (`0x05`) and previous (`0x09`) record
//! by 1-based record number (`docs/dev/ec-starbase-spec.md`).

use super::*;

impl CoreGameData {
    /// The record indexes (0-based) of empire `empire`'s chain, head first,
    /// as ECMAINT's loader walks it (`2895:5d72`).
    pub fn base_chain(&self, empire: u8) -> Result<Vec<usize>, String> {
        let Some(player) = self.players.get(usize::from(empire).wrapping_sub(1)) else {
            return Ok(Vec::new());
        };
        let mut chain = Vec::new();
        let mut link = player.bases_first.0;
        while link != 0 {
            let idx = usize::from(link) - 1;
            let Some(base) = self.bases.get(idx) else {
                return Err(format!(
                    "PLAYER[{empire}] base chain names BASES record {link}, past the end"
                ));
            };
            if chain.contains(&idx) {
                return Err(format!(
                    "PLAYER[{empire}] base chain loops back to BASES record {link}"
                ));
            }
            if base.owner.0 != empire {
                return Err(format!(
                    "BASES[{link}] in PLAYER[{empire}]'s chain is owned by empire {}",
                    base.owner.0
                ));
            }
            chain.push(idx);
            link = base.next.0;
        }
        Ok(chain)
    }

    /// Chains ECMAINT cannot load: a link past the end of BASES.DAT, a
    /// loop, or a record of another owner.
    pub fn base_chain_errors(&self) -> Vec<String> {
        (1..=self.players.len())
            .filter_map(|empire| self.base_chain(empire as u8).err())
            .collect()
    }

    /// Live bases that no chain reaches; ECMAINT would never see them.
    pub fn unchained_base_errors(&self) -> Vec<String> {
        let mut chained = Vec::new();
        for empire in 1..=self.players.len() {
            chained.extend(self.base_chain(empire as u8).unwrap_or_default());
        }
        self.bases
            .iter()
            .enumerate()
            .filter(|(idx, base)| base.base_id.0 != 0 && !chained.contains(idx))
            .map(|(idx, base)| {
                format!(
                    "BASES[{}] (empire {}, Starbase {}) is not in its owner's chain",
                    idx + 1,
                    base.owner.0,
                    base.base_id.0
                )
            })
            .collect()
    }

    /// Link record `record_idx` into its owner's chain under the lowest
    /// unused starbase id, keeping the chain sorted by id (ECMAINT
    /// `2000:6493`). Returns the id.
    pub fn link_new_base(&mut self, record_idx: usize) -> u16 {
        let empire = self.bases[record_idx].owner.0;
        let chain = self.base_chain(empire).unwrap_or_default();
        let mut base_id = 1;
        let mut next = None;
        for &idx in &chain {
            if self.bases[idx].base_id.0 > base_id {
                next = Some(idx);
                break;
            }
            base_id += 1;
        }
        let previous = match next {
            Some(next) => chain
                .iter()
                .position(|&idx| idx == next)
                .and_then(|at| at.checked_sub(1).map(|at| chain[at])),
            None => chain.last().copied(),
        };
        let record_number = |idx: usize| (idx + 1) as u16;
        let base = &mut self.bases[record_idx];
        base.base_id.0 = base_id;
        base.record.0 = record_number(record_idx);
        base.next.0 = next.map_or(0, record_number);
        base.prev.0 = previous.map_or(0, record_number);
        if let Some(next) = next {
            self.bases[next].prev.0 = record_number(record_idx);
        }
        if let Some(previous) = previous {
            self.bases[previous].next.0 = record_number(record_idx);
        }
        let player = &mut self.players[usize::from(empire) - 1];
        if previous.is_none() {
            player.bases_first.0 = record_number(record_idx);
        }
        if next.is_none() {
            player.bases_last.0 = record_number(record_idx);
        }
        base_id
    }

    /// Take record `record_idx` out of its owner's chain, joining its
    /// neighbours.
    pub fn unlink_base(&mut self, record_idx: usize) {
        let base = &self.bases[record_idx];
        let empire = base.owner.0;
        let next = base.next.0;
        let previous = base.prev.0;
        if let Some(next_base) = usize::from(next)
            .checked_sub(1)
            .and_then(|idx| self.bases.get_mut(idx))
        {
            next_base.prev.0 = previous;
        }
        if let Some(previous_base) = usize::from(previous)
            .checked_sub(1)
            .and_then(|idx| self.bases.get_mut(idx))
        {
            previous_base.next.0 = next;
        }
        let record_number = (record_idx + 1) as u16;
        let Some(player) = self.players.get_mut(usize::from(empire).wrapping_sub(1)) else {
            return;
        };
        if player.bases_first.0 == record_number {
            player.bases_first.0 = next;
        }
        if player.bases_last.0 == record_number {
            player.bases_last.0 = previous;
        }
        let base = &mut self.bases[record_idx];
        base.next.0 = 0;
        base.prev.0 = 0;
    }

    /// Rebuild empire `empire`'s chain from its live bases, sorted by id:
    /// the record links and PLAYER `0x44`. PLAYER `0x46` is left alone;
    /// ECMAINT accepts 0 there and sets it after a run.
    pub fn relink_base_chain(&mut self, empire: u8) {
        let mut owned: Vec<usize> = (0..self.bases.len())
            .filter(|&idx| {
                let base = &self.bases[idx];
                base.base_id.0 != 0 && base.owner.0 == empire
            })
            .collect();
        owned.sort_by_key(|&idx| self.bases[idx].base_id.0);
        let record_number = |idx: Option<&usize>| idx.map_or(0, |idx| (idx + 1) as u16);
        for (at, &idx) in owned.iter().enumerate() {
            let previous = at.checked_sub(1).and_then(|at| owned.get(at));
            let next = owned.get(at + 1);
            let base = &mut self.bases[idx];
            base.record.0 = (idx + 1) as u16;
            base.next.0 = record_number(next);
            base.prev.0 = record_number(previous);
        }
        if let Some(player) = self.players.get_mut(usize::from(empire).wrapping_sub(1)) {
            player.bases_first.0 = record_number(owned.first());
        }
    }

    /// Point each empire's PLAYER `0x46` at the tail of its chain, as
    /// ECMAINT leaves it after a run.
    pub fn sync_last_base_records(&mut self) {
        for empire in 1..=self.players.len() {
            let tail = self
                .base_chain(empire as u8)
                .ok()
                .and_then(|chain| chain.last().map(|idx| (idx + 1) as u16))
                .unwrap_or(0);
            self.players[empire - 1].bases_last.0 = tail;
        }
    }
}
