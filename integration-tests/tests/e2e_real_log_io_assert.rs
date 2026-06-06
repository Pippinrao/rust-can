//! E2E-IO-005: real_log_io assert-count via library path (subprocess when bin exists).

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::BlfReader;

fn fixture_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
        .join("generated")
}

fn default_asc_path() -> std::path::PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
        .join("extracted");
    let mut files = Vec::new();
    collect_asc_files(&root, &mut files);
    files
        .into_iter()
        .max_by_key(|path| path.metadata().map(|meta| meta.len()).unwrap_or(0))
        .unwrap_or_else(|| root.join("missing.asc"))
}

fn collect_asc_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_asc_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "asc") {
            files.push(path);
        }
    }
}

fn assert_counts(asc_messages: usize, blf_messages: usize, blf_classic: usize, blf_fd: usize, asc_limit: usize) {
    assert!(asc_messages > 0, "ASC should contain messages");
    assert!(blf_messages > 0, "BLF should contain messages");
    assert!(blf_classic + blf_fd > 0, "BLF should contain CAN or CANFD frames");
    assert!(asc_messages <= asc_limit, "ASC message count should respect limit");
}

#[test]
fn e2e_io_005_real_log_io_assert_count_lib() {
    let blf_path = fixture_root().join("real_can_canfd_10000.blf");
    if !blf_path.exists() {
        eprintln!("skip E2E-IO-005 lib: missing BLF fixture {}", blf_path.display());
        return;
    }

    let asc_path = default_asc_path();
    if !asc_path.exists() {
        eprintln!("skip E2E-IO-005 lib: missing ASC fixture {}", asc_path.display());
        return;
    }

    let asc_limit = 100_000;
    let asc_stats = AscReader::new(BufReader::new(File::open(&asc_path).unwrap()))
        .scan_can_stats_limit(asc_limit)
        .expect("ASC scan should succeed");
    let mut blf_reader = BlfReader::new(BufReader::new(File::open(&blf_path).unwrap()))
        .expect("BLF header should parse");
    let blf_stats = blf_reader.scan_can_stats().expect("BLF scan should succeed");

    assert_counts(
        asc_stats.messages,
        blf_stats.messages,
        blf_stats.classic,
        blf_stats.fd,
        asc_limit,
    );
    assert_eq!(blf_stats.messages, 10_000);
}

#[test]
fn e2e_io_005_real_log_io_assert_count_subprocess_when_bin_exists() {
    let blf_path = fixture_root().join("real_can_canfd_10000.blf");
    if !blf_path.exists() {
        return;
    }
    let asc_path = default_asc_path();
    if !asc_path.exists() {
        return;
    }

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let bin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join(&profile)
        .join("real_log_io.exe");
    let bin = if bin.exists() {
        bin
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join(profile)
            .join("real_log_io")
    };
    if !bin.exists() {
        eprintln!("skip subprocess path: real_log_io bin not built");
        return;
    }

    let output = Command::new(bin)
        .arg("--assert-count")
        .arg(&asc_path)
        .arg(&blf_path)
        .arg("100000")
        .output()
        .expect("real_log_io subprocess should spawn");
    assert!(output.status.success());
}
