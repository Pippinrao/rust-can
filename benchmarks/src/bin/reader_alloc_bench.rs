//! Per-reader allocation benchmark for ASC and BLF log readers.
//!
//! Reuses the CountingAlloc global allocator to attribute every
//! allocation to the reader code path under test. Runs both the
//! `collect_events` (allocating) and `scan_can_stats` (alloc-once
//! for internal buffers) entry points against the real corpus
//! fixtures so the report covers the realistic memory profile.

#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::BlfReader;
use serde_json::json;

struct CountingAlloc;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

fn reset() {
    ALLOCATED.store(0, Ordering::Relaxed);
    DEALLOCATED.store(0, Ordering::Relaxed);
    ALLOCS.store(0, Ordering::Relaxed);
}

fn snapshot() -> (usize, usize, usize) {
    (
        ALLOCATED.load(Ordering::Relaxed),
        DEALLOCATED.load(Ordering::Relaxed),
        ALLOCS.load(Ordering::Relaxed),
    )
}

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

    // --- ASC: scan_can_stats_limit (zero event Vec; only reader buffers) ---
    reset();
    let start = Instant::now();
    let file = File::open(&asc_path).expect("ASC file should open");
    let stats = AscReader::new(BufReader::new(file))
        .scan_can_stats_limit(100_000)
        .expect("ASC scan should succeed");
    let asc_scan_elapsed = start.elapsed();
    let asc_messages = stats.messages;
    let (asc_scan_alloc_bytes, asc_scan_dealloc_bytes, asc_scan_allocs) = snapshot();

    // --- ASC: collect_events (allocates Vec<LogEvent> + per-event Payload) ---
    reset();
    let start = Instant::now();
    let file = File::open(&asc_path).expect("ASC file should open");
    let events = AscReader::new(BufReader::new(file))
        .collect_events()
        .expect("ASC collect should succeed");
    let asc_collect_elapsed = start.elapsed();
    drop(events);
    let (asc_collect_alloc_bytes, asc_collect_dealloc_bytes, asc_collect_allocs) = snapshot();

    // --- BLF: scan_can_stats (zero event Vec; allocates body/decompress bufs) ---
    reset();
    let start = Instant::now();
    let file = File::open(&blf_path).expect("BLF file should open");
    let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
    let blf_stats = reader.scan_can_stats().expect("BLF scan should succeed");
    let blf_scan_elapsed = start.elapsed();
    drop(reader);
    let (blf_scan_alloc_bytes, blf_scan_dealloc_bytes, blf_scan_allocs) = snapshot();

    // --- BLF: collect_events (allocates Vec<LogEvent> + Payload vecs) ---
    reset();
    let start = Instant::now();
    let file = File::open(&blf_path).expect("BLF file should open");
    let mut reader = BlfReader::new(BufReader::new(file)).expect("BLF header should parse");
    let events = reader.collect_events().expect("BLF collect should succeed");
    let blf_collect_elapsed = start.elapsed();
    let blf_collect_count = events.len();
    drop(events);
    let (blf_collect_alloc_bytes, blf_collect_dealloc_bytes, blf_collect_allocs) = snapshot();

    let report = json!({
        "language": "rust",
        "scenario": "PERF-IO-ALLOC-001",
        "asc": {
            "source": asc_path,
            "messages": asc_messages,
            "scan": {
                "seconds": asc_scan_elapsed.as_secs_f64(),
                "alloc_count": asc_scan_allocs,
                "bytes_allocated": asc_scan_alloc_bytes,
                "bytes_deallocated": asc_scan_dealloc_bytes,
                "bytes_per_message": if asc_messages > 0 { asc_scan_alloc_bytes as f64 / asc_messages as f64 } else { 0.0 },
            },
            "collect": {
                "seconds": asc_collect_elapsed.as_secs_f64(),
                "alloc_count": asc_collect_allocs,
                "bytes_allocated": asc_collect_alloc_bytes,
                "bytes_deallocated": asc_collect_dealloc_bytes,
                "allocs_per_message": if asc_messages > 0 { asc_collect_allocs as f64 / asc_messages as f64 } else { 0.0 },
                "bytes_per_message": if asc_messages > 0 { asc_collect_alloc_bytes as f64 / asc_messages as f64 } else { 0.0 },
            }
        },
        "blf": {
            "source": blf_path,
            "messages": blf_stats.messages,
            "scan": {
                "seconds": blf_scan_elapsed.as_secs_f64(),
                "alloc_count": blf_scan_allocs,
                "bytes_allocated": blf_scan_alloc_bytes,
                "bytes_deallocated": blf_scan_dealloc_bytes,
                "bytes_per_message": if blf_stats.messages > 0 { blf_scan_alloc_bytes as f64 / blf_stats.messages as f64 } else { 0.0 },
            },
            "collect": {
                "seconds": blf_collect_elapsed.as_secs_f64(),
                "events": blf_collect_count,
                "alloc_count": blf_collect_allocs,
                "bytes_allocated": blf_collect_alloc_bytes,
                "bytes_deallocated": blf_collect_dealloc_bytes,
                "allocs_per_message": if blf_collect_count > 0 { blf_collect_allocs as f64 / blf_collect_count as f64 } else { 0.0 },
                "bytes_per_message": if blf_collect_count > 0 { blf_collect_alloc_bytes as f64 / blf_collect_count as f64 } else { 0.0 },
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("JSON should serialize"));
}
