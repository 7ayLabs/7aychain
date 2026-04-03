#!/usr/bin/env python3
"""Publish deterministic external scan data for the devnet bridge."""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path


def parse_device(entry: str) -> dict:
    parts = [part.strip() for part in entry.split(",")]
    if len(parts) < 3:
        raise ValueError(
            "device entries must be MAC_HASH,RSSI,SIGNAL_TYPE[,DEVICE_TYPE[,DEVICE_NAME[,FREQUENCY]]]"
        )

    device = {
        "mac_hash": parts[0],
        "rssi": int(parts[1]),
        "signal_type": parts[2],
    }

    if len(parts) >= 4 and parts[3]:
        device["device_type"] = parts[3]
    if len(parts) >= 5 and parts[4]:
        device["device_name"] = parts[4]
    if len(parts) >= 6 and parts[5]:
        device["frequency"] = int(parts[5])

    return device


def build_sample_devices() -> list[dict]:
    return [
        {
            "mac_hash": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "rssi": -41,
            "signal_type": "wifi",
            "device_type": "iphone",
            "device_name": "alice-phone",
            "frequency": 2412,
        },
        {
            "mac_hash": "0x2222222222222222222222222222222222222222222222222222222222222222",
            "rssi": -58,
            "signal_type": "ble",
            "device_type": "airpods",
            "device_name": "alice-audio",
        },
        {
            "mac_hash": "0x3333333333333333333333333333333333333333333333333333333333333333",
            "rssi": -66,
            "signal_type": "wifi",
            "device_type": "macbook",
            "device_name": "alice-laptop",
            "frequency": 5180,
        },
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        default="devnet/state/alice-scan.json",
        help="Path to the external scan JSON file.",
    )
    parser.add_argument(
        "--device",
        action="append",
        default=[],
        help="Device tuple: MAC_HASH,RSSI,SIGNAL_TYPE[,DEVICE_TYPE[,DEVICE_NAME[,FREQUENCY]]]",
    )
    parser.add_argument(
        "--sample",
        action="store_true",
        help="Write a deterministic sample payload for smoke testing.",
    )
    parser.add_argument(
        "--clear",
        action="store_true",
        help="Write an empty device list to disable scan ingestion.",
    )
    args = parser.parse_args()

    devices = [parse_device(device) for device in args.device]
    if args.sample and not devices:
        devices = build_sample_devices()
    if args.clear:
        devices = []

    payload = {
        "generated_at": int(time.time()),
        "devices": devices,
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    print(f"Wrote {len(devices)} device(s) to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
