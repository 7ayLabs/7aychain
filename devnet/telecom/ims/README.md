# IMS Devnet Scaffold

This directory holds the devnet-only IMS/SIP testing model for 7AY.

What this is:
- a local representation of IMS registration state
- a small state/config pair for service-gate testing
- a stand-in for how a regional carrier core would look to the chain-adjacent plane
- an optional Kamailio edge proxy config that can front Asterisk on `127.0.0.1:5062`

What this is not:
- a real SM-DP+ server
- a real mobile IMS core
- native iPhone carrier provisioning
- PSTN interconnect

The sample files are meant to be used by `devnet/scripts/carrier_signal_plane.py`
and by local testing harnesses that want to inspect or override IMS state.

Optional proxy path:

- `kamailio.cfg` provides a minimal IMS-style SIP edge for docker-compose testing
- `dispatcher.list` routes requests to the internal `asterisk:5060` upstream
