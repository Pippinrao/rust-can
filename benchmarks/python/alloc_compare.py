#!/usr/bin/env python3
"""Allocation comparison for CAN message creation (PERF-ALLOC-001)."""

from __future__ import annotations

import json
import pathlib
import sys
import tracemalloc


ROOT = pathlib.Path(__file__).resolve().parents[2]
PYTHON_CAN = ROOT / ".external" / "python-can"
sys.path.insert(0, str(PYTHON_CAN))

import can  # noqa: E402


def bench(name: str, iterations: int, func) -> dict[str, object]:
    tracemalloc.start()
    for _ in range(iterations):
        func()
    _, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    return {
        "name": name,
        "iterations": iterations,
        "peak_bytes": peak,
        "bytes_per_iter": peak / iterations,
    }


def main() -> int:
    iterations = int(sys.argv[1]) if len(sys.argv) > 1 else 100_000
    classic_payload = bytes([1, 2, 3, 4, 5, 6, 7, 8])
    fd_payload = bytes([0xAA] * 64)

    results = [
        bench(
            "classic_message_create_8b_alloc",
            iterations,
            lambda: can.Message(
                arbitration_id=0x123,
                is_extended_id=False,
                data=classic_payload,
            ),
        ),
        bench(
            "fd_message_create_64b_alloc",
            iterations,
            lambda: can.Message(
                arbitration_id=0x18FF_50E5,
                is_extended_id=True,
                is_fd=True,
                bitrate_switch=True,
                data=fd_payload,
            ),
        ),
    ]

    print(
        json.dumps(
            {
                "language": "python",
                "scenario": "PERF-ALLOC-001",
                "iterations": iterations,
                "results": results,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
