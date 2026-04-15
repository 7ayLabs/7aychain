# 7AY Telecom Test Stack

This branch adds a devnet-only telecom testing stack around the existing 7AY carrier flow.

What it includes:

- mock SM-DP+ / eSIM HTTP service
- provisioning API for softphone clients
- signal-plane service for IMS/radio readiness summaries
- optional Kamailio SIP edge in front of Asterisk

What it does not include:

- real GSMA RSP / production SM-DP+
- native iPhone carrier activation
- real LTE/5G radio access
- PSTN settlement or commercial interconnect

## Run

```bash
python3 -m venv devnet/.venv
devnet/.venv/bin/pip install -r devnet/scripts/requirements.txt

python3 devnet/scripts/publish_carrier_catalog.py --sample --smdp-plus 127.0.0.1:8091
bash devnet/scripts/telecom-stack-dev.sh
```

Optional SIP edge:

```bash
DOCKER_STACK=true SIP_PORT=5062 bash devnet/scripts/telecom-stack-dev.sh
```

Useful endpoints:

- `http://127.0.0.1:8090/api/v1/subscribers`
- `http://127.0.0.1:8091/`
- `http://127.0.0.1:8091/api/v1/discovery`
- `http://127.0.0.1:8092/api/v1/summary`

The intended flow is:

1. Chain state authorizes the carrier number.
2. `carrier_bridge.py` writes a carrier snapshot.
3. `carrier_esim_bridge.py` emits eSIM artifacts and QR payloads.
4. `carrier_smdp_server.py` exposes the mock provisioning surface referenced by the catalog.
5. `carrier_signal_plane.py` provides IMS/radio readiness summaries for local testing.
6. If Docker is enabled, Kamailio fronts Asterisk as a minimal SIP edge on port `5062`.
