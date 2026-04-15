#!/usr/bin/env python3
"""Devnet-only mock SM-DP+ / eSIM provisioning server.

This is a pragmatic local-testing scaffold for the 7AY telecom branch. It:
  - consumes the generated `devnet/telecom/esim/esim_profiles.json` artifact
  - exposes mock SM-DP+ style HTTP endpoints for profile discovery/status
  - serves the generated QR images and activation payloads
  - renders a lightweight HTML dashboard for manual browser testing

It does not implement the GSMA RSP protocol, real carrier provisioning,
Apple carrier bundles, or any baseband-level eSIM activation flow.
"""

from __future__ import annotations

import argparse
import html
import json
import mimetypes
import os
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from string import Template
from typing import Any
from urllib.parse import unquote, urlparse


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PROFILES_JSON = REPO_ROOT / "devnet" / "telecom" / "esim" / "esim_profiles.json"
DEFAULT_PROFILES_DIR = REPO_ROOT / "devnet" / "telecom" / "esim"
DEFAULT_TEMPLATE_DIR = REPO_ROOT / "devnet" / "telecom" / "provisioning" / "templates"
DEFAULT_CONFIG = REPO_ROOT / "devnet" / "telecom" / "provisioning" / "config.example.json"

ACTIVE_STATUSES = {"Activated", "Recovered"}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def safe_load_json(path: Path) -> dict[str, Any]:
    try:
        data = load_json(path)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}
    return data if isinstance(data, dict) else {}


