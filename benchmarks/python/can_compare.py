#!/usr/bin/env python3
"""Compare the rust-can ASC/BLF readers against python-can on the
real corpus under data/.

Outputs a JSON file (default: can_compare.json) with per-format
throughput (msgs/s) and peak resident-set size after parsing.

Usage:
    python can_compare.py
    python can_compare.py --runs 5
    python can_compare.py --asc data/extracted/.../...asc --blf data/generated/...blf
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import sys
import time
from pathlib import Path

# python-can logs a warning for every CAN FD frame whose DLC code
# does not match the data length — these are benign for benchmarking.
logging.getLogger("can.io.asc").setLevel(logging.ERROR)
logging.getLogger("can.io.blf").setLevel(logging.ERROR)


def rss_bytes() -> int:
    """Peak resident-set size in bytes. Cross-platform fallback.

    Order:
      1. POSIX `resource` (Linux, macOS).
      2. Windows `GetProcessMemoryInfo` via `ctypes`.
    """
    try:
        import resource

        return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
    except (ImportError, AttributeError):
        pass
    if os.name == "nt":
        import ctypes
        from ctypes import wintypes

        class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
            _fields_ = [
                ("cb", wintypes.DWORD),
                ("PageFaultCount", wintypes.DWORD),
                ("PeakWorkingSetSize", ctypes.c_size_t),
                ("WorkingSetSize", ctypes.c_size_t),
                ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
                ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                ("PagefileUsage", ctypes.c_size_t),
                ("PeakPagefileUsage", ctypes.c_size_t),
            ]

        counters = PROCESS_MEMORY_COUNTERS()
        counters.cb = ctypes.sizeof(PROCESS_MEMORY_COUNTERS)
        # ctypes.windll.psapi works but we must set argtypes so 64-bit
        # values (HANDLE, SIZE_T) are not truncated to 32-bit c_int.
        psapi = ctypes.WinDLL("psapi")
        psapi.GetProcessMemoryInfo.argtypes = [
            wintypes.HANDLE,
            ctypes.POINTER(PROCESS_MEMORY_COUNTERS),
            wintypes.DWORD,
        ]
        psapi.GetProcessMemoryInfo.restype = wintypes.BOOL
        psapi.GetProcessMemoryInfo(
            ctypes.windll.kernel32.GetCurrentProcess(),
            ctypes.byref(counters),
            counters.cb,
        )
        return counters.PeakWorkingSetSize
    raise OSError("no RSS source available on this platform")


def rss_kb() -> int:
    return rss_bytes() // 1024


def first_asc_under(data_dir: Path) -> Path:
    """Return the path to the first real ASC file under data/extracted/."""
    extracted = data_dir / "extracted"
    if not extracted.is_dir():
        raise SystemExit(f"data dir not found: {extracted}")
    for sub in sorted(extracted.iterdir()):
        if not sub.is_dir():
            continue
        for f in sub.iterdir():
            if f.suffix.lower() == ".asc":
                return f
    raise SystemExit("no .asc files under data/extracted/")


def default_blf(data_dir: Path) -> Path:
    return data_dir / "generated" / "real_can_canfd_10000.blf"


def _is_lin_line(line: str) -> bool:
    """True when *line* is a LIN frame that python-can's ASCReader cannot parse."""
    stripped = line.lstrip()
    if not stripped:
        return False
    parts = stripped.split(maxsplit=3)
    if len(parts) < 3:
        return False
    tok = parts[1]
    # LIN channel tokens look like "L11", "L14", "L15" — an 'L'
    # prefix followed by digits.
    return len(tok) >= 2 and tok[0] == "L" and tok[1:].isdigit()


