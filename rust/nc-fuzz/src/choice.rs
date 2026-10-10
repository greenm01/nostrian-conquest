//! The only thing a fuzz driver implements: where decisions come from.
//!
//! Every decision a fuzz game makes is a small number or a yes/no answer,
//! resolved against the live state ("fleet 2 of 7"), so a fixed choice list
//! stays meaningful as the game changes. The lowest number and `false` are
//! always the simplest answer, so a shrinker that drives choices toward zero
//! drives the game toward doing less.

pub trait ChoiceSource {
    /// A number in `lo..=hi`; `lo` is the simplest answer.
    fn pick(&mut self, lo: u32, hi: u32) -> u32;

    /// A yes/no decision; `false` is the simplest answer.
    fn flip(&mut self) -> bool;

    /// One line of progress, printed with a failing case by drivers that can.
    fn note(&mut self, _line: &str) {}
}

/// Index of one item out of `len`, or `None` when there is nothing to pick.
pub fn pick_index(choices: &mut dyn ChoiceSource, len: usize) -> Option<usize> {
    let last = u32::try_from(len.checked_sub(1)?).unwrap_or(u32::MAX);
    Some(choices.pick(0, last) as usize)
}

/// One item out of a slice.
pub fn pick_from<T: Copy>(choices: &mut dyn ChoiceSource, items: &[T]) -> Option<T> {
    pick_index(choices, items.len()).map(|idx| items[idx])
}

/// A fixed list of choice numbers consumed in order. Values are folded into
/// the requested range, and an exhausted tape answers with the simplest
/// choice, so any list is a valid game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChoiceTape {
    values: Vec<u32>,
    next: usize,
}

impl ChoiceTape {
    pub fn new(values: Vec<u32>) -> Self {
        Self { values, next: 0 }
    }

    fn take(&mut self) -> u32 {
        let value = self.values.get(self.next).copied().unwrap_or(0);
        self.next += 1;
        value
    }
}

impl ChoiceSource for ChoiceTape {
    fn pick(&mut self, lo: u32, hi: u32) -> u32 {
        let value = self.take();
        let span = u64::from(hi.saturating_sub(lo)) + 1;
        lo + (u64::from(value) % span) as u32
    }

    fn flip(&mut self) -> bool {
        self.take() % 2 == 1
    }
}
