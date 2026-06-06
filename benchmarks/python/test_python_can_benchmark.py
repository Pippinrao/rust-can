from __future__ import annotations

import copy
import pathlib
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
PYTHON_CAN = ROOT / ".external" / "python-can"
sys.path.insert(0, str(PYTHON_CAN))

import can  # noqa: E402


CLASSIC_PAYLOAD = bytes([1, 2, 3, 4, 5, 6, 7, 8])
FD_PAYLOAD = bytes([0xAA] * 64)


def test_classic_message_create_8b(benchmark):
    benchmark(
        lambda: can.Message(
            arbitration_id=0x123,
            is_extended_id=False,
            data=CLASSIC_PAYLOAD,
        )
    )


def test_fd_message_create_64b(benchmark):
    benchmark(
        lambda: can.Message(
            arbitration_id=0x18FF_50E5,
            is_extended_id=True,
            is_fd=True,
            bitrate_switch=True,
            data=FD_PAYLOAD,
        )
    )


def test_message_clone_8b(benchmark):
    msg = can.Message(arbitration_id=0x123, is_extended_id=False, data=CLASSIC_PAYLOAD)
    benchmark(lambda: copy.copy(msg))


def test_message_validate_8b(benchmark):
    msg = can.Message(arbitration_id=0x123, is_extended_id=False, data=CLASSIC_PAYLOAD)
    benchmark(lambda: msg._check())


def test_filter_match_4_filters(benchmark):
    msg = can.Message(
        arbitration_id=0x18FF_50E5,
        is_extended_id=True,
        data=CLASSIC_PAYLOAD,
    )
    bus = can.Bus(
        interface="virtual",
        channel=f"rust-can-pytest-bench-{id(msg)}",
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
        benchmark(lambda: bus._matches_filters(msg))
    finally:
        bus.shutdown()