def _prepare_asc_text(path: Path) -> str:
    """Return ASC file content compatible with python-can.

    Two transforms are applied:

    1. LIN lines are removed — python-can 4.6.1's ASCReader does not
       handle them (the ``L11`` channel token is misinterpreted as a
       CAN-id prefix).
    2. CAN FD lines are reordered — python-can 4.6.1 assumes the token
       sequence ``channel direction arb_id …`` but the corpus uses
       ``channel arb_id direction …``.  We swap the third and fourth
       tokens so python-can's parser sees the order it expects.
    """
    text = path.read_text(encoding="utf-8", errors="replace")
    out: list[str] = []
    for line in text.splitlines():
        if _is_lin_line(line):
            continue
        parts = line.split(maxsplit=5)
        if len(parts) >= 5 and parts[1] == "CANFD":
            # Swap token-3 (arb_id) and token-4 (direction).
            parts[3], parts[4] = parts[4], parts[3]
            out.append(" ".join(parts))
        else:
            out.append(line)
    return "\n".join(out) + "\n"


def bench_asc(path: Path, runs: int) -> dict:
    from can.io.asc import ASCReader
    import io as _io

    # Pre-filter LIN lines and reorder CAN FD tokens so python-can can
    # parse the real corpus. This is done once outside the timed loop;
    # only ASCReader parsing is measured.
    filtered = _prepare_asc_text(path)

    # Warm: parse once so Python bytecode / regex caches are hot.
    with _io.StringIO(filtered) as f:
        sum(1 for _ in ASCReader(f))

    timings: list[float] = []
    peak_rss: list[int] = []
    for _ in range(runs):
        start = time.perf_counter()
        with _io.StringIO(filtered) as f:
            n = sum(1 for _ in ASCReader(f))
        elapsed = time.perf_counter() - start
        timings.append(elapsed)
        peak_rss.append(rss_kb())
    return {
        "source": str(path),
        "runs": runs,
        "mean_seconds": sum(timings) / len(timings),
        "min_seconds": min(timings),
        "messages_per_run": n,
        "mean_messages_per_second": n / (sum(timings) / len(timings)),
        "max_messages_per_second": n / min(timings),
        "peak_rss_kb": max(peak_rss),
    }


def bench_blf(path: Path, runs: int) -> dict:
    from can.io.blf import BLFReader

    # Warm the cache.
    with path.open("rb") as f:
        for _ in BLFReader(f):
            pass

    timings: list[float] = []
    peak_rss: list[int] = []
    for _ in range(runs):
        start = time.perf_counter()
        with path.open("rb") as f:
            count = 0
            for msg in BLFReader(f):
                count += 1
        elapsed = time.perf_counter() - start
        timings.append(elapsed)
        peak_rss.append(rss_kb())
    return {
        "source": str(path),
        "runs": runs,
        "mean_seconds": sum(timings) / len(timings),
        "min_seconds": min(timings),
        "messages_per_run": count,
        "mean_messages_per_second": count / (sum(timings) / len(timings)),
        "max_messages_per_second": count / min(timings),
        "peak_rss_kb": max(peak_rss),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--data-dir",
        type=Path,
        default=Path("data"),
        help="root of the rust-can data/ directory (default: data)",
    )
    parser.add_argument(
        "--asc",
        type=Path,
        default=None,
        help="path to an ASC fixture (default: first .asc under <data>/extracted)",
    )
    parser.add_argument(
        "--blf",
        type=Path,
        default=None,
        help="path to a BLF fixture (default: <data>/generated/real_can_canfd_10000.blf)",
    )
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("benchmarks/results/2026-06-07/can_compare.json"),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    asc_path = args.asc or first_asc_under(args.data_dir)
    blf_path = args.blf or default_blf(args.data_dir)
    if not asc_path.is_file():
        raise SystemExit(f"ASC fixture not found: {asc_path}")
    if not blf_path.is_file():
        raise SystemExit(f"BLF fixture not found: {blf_path}")

    print(f"python-can {__import__('can').__version__}", file=sys.stderr)
    print(f"ASC fixture: {asc_path}", file=sys.stderr)
    print(f"BLF fixture: {blf_path}", file=sys.stderr)
    print(f"runs: {args.runs}", file=sys.stderr)

    asc = bench_asc(asc_path, args.runs)
    blf = bench_blf(blf_path, args.runs)

    out = {
        "language": "python",
        "python_can_version": __import__("can").__version__,
        "runs": args.runs,
        "asc": asc,
        "blf": blf,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, indent=2))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
