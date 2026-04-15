# Radio Devnet Scaffold

This directory holds the devnet-only radio/presence model for 7AY.

What this is:
- a regional coverage summary tied to present 7AY nodes
- a mock signal-plane input for service gating
- a local test artifact for coverage and eligibility checks

What this is not:
- a real LTE/5G RAN
- a real SIM/baseband radio path
- a substitute for carrier network infrastructure

The sample files are consumed by `devnet/scripts/carrier_signal_plane.py`.
