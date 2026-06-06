//! Benchmark real ASC and BLF log readers and emit JSON results.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rust_can_io::formats::asc::AscCanStats;
use rust_can_io::formats::asc::AscReader;
use rust_can_io::formats::blf::BlfCanStats;
use rust_can_io::formats::blf::BlfReader;
use serde_json::json;

#[derive(Debug)]
struct Run {
    seconds: f64,
    messages: usize,
    classic: usize,
    fd: usize,
    lin: usize,
}

impl Run {
    fn messages_per_second(&self) -> f64 {
        self.messages as f64 / self.seconds
    }
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
    let asc_limit = args
        .get(3)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100_000);
    let runs = args
        .get(4)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5);

    let asc_runs = (0..runs)
        .map(|_| run_asc(&asc_path, asc_limit))
        .collect::<Result<Vec<_>, _>>()
        .expect("ASC benchmark should run");
    let blf_runs = (0..runs)
        .map(|_| run_blf(&blf_path))
        .collect::<Result<Vec<_>, _>>()
        .expect("BLF benchmark should run");

    let report = json!({
        "language": "rust",
        "asc_source": asc_path,
        "blf_source": blf_path,
        "asc_limit": asc_limit,
        "runs": runs,
        "asc_runs": runs_to_json(&asc_runs),
        "blf_runs": runs_to_json(&blf_runs),
        "summary": {
            "asc_mean_messages_per_second": mean_mps(&asc_runs),
            "asc_min_messages_per_second": min_mps(&asc_runs),
            "blf_mean_messages_per_second": mean_mps(&blf_runs),
            "blf_min_messages_per_second": min_mps(&blf_runs),
        }
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("JSON should serialize"));
}

fn run_asc(path: &Path, limit: usize) -> Result<Run, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let file = File::open(path)?;
    let stats = AscReader::new(BufReader::new(file)).scan_can_stats_limit(limit)?;
    let AscCanStats {
        messages,
        classic,
        fd,
        ..
    } = stats;
    Ok(Run {
        seconds: start.elapsed().as_secs_f64(),
        messages,
        classic,
        fd,
        lin: 0,
    })
}

fn run_blf(path: &Path) -> Result<Run, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let file = File::open(path)?;
    let mut reader = BlfReader::new(BufReader::new(file))?;
    let stats = reader.scan_can_stats()?;
    let BlfCanStats {
        messages,
        classic,
        fd,
        ..
    } = stats;
    Ok(Run {
        seconds: start.elapsed().as_secs_f64(),
        messages,
        classic,
        fd,
        lin: 0,
    })
}

fn runs_to_json(runs: &[Run]) -> Vec<serde_json::Value> {
    runs.iter()
        .map(|run| {
            json!({
                "seconds": run.seconds,
                "messages": run.messages,
                "classic": run.classic,
                "fd": run.fd,
                "lin": run.lin,
                "messages_per_second": run.messages_per_second(),
            })
        })
        .collect()
}

fn mean_mps(runs: &[Run]) -> f64 {
    runs.iter().map(Run::messages_per_second).sum::<f64>() / runs.len().max(1) as f64
}

fn min_mps(runs: &[Run]) -> f64 {
    runs.iter()
        .map(Run::messages_per_second)
        .fold(f64::INFINITY, f64::min)
}

fn default_blf_path() -> PathBuf {
    PathBuf::from("data/generated/real_can_canfd_10000.blf")
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

fn collect_asc_files(dir: &Path, files: &mut Vec<PathBuf>) {
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

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{collect_asc_files, mean_mps, min_mps, runs_to_json, Run};

    #[test]
    fn run_reports_messages_per_second() {
        let run = Run {
            seconds: 0.5,
            messages: 100,
            classic: 40,
            fd: 60,
            lin: 0,
        };

        assert_eq!(run.messages_per_second(), 200.0);
    }

    #[test]
    fn summarizes_runs_and_serializes_counts() {
        let runs = [
            Run {
                seconds: 1.0,
                messages: 100,
                classic: 25,
                fd: 75,
                lin: 0,
            },
            Run {
                seconds: 0.5,
                messages: 100,
                classic: 20,
                fd: 80,
                lin: 0,
            },
        ];

        assert_eq!(mean_mps(&runs), 150.0);
        assert_eq!(min_mps(&runs), 100.0);
        let json = runs_to_json(&runs);
        assert_eq!(json[0]["classic"], 25);
        assert_eq!(json[1]["fd"], 80);
    }

    #[test]
    fn collect_asc_files_recurses_and_filters_extensions() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rust-can-real-log-io-{unique}"));
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("test directory should be created");
        File::create(root.join("root.asc")).expect("ASC file should be created");
        File::create(nested.join("nested.asc")).expect("nested ASC file should be created");
        File::create(nested.join("ignored.txt")).expect("ignored file should be created");

        let mut files = Vec::new();
        collect_asc_files(&root, &mut files);
        files.sort();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("root.asc")));
        assert!(files.iter().any(|path| path.ends_with("nested.asc")));

        fs::remove_dir_all(&root).expect("test directory should be removed");
    }
}
