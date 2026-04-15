#!/usr/bin/env python3
"""Lightweight HTTP provisioning API for 7AY carrier softphone auto-configuration.

Serves subscriber provisioning data, Linphone/Zoiper auto-config XML, health
status, and WebRTC configuration.

Endpoints:
  GET /api/v1/subscribers           - all active subscriber provisioning
  GET /api/v1/subscribers/<number_id> - single subscriber
  GET /api/v1/provision/<sip_user>  - Linphone/Zoiper auto-config XML
  GET /api/v1/health                - bridge health + stats
  GET /api/v1/webrtc-config/<user>  - WebRTC connection params
  GET /webrtc/                      - serve WebRTC dialer page
"""

from __future__ import annotations

import argparse
import json
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


ACTIVE_STATUSES = {"Activated", "Recovered"}


def load_json_safe(path: str | Path) -> Any:
    """Load JSON from file, returning empty dict on failure."""
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


class ProvisioningHandler(BaseHTTPRequestHandler):
    """HTTP request handler for the provisioning API."""

    def log_message(self, format: str, *args: Any) -> None:
        """Minimal logging."""
        print(f"[provisioning] {args[0]} {args[1]} {args[2]}")

    def _send_json(self, data: Any, status: int = 200) -> None:
        body = json.dumps(data, indent=2).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def _send_xml(self, xml: str, status: int = 200) -> None:
        body = xml.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/xml")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _send_html(self, html: str, status: int = 200) -> None:
        body = html.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _send_404(self, msg: str = "Not found") -> None:
        self._send_json({"error": msg}, 404)

    def _get_bridge_data(self) -> dict[str, Any]:
        return load_json_safe(self.server.bridge_json)

    def _get_catalog_data(self) -> dict[str, Any]:
        return load_json_safe(self.server.catalog_json)

    def _active_subscribers(self) -> list[dict[str, Any]]:
        """Get active subscribers with catalog metadata merged."""
        bridge = self._get_bridge_data()
        catalog = self._get_catalog_data()
        result = []
        for sub in bridge.get("subscribers", []):
            if not isinstance(sub, dict):
                continue
            if sub.get("status") not in ACTIVE_STATUSES:
                continue
            if sub.get("provisioning_state") == "Failed":
                continue
            cat = sub.get("catalog", {})
            if not cat:
                nid = sub.get("number_id", "")
                cat = catalog.get(nid, {})
            if not cat.get("sip_username"):
                continue
            result.append({
                "number_id": sub.get("number_id"),
                "dial_number": cat.get("dial_number"),
                "display_name": cat.get("display_name", cat.get("dial_number")),
                "sip_username": cat.get("sip_username"),
                "sip_password": cat.get("sip_password"),
                "sip_domain": self.server.sip_domain,
                "sip_port": self.server.sip_port,
                "transport": "udp",
                "status": sub.get("status"),
                "activation_epoch": sub.get("activation_epoch"),
                "owner": sub.get("owner"),
                "device_id": sub.get("device_id"),
                "region_id": sub.get("region_id"),
                "provisioning_state": sub.get("provisioning_state", "None"),
                "provisioning_receipt": sub.get("provisioning_receipt"),
                "smdp_plus": cat.get("esim", {}).get("smdp_plus"),
                "activation_code": cat.get("esim", {}).get("activation_code"),
                "confirmation_code": cat.get("esim", {}).get("confirmation_code"),
                "pstn_did": cat.get("pstn_did"),
                "signal_quality": sub.get("signal_quality"),
            })
        return result

    def _find_subscriber_by_username(self, username: str) -> dict[str, Any] | None:
        for sub in self._active_subscribers():
            if sub.get("sip_username") == username:
                return sub
        return None

    def _find_subscriber_by_id(self, number_id: str) -> dict[str, Any] | None:
        for sub in self._active_subscribers():
            if sub.get("number_id") == number_id:
                return sub
        return None

    def do_GET(self) -> None:  # noqa: N802
        path = urlparse(self.path).path.rstrip("/")

        if path == "/api/v1/subscribers":
            self._handle_subscribers()
        elif path.startswith("/api/v1/subscribers/"):
            number_id = path[len("/api/v1/subscribers/"):]
            self._handle_subscriber_detail(number_id)
        elif path.startswith("/api/v1/provision/"):
            username = path[len("/api/v1/provision/"):]
            self._handle_provision(username)
        elif path == "/api/v1/health":
            self._handle_health()
        elif path.startswith("/api/v1/webrtc-config/"):
            username = path[len("/api/v1/webrtc-config/"):]
            self._handle_webrtc_config(username)
        elif path in ("/webrtc", "/webrtc/index.html"):
            self._handle_webrtc_page()
        else:
            self._send_404()

    def _handle_subscribers(self) -> None:
        subs = self._active_subscribers()
        self._send_json({
            "generated_at": int(time.time()),
            "subscriber_count": len(subs),
            "subscribers": subs,
        })

    def _handle_subscriber_detail(self, number_id: str) -> None:
        sub = self._find_subscriber_by_id(number_id)
        if not sub:
            self._send_404(f"subscriber {number_id} not found")
            return
        self._send_json(sub)

    def _handle_provision(self, username: str) -> None:
        """Linphone/Zoiper auto-provisioning XML."""
        sub = self._find_subscriber_by_username(username)
        if not sub:
            self._send_404(f"subscriber {username} not found")
            return

        xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<config xmlns="http://www.linphone.org/xsds/lpconfig.xsd"
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        xsi:schemaLocation="http://www.linphone.org/xsds/lpconfig.xsd lpconfig.xsd">
  <section name="proxy_0">
    <entry name="reg_proxy" overwrite="true">sip:{sub['sip_domain']}:{sub['sip_port']}</entry>
    <entry name="reg_identity" overwrite="true">sip:{sub['sip_username']}@{sub['sip_domain']}</entry>
    <entry name="realm" overwrite="true">seveny-devnet</entry>
    <entry name="reg_sendregister" overwrite="true">1</entry>
    <entry name="publish" overwrite="true">0</entry>
  </section>
  <section name="auth_info_0">
    <entry name="username" overwrite="true">{sub['sip_username']}</entry>
    <entry name="passwd" overwrite="true">{sub['sip_password']}</entry>
    <entry name="realm" overwrite="true">seveny-devnet</entry>
  </section>
