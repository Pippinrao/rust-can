//! Profile runner for the ASC / BLF readers.
//!
//! Builds the real readers with the `profile` feature on, runs them
//! against the real corpus, and dumps folded-stack samples that
//! `flamegraph.exe` (or `inferno-flamegraph`) can render as an SVG.
//!
//! Usage:
//!     cargo run --release --bin reader_profile_runner --features profile -- \
//!         <asc_path> <blf_path> <asc_limit> <runs> <out_folded.txt>

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::BlfReader;
use rust_can_io::prof;

fn default_asc_path() -> PathBuf {
    let extracted = std::path::Path::new("data").join("extracted");
    if let Some(first_dir) = std::fs::read_dir(&extracted)
        .ok()
        .and_then(|d| d.flatten().next())
    {
        let path = std::fs::read_dir(first_dir.path())
            .ok()
            .and_then(|d| {
                d.flatten().find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("asc"))
            })
            .map(|entry| entry.path());
        if let Some(file) = path {
            return file;
        }
    }
    PathBuf::from("data/extracted/sample.asc")
}

fn default_blf_path() -> PathBuf {
    PathBuf::from("data/generated/real_can_canfd_10000.blf")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let asc_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_asc_path);
    let blf_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(default_blf_path);
    let asc_limit: usize = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);
    let runs: usize = args
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let out_path = args
        .get(5)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/flamegraph/folded.txt"));

    std::fs::create_dir_all(out_path.parent().unwrap()).ok();
    prof::enable();

    eprintln!(
        "profiling ASC ({}x) + BLF ({}x) -> {}",
        runs, runs, out_path.display()
    );

    // --- ASC collect ---
    for _ in 0..runs {
        let file = File::open(&asc_path).expect("ASC file should open");
        let _events = AscReader::new(BufReader::new(file))
            .collect_events()
            .expect("ASC collect should succeed");
    }

    // --- ASC scan ---
    for _ in 0..runs {
        let file = File::open(&asc_path).expect("ASC file should open");
        let _stats = AscReader::new(BufReader::new(file))
            .scan_can_stats_limit(asc_limit)
            .expect("ASC scan should succeed");
    }

    // --- BLF collect ---
    for _ in 0..runs {
        let file = File::open(&blf_path).expect("BLF file should open");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
        let _events = reader.collect_events().expect("BLF collect should succeed");
    }

    // --- BLF scan ---
    for _ in 0..runs {
        let file = File::open(&blf_path).expect("BLF file should open");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
        let _stats = reader.scan_can_stats().expect("BLF scan should succeed");
    }

    let total_start = Instant::now();
    let mut out = BufWriter::new(File::create(&out_path).expect("out file should create"));
    prof::dump_and_reset(&mut out).expect("dump should succeed");
    out.flush().ok();
    let dump_elapsed = total_start.elapsed();
    eprintln!(
        "wrote {} in {:.2} ms; pass to flamegraph.exe to render",
        out_path.display(),
        dump_elapsed.as_secs_f64() * 1000.0
    );
}
