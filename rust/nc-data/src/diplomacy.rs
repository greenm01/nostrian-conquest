//! One empire's stance toward each other empire, PLAYER.DAT 0x54.

use crate::{DiplomaticRelation, Player};

fn diplomacy_slot(other_empire: u8) -> Option<usize> {
    (1..=25)
        .contains(&other_empire)
        .then(|| usize::from(other_empire) - 1)
}

/// The stored relation toward `other_empire` (1 to 25), or `None` for an
/// empire out of range or a code no classic program writes.
pub fn diplomatic_relation_toward(player: &Player, other_empire: u8) -> Option<DiplomaticRelation> {
    match player.diplomacy.get(diplomacy_slot(other_empire)?)? {
        DiplomaticRelation::Unknown(_) => None,
        relation => Some(*relation),
    }
}

/// The stored relation toward `other_empire` as it stands, unknown codes
/// included; `None` for an empire out of range.
pub fn stored_diplomatic_relation(player: &Player, other_empire: u8) -> Option<DiplomaticRelation> {
    player.diplomacy.get(diplomacy_slot(other_empire)?).copied()
}

/// Sets the relation toward `other_empire`; false for an empire out of
/// range.
pub fn set_diplomatic_relation_toward(
    player: &mut Player,
    other_empire: u8,
    relation: DiplomaticRelation,
) -> bool {
    match diplomacy_slot(other_empire) {
        Some(slot) => {
            player.diplomacy[slot] = relation;
            true
        }
        None => false,
    }
}
