//! A long run of the campaign store under hosted load (t205): player
//! processes change a 25-empire campaign while a reader loads it, maintenance
//! closes a year every few minutes and backups are taken.
//!
//! ```text
//! cargo build --release -p nc-data --example stress_writer --example soak
//! target/release/examples/soak DIR [--minutes 60] [--writers 8]
//! ```
//!
//! It prints each writer's change latencies, the reader's load latencies,
//! each maintenance and backup (`VACUUM INTO`, a consistent copy of the live file), the largest WAL file seen and the database
//! size, and fails if a writer fails, a backup or the final file is not
//! intact, or a fleet chain is broken.

#[allow(dead_code)]
#[path = "../tests/random_play/mod.rs"]
mod random_play;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use nc_data::CampaignStore;

const PLAYERS: u8 = 25;

fn arg(args: &[String], name: &str, default: u64) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(match args.iter().position(|a| a == name) {
        Some(at) => args.get(at + 1).ok_or("missing value")?.parse()?,
        None => default,
    })
}

fn integrity(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open(path)?;
    let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if check != "ok" {
        return Err(format!("{}: {check}", path.display()).into());
    }
    Ok(())
}

fn percentile(sorted: &[Duration], q: f64) -> f64 {
    sorted
        .get(((sorted.len().max(1) - 1) as f64 * q) as usize)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = PathBuf::from(
        args.first()
            .ok_or("usage: soak DIR [--minutes M] [--writers N]")?,
    );
    let minutes = arg(&args, "--minutes", 60)?;
    let writers = arg(&args, "--writers", 8)? as u8;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let store = CampaignStore::open_default_in_dir(&dir)?;
    random_play::seed_campaign(&store, PLAYERS);
    let path = store.path().to_path_buf();
    let wal = path.with_extension("db-wal");
    let writer_exe = std::env::current_exe()?
        .with_file_name(format!("stress_writer{}", std::env::consts::EXE_SUFFIX));
    let seconds = minutes * 60;
    let started = Instant::now();
    println!(
        "soak: {PLAYERS} empires, {writers} writers, {minutes} min, in {}",
        dir.display()
    );

    let children: Vec<_> = (1..=writers)
        .map(|empire| {
            std::process::Command::new(&writer_exe)
                .arg(&dir)
                .arg(empire.to_string())
                .arg((u64::from(empire) * 104_729).to_string())
                .arg(dir.join(format!("log-{empire}")))
                .args(["--seconds", &seconds.to_string()])
                .spawn()
        })
        .collect::<Result<_, _>>()?;

    let stop = Arc::new(AtomicBool::new(false));
    let largest_wal = Arc::new(AtomicU64::new(0));
    let reader = {
        let (path, stop, largest_wal, wal) =
            (path.clone(), stop.clone(), largest_wal.clone(), wal.clone());
        std::thread::spawn(move || -> Vec<Duration> {
            let store = CampaignStore::open(&path).expect("reader");
            let mut loads = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                let at = Instant::now();
                store
                    .load_latest_runtime_state()
                    .expect("load")
                    .expect("a snapshot");
                loads.push(at.elapsed());
                if let Ok(meta) = std::fs::metadata(&wal) {
                    largest_wal.fetch_max(meta.len(), Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            loads
        })
    };

    let (maintain_every, backup_every) = (Duration::from_secs(180), Duration::from_secs(60));
    let (mut next_maintenance, mut next_backup) = (maintain_every, backup_every);
    let mut backups = 0;
    while started.elapsed() < Duration::from_secs(seconds) {
        std::thread::sleep(Duration::from_secs(1));
        if started.elapsed() >= next_maintenance {
            next_maintenance += maintain_every;
            let at = Instant::now();
            let run = store.begin_maintenance()?;
            let mut year = nc_engine::CampaignMaintenance::load(&store)?;
            year.run_turn()?;
            let to = year.game_data.conquest.game_year;
            year.save(run.store())?;
            drop(run);
            println!(
                "{:>6.0}s maintenance to {to} took {:.0} ms",
                started.elapsed().as_secs_f64(),
                at.elapsed().as_secs_f64() * 1000.0
            );
        }
        if started.elapsed() >= next_backup {
            next_backup += backup_every;
            let at = Instant::now();
            let target = dir.join(format!("backup-{backups}.db"));
            nc_data::sqlite::open(&path)?
                .execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
            integrity(&target)?;
            std::fs::remove_file(&target)?;
            backups += 1;
            println!(
                "{:>6.0}s backup {backups} took {:.0} ms",
                started.elapsed().as_secs_f64(),
                at.elapsed().as_secs_f64() * 1000.0
            );
        }
    }

    for mut child in children {
        if !child.wait()?.success() {
            return Err("a writer failed".into());
        }
    }
    stop.store(true, Ordering::Relaxed);
    let mut loads = reader.join().map_err(|_| "reader panicked")?;
    loads.sort();
    integrity(&path)?;
    let state = store.load_latest_runtime_state()?.ok_or("no campaign")?;
    let ids: BTreeSet<_> = state
        .game_data
        .fleets
        .iter()
        .map(|fleet| fleet.record.0)
        .collect();
    if ids.len() != state.game_data.fleets.len()
        || !state.game_data.unchained_base_errors().is_empty()
    {
        return Err("a fleet or base chain is broken".into());
    }
    println!(
        "reader: {} loads, p50 {:.1} ms, p99 {:.1} ms, max {:.1} ms",
        loads.len(),
        percentile(&loads, 0.5),
        percentile(&loads, 0.99),
        percentile(&loads, 1.0)
    );
    println!(
        "year {}, {} fleets; largest WAL {} KiB; database {} KiB; {backups} backups intact",
        state.game_year,
        state.game_data.fleets.len(),
        largest_wal.load(Ordering::Relaxed) / 1024,
        std::fs::metadata(&path)?.len() / 1024
    );
    Ok(())
}
