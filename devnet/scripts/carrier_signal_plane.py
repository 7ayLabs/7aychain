#!/usr/bin/env python3
"""Devnet-only IMS and radio signal-plane scaffold for 7AY testing.

This script models a lean control/service plane for local testing:
  - IMS/SIP registration and call state
  - regional radio presence and coverage summaries
  - a simple HTTP endpoint for health and state inspection

It is intentionally not a real SM-DP+, IMS core, eNodeB/gNodeB, or PSTN
implementation. The goal is to let developers exercise the chain-adjacent
service logic that would sit beside 7AY regional node coverage.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


ROOT = Path("devnet/telecom")
IMS_DIR = ROOT / "ims"
RADIO_DIR = ROOT / "radio"
DEFAULT_IMS_CONFIG = IMS_DIR / "config.json"
DEFAULT_IMS_STATE = IMS_DIR / "state.json"
DEFAULT_RADIO_CONFIG = RADIO_DIR / "config.json"
DEFAULT_RADIO_STATE = RADIO_DIR / "state.json"

SAMPLE_IMS_CONFIG = {
    "service_name": "7AY IMS Devnet",
    "realm": "ims.7aychain.local",
    "sip_domain": "127.0.0.1",
    "sip_port": 5062,
    "registration_ttl_seconds": 3600,
    "routing_policy": "regional",
    "notes": [
        "Devnet-only IMS/SIP scaffold.",
        "Use this for browser or local softphone testing, not native carrier service.",
    ],
}

SAMPLE_RADIO_CONFIG = {
    "service_name": "7AY Radio Presence Plane",
    "coverage_thresholds": {"weak": 1, "available": 2, "strong": 4},
    "presence_window_blocks": 32,
    "regions": [
        {
            "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "name": "North Lab",
            "min_present_nodes": 2,
        },
        {
            "region_id": "0x2222222222222222222222222222222222222222222222222222222222222222",
            "name": "Central Lab",
            "min_present_nodes": 2,
        },
        {
            "region_id": "0x3333333333333333333333333333333333333333333333333333333333333333",
            "name": "South Lab",
            "min_present_nodes": 3,
        },
    ],
}

SAMPLE_IMS_STATE = {
    "generated_at": "2026-04-15T00:00:00Z",
    "registrations": [
        {
            "public_identity": "sip:alice@ims.7aychain.local",
            "imsi": "001011234567890",
            "msisdn": "+15550007001",
            "display_name": "Alice 7AY",
            "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "status": "Registered",
            "contact_uri": "sip:alice@192.168.100.10:5060",
            "expires_in_seconds": 3170,
        }
    ],
    "sessions": [
        {
            "call_id": "ims-call-7001",
            "from": "sip:alice@ims.7aychain.local",
            "to": "sip:bob@ims.7aychain.local",
            "state": "Established",
            "media": "audio",
            "path": ["ims-edge", "regional-anchor", "softphone"],
        }
    ],
}

SAMPLE_RADIO_STATE = {
    "generated_at": "2026-04-15T00:00:00Z",
    "regions": [
        {
            "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "name": "North Lab",
            "present_nodes": 3,
            "required_nodes": 2,
            "signal_state": "Available",
            "coverage_score": 87,
            "signal_bars": 4,
            "witnessed_by": ["validator-a", "validator-b", "validator-c"],
            "last_presence_epoch": 1,
        },
        {
            "region_id": "0x2222222222222222222222222222222222222222222222222222222222222222",
            "name": "Central Lab",
            "present_nodes": 2,
            "required_nodes": 2,
            "signal_state": "Available",
            "coverage_score": 76,
            "signal_bars": 3,
            "witnessed_by": ["validator-a", "validator-d"],
            "last_presence_epoch": 1,
        },
        {
            "region_id": "0x3333333333333333333333333333333333333333333333333333333333333333",
            "name": "South Lab",
            "present_nodes": 1,
            "required_nodes": 3,
            "signal_state": "Weak",
            "coverage_score": 34,
            "signal_bars": 1,
            "witnessed_by": ["validator-e"],
            "last_presence_epoch": 1,
        },
    ],
    "notes": [
        "Coverage is a chain-backed devnet approximation.",
        "Presence and service-node membership are treated as the input signal.",
    ],
}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def dump_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def ensure_sample_files(
    ims_config_path: Path,
    ims_state_path: Path,
    radio_config_path: Path,
    radio_state_path: Path,
) -> None:
    for path, data in (
        (ims_config_path, SAMPLE_IMS_CONFIG),
        (ims_state_path, SAMPLE_IMS_STATE),
        (radio_config_path, SAMPLE_RADIO_CONFIG),
        (radio_state_path, SAMPLE_RADIO_STATE),
    ):
        if not path.exists():
            dump_json(path, data)


@dataclass(frozen=True)
class PlaneSnapshot:
    ims_config: dict[str, Any]
    ims_state: dict[str, Any]
    radio_config: dict[str, Any]
    radio_state: dict[str, Any]

    def summary(self) -> dict[str, Any]:
        registrations = [entry for entry in self.ims_state.get("registrations", []) if isinstance(entry, dict)]
        sessions = [entry for entry in self.ims_state.get("sessions", []) if isinstance(entry, dict)]
        regions = [entry for entry in self.radio_state.get("regions", []) if isinstance(entry, dict)]

        coverage_by_region = {}
        for region in regions:
            region_id = region.get("region_id")
            present_nodes = int(region.get("present_nodes", 0) or 0)
            required_nodes = int(region.get("required_nodes", 0) or 0)
            if present_nodes >= 4:
                coverage_level = "Strong"
            elif present_nodes >= max(required_nodes, 2):
                coverage_level = "Available"
            elif present_nodes >= 1:
                coverage_level = "Weak"
            else:
                coverage_level = "Unavailable"
            coverage_by_region[region_id] = {
                "name": region.get("name"),
                "present_nodes": present_nodes,
                "required_nodes": required_nodes,
                "signal_state": region.get("signal_state", coverage_level),
                "coverage_score": int(region.get("coverage_score", 0) or 0),
                "signal_bars": int(region.get("signal_bars", 0) or 0),
                "coverage_level": coverage_level,
                "witnessed_by": region.get("witnessed_by", []),
                "last_presence_epoch": region.get("last_presence_epoch"),
            }

        ready_regions = [
            region_id
            for region_id, details in coverage_by_region.items()
            if details["coverage_level"] in {"Available", "Strong"}
        ]

        return {
            "service_name": self.ims_config.get("service_name", "7AY IMS Devnet"),
            "generated_at": self.ims_state.get("generated_at") or self.radio_state.get("generated_at"),
            "ims": {
                "realm": self.ims_config.get("realm"),
                "sip_domain": self.ims_config.get("sip_domain"),
                "sip_port": self.ims_config.get("sip_port"),
                "registrations": len(registrations),
                "sessions": len(sessions),
            },
            "radio": {
                "regions": len(regions),
                "ready_regions": len(ready_regions),
                "ready_region_ids": ready_regions,
                "coverage_by_region": coverage_by_region,
            },
            "7ay_line_ready": bool(registrations) and bool(ready_regions),
            "notes": [
                "This is a devnet-only approximation of IMS + radio signal gating.",
                "A real carrier radio stack still requires SM-DP+, IMS core, and RAN/PSTN integration.",
            ],
        }


def load_snapshot(args: argparse.Namespace) -> PlaneSnapshot:
    ims_config_path = Path(args.ims_config)
    ims_state_path = Path(args.ims_state)
    radio_config_path = Path(args.radio_config)
    radio_state_path = Path(args.radio_state)

    if args.sample:
        ensure_sample_files(ims_config_path, ims_state_path, radio_config_path, radio_state_path)

    ims_config = load_json(ims_config_path) if ims_config_path.exists() else SAMPLE_IMS_CONFIG
    ims_state = load_json(ims_state_path) if ims_state_path.exists() else SAMPLE_IMS_STATE
    radio_config = load_json(radio_config_path) if radio_config_path.exists() else SAMPLE_RADIO_CONFIG
    radio_state = load_json(radio_state_path) if radio_state_path.exists() else SAMPLE_RADIO_STATE

    return PlaneSnapshot(
        ims_config=ims_config,
        ims_state=ims_state,
        radio_config=radio_config,
        radio_state=radio_state,
    )


class SignalPlaneHandler(BaseHTTPRequestHandler):
    snapshot: PlaneSnapshot | None = None

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A003
        return

    def _send_json(self, status: int, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, indent=2, sort_keys=True).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802
        if self.snapshot is None:
            self._send_json(500, {"error": "signal plane not initialized"})
            return

        if self.path in {"/", "/health"}:
            self._send_json(200, self.snapshot.summary())
            return
        if self.path == "/api/v1/state":
            self._send_json(200, {
                "ims_config": self.snapshot.ims_config,
                "ims_state": self.snapshot.ims_state,
                "radio_config": self.snapshot.radio_config,
                "radio_state": self.snapshot.radio_state,
            })
            return
        if self.path == "/api/v1/ims/registrations":
            registrations = self.snapshot.ims_state.get("registrations", [])
            self._send_json(200, {"registrations": registrations})
            return
        if self.path == "/api/v1/ims/sessions":
            sessions = self.snapshot.ims_state.get("sessions", [])
            self._send_json(200, {"sessions": sessions})
            return
        if self.path == "/api/v1/radio/regions":
            regions = self.snapshot.radio_state.get("regions", [])
            self._send_json(200, {"regions": regions})
            return
        if self.path == "/api/v1/radio/coverage":
            self._send_json(200, self.snapshot.summary()["radio"])
            return

        self._send_json(404, {"error": "not found", "path": self.path})


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the devnet IMS/radio signal plane")
    parser.add_argument("--ims-config", default=str(DEFAULT_IMS_CONFIG))
    parser.add_argument("--ims-state", default=str(DEFAULT_IMS_STATE))
    parser.add_argument("--radio-config", default=str(DEFAULT_RADIO_CONFIG))
    parser.add_argument("--radio-state", default=str(DEFAULT_RADIO_STATE))
    parser.add_argument("--bind", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8188)
    parser.add_argument("--sample", action="store_true", help="seed missing config/state files with sample data")
    parser.add_argument("--once", action="store_true", help="print a single JSON summary and exit")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    snapshot = load_snapshot(args)

    if args.once:
        print(json.dumps(snapshot.summary(), indent=2, sort_keys=True))
        return 0

    SignalPlaneHandler.snapshot = snapshot
    server = ThreadingHTTPServer((args.bind, args.port), SignalPlaneHandler)
    print(f"7AY IMS/radio signal plane listening on http://{args.bind}:{args.port}")
    print(f"Health: http://{args.bind}:{args.port}/health")
    print(f"State:  http://{args.bind}:{args.port}/api/v1/state")
    print(f"IMS:    http://{args.bind}:{args.port}/api/v1/ims/registrations")
    print(f"Radio:  http://{args.bind}:{args.port}/api/v1/radio/coverage")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
