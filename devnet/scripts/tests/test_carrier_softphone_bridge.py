"""Tests for the carrier softphone bridge scaffolding."""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import carrier_softphone_bridge as bridge


def sample_snapshot():
    return {
        "subscribers": [
            {
                "number_id": "0xaaa",
                "owner": "0xactor1",
                "device_id": 0,
                "region_id": "0xregion",
                "status": "Activated",
                "provisioning_state": "Pending",
                "catalog": {
                    "dial_number": "7001",
                    "sip_username": "alice7001",
                    "sip_password": "secret1",
                    "display_name": "Alice 7AY",
                },
            },
            {
                "number_id": "0xbbb",
                "owner": "0xactor2",
                "device_id": 1,
                "region_id": "0xregion",
                "status": "Suspended",
                "provisioning_state": "Provisioned",
                "catalog": {
                    "dial_number": "7002",
                    "sip_username": "bob7002",
                    "sip_password": "secret2",
                    "display_name": "Bob 7AY",
                },
            },
        ]
    }


def test_build_provisioning_filters_only_active_subscribers():
    provisioning = bridge.build_provisioning(sample_snapshot(), "10.0.0.5", 5060)
    assert provisioning["summary"]["softphone_subscriber_count"] == 1
    subscriber = provisioning["subscribers"][0]
    assert subscriber["dial_number"] == "7001"
    assert subscriber["sip_domain"] == "10.0.0.5"
    assert subscriber["provisioning_state"] == "Pending"


def test_render_pjsip_contains_only_active_endpoint():
    rendered = bridge.render_pjsip(sample_snapshot()["subscribers"], "seveny-devnet")
    assert "[alice7001]" in rendered
    assert "password=secret1" in rendered
    assert "bob7002" not in rendered


def test_render_extensions_contains_broadcast_and_direct_dial():
    rendered = bridge.render_extensions(sample_snapshot()["subscribers"])
    assert "exten => 7001,1,NoOp(7AY inbound call for 7001)" in rendered
    assert "exten => 7999,1,NoOp(7AY broadcast ring)" in rendered
    assert "bob7002" not in rendered


def test_write_outputs_creates_expected_files(tmp_path):
    provisioning = bridge.build_provisioning(sample_snapshot(), "127.0.0.1", 5060)
    bridge.write_outputs(
        tmp_path,
        provisioning,
        bridge.render_pjsip(sample_snapshot()["subscribers"], "seveny-devnet"),
        bridge.render_extensions(sample_snapshot()["subscribers"]),
    )
    generated = json.loads((tmp_path / "subscribers.softphone.json").read_text())
    assert generated["summary"]["softphone_subscriber_count"] == 1
    assert (tmp_path / "pjsip.generated.conf").exists()
    assert (tmp_path / "extensions.generated.conf").exists()


def test_build_provisioning_skips_failed_provisioning_state():
    snapshot = sample_snapshot()
    snapshot["subscribers"][0]["provisioning_state"] = "Failed"
    provisioning = bridge.build_provisioning(snapshot, "10.0.0.5", 5060)
    assert provisioning["summary"]["softphone_subscriber_count"] == 0
