//! Hegel driver: Hegel's test case is the choice source, so Hegel generates
//! and shrinks the decisions every fuzz game makes.
//!
//! `NC_FUZZ_CASES` sets the number of games (default 30) and `NC_FUZZ_TURNS`
//! the most turns per game. The cases are the same on every run, so a
//! failure is the code's, not the draw's: `HEGEL_SEED=<n>` plays another
//! fixed batch and `HEGEL_DERANDOMIZE=false` explores at random. Failing
//! examples are kept in Hegel's database under the cargo target directory
//! and replayed first on the next run.

use hegel::generators as gs;
use hegel::{HealthCheck, Settings, TestCase};
use nc_fuzz::{ChoiceSource, case_count_from_env, check_case};

const DEFAULT_CASES: u32 = 30;

struct HegelChoices<'a>(&'a TestCase);

impl ChoiceSource for HegelChoices<'_> {
    fn pick(&mut self, lo: u32, hi: u32) -> u32 {
        self.0
            .draw_silent(gs::integers::<u32>().min_value(lo).max_value(hi))
    }

    fn flip(&mut self) -> bool {
        self.0.draw_silent(gs::booleans())
    }

    fn note(&mut self, line: &str) {
        self.0.note(line);
    }
}

fn settings() -> Settings {
    let database = format!("{}/hegel", env!("CARGO_TARGET_TMPDIR"));
    let settings = Settings::new()
        .test_cases(u64::from(case_count_from_env(DEFAULT_CASES)))
        .database(Some(database))
        .suppress_health_check([HealthCheck::TooSlow]);
    // A fixed seed from the environment already wins over this.
    if std::env::var_os("HEGEL_DERANDOMIZE").is_some() {
        settings
    } else {
        settings.derandomize(true)
    }
}

#[hegel::test(settings())]
fn engine_invariants_hold_under_hegel(tc: TestCase) {
    check_case(&mut HegelChoices(&tc), "hegel");
}
