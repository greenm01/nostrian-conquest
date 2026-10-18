//! One player's process in the storage stress tests (t205): it makes random
//! changes of every kind to a campaign, as a door session does, and logs
//! each change it committed.
//!
//! ```text
//! cargo run -p nc-data --example stress_writer -- DIR EMPIRE SEED LOG [--from N] [--seconds S]
//! ```
//!
//! Every committed change also queues a mail with the subject
//! `stress EMPIRE:N`, numbered from `--from` (0) in commit order, so a test
//! can check the store holds exactly the changes the process committed.
//! The number is logged to LOG (one per line) after the commit; a process
//! killed between the two leaves one change stored but unlogged. With
//! `--seconds` it stops after that long and prints its change latencies;
//! without, it runs until killed. A change refused while maintenance blocks
//! players is retried; once maintenance closes the year, the process goes
//! on in the new one.

#[allow(dead_code)]
#[path = "../tests/random_play/mod.rs"]
mod random_play;

use std::io::Write;
use std::time::{Duration, Instant};

use nc_data::{CampaignStore, CampaignStoreError};
use random_play::{Rng, apply, mail};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, empire, seed, log, rest @ ..] = args.as_slice() else {
        return Err("usage: stress_writer DIR EMPIRE SEED LOG [--from N] [--seconds S]".into());
    };
    let flag = |name: &str| {
        rest.iter()
            .position(|arg| arg == name)
            .and_then(|at| rest.get(at + 1))
            .map(|value| value.parse::<u64>())
            .transpose()
    };
    let empire: u8 = empire.parse()?;
    let mut n = flag("--from")?.unwrap_or(0);
    let until = flag("--seconds")?.map(|s| Instant::now() + Duration::from_secs(s));
    let store = CampaignStore::open_default_in_dir(std::path::Path::new(dir))?;
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(log)?;
    let mut rng = Rng(seed.parse::<u64>()? | 1);
    let latest = store.load_latest_runtime_state()?.ok_or("no campaign")?;
    let (mut year, players) = (latest.game_year, latest.game_data.conquest.player_count);
    let mut latencies = Vec::new();
    while until.is_none_or(|until| Instant::now() < until) {
        let step = rng.step(Some(empire), players);
        let started = Instant::now();
        let changed = store.change(year, |state| {
            apply(step, state)?;
            let mut marker = mail(empire, empire % players + 1, format!("stress {empire}:{n}"));
            marker.year = state.game_data.conquest.game_year;
            state.queued_mail.push(marker);
            Ok::<_, Box<dyn std::error::Error>>(())
        });
        match changed {
            Ok(_) => {
                latencies.push(started.elapsed());
                writeln!(log, "{n}")?;
                log.flush()?;
                n += 1;
            }
            Err(err) => match err.downcast_ref::<CampaignStoreError>() {
                Some(CampaignStoreError::HostingPaused) => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Some(CampaignStoreError::StaleSnapshot { latest_year, .. }) => year = *latest_year,
                Some(other) => return Err(format!("empire {empire}: {other}").into()),
                // The rules refused the order: nothing was written.
                None => {}
            },
        }
    }
    latencies.sort();
    let at = |q: f64| latencies[((latencies.len() as f64 - 1.0) * q) as usize];
    if !latencies.is_empty() {
        println!(
            "empire {empire}: {} changes, p50 {:.1} ms, p99 {:.1} ms, max {:.1} ms",
            latencies.len(),
            at(0.5).as_secs_f64() * 1000.0,
            at(0.99).as_secs_f64() * 1000.0,
            at(1.0).as_secs_f64() * 1000.0
        );
    }
    Ok(())
}
