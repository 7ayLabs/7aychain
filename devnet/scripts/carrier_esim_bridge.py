#!/usr/bin/env python3
"""Generate decentralized eSIM enrollment artifacts from carrier state.

This bridge consumes the carrier snapshot and catalog metadata, then emits:
  - JSON eSIM profile data for active subscribers
  - LPA-style activation payloads
  - QR PNG files when `qrcode` is installed
  - an HTML gallery for scan-on-phone testing

It is a devnet provisioning artifact generator. It does not talk to a real
SM-DP+, Apple carrier bundle service, or a mobile radio network.
"""

from __future__ import annotations

import argparse
import html
import json
import time
from pathlib import Path
from typing import Any

try:
    import qrcode
except ImportError:  # pragma: no cover - dependency depends on environment
    qrcode = None


ACTIVE_STATUSES = {"Activated", "Recovered"}


def load_json(path: str | Path) -> Any:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def wait_for_snapshot(path: Path, interval: int) -> dict[str, Any]:
    announced = False
    while True:
        try:
            return load_json(path)
        except FileNotFoundError:
            if not announced:
                print(f"Waiting for carrier snapshot at {path} ...")
                announced = True
            time.sleep(max(interval, 1))
        except json.JSONDecodeError:
            if not announced:
                print(f"Waiting for valid carrier snapshot JSON at {path} ...")
                announced = True
            time.sleep(max(interval, 1))


def build_lpa_payload(esim: dict[str, Any]) -> str:
    smdp_plus = str(esim.get("smdp_plus", "")).strip()
    activation_code = str(esim.get("activation_code", "")).strip()
    confirmation_code = str(esim.get("confirmation_code", "")).strip()
    if not smdp_plus or not activation_code:
        return ""
    payload = f"LPA:1${smdp_plus}${activation_code}"
    if confirmation_code:
        payload += f"${confirmation_code}"
    return payload


def build_profile_entries(snapshot: dict[str, Any], qr_base_path: str) -> list[dict[str, Any]]:
    entries = []
    for subscriber in snapshot.get("subscribers", []):
        if not isinstance(subscriber, dict):
            continue
        if subscriber.get("status") not in ACTIVE_STATUSES:
            continue
        if subscriber.get("provisioning_state") == "Failed":
            continue
        catalog = subscriber.get("catalog", {})
        esim = catalog.get("esim", {})
        payload = build_lpa_payload(esim)
        if not payload:
            continue
        number_id = str(subscriber.get("number_id", ""))
        dial_number = catalog.get("dial_number")
        username = catalog.get("sip_username")
        qr_slug = number_id.removeprefix("0x")[:16] or "profile"
        qr_filename = f"esim-{qr_slug}.png"
        entries.append(
            {
                "number_id": number_id,
                "dial_number": dial_number,
                "display_name": catalog.get("display_name", dial_number),
                "owner": subscriber.get("owner"),
                "device_id": subscriber.get("device_id"),
                "region_id": subscriber.get("region_id"),
                "status": subscriber.get("status"),
                "activation_epoch": subscriber.get("activation_epoch"),
                "provisioning_state": subscriber.get("provisioning_state", "None"),
                "provisioning_receipt": subscriber.get("provisioning_receipt"),
                "sip_username": username,
                "provider_name": esim.get("provider_name", "7AYchain Devnet"),
                "profile_label": esim.get("profile_label", dial_number),
                "smdp_plus": esim.get("smdp_plus"),
                "activation_code": esim.get("activation_code"),
                "confirmation_code": esim.get("confirmation_code", ""),
                "lpa_payload": payload,
                "qr_filename": qr_filename,
                "qr_path": f"{qr_base_path.rstrip('/')}/{qr_filename}",
            }
        )
    return entries


def write_qr_image(payload: str, output_path: Path) -> bool:
    if qrcode is None:
        return False
    image = qrcode.make(payload)
    image.save(output_path)
    return True


