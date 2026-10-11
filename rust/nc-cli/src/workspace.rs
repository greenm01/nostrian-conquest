use std::fs;
use std::path::Path;

use crate::support::paths::{
    display_repo_path, init_fixture_dir, post_maint_fixture_dir,
    pre_maint_replay_context_fixture_dir,
};
use nc_compat::{
    ensure_classic_auxiliary_files, import_directory_snapshot,
    regenerate_database_dat_from_directory,
};
use nc_data::CampaignStore;

pub(crate) const INIT_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "FLEETS.DAT",
    "IPBM.DAT",
    "MESSAGES.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "RESULTS.DAT",
    "SETUP.DAT",
    // Note: DATABASE.DAT is now generated, not copied
];

pub(crate) const CURRENT_KNOWN_CORE_FILES: &[&str] = &[
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "SETUP.DAT",
    "CONQUEST.DAT",
];

pub(crate) const PRE_MAINT_REPLAY_CONTEXT_FILES: &[&str] = &["CONQUEST.DAT", "DATABASE.DAT"];

const ORIGINAL_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "SETUP.DAT",
];

const CLASSIC_RUNTIME_SUPPORT_FILES: &[&str] = &[
    "ECGAME.EXE",
    "ECMAINT.EXE",
    "ECUTIL.EXE",
    "ECKEY1X.BIN",
    "ECPLAYER.DOC",
    "ECQSTART.DOC",
    "ECREADME.DOC",
    "WHATSNEW.DOC",
];

pub fn initialize_dir(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_top_level_files(source, target)?;

    let init_dir = init_fixture_dir();
    copy_named_files(&init_dir, target, INIT_FILES)?;

    // Generate DATABASE.DAT from PLANETS.DAT + CONQUEST.DAT
    generate_database_dat(target)?;

    // Ensure auxiliary files exist
    ensure_classic_auxiliary_files(target)?;
    refresh_runtime_store(target)?;

    println!("Initialized game directory: {}", target.display());
    println!("  source snapshot: {}", display_repo_path(source));
    println!("  init fixture set: {}", display_repo_path(&init_dir));
    println!("  overlaid files:");
    for name in INIT_FILES {
        println!("    {name}");
    }
    println!("  generated files:");
    println!("    DATABASE.DAT");

    Ok(())
}

pub fn copy_init_files(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(source, target, INIT_FILES)?;
    generate_database_dat(target)?;
    ensure_classic_auxiliary_files(target)?;
    refresh_runtime_store(target)
}

pub fn copy_current_known_core_files(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(source, target, CURRENT_KNOWN_CORE_FILES)
}

/// Copy the replay context's classic files over `target`. The store is not
/// refreshed: the context is year 3000 and the store already holds the later
/// year from the init files, which it reads in preference, so importing the
/// older year would only be refused.
pub fn copy_pre_maint_replay_context_files(
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(
        &pre_maint_replay_context_fixture_dir(),
        target,
        PRE_MAINT_REPLAY_CONTEXT_FILES,
    )
}

pub fn refresh_runtime_store(target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(target)?;
    import_directory_snapshot(&store, target)?;
    Ok(())
}

pub fn seed_classic_runtime_files(target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(
        &crate::support::paths::default_fixture_dir(),
        target,
        CLASSIC_RUNTIME_SUPPORT_FILES,
    )
}

pub fn overlay_classic_runtime_files(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for name in CLASSIC_RUNTIME_SUPPORT_FILES {
        let source_path = source.join(name);
        let target_path = target.join(name);
        if source_path.exists() && source_path != target_path {
            fs::create_dir_all(target)?;
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

pub fn match_fixture_set(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let candidates = [
        (
            "original/v1.5",
            crate::support::paths::default_fixture_dir(),
            ORIGINAL_FILES,
        ),
        ("fixtures/ecutil-init/v1.5", init_fixture_dir(), INIT_FILES),
        (
            "fixtures/ecmaint-post/v1.5",
            post_maint_fixture_dir(),
            INIT_FILES,
        ),
    ];

    println!("Directory: {}", dir.display());
    let mut matched_any = false;
    let current_known_core = post_maint_fixture_dir();
    if dir_matches(dir, &current_known_core, CURRENT_KNOWN_CORE_FILES)? {
        println!("MATCH current-known-post-maint-baseline-core");
        matched_any = true;
    }

    for (label, candidate, files) in candidates {
        if dir_matches(dir, &candidate, files)? {
            println!("MATCH {label}");
            matched_any = true;
        }
    }
    if !matched_any {
        println!("MATCH none");
    }

    Ok(())
}

fn dir_matches(
    dir: &Path,
    candidate: &Path,
    files: &[&str],
) -> Result<bool, Box<dyn std::error::Error>> {
    for name in files {
        if fs::read(dir.join(name))? != fs::read(candidate.join(name))? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn copy_top_level_files(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        fs::copy(&path, target.join(file_name))?;
    }

    Ok(())
}

fn copy_named_files(
    source: &Path,
    target: &Path,
    names: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;

    for name in names {
        fs::copy(source.join(name), target.join(name))?;
    }

    Ok(())
}

/// Generate DATABASE.DAT from PLANETS.DAT and CONQUEST.DAT.
///
/// Reads the template from the init fixture, copies planet names from PLANETS.DAT,
/// and embeds the CONQUEST.DAT year in homeworld records.
pub fn generate_database_dat(target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let init_dir = init_fixture_dir();
    regenerate_database_dat_from_directory(target, Some(&init_dir.join("DATABASE.DAT")))?;
    Ok(())
}
