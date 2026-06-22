"""Tests for the carrier eSIM bridge scaffolding."""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import carrier_esim_bridge as bridge


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
                    "display_name": "Alice 7AY",
                    "sip_username": "alice7001",
                    "esim": {
                        "provider_name": "7AYchain Devnet",
                        "profile_label": "Alice 7AY",
                        "smdp_plus": "smdp.7aychain.local",
                        "activation_code": "7AY-ACT-7001-ALICE",
                        "confirmation_code": "",
                    },
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
                    "display_name": "Bob 7AY",
                    "sip_username": "bob7002",
                    "esim": {
                        "provider_name": "7AYchain Devnet",
                        "profile_label": "Bob 7AY",
                        "smdp_plus": "smdp.7aychain.local",
                        "activation_code": "7AY-ACT-7002-BOB",
                        "confirmation_code": "",
                    },
                },
            },
        ]
    }


def test_build_lpa_payload_without_confirmation_code():
    payload = bridge.build_lpa_payload(
        {
            "smdp_plus": "smdp.7aychain.local",
            "activation_code": "7AY-ACT-7001-ALICE",
            "confirmation_code": "",
        }
    )
    assert payload == "LPA:1$smdp.7aychain.local$7AY-ACT-7001-ALICE"


def test_build_profile_entries_filters_only_active_subscribers():
    profiles = bridge.build_profile_entries(sample_snapshot(), "qrs")
    assert len(profiles) == 1
    assert profiles[0]["dial_number"] == "7001"
    assert profiles[0]["qr_path"] == "qrs/esim-aaa.png"
    assert profiles[0]["provisioning_state"] == "Pending"


def test_render_gallery_contains_lpa_payload():
    profiles = bridge.build_profile_entries(sample_snapshot(), "qrs")
    gallery = bridge.render_gallery(profiles, qr_available=False)
    assert "7AYchain eSIM Gallery" in gallery
    assert "LPA:1$smdp.7aychain.local$7AY-ACT-7001-ALICE" in gallery
    assert "Install qrcode to render PNG QR files." in gallery


def test_write_outputs_creates_profile_json_and_html(tmp_path):
    profiles = bridge.build_profile_entries(sample_snapshot(), "qrs")
    bridge.write_outputs(tmp_path, profiles)
    profile_json = json.loads((tmp_path / "esim_profiles.json").read_text())
    assert profile_json["summary"]["profile_count"] == 1
    assert (tmp_path / "index.html").exists()
    assert (tmp_path / "esim-aaa.png.txt").exists()


def test_build_profile_entries_skips_failed_provisioning_state():
    snapshot = sample_snapshot()
    snapshot["subscribers"][0]["provisioning_state"] = "Failed"
    profiles = bridge.build_profile_entries(snapshot, "qrs")
    assert profiles == []
