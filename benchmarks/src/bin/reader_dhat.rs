//! Per-allocation-site heap profile for ASC and BLF log readers.
//!
//! Uses dhat-rs as a global allocator substitute. dhat attributes
//! every allocation to a call site and writes a JSON report that
//! can be converted to a flamegraph. This pinpoints the exact line
//! in rust-can-io that drives the allocation count we see in
//! reader_alloc_bench.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::BlfReader;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn default_asc_path() -> PathBuf {
    let extracted = Path::new("data").join("extracted");
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

    // -- ASC: collect_events --
    let asc_prof = dhat::Profiler::new_heap();
    {
        let file = File::open(&asc_path).expect("ASC file should open");
        let _events = AscReader::new(BufReader::new(file))
            .collect_events()
            .expect("ASC collect should succeed");
    }
    drop(asc_prof);

    // -- ASC: scan_can_stats_limit --
    let asc_scan_prof = dhat::Profiler::new_heap();
    {
        let file = File::open(&asc_path).expect("ASC file should open");
        let _stats = AscReader::new(BufReader::new(file))
            .scan_can_stats_limit(100_000)
            .expect("ASC scan should succeed");
    }
    drop(asc_scan_prof);

    // -- BLF: collect_events --
    let blf_prof = dhat::Profiler::new_heap();
    {
        let file = File::open(&blf_path).expect("BLF file should open");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
        let _events = reader.collect_events().expect("BLF collect should succeed");
    }
    drop(blf_prof);

    // -- BLF: scan_can_stats --
    let blf_scan_prof = dhat::Profiler::new_heap();
    {
        let file = File::open(&blf_path).expect("BLF file should open");
        let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
        let _stats = reader.scan_can_stats().expect("BLF scan should succeed");
    }
    drop(blf_scan_prof);

    eprintln!(
        "dhat reports written. Convert to flamegraphs with:\n  \
         dhat-to-flamegraph dhat-heap-*.json > flamegraph.html"
    );
}
