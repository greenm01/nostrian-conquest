//! Workspace policy: the seven core .DAT codecs belong at the compat edge.
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_CRATES: &[&str] = &[
    "nc-data",
    "nc-engine",
    "nc-game",
    "nc-helm",
    "nc-host",
    "nc-session",
    "nc-sysop",
];
const RUNTIME_CRATES: &[&str] = &["nc-engine", "nc-game", "nc-helm", "nc-host", "nc-session"];

fn source_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            source_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn code_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("//")
                && !line.starts_with("/*")
                && !line.starts_with('*')
                && !line.starts_with("*/")
        })
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn codec_identifier(word: &str) -> bool {
    if matches!(
        word,
        "ClassicFile" | "ignored_bytes" | "without_ignored_bytes"
    ) {
        return true;
    }
    [
        "player",
        "players",
        "planet",
        "planets",
        "fleet",
        "fleets",
        "base",
        "bases",
        "starbase",
        "starbases",
        "ipbm",
        "ipbms",
        "setup",
        "conquest",
    ]
    .iter()
    .any(|name| {
        [
            format!("parse_{name}_dat"),
            format!("serialize_{name}_dat"),
            format!("encode_{name}"),
            format!("decode_{name}"),
        ]
        .contains(&word.to_owned())
    })
}

fn imports_classic(compact: &str, namespace: &str) -> bool {
    compact.contains(&format!("{namespace}::classic"))
        || compact
            .split(&format!("{namespace}::{{"))
            .skip(1)
            .any(|group| {
                group
                    .split('}')
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .any(|item| {
                        item == "classic"
                            || item.starts_with("classic::")
                            || item.starts_with("classicas")
                    })
            })
}

fn source_violations(path: &Path, source: &str) -> Vec<String> {
    // Match source-policy.rs's comment convention. This is a lexical guard,
    // not a Rust parser; codec names in non-comment strings are also checked.
    let code = code_text(source);
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let words: Vec<_> = code
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect();
    let mut errors: Vec<String> = words
        .iter()
        .filter(|word| codec_identifier(word))
        .map(|word| format!("codec identifier {word}"))
        .collect();
    if compact.contains(".raw[") {
        errors.push("direct .raw byte indexing".into());
    }
    let path = path.to_string_lossy().replace('\\', "/");
    let theme_module = path.ends_with("/theme.rs") || path.ends_with("/theme/mod.rs");
    if !theme_module && words.windows(2).any(|pair| pair == ["mod", "classic"]) {
        errors.push("classic module outside a UI theme".into());
    }
    // Bare classic:: is the UI style namespace only when that namespace is
    // imported from a theme. Explicit codec imports are never exempt.
    let imports_theme = imports_classic(&compact, "theme");
    if compact.contains("classic::") && !imports_theme && !theme_module {
        errors.push("classic namespace outside a UI theme import".into());
    }
    for namespace in ["nc_data", "nc_compat", "nc_classic", "crate"] {
        if imports_classic(&compact, namespace) {
            errors.push(format!("{namespace} classic import"));
        }
    }
    errors.sort();
    errors.dedup();
    errors
}

fn dependency_violations(manifest: &str) -> Vec<String> {
    let mut runtime_dependencies = false;
    let mut errors = Vec::new();
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') {
            let section = line.trim_matches(['[', ']']);
            runtime_dependencies = section == "dependencies"
                || section.starts_with("dependencies.")
                || section.contains(".dependencies.")
                || section.ends_with(".dependencies");
        }
        if runtime_dependencies
            && ["nc-compat", "nc-classic"]
                .iter()
                .any(|name| line.contains(name))
        {
            errors.push(line.to_owned());
        }
    }
    errors
}

#[test]
fn core_classic_codecs_stay_out_of_runtime_sources() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut files = Vec::new();
    for name in SOURCE_CRATES {
        source_files(&workspace.join(name).join("src"), &mut files);
    }
    files.sort();
    let offenders: Vec<_> = files
        .iter()
        .filter_map(|path| {
            let errors = source_violations(path, &fs::read_to_string(path).expect("read source"));
            (!errors.is_empty()).then(|| (path.strip_prefix(workspace).unwrap(), errors))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "Core .DAT codecs belong in nc-compat, not runtime sources: {offenders:#?}"
    );
}

#[test]
fn runtime_dependencies_do_not_include_classic_crates() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let offenders: Vec<_> = RUNTIME_CRATES
        .iter()
        .filter_map(|name| {
            let text =
                fs::read_to_string(workspace.join(name).join("Cargo.toml")).expect("read manifest");
            let errors = dependency_violations(&text);
            (!errors.is_empty()).then_some((name, errors))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "Classic crates may only be dev-dependencies here: {offenders:#?}"
    );
}

#[test]
fn policy_distinguishes_themes_and_dev_dependencies_from_codecs() {
    let path = Path::new("nc-game/src/screen.rs");
    assert!(
        source_violations(
            path,
            "use crate::theme::{ThemeEntry, classic}; classic::body_style();"
        )
        .is_empty()
    );
    assert!(
        source_violations(
            Path::new("nc-helm/src/dashboard/theme/mod.rs"),
            "pub mod classic {}"
        )
        .is_empty()
    );
    for source in [
        "pub mod classic;",
        "use nc_data::{Fleet, classic};",
        "classic :: decode_fleet(bytes);",
        "let x = fleet.raw [0];",
        "serialize_planets_dat(p);",
        "use nc_compat::classic::*;",
        "use crate::theme::classic; let x = ClassicFile::Fleets;",
        "ignored_bytes(f, b);",
    ] {
        assert!(
            !source_violations(path, source).is_empty(),
            "missed {source}"
        );
    }
    assert!(dependency_violations("[dependencies]\nnc-data = {}\n[dev-dependencies]\nnc-compat = {}\n[target.'cfg(unix)'.dev-dependencies]\nnc-classic = {}").is_empty());
    for manifest in [
        "[dependencies]\nnc-compat = {}",
        "[dependencies.alias]\npackage = \"nc-classic\"",
        "[target.'cfg(unix)'.dependencies]\ncompat = { package = \"nc-compat\" }",
    ] {
        assert!(
            !dependency_violations(manifest).is_empty(),
            "missed {manifest}"
        );
    }
}