</config>"""
        self._send_xml(xml)

    def _handle_health(self) -> None:
        bridge = self._get_bridge_data()
        subs = self._active_subscribers()
        self._send_json({
            "status": "ok",
            "timestamp": int(time.time()),
            "bridge_generated_at": bridge.get("generated_at"),
            "active_subscriber_count": len(subs),
            "bridge_source": bridge.get("source", {}),
            "summary": bridge.get("summary", {}),
        })

    def _handle_webrtc_config(self, username: str) -> None:
        sub = self._find_subscriber_by_username(username)
        if not sub:
            self._send_404(f"subscriber {username} not found")
            return

        self._send_json({
            "sip_username": sub["sip_username"],
            "sip_password": sub["sip_password"],
            "ws_uri": f"wss://{sub['sip_domain']}:8089/ws",
            "sip_uri": f"sip:{sub['sip_username']}@{sub['sip_domain']}",
            "display_name": sub.get("display_name", sub["sip_username"]),
        })

    def _handle_webrtc_page(self) -> None:
        webrtc_path = Path("devnet/telecom/webrtc/index.html")
        if webrtc_path.exists():
            self._send_html(webrtc_path.read_text(encoding="utf-8"))
        else:
            self._send_404("WebRTC dialer page not found")


class ProvisioningServer(HTTPServer):
    """HTTP server with extra config attributes."""

    bridge_json: str = ""
    catalog_json: str = ""
    sip_domain: str = "127.0.0.1"
    sip_port: int = 5060


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--port", type=int, default=8090,
        help="HTTP port for the provisioning API.",
    )
    parser.add_argument(
        "--bridge-json",
        default="devnet/state/carrier-bridge.json",
        help="Path to carrier-bridge.json snapshot.",
    )
    parser.add_argument(
        "--catalog-json",
        default="devnet/state/carrier-catalog.json",
        help="Path to carrier-catalog.json.",
    )
    parser.add_argument(
        "--sip-domain",
        default="127.0.0.1",
        help="SIP domain for provisioning responses.",
    )
    parser.add_argument(
        "--sip-port", type=int, default=5060,
        help="SIP port for provisioning responses.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    server = ProvisioningServer(("0.0.0.0", args.port), ProvisioningHandler)
    server.bridge_json = args.bridge_json
    server.catalog_json = args.catalog_json
    server.sip_domain = args.sip_domain
    server.sip_port = args.sip_port

    print(f"Provisioning API listening on http://0.0.0.0:{args.port}")
    print(f"  Subscribers: http://localhost:{args.port}/api/v1/subscribers")
    print(f"  Health:      http://localhost:{args.port}/api/v1/health")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
