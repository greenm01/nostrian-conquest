//! Each empire's starbases form a doubly linked list in BASES.DAT, sorted
//! by starbase id. PLAYER.DAT `0x44` names the first record and `0x46` the
//! last; each record links the next (`0x05`) and previous (`0x09`) record
//! by 1-based record number (`docs/dev/ec-starbase-spec.md`).

use super::*;

impl CoreGameData {
    /// The record indexes (0-based) of empire `empire`'s chain, head first,
    /// as ECMAINT's loader walks it (`2895:5d72`).
    pub fn base_chain(&self, empire: u8) -> Result<Vec<usize>, String> {
        let Some(player) = self.player.records.get(usize::from(empire).wrapping_sub(1)) else {
            return Ok(Vec::new());
        };
        let mut chain = Vec::new();
        let mut link = player.first_base_record_raw();
        while link != 0 {
            let idx = usize::from(link) - 1;
            let Some(base) = self.bases.records.get(idx) else {
                return Err(format!(
                    "PLAYER[{empire}] base chain names BASES record {link}, past the end"
                ));
            };
            if chain.contains(&idx) {
                return Err(format!(
                    "PLAYER[{empire}] base chain loops back to BASES record {link}"
                ));
            }
            if base.owner_empire_raw() != empire {
                return Err(format!(
                    "BASES[{link}] in PLAYER[{empire}]'s chain is owned by empire {}",
                    base.owner_empire_raw()
                ));
            }
            chain.push(idx);
            link = base.next_base_link_raw();
        }
        Ok(chain)
    }

    /// Chains ECMAINT cannot load: a link past the end of BASES.DAT, a
    /// loop, or a record of another owner.
    pub fn base_chain_errors(&self) -> Vec<String> {
        (1..=self.player.records.len())
            .filter_map(|empire| self.base_chain(empire as u8).err())
            .collect()
    }

    /// Live bases that no chain reaches; ECMAINT would never see them.
    pub fn unchained_base_errors(&self) -> Vec<String> {
        let mut chained = Vec::new();
        for empire in 1..=self.player.records.len() {
            chained.extend(self.base_chain(empire as u8).unwrap_or_default());
        }
        self.bases
            .records
            .iter()
            .enumerate()
            .filter(|(idx, base)| base.base_id_raw() != 0 && !chained.contains(idx))
            .map(|(idx, base)| {
                format!(
                    "BASES[{}] (empire {}, Starbase {}) is not in its owner's chain",
                    idx + 1,
                    base.owner_empire_raw(),
                    base.base_id_raw()
                )
            })
            .collect()
    }

    /// Link record `record_idx` into its owner's chain under the lowest
    /// unused starbase id, keeping the chain sorted by id (ECMAINT
    /// `2000:6493`). Returns the id.
    pub fn link_new_base(&mut self, record_idx: usize) -> u16 {
        let empire = self.bases.records[record_idx].owner_empire_raw();
        let chain = self.base_chain(empire).unwrap_or_default();
        let mut base_id = 1;
        let mut next = None;
        for &idx in &chain {
            if self.bases.records[idx].base_id_raw() > base_id {
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
        let base = &mut self.bases.records[record_idx];
        base.set_base_id_raw(base_id);
        base.set_record_number_raw(record_number(record_idx));
        base.set_next_base_link_raw(next.map_or(0, record_number));
        base.set_previous_base_link_raw(previous.map_or(0, record_number));
        if let Some(next) = next {
            self.bases.records[next].set_previous_base_link_raw(record_number(record_idx));
        }
        if let Some(previous) = previous {
            self.bases.records[previous].set_next_base_link_raw(record_number(record_idx));
        }
        let player = &mut self.player.records[usize::from(empire) - 1];
        if previous.is_none() {
            player.set_first_base_record_raw(record_number(record_idx));
        }
        if next.is_none() {
            player.set_last_base_record_raw(record_number(record_idx));
        }
        base_id
    }

    /// Take record `record_idx` out of its owner's chain, joining its
    /// neighbours.
    pub fn unlink_base(&mut self, record_idx: usize) {
        let base = &self.bases.records[record_idx];
        let empire = base.owner_empire_raw();
        let next = base.next_base_link_raw();
        let previous = base.previous_base_link_raw();
        if let Some(next_base) = usize::from(next)
            .checked_sub(1)
            .and_then(|idx| self.bases.records.get_mut(idx))
        {
            next_base.set_previous_base_link_raw(previous);
        }
        if let Some(previous_base) = usize::from(previous)
            .checked_sub(1)
            .and_then(|idx| self.bases.records.get_mut(idx))
        {
            previous_base.set_next_base_link_raw(next);
        }
        let record_number = (record_idx + 1) as u16;
        let Some(player) = self
            .player
            .records
            .get_mut(usize::from(empire).wrapping_sub(1))
        else {
            return;
        };
        if player.first_base_record_raw() == record_number {
            player.set_first_base_record_raw(next);
        }
        if player.last_base_record_raw() == record_number {
            player.set_last_base_record_raw(previous);
        }
        let base = &mut self.bases.records[record_idx];
        base.set_next_base_link_raw(0);
        base.set_previous_base_link_raw(0);
    }

    /// Rebuild empire `empire`'s chain from its live bases, sorted by id:
    /// the record links and PLAYER `0x44`. PLAYER `0x46` is left alone;
    /// ECMAINT accepts 0 there and sets it after a run.
    pub fn relink_base_chain(&mut self, empire: u8) {
        let mut owned: Vec<usize> = (0..self.bases.records.len())
            .filter(|&idx| {
                let base = &self.bases.records[idx];
                base.base_id_raw() != 0 && base.owner_empire_raw() == empire
            })
            .collect();
        owned.sort_by_key(|&idx| self.bases.records[idx].base_id_raw());
        let record_number = |idx: Option<&usize>| idx.map_or(0, |idx| (idx + 1) as u16);
        for (at, &idx) in owned.iter().enumerate() {
            let previous = at.checked_sub(1).and_then(|at| owned.get(at));
            let next = owned.get(at + 1);
            let base = &mut self.bases.records[idx];
            base.set_record_number_raw((idx + 1) as u16);
            base.set_next_base_link_raw(record_number(next));
            base.set_previous_base_link_raw(record_number(previous));
        }
        if let Some(player) = self
            .player
            .records
            .get_mut(usize::from(empire).wrapping_sub(1))
        {
            player.set_first_base_record_raw(record_number(owned.first()));
        }
    }

    /// Point each empire's PLAYER `0x46` at the tail of its chain, as
    /// ECMAINT leaves it after a run.
    pub fn sync_last_base_records(&mut self) {
        for empire in 1..=self.player.records.len() {
            let tail = self
                .base_chain(empire as u8)
                .ok()
                .and_then(|chain| chain.last().map(|idx| (idx + 1) as u16))
                .unwrap_or(0);
            self.player.records[empire - 1].set_last_base_record_raw(tail);
        }
    }
}
