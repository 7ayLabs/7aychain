# Mock SM-DP+ provisioning scaffold

This directory contains the devnet-only documents and templates for the 7AY telecom testing branch.

What it does:

- consumes the generated `devnet/telecom/esim/esim_profiles.json` artifact
- exposes a local mock SM-DP+ style HTTP surface through `devnet/scripts/carrier_smdp_server.py`
- serves the generated QR images and activation payloads for browser-based testing
- provides a small dashboard and status/discovery endpoints for manual validation

What it does not do:

- it does not implement real GSMA RSP / SM-DP+ production behavior
- it does not provision Apple carrier bundles or iPhone baseband credentials
- it does not replace a real telecom core, IMS/SIP network, or radio layer

Quick start:

```bash
python3 devnet/scripts/carrier_smdp_server.py
```

Useful endpoints:

- `GET /` - dashboard
- `GET /api/v1/health` - health and counts
- `GET /api/v1/discovery` - mock SM-DP+ discovery metadata
- `GET /api/v1/profiles` - loaded profile list
- `GET /api/v1/profiles/<id>` - single profile JSON
- `GET /api/v1/profiles/<id>/activation` - mock activation package JSON
- `GET /api/v1/profiles/<id>/qr` - QR image or plain text payload

Default inputs:

- `devnet/telecom/esim/esim_profiles.json`
- `devnet/telecom/esim/qrs/`

If you want to pin a different artifact location, edit `config.example.json` or pass explicit CLI arguments to the server.
