//! E2E-IO-004: read real corpus BLF/ASC fixtures and assert counts + sample payload.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use rust_can_io::event::LogEvent;
use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::{BlfCanStats, BlfReader};

fn fixture_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
        .join("generated")
}

fn blf_fixture_path() -> std::path::PathBuf {
    fixture_root().join("real_can_canfd_10000.blf")
}

#[test]
fn e2e_io_004_real_blf_corpus_counts_and_sample_payload() {
    let path = blf_fixture_path();
    if !path.exists() {
        eprintln!("skip E2E-IO-004: missing fixture {}", path.display());
        return;
    }

    let file = File::open(&path).expect("BLF fixture should open");
    let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
    let stats = reader.scan_can_stats().expect("BLF scan should succeed");
    let BlfCanStats {
        messages,
        classic,
        fd,
        ..
    } = stats;

    assert_eq!(messages, 10_000);
    assert_eq!(classic, 1_486);
    assert_eq!(fd, 8_514);

    let file = File::open(&path).expect("BLF fixture should reopen");
    let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
    let events = reader.collect_events().expect("BLF events should parse");
    let sample = events
        .iter()
        .find_map(|event| match event {
            LogEvent::Can(can) => Some(can.data.as_slice().to_vec()),
            _ => None,
        })
        .expect("fixture should contain at least one classic CAN frame");
    assert!(!sample.is_empty());
}

#[test]
fn e2e_io_004_real_asc_corpus_sample_if_present() {
    let extracted = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
        .join("extracted");
    let Some(asc_path) = extracted
        .read_dir()
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "asc"))
                .max_by_key(|path| path.metadata().map(|meta| meta.len()).unwrap_or(0))
        })
    else {
        eprintln!("skip E2E-IO-004 ASC: no extracted ASC fixtures");
        return;
    };

    let file = File::open(&asc_path).expect("ASC fixture should open");
    let stats = AscReader::new(BufReader::new(file))
        .scan_can_stats_limit(1_000)
        .expect("ASC scan should succeed");
    assert!(stats.messages > 0);
    assert!(stats.classic + stats.fd > 0);
}
