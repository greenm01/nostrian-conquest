//! proptest driver: a generated list of choice numbers is the choice source,
//! so proptest shrinks the list and the game re-plays it.
//!
//! `NC_FUZZ_CASES` sets the number of games (default 30) and `NC_FUZZ_TURNS`
//! the most turns per game. Failing lists persist in
//! `proptest_driver.proptest-regressions` next to this file.

use nc_fuzz::{ChoiceTape, case_count_from_env, check_case};
use proptest::collection::vec;
use proptest::prelude::*;

const DEFAULT_CASES: u32 = 30;
/// Enough choices for a full game; an exhausted list answers with the
/// simplest choice.
const MAX_CHOICES: usize = 4096;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: case_count_from_env(DEFAULT_CASES),
        ..ProptestConfig::default()
    })]

    #[test]
    fn engine_invariants_hold_under_proptest(choices in vec(any::<u32>(), 0..MAX_CHOICES)) {
        check_case(&mut ChoiceTape::new(choices), "proptest");
    }
}
