#!/usr/bin/env python3
"""
LAUD NETWORKS 7ayLabs - Proof of Presence Protocol
Interactive CLI for the 7aychain network.

Usage:
    python3 laud-cli.py [--url ws://host:port]

Requirements:
    pip install substrate-interface
"""

import sys, os, time, json, hashlib, secrets, argparse, pathlib, re
from datetime import datetime

_ANSI_RE = re.compile(r'\033\[[0-9;]*m')

SUBSTRATE_OK = False
try:
    from substrateinterface import SubstrateInterface, Keypair
    from substrateinterface.exceptions import SubstrateRequestException
    SUBSTRATE_OK = True
except ImportError:
    pass

from laud_registry import (
    DOMAINS, GROUP_DISPLAY_ORDER,
    find_domain, find_command,
    build_menu_aliases, build_sub_aliases,
    build_cmd_names, build_cmd_subs,
    get_domains_for_mode, get_commands_for_mode,
    get_group_display_order,
    build_menu_aliases_for_mode,
    build_cmd_names_for_mode, build_cmd_subs_for_mode,
)
from laud_files import (
    hash_file_blake2b, store_encrypted_file, retrieve_and_decrypt,
    add_to_index, get_vault_files, verify_file as verify_vault_file,
    store_share, load_share, load_all_shares,
    export_share_hex, import_share_hex, secure_zero,
    anonymize_existing_index,
)
from laud_crypto import (
    ShamirScheme, key_fingerprint, generate_fek,
    DOMAIN_VAULT_FEK,
)


class C:
    B   = "\033[94m"
    BB  = "\033[1;94m"
    G   = "\033[92m"
    Y   = "\033[93m"
    RED = "\033[91m"
    CY  = "\033[96m"
    W   = "\033[1;97m"
    DIM = "\033[2m"
    R   = "\033[0m"