def render_gallery(entries: list[dict[str, Any]], qr_available: bool) -> str:
    cards = []
    for entry in entries:
        title = html.escape(str(entry.get("display_name", entry.get("dial_number", ""))))
        provider = html.escape(str(entry.get("provider_name", "")))
        dial_number = html.escape(str(entry.get("dial_number", "")))
        qr_markup = (
            f'<img src="qrs/{html.escape(entry["qr_filename"])}" alt="QR for {title}" />'
            if qr_available
            else '<div class="qr-missing">Install qrcode to render PNG QR files.</div>'
        )
        cards.append(
            f"""
            <article class="card">
              <div class="meta">
                <h2>{title}</h2>
                <p class="provider">{provider}</p>
                <p><strong>Dial Number:</strong> {dial_number}</p>
                <p><strong>SM-DP+:</strong> {html.escape(str(entry.get("smdp_plus", "")))}</p>
                <p><strong>Activation Code:</strong> {html.escape(str(entry.get("activation_code", "")))}</p>
                <p><strong>LPA Payload:</strong></p>
                <pre>{html.escape(str(entry.get("lpa_payload", "")))}</pre>
              </div>
              <div class="qr">{qr_markup}</div>
            </article>
            """
        )

    body = "\n".join(cards) if cards else "<p>No active eSIM profiles available.</p>"
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>7AYchain eSIM Gallery</title>
  <style>
    :root {{
      --bg: #f4f1ea;
      --panel: #fffaf0;
      --ink: #1e2a26;
      --muted: #5b6b66;
      --accent: #0f766e;
      --line: #d9d2c3;
    }}
    body {{
      margin: 0;
      padding: 2rem;
      font-family: "IBM Plex Sans", "Helvetica Neue", sans-serif;
      background: radial-gradient(circle at top, #fffaf3, var(--bg));
      color: var(--ink);
    }}
    h1 {{ margin-top: 0; }}
    .card {{
      display: grid;
      grid-template-columns: minmax(0, 1.7fr) minmax(220px, 280px);
      gap: 1.5rem;
      margin: 1rem 0;
      padding: 1.25rem;
      border: 1px solid var(--line);
      background: var(--panel);
      border-radius: 20px;
    }}
    .provider {{ color: var(--accent); font-weight: 600; }}
    .qr {{ display: flex; align-items: center; justify-content: center; }}
    .qr img {{ width: 240px; height: 240px; background: white; padding: 0.5rem; border-radius: 18px; }}
    .qr-missing {{
      padding: 1rem;
      border-radius: 16px;
      border: 1px dashed var(--line);
      color: var(--muted);
      text-align: center;
    }}
    pre {{
      white-space: pre-wrap;
      word-break: break-all;
      padding: 0.75rem;
      border-radius: 12px;
      background: #f3efe7;
      border: 1px solid var(--line);
    }}
    @media (max-width: 720px) {{
      body {{ padding: 1rem; }}
      .card {{ grid-template-columns: 1fr; }}
      .qr img {{ width: 100%; max-width: 260px; height: auto; }}
    }}
  </style>
</head>
<body>
  <h1>7AYchain eSIM Gallery</h1>
  <p>Scan these LPA-style devnet payloads from your phone. This generates enrollment artifacts, not real carrier radio service.</p>
  {body}
</body>
</html>
"""


def write_outputs(output_dir: Path, profiles: list[dict[str, Any]]) -> int:
    output_dir.mkdir(parents=True, exist_ok=True)
    qrs_dir = output_dir / "qrs"
    qrs_dir.mkdir(parents=True, exist_ok=True)

    qr_written = 0
    for profile in profiles:
        qr_path = qrs_dir / profile["qr_filename"]
        if write_qr_image(profile["lpa_payload"], qr_path):
            qr_written += 1

    payload = {
        "generated_at": int(time.time()),
        "summary": {
            "profile_count": len(profiles),
            "qr_images_written": qr_written,
            "qr_supported": qrcode is not None,
        },
        "profiles": profiles,
    }
    (output_dir / "esim_profiles.json").write_text(
        json.dumps(payload, indent=2) + "\n",
        encoding="utf-8",
    )
    (output_dir / "index.html").write_text(
        render_gallery(profiles, qr_written > 0),
        encoding="utf-8",
    )
    for profile in profiles:
        (output_dir / f"{profile['qr_filename']}.txt").write_text(
            profile["lpa_payload"] + "\n",
            encoding="utf-8",
        )
    return qr_written


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--snapshot",
        default="devnet/state/carrier-bridge.json",
        help="Carrier snapshot JSON produced by carrier_bridge.py.",
    )
    parser.add_argument(
        "--output-dir",
        default="devnet/telecom/esim",
        help="Generated eSIM artifact directory.",
    )
    parser.add_argument(
        "--watch",
        action="store_true",
        help="Regenerate continuously.",
    )
    parser.add_argument(
        "--interval",
        type=int,
        default=5,
        help="Watch interval in seconds.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    snapshot_path = Path(args.snapshot)
    output_dir = Path(args.output_dir)

    def run_once() -> None:
        snapshot = wait_for_snapshot(snapshot_path, args.interval)
        profiles = build_profile_entries(snapshot, "qrs")
        qr_written = write_outputs(output_dir, profiles)
        print(
            f"Wrote {len(profiles)} eSIM profile(s) into {output_dir}"
            + ("" if qrcode is not None else " (install qrcode for PNG QR output)")
            + f"; QR images: {qr_written}"
        )

    run_once()
    if not args.watch:
        return 0

    while True:
        time.sleep(max(args.interval, 1))
        run_once()


if __name__ == "__main__":
    raise SystemExit(main())
