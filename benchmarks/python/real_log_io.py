"""Benchmark patched python-can on the same real ASC and BLF files as rust-can."""

from __future__ import annotations

import argparse
import datetime
import json
import statistics
import sys
import time
from pathlib import Path
from typing import Any


def iter_asc_files(root: Path) -> list[Path]:
    return sorted(root.rglob("*.asc"), key=lambda path: path.stat().st_size, reverse=True)


def default_asc_path() -> Path:
    files = iter_asc_files(Path("data/extracted"))
    if not files:
        raise FileNotFoundError("data/extracted does not contain ASC files")
    return files[0]


def run_asc(can_module: Any, asc_path: Path, limit: int) -> dict[str, Any]:
    start = time.perf_counter()
    classic = 0
    fd = 0
    with can_module.ASCReader(str(asc_path)) as reader:
        for message in reader:
            if classic + fd >= limit:
                break
            if message.is_fd:
                fd += 1
            else:
                classic += 1
    seconds = time.perf_counter() - start
    messages = classic + fd
    return {
        "seconds": seconds,
        "messages": messages,
        "fd": fd,
        "classic": classic,
        "messages_per_second": messages / seconds,
    }


def run_blf(can_module: Any, blf_path: Path) -> dict[str, Any]:
    start = time.perf_counter()
    classic = 0
    fd = 0
    with can_module.BLFReader(str(blf_path)) as reader:
        for message in reader:
            if message.is_fd:
                fd += 1
            else:
                classic += 1
    seconds = time.perf_counter() - start
    messages = classic + fd
    return {
        "seconds": seconds,
        "messages": messages,
        "fd": fd,
        "classic": classic,
        "messages_per_second": messages / seconds,
    }


def summarize(runs: list[dict[str, Any]]) -> tuple[float, float]:
    speeds = [run["messages_per_second"] for run in runs]
    return statistics.mean(speeds), min(speeds)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--asc", type=Path, default=None)
    parser.add_argument("--blf", type=Path, default=Path("data/generated/real_can_canfd_10000.blf"))
    parser.add_argument("--asc-limit", type=int, default=100_000)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--python-can", type=Path, default=Path(".external/python-can"))
    args = parser.parse_args()

    sys.path.insert(0, str(args.python_can.resolve()))
    import can  # type: ignore

    asc_path = args.asc or default_asc_path()
    asc_runs = [run_asc(can, asc_path, args.asc_limit) for _ in range(args.runs)]
    blf_runs = [run_blf(can, args.blf) for _ in range(args.runs)]
    asc_mean, asc_min = summarize(asc_runs)
    blf_mean, blf_min = summarize(blf_runs)

    print(
        json.dumps(
            {
                "date": datetime.date.today().isoformat(),
                "language": "python",
                "python_can_source": ".external/python-can patched ASCReader for current CANFD dialect",
                "asc_source": str(asc_path),
                "blf_source": str(args.blf),
                "asc_limit": args.asc_limit,
                "runs": args.runs,
                "asc_runs": asc_runs,
                "blf_runs": blf_runs,
                "summary": {
                    "asc_mean_messages_per_second": asc_mean,
                    "asc_min_messages_per_second": asc_min,
                    "blf_mean_messages_per_second": blf_mean,
                    "blf_min_messages_per_second": blf_min,
                },
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