class LaudCLI:

    MODE_CONFIG_FILE = os.path.expanduser('~/.laud_config.json')

    def __init__(self, url="ws://127.0.0.1:9944", mode=None):
        self.url = url
        self.substrate = None
        self.keypairs = {}
        self.connected = False
        self._ctx_epoch = None
        self._ctx_account = 'alice'
        self._nav_stack = []
        self._history_file = os.path.expanduser('~/.laud_history')
        self._mode = mode or self._load_mode()
        self._menu_aliases = build_menu_aliases_for_mode(self._mode)
        self._sub_aliases = build_sub_aliases()

    def _load_mode(self):
        """Load mode from config file. Defaults to 'normal'."""
        try:
            with open(self.MODE_CONFIG_FILE, 'r') as f:
                config = json.load(f)
                return config.get('mode', 'normal')
        except (FileNotFoundError, json.JSONDecodeError, KeyError):
            return 'normal'

    def _save_mode(self):
        """Persist current mode to config file."""
        config = {}
        try:
            with open(self.MODE_CONFIG_FILE, 'r') as f:
                config = json.load(f)
        except (FileNotFoundError, json.JSONDecodeError):
            pass
        config['mode'] = self._mode
        config_dir = os.path.dirname(self.MODE_CONFIG_FILE)
        if config_dir:
            os.makedirs(config_dir, exist_ok=True)
        with open(self.MODE_CONFIG_FILE, 'w') as f:
            json.dump(config, f, indent=2)

    def _rebuild_indexes(self):
        """Rebuild all cached indexes after mode change."""
        from laud_registry import _DOMAIN_INDEX
        _DOMAIN_INDEX.clear()
        self._menu_aliases = build_menu_aliases_for_mode(self._mode)
        self._sub_aliases = build_sub_aliases()
        self.__class__._CMD_NAMES = build_cmd_names_for_mode(self._mode)
        self.__class__._CMD_SUBS = build_cmd_subs_for_mode(self._mode)

    # ------------------------------------------------------------------
    # Connection
    # ------------------------------------------------------------------

    def connect(self, url=None):
        url = url or self.url
        if not SUBSTRATE_OK:
            self._err("substrate-interface not installed")
            print(f"  Run: {C.Y}pip install substrate-interface{C.R}")
            return False
        try:
            self._info(f"Connecting to {url}...")
            self.substrate = SubstrateInterface(
                url=url, auto_reconnect=True,
                ws_options={'open_timeout': 10, 'ping_interval': 30,
                            'ping_timeout': 10},
            )
            self.url = url
            self.connected = True
            for name in ['alice', 'bob', 'charlie', 'dave', 'eve', 'ferdie']:
                self.keypairs[name] = Keypair.create_from_uri(
                    f"//{name.capitalize()}")
            chain = self.substrate.rpc_request("system_chain", [])['result']
            ver = self.substrate.rpc_request("system_version", [])['result']
            self._ok(f"Connected to {C.W}{chain}{C.R} v{ver}")
            return True
        except (ConnectionError, BrokenPipeError, OSError) as e:
            self._err(f"Connection failed: {e}")
            return False
        except Exception as e:
            self._err(f"Connection failed: {e}")
            return False

    def _reconnect(self):
        try:
            self.substrate.rpc_request("system_chain", [])
            return True
        except (ConnectionError, BrokenPipeError, OSError):
            if self._mode == 'dev':
                self._info("Connection check failed, attempting reconnect")
        except Exception:
            if self._mode == 'dev':
                self._info("Connection check failed, attempting reconnect")
        try:
            self._info("Reconnecting...")
            self.substrate = SubstrateInterface(
                url=self.url, auto_reconnect=True,
                ws_options={'open_timeout': 10, 'ping_interval': 30,
                            'ping_timeout': 10},
            )
            self.connected = True
            return True
        except (ConnectionError, BrokenPipeError, OSError) as e:
            self._err(f"Reconnect failed: {e}")
            self.connected = False
            return False
        except Exception as e:
            self._err(f"Reconnect failed: {e}")
            self.connected = False
            return False

    def _ensure(self):
        if not self.connected:
            self._err("Not connected. Use option 1 first.")
            return False
        if not self._reconnect():
            return False
        return True

    # ------------------------------------------------------------------
    # Readline / Autocomplete
    # ------------------------------------------------------------------

    def _setup_readline(self):
        try:
            import readline
            readline.set_completer(self._completer)
            readline.parse_and_bind('tab: complete')
            readline.set_completer_delims(' ')
            try:
                readline.read_history_file(self._history_file)
            except FileNotFoundError:
                pass
            readline.set_history_length(500)
            import atexit
            atexit.register(readline.write_history_file, self._history_file)
        except ImportError:
            pass

    _CMD_NAMES = build_cmd_names()
    _CMD_SUBS = build_cmd_subs()

    def _completer(self, text, state):
        try:
            import readline
            line = readline.get_line_buffer().lstrip()
            parts = line.split()
            if not parts or (len(parts) == 1 and not line.endswith(' ')):
                prefix = parts[0] if parts else ''
                candidates = [c + ' ' for c in self._CMD_NAMES
                              if c.startswith(prefix)]
            else:
                parent = parts[0].lower()
                subs = self._CMD_SUBS.get(parent, [])
                prefix = text.lower()
                candidates = [s + ' ' for s in subs if s.startswith(prefix)]
            return candidates[state] if state < len(candidates) else None
        except (IndexError, ValueError, TypeError):
            return None
        except Exception:
            return None

    # ------------------------------------------------------------------
    # Chain interaction (submit / query)
    # ------------------------------------------------------------------

    # Aura slot duration in seconds (MinimumPeriod 3500ms * 2).
    _SLOT_SECS = 7

    def _slot_wait(self):
        """Wait until the next Aura slot boundary."""
        now = time.time()
        next_slot = (int(now / self._SLOT_SECS) + 1) * self._SLOT_SECS
        time.sleep(next_slot - now + 0.1)

    def _submit(self, module, fn, params, signer='alice', sudo=False,
                _skip_confirm=False):
        if not self._ensure():
            return None
        if sudo and not _skip_confirm:
            if not self._prompt_bool(
                    "This is an admin operation. Continue?"):
                self._info("Cancelled.")
                return None
        for attempt in range(3):
            try:
                call = self.substrate.compose_call(module, fn, params)
                if sudo:
                    call = self.substrate.compose_call(
                        'Sudo', 'sudo', {'call': call})
                    signer = 'alice'
                kp = self.keypairs[signer]
                ext = self.substrate.create_signed_extrinsic(
                    call=call, keypair=kp)
                tag = f"{C.DIM}[admin]{C.R} " if sudo else ""
                self._info(
                    f"{tag}{C.W}{module}.{fn}{C.R} "
                    f"{C.DIM}as{C.R} {C.Y}{signer}{C.R}")
                receipt = self.substrate.submit_extrinsic(
                    ext, wait_for_inclusion=True)
                if receipt.is_success:
                    blk_num = ""
                    try:
                        hdr = self.substrate.get_block_header(
                            block_hash=receipt.block_hash)
                        blk_num = f"#{hdr['header']['number']}"
                    except (ConnectionError, BrokenPipeError, OSError):
                        blk_num = str(receipt.block_hash)[:16]
                    except (KeyError, TypeError):
                        blk_num = str(receipt.block_hash)[:16]
                    except Exception:
                        blk_num = str(receipt.block_hash)[:16]
                    pallet_events = []
                    for ev in receipt.triggered_events:
                        ev_val = ev.value
                        if isinstance(ev_val, dict) and 'event' in ev_val:
                            edata = ev_val['event']
                            mid = edata.get('module_id',
                                            edata.get('event_index', '?'))
                            eid = edata.get('event_id', '')
                            if (mid == 'System'
                                    and eid == 'ExtrinsicFailed'):
                                pallet_events.append(
                                    f"{C.RED}{mid}.{eid}{C.R}")
                            elif mid not in ('System',
                                             'TransactionPayment',
                                             'Balances', 0):
                                pallet_events.append(f"{mid}.{eid}")
                    ev_str = (f" {C.DIM}({', '.join(pallet_events)}){C.R}"
                              if pallet_events else "")
                    self._ok(f"Block {blk_num}{ev_str}")
                else:
                    self._err(f"{receipt.error_message}")
                    if (hasattr(receipt, 'error_message')
                            and receipt.error_message):
                        err = receipt.error_message
                        if isinstance(err, dict):
                            print(f"       {C.RED}Detail: "
                                  f"{json.dumps(err, indent=2)}{C.R}")
                    hint = self._error_hint(receipt.error_message)
                    if hint:
                        print(f"       {C.Y}Hint: {hint}{C.R}")
                # Always wait for next slot after any submission.
                self._slot_wait()
                return receipt
            except (ConnectionError, BrokenPipeError, OSError) as e:
                if attempt < 2:
                    self._info("Connection lost, reconnecting...")
                    self._slot_wait()
                    if self._reconnect():
                        continue
                self._err(str(e))
                return None
            except Exception as e:
                err_msg = str(e).lower()
                if 'priority' in err_msg or 'pool' in err_msg:
                    if attempt < 2:
                        self._info("Pool busy, waiting for next slot...")
                        self._slot_wait()
                        continue
                if (attempt < 2
                        and ('connection' in err_msg or 'lost' in err_msg
                             or 'closed' in err_msg
                             or 'websocket' in err_msg)):
                    self._info("Connection lost, reconnecting...")
                    self._slot_wait()
                    if self._reconnect():
                        continue
                self._err(str(e))
                return None

    def _query(self, module, fn, params=None):
        if not self._ensure():
            return None
        for attempt in range(2):
            try:
                result = self.substrate.query(module, fn, params or [])
                self._info(f"{C.DIM}query{C.R} {module}.{fn}")
                return result
            except Exception as e:
                err_msg = str(e).lower()
                if (attempt == 0
                        and ('connection' in err_msg or 'lost' in err_msg
                             or 'closed' in err_msg
                             or 'websocket' in err_msg)):
                    if self._reconnect():
                        continue
                self._err(f"{module}.{fn}: {e}")
                return None

    def _safe_query(self, module, fn, params=None, fallback=None):
        """Query with graceful error handling."""
        try:
            result = self.substrate.query(module, fn, params or [])
            return result
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
            return fallback
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"Query {module}.{fn} unavailable: {e}")
            return fallback

    def _query_map(self, module, fn):
        if not self._ensure():
            return []
        for attempt in range(2):
            try:
                entries = list(self.substrate.query_map(module, fn))
                self._info(
                    f"{C.DIM}query_map{C.R} {module}.{fn} "
                    f"{C.DIM}({len(entries)} entries){C.R}")
                return entries
            except Exception as e:
                err_msg = str(e).lower()
                if (attempt == 0
                        and ('connection' in err_msg or 'lost' in err_msg
                             or 'closed' in err_msg
                             or 'websocket' in err_msg)):
                    if self._reconnect():
                        continue
                self._err(f"{module}.{fn}: {e}")
            return []

    # ------------------------------------------------------------------
    # Display helpers
    # ------------------------------------------------------------------

    def _show(self, result, label=None):
        if result is None:
            return
        val = result.value if hasattr(result, 'value') else result
        if label:
            print(f"  {C.DIM}{label}:{C.R}")
        if isinstance(val, dict):
            for k, v in val.items():
                if isinstance(v, dict):
                    print(f"    {C.CY}{k}{C.R}:")
                    for k2, v2 in v.items():
                        print(f"      {C.DIM}{k2:>20}{C.R}: "
                              f"{C.W}{v2}{C.R}")
                elif isinstance(v, list):
                    print(f"    {C.CY}{k:>24}{C.R}: "
                          f"{C.W}[{len(v)} items]{C.R}")
                    for i, item in enumerate(v[:5]):
                        print(f"      {C.DIM}[{i}]{C.R} {item}")
                    if len(v) > 5:
                        print(f"      {C.DIM}... +{len(v)-5} more{C.R}")
                else:
                    print(f"    {C.CY}{k:>24}{C.R}: {C.W}{v}{C.R}")
        elif isinstance(val, list):
            for i, item in enumerate(val[:20]):
                print(f"    {C.DIM}[{i}]{C.R} {item}")
            if len(val) > 20:
                print(f"    {C.DIM}... +{len(val)-20} more{C.R}")
        else:
            print(f"    {C.W}{val}{C.R}")

    def _ok(self, msg):
        if self._mode == 'normal':
            print(f"  {C.G}[Done]{C.R} {msg}")
        else:
            print(f"  {C.G}[OK]{C.R} {msg}")

    def _err(self, msg):
        if self._mode == 'normal':
            # Strip technical details for normal users
            friendly = self._friendly_error(msg)
            print(f"  {C.RED}[!]{C.R} {friendly}")
        else:
            print(f"  {C.RED}[ERR]{C.R} {msg}")

    def _friendly_error(self, msg):
        """Convert technical error to user-friendly message."""
        msg_lower = str(msg).lower()
        if 'connection' in msg_lower or 'websocket' in msg_lower:
            return "Can't reach the network. Make sure the node is running."
        if 'pool' in msg_lower or 'priority' in msg_lower:
            return "The network is busy. Please try again in a moment."
        if 'not found' in msg_lower:
            return "The requested item was not found."
        if 'insufficient' in msg_lower:
            return "Not enough funds or permissions for this action."
        # Strip hex values and technical identifiers for normal mode
        cleaned = re.sub(r'0x[0-9a-fA-F]{8,}', '[hash]', str(msg))
        return cleaned

    def _info(self, msg):
        print(f"  {C.CY}[..]{C.R} {msg}")

    def _progress(self, step, total, msg):
        bar_len = 20
        filled = int(bar_len * step / total)
        bar = f"{C.G}{'█' * filled}{C.DIM}{'░' * (bar_len - filled)}{C.R}"
        print(f"  {bar} {C.DIM}[{step}/{total}]{C.R} {msg}")

    _STATE_COLORS = {
        'Finalized': C.G, 'Validated': C.G,
        'Declared': C.Y, 'Active': C.Y, 'Scheduled': C.Y,
        'Slashed': C.RED, 'Suspended': C.RED, 'Revoked': C.RED,
        'Closed': C.DIM,
    }

    def _colorize_state(self, val):
        """Color-code known presence/lifecycle states."""
        s = str(val)
        color = self._STATE_COLORS.get(s)
        if color:
            return f"{color}{s}{C.R}"
        return f"{C.W}{s}{C.R}"

    def _val(self, key, val):
        v = val.value if hasattr(val, 'value') else val
        if isinstance(v, dict):
            print(f"  {C.CY}{key}{C.R}:")
            for k2, v2 in v.items():
                display = (self._colorize_state(v2)
                           if k2 in ('state', 'status')
                           else f"{C.W}{v2}{C.R}")
                print(f"    {C.DIM}{k2:>22}:{C.R} {display}")
        else:
            display = (self._colorize_state(v)
                       if key.lower() in ('state', 'status', 'final state')
                       else f"{C.W}{v}{C.R}")
            print(f"  {C.CY}{key}:{C.R} {display}")

    def _table(self, headers, rows):
        if not rows:
            print(f"  {C.DIM}(no data){C.R}")
            return
        widths = [len(str(h)) for h in headers]
        for row in rows:
            for i, cell in enumerate(row):
                if i < len(widths):
                    widths[i] = max(widths[i], len(str(cell)))
        widths = [min(w, 32) for w in widths]
        hdr = "  "
        sep = "  "
        for i, h in enumerate(headers):
            w = widths[i] if i < len(widths) else 10
            hdr += f"{C.BB}{str(h):<{w}}{C.R}  "
            sep += f"{C.DIM}{'─' * w}{C.R}  "
        print(hdr)
        print(sep)
        for row in rows:
            line = "  "
            for i, cell in enumerate(row):
                w = widths[i] if i < len(widths) else 10
                s = str(cell)
                if len(s) > w:
                    s = s[:w-1] + '…'
                if i == 0:
                    line += f"{C.CY}{s:<{w}}{C.R}  "
                else:
                    line += f"{C.W}{s:<{w}}{C.R}  "
            print(line)
        print()

    def _box(self, lines, color=None, width=53):
        c = color or C.BB
        top = f"  {c}\u256d\u2500{'\u2500' * width}\u2500\u256e{C.R}"
        bot = f"  {c}\u2570\u2500{'\u2500' * width}\u2500\u256f{C.R}"
        print(top)
        for ln in lines:
            vis = len(_ANSI_RE.sub('', ln))
            pad = max(0, width - vis)
            print(f"  {c}\u2502{C.R} {ln}{' ' * pad} {c}\u2502{C.R}")
        print(bot)

    def _separator_line(self, label="", width=53):
        total = width + 4
        if label:
            dash_len = max(0, total - len(label) - 4)
            print(f"  {C.DIM}\u2500\u2500 {C.B}{label}{C.DIM} "
                  f"{'\u2500' * dash_len}{C.R}")
        else:
            print(f"  {C.DIM}{'\u2500' * total}{C.R}")

    def _header(self, title):
        print()
        self._separator_line(title)

    def _menu_display(self, title, options, domain=None):
        # Box header with summary
        if domain and domain.help_summary:
            print()
            self._box([
                f"{C.BB}{title}{C.R}",
                f"{C.DIM}{domain.help_summary[:51]}{C.R}",
            ], color=C.BB)
        else:
            print()
            self._separator_line(title)

        # Build command lookup for help_text
        cmd_info = {}
        if domain:
            for cmd in domain.commands:
                cmd_info[cmd.key] = cmd

        for key, label in options:
            if key == "\u2500" or key == "---":
                if label:
                    print()
                    self._separator_line(label.upper())
                else:
                    print()
                continue
            if key in ("?", "i", "0"):
                continue  # shown in footer

            # Lookup help_text
            ci = cmd_info.get(key)
            ht = ""
            if ci and ci.help_text:
                ht = ci.help_text[:30]

            if ht:
                lbl = label[:22]
                dot_n = max(2, 26 - len(lbl))
                dots = '\u00b7' * dot_n
                print(f"   {C.Y}{key:>2}{C.R}  "
                      f"{lbl:<22}"
                      f"{C.DIM}{dots}{C.R} "
                      f"{C.DIM}{ht}{C.R}")
            else:
                print(f"   {C.Y}{key:>2}{C.R}  {label}")

        # Footer
        print()
        self._separator_line()
        print(f"    {C.DIM}i{C.R}  instructions"
              f"    {C.DIM}?{C.R}  refresh"
              f"    {C.DIM}0{C.R}  back")
        print()

    # ------------------------------------------------------------------
    # Prompt helpers
    # ------------------------------------------------------------------

    def _prompt(self, text, default=None):
        suffix = f" [{C.DIM}{default}{C.R}]" if default else ""
        try:
            val = input(f"  {C.CY}>{C.R} {text}{suffix}: ").strip()
            return val if val else (default or "")
        except (EOFError, KeyboardInterrupt):
            print()
            return default or ""

    def _prompt_int(self, text, default=0):
        try:
            return int(self._prompt(text, str(default)))
        except ValueError:
            return default

    def _prompt_bool(self, text, default=True):
        d = "Y/n" if default else "y/N"
        val = self._prompt(f"{text} ({d})", "").lower()
        if val in ('y', 'yes'):
            return True
        if val in ('n', 'no'):
            return False
        return default

    def _prompt_account(self, label="Account"):
        names = list(self.keypairs.keys())
        default = self._ctx_account
        print(f"  {C.DIM}Accounts: {', '.join(names)}  "
              f"(active: {default}){C.R}")
        name = self._prompt(label, default).lower()
        return name if name in names else default

    def _prompt_epoch(self, label="Epoch"):
        default = self._ctx_epoch if self._ctx_epoch is not None else 1
        return self._prompt_int(label, default)

    def _prompt_position(self, label="Position"):
        x = self._prompt_int(f"{label} X (m)", 0)
        y = self._prompt_int(f"{label} Y (m)", 0)
        z = self._prompt_int(f"{label} Z (m)", 0)
        return {"x": x, "y": y, "z": z}

    def _prompt_h256(self, label="32-byte hex"):
        val = self._prompt(label, "0x" + "00" * 32)
        if not val.startswith("0x"):
            val = "0x" + val
        # Validate hex format
        hex_part = val[2:]
        try:
            bytes.fromhex(hex_part)
        except ValueError:
            self._err(f"Invalid hex: '{val}'. Using default.")
            val = "0x" + "00" * 32
        if len(hex_part) != 64:
            self._info(f"Expected 32 bytes (64 hex chars), got {len(hex_part)}. "
                       "Padding/truncating.")
            hex_part = hex_part.ljust(64, '0')[:64]
            val = "0x" + hex_part
        return val

    def _prompt_actor(self, label="Identity"):
        use_name = self._prompt_bool("Use an account name? (alice, bob, ...)")
        if use_name:
            name = self._prompt_account(label)
            aid = self._actor_id(name)
            print(f"  {C.DIM}ID: {aid[:20]}...{C.R}")
            return aid
        return self._prompt_h256(f"{label} (32-byte hex)")

    def _prompt_enum(self, label, options):
        for i, opt in enumerate(options, 1):
            print(f"    {C.Y}{i}{C.R} {opt}")
        idx = self._prompt_int(label, 1) - 1
        return options[max(0, min(idx, len(options) - 1))]

    def _actor_id(self, name):
        kp = self.keypairs[name]
        return '0x' + hashlib.blake2b(
            kp.public_key, digest_size=32).hexdigest()

    def _validator_id(self, name):
        return self._actor_id(name)

    def _pause(self):
        print(f"  {C.DIM}{'\u2500' * 40}{C.R}")

    # ------------------------------------------------------------------
    # Error hints
    # ------------------------------------------------------------------

    ERROR_HINTS = {
        'EpochNotActive':
            'The time period is not active yet. '
            'Type "bootstrap" to set up the network.',
        'NotAValidator':
            'This account is not a validator. '
            'Type "bootstrap" to register all test validators.',
        'NotAnActiveValidator':
            'This validator is not active. '
            'Type "bootstrap" to register and activate validators.',
        'PositionAlreadyClaimed':
            'You already claimed a position this period. '
            'Try "use epoch <N>" to switch periods.',
        'DuplicateAttestation':
            'This witness already confirmed this period. '
            'Use a different witness account.',
        'DuplicatePresence':
            'Already declared this period. '
            'Try "use epoch <N>" to switch periods.',
        'DuplicateVote':
            'Already voted this period. '
            'Try "use epoch <N>" to switch periods.',
        'PresenceImmutable':
            'This check-in is already confirmed. '
            'Confirmed records cannot be changed.',
        'SelfAttestation':
            'You cannot verify your own location. '
            'Ask another verifier to confirm you.',
        'InsufficientAttestations':
            'Not enough location confirmations yet. '
            'You need at least 3 nearby verifiers.',
        'InsufficientWitnesses':
            'Not enough witnesses yet. '
            'Need at least 3 attestations before verifying.',
        'AlreadyDeclared':
            'Already declared presence this period.',
        'QuorumNotReached':
            'Not enough votes yet. '
            'Need at least 3 validator votes to finalize.',
        'VaultLocked':
            'This vault is locked and cannot be accessed right now.',
        'InvalidRingSize':
            'The vault requires between 3 and 10 members.',
        'ThresholdTooLow':
            'At least 2 key holders are required for security.',
        'AlreadyRegistered':
            'This account is already registered.',
        'BondingPeriod':
            'The security deposit bonding period has not elapsed yet.',
    }

    def _error_hint(self, err):
        err_str = str(err)
        for key, hint in self.ERROR_HINTS.items():
            if key in err_str:
                return hint
        return None

    # ------------------------------------------------------------------
    # Generic menu engine (registry-driven)
    # ------------------------------------------------------------------

    def _run_domain(self, domain, _direct=None):
        """Drive any domain from its registry definition."""
        if domain.check_epoch:
            self._check_epoch()
        self._nav_stack.append(domain.name)

        visible_cmds = get_commands_for_mode(domain, self._mode)
        opts = []
        for cmd in visible_cmds:
            if cmd.action == "separator":
                opts.append(("---", cmd.label))
            else:
                label = (cmd.normal_label
                         if (self._mode == 'normal'
                             and cmd.normal_label)
                         else cmd.label)
                opts.append((cmd.key, label))
        opts.append(("i", "Instructions"))
        opts.append(("?", "Show options"))
        opts.append(("0", "Back"))

        title = (domain.normal_title
                 if self._mode == 'normal' and domain.normal_title
                 else domain.title)
        if not _direct:
            self._menu_display(title, opts, domain=domain)

        while True:
            if _direct:
                c = _direct
                _direct = None
            else:
                c = self._prompt("", "0")

            if c in ("0", "back"):
                self._nav_stack.pop()
                break
            if c == "?":
                self._menu_display(title, opts, domain=domain)
                continue
            if c == "i" or c.startswith("i "):
                parts = c.split(None, 1)
                if len(parts) > 1:
                    cmd = find_command(domain, parts[1])
                    if cmd:
                        self._show_command_instructions(cmd)
                    else:
                        self._err(f"Unknown command: '{parts[1]}'")
                else:
                    self._show_domain_instructions(domain)
                continue

            cmd = find_command(domain, c)
            if not cmd:
                self._err(f"Unknown option: '{c}'. Type ? for options.")
                continue

            self._execute_command(cmd)
            self._pause()

    def _execute_command(self, cmd):
        """Execute a registry command: collect params, submit/query."""
        if cmd.action == "custom":
            handler = getattr(self, cmd.custom_handler, None)
            if handler:
                handler()
            else:
                self._err(f"Handler not found: {cmd.custom_handler}")
            return

        if cmd.action == "separator":
            return

        if not self._ensure():
            return

        params = {}
        if cmd.fixed_params:
            params.update(cmd.fixed_params)

        signer = self._ctx_account
        has_account_param = False

        for p in cmd.params:
            if p.kind == "epoch":
                params[p.name] = self._prompt_epoch(p.label)
            elif p.kind == "int":
                params[p.name] = self._prompt_int(p.label, p.default or 0)
            elif p.kind == "str":
                params[p.name] = self._prompt(p.label, p.default or "")
            elif p.kind == "bool":
                params[p.name] = self._prompt_bool(
                    p.label,
                    p.default if p.default is not None else True)
            elif p.kind == "account":
                signer = self._prompt_account(p.label)
                has_account_param = True
            elif p.kind == "actor":
                params[p.name] = self._prompt_actor(p.label)
            elif p.kind == "h256":
                params[p.name] = self._prompt_h256(p.label)
            elif p.kind == "position":
                params[p.name] = self._prompt_position(p.label)
            elif p.kind == "enum":
                params[p.name] = self._prompt_enum(p.label, p.options)

        if cmd.action == "submit":
            if not has_account_param and not cmd.sudo:
                signer = self._prompt_account("Account")
            self._submit(cmd.pallet, cmd.function, params,
                         signer, sudo=cmd.sudo)

        elif cmd.action == "query":
            param_list = ([params[p.name] for p in cmd.params]
                          if cmd.params else None)
            result = self._query(cmd.pallet, cmd.function, param_list)
            self._val(cmd.label, result)

        elif cmd.action == "query_map":
            entries = self._query_map(cmd.pallet, cmd.function)
            for k, v in entries[:10]:
                kv = k.value if hasattr(k, 'value') else str(k)
                vv = v.value if hasattr(v, 'value') else str(v)
                kid = str(kv)[:20] if kv else '?'
                print(f"    {C.DIM}{kid}... = {vv}{C.R}")

    # ------------------------------------------------------------------
    # Instructions system
    # ------------------------------------------------------------------

    def _show_domain_instructions(self, domain):
        print()
        self._box([
            f"{C.BB}About: {domain.title}{C.R}",
        ], color=C.CY)
        if domain.instructions:
            print(domain.instructions)
        elif domain.help_summary:
            print(f"  {domain.help_summary}")
        print()
        self._separator_line("COMMANDS")
        for cmd in domain.commands:
            if cmd.action == "separator":
                continue
            print(f"   {C.Y}{cmd.key:>2}{C.R}  "
                  f"{C.W}{cmd.label}{C.R}")
            if cmd.help_text:
                print(f"       {C.DIM}{cmd.help_text}{C.R}")
        print()

    def _show_command_instructions(self, cmd):
        print()
        self._box([
            f"{C.BB}{cmd.label}{C.R}",
        ], color=C.CY)
        if cmd.instructions:
            print(cmd.instructions)
        elif cmd.help_text:
            print(f"  {cmd.help_text}")
        else:
            print(f"  {C.DIM}No detailed instructions "
                  f"for this command.{C.R}")
        if cmd.aliases:
            print(f"\n  {C.DIM}Shortcuts: "
                  f"{', '.join(cmd.aliases)}{C.R}")
        print()

    # ------------------------------------------------------------------
    # Context / Status / Bootstrap
    # ------------------------------------------------------------------

    def _cmd_use(self, args):
        if not args:
            self._info(
                f"epoch={C.W}{self._ctx_epoch or 'auto'}{C.R}  "
                f"account={C.W}{self._ctx_account}{C.R}")
            self._info(
                "Usage: use epoch <N> | use <account> | use clear")
            return
        if args[0] == 'epoch':
            if len(args) > 1:
                try:
                    self._ctx_epoch = int(args[1])
                    self._ok(f"Context epoch set to {self._ctx_epoch}")
                except ValueError:
                    self._err(f"Invalid epoch: {args[1]}")
            else:
                self._val("Current epoch", self._ctx_epoch or "not set")
        elif args[0] == 'clear':
            self._ctx_epoch = None
            self._ctx_account = 'alice'
            self._ok("Context cleared (epoch=auto, account=alice)")
        elif args[0] in self.keypairs:
            self._ctx_account = args[0]
            self._ok(f"Context account set to {self._ctx_account}")
        else:
            self._err(
                f"Unknown: '{args[0]}'. "
                "Try: use epoch 5, use bob, use clear")

    def _show_status(self):
        parts = [f"{C.BB}laud{C.R}"]
        if self.connected:
            parts.append(f"{C.DIM}{self.url}{C.R}")
            try:
                blk = self.substrate.get_block_header(
                )['header']['number']
                parts.append(f"{C.G}block #{blk}{C.R}")
            except (ConnectionError, BrokenPipeError, OSError):
                parts.append(f"{C.G}connected{C.R}")
                if self._mode == 'dev':
                    self._info("Block header fetch failed (connection)")
            except Exception:
                parts.append(f"{C.G}connected{C.R}")
                if self._mode == 'dev':
                    self._info("Block header fetch failed")
        else:
            parts.append(f"{C.RED}offline{C.R}")
        if self._ctx_epoch is not None:
            parts.append(f"{C.Y}epoch {self._ctx_epoch}{C.R}")
        acct = self._ctx_account
        admin_tag = (f" {C.DIM}(admin){C.R}" if acct == 'alice' else "")
        parts.append(f"account: {C.W}{acct}{C.R}{admin_tag}")
        print(f"  {'  '.join(parts)}")

        # Epoch dashboard
        status = self._fetch_epoch_status()
        if status:
            eid = status.get('epoch_id', 0)
            state = status.get('epoch_state', 'None')
            pres = status.get('presence_state')
            is_part = status.get('is_participant', False)
            cur_blk = status.get('current_block', 0)
            end_blk = status.get('end_block', 0)
            pc = status.get('participant_count', 0)

            if self._mode == 'normal':
                state_labels = {
                    'None': 'No Session', 'Scheduled': 'Upcoming',
                    'Active': 'In Progress', 'Closed': 'Wrapping Up',
                    'Finalized': 'Complete',
                }
                sl = state_labels.get(state, state)
                print(f"  Session: {C.W}E{eid}{C.R}  "
                      f"State: {C.W}{sl}{C.R}  "
                      f"Participants: {C.W}{pc}{C.R}")
            else:
                print(f"  Epoch: {C.W}{eid}{C.R}  "
                      f"State: {C.W}{state}{C.R}  "
                      f"Participants: {C.W}{pc}{C.R}")

            # Progress bar for active epoch
            if state == 'Active' and end_blk > 0:
                start = status.get('start_block', 0)
                if end_blk > start:
                    pct = min(100, max(0, int(
                        (cur_blk - start) /
                        (end_blk - start) * 100)))
                    remaining = max(0, end_blk - cur_blk)
                    bar_len = 20
                    filled = int(bar_len * pct / 100)
                    bar = ('█' * filled +
                           '░' * (bar_len - filled))
                    print(f"  {C.G}{bar}{C.R} {pct}%  "
                          f"{C.DIM}~{remaining} blocks left{C.R}")

            # Participation status
            if pres == 'Finalized':
                print(f"  You: {C.W}Verified{C.R}")
            elif pres == 'Slashed':
                print(f"  You: {C.RED}Slashed{C.R}")
            elif pres == 'Validated':
                print(f"  You: {C.BB}Validated{C.R}")
            elif pres == 'Declared':
                vc = status.get('vote_count', 0)
                qt = status.get('quorum_threshold', 2)
                print(f"  You: {C.CY}Declared "
                      f"({vc}/{qt} votes){C.R}")
            elif is_part:
                print(f"  You: {C.G}Registered{C.R}")
            elif state not in ('None', 'Finalized'):
                print(f"  You: {C.DIM}Not participating{C.R}")

            print(f"  {C.DIM}Tip: type 'flow' for next steps{C.R}")

    def bootstrap(self):
        self._auto_setup_validators()

    def _check_epoch(self):
        if not self._ensure():
            return False
        try:
            epoch_r = self.substrate.query("Epoch", "CurrentEpoch")
            epoch_id = epoch_r.value if epoch_r else 0
            if epoch_id > 0:
                info = self.substrate.query(
                    "Epoch", "EpochInfo", [epoch_id])
                if info and info.value:
                    state = info.value.get('state', 'None')
                    if state == 'Active':
                        return True
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost while checking epoch status")
            return False
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"Epoch check failed: {e}")
        self._info("No active session found.")
        if self._prompt_bool(
                "Run bootstrap? (sets up session + validators + positions)"):
            self.bootstrap()
            return True
        return False

    # ------------------------------------------------------------------
    # Epoch status and flow
    # ------------------------------------------------------------------

    def _fetch_epoch_status(self):
        """Fetch and cache epoch state + user participation.
        Caches for 5 seconds to avoid RPC spam on every prompt.
        """
        now = time.time()
        if (hasattr(self, '_epoch_cache')
                and self._epoch_cache
                and now - self._epoch_cache.get('_ts', 0) < 5):
            return self._epoch_cache

        if not self.connected or not self.substrate:
            return None

        try:
            result = {}

            # Current epoch from Epoch pallet
            epoch_r = self.substrate.query("Epoch", "CurrentEpoch")
            epoch_id = epoch_r.value if epoch_r else 0
            result['epoch_id'] = epoch_id

            # Epoch metadata (state, blocks, participants)
            info = self.substrate.query("Epoch", "EpochInfo", [epoch_id])
            if info and info.value:
                result['epoch_state'] = info.value.get('state', 'Unknown')
                result['start_block'] = info.value.get('start_block', 0)
                result['end_block'] = info.value.get('end_block', 0)
                result['participant_count'] = info.value.get(
                    'participant_count', 0)
            else:
                result['epoch_state'] = 'None'

            # Check participation for current account
            acct = self._ctx_account
            if acct in self.keypairs:
                kp = self.keypairs[acct]
                try:
                    is_part = self.substrate.query(
                        "Epoch", "EpochParticipants",
                        [epoch_id, kp.ss58_address])
                    result['is_participant'] = bool(
                        is_part and is_part.value)
                except (ConnectionError, BrokenPipeError, OSError):
                    result['is_participant'] = False
                except Exception as e:
                    if self._mode == 'dev':
                        self._info(
                            f"Participant query failed: {e}")
                    result['is_participant'] = False

                # Check presence state
                aid = self._actor_id(acct)
                pres_epoch = self._ctx_epoch or epoch_id
                try:
                    pres = self.substrate.query(
                        "Presence", "Presences",
                        [pres_epoch, aid])
                    if pres and pres.value:
                        result['presence_state'] = pres.value.get(
                            'state', None)
                        result['vote_count'] = pres.value.get(
                            'vote_count', 0)
                    else:
                        result['presence_state'] = None
                except (ConnectionError, BrokenPipeError, OSError):
                    result['presence_state'] = None
                except Exception as e:
                    if self._mode == 'dev':
                        self._info(
                            f"Presence query failed: {e}")
                    result['presence_state'] = None

            # Quorum config
            try:
                qc = self.substrate.query(
                    "Presence", "QuorumConfigStorage")
                if qc and qc.value:
                    result['quorum_threshold'] = qc.value.get(
                        'threshold', 2)
                else:
                    result['quorum_threshold'] = 2
            except (ConnectionError, BrokenPipeError, OSError):
                result['quorum_threshold'] = 2
            except Exception as e:
                if self._mode == 'dev':
                    self._info(f"Quorum query failed: {e}")
                result['quorum_threshold'] = 2

            # Current block
            try:
                header = self.substrate.get_block_header()['header']
                result['current_block'] = int(header['number'])
            except (ConnectionError, BrokenPipeError, OSError):
                result['current_block'] = 0
            except Exception as e:
                if self._mode == 'dev':
                    self._info(
                        f"Block header query failed: {e}")
                result['current_block'] = 0

            result['_ts'] = now
            self._epoch_cache = result
            return result

        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
            return None
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"Epoch status fetch failed: {e}")
            return None

    def _epoch_status_tag(self):
        """Build colorized epoch status tag for the prompt."""
        status = self._fetch_epoch_status()
        if not status:
            return ""

        eid = status.get('epoch_id', 0)
        state = status.get('epoch_state', 'None')

        if state == 'None' and eid == 0:
            return f" {C.DIM}[no epoch]{C.R}"

        state_map = {
            'Scheduled': (C.Y, 'SCHED'),
            'Active': (C.G, 'ACTIVE'),
            'Closed': (C.DIM, 'CLOSED'),
            'Finalized': (C.DIM, 'FINAL'),
        }
        color, label = state_map.get(state, (C.DIM, state[:6]))

        # Participation tag
        pres = status.get('presence_state')
        is_part = status.get('is_participant', False)

        if pres == 'Finalized':
            ptag = f"{C.W}DONE"
        elif pres == 'Slashed':
            ptag = f"{C.RED}SLASHED"
        elif pres == 'Validated':
            ptag = f"{C.BB}VALID"
        elif pres == 'Declared':
            vc = status.get('vote_count', 0)
            qt = status.get('quorum_threshold', 2)
            ptag = f"{C.CY}DECL {vc}/{qt}"
        elif is_part:
            ptag = f"{C.G}REG"
        else:
            ptag = f"{C.DIM}--"

        return f" [{color}E{eid}:{label}{C.R}|{ptag}{C.R}]"

    def _show_epoch_flow(self):
        """Show contextual actions based on epoch state and participation."""
        status = self._fetch_epoch_status()
        if not status:
            self._err("Cannot fetch epoch status. Are you connected?")
            return

        eid = status.get('epoch_id', 0)
        state = status.get('epoch_state', 'None')
        pres = status.get('presence_state')
        is_part = status.get('is_participant', False)
        vc = status.get('vote_count', 0)
        qt = status.get('quorum_threshold', 2)
        cur_blk = status.get('current_block', 0)
        end_blk = status.get('end_block', 0)

        # Header
        if self._mode == 'normal':
            state_labels = {
                'None': 'No Session', 'Scheduled': 'Upcoming',
                'Active': 'In Progress', 'Closed': 'Wrapping Up',
                'Finalized': 'Complete',
            }
            sl = state_labels.get(state, state)
            self._header(f"SESSION {eid}  [{sl}]")
        else:
            self._header(f"EPOCH {eid}  [{state}]")

        # Progress
        if state == 'Active' and end_blk > 0:
            start = status.get('start_block', 0)
            if end_blk > start:
                pct = min(100, max(0, int(
                    (cur_blk - start) / (end_blk - start) * 100)))
                remaining = max(0, end_blk - cur_blk)
                bar_len = 20
                filled = int(bar_len * pct / 100)
                bar = f"{'█' * filled}{'░' * (bar_len - filled)}"
                print(f"  {C.G}{bar}{C.R} {pct}%  "
                      f"{C.DIM}~{remaining} blocks left{C.R}")
                print()

        # Participation summary
        pc = status.get('participant_count', 0)
        if self._mode == 'normal':
            print(f"  Participants: {C.W}{pc}{C.R}    "
                  f"Your status: ", end="")
        else:
            print(f"  Participants: {C.W}{pc}{C.R}  |  "
                  f"Presence: ", end="")

        if pres == 'Finalized':
            print(f"{C.W}Verified and locked{C.R}")
        elif pres == 'Slashed':
            print(f"{C.RED}Rejected{C.R}")
        elif pres == 'Validated':
            print(f"{C.BB}Confirmed, ready to finalize{C.R}")
        elif pres == 'Declared':
            needed = max(0, qt - vc)
            print(f"{C.CY}Checked in, need {needed} more "
                  f"vote(s) ({vc}/{qt}){C.R}")
        elif is_part:
            print(f"{C.G}Registered, not yet checked in{C.R}")
        else:
            print(f"{C.DIM}Not participating{C.R}")
        print()

        # Actions
        if self._mode == 'normal':
            print(f"  {C.BB}WHAT TO DO NEXT:{C.R}")
        else:
            print(f"  {C.BB}AVAILABLE ACTIONS:{C.R}")

        actions = []

        if state == 'None' or (state == 'None' and eid == 0):
            actions.append((True,
                "Schedule a new time period",
                "epoch > 1 (Schedule)"))
            actions.append((True,
                "Create a vault (no epoch needed)",
                "vault > 1 (Create)"))

        elif state == 'Scheduled':
            actions.append((not is_part,
                "Register for this epoch",
                "epoch > 5 (Register)"))
            actions.append((True,
                "Start the epoch (admin)",
                "epoch > 2 (Start)"))

        elif state == 'Active':
            if not is_part:
                actions.append((True,
                    "Register as participant (required first)",
                    "epoch > 5"))
            elif pres is None:
                actions.append((True,
                    "Declare your presence",
                    "presence > 1"))
                actions.append((True,
                    "Declare with commitment (private)",
                    "presence > 2"))
            elif pres == 'Declared':
                needed = max(0, qt - vc)
                actions.append((False,
                    f"Waiting for {needed} more vote(s) "
                    f"({vc}/{qt})",
                    None))
                actions.append((True,
                    "Vote on another's presence (validator)",
                    "presence > 4"))
                actions.append((True,
                    "Submit witness attestation",
                    "pbt > 3"))
            elif pres == 'Validated':
                actions.append((True,
                    "Finalize your presence (quorum met!)",
                    "presence > 5"))
                actions.append((True,
                    "Secure a document in vault",
                    "vault > 9"))
            elif pres == 'Finalized':
                actions.append((False,
                    "Presence complete! You are verified.",
                    None))
                actions.append((True,
                    "Secure a document in vault",
                    "vault > 9"))
                actions.append((True,
                    "Vote on others' presence (validator)",
                    "presence > 4"))
            elif pres == 'Slashed':
                actions.append((True,
                    "Open a dispute if you disagree",
                    "dispute > 1"))

        elif state == 'Closed':
            actions.append((True,
                "Finalize remaining presences",
                "presence > 5"))
            actions.append((True,
                "Open dispute if needed",
                "dispute > 1"))
            actions.append((True,
                "Close the epoch (admin)",
                "epoch > 3"))

        elif state == 'Finalized':
            actions.append((False,
                "Epoch is finalized. Records are locked.",
                None))
            actions.append((True,
                "Unlock secured documents",
                "vault > 10"))
            actions.append((True,
                "View your presence record",
                "presence > b"))

        # Always-available
        actions.append((True,
            "View full status",
            "status"))

        for actionable, label, shortcut in actions:
            if actionable and shortcut:
                print(f"    {C.G}>{C.R} {label}")
                print(f"      {C.DIM}run: {shortcut}{C.R}")
            elif not actionable and shortcut:
                print(f"    {C.CY}>{C.R} {label}")
                print(f"      {C.DIM}run: {shortcut}{C.R}")
            else:
                print(f"    {C.DIM}  {label}{C.R}")
        print()

    def _next_test_epoch(self):
        try:
            count_r = self.substrate.query("Epoch", "EpochCount")
            next_id = (count_r.value if count_r else 0) + 1
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
            next_id = 2
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"EpochCount query failed: {e}")
            next_id = 2
        try:
            header = self.substrate.get_block_header()['header']
            current_block = int(header['number'])
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
            current_block = 10
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"Block header query failed: {e}")
            current_block = 10
        self._info(f"Scheduling session {next_id}")
        self._submit("Epoch", "schedule_epoch",
                     {"start_block": current_block + 5, "duration": 200},
                     sudo=True, _skip_confirm=True)
        self._info(f"Starting session {next_id}")
        self._submit("Epoch", "start_epoch",
                     {"epoch_id": next_id},
                     sudo=True, _skip_confirm=True)
        return next_id

    # ------------------------------------------------------------------
    # Custom handlers: Presence
    # ------------------------------------------------------------------

    def _presence_commit(self):
        e = self._prompt_epoch()
        a = self._prompt_account("Signer")
        sec = secrets.token_hex(32)
        rnd = secrets.token_hex(32)
        h = hashlib.blake2b(
            bytes.fromhex(sec + rnd), digest_size=32).hexdigest()
        print(f"  {C.DIM}Secret:     {sec[:32]}...{C.R}")
        print(f"  {C.DIM}Randomness: {rnd[:32]}...{C.R}")
        print(f"  {C.DIM}Commitment: 0x{h[:32]}...{C.R}")
        self._submit("Presence", "declare_presence_with_commitment",
                     {"epoch": e, "commitment": "0x" + h}, a)

    def _presence_commitment_count(self):
        e = self._prompt_epoch()
        self._val("Commitments",
                  self._query("Presence", "CommitmentCount", [e]))
        self._val("Reveals",
                  self._query("Presence", "RevealCount", [e]))

    # ------------------------------------------------------------------
    # Custom handlers: Validator
    # ------------------------------------------------------------------

    def _validator_count_stake(self):
        self._val("Count", self._query("Validator", "ValidatorCount"))
        self._val("Total Stake", self._query("Validator", "TotalStake"))

    # ------------------------------------------------------------------
    # Custom handlers: PBT / Bootstrap
    # ------------------------------------------------------------------

    def _pbt_set_position(self):
        name = self._prompt_account("Validator")
        vid = self._validator_id(name)
        pos = self._prompt_position("Validator position")
        self._submit("Presence", "set_validator_position",
                     {"validator": vid, "position": pos}, name)

    def _auto_setup_validators(self):
        if not self._ensure():
            return
        positions = {
            'alice':   {"x": 0,      "y": 0,      "z": 0},
            'bob':     {"x": 50000,  "y": 0,      "z": 0},
            'charlie': {"x": 25000,  "y": 43301,  "z": 0},
            'dave':    {"x": -25000, "y": 43301,  "z": 0},
            'eve':     {"x": -50000, "y": 0,      "z": 0},
            'ferdie':  {"x": -25000, "y": -43301, "z": 0},
        }
        # Steps: schedule + start + quorum + N*(register + activate + position)
        total = 3 + len(positions) * 3
        step = 0

        # Step 1: Get current block for scheduling
        try:
            header = self.substrate.get_block_header()['header']
            current_block = int(header['number'])
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
            current_block = 1
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"Block header query failed: {e}")
            current_block = 1

        # Step 2: Schedule epoch 1
        step += 1
        self._progress(step, total, "Scheduling session 1")
        r = self._submit("Epoch", "schedule_epoch",
                         {"start_block": current_block + 3,
                          "duration": 200},
                         sudo=True, _skip_confirm=True)
        if r and not r.is_success:
            # Epoch may already exist, try starting it directly
            self._info("Session may already be scheduled, continuing...")

        # Step 3: Start epoch 1
        step += 1
        self._progress(step, total, "Starting session 1")
        r = self._submit("Epoch", "start_epoch",
                         {"epoch_id": 1},
                         sudo=True, _skip_confirm=True)
        if r and not r.is_success:
            self._info("Session may already be active, continuing...")

        # Step 4: Set quorum config
        step += 1
        self._progress(step, total, "Setting approval threshold (2 of 3)")
        self._submit("Presence", "set_quorum_config",
                     {"threshold": 2, "total": 3},
                     sudo=True, _skip_confirm=True)

        # Step 5: Register, activate, and position each validator
        for name, pos in positions.items():
            vid = self._validator_id(name)

            step += 1
            self._progress(step, total,
                           f"Register {C.W}{name}{C.R}")
            self._submit("Validator", "register_validator",
                         {"stake": 1000000000000},
                         name, _skip_confirm=True)

            step += 1
            self._progress(step, total,
                           f"Activate {C.W}{name}{C.R}")
            self._submit("Validator", "activate_validator",
                         {}, name, _skip_confirm=True)

            step += 1
            self._progress(step, total,
                           f"Position {C.W}{name}{C.R} "
                           f"({pos['x']}, {pos['y']}, {pos['z']})")
            self._submit("Presence", "set_validator_position",
                         {"validator": vid, "position": pos}, name)

        self._ok("Bootstrap complete — 6 validators in hexagonal"
                 " formation")

    def _auto_pbt_test(self):
        if not self._ensure():
            return
        self._check_epoch()
        epoch = self._next_test_epoch()
        alice_id = self._actor_id('alice')
        claim = {"x": 16666, "y": 28867, "z": 0}

        self._header("PBT TEST FLOW")
        self._info(f"Epoch {epoch} — Alice claims "
                   f"({claim['x']}, {claim['y']}, {claim['z']})")
        self._info("  = centroid of bob, charlie, dave "
                   "(equal-weight triangulation)")
        self._submit("Presence", "claim_position",
                     {"epoch": epoch, "position": claim}, "alice")

        for w in ['bob', 'charlie', 'dave']:
            self._info(f"{w} attesting (10ms RTT)...")
            self._submit("Presence", "submit_witness_attestation",
                         {"target": alice_id, "epoch": epoch,
                          "latency_ms": 10,
                          "direct_connection": True}, w)

        self._info("Verifying position via triangulation...")
        self._submit("Presence", "verify_position",
                     {"target": alice_id, "epoch": epoch}, "bob")

        r = self._query("Presence", "PositionClaims",
                        [epoch, alice_id])
        self._val("Result", r)
        if r and hasattr(r, 'value'):
            rv = r.value if hasattr(r, 'value') else r
            if isinstance(rv, dict):
                v = rv.get('verified', False)
                c = rv.get('confidence', 0)
                self._val("Verified", v)
                self._val("Confidence", f"{c}%")
        self._ok("PBT test complete!")

    # ------------------------------------------------------------------
    # Custom handlers: Triangulation
    # ------------------------------------------------------------------

    def _triangulation_report_signal(self):
        rid = self._prompt_int("Reporter ID", 0)
        mac = self._prompt_h256("MAC hash")
        rssi = self._prompt_int("RSSI (dBm)", -60)
        st = self._prompt_enum("Signal:", [
            "NetworkLatency", "PeerTopology", "BlockPropagation",
            "IPGeolocation", "GPSConsent", "ConsensusWitness"])
        freq = self._prompt_int("Freq MHz (0=none)", 0)
        a = self._prompt_account()
        self._submit("Triangulation", "report_signal",
                     {"reporter_id": rid, "mac_hash": mac, "rssi": rssi,
                      "signal_type": st,
                      "frequency": None if freq == 0 else freq}, a)

    def _triangulation_fraud_proof(self):
        sub = self._prompt_int("Submitter reporter ID", 0)
        acc = self._prompt_int("Accused reporter ID", 1)
        z = self._prompt_int("Z-score x100", 350)
        n = self._prompt_int("Sample size", 10)
        a = self._prompt_account()
        self._submit("Triangulation", "submit_fraud_proof",
                     {"submitter_id": sub, "proof": {
                         "accused_reporter": acc,
                         "conflicting_readings": [],
                         "z_score_scaled": z, "sample_size": n}}, a)

    def _triangulation_counts(self):
        self._val("Devices",
                  self._query("Triangulation", "DeviceCount"))
        self._val("Ghosts",
                  self._query("Triangulation", "GhostCount"))

    # ------------------------------------------------------------------
    # Custom handlers: Device
    # ------------------------------------------------------------------

    def _device_activate(self):
        did = self._prompt_int("Device ID", 0)
        a = self._prompt_account()
        act = self._prompt_enum(
            "Action:", ["activate_device", "reactivate_device"])
        self._submit("Device", act, {"device_id": did}, a)

    def _device_revoke(self):
        did = self._prompt_int("Device ID", 0)
        act = self._prompt_enum(
            "Action:", ["revoke_device", "mark_compromised"])
        a = self._prompt_account()
        self._submit("Device", act, {"device_id": did}, a)

    def _device_attestation(self):
        did = self._prompt_int("Device ID", 0)
        ah = self._prompt_h256("Attestation hash")
        a = self._prompt_account()
        self._submit("Device", "submit_attestation",
                     {"device_id": did, "attestation_hash": ah,
                      "attester": None}, a)

    # ------------------------------------------------------------------
    # Custom handlers: Lifecycle
    # ------------------------------------------------------------------

    def _lifecycle_suspend_reactivate(self):
        actor = self._prompt_actor("Actor")
        act = self._prompt_enum(
            "Action:", ["suspend_actor", "reactivate_actor"])
        self._submit("Lifecycle", act, {"actor": actor}, sudo=True)

    def _lifecycle_count(self):
        self._val("Count", self._query("Lifecycle", "ActorCount"))
        self._val("Active", self._query("Lifecycle", "ActiveActors"))

    # ------------------------------------------------------------------
    # Custom handlers: Governance
    # ------------------------------------------------------------------

    def _gov_grant(self):
        grantee = self._prompt_actor("Grantee")
        res = self._prompt_h256("Resource ID")
        perms = self._prompt_int(
            "Permissions bitmask (R=1 W=2 X=4 D=8 A=16)", 7)
        deleg = self._prompt_bool("Delegatable?")
        a = self._prompt_account()
        self._submit("Governance", "grant_capability",
                     {"grantee": grantee, "resource": res,
                      "permissions": perms, "expires_at": None,
                      "delegatable": deleg}, a)

    def _gov_delegate(self):
        cid = self._prompt_int("Capability ID", 0)
        dele = self._prompt_actor("Delegatee")
        p = self._prompt_int("Permissions", 1)
        a = self._prompt_account()
        self._submit("Governance", "delegate_capability",
                     {"capability_id": cid, "delegatee": dele,
                      "permissions": p, "expires_at": None}, a)

    # ------------------------------------------------------------------
    # Custom handlers: Semantic
    # ------------------------------------------------------------------

    def _semantic_create(self):
        to = self._prompt_actor("To actor")
        rtype = self._prompt("Relationship type", "Trust")
        trust = self._prompt_int("Trust (0-100)", 50)
        bidir = self._prompt_bool("Bidirectional?")
        a = self._prompt_account()
        self._submit("Semantic", "create_relationship",
                     {"to_actor": to, "relationship_type": rtype,
                      "trust_level": trust, "expires_at": None,
                      "bidirectional": bidir}, a)

    # ------------------------------------------------------------------
    # Custom handlers: ZK
    # ------------------------------------------------------------------

    def _zk_share_proof(self):
        cm = self._prompt_h256("Commitment hash")
        pr = self._prompt("Proof hex", "00" * 32)
        a = self._prompt_account()
        self._submit("Zk", "verify_share_proof",
                     {"statement": {"commitment_hash": cm},
                      "proof": "0x" + pr}, a)

    def _zk_presence_proof(self):
        actor = self._prompt_actor("Actor")
        e = self._prompt_epoch()
        pr = self._prompt("Proof hex", "00" * 32)
        a = self._prompt_account()
        self._submit("Zk", "verify_presence_proof",
                     {"statement": {"actor": actor, "epoch": e},
                      "proof": "0x" + pr}, a)

    def _zk_access_proof(self):
        actor = self._prompt_actor("Actor")
        res = self._prompt_h256("Resource ID")
        pr = self._prompt("Proof hex", "00" * 32)
        a = self._prompt_account()
        self._submit("Zk", "verify_access_proof",
                     {"statement": {"actor": actor, "resource": res},
                      "proof": "0x" + pr}, a)

    def _zk_register_circuit(self):
        cid = self._prompt_h256("Circuit ID")
        pt = self._prompt_enum("Type:", ["Groth16", "PlonK", "Halo2"])
        vk = self._prompt("VK hex", "00" * 32)
        self._submit("Zk", "register_circuit",
                     {"circuit_id": cid, "proof_type": pt,
                      "vk": "0x" + vk}, sudo=True)

    def _zk_verify_snark(self):
        cid = self._prompt_h256("Circuit ID")
        pr = self._prompt("Proof hex", "00" * 64)
        a = self._prompt_account()
        self._submit("Zk", "verify_snark",
                     {"circuit_id": cid, "proof": "0x" + pr,
                      "inputs": []}, a)

    def _zk_trusted_verifier(self):
        act = self._prompt_enum(
            "Action:", ["add_trusted_verifier", "remove_trusted_verifier"])
        v = self._prompt_actor("Verifier")
        self._submit("Zk", act, {"verifier": v}, sudo=True)

    # ------------------------------------------------------------------
    # Custom handlers: Octopus
    # ------------------------------------------------------------------

    def _octopus_create_cluster(self):
        a = self._prompt_account()
        owner = self._prompt_actor("Owner")
        self._submit("Octopus", "create_cluster",
                     {"owner": owner}, a)

    def _octopus_register_subnode(self):
        a = self._prompt_account()
        cid = self._prompt_int("Cluster ID", 0)
        op = self._prompt_actor("Operator")
        self._submit("Octopus", "register_subnode",
                     {"cluster_id": cid, "operator": op}, a)

    def _octopus_activate_subnode(self):
        a = self._prompt_account()
        self._submit("Octopus", "activate_subnode",
                     {"subnode_id": self._prompt_int("Subnode ID", 0)}, a)

    def _octopus_start_deactivation(self):
        a = self._prompt_account()
        self._submit("Octopus", "start_deactivation",
                     {"subnode_id": self._prompt_int("Subnode ID", 0)}, a)

    def _octopus_update_throughput(self):
        a = self._prompt_account()
        cid = self._prompt_int("Cluster ID", 0)
        tp = self._prompt_int("Throughput score", 450000000)
        self._submit("Octopus", "update_throughput",
                     {"cluster_id": cid, "throughput": tp}, a)

    def _octopus_evaluate_scaling(self):
        a = self._prompt_account()
        self._submit("Octopus", "evaluate_scaling",
                     {"cluster_id": self._prompt_int("Cluster ID", 0)}, a)

    def _octopus_update_subnode_throughput(self):
        a = self._prompt_account()
        sid = self._prompt_int("Subnode ID", 0)
        tp = self._prompt_int("Throughput score", 500000000)
        pr = self._prompt_int("Processed", 100)
        self._submit("Octopus", "update_subnode_throughput",
                     {"subnode_id": sid, "throughput": tp,
                      "processed": pr}, a)

    def _octopus_record_heartbeat(self):
        a = self._prompt_account()
        self._submit("Octopus", "record_heartbeat",
                     {"subnode_id": self._prompt_int("Subnode ID", 0)}, a)

    def _octopus_device_observation(self):
        a = self._prompt_account()
        sid = self._prompt_int("Subnode ID", 0)
        dc = self._prompt_int("Device count", 5)
        cm = self._prompt_h256("Commitment hash")
        self._submit("Octopus", "record_device_observation",
                     {"subnode_id": sid, "device_count": dc,
                      "commitment": cm}, a)

    def _octopus_position_confirmation(self):
        a = self._prompt_account()
        sid = self._prompt_int("Subnode ID", 0)
        x = self._prompt_int("X", 0)
        y = self._prompt_int("Y", 0)
        z = self._prompt_int("Z", 0)
        self._submit("Octopus", "record_position_confirmation",
                     {"subnode_id": sid, "position_x": x,
                      "position_y": y, "position_z": z}, a)

    def _octopus_heartbeat_device_proof(self):
        a = self._prompt_account()
        sid = self._prompt_int("Subnode ID", 0)
        dc = self._prompt_int("Device count", 5)
        cm = self._prompt_h256("Commitment")
        self._submit("Octopus", "heartbeat_with_device_proof",
                     {"subnode_id": sid, "device_count": dc,
                      "commitment": cm}, a)

    def _octopus_set_fusion_weights(self):
        a = self._prompt_account()
        hw = self._prompt_int("Heartbeat weight", 40)
        dw = self._prompt_int("Device weight", 40)
        pw = self._prompt_int("Position weight", 20)
        self._submit("Octopus", "set_fusion_weights",
                     {"heartbeat_weight": hw, "device_weight": dw,
                      "position_weight": pw}, a)

    # ------------------------------------------------------------------
    # Custom handlers: Chain Status
    # ------------------------------------------------------------------

    def _chain_health(self):
        if not self._ensure():
            return
        r = self.substrate.rpc_request("system_health", [])['result']
        self._val("Peers", r.get('peers', 0))
        self._val("Syncing", r.get('isSyncing', False))
        self._val("Chain",
                  self.substrate.rpc_request("system_chain", [])['result'])
        self._val("Version",
                  self.substrate.rpc_request(
                      "system_version", [])['result'])

    def _chain_latest_block(self):
        if not self._ensure():
            return
        h = self.substrate.get_block_header()
        self._val("Block", h['header']['number'])
        self._val("Hash", self.substrate.get_block_hash())

    def _chain_runtime_version(self):
        if not self._ensure():
            return
        rv = self.substrate.rpc_request(
            "state_getRuntimeVersion", [])['result']
        for k in ['specName', 'specVersion', 'implVersion',
                  'transactionVersion']:
            self._val(k, rv.get(k))

    def _chain_balances(self):
        if not self._ensure():
            return
        for name, kp in self.keypairs.items():
            r = self.substrate.query(
                'System', 'Account', [kp.ss58_address])
            free = r.value['data']['free'] if r else 0
            self._val(f"{name:>8}", f"{free / 1e12:.4f} UNIT")

    def _chain_events(self):
        if not self._ensure():
            return
        events = self.substrate.query('System', 'Events')
        if events and events.value:
            for ev in events.value[-15:]:
                mid = ev.get('event', {}).get('module_id', '?')
                eid = ev.get('event', {}).get('event_id', '?')
                print(f"    {C.DIM}{mid}.{eid}{C.R}")

    def _chain_pallets(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        for p in md.pallets:
            nc = len(p.calls) if p.calls else 0
            ns = len(p.storage) if p.storage else 0
            print(f"    {C.B}{p.name:>20}{C.R}  calls={nc}  storage={ns}")

    # ------------------------------------------------------------------
    # Custom handlers: Block Explorer
    # ------------------------------------------------------------------

    def _blocks_by_number(self):
        if not self._ensure():
            return
        num = self._prompt_int("Block number", 1)
        bh = self.substrate.get_block_hash(num)
        if bh:
            self._val("Hash", bh)
            block = self.substrate.get_block(block_hash=bh)
            header = block['header']
            self._val("Number", header['number'])
            self._val("Parent", header['parentHash'])
            self._val("State Root", header['stateRoot'])
            self._val("Extrinsics Root", header['extrinsicsRoot'])
            exts = block.get('extrinsics', [])
            self._val("Extrinsic Count", len(exts))
        else:
            self._err("Block not found")

    def _blocks_by_hash(self):
        if not self._ensure():
            return
        bh = self._prompt("Block hash", "")
        if bh:
            block = self.substrate.get_block(block_hash=bh)
            if block:
                header = block['header']
                self._val("Number", header['number'])
                self._val("Parent", header['parentHash'])
                self._val("State Root", header['stateRoot'])
                self._val("Extrinsics Root", header['extrinsicsRoot'])
                exts = block.get('extrinsics', [])
                self._val("Extrinsic Count", len(exts))
            else:
                self._err("Block not found")

    def _blocks_latest(self):
        if not self._ensure():
            return
        header = self.substrate.get_block_header()['header']
        bh = self.substrate.get_block_hash()
        self._val("Number", header['number'])
        self._val("Hash", bh)
        self._val("Parent", header['parentHash'])
        self._val("State Root", header['stateRoot'])
        self._val("Extrinsics Root", header['extrinsicsRoot'])
        if 'digest' in header and 'logs' in header['digest']:
            for i, log in enumerate(header['digest']['logs']):
                print(f"    {C.DIM}Digest[{i}]: "
                      f"{str(log)[:80]}{C.R}")

    def _blocks_decode_ext(self):
        if not self._ensure():
            return
        num = self._prompt_int("Block number (0=latest)", 0)
        bh = (self.substrate.get_block_hash(num)
              if num > 0 else self.substrate.get_block_hash())
        block = self.substrate.get_block(block_hash=bh)
        exts = block.get('extrinsics', [])
        if not exts:
            self._info("No extrinsics in this block")
            return
        rows = []
        for i, ext in enumerate(exts):
            call = ext.value if hasattr(ext, 'value') else ext
            call_data = (call.get('call', {})
                         if isinstance(call, dict) else {})
            mod = call_data.get('call_module', '?')
            fn = call_data.get('call_function', '?')
            rows.append([i, mod, fn])
        self._table(["#", "Module", "Function"], rows)
        idx = self._prompt_int("Decode index", 0)
        if 0 <= idx < len(exts):
            ext = exts[idx]
            val = ext.value if hasattr(ext, 'value') else ext
            print(f"\n  {C.W}Extrinsic [{idx}]:{C.R}")
            if isinstance(val, dict):
                call_data = val.get('call', {})
                self._val("Module", call_data.get('call_module', '?'))
                self._val("Function",
                          call_data.get('call_function', '?'))
                args = call_data.get('call_args', [])
                if args:
                    print(f"  {C.CY}Arguments:{C.R}")
                    for arg in args:
                        name = arg.get('name', '?')
                        value = arg.get('value', '?')
                        print(f"    {C.DIM}{name}:{C.R} "
                              f"{C.W}{value}{C.R}")
            else:
                print(f"    {C.DIM}{val}{C.R}")

    def _blocks_events(self):
        if not self._ensure():
            return
        num = self._prompt_int("Block number (0=latest)", 0)
        bh = (self.substrate.get_block_hash(num)
              if num > 0 else self.substrate.get_block_hash())
        events = self.substrate.query("System", "Events", block_hash=bh)
        if events and events.value:
            for i, ev in enumerate(events.value):
                mid = ev.get('event', {}).get('module_id', '?')
                eid = ev.get('event', {}).get('event_id', '?')
                attrs = ev.get('event', {}).get('attributes', '')
                attr_str = (f" {C.DIM}{str(attrs)[:60]}{C.R}"
                            if attrs else "")
                print(f"    {C.DIM}[{i:>3}]{C.R} "
                      f"{C.W}{mid}.{eid}{C.R}{attr_str}")
        else:
            self._info("No events at this block")

    def _blocks_finalized(self):
        if not self._ensure():
            return
        fh = self.substrate.rpc_request(
            "chain_getFinalizedHead", [])['result']
        self._val("Finalized Hash", fh)
        header = self.substrate.get_block_header(block_hash=fh)
        self._val("Finalized Block", header['header']['number'])

    def _blocks_compare(self):
        if not self._ensure():
            return
        n1 = self._prompt_int("Block number A", 1)
        n2 = self._prompt_int("Block number B", 2)
        h1 = self.substrate.get_block_hash(n1)
        h2 = self.substrate.get_block_hash(n2)
        b1 = self.substrate.get_block(block_hash=h1)
        b2 = self.substrate.get_block(block_hash=h2)
        hdr1, hdr2 = b1['header'], b2['header']
        print(f"\n  {C.W}{'Field':>20}  "
              f"{'Block '+str(n1):>30}  "
              f"{'Block '+str(n2):>30}{C.R}")
        print(f"  {C.DIM}{'─'*84}{C.R}")
        for field in ['stateRoot', 'extrinsicsRoot', 'parentHash']:
            v1 = str(hdr1.get(field, ''))[:28]
            v2 = str(hdr2.get(field, ''))[:28]
            diff = (" *"
                    if hdr1.get(field) != hdr2.get(field) else "")
            print(f"  {C.CY}{field:>20}{C.R}  {v1:>30}  "
                  f"{v2:>30}{C.Y}{diff}{C.R}")
        ext1 = len(b1.get('extrinsics', []))
        ext2 = len(b2.get('extrinsics', []))
        diff = " *" if ext1 != ext2 else ""
        print(f"  {C.CY}{'extrinsicCount':>20}{C.R}  {ext1:>30}  "
              f"{ext2:>30}{C.Y}{diff}{C.R}")

    # ------------------------------------------------------------------
    # Custom handlers: Storage Inspector
    # ------------------------------------------------------------------

    def _si_query_pallet(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        pallets = [p.name for p in md.pallets if p.storage]
        print(f"  {C.DIM}Pallets with storage:{C.R}")
        for i, name in enumerate(pallets):
            print(f"    {C.Y}{i+1:>3}{C.R} {name}")
        idx = self._prompt_int("Pallet #", 1) - 1
        if 0 <= idx < len(pallets):
            pallet_name = pallets[idx]
            pallet = [p for p in md.pallets
                      if p.name == pallet_name][0]
            items = [s.name for s in pallet.storage]
            print(f"  {C.DIM}Storage items in {pallet_name}:{C.R}")
            for i, name in enumerate(items):
                print(f"    {C.Y}{i+1:>3}{C.R} {name}")
            sidx = self._prompt_int("Item #", 1) - 1
            if 0 <= sidx < len(items):
                item_name = items[sidx]
                params_str = self._prompt(
                    "Parameters (comma-separated, or empty)", "")
                params = ([p.strip() for p in params_str.split(",")
                           if p.strip()] if params_str else [])
                converted = []
                for p in params:
                    try:
                        converted.append(int(p))
                    except ValueError:
                        converted.append(p)
                result = self.substrate.query(
                    pallet_name, item_name, converted or None)
                self._val(f"{pallet_name}.{item_name}", result)

    def _si_raw_key(self):
        if not self._ensure():
            return
        key = self._prompt("Storage key (hex)", "")
        if key:
            result = self.substrate.rpc_request(
                "state_getStorage", [key])
            raw = result.get('result')
            self._val("Raw value", raw if raw else "(empty)")

    def _si_enum_keys(self):
        if not self._ensure():
            return
        prefix = self._prompt("Hex prefix or pallet name", "")
        if prefix and not prefix.startswith("0x"):
            try:
                import xxhash
                h = xxhash.xxh64(prefix.encode(), seed=0).hexdigest()
                h += xxhash.xxh64(prefix.encode(), seed=1).hexdigest()
                prefix = "0x" + h
                self._info(f"Prefix: {prefix}")
            except ImportError:
                self._err("xxhash not available for name conversion")
        count = self._prompt_int("Max keys", 20)
        result = self.substrate.rpc_request(
            "state_getKeysPaged", [prefix, count, prefix])
        keys = result.get('result', [])
        self._val("Keys found", len(keys))
        for i, k in enumerate(keys[:count]):
            print(f"    {C.DIM}[{i}]{C.R} {k}")

    def _si_storage_size(self):
        if not self._ensure():
            return
        key = self._prompt("Storage key (hex)", "")
        if key:
            result = self.substrate.rpc_request(
                "state_getStorageSize", [key])
            size = result.get('result')
            self._val("Size (bytes)",
                      size if size is not None else "key not found")

    def _si_diff(self):
        if not self._ensure():
            return
        key = self._prompt("Storage key (hex)", "")
        n1 = self._prompt_int("Block number A", 1)
        n2 = self._prompt_int("Block number B (0=latest)", 0)
        h1 = self.substrate.get_block_hash(n1)
        h2 = (self.substrate.get_block_hash(n2)
              if n2 > 0 else self.substrate.get_block_hash())
        r1 = self.substrate.rpc_request(
            "state_getStorage", [key, h1]).get('result')
        r2 = self.substrate.rpc_request(
            "state_getStorage", [key, h2]).get('result')
        self._val(f"Block {n1}", r1 if r1 else "(empty)")
        n2_label = n2 if n2 > 0 else "latest"
        self._val(f"Block {n2_label}", r2 if r2 else "(empty)")
        if r1 == r2:
            self._info("Values are identical")
        else:
            print(f"  {C.Y}Values differ{C.R}")

    def _si_proof(self):
        if not self._ensure():
            return
        key = self._prompt("Storage key (hex)", "")
        if key:
            bh = self.substrate.get_block_hash()
            result = self.substrate.rpc_request(
                "state_getReadProof", [[key], bh])
            proof = result.get('result', {})
            self._val("At block", proof.get('at', '?'))
            nodes = proof.get('proof', [])
            self._val("Proof nodes", len(nodes))
            for i, node in enumerate(nodes[:10]):
                print(f"    {C.DIM}[{i}] {node[:80]}...{C.R}")

    # ------------------------------------------------------------------
    # Custom handlers: Runtime Inspector
    # ------------------------------------------------------------------

    def _rt_list_pallets(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        print(f"\n  {C.W}{'Pallet':>20}  {'Calls':>6}  "
              f"{'Storage':>8}  {'Events':>7}  "
              f"{'Errors':>7}  {'Consts':>7}{C.R}")
        print(f"  {C.DIM}{'─'*62}{C.R}")
        for p in md.pallets:
            nc = len(p.calls) if p.calls else 0
            ns = len(p.storage) if p.storage else 0
            ne = len(p.events) if p.events else 0
            nerr = len(p.errors) if p.errors else 0
            nconst = len(p.constants) if p.constants else 0
            print(f"  {C.B}{p.name:>20}{C.R}  {nc:>6}  {ns:>8}  "
                  f"{ne:>7}  {nerr:>7}  {nconst:>7}")

    def _rt_pallet_detail(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        pallets = [p.name for p in md.pallets]
        for i, name in enumerate(pallets):
            print(f"    {C.Y}{i+1:>3}{C.R} {name}")
        idx = self._prompt_int("Pallet #", 1) - 1
        if 0 <= idx < len(pallets):
            p = md.pallets[idx]
            self._header(f"Pallet: {p.name}")
            if p.calls:
                print(f"  {C.W}Calls ({len(p.calls)}):{C.R}")
                for call in p.calls:
                    args = ""
                    if hasattr(call, 'args') and call.args:
                        args = ", ".join(
                            f"{a.name}: {a.type}" for a in call.args)
                    print(f"    {C.G}{call.name}{C.R}"
                          f"({C.DIM}{args}{C.R})")
            if p.storage:
                print(f"\n  {C.W}Storage ({len(p.storage)}):{C.R}")
                for s in p.storage:
                    stype = (str(s.type)
                             if hasattr(s, 'type') else '?')
                    print(f"    {C.CY}{s.name}{C.R} "
                          f"{C.DIM}{stype[:60]}{C.R}")
            if p.events:
                print(f"\n  {C.W}Events ({len(p.events)}):{C.R}")
                for ev in p.events:
                    print(f"    {C.Y}{ev.name}{C.R}")
            if p.errors:
                print(f"\n  {C.W}Errors ({len(p.errors)}):{C.R}")
                for err in p.errors:
                    doc = ""
                    if hasattr(err, 'docs') and err.docs:
                        doc = (f" {C.DIM}— "
                               f"{' '.join(err.docs)[:60]}{C.R}")
                    print(f"    {C.RED}{err.name}{C.R}{doc}")
            if p.constants:
                print(f"\n  {C.W}Constants ({len(p.constants)}):{C.R}")
                for const in p.constants:
                    val = (const.value
                           if hasattr(const, 'value') else '?')
                    print(f"    {C.CY}{const.name}{C.R} = "
                          f"{C.W}{val}{C.R}")

    def _rt_version(self):
        if not self._ensure():
            return
        rv = self.substrate.rpc_request(
            "state_getRuntimeVersion", [])['result']
        for k in ['specName', 'specVersion', 'implVersion',
                  'authoringVersion', 'transactionVersion',
                  'stateVersion']:
            self._val(k, rv.get(k, 'n/a'))

    def _rt_search_call(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        q = self._prompt("Call name (substring)", "").lower()
        if q:
            found = 0
            for p in md.pallets:
                if p.calls:
                    for call in p.calls:
                        if q in call.name.lower():
                            args = ""
                            if hasattr(call, 'args') and call.args:
                                args = ", ".join(
                                    f"{a.name}: {a.type}"
                                    for a in call.args)
                            print(f"    {C.B}{p.name}{C.R}."
                                  f"{C.G}{call.name}{C.R}"
                                  f"({C.DIM}{args}{C.R})")
                            found += 1
            self._info(f"{found} calls matched '{q}'")

    def _rt_search_storage(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        q = self._prompt("Storage name (substring)", "").lower()
        if q:
            found = 0
            for p in md.pallets:
                if p.storage:
                    for s in p.storage:
                        if q in s.name.lower():
                            stype = (str(s.type)
                                     if hasattr(s, 'type') else '?')
                            print(f"    {C.B}{p.name}{C.R}."
                                  f"{C.CY}{s.name}{C.R} "
                                  f"{C.DIM}{stype[:50]}{C.R}")
                            found += 1
            self._info(f"{found} storage items matched '{q}'")

    def _rt_search_error(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        q = self._prompt("Error name (substring)", "").lower()
        if q:
            found = 0
            for p in md.pallets:
                if p.errors:
                    for err in p.errors:
                        if q in err.name.lower():
                            doc = ""
                            if hasattr(err, 'docs') and err.docs:
                                doc = (" — "
                                       f"{' '.join(err.docs)[:60]}")
                            print(f"    {C.B}{p.name}{C.R}."
                                  f"{C.RED}{err.name}{C.R}"
                                  f"{C.DIM}{doc}{C.R}")
                            found += 1
            self._info(f"{found} errors matched '{q}'")

    # ------------------------------------------------------------------
    # Custom handlers: Network
    # ------------------------------------------------------------------

    def _net_peers(self):
        if not self._ensure():
            return
        peers = self.substrate.rpc_request(
            "system_peers", [])['result']
        if not peers:
            self._info("No connected peers (single-node devnet)")
        else:
            print(f"\n  {C.W}{'Peer ID':>20}  {'Best #':>8}  "
                  f"{'Roles':>10}{C.R}")
            print(f"  {C.DIM}{'─'*44}{C.R}")
            for p in peers:
                pid = p.get('peerId', '?')[:16]
                best = p.get('bestNumber', '?')
                roles = p.get('roles', '?')
                print(f"  {C.DIM}{pid}...{C.R}  {best:>8}  "
                      f"{roles:>10}")
            self._val("Total peers", len(peers))

    def _net_identity(self):
        if not self._ensure():
            return
        pid = self.substrate.rpc_request(
            "system_localPeerId", [])['result']
        addrs = self.substrate.rpc_request(
            "system_localListenAddresses", [])['result']
        self._val("Peer ID", pid)
        self._val("Listen Addresses", len(addrs))
        for addr in addrs:
            print(f"    {C.DIM}{addr}{C.R}")

    def _net_sync(self):
        if not self._ensure():
            return
        state = self.substrate.rpc_request(
            "system_syncState", [])['result']
        self._val("Starting Block", state.get('startingBlock', '?'))
        self._val("Current Block", state.get('currentBlock', '?'))
        self._val("Highest Block", state.get('highestBlock', '?'))

    def _net_health(self):
        if not self._ensure():
            return
        h = self.substrate.rpc_request("system_health", [])['result']
        self._val("Peers", h.get('peers', 0))
        self._val("Is Syncing", h.get('isSyncing', False))
        self._val("Should Have Peers",
                  h.get('shouldHavePeers', False))

    def _net_roles(self):
        if not self._ensure():
            return
        roles = self.substrate.rpc_request(
            "system_nodeRoles", [])['result']
        self._val("Node Roles", roles)

    def _net_chain_type(self):
        if not self._ensure():
            return
        chain_type = self.substrate.rpc_request(
            "system_chainType", [])['result']
        self._val("Chain Type", chain_type)

    def _net_pending(self):
        if not self._ensure():
            return
        pending = self.substrate.rpc_request(
            "author_pendingExtrinsics", [])['result']
        self._val("Pending Count", len(pending))
        for i, ext in enumerate(pending[:10]):
            print(f"    {C.DIM}[{i}] {str(ext)[:80]}{C.R}")

    def _net_reserved_peer(self):
        if not self._ensure():
            return
        action = self._prompt_enum(
            "Action:",
            ["Add reserved peer", "Remove reserved peer"])
        addr = self._prompt("Multiaddr", "")
        if addr:
            if "Add" in action:
                r = self.substrate.rpc_request(
                    "system_addReservedPeer", [addr])
            else:
                r = self.substrate.rpc_request(
                    "system_removeReservedPeer", [addr])
            self._ok(f"Result: {r.get('result', r)}")

    # ------------------------------------------------------------------
    # Custom handlers: Crypto Toolbox
    # ------------------------------------------------------------------

    def _crypto_generate(self):
        scheme = self._prompt_enum("Scheme:", ["sr25519", "ed25519"])
        mnemonic = Keypair.generate_mnemonic()
        crypto = 1 if scheme == "sr25519" else 2
        kp = Keypair.create_from_mnemonic(mnemonic, crypto_type=crypto)
        self._val("Mnemonic", mnemonic)
        self._val("Public Key", f"0x{kp.public_key.hex()}")
        self._val("SS58 Address", kp.ss58_address)

    def _crypto_derive(self):
        uri = self._prompt("URI (e.g. //Alice or mnemonic)", "//Alice")
        try:
            kp = Keypair.create_from_uri(uri)
        except (ValueError, TypeError) as e:
            if self._mode == 'dev':
                self._info(f"URI parse failed, trying mnemonic: {e}")
            kp = Keypair.create_from_mnemonic(uri)
        except Exception as e:
            if self._mode == 'dev':
                self._info(f"URI parse failed, trying mnemonic: {e}")
            kp = Keypair.create_from_mnemonic(uri)
        self._val("Public Key", f"0x{kp.public_key.hex()}")
        self._val("SS58 Address", kp.ss58_address)
        self._val("AccountId", f"0x{kp.public_key.hex()}")

    def _crypto_ss58(self):
        direction = self._prompt_enum(
            "Direction:", ["Hex to SS58", "SS58 to Hex"])
        if "SS58 to" in direction:
            ss58 = self._prompt("SS58 address", "")
            if ss58:
                kp = Keypair(ss58_address=ss58)
                self._val("Public Key (hex)",
                          f"0x{kp.public_key.hex()}")
        else:
            hex_key = self._prompt("Public key (hex)", "")
            prefix = self._prompt_int("SS58 prefix", 42)
            if hex_key:
                if hex_key.startswith("0x"):
                    hex_key = hex_key[2:]
                kp = Keypair(public_key=bytes.fromhex(hex_key),
                             ss58_format=prefix)
                self._val("SS58 Address", kp.ss58_address)

    def _crypto_blake2b(self):
        data = self._prompt("Input (hex or text)", "")
        if data:
            raw = (bytes.fromhex(data[2:])
                   if data.startswith("0x") else data.encode())
            digest = hashlib.blake2b(raw, digest_size=32).hexdigest()
            self._val("Blake2b-256", f"0x{digest}")

    def _crypto_keccak(self):
        data = self._prompt("Input (hex or text)", "")
        if data:
            raw = (bytes.fromhex(data[2:])
                   if data.startswith("0x") else data.encode())
            try:
                from Crypto.Hash import keccak as _keccak
                kh = _keccak.new(digest_bits=256, data=raw)
                self._val("Keccak-256", f"0x{kh.hexdigest()}")
            except ImportError:
                digest = hashlib.sha3_256(raw).hexdigest()
                self._val("SHA3-256", f"0x{digest}")
                self._info("Note: install pycryptodome for Keccak-256")

    def _crypto_twox128(self):
        data = self._prompt("Input string", "")
        if data:
            try:
                import xxhash
                h0 = xxhash.xxh64(data.encode(), seed=0).hexdigest()
                h1 = xxhash.xxh64(data.encode(), seed=1).hexdigest()
                self._val("TwoX128", f"0x{h0}{h1}")
            except ImportError:
                self._err("xxhash not installed (pip install xxhash)")

    def _crypto_storage_key(self):
        pallet = self._prompt("Pallet name", "System")
        storage = self._prompt("Storage name", "Number")
        try:
            import xxhash
            p0 = xxhash.xxh64(pallet.encode(), seed=0).hexdigest()
            p1 = xxhash.xxh64(pallet.encode(), seed=1).hexdigest()
            s0 = xxhash.xxh64(storage.encode(), seed=0).hexdigest()
            s1 = xxhash.xxh64(storage.encode(), seed=1).hexdigest()
            key = f"0x{p0}{p1}{s0}{s1}"
            self._val("Storage Key", key)
            self._info(f"TwoX128({pallet}) = {p0}{p1}")
            self._info(f"TwoX128({storage}) = {s0}{s1}")
        except ImportError:
            self._err("xxhash not installed (pip install xxhash)")

    def _crypto_scale_encode(self):
        if not self._ensure():
            return
        type_str = self._prompt(
            "SCALE type (e.g. u32, AccountId, Vec<u8>)", "u32")
        value = self._prompt("Value", "42")
        try:
            val = int(value) if value.isdigit() else value
        except (ValueError, TypeError):
            val = value
        try:
            obj = self.substrate.runtime_config.create_scale_object(
                type_str)
            obj.encode(val)
            self._val("Encoded", f"0x{obj.data.to_hex()}")
        except (ValueError, TypeError) as e:
            self._err(f"SCALE encode: {e}")
        except Exception as e:
            self._err(f"SCALE encode: {e}")

    def _crypto_scale_decode(self):
        if not self._ensure():
            return
        type_str = self._prompt(
            "SCALE type (e.g. u32, AccountId)", "u32")
        hex_data = self._prompt("Hex data", "0x2a000000")
        try:
            from scalecodec import ScaleBytes
            obj = self.substrate.runtime_config.create_scale_object(
                type_str)
            obj.decode(ScaleBytes(hex_data))
            self._val("Decoded", obj.value)
        except (ValueError, TypeError) as e:
            self._err(f"SCALE decode: {e}")
        except Exception as e:
            self._err(f"SCALE decode: {e}")

    def _crypto_sign(self):
        uri = self._prompt(
            "Keypair URI (e.g. //Alice)", "//Alice")
        message = self._prompt("Message", "hello")
        kp = Keypair.create_from_uri(uri)
        raw = (message.encode()
               if not message.startswith("0x")
               else bytes.fromhex(message[2:]))
        sig = kp.sign(raw)
        self._val("Signature", f"0x{sig.hex()}")
        self._val("Signer", kp.ss58_address)

    def _crypto_verify(self):
        pub = self._prompt("Public key (hex or SS58)", "")
        message = self._prompt("Message", "hello")
        sig_hex = self._prompt("Signature (hex)", "")
        try:
            if pub.startswith("0x"):
                kp = Keypair(public_key=bytes.fromhex(pub[2:]))
            else:
                kp = Keypair(ss58_address=pub)
            raw = (message.encode()
                   if not message.startswith("0x")
                   else bytes.fromhex(message[2:]))
            sig = (bytes.fromhex(sig_hex[2:])
                   if sig_hex.startswith("0x")
                   else bytes.fromhex(sig_hex))
            valid = kp.verify(raw, sig)
            if valid:
                self._ok("Signature is VALID")
            else:
                self._err("Signature is INVALID")
        except (ValueError, TypeError) as e:
            self._err(f"Verify failed: {e}")
        except Exception as e:
            self._err(f"Verify failed: {e}")

    def _crypto_random(self):
        h = "0x" + secrets.token_hex(32)
        self._val("Random 32-byte hex", h)

    # ------------------------------------------------------------------
    # Custom handlers: Account Inspector
    # ------------------------------------------------------------------

    def _acct_full_info(self):
        if not self._ensure():
            return
        name = self._prompt_account("Account")
        kp = self.keypairs[name]
        r = self.substrate.query(
            'System', 'Account', [kp.ss58_address])
        if r and r.value:
            self._val("Nonce", r.value.get('nonce', 0))
            data = r.value.get('data', {})
            self._val("Free",
                      f"{data.get('free', 0) / 1e12:.6f} UNIT")
            self._val("Reserved",
                      f"{data.get('reserved', 0) / 1e12:.6f} UNIT")
            self._val("Frozen",
                      f"{data.get('frozen', 0) / 1e12:.6f} UNIT")
            self._val("Flags", data.get('flags', 0))
        else:
            self._info("Account not found or empty")

    def _acct_nonce(self):
        if not self._ensure():
            return
        name = self._prompt_account("Account")
        kp = self.keypairs[name]
        r = self.substrate.rpc_request(
            "system_accountNextIndex", [kp.ss58_address])
        self._val("Next Nonce", r.get('result', '?'))

    def _acct_balances(self):
        if not self._ensure():
            return
        rows = []
        for name, kp in self.keypairs.items():
            r = self.substrate.query(
                'System', 'Account', [kp.ss58_address])
            if r and r.value:
                data = r.value.get('data', {})
                free = data.get('free', 0)
                reserved = data.get('reserved', 0)
                total = free + reserved
                rows.append([name, f"{free/1e12:.4f}",
                             f"{reserved/1e12:.4f}",
                             f"{total/1e12:.4f}"])
            else:
                rows.append([name, "—", "—", "—"])
        self._table(["Account", "Free", "Reserved", "Total"], rows)

    def _acct_fee(self):
        if not self._ensure():
            return
        mod = self._prompt("Call module", "Presence")
        fn = self._prompt("Call function", "declare_presence")
        params_str = self._prompt(
            "Params JSON (or empty for {})", "{}")
        try:
            params = json.loads(params_str) if params_str else {}
        except json.JSONDecodeError:
            params = {}
        name = self._prompt_account("Signer")
        kp = self.keypairs[name]
        call = self.substrate.compose_call(mod, fn, params)
        ext = self.substrate.create_signed_extrinsic(
            call=call, keypair=kp)
        info = self.substrate.rpc_request(
            "payment_queryInfo", [ext.value])
        result = info.get('result', {})
        self._val("Weight", result.get('weight', '?'))
        self._val("Partial Fee", result.get('partialFee', '?'))
        self._val("Class", result.get('class', '?'))

    def _acct_dry_run(self):
        if not self._ensure():
            return
        mod = self._prompt("Call module", "Presence")
        fn = self._prompt("Call function", "declare_presence")
        params_str = self._prompt(
            "Params JSON (or empty for {})", "{}")
        try:
            params = json.loads(params_str) if params_str else {}
        except json.JSONDecodeError:
            params = {}
        name = self._prompt_account("Signer")
        kp = self.keypairs[name]
        call = self.substrate.compose_call(mod, fn, params)
        ext = self.substrate.create_signed_extrinsic(
            call=call, keypair=kp)
        result = self.substrate.rpc_request(
            "system_dryRun", [ext.value])
        dry = result.get('result', '?')
        if isinstance(dry, str) and 'Ok' in dry:
            self._ok(f"Dry run: {dry}")
        else:
            self._err(f"Dry run: {dry}")

    # ------------------------------------------------------------------
    # Custom handlers: Event Decoder
    # ------------------------------------------------------------------

    def _ev_latest(self):
        if not self._ensure():
            return
        events = self.substrate.query("System", "Events")
        if events and events.value:
            for i, ev in enumerate(events.value):
                mid = ev.get('event', {}).get('module_id', '?')
                eid = ev.get('event', {}).get('event_id', '?')
                attrs = ev.get('event', {}).get('attributes', '')
                attr_str = (f" {C.DIM}{str(attrs)[:60]}{C.R}"
                            if attrs else "")
                print(f"    {C.DIM}[{i:>3}]{C.R} "
                      f"{C.W}{mid}.{eid}{C.R}{attr_str}")
            self._val("Total events", len(events.value))
        else:
            self._info("No events at latest block")

    def _ev_at_block(self):
        if not self._ensure():
            return
        num = self._prompt_int("Block number", 1)
        bh = self.substrate.get_block_hash(num)
        events = self.substrate.query(
            "System", "Events", block_hash=bh)
        if events and events.value:
            for i, ev in enumerate(events.value):
                mid = ev.get('event', {}).get('module_id', '?')
                eid = ev.get('event', {}).get('event_id', '?')
                attrs = ev.get('event', {}).get('attributes', '')
                attr_str = (f" {C.DIM}{str(attrs)[:60]}{C.R}"
                            if attrs else "")
                print(f"    {C.DIM}[{i:>3}]{C.R} "
                      f"{C.W}{mid}.{eid}{C.R}{attr_str}")
            self._val("Total events", len(events.value))
        else:
            self._info(f"No events at block {num}")

    def _ev_filter(self):
        if not self._ensure():
            return
        pallet = self._prompt("Pallet name", "Presence")
        events = self.substrate.query("System", "Events")
        if events and events.value:
            filtered = [
                ev for ev in events.value
                if ev.get('event', {}).get('module_id', '') == pallet]
            if filtered:
                for i, ev in enumerate(filtered):
                    eid = ev.get('event', {}).get('event_id', '?')
                    attrs = ev.get('event', {}).get('attributes', '')
                    attr_str = (f" {C.DIM}{str(attrs)[:60]}{C.R}"
                                if attrs else "")
                    print(f"    {C.DIM}[{i:>3}]{C.R} "
                          f"{C.W}{pallet}.{eid}{C.R}{attr_str}")
                self._val(f"{pallet} events", len(filtered))
            else:
                self._info(f"No {pallet} events at latest block")
        else:
            self._info("No events at latest block")

    def _ev_history(self):
        if not self._ensure():
            return
        pallet = self._prompt(
            "Pallet name (or empty for all)", "")
        n = self._prompt_int("Last N blocks", 5)
        header = self.substrate.get_block_header()['header']
        current = header['number']
        total = 0
        for blk in range(max(1, current - n + 1), current + 1):
            bh = self.substrate.get_block_hash(blk)
            events = self.substrate.query(
                "System", "Events", block_hash=bh)
            if events and events.value:
                evts = events.value
                if pallet:
                    evts = [
                        ev for ev in evts
                        if ev.get('event', {}).get(
                            'module_id', '') == pallet]
                if evts:
                    print(f"  {C.B}Block {blk}{C.R}")
                    for ev in evts:
                        mid = ev.get('event', {}).get(
                            'module_id', '?')
                        eid = ev.get('event', {}).get(
                            'event_id', '?')
                        print(f"    {C.DIM}{mid}.{eid}{C.R}")
                    total += len(evts)
        self._val("Total events found", total)

    def _ev_types(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        rows = []
        for p in md.pallets:
            if p.events:
                for ev in p.events:
                    fields = ""
                    if hasattr(ev, 'args') and ev.args:
                        fields = ", ".join(
                            str(a) for a in ev.args)
                    elif (hasattr(ev, 'value')
                          and isinstance(ev.value, dict)):
                        fields = ", ".join(
                            ev.value.get('args', []))
                    rows.append([p.name, ev.name, fields])
        self._table(["Pallet", "Event", "Fields"], rows)
        self._val("Total event types", len(rows))

    # ------------------------------------------------------------------
    # Custom handlers: Dashboard
    # ------------------------------------------------------------------

    def _dashboard_overview(self):
        if not self._ensure():
            return
        self._header("NETWORK OVERVIEW")
        h = self.substrate.rpc_request("system_health", [])['result']
        self._val("Chain",
                  self.substrate.rpc_request("system_chain", [])['result'])
        header = self.substrate.get_block_header()['header']
        self._val("Block", header['number'])
        self._val("Peers", h.get('peers', 0))
        self._val("Syncing", h.get('isSyncing', False))
        try:
            epoch = self.substrate.query("Epoch", "CurrentEpoch")
            self._val("Current Session", epoch)
            info = self.substrate.query(
                "Epoch", "EpochInfo",
                [epoch.value if epoch else 0])
            if info and info.value:
                state = info.value.get('state', 'Unknown')
                self._val("Session Status",
                          self._colorize_state(state))
            vc = self.substrate.query("Validator", "ValidatorCount")
            self._val("Validators", vc)
            ts = self.substrate.query("Validator", "TotalStake")
            self._val("Total Stake", ts)
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
        except Exception as e:
            self._info("Could not fetch session data")
            if self._mode == 'dev':
                self._info(f"Detail: {e}")

    def _dashboard_my_status(self):
        if not self._ensure():
            return
        name = self._ctx_account
        kp = self.keypairs[name]
        self._header(f"STATUS: {name}")
        r = self.substrate.query(
            'System', 'Account', [kp.ss58_address])
        if r and r.value:
            data = r.value.get('data', {})
            self._val("Balance",
                      f"{data.get('free', 0) / 1e12:.4f} UNIT")
        aid = self._actor_id(name)
        try:
            epoch_r = self.substrate.query("Epoch", "CurrentEpoch")
            epoch = epoch_r.value if epoch_r else 1
            pres = self.substrate.query(
                "Presence", "Presences", [epoch, aid])
            if pres and pres.value:
                self._val(f"Check-in (session {epoch})", pres)
            else:
                self._info(f"Not checked in for session {epoch}")
        except (ConnectionError, BrokenPipeError, OSError):
            self._err("Connection lost")
        except Exception:
            self._info("Could not fetch check-in status")
        try:
            vid = self._validator_id(name)
            val_info = self.substrate.query(
                "Validator", "Validators", [vid])
            if val_info and val_info.value:
                self._val("Validator", val_info)
            else:
                self._info("Not registered as validator")
        except Exception:
            pass

    def _dashboard_activity(self):
        if not self._ensure():
            return
        self._header("RECENT ACTIVITY")
        events = self.substrate.query("System", "Events")
        if events and events.value:
            interesting = [
                ev for ev in events.value
                if ev.get('event', {}).get('module_id', '')
                not in ('System', 'TransactionPayment', 'Balances', 0)]
            if interesting:
                for ev in interesting[-10:]:
                    mid = ev.get('event', {}).get('module_id', '?')
                    eid = ev.get('event', {}).get('event_id', '?')
                    print(f"    {C.W}{mid}.{eid}{C.R}")
                self._val("Shown", f"{min(10, len(interesting))} of "
                          f"{len(interesting)}")
            else:
                self._info("No notable events at latest block")
        else:
            self._info("No events at latest block")

    # ------------------------------------------------------------------
    # Custom handlers: Vault Secure Documents
    # ------------------------------------------------------------------

    def _vault_secure_file(self):
        """Encrypt a file with AES-256-GCM, split key via Shamir, register on-chain."""
        if not self._ensure():
            return
        vault_id = self._prompt_int("Vault ID", 0)
        vault_info = self._query("Vault", "Vaults", [vault_id])
        if not vault_info or not vault_info.value:
            self._err("Vault not found")
            return
        vi = vault_info.value
        status = vi.get('status')
        if status != 'Active':
            self._err(f"Vault is not active (status: {status})")
            return
        threshold = vi.get('threshold', 2)
        ring_size = vi.get('ring_size', 3)
        self._val("Vault", f"#{vault_id}  t={threshold} n={ring_size}")

        file_path = self._prompt("File path", "")
        path = pathlib.Path(file_path).expanduser().resolve()
        if not path.is_file():
            self._err(f"File not found: {path}")
            return
        size = path.stat().st_size
        if size > 100 * 1024 * 1024:
            self._err(f"File too large: {size:,} bytes (max 100 MB)")
            return
        privacy = self._prompt_bool(
            "Hide filename for privacy? "
            "(recommended for sensitive documents)",
            default=True)
        self._info(f"Securing {path.name} ({size:,} bytes)...")
        self._info(f"Protection: {threshold}-of-{ring_size} threshold")

        # Generate FEK and encrypt
        fek = bytearray(generate_fek())
        try:
            enc_hash, pt_hash, sz, dest = store_encrypted_file(
                vault_id, str(path), bytes(fek))
        except Exception as e:
            secure_zero(fek)
            self._err(f"Encryption failed: {e}")
            return

        # Compute key fingerprint
        fp = key_fingerprint(bytes(fek))
        fp_hex = fp.hex()

        # Split FEK via Shamir
        shares = ShamirScheme.split(bytes(fek), threshold, ring_size)
        if shares is None:
            secure_zero(fek)
            self._err("Shamir split failed")
            return

        # Store shares locally
        for idx, value in shares:
            store_share(vault_id, idx, value)

        self._val("Encrypted hash", f"0x{enc_hash[:32]}...")
        self._val("Plaintext hash", f"0x{pt_hash[:32]}...")
        self._val("Key fingerprint", f"0x{fp_hex[:32]}...")
        self._val("Shares created", f"{len(shares)}")
        self._val("Local .enc file", str(dest))

        # Register on-chain
        epoch = self._prompt_epoch()
        signer = self._prompt_account("Signer")

        self._info("Registering file on-chain (Vault.register_file)...")
        receipt = self._submit("Vault", "register_file", {
            "vault_id": vault_id,
            "enc_hash": f"0x{enc_hash}",
            "plaintext_hash": f"0x{pt_hash}",
            "key_fingerprint": f"0x{fp_hex}",
            "size_bytes": sz,
        }, signer)

        if not (receipt and receipt.is_success):
            secure_zero(fek)
            self._err("On-chain file registration failed")
            return

        self._info("Storing proof in Storage pallet...")
        self._submit("Storage", "store_data", {
            "epoch": epoch,
            "key": f"0x{enc_hash}",
            "data_hash": f"0x{enc_hash}",
            "data_type": "VaultFile",
            "size_bytes": sz,
            "retention": "Persistent",
        }, signer)

        # Commit only the signer's own share (one commit per member)
        self._info("Committing signer's share hash on-chain...")
        own_idx, own_value = shares[0]
        commitment = ShamirScheme.create_commitment(own_idx, own_value)
        self._submit("Vault", "commit_share", {
            "vault_id": vault_id,
            "commitment": f"0x{commitment.hex()}",
        }, signer)

        # Update local index
        add_to_index(
            vault_id=vault_id,
            enc_hash=enc_hash,
            plaintext_hash=pt_hash,
            original_name=path.name,
            size_bytes=sz,
            uploader=signer,
            epoch=epoch,
            key_fingerprint_hex=fp_hex,
            threshold=threshold,
            ring_size=ring_size,
            privacy_mode=privacy,
        )

        secure_zero(fek)
        self._ok(f"Document secured: {path.name}")
        if privacy:
            self._info(
                "Filename is hidden. "
                "Only the document hash is stored.")
        self._info(f"Requires {threshold} of {ring_size} shares to unlock")

    def _vault_unlock_file(self):
        """Collect shares, reconstruct FEK, decrypt and export a vault file."""
        if not self._ensure():
            return
        vault_id = self._prompt_int("Vault ID", 0)
        files = get_vault_files(vault_id)
        if not files:
            self._info("No files in this vault")
            return

        for i, f in enumerate(files):
            if f.get('name_redacted', False):
                display = f"Document #{i+1} (private)"
            else:
                display = f['original_name']
            print(f"    {C.Y}{i+1}{C.R} {display} "
                  f"{C.DIM}({f.get('enc_hash', '?')[:16]}...){C.R}")
        idx = self._prompt_int("File #", 1) - 1
        if not (0 <= idx < len(files)):
            return
        f = files[idx]
        enc_hash = f.get('enc_hash', '')
        threshold = f.get('threshold', 2)

        if f.get('name_redacted', False):
            self._val("File", f"Document #{idx+1} (private)")
        else:
            self._val("File", f['original_name'])
        self._val("Threshold", f"{threshold} shares needed")

        # Request unlock on-chain
        signer = self._prompt_account("Signer")
        self._info("Requesting unlock on-chain (Vault.request_unlock)...")
        receipt = self._submit("Vault", "request_unlock", {
            "vault_id": vault_id,
            "file_enc_hash": f"0x{enc_hash}",
        }, signer)

        if not (receipt and receipt.is_success):
            self._err("Unlock request failed (may need approvals first)")

        # Collect shares
        self._info("Loading local shares...")
        local_shares = load_all_shares(vault_id)
        self._val("Local shares found", str(len(local_shares)))

        all_shares = list(local_shares)

        while len(all_shares) < threshold:
            self._info(f"Need {threshold - len(all_shares)} more share(s)")
            hex_input = self._prompt("Paste share hex (or 'done')", "done")
            if hex_input.lower() == 'done':
                break
            parsed = import_share_hex(hex_input)
            if parsed is None:
                self._err("Invalid share format (expected: XX:hex...)")
                continue
            # Avoid duplicate indices
            existing_indices = {s[0] for s in all_shares}
            if parsed[0] in existing_indices:
                self._err(f"Share index {parsed[0]} already loaded")
                continue
            all_shares.append(parsed)
            self._ok(f"Share #{parsed[0]} imported")

        if len(all_shares) < threshold:
            self._err(f"Not enough shares ({len(all_shares)}/{threshold})")
            return

        # Reconstruct FEK
        self._info("Reconstructing encryption key...")
        fek = bytearray(ShamirScheme.reconstruct(all_shares, threshold))
        if fek is None:
            self._err("Reconstruction failed")
            return

        # Verify fingerprint matches
        expected_fp = f.get('key_fingerprint', '')
        computed_fp = key_fingerprint(bytes(fek)).hex()
        if expected_fp and computed_fp != expected_fp:
            secure_zero(fek)
            self._err("Key fingerprint mismatch - wrong shares?")
            return
        self._ok("Key fingerprint verified")

        # Decrypt
        if f.get('name_redacted', False):
            orig = f.get('original_name', '')
            ext = pathlib.Path(orig).suffix if orig else ''
            default_dest = f"./{enc_hash[:16]}{ext}"
        else:
            default_dest = f"./{f['original_name']}"
        dest = self._prompt("Export to", default_dest)
        try:
            resolved = retrieve_and_decrypt(
                vault_id, enc_hash, bytes(fek), dest)
            self._ok(f"Document decrypted to {resolved}")
        except Exception as e:
            self._err(f"Decryption failed: {e}")
        finally:
            secure_zero(fek)

    def _vault_list_files(self):
        vault_id = self._prompt_int("Vault ID", 0)
        files = get_vault_files(vault_id)
        if not files:
            self._info("No files in this vault")
            return
        rows = []
        for i, f in enumerate(files):
            h = f.get('enc_hash', f.get('file_hash', '?'))
            if f.get('name_redacted', False):
                display = f"Document #{i+1} (private)"
            else:
                display = f['original_name']
            rows.append([
                display[:24],
                f"{f['size_bytes']:,}",
                h[:16] + '...',
                f['uploaded_at'][:10],
                f"{f.get('threshold', '?')}/{f.get('ring_size', '?')}",
            ])
        self._table(
            ["Name", "Size", "Hash", "Date", "t/n"], rows)

    def _vault_verify_file(self):
        vault_id = self._prompt_int("Vault ID", 0)
        files = get_vault_files(vault_id)
        if not files:
            self._info("No files in this vault")
            return
        for i, f in enumerate(files):
            h = f.get('enc_hash', f.get('file_hash', '?'))
            if f.get('name_redacted', False):
                display = f"Document #{i+1} (private)"
            else:
                display = f['original_name']
            print(f"    {C.Y}{i+1}{C.R} {display} "
                  f"{C.DIM}({h[:16]}...){C.R}")
        idx = self._prompt_int("File #", 1) - 1
        if not (0 <= idx < len(files)):
            return
        f = files[idx]
        enc_hash = f.get('enc_hash', f.get('file_hash', ''))
        ok, current_hash = verify_vault_file(vault_id, enc_hash)
        if ok is None:
            self._err("Local file not found (may have been deleted)")
        elif ok:
            self._ok("File integrity verified (hash matches)")
        else:
            self._err("HASH MISMATCH -- file has been modified!")
            self._val("Expected", enc_hash[:32] + '...')
            self._val("Got", current_hash[:32] + '...')

    def _vault_export_share(self):
        vault_id = self._prompt_int("Vault ID", 0)
        share_idx = self._prompt_int("Share index", 1)
        hex_str = export_share_hex(vault_id, share_idx)
        if hex_str is None:
            self._err(f"Share #{share_idx} not found for vault {vault_id}")
            return
        self._ok("Share exported (copy this hex string):")
        print(f"    {C.CY}{hex_str}{C.R}")

    def _vault_import_share(self):
        vault_id = self._prompt_int("Vault ID", 0)
        hex_input = self._prompt("Share hex (XX:value...)", "")
        parsed = import_share_hex(hex_input)
        if parsed is None:
            self._err("Invalid share format (expected: XX:hex...)")
            return
        idx, value = parsed
        path = store_share(vault_id, idx, value)
        self._ok(f"Share #{idx} imported and saved to {path}")

    # -- Dev mode vault tools --

    def _vault_dev_split(self):
        secret_hex = self._prompt("Secret (64-char hex)", "")
        try:
            secret = bytes.fromhex(secret_hex)
        except ValueError:
            self._err("Invalid hex")
            return
        if len(secret) != 32:
            self._err("Secret must be exactly 32 bytes (64 hex chars)")
            return
        threshold = self._prompt_int("Threshold (t)", 2)
        total = self._prompt_int("Total shares (n)", 3)
        shares = ShamirScheme.split(secret, threshold, total)
        if shares is None:
            self._err("Invalid parameters (need 2 <= t <= n)")
            return
        self._ok(f"Split into {len(shares)} shares (t={threshold})")
        for idx, value in shares:
            print(f"    Share {C.Y}#{idx}{C.R}: {value.hex()}")

    def _vault_dev_reconstruct(self):
        threshold = self._prompt_int("Threshold (t)", 2)
        shares = []
        for i in range(threshold):
            hex_input = self._prompt(f"Share {i+1} (XX:value...)", "")
            parsed = import_share_hex(hex_input)
            if parsed is None:
                self._err(f"Invalid share format for share {i+1}")
                return
            shares.append(parsed)
        result = ShamirScheme.reconstruct(shares, threshold)
        if result is None:
            self._err("Reconstruction failed")
            return
        self._ok("Reconstructed secret:")
        print(f"    {C.CY}{result.hex()}{C.R}")

    def _vault_dev_encrypt(self):
        file_path = self._prompt("File path", "")
        fek_hex = self._prompt("FEK (64-char hex)", "")
        try:
            fek = bytes.fromhex(fek_hex)
        except ValueError:
            self._err("Invalid hex")
            return
        if len(fek) != 32:
            self._err("FEK must be 32 bytes")
            return
        vault_id = self._prompt_int("Vault ID (for storage)", 0)
        try:
            enc_hash, pt_hash, sz, dest = store_encrypted_file(
                vault_id, file_path, fek)
            self._ok("Encrypted successfully")
            self._val("Enc hash", enc_hash)
            self._val("PT hash", pt_hash)
            self._val("Size", f"{sz:,}")
            self._val("Path", str(dest))
        except Exception as e:
            self._err(str(e))

    def _vault_dev_decrypt(self):
        vault_id = self._prompt_int("Vault ID", 0)
        enc_hash = self._prompt("Enc hash (hex, no 0x)", "")
        fek_hex = self._prompt("FEK (64-char hex)", "")
        try:
            fek = bytes.fromhex(fek_hex)
        except ValueError:
            self._err("Invalid hex")
            return
        if len(fek) != 32:
            self._err("FEK must be 32 bytes")
            return
        dest = self._prompt("Export to", "./decrypted_output")
        try:
            resolved = retrieve_and_decrypt(vault_id, enc_hash, fek, dest)
            self._ok(f"Decrypted to {resolved}")
        except Exception as e:
            self._err(str(e))

    def _vault_anonymize_index(self):
        """Hash all plaintext filenames in the vault index."""
        count = anonymize_existing_index()
        if count == 0:
            self._info("No entries to anonymize")
        else:
            self._ok(f"Anonymized {count} file entries in vault index")

    # ------------------------------------------------------------------
    # Custom handlers: Dev Extensions
    # ------------------------------------------------------------------

    def _devx_raw_extrinsic(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        pallets = [p for p in md.pallets if p.calls]
        for i, p in enumerate(pallets):
            print(f"    {C.Y}{i+1:>3}{C.R} {p.name} "
                  f"{C.DIM}({len(p.calls)} calls){C.R}")
        idx = self._prompt_int("Pallet #", 1) - 1
        if not (0 <= idx < len(pallets)):
            return
        pallet = pallets[idx]
        for i, call in enumerate(pallet.calls):
            args = ""
            if hasattr(call, 'args') and call.args:
                args = ", ".join(
                    f"{a.name}: {a.type}" for a in call.args)
            print(f"    {C.Y}{i+1:>3}{C.R} {call.name}"
                  f"({C.DIM}{args}{C.R})")
        cidx = self._prompt_int("Call #", 1) - 1
        if not (0 <= cidx < len(pallet.calls)):
            return
        call = pallet.calls[cidx]
        params_str = self._prompt("Params JSON", "{}")
        try:
            params = json.loads(params_str)
        except json.JSONDecodeError as e:
            self._err(f"Invalid JSON: {e}")
            return
        sudo = self._prompt_bool("Sudo?", False)
        signer = self._prompt_account("Signer")
        self._submit(pallet.name, call.name, params, signer, sudo=sudo)

    def _devx_batch(self):
        if not self._ensure():
            return
        calls = []
        self._info("Enter calls. Type 'done' when finished.")
        while True:
            mod = self._prompt("Module (or 'done')", "done")
            if mod.lower() == 'done':
                break
            fn = self._prompt("Function", "")
            params_str = self._prompt("Params JSON", "{}")
            try:
                params = json.loads(params_str)
            except json.JSONDecodeError:
                params = {}
            try:
                call = self.substrate.compose_call(mod, fn, params)
                calls.append(call)
                self._ok(f"Added {mod}.{fn} (#{len(calls)})")
            except (ConnectionError, BrokenPipeError, OSError) as e:
                self._err(f"Connection lost: {e}")
            except Exception as e:
                self._err(f"Failed to compose: {e}")
        if not calls:
            self._info("No calls to batch")
            return
        self._info(f"Batching {len(calls)} calls...")
        signer = self._prompt_account("Signer")
        batch_call = self.substrate.compose_call(
            'Utility', 'batch', {'calls': calls})
        kp = self.keypairs[signer]
        ext = self.substrate.create_signed_extrinsic(
            call=batch_call, keypair=kp)
        receipt = self.substrate.submit_extrinsic(
            ext, wait_for_inclusion=True)
        if receipt.is_success:
            self._ok(f"Batch of {len(calls)} calls succeeded")
        else:
            self._err(f"Batch failed: {receipt.error_message}")
        self._slot_wait()

    def _devx_storage_key_calc(self):
        if not self._ensure():
            return
        pallet = self._prompt("Pallet name", "Presence")
        item = self._prompt("Storage item", "Presences")
        try:
            import xxhash
            p0 = xxhash.xxh64(pallet.encode(), seed=0).hexdigest()
            p1 = xxhash.xxh64(pallet.encode(), seed=1).hexdigest()
            s0 = xxhash.xxh64(item.encode(), seed=0).hexdigest()
            s1 = xxhash.xxh64(item.encode(), seed=1).hexdigest()
            prefix = f"0x{p0}{p1}{s0}{s1}"
            self._val("Pallet hash", f"0x{p0}{p1}")
            self._val("Item hash", f"0x{s0}{s1}")
            self._val("Storage prefix", prefix)
            keys = self.substrate.rpc_request(
                "state_getKeysPaged", [prefix, 5, prefix])
            found = keys.get('result', [])
            self._val("Entries found", len(found))
            for k in found:
                suffix = k[len(prefix):]
                val = self.substrate.rpc_request(
                    "state_getStorage", [k]).get('result')
                print(f"    {C.DIM}key: {suffix[:40]}...{C.R}")
                print(f"    {C.DIM}val: {str(val)[:60]}{C.R}")
        except ImportError:
            self._err("xxhash required: pip install xxhash")

    def _devx_weight_estimate(self):
        if not self._ensure():
            return
        mod = self._prompt("Module", "Presence")
        fn = self._prompt("Function", "declare_presence")
        params_str = self._prompt("Params JSON", "{}")
        try:
            params = json.loads(params_str) if params_str else {}
        except json.JSONDecodeError:
            params = {}
        signer = self._prompt_account("Signer")
        kp = self.keypairs[signer]
        try:
            call = self.substrate.compose_call(mod, fn, params)
            ext = self.substrate.create_signed_extrinsic(
                call=call, keypair=kp)
            info = self.substrate.rpc_request(
                "payment_queryInfo", [ext.value])
            result = info.get('result', {})
            weight = result.get('weight', {})
            if isinstance(weight, dict):
                self._val("Ref Time",
                          weight.get('ref_time', '?'))
                self._val("Proof Size",
                          weight.get('proof_size', '?'))
            else:
                self._val("Weight", weight)
            self._val("Partial Fee",
                      result.get('partialFee', '?'))
            self._val("Class", result.get('class', '?'))
        except Exception as e:
            self._err(f"Weight estimation failed: {e}")

    def _devx_metadata_explorer(self):
        if not self._ensure():
            return
        md = self.substrate.get_metadata()
        action = self._prompt_enum("Explore:", [
            "Type registry summary",
            "Pallet constants with values",
            "Extrinsic version info",
        ])
        if "Type registry" in action:
            self._val("Pallets", len(md.pallets))
            total_calls = sum(
                len(p.calls) for p in md.pallets if p.calls)
            total_storage = sum(
                len(p.storage) for p in md.pallets if p.storage)
            total_events = sum(
                len(p.events) for p in md.pallets if p.events)
            total_errors = sum(
                len(p.errors) for p in md.pallets if p.errors)
            self._val("Total calls", total_calls)
            self._val("Total storage items", total_storage)
            self._val("Total event types", total_events)
            self._val("Total error types", total_errors)
        elif "constants" in action:
            for p in md.pallets:
                if p.constants:
                    print(f"\n  {C.B}{p.name}{C.R}")
                    for c in p.constants:
                        val = c.value if hasattr(c, 'value') else '?'
                        print(f"    {C.CY}{c.name}{C.R} = "
                              f"{C.W}{val}{C.R}")
        elif "Extrinsic" in action:
            rv = self.substrate.rpc_request(
                "state_getRuntimeVersion", [])['result']
            self._val("Spec Name", rv.get('specName'))
            self._val("Spec Version", rv.get('specVersion'))
            self._val("Tx Version",
                      rv.get('transactionVersion'))

    def _devx_benchmark(self):
        if not self._ensure():
            return
        n = self._prompt_int("Number of transactions", 10)
        self._info(f"Sending {n} balance transfers...")
        kp_alice = self.keypairs['alice']
        kp_bob = self.keypairs['bob']
        start = time.time()
        success = 0
        for i in range(n):
            try:
                call = self.substrate.compose_call(
                    'Balances', 'transfer_allow_death',
                    {'dest': kp_bob.ss58_address, 'value': 1})
                ext = self.substrate.create_signed_extrinsic(
                    call=call, keypair=kp_alice)
                receipt = self.substrate.submit_extrinsic(
                    ext, wait_for_inclusion=True)
                if receipt.is_success:
                    success += 1
                self._slot_wait()
            except Exception as e:
                self._err(f"Tx {i+1} failed: {e}")
        elapsed = time.time() - start
        tps = success / elapsed if elapsed > 0 else 0
        self._val("Sent", n)
        self._val("Succeeded", success)
        self._val("Elapsed", f"{elapsed:.1f}s")
        self._val("Throughput", f"{tps:.2f} tx/s")

    def _devx_snapshot(self):
        if not self._ensure():
            return
        action = self._prompt_enum(
            "Action:", ["Save snapshot", "Load snapshot"])
        snap_dir = os.path.expanduser('~/.laud/snapshots/')
        os.makedirs(snap_dir, exist_ok=True)
        if "Save" in action:
            name = self._prompt("Snapshot name", "snap1")
            prefix = self._prompt(
                "Storage prefix (hex)", "")
            count = self._prompt_int("Max keys", 100)
            keys_result = self.substrate.rpc_request(
                "state_getKeysPaged",
                [prefix, count, prefix])
            keys = keys_result.get('result', [])
            snapshot = {}
            for k in keys:
                val = self.substrate.rpc_request(
                    "state_getStorage", [k]).get('result')
                snapshot[k] = val
            path = os.path.join(snap_dir, f"{name}.json")
            with open(path, 'w') as f:
                json.dump(snapshot, f, indent=2)
            self._ok(f"Saved {len(snapshot)} keys to {path}")
        else:
            snaps = [f for f in os.listdir(snap_dir)
                     if f.endswith('.json')]
            if not snaps:
                self._info("No snapshots found")
                return
            for i, s in enumerate(snaps):
                print(f"    {C.Y}{i+1}{C.R} {s}")
            idx = self._prompt_int("Snapshot #", 1) - 1
            if 0 <= idx < len(snaps):
                path = os.path.join(snap_dir, snaps[idx])
                with open(path, 'r') as f:
                    snapshot = json.load(f)
                self._info(
                    f"Loaded {len(snapshot)} keys from {path}")
                self._info("Comparing with current state:")
                diffs = 0
                for k, v in snapshot.items():
                    current = self.substrate.rpc_request(
                        "state_getStorage",
                        [k]).get('result')
                    if current != v:
                        diffs += 1
                        print(f"  {C.Y}DIFF{C.R} "
                              f"{k[:40]}...")
                self._val("Total keys", len(snapshot))
                self._val("Changed", diffs)

    def _devx_event_stream(self):
        if not self._ensure():
            return
        pallet = self._prompt(
            "Pallet filter (empty for all)", "")
        interval = self._prompt_int(
            "Poll interval (seconds)", 7)
        self._info(
            f"Streaming events every {interval}s. "
            "Ctrl+C to stop.")
        last_block = 0
        try:
            while True:
                header = self.substrate.get_block_header(
                )['header']
                current = header['number']
                if current > last_block:
                    for blk in range(
                            max(last_block + 1, 1),
                            current + 1):
                        bh = self.substrate.get_block_hash(blk)
                        events = self.substrate.query(
                            "System", "Events",
                            block_hash=bh)
                        if events and events.value:
                            evts = events.value
                            if pallet:
                                evts = [
                                    ev for ev in evts
                                    if ev.get('event', {}).get(
                                        'module_id',
                                        '') == pallet]
                            for ev in evts:
                                mid = ev.get(
                                    'event', {}).get(
                                    'module_id', '?')
                                eid = ev.get(
                                    'event', {}).get(
                                    'event_id', '?')
                                ts = datetime.now().strftime(
                                    '%H:%M:%S')
                                print(
                                    f"  {C.DIM}{ts}{C.R} "
                                    f"block {blk}: "
                                    f"{C.W}{mid}.{eid}{C.R}")
                    last_block = current
                time.sleep(interval)
        except KeyboardInterrupt:
            self._info("Stream stopped.")

    # ------------------------------------------------------------------
    # Test flows
    # ------------------------------------------------------------------

    def test_full_lifecycle(self):
        if not self._ensure():
            return
        self._check_epoch()
        self._header("FULL PoP LIFECYCLE TEST")
        epoch = self._next_test_epoch()

        self._info(
            f"Step 1: Using epoch {epoch} "
            "(validators active from bootstrap)")

        self._info("Step 2: Eve declares presence")
        self._submit("Presence", "declare_presence",
                     {"epoch": epoch}, "eve")

        eve_id = self._actor_id('eve')
        self._info(
            "Step 3: Validators vote on Eve (3 of 6 = quorum)")
        for voter in ['alice', 'bob', 'charlie']:
            self._submit("Presence", "vote_presence",
                         {"actor": eve_id, "epoch": epoch,
                          "approve": True}, voter)

        vc = self._query("Presence", "VoteCount",
                         [epoch, eve_id])
        self._val("Eve votes", vc)

        self._info("Step 4: Finalize Eve's presence")
        self._submit("Presence", "finalize_presence",
                     {"actor": eve_id, "epoch": epoch}, "alice")

        r = self._query("Presence", "Presences",
                        [epoch, eve_id])
        self._val("Final state", r)
        self._ok("Full lifecycle test complete!")
        self._pause()

    def test_commit_reveal(self):
        if not self._ensure():
            return
        self._check_epoch()
        self._header("COMMIT-REVEAL TEST")
        epoch = self._next_test_epoch()

        sec = secrets.token_hex(32)
        rnd = secrets.token_hex(32)
        h = hashlib.blake2b(
            bytes.fromhex(sec + rnd), digest_size=32).hexdigest()

        self._info(f"Committing (hash: 0x{h[:16]}...)")
        self._submit("Presence", "declare_presence_with_commitment",
                     {"epoch": epoch, "commitment": "0x" + h}, "ferdie")

        self._val("Commitments",
                  self._query("Presence", "CommitmentCount", [epoch]))

        self._info("Revealing...")
        self._submit("Presence", "reveal_commitment",
                     {"epoch": epoch, "secret": "0x" + sec,
                      "randomness": "0x" + rnd}, "ferdie")

        self._val("Reveals",
                  self._query("Presence", "RevealCount", [epoch]))
        self._ok("Commit-reveal test complete!")
        self._pause()

    # ------------------------------------------------------------------
    # Compact menu (auto-generated from registry)
    # ------------------------------------------------------------------

    def _show_compact_menu(self):
        visible = get_domains_for_mode(self._mode)
        group_order = get_group_display_order(self._mode)

        groups = {}
        for d in visible:
            g = (d.normal_group
                 if self._mode == 'normal' and d.normal_group
                 else d.group)
            groups.setdefault(g, []).append(d)

        print()
        if self._mode == 'dev':
            print(f"  {C.Y}[DEV]{C.R}  "
                  f"{C.DIM}mode normal to simplify{C.R}")
        else:
            print(f"  {C.G}[NORMAL]{C.R}  "
                  f"{C.DIM}mode dev for full access{C.R}")

        for gkey, gtitle in group_order:
            domains = groups.get(gkey, [])
            if not domains:
                continue
            print()
            self._separator_line(gtitle)
            for d in domains:
                title = (d.normal_title
                         if self._mode == 'normal'
                         and d.normal_title
                         else d.name.capitalize())
                desc = (d.help_summary[:32]
                        if d.help_summary else "")
                tname = title[:16]
                dot_n = max(2, 34 - len(tname))
                dots = '\u00b7' * dot_n
                print(f"   {C.Y}{d.number:>2}{C.R}  "
                      f"{tname:<16}"
                      f"{C.DIM}{dots}{C.R} "
                      f"{desc}")

        print()
        self._separator_line("TESTS")
        for key, name, desc in [
            ("t1", "test pop", "Full PoP lifecycle"),
            ("t2", "test pbt", "PBT triangulation"),
            ("t3", "test commit", "Commit-reveal"),
        ]:
            dot_n = max(2, 24 - len(name))
            dots = '\u00b7' * dot_n
            print(f"   {C.Y}{key:>2}{C.R}  "
                  f"{name:<16}"
                  f"{C.DIM}{dots}{C.R} "
                  f"{desc}")
        print()
        self._separator_line()
        print(f"  {C.DIM}status  bootstrap (b)  "
              f"flow (f)  connect (1)  help  ?  exit{C.R}")
        print()

    # ------------------------------------------------------------------
    # Help
    # ------------------------------------------------------------------

    def _cmd_help(self, args=None):
        if not args:
            print(f"""
  {C.BB}LAUD NETWORKS{C.R}  {C.DIM}7ayLabs{C.R}

  {C.W}Navigation{C.R}
    menu              Show all commands with numbers
    <command>         Enter submenu (e.g. 'presence' or '2')
    <cmd> <action>    Direct action (e.g. 'presence declare')
    back              Return to parent menu
    0                 Back / exit current submenu

  {C.W}Context{C.R}
    use epoch <N>     Set default epoch for all commands
    use <name>        Set default account (alice, bob, ...)
    use clear         Reset to defaults
    status            Show chain / epoch / account status

  {C.W}Quick Actions{C.R}
    b / bootstrap     Bootstrap devnet (epoch + validators)
    f / flow          Show what to do next (epoch-aware)
    t1 / test pop     Full PoP lifecycle test
    t2 / test pbt     PBT triangulation test
    t3 / test commit  Commit-reveal test
    1 / connect       Connect to node

  {C.W}Tips{C.R}
    Tab               Autocomplete commands
    Up/Down           Command history
    Ctrl+C            Cancel / back to root
    i                 Instructions (inside any submenu)
    ?                 Quick start guide

  {C.DIM}Type 'help <topic>' for details (e.g. 'help presence'){C.R}
""")
            return
        topic = args[0].lower()
        domain = find_domain(topic)
        if domain:
            self._show_domain_instructions(domain)
        else:
            self._err(
                f"No help for '{topic}'. Type 'help' for general help.")

    def show_guide(self):
        self._header("QUICK START GUIDE")
        print(f"""  {C.W}KEY CONCEPTS{C.R}
  {C.DIM}  Time Period = a window during which presence proofs happen
    Validator   = a node that votes on presence claims
    Identity    = a participant identified by their public key
    PBT         = position-based triangulation (location proofs){C.R}

  {C.W}1. Start the devnet{C.R}
     {C.Y}cd devnet && ./scripts/dev.sh{C.R}
     {C.DIM}Or multi-node:  docker compose up -d --build{C.R}

  {C.W}2. Connect + bootstrap{C.R}
     {C.DIM}CLI auto-connects on start. Type {C.Y}bootstrap{C.DIM} or {C.Y}b{C.DIM}:
     activates time period 1, registers 6 validators, sets positions.{C.R}

  {C.W}3. Run automated tests{C.R}
     {C.Y}t1{C.R}  {C.DIM}Full PoP lifecycle    {C.Y}test pop{C.R}
     {C.Y}t2{C.R}  {C.DIM}PBT flow             {C.Y}test pbt{C.R}
     {C.Y}t3{C.R}  {C.DIM}Commit-reveal        {C.Y}test commit{C.R}

  {C.W}4. Set context{C.R}
     {C.Y}use epoch 5{C.R}   {C.DIM}all commands use time period 5{C.R}
     {C.Y}use bob{C.R}       {C.DIM}all commands sign as bob{C.R}
     {C.Y}use clear{C.R}     {C.DIM}reset to defaults{C.R}

  {C.W}5. Direct commands{C.R}
     {C.Y}presence declare{C.R}   {C.DIM}or{C.R}  {C.Y}p d{C.R}
     {C.Y}presence vote{C.R}      {C.DIM}or{C.R}  {C.Y}p v{C.R}
     {C.Y}pbt test{C.R}           {C.DIM}full PBT test flow{C.R}

  {C.W}6. Instructions{C.R}
     {C.DIM}Type {C.Y}i{C.DIM} inside any submenu to learn how it works.
     Type {C.Y}i 1{C.DIM} to see details about a specific command.{C.R}

  {C.W}7. Accounts{C.R}
     {C.DIM}alice {C.Y}(admin){C.DIM}, bob, charlie, dave, eve, ferdie
     All pre-funded with 10M UNIT on devnet{C.R}
""")

    # ------------------------------------------------------------------
    # Main dispatch
    # ------------------------------------------------------------------

    def _cmd_connect(self):
        url = self._prompt("URL", self.url)
        self.connect(url)

    def _build_prompt(self):
        mode_tag = (f"{C.Y}dev{C.R}:" if self._mode == 'dev'
                    else "")
        path = "/".join(["laud"] + self._nav_stack)
        extras = []
        if self._ctx_account != 'alice':
            extras.append(f"{C.Y}{self._ctx_account}{C.R}")
        if self._ctx_epoch is not None:
            extras.append(f"{C.DIM}epoch:{self._ctx_epoch}{C.R}")
        epoch_tag = self._epoch_status_tag()
        extra = " " + " ".join(extras) if extras else ""
        return f"  {mode_tag}{C.B}{path}{C.R}{extra}{epoch_tag} > "

    def _dispatch(self, line):
        parts = line.strip().split()
        if not parts:
            return
        cmd = parts[0].lower()

        if cmd in ('exit', 'quit', '0'):
            raise SystemExit
        if cmd in ('help', 'h'):
            self._cmd_help(parts[1:] if len(parts) > 1 else None)
            return
        if cmd == 'use':
            self._cmd_use(parts[1:])
            return
        if cmd == 'status':
            self._show_status()
            return
        if cmd in ('menu', 'm'):
            self._show_compact_menu()
            return
        if cmd in ('flow', 'f'):
            self._show_epoch_flow()
            return
        if cmd == 'back':
            if self._nav_stack:
                self._nav_stack.pop()
            return

        # Mode switching
        if cmd == 'mode':
            if len(parts) > 1:
                new_mode = parts[1].lower()
                if new_mode in ('dev', 'developer'):
                    self._mode = 'dev'
                    self._save_mode()
                    self._rebuild_indexes()
                    self._ok("Switched to DEVELOPER mode")
                    self._show_compact_menu()
                elif new_mode in ('normal', 'user', 'business'):
                    self._mode = 'normal'
                    self._save_mode()
                    self._rebuild_indexes()
                    self._ok("Switched to NORMAL mode")
                    self._show_compact_menu()
                else:
                    self._err(f"Unknown mode: '{new_mode}'. "
                              "Use 'mode dev' or 'mode normal'.")
            else:
                label = "DEVELOPER" if self._mode == 'dev' else "NORMAL"
                self._info(f"Current mode: {C.W}{label}{C.R}")
                self._info("Switch with: mode dev | mode normal")
            return

        # Test shortcuts
        if cmd == 'test' and len(parts) > 1:
            sub = parts[1].lower()
            test_map = {
                'pop': 'test_full_lifecycle',
                '1': 'test_full_lifecycle',
                'pbt': '_auto_pbt_test',
                '2': '_auto_pbt_test',
                'commit': 'test_commit_reveal',
                '3': 'test_commit_reveal',
            }
            handler_name = test_map.get(sub)
            if handler_name:
                getattr(self, handler_name)()
                return

        # Direct test shortcuts
        if cmd == 't1':
            self.test_full_lifecycle()
            return
        if cmd == 't2':
            self._auto_pbt_test()
            return
        if cmd == 't3':
            self.test_commit_reveal()
            return

        # Top-level shortcuts
        if cmd == 'protect':
            self._vault_secure_file()
            return
        if cmd == 'unlock':
            self._vault_unlock_file()
            return

        # Bootstrap
        if cmd in ('b', 'boot', 'bootstrap'):
            self.bootstrap()
            return

        # Connect
        if cmd in ('1', 'connect', 'reconnect'):
            self._cmd_connect()
            return

        # Guide
        if cmd == '?':
            self.show_guide()
            return

        # Domain lookup from registry
        domain_name = self._menu_aliases.get(cmd)
        if domain_name:
            domain = find_domain(domain_name)
            if domain:
                # Check for direct sub-command
                if len(parts) > 1:
                    sub_map = self._sub_aliases.get(domain_name, {})
                    sub_alias = parts[1].lower()
                    sub_key = sub_map.get(sub_alias)
                    if sub_key:
                        self._run_domain(domain, _direct=sub_key)
                        return
                self._run_domain(domain)
                return

        self._err(f"Unknown: '{line}'. Type 'help' or 'menu'.")

    # ------------------------------------------------------------------
    # Entry point
    # ------------------------------------------------------------------

    def _first_run_onboarding(self):
        if os.path.exists(self.MODE_CONFIG_FILE):
            return  # Not first run
        print()
        self._box([
            f"{C.BB}Welcome to LAUD NETWORKS{C.R}  {C.DIM}7ayLabs{C.R}",
            "",
            f"  {C.W}What you can do:{C.R}",
            f"  {C.G}>{C.R} Prove your presence at events and sessions",
            f"  {C.G}>{C.R} Protect documents with split-key encryption",
            f"  {C.G}>{C.R} Build verifiable trust relationships",
            "",
        ], color=C.BB)
        choice = self._prompt_int(
            "Choose your mode:
"
            "  1  Standard  (simple interface)
"
            "  2  Developer (full protocol access)
"
            "Mode", 1)
        self._mode = 'dev' if choice == 2 else 'normal'
        self._save_mode()
        self._menu_aliases = build_menu_aliases_for_mode(self._mode)

    def _print_welcome(self):
        mode_str = "DEVELOPER" if self._mode == 'dev' else "NORMAL"
        mode_color = C.Y if self._mode == 'dev' else C.G
        print()
        self._box([
            f"{C.BB}LAUD NETWORKS{C.R}  {C.DIM}7ayLabs{C.R}",
            f"{C.DIM}Proof of Presence Protocol"
            f"              v0.8.27{C.R}",
            "",
            f"{mode_color}[{mode_str} MODE]{C.R}"
            f"   {C.DIM}switch: mode dev | mode normal{C.R}",
        ], color=C.BB)
        print(f"  {C.DIM}Type{C.R} menu {C.DIM}for commands,"
              f"{C.R} flow {C.DIM}for next steps,"
              f"{C.R} ? {C.DIM}for guide{C.R}")
        print()

    def run(self):
        self._first_run_onboarding()
        self._print_welcome()
        self._setup_readline()

        if not SUBSTRATE_OK:
            print(f"  {C.RED}substrate-interface not found.{C.R}")
            print(f"  Run: {C.Y}pip install substrate-interface{C.R}")
            print(f"  Or:  {C.Y}source .venv/bin/activate{C.R}\n")
        else:
            self.connect(self.url)
            if not self.connected:
                print(f"  {C.DIM}Tip: run ./scripts/dev.sh "
                      f"then type 'connect'{C.R}\n")

        while True:
            try:
                line = input(self._build_prompt()).strip()
                if not line:
                    continue
                self._dispatch(line)
            except SystemExit:
                print(f"\n  {C.DIM}LAUD NETWORKS 7ayLabs{C.R}\n")
                break
            except KeyboardInterrupt:
                print()
                if self._nav_stack:
                    self._nav_stack.clear()
                    continue
                print(f"  {C.DIM}(Ctrl+C again or type 'exit' "
                      f"to quit){C.R}")
            except EOFError:
                print(f"\n  {C.DIM}LAUD NETWORKS 7ayLabs{C.R}\n")
                break


if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description="LAUD NETWORKS 7ayLabs - Proof of Presence Protocol")
    parser.add_argument(
        '--url', default='ws://127.0.0.1:9944',
        help='WebSocket endpoint (default: ws://127.0.0.1:9944)')
    parser.add_argument(
        '--mode', choices=['dev', 'normal'], default=None,
        help='UI mode: dev (all domains) or normal (simplified)')
    args = parser.parse_args()

    cli = LaudCLI(url=args.url, mode=args.mode)
    cli.run()
