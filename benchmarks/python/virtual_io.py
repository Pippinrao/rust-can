#!/usr/bin/env python3
"""python-can virtual bus throughput comparison (PERF-ADP-001)."""

from __future__ import annotations

import json
import pathlib
import sys
import time
from typing import Callable


ROOT = pathlib.Path(__file__).resolve().parents[2]
PYTHON_CAN = ROOT / ".external" / "python-can"
sys.path.insert(0, str(PYTHON_CAN))

import can  # noqa: E402


def bench(name: str, iterations: int, func: Callable[[], object]) -> dict[str, object]:
    start = time.perf_counter_ns()
    for _ in range(iterations):
        func()
    total_ns = time.perf_counter_ns() - start
    return {
        "name": name,
        "iterations": iterations,
        "total_ns": total_ns,
        "ns_per_iter": total_ns / iterations,
    }


def main() -> int:
    iterations = int(sys.argv[1]) if len(sys.argv) > 1 else 100_000
    channel = f"rust-can-virtual-{id(iterations)}"
    tx = can.Bus(interface="virtual", channel=channel, receive_own_messages=False)
    rx = can.Bus(interface="virtual", channel=channel, receive_own_messages=False)
    msg = can.Message(arbitration_id=0x123, is_extended_id=False, data=bytes([1, 2, 3, 4]))

    try:
        results = [
            bench(
                "virtual_send_recv_roundtrip",
                iterations,
                lambda: (
                    tx.send(msg),
                    rx.recv(timeout=0.01),
                ),
            ),
        ]
    finally:
        tx.shutdown()
        rx.shutdown()

    print(
        json.dumps(
            {
                "language": "python",
                "python_can_commit": "491a691fd1faffab1c48956bafd711e7c653db54",
                "scenario": "PERF-ADP-001",
                "iterations": iterations,
                "results": results,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
