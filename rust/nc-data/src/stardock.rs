//! The stardock is 8 u32 counts at PLANETS.DAT `+0x38`, one per item type,
//! in this order: destroyer, cruiser, battleship, scout, troop transport,
//! ETAC, starbase, IPBM (`docs/dev/ec-battle-spec.md`).

use crate::{Planet, ProductionItemKind};

pub const STARDOCK_SLOT_COUNT: usize = 8;
/// The build kind each stardock slot holds. Rust builds no IPBMs, so that
/// slot has no kind.
const STARDOCK_SLOT_KINDS: [u8; STARDOCK_SLOT_COUNT] = [1, 2, 3, 4, 5, 6, 9, 0];

/// The stardock slot that holds items of build kind `kind_raw`, if any.
pub fn stardock_slot_for_kind(kind_raw: u8) -> Option<usize> {
    (kind_raw != 0)
        .then(|| {
            STARDOCK_SLOT_KINDS
                .iter()
                .position(|&kind| kind == kind_raw)
        })
        .flatten()
}

/// The kind of the items waiting in stardock slot `slot`, or
/// `ProductionItemKind::Unknown(0)` when the slot is empty.
pub fn stardock_slot_kind(planet: &Planet, slot: usize) -> ProductionItemKind {
    if planet.stardock[slot] == 0 {
        ProductionItemKind::Unknown(0)
    } else {
        ProductionItemKind::from_code(STARDOCK_SLOT_KINDS[slot])
    }
}
