# 7AY Carrier Softphone Devnet

This devnet path lets you test carrier-like call routing from chain state using
a SIP softphone on an iPhone or another device connected over Wi-Fi.

It also generates decentralized eSIM-style activation artifacts and QR files
for scan-on-phone testing.

It does **not** provision a real iPhone carrier bundle, native cellular eSIM,
or native Phone app integration. It emulates the telecom service layer above
the chain.

## Components

- `carrier_bridge.py`
  Reads on-chain carrier state and writes `devnet/state/carrier-bridge.json`.
- `publish_carrier_catalog.py`
  Adds dial numbers and SIP credentials for chain-native `number_id` values.
- `carrier_softphone_bridge.py`
  Converts the carrier snapshot into generated Asterisk configs and a
  softphone provisioning JSON file.
- `carrier_esim_bridge.py`
  Converts active subscriber state into LPA-style activation payloads, QR
  assets, and an HTML gallery for phone scanning.
- `docker-compose.telecom.yml`
  Starts a local Asterisk instance for SIP-based call testing.

## Step-by-Step

1. Start the 7AY devnet node:

   ```bash
   devnet/scripts/dev.sh native
   ```

2. Use `python3 devnet/scripts/laud-cli.py` to complete:
   - identity registration + activation
   - device registration + activation
   - proof-of-presence verification
   - carrier number activation

   If `pip` is not available in your shell, install Python dependencies with:

   ```bash
   python3 -m pip install -r devnet/scripts/requirements.txt
   ```

3. Publish a sample telecom catalog:

   ```bash
   python3 devnet/scripts/publish_carrier_catalog.py --sample
   ```

4. Start the chain-to-softphone bridge watchers:

   ```bash
   bash devnet/scripts/telecom-dev.sh
   ```

5. Start the SIP server:

   ```bash
   cd devnet
   docker compose -f docker-compose.telecom.yml up -d
   ```

6. Inspect generated provisioning:

   ```bash
   cat devnet/telecom/generated/subscribers.softphone.json
   ```

7. Inspect generated eSIM artifacts:

   ```bash
   cat devnet/telecom/esim/esim_profiles.json
   open devnet/telecom/esim/index.html
   ```

8. On your iPhone, install a SIP softphone such as Linphone or Zoiper.

9. Configure the softphone using one entry from
   `subscribers.softphone.json`:
   - username: `sip_username`
   - password: `sip_password`
   - domain: your Mac's LAN IP address
   - port: `5060`
   - transport: UDP

10. For eSIM-style enrollment testing, serve the gallery to the phone:

    ```bash
    python3 -m http.server 8000
    ```

    Then open on the iPhone:

    ```text
    http://<your-mac-lan-ip>:8000/devnet/telecom/esim/
    ```

    Scan the QR from the phone. This tests the provisioning artifact and QR
    flow, not native carrier activation on the baseband.

11. Register two active subscribers and call between them using their
   `dial_number` values.

12. Change chain state:
    - suspend number
    - revoke number
    - recover onto another device

    Then watch the generated provisioning update and re-test calling.

## Files Produced

- `devnet/state/carrier-catalog.json`
- `devnet/state/carrier-bridge.json`
- `devnet/telecom/generated/subscribers.softphone.json`
- `devnet/telecom/generated/pjsip.generated.conf`
- `devnet/telecom/generated/extensions.generated.conf`
- `devnet/telecom/esim/esim_profiles.json`
- `devnet/telecom/esim/index.html`
- `devnet/telecom/esim/qrs/*.png`

## Notes

- For iPhone testing, your phone and Mac must be on the same network.
- Use your Mac's LAN IP instead of `127.0.0.1` in the softphone.
- The QR payloads are LPA-style devnet artifacts. They are not a real
  Apple/carrier eSIM provisioning path by themselves.
- If you want audio over the internet or PSTN bridging, that is a separate
  step beyond this devnet scaffold.
