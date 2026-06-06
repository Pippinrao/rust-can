from __future__ import annotations

import copy
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


def main() -> None:
    iterations = int(sys.argv[1]) if len(sys.argv) > 1 else 1_000_000
    classic_payload = bytes([1, 2, 3, 4, 5, 6, 7, 8])
    fd_payload = bytes([0xAA] * 64)
    clone_msg = can.Message(
        arbitration_id=0x123,
        is_extended_id=False,
        data=classic_payload,
    )
    validate_msg = can.Message(
        arbitration_id=0x123,
        is_extended_id=False,
        data=classic_payload,
    )
    filter_msg = can.Message(
        arbitration_id=0x18FF_50E5,
        is_extended_id=True,
        data=classic_payload,
    )
    bus = can.Bus(
        interface="virtual",
        channel=f"rust-can-bench-{id(filter_msg)}",
        receive_own_messages=True,
    )
    bus.set_filters(
        [
            {"can_id": 0x100, "can_mask": 0x700, "extended": False},
            {"can_id": 0x200, "can_mask": 0x700, "extended": False},
            {"can_id": 0x18FF_0000, "can_mask": 0x1FFF_0000, "extended": True},
            {"can_id": 0x7E8, "can_mask": 0x7FF, "extended": False},
        ]
    )

    try:
        results = [
            bench(
                "classic_message_create_8b",
                iterations,
                lambda: can.Message(
                    arbitration_id=0x123,
                    is_extended_id=False,
                    data=classic_payload,
                ),
            ),
            bench(
                "fd_message_create_64b",
                iterations,
                lambda: can.Message(
                    arbitration_id=0x18FF_50E5,
                    is_extended_id=True,
                    is_fd=True,
                    bitrate_switch=True,
                    data=fd_payload,
                ),
            ),
            bench("message_clone_8b", iterations, lambda: copy.copy(clone_msg)),
            bench("message_validate_8b", iterations, lambda: validate_msg._check()),
            bench("filter_match_4_filters", iterations, lambda: bus._matches_filters(filter_msg)),
        ]
    finally:
        bus.shutdown()

    print(
        json.dumps(
            {
                "language": "python",
                "python_can_commit": "491a691fd1faffab1c48956bafd711e7c653db54",
                "iterations": iterations,
                "results": results,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
