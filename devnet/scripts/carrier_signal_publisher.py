#!/usr/bin/env python3
"""Devnet helper: emit carrier-signal samples for a running 7aychain node.

Each running node has a `--carrier-signal-mode=file` configuration that
points it at a JSON file. This publisher writes that file on a tick,
modelling the local carrier signal (RSSI, SNR, jitter, packet loss) as
seen by that operator. One publisher per node = "each node, each person
on their carrier."

Usage:
    python3 carrier_signal_publisher.py \\
        --output /tmp/7ay-alice-signal.json \\
        --carrier-id 7AY-Alice \\
        --region 0x1111111111111111111111111111111111111111111111111111111111111111 \\
        --network LTE \\
        --interval 5

Pair with a node:
    seveny-node --dev \\
        --carrier-signal-mode file \\
        --carrier-signal-file /tmp/7ay-alice-signal.json \\
        --carrier-signal-interval 5
"""

from __future__ import annotations

import argparse
import json
import random
import signal
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass
class Sample:
    carrier_id: str
    region_id: str
    rssi_dbm: int
    snr_db: int
    bars: int
    network_type: str
    latency_ms: int
    jitter_ms: int
    packet_loss_pct: int
    captured_at: int


def rssi_to_bars(rssi: int) -> int:
    if rssi > -55:
        return 5
    if rssi > -70:
        return 4
    if rssi > -85:
        return 3
    if rssi > -100:
        return 2
    if rssi > -115:
        return 1
    return 0


def rssi_to_snr(rssi: int) -> int:
    if rssi > -60:
        return 30
    if rssi > -75:
        return 22
    if rssi > -90:
        return 14
    if rssi > -105:
        return 6
    return 0


def step_rssi(rng: random.Random, current: int) -> int:
    delta = rng.randint(-3, 3)
    nxt = current + delta
    if rng.randint(0, 19) == 0:
        # rare fade
        nxt -= 10
    return max(-110, min(-45, nxt))


def make_sample(rng: random.Random, args, rssi: int) -> Sample:
    bars = rssi_to_bars(rssi)
    snr = rssi_to_snr(rssi)
    latency = rng.randint(20, 80)
    jitter = rng.randint(0, 8)
    loss = rng.randint(1, 5) if rng.randint(0, 9) == 0 else 0
    return Sample(
        carrier_id=args.carrier_id,
        region_id=args.region,
        rssi_dbm=rssi,
        snr_db=snr,
        bars=bars,
        network_type=args.network,
        latency_ms=latency,
        jitter_ms=jitter,
        packet_loss_pct=loss,
        captured_at=int(time.time()),
    )


def write_atomic(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True,
                        help="Path the node reads (matches --carrier-signal-file)")
    parser.add_argument("--carrier-id", default="7AY-Test")
    parser.add_argument("--region",
                        default="0x" + "11" * 32,
                        help="32-byte region hex (with or without 0x prefix)")
    parser.add_argument("--network", default="LTE",
                        help="Network type label, e.g. LTE / 5G / 7AY-MESH")
    parser.add_argument("--interval", type=float, default=5.0,
                        help="Seconds between samples")
    parser.add_argument("--start-rssi", type=int, default=-72)
    parser.add_argument("--seed", type=int, default=None)
    parser.add_argument("--once", action="store_true",
                        help="Emit a single sample and exit")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rng = random.Random(args.seed) if args.seed is not None else random.Random()
    rssi = args.start_rssi
    if not args.region.startswith("0x"):
        args.region = "0x" + args.region

    stop = False

    def handle_signal(_signum, _frame):
        nonlocal stop
        stop = True

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    print(
        f"[carrier-signal-publisher] writing to {args.output} "
        f"every {args.interval}s as carrier={args.carrier_id} region={args.region[:14]}…",
        file=sys.stderr,
    )

    while not stop:
        rssi = step_rssi(rng, rssi)
        sample = make_sample(rng, args, rssi)
        write_atomic(args.output, asdict(sample))
        print(
            f"  seq @ {sample.captured_at}: rssi={sample.rssi_dbm}dBm "
            f"bars={sample.bars}/5 lat={sample.latency_ms}ms loss={sample.packet_loss_pct}%",
            file=sys.stderr,
        )
        if args.once:
            break
        time.sleep(args.interval)

    return 0


if __name__ == "__main__":
    sys.exit(main())
