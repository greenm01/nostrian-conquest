//! proptest driver: a generated list of choice numbers is the choice source,
//! so proptest shrinks the list and the game re-plays it.
//!
//! `NC_FUZZ_CASES` sets the number of games (default 30) and `NC_FUZZ_TURNS`
//! the most turns per game. The cases are the same on every run;
//! `PROPTEST_RNG_SEED=<n>` plays another fixed batch. Failing lists persist
//! in `proptest_driver.proptest-regressions` next to this file.

use nc_fuzz::{ChoiceTape, case_count_from_env, check_case};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::RngSeed;

const DEFAULT_CASES: u32 = 30;
const DEFAULT_SEED: u64 = 0x4e43_4655_5a5a; // "NCFUZZ"
/// Enough choices for a full game; an exhausted list answers with the
/// simplest choice.
const MAX_CHOICES: usize = 4096;

fn config() -> ProptestConfig {
    let mut config = ProptestConfig {
        cases: case_count_from_env(DEFAULT_CASES),
        ..ProptestConfig::default()
    };
    if config.rng_seed == RngSeed::Random {
        config.rng_seed = RngSeed::Fixed(DEFAULT_SEED);
    }
    config
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn engine_invariants_hold_under_proptest(choices in vec(any::<u32>(), 0..MAX_CHOICES)) {
        check_case(&mut ChoiceTape::new(choices), "proptest");
    }
}
