//! Prepare BLF fixtures with the rust-can BLF writer.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::BlfWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let asc_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_asc_path);
    let output_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/generated/rust_can_canfd_100000.blf"));
    let limit = args
        .get(3)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100_000);

    let input = File::open(&asc_path)?;
    let events = AscReader::new(BufReader::new(input)).collect_can_events_limit(limit)?;
    let output = File::create(&output_path)?;
    let mut writer = BlfWriter::new(BufWriter::new(output));
    for event in &events {
        writer.write_event(event)?;
    }
    writer.finish()?;

    println!(
        "{}",
        serde_json::json!({
            "asc_source": asc_path,
            "blf_output": output_path,
            "messages": events.len(),
        })
    );
    Ok(())
}

fn default_asc_path() -> PathBuf {
    let root = PathBuf::from("data/extracted");
    let mut files = Vec::new();
    collect_asc_files(&root, &mut files);
    files
        .into_iter()
        .max_by_key(|path| path.metadata().map(|metadata| metadata.len()).unwrap_or(0))
        .unwrap_or_else(|| PathBuf::from("data/generated/missing.asc"))
}

fn collect_asc_files(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_asc_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "asc") {
            files.push(path);
        }
    }
}
