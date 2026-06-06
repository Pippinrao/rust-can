"""Prepare small real-log fixtures from the ASC files under data/.

The source ASC files in this repository use a CANoe/Vector-like dialect where
CAN FD records are written as:

    <ts> CANFD <channel> <id> <Rx|Tx> <brs> <esi> d <dlc> <len> <bytes...>

The local python-can clone has been patched so ASCReader can read this CANFD
dialect directly. This script uses that patched ASCReader to generate CAN/CANFD
BLF fixtures and separately extracts LIN records as JSONL because python-can's
Message model does not represent LIN frames.
"""

from __future__ import annotations

import argparse
import json
import sys
import zipfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


@dataclass
class LinEvent:
    timestamp: float
    channel: str
    frame_id: int
    direction: str
    data: list[int]
    checksum: int | None


@dataclass
class FixtureStats:
    source_files: int = 0
    source_lines: int = 0
    can_messages: int = 0
    canfd_messages: int = 0
    lin_events: int = 0
    skipped_lines: int = 0
    errors: int = 0


def ensure_extracted(data_dir: Path, extracted_dir: Path) -> None:
    extracted_dir.mkdir(parents=True, exist_ok=True)
    for zip_path in sorted(data_dir.glob("*.zip")):
        target = extracted_dir / zip_path.stem
        if target.exists():
            continue
        target.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(zip_path) as archive:
            archive.extractall(target)


def parse_hex_int(token: str) -> tuple[int, bool]:
    extended = token.endswith(("x", "X"))
    if extended:
        token = token[:-1]
    return int(token, 16), extended


def parse_data(tokens: list[str], length: int) -> list[int]:
    return [int(token, 16) for token in tokens[:length]]


def parse_classic(tokens: list[str], can_module):
    timestamp = float(tokens[0])
    channel = int(tokens[1]) - 1
    arbitration_id, is_extended_id = parse_hex_int(tokens[2])
    direction = tokens[3]
    frame_kind = tokens[4].lower()
    dlc = int(tokens[5], 16)
    if frame_kind == "r":
        data: list[int] = []
        is_remote_frame = True
    else:
        data = parse_data(tokens[6:], min(dlc, 8))
        is_remote_frame = False
    return can_module.Message(
        timestamp=timestamp,
        arbitration_id=arbitration_id,
        is_extended_id=is_extended_id,
        is_rx=direction == "Rx",
        is_remote_frame=is_remote_frame,
        dlc=dlc if is_remote_frame else len(data),
        data=data,
        channel=channel,
    )


def parse_canfd(tokens: list[str], can_module):
    timestamp = float(tokens[0])
    channel = int(tokens[2]) - 1
    arbitration_id, is_extended_id = parse_hex_int(tokens[3])
    direction = tokens[4]
    brs = tokens[5] == "1"
    esi = tokens[6] == "1"
    dlc = int(tokens[8], 16)
    length = int(tokens[9])
    data = parse_data(tokens[10:], length)
    return can_module.Message(
        timestamp=timestamp,
        arbitration_id=arbitration_id,
        is_extended_id=is_extended_id,
        is_rx=direction == "Rx",
        is_fd=True,
        bitrate_switch=brs,
        error_state_indicator=esi,
        dlc=length if length else dlc,
        data=data,
        channel=channel,
    )


def parse_lin(tokens: list[str]) -> LinEvent:
    checksum = None
    if "checksum" in tokens:
        index = tokens.index("checksum")
        if index + 2 < len(tokens) and tokens[index + 1] == "=":
            checksum = int(tokens[index + 2], 16)
        payload_tokens = tokens[5:index]
    else:
        payload_tokens = tokens[5:]
    length = int(tokens[4])
    return LinEvent(
        timestamp=float(tokens[0]),
        channel=tokens[1],
        frame_id=int(tokens[2], 16),
        direction=tokens[3],
        data=parse_data(payload_tokens, length),
        checksum=checksum,
    )


def iter_asc_files(root: Path) -> Iterable[Path]:
    yield from sorted(root.rglob("*.asc"), key=lambda path: path.stat().st_size, reverse=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data-dir", type=Path, default=Path("data"))
    parser.add_argument("--extracted-dir", type=Path, default=Path("data/extracted"))
    parser.add_argument("--out-dir", type=Path, default=Path("data/generated"))
    parser.add_argument("--limit", type=int, default=10000)
    parser.add_argument("--lin-limit", type=int, default=1000)
    parser.add_argument("--extract", action="store_true")
    args = parser.parse_args()

    if args.extract:
        ensure_extracted(args.data_dir, args.extracted_dir)

    sys.path.insert(0, str(Path(".external/python-can").resolve()))
    import can  # type: ignore

    args.out_dir.mkdir(parents=True, exist_ok=True)
    blf_path = args.out_dir / f"real_can_canfd_{args.limit}.blf"
    lin_path = args.out_dir / f"real_lin_{args.lin_limit}.jsonl"
    stats_path = args.out_dir / "fixture_stats.json"

    stats = FixtureStats()
    asc_files = list(iter_asc_files(args.extracted_dir))
    stats.source_files = len(asc_files)

    writer = can.BLFWriter(str(blf_path))
    try:
        for asc_path in asc_files:
            with can.ASCReader(str(asc_path)) as reader:
                for msg in reader:
                    if stats.can_messages + stats.canfd_messages >= args.limit:
                        break
                    writer.on_message_received(msg)
                    if msg.is_fd:
                        stats.canfd_messages += 1
                    else:
                        stats.can_messages += 1
            if stats.can_messages + stats.canfd_messages >= args.limit:
                break
    finally:
        writer.stop()

    with lin_path.open("w", encoding="utf-8") as lin_out:
        for asc_path in asc_files:
            with asc_path.open("r", encoding="utf-8", errors="replace") as asc_file:
                for line in asc_file:
                    stats.source_lines += 1
                    tokens = line.split()
                    if len(tokens) >= 6 and tokens[1].startswith("L"):
                        try:
                            lin_out.write(json.dumps(asdict(parse_lin(tokens))) + "\n")
                            stats.lin_events += 1
                        except Exception:
                            stats.errors += 1
                    else:
                        stats.skipped_lines += 1
                    if stats.lin_events >= args.lin_limit:
                        break
            if stats.lin_events >= args.lin_limit:
                break

    stats_path.write_text(json.dumps(asdict(stats), indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"blf": str(blf_path), "lin": str(lin_path), "stats": asdict(stats)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
