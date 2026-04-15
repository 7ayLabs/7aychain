#!/usr/bin/env python3
"""Project 7ay carrier state into a telecom-facing JSON snapshot.

This is a small off-chain scaffold for the native carrier plane. It does not
run SIP, IMS, eSIM, or LTE/5G core software. Instead, it watches the on-chain
Carrier pallet and emits a clean JSON control-plane view that a future telecom
gateway can consume.

Example:
  python3 devnet/scripts/carrier_bridge.py --once
  python3 devnet/scripts/carrier_bridge.py --watch --interval 7
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any

try:
    from substrateinterface import SubstrateInterface
except ImportError as exc:  # pragma: no cover - import failure is environment-specific
    raise SystemExit(
        "substrate-interface is required. Install it with "
        "`python3 -m pip install -r devnet/scripts/requirements.txt`."
    ) from exc


def to_plain(value: Any) -> Any:
    """Convert substrate-interface wrapper objects into plain Python values."""
    if hasattr(value, "value"):
        return to_plain(value.value)
    if isinstance(value, dict):
        return {str(k): to_plain(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [to_plain(item) for item in value]
    return value


def load_catalog(path: str | None) -> dict[str, dict[str, Any]]:
    """Load optional number metadata keyed by number_id."""
    if not path:
        return {}

    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    if isinstance(raw, dict):
        return {
            str(number_id): metadata
            for number_id, metadata in raw.items()
            if isinstance(metadata, dict)
        }

    if isinstance(raw, list):
        catalog = {}
        for item in raw:
            if isinstance(item, dict) and item.get("number_id"):
                catalog[str(item["number_id"])] = item
        return catalog

    return {}


def rpc_carrier_status(substrate: SubstrateInterface, number_id: str) -> dict[str, Any] | None:
    """Fetch runtime-API carrier status when the node exposes the RPC shim."""
    try:
        result = substrate.rpc_request("carrier_numberStatus", [number_id])
    except Exception:
        return None

    payload = result.get("result") if isinstance(result, dict) else None
    if not payload:
        return None
    payload = to_plain(payload)

    status = payload.get("status")
    if isinstance(status, dict):
        status = next(iter(status.keys()), status)

    return {
        "owner": payload.get("owner"),
        "device_id": payload.get("device_id", payload.get("deviceId")),
        "region_id": payload.get("region_id", payload.get("regionId")),
        "status": status,
        "activation_epoch": payload.get(
            "activation_epoch", payload.get("activationEpoch")
        ),
        "request_epoch": payload.get("request_epoch", payload.get("requestEpoch")),
        "service_lease_until": payload.get(
            "service_lease_until", payload.get("serviceLeaseUntil")
        ),
        "activated_at": payload.get("activated_at", payload.get("activatedAt")),
        "avg_signal_score": payload.get(
            "avg_signal_score", payload.get("avgSignalScore")
        ),
        "signal_sample_count": payload.get(
            "signal_sample_count", payload.get("signalSampleCount")
        ),
        "witness_count": payload.get("witness_count", payload.get("witnessCount")),
        "provisioning_state": payload.get(
            "provisioning_state", payload.get("provisioningState")
        ),
        "provisioning_receipt": payload.get(
            "provisioning_receipt", payload.get("provisioningReceipt")
        ),
        "sim_profile_commitment": payload.get(
            "sim_profile_commitment", payload.get("simProfileCommitment")
        ),
    }


def collect_numbers(
    substrate: SubstrateInterface,
    catalog: dict[str, dict[str, Any]],
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    """Collect all reserved carrier numbers and summarize states."""
    subscribers: list[dict[str, Any]] = []
    status_totals: dict[str, int] = {}

    for key_obj, value_obj in substrate.query_map("Carrier", "Numbers"):
        number_id = str(to_plain(key_obj))
        binding = to_plain(value_obj)
        binding_status = str(binding.get("status", "Unknown"))
        rpc_status = rpc_carrier_status(substrate, number_id) or {}
        status = rpc_status.get("status", binding_status)

        subscriber = {
            "number_id": number_id,
            "owner": rpc_status.get("owner", binding.get("owner")),
            "device_id": rpc_status.get("device_id", binding.get("current_device_id")),
            "region_id": rpc_status.get("region_id", binding.get("region_id")),
            "status": status,
            "activation_epoch": rpc_status.get(
                "activation_epoch", binding.get("activation_epoch")
            ),
            "reserved_at": binding.get("reserved_at"),
            "activated_at": rpc_status.get("activated_at", binding.get("activated_at")),
            "service_lease_until": rpc_status.get(
                "service_lease_until", binding.get("service_lease_until")
            ),
            "sim_profile_commitment": rpc_status.get(
                "sim_profile_commitment", binding.get("sim_profile_commitment")
            ),
            "avg_signal_score": rpc_status.get("avg_signal_score"),
            "signal_sample_count": rpc_status.get("signal_sample_count"),
            "witness_count": rpc_status.get("witness_count"),
            "provisioning_state": rpc_status.get(
                "provisioning_state", binding.get("provisioning_state", "None")
            ),
            "provisioning_receipt": rpc_status.get(
                "provisioning_receipt", binding.get("provisioning_receipt")
            ),
            "catalog": catalog.get(number_id, {}),
        }
        subscribers.append(subscriber)
        status_totals[status] = status_totals.get(status, 0) + 1

    subscribers.sort(key=lambda item: item["number_id"])
    return subscribers, status_totals


def collect_requests(substrate: SubstrateInterface) -> list[dict[str, Any]]:
    """Collect active carrier service requests keyed through CurrentRequestEpoch."""
    requests: list[dict[str, Any]] = []

    for key_obj, value_obj in substrate.query_map("Carrier", "CurrentRequestEpoch"):
        number_id = str(to_plain(key_obj))
        epoch = to_plain(value_obj)
        request = substrate.query("Carrier", "ServiceRequests", [epoch, number_id])
        request_value = to_plain(request)
        if not request_value:
            continue

        witness_count = substrate.query("Carrier", "ServiceWitnessCount", [epoch, number_id])
        request_value["number_id"] = number_id
        request_value["epoch"] = epoch
        request_value["witness_count_observed"] = to_plain(witness_count) or 0
        requests.append(request_value)

    requests.sort(key=lambda item: (item.get("epoch", 0), item.get("number_id", "")))
    return requests


def collect_witness_leaderboard(
    substrate: SubstrateInterface,
) -> list[dict[str, Any]]:
    """Collect validator witness counts for the leaderboard."""
    leaderboard: list[dict[str, Any]] = []
    try:
        for key_obj, value_obj in substrate.query_map("Carrier", "ValidatorWitnessCount"):
            validator_id = str(to_plain(key_obj))
            count = to_plain(value_obj) or 0
            if count > 0:
                leaderboard.append({"validator_id": validator_id, "witness_count": count})
    except Exception:
        pass
    leaderboard.sort(key=lambda x: x.get("witness_count", 0), reverse=True)
    return leaderboard


def build_snapshot(
    substrate: SubstrateInterface,
    catalog: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    """Assemble a telecom-facing snapshot of the carrier plane."""
    subscribers, status_totals = collect_numbers(substrate, catalog)
    requests = collect_requests(substrate)
    leaderboard = collect_witness_leaderboard(substrate)

    return {
        "generated_at": int(time.time()),
        "source": {
            "chain": substrate.rpc_request("system_chain", []).get("result"),
            "url": substrate.url,
        },
        "summary": {
            "subscriber_count": len(subscribers),
            "pending_request_count": len(requests),
            "status_totals": status_totals,
            "witness_leaderboard_size": len(leaderboard),
        },
        "subscribers": subscribers,
        "pending_requests": requests,
        "witness_leaderboard": leaderboard,
    }


def write_snapshot(output: Path, snapshot: dict[str, Any]) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(snapshot, indent=2) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--url",
        default="ws://127.0.0.1:9944",
        help="WebSocket endpoint for the local devnet node.",
    )
    parser.add_argument(
        "--output",
        default="devnet/state/carrier-bridge.json",
        help="Output JSON snapshot path.",
    )
    parser.add_argument(
        "--catalog",
        help="Optional JSON file with metadata keyed by number_id.",
    )
    parser.add_argument(
        "--interval",
        type=int,
        default=7,
        help="Polling interval in seconds when --watch is enabled.",
    )
    parser.add_argument(
        "--watch",
        action="store_true",
        help="Poll forever and refresh the output snapshot.",
    )
    parser.add_argument(
        "--once",
        action="store_true",
        help="Write a single snapshot and exit.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    catalog = load_catalog(args.catalog)
    output = Path(args.output)
    substrate = SubstrateInterface(url=args.url)

    def run_once() -> None:
        snapshot = build_snapshot(substrate, catalog)
        write_snapshot(output, snapshot)
        print(
            f"Wrote {snapshot['summary']['subscriber_count']} subscriber(s) and "
            f"{snapshot['summary']['pending_request_count']} request(s) to {output}"
        )

    run_once()
    if args.once or not args.watch:
        return 0

    while True:
        time.sleep(max(args.interval, 1))
        run_once()


if __name__ == "__main__":
    raise SystemExit(main())