def read_text(path: Path, fallback: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return fallback


def resolve_path(value: str | os.PathLike[str], base_dir: Path) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    return (base_dir / path).resolve()


def merge_configs(*configs: dict[str, Any]) -> dict[str, Any]:
    merged: dict[str, Any] = {}
    for config in configs:
        for key, value in config.items():
            if value is not None:
                merged[key] = value
    return merged


def normalize_profiles(manifest: Any, profiles_dir: Path, base_url: str) -> list[dict[str, Any]]:
    profiles: list[dict[str, Any]] = []
    raw_profiles = []
    if isinstance(manifest, dict):
        maybe_profiles = manifest.get("profiles", [])
        if isinstance(maybe_profiles, list):
            raw_profiles = maybe_profiles
    elif isinstance(manifest, list):
        raw_profiles = manifest

    for item in raw_profiles:
        if not isinstance(item, dict):
            continue

        number_id = str(item.get("number_id", "")).strip()
        activation_code = str(item.get("activation_code", "")).strip()
        dial_number = str(item.get("dial_number", "")).strip()
        qr_filename = str(item.get("qr_filename", "")).strip()
        qr_path = item.get("qr_path")
        qr_rel = str(qr_path).strip() if qr_path else f"qrs/{qr_filename}" if qr_filename else ""
        qr_file = (profiles_dir / qr_rel).resolve() if qr_rel else None

        profile = {
            "number_id": number_id,
            "dial_number": dial_number,
            "display_name": item.get("display_name", dial_number),
            "owner": item.get("owner"),
            "device_id": item.get("device_id"),
            "region_id": item.get("region_id"),
            "status": item.get("status"),
            "activation_epoch": item.get("activation_epoch"),
            "provisioning_state": item.get("provisioning_state", "None"),
            "provisioning_receipt": item.get("provisioning_receipt"),
            "sip_username": item.get("sip_username"),
            "provider_name": item.get("provider_name", "7AYchain Devnet"),
            "profile_label": item.get("profile_label", dial_number),
            "smdp_plus": item.get("smdp_plus"),
            "activation_code": activation_code,
            "confirmation_code": item.get("confirmation_code", ""),
            "lpa_payload": item.get("lpa_payload"),
            "qr_filename": qr_filename,
            "qr_path": qr_rel,
            "qr_url": f"{base_url.rstrip('/')}/api/v1/profiles/{number_id or activation_code}/qr",
            "detail_url": f"{base_url.rstrip('/')}/api/v1/profiles/{number_id or activation_code}",
            "activation_url": f"{base_url.rstrip('/')}/api/v1/profiles/{number_id or activation_code}/activation",
        }

        if qr_file and qr_file.exists():
            profile["qr_file"] = str(qr_file)

        profiles.append(profile)

    return profiles


def load_state(profiles_json: Path, profiles_dir: Path, base_url: str) -> dict[str, Any]:
    manifest = safe_load_json(profiles_json)
    profiles = normalize_profiles(manifest, profiles_dir, base_url)
    active_profiles = [profile for profile in profiles if profile.get("status") in ACTIVE_STATUSES]
    by_key: dict[str, dict[str, Any]] = {}
    for profile in profiles:
        for key in (
            profile.get("number_id"),
            profile.get("activation_code"),
            profile.get("dial_number"),
            profile.get("qr_filename"),
        ):
            if key:
                by_key[str(key)] = profile

    summary: dict[str, Any] = {}
    if isinstance(manifest, dict):
        summary.update(manifest.get("summary", {}))
    summary.setdefault("profile_count", len(profiles))
    summary.setdefault("active_profile_count", len(active_profiles))
    summary.setdefault("inactive_profile_count", max(len(profiles) - len(active_profiles), 0))

    return {
        "generated_at": manifest.get("generated_at") if isinstance(manifest, dict) else None,
        "summary": summary,
        "profiles": profiles,
        "active_profiles": active_profiles,
        "by_key": by_key,
        "source": {
            "profiles_json": str(profiles_json),
            "profiles_dir": str(profiles_dir),
        },
    }


def render_template(
    template_path: Path,
    fallback: str,
    *,
    raw_keys: set[str] | frozenset[str] | None = None,
    **values: Any,
) -> str:
    template = Template(read_text(template_path, fallback))
    raw_keys = raw_keys or set()
    rendered: dict[str, str] = {}
    for key, value in values.items():
        rendered[key] = str(value) if key in raw_keys else html.escape(str(value))
    return template.safe_substitute(rendered)


def build_profile_card(profile: dict[str, Any]) -> str:
    status = html.escape(str(profile.get("status", "")))
    title = html.escape(str(profile.get("display_name", profile.get("dial_number", ""))))
    dial_number = html.escape(str(profile.get("dial_number", "")))
    lpa_payload = html.escape(str(profile.get("lpa_payload", "")))
    return f"""
    <article class="card">
      <div class="meta">
        <h2>{title}</h2>
        <p class="status">{status}</p>
        <p><strong>Dial Number:</strong> {dial_number}</p>
        <p><strong>SM-DP+:</strong> {html.escape(str(profile.get("smdp_plus", "")))}</p>
        <p><strong>Activation Code:</strong> {html.escape(str(profile.get("activation_code", "")))}</p>
        <p><strong>LPA Payload:</strong></p>
        <pre>{lpa_payload}</pre>
        <p class="links">
          <a href="{html.escape(str(profile.get("detail_url", "#")))}">JSON detail</a>
          <a href="{html.escape(str(profile.get("activation_url", "#")))}">Activation package</a>
          <a href="{html.escape(str(profile.get("qr_url", "#")))}">QR asset</a>
        </p>
      </div>
      <div class="qr">
        <img src="{html.escape(str(profile.get("qr_url", "")))}" alt="QR for {title}" />
      </div>
    </article>
    """


def render_index(state: dict[str, Any], template_dir: Path, base_url: str) -> str:
    cards = "\n".join(build_profile_card(profile) for profile in state["profiles"])
    if not cards:
        cards = '<p class="empty">No eSIM profiles were found.</p>'

    fallback = """<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>$title</title>
  <style>
    :root {
      --bg: #0f172a;
      --panel: #111827;
      --panel-soft: #1f2937;
      --ink: #e5eef8;
      --muted: #9ca3af;
      --accent: #38bdf8;
      --line: #334155;
    }
    body {
      margin: 0;
      padding: 2rem;
      font-family: "Inter", "IBM Plex Sans", "Helvetica Neue", sans-serif;
      background:
        radial-gradient(circle at top left, rgba(56, 189, 248, 0.18), transparent 30%),
        radial-gradient(circle at top right, rgba(16, 185, 129, 0.12), transparent 24%),
        var(--bg);
      color: var(--ink);
    }
    h1 { margin: 0 0 0.5rem; }
    .subtle { color: var(--muted); }
    .stats {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 1rem;
      margin: 1.5rem 0;
    }
    .stat, .card {
      border: 1px solid var(--line);
      background: linear-gradient(180deg, rgba(255,255,255,0.03), rgba(255,255,255,0.01));
      border-radius: 18px;
      box-shadow: 0 16px 40px rgba(0,0,0,0.18);
    }
    .stat { padding: 1rem 1.2rem; }
    .stat .value { font-size: 1.6rem; font-weight: 700; }
    .card {
      display: grid;
      grid-template-columns: minmax(0, 1.8fr) minmax(220px, 280px);
      gap: 1.25rem;
      margin: 1rem 0;
      padding: 1.25rem;
      background: linear-gradient(180deg, rgba(31,41,55,0.95), rgba(17,24,39,0.95));
    }
    .status { color: var(--accent); font-weight: 700; }
    .links a {
      display: inline-block;
      margin-right: 0.75rem;
      color: var(--accent);
      text-decoration: none;
    }
    .qr {
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .qr img {
      width: 240px;
      height: 240px;
      padding: 0.5rem;
      border-radius: 18px;
      background: white;
    }
    pre {
      white-space: pre-wrap;
      word-break: break-all;
      padding: 0.75rem;
      border-radius: 12px;
      background: rgba(15, 23, 42, 0.7);
      border: 1px solid var(--line);
      color: #dbeafe;
    }
    .empty {
      padding: 1rem 1.2rem;
      border: 1px dashed var(--line);
      border-radius: 14px;
      color: var(--muted);
    }
    @media (max-width: 760px) {
      body { padding: 1rem; }
      .card { grid-template-columns: 1fr; }
      .qr img { width: 100%; max-width: 260px; height: auto; }
    }
  </style>
</head>
<body>
  <h1>$title</h1>
  <p class="subtle">$subtitle</p>
  <div class="stats">
    <div class="stat"><div class="label">Profiles</div><div class="value">$profile_count</div></div>
    <div class="stat"><div class="label">Active</div><div class="value">$active_count</div></div>
    <div class="stat"><div class="label">Source</div><div class="value">$source_path</div></div>
    <div class="stat"><div class="label">Discovery</div><div class="value"><a href="$discovery_url">$discovery_url</a></div></div>
  </div>
  $cards
</body>
</html>"""

    template_path = template_dir / "index.html"
    if template_path.exists():
        return render_template(
            template_path,
            fallback,
            raw_keys={"cards"},
            title="7AYchain Mock SM-DP+",
            subtitle=f"Devnet-only provisioning view at {base_url}.",
            profile_count=len(state["profiles"]),
            active_count=len(state["active_profiles"]),
            source_path=state["source"]["profiles_json"],
            discovery_url=f"{base_url.rstrip('/')}/api/v1/discovery",
            cards=cards,
        )

    return Template(fallback).safe_substitute(
        {
            "title": "7AYchain Mock SM-DP+",
            "subtitle": f"Devnet-only provisioning view at {html.escape(base_url)}.",
            "profile_count": len(state["profiles"]),
            "active_count": len(state["active_profiles"]),
            "source_path": html.escape(state["source"]["profiles_json"]),
            "discovery_url": f"{base_url.rstrip('/')}/api/v1/discovery",
            "cards": cards,
        }
    )


def render_profile_page(profile: dict[str, Any], template_dir: Path) -> str:
    fallback = """<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>$title</title>
  <style>
    body {
      margin: 0;
      padding: 2rem;
      font-family: "Inter", "Helvetica Neue", sans-serif;
      background: #09111f;
      color: #ecf2ff;
    }
    .panel {
      display: grid;
      grid-template-columns: minmax(0, 1.4fr) minmax(220px, 280px);
      gap: 1.5rem;
      padding: 1.25rem;
      border-radius: 18px;
      background: #111827;
      border: 1px solid #243041;
    }
    pre {
      white-space: pre-wrap;
      word-break: break-word;
      padding: 0.75rem;
      border-radius: 12px;
      background: #0b1324;
      border: 1px solid #243041;
    }
    img {
      width: 240px;
      height: 240px;
      background: white;
      border-radius: 18px;
      padding: 0.5rem;
    }
    a { color: #7dd3fc; }
  </style>
</head>
<body>
  <p><a href="/">Back to index</a></p>
  <h1>$title</h1>
  <div class="panel">
    <div>
      <p><strong>Status:</strong> $status</p>
      <p><strong>Dial Number:</strong> $dial_number</p>
      <p><strong>SM-DP+:</strong> $smdp_plus</p>
      <p><strong>Activation Code:</strong> $activation_code</p>
      <p><strong>LPA Payload:</strong></p>
      <pre>$lpa_payload</pre>
      <p><a href="$qr_url">QR asset</a> | <a href="$activation_url">Activation package</a></p>
    </div>
    <div>
      <img src="$qr_url" alt="QR for $title" />
    </div>
  </div>
</body>
</html>"""
    template_path = template_dir / "profile.html"
    return render_template(
        template_path,
        fallback,
        title=profile.get("display_name", profile.get("dial_number", "Profile")),
        status=profile.get("status", "Unknown"),
        dial_number=profile.get("dial_number", ""),
        smdp_plus=profile.get("smdp_plus", ""),
        activation_code=profile.get("activation_code", ""),
        lpa_payload=profile.get("lpa_payload", ""),
        qr_url=profile.get("qr_url", ""),
        activation_url=profile.get("activation_url", ""),
    )


def build_activation_package(profile: dict[str, Any], base_url: str) -> dict[str, Any]:
    return {
        "mock": True,
        "status": "ok",
        "generated_at": int(time.time()),
        "profile": {
            "number_id": profile.get("number_id"),
            "dial_number": profile.get("dial_number"),
            "display_name": profile.get("display_name"),
            "status": profile.get("status"),
            "provisioning_state": profile.get("provisioning_state"),
            "provisioning_receipt": profile.get("provisioning_receipt"),
            "smdp_plus": profile.get("smdp_plus"),
            "activation_code": profile.get("activation_code"),
            "confirmation_code": profile.get("confirmation_code"),
            "lpa_payload": profile.get("lpa_payload"),
        },
        "links": {
            "index": f"{base_url.rstrip('/')}/",
            "profile": profile.get("detail_url"),
            "qr": profile.get("qr_url"),
        },
        "notes": [
            "This is a mock SM-DP+ scaffold for devnet testing only.",
            "It exposes the generated LPA payload and status metadata, but it does not provision a real eSIM.",
        ],
    }


class MockSMDPHandler(BaseHTTPRequestHandler):
    server: "MockSMDPServer"

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A003
        print(f"[smdp] {self.address_string()} {format % args}")

    def _send(self, status: int, content_type: str, body: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def _send_json(self, data: Any, status: int = 200) -> None:
        self._send(status, "application/json", json.dumps(data, indent=2).encode("utf-8"))

    def _send_html(self, data: str, status: int = 200) -> None:
        self._send(status, "text/html; charset=utf-8", data.encode("utf-8"))

    def _send_text(self, data: str, status: int = 200, content_type: str = "text/plain; charset=utf-8") -> None:
        self._send(status, content_type, data.encode("utf-8"))

    def _current_state(self) -> dict[str, Any]:
        return load_state(self.server.profiles_json, self.server.profiles_dir, self.server.base_url)

    def _find_profile(self, key: str) -> dict[str, Any] | None:
        state = self._current_state()
        return state["by_key"].get(key)

    def _serve_profile_qr(self, profile: dict[str, Any]) -> None:
        qr_file = profile.get("qr_file")
        if qr_file:
            path = Path(qr_file)
            if path.exists() and path.is_file():
                mime = mimetypes.guess_type(path.name)[0] or "application/octet-stream"
                self._send(200, mime, path.read_bytes())
                return
        payload = str(profile.get("lpa_payload", ""))
        if payload:
            self._send_text(payload + "\n")
            return
        self._send_json({"error": "QR asset not available"}, 404)

    def do_GET(self) -> None:  # noqa: N802
        parsed = urlparse(self.path)
        path = unquote(parsed.path).rstrip("/") or "/"

        if path == "/":
            state = self._current_state()
            self._send_html(render_index(state, self.server.template_dir, self.server.base_url))
            return

        if path == "/health":
            self._send_json(self._current_state()["summary"])
            return

        if path == "/api/v1/health":
            state = self._current_state()
            self._send_json({
                "status": "ok",
                "base_url": self.server.base_url,
                "timestamp": int(__import__("time").time()),
                "profiles_count": len(state["profiles"]),
                "active_profiles_count": len(state["active_profiles"]),
                "source": state["source"],
                "summary": state["summary"],
            })
            return

        if path == "/api/v1/config":
            self._send_json({
                "bind": self.server.bind,
                "port": self.server.server_port,
                "base_url": self.server.base_url,
                "profiles_json": str(self.server.profiles_json),
                "profiles_dir": str(self.server.profiles_dir),
                "template_dir": str(self.server.template_dir),
            })
            return

        if path == "/api/v1/discovery":
            state = self._current_state()
            self._send_json({
                "mock": True,
                "smdp_plus": self.server.smdp_plus,
                "base_url": self.server.base_url,
                "profiles_endpoint": f"{self.server.base_url}/api/v1/profiles",
                "health_endpoint": f"{self.server.base_url}/api/v1/health",
                "profiles_count": len(state["profiles"]),
                "active_profiles_count": len(state["active_profiles"]),
                "note": "This is a local devnet discovery manifest, not a GSMA production endpoint.",
            })
            return

        if path == "/api/v1/profiles":
            state = self._current_state()
            self._send_json({
                "generated_at": state["generated_at"],
                "summary": state["summary"],
                "profiles": state["profiles"],
            })
            return

        if path.startswith("/api/v1/profiles/"):
            remainder = path[len("/api/v1/profiles/"):]
            if remainder.endswith("/qr"):
                key = remainder[:-3]
                profile = self._find_profile(key)
                if not profile:
                    self._send_json({"error": f"profile {key} not found"}, 404)
                    return
                self._serve_profile_qr(profile)
                return
            if remainder.endswith("/activation"):
                key = remainder[:-11]
                profile = self._find_profile(key)
                if not profile:
                    self._send_json({"error": f"profile {key} not found"}, 404)
                    return
                self._send_json(build_activation_package(profile, self.server.base_url))
                return
            if remainder.endswith("/page"):
                key = remainder[:-5]
                profile = self._find_profile(key)
                if not profile:
                    self._send_json({"error": f"profile {key} not found"}, 404)
                    return
                self._send_html(render_profile_page(profile, self.server.template_dir))
                return

            profile = self._find_profile(remainder)
            if not profile:
                self._send_json({"error": f"profile {remainder} not found"}, 404)
                return
            self._send_json(profile)
            return

        if path == "/api/v1/manifest":
            state = self._current_state()
            self._send_json({
                "generated_at": state["generated_at"],
                "summary": state["summary"],
                "source": state["source"],
                "active_profiles": state["active_profiles"],
                "profiles": state["profiles"],
            })
            return

        self._send_json({"error": "not found", "path": path}, 404)


class MockSMDPServer(ThreadingHTTPServer):
    bind: str = "0.0.0.0"
    base_url: str = "http://127.0.0.1:8091"
    profiles_json: Path
    profiles_dir: Path
    template_dir: Path
    smdp_plus: str = "smdp.7aychain.local"


def load_config(config_path: Path) -> dict[str, Any]:
    if not config_path.exists():
        return {}
    data = safe_load_json(config_path)
    if not data:
        return {}
    return data


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bind", default=None, help="Bind address.")
    parser.add_argument("--port", type=int, default=None, help="HTTP port.")
    parser.add_argument("--base-url", default=None, help="Base URL used in docs and JSON links.")
    parser.add_argument("--profiles-json", default=None, help="Path to esim_profiles.json.")
    parser.add_argument("--profiles-dir", default=None, help="Directory containing QR assets.")
    parser.add_argument("--template-dir", default=None, help="Directory with HTML templates.")
    parser.add_argument("--config", default=str(DEFAULT_CONFIG), help="Optional JSON config file.")
    parser.add_argument("--smdp-plus", default=None, help="Mock SM-DP+ host used in discovery metadata.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    config_path = Path(args.config).resolve()
    config = load_config(config_path)
    config_base = config_path.parent

    bind = str(args.bind or config.get("bind") or "0.0.0.0")
    port = int(args.port or config.get("port") or 8091)
    base_url = str(args.base_url or config.get("base_url") or f"http://127.0.0.1:{port}")
    profiles_json_value = args.profiles_json or config.get("profiles_json") or str(DEFAULT_PROFILES_JSON)
    profiles_dir_value = args.profiles_dir or config.get("profiles_dir") or str(DEFAULT_PROFILES_DIR)
    template_dir_value = args.template_dir or config.get("template_dir") or str(DEFAULT_TEMPLATE_DIR)
    smdp_plus = str(args.smdp_plus or config.get("smdp_plus") or "smdp.7aychain.local")

    profiles_json = resolve_path(profiles_json_value, config_base if args.profiles_json is None and config.get("profiles_json") else Path.cwd())
    profiles_dir = resolve_path(profiles_dir_value, config_base if args.profiles_dir is None and config.get("profiles_dir") else Path.cwd())
    template_dir = resolve_path(template_dir_value, config_base if args.template_dir is None and config.get("template_dir") else Path.cwd())

    server = MockSMDPServer((bind, port), MockSMDPHandler)
    server.bind = bind
    server.base_url = base_url.rstrip("/")
    server.profiles_json = profiles_json
    server.profiles_dir = profiles_dir
    server.template_dir = template_dir
    server.smdp_plus = smdp_plus

    state = load_state(profiles_json, profiles_dir, server.base_url)
    print("=== 7AY Mock SM-DP+ ===")
    print(f"Base URL:      {server.base_url}")
    print(f"Profiles JSON:  {profiles_json}")
    print(f"Profiles dir:   {profiles_dir}")
    print(f"Templates dir:  {template_dir}")
    print(f"Discovery:      {server.base_url}/api/v1/discovery")
    print(f"Profiles:       {len(state['profiles'])} total, {len(state['active_profiles'])} active")
    print(f"Health:         {server.base_url}/api/v1/health")
    print(f"Index:          {server.base_url}/")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
