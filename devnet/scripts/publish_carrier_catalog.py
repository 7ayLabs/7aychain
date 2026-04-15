#!/usr/bin/env python3
"""Write a carrier subscriber catalog for the softphone and eSIM bridges.

The catalog gives human-facing telecom metadata to chain-native `number_id`
values. It is intentionally off-chain so operators can control SIP usernames,
passwords, display names, and eSIM enrollment metadata without changing the
chain state model.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def parse_entry(
    entry: str,
    smdp_plus: str = "smdp.7aychain.local",
    provider_name: str = "7AYchain Devnet",
) -> dict:
    parts = [part.strip() for part in entry.split(",")]
    if len(parts) < 4:
        raise ValueError(
            "entries must be NUMBER_ID,DIAL_NUMBER,SIP_USERNAME,SIP_PASSWORD"
            "[,DISPLAY_NAME][,PSTN_DID]"
        )

    number_id = parts[0]
    dial_number = parts[1]
    display_name = parts[4] if len(parts) >= 5 and parts[4] else dial_number
    pstn_did = parts[5] if len(parts) >= 6 and parts[5] else None
    record: dict = {
        "number_id": number_id,
        "dial_number": dial_number,
        "sip_username": parts[2],
        "sip_password": parts[3],
        "display_name": display_name,
        "esim": {
            "provider_name": provider_name,
            "profile_label": display_name,
            "smdp_plus": smdp_plus,
            "activation_code": f"7AY-{dial_number}-{number_id[-6:]}",
            "confirmation_code": "",
        },
    }
    if pstn_did:
        record["pstn_did"] = pstn_did
    return record


def build_sample(
    trunk_config: str | None = None,
    smdp_plus: str = "smdp.7aychain.local",
    provider_name: str = "7AYchain Devnet",
) -> list[dict]:
    """Build sample catalog entries, optionally with PSTN DIDs from trunk config."""
    dids: list[dict] = []
    if trunk_config:
        try:
            tc = json.loads(Path(trunk_config).read_text(encoding="utf-8"))
            dids = tc.get("dids", [])
        except (FileNotFoundError, json.JSONDecodeError):
            pass

    alice_did = dids[0]["e164"] if len(dids) > 0 else None
    bob_did = dids[1]["e164"] if len(dids) > 1 else None

    entries = [
        {
            "number_id": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "dial_number": "7001",
            "sip_username": "alice7001",
            "sip_password": "7ay-alice-7001",
            "display_name": "Alice 7AY",
            "esim": {
                "provider_name": provider_name,
                "profile_label": "Alice 7AY",
                "smdp_plus": smdp_plus,
                "activation_code": "7AY-ACT-7001-ALICE",
                "confirmation_code": "",
            },
        },
        {
            "number_id": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "dial_number": "7002",
            "sip_username": "bob7002",
            "sip_password": "7ay-bob-7002",
            "display_name": "Bob 7AY",
            "esim": {
                "provider_name": provider_name,
                "profile_label": "Bob 7AY",
                "smdp_plus": smdp_plus,
                "activation_code": "7AY-ACT-7002-BOB",
                "confirmation_code": "",
            },
        },
    ]

    if alice_did:
        entries[0]["pstn_did"] = alice_did
    if bob_did:
        entries[1]["pstn_did"] = bob_did

    return entries


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        default="devnet/state/carrier-catalog.json",
        help="Path to the catalog JSON file.",
    )
    parser.add_argument(
        "--entry",
        action="append",
        default=[],
        help="NUMBER_ID,DIAL_NUMBER,SIP_USERNAME,SIP_PASSWORD[,DISPLAY_NAME][,PSTN_DID]",
    )
    parser.add_argument(
        "--sample",
        action="store_true",
        help="Write a deterministic sample carrier catalog.",
    )
    parser.add_argument(
        "--trunk-config",
        default=None,
        help="Path to SIP trunk config JSON (used with --sample to populate DIDs).",
    )
    parser.add_argument(
        "--smdp-plus",
        default="smdp.7aychain.local",
        help="SM-DP+ host:port or hostname written into generated eSIM metadata.",
    )
    parser.add_argument(
        "--provider-name",
        default="7AYchain Devnet",
        help="Provider label written into generated eSIM metadata.",
    )
    args = parser.parse_args()

    records = [
        parse_entry(
            entry,
            smdp_plus=args.smdp_plus,
            provider_name=args.provider_name,
        )
        for entry in args.entry
    ]
    if args.sample and not records:
        records = build_sample(
            args.trunk_config,
            smdp_plus=args.smdp_plus,
            provider_name=args.provider_name,
        )

    payload = {record["number_id"]: record for record in records}
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(records)} catalog entrie(s) to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
