#!/usr/bin/env python3
"""
LAUD NETWORKS - PoP Protocol Testing Suite
Interactive CLI for testing all 7aychain features.

Usage:
    python3 laud-cli.py [--url ws://host:port]

Requirements:
    pip install substrate-interface
"""

import sys, os, time, json, hashlib, secrets, argparse
from datetime import datetime

# ═══════════════════════════════════════════════════════════════════
#  Dependency Check
# ═══════════════════════════════════════════════════════════════════

SUBSTRATE_OK = False
try:
    from substrateinterface import SubstrateInterface, Keypair
    from substrateinterface.exceptions import SubstrateRequestException
    SUBSTRATE_OK = True
except ImportError:
    pass

# ═══════════════════════════════════════════════════════════════════
#  Terminal Colors
# ═══════════════════════════════════════════════════════════════════

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

def clear():
    os.system('cls' if os.name == 'nt' else 'clear')

# ═══════════════════════════════════════════════════════════════════
#  Branding
# ═══════════════════════════════════════════════════════════════════

def print_banner():
    clear()
    print(f"""
  {C.BB}██╗      █████╗ ██╗   ██╗██████╗
  ██║     ██╔══██╗██║   ██║██╔══██╗
  ██║     ███████║██║   ██║██║  ██║
  ██║     ██╔══██║██║   ██║██║  ██║
  ███████╗██║  ██║╚██████╔╝██████╔╝
  ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═════╝{C.R}
  {C.B}N E T W O R K S{C.R}    {C.DIM}a 7aylabs product{C.R}
  {C.DIM}─────────────────────────────────────{C.R}
  {C.W}PoP Protocol Testing Suite{C.R}  {C.DIM}v0.9.0{C.R}
""")


# ═══════════════════════════════════════════════════════════════════
#  Main CLI Class
# ═══════════════════════════════════════════════════════════════════

class LaudCLI:

    def __init__(self, url="ws://127.0.0.1:9944"):
        self.url = url
        self.substrate = None
        self.keypairs = {}
        self.connected = False
        # Context state (persistent across commands)
        self._ctx_epoch = None
        self._ctx_account = 'alice'
        self._nav_stack = []
        self._history_file = os.path.expanduser('~/.laud_history')

    # ── Connection ─────────────────────────────────────────────────

    def connect(self, url=None):
        url = url or self.url
        if not SUBSTRATE_OK:
            self._err("substrate-interface not installed")
            print(f"  Run: {C.Y}pip install substrate-interface{C.R}")
            return False
        try:
            self._info(f"Connecting to {url}...")
            self.substrate = SubstrateInterface(
                url=url,
                auto_reconnect=True,
                ws_options={'open_timeout': 10, 'ping_interval': 30, 'ping_timeout': 10},
            )
            self.url = url
            self.connected = True
            for name in ['alice','bob','charlie','dave','eve','ferdie']:
                self.keypairs[name] = Keypair.create_from_uri(f"//{name.capitalize()}")
            chain = self.substrate.rpc_request("system_chain", [])['result']
            ver = self.substrate.rpc_request("system_version", [])['result']
            self._ok(f"Connected to {C.W}{chain}{C.R} v{ver}")
            return True
        except Exception as e:
            self._err(f"Connection failed: {e}")
            return False

    def _reconnect(self):
        """Silently reconnect if the WebSocket has dropped."""
        try:
            self.substrate.rpc_request("system_chain", [])
            return True
        except Exception:
            pass
        try:
            self._info("Reconnecting...")
            self.substrate = SubstrateInterface(
                url=self.url,
                auto_reconnect=True,
                ws_options={'open_timeout': 10, 'ping_interval': 30, 'ping_timeout': 10},
            )
            self.connected = True
            return True
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

    # ── readline ────────────────────────────────────────────────

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
            pass  # Windows fallback

    _CMD_NAMES = [
        'help', 'use', 'status', 'menu', 'back', 'exit', 'bootstrap', 'connect',
        'presence', 'epoch', 'validator', 'pbt', 'triangulation',
        'dispute', 'zk', 'vault', 'device', 'lifecycle', 'governance',
        'semantic', 'boomerang', 'autonomous', 'octopus', 'storage',
        'blocks', 'inspect', 'runtime', 'network', 'crypto', 'accounts', 'events',
        'test',
    ]
    _CMD_SUBS = {
        'presence': ['declare','commit','reveal','vote','finalize','slash','quorum'],
        'epoch': ['schedule','start','close','finalize','register','update','force'],
        'validator': ['register','activate','deactivate','withdraw','stake','slash'],
        'pbt': ['position','claim','attest','verify','setup','test'],
        'test': ['pop','pbt','commit'],
        'use': ['epoch','alice','bob','charlie','dave','eve','ferdie','clear'],
    }

    def _completer(self, text, state):
        try:
            import readline
            line = readline.get_line_buffer().lstrip()
            parts = line.split()
            if not parts or (len(parts) == 1 and not line.endswith(' ')):
                prefix = parts[0] if parts else ''
                candidates = [c + ' ' for c in self._CMD_NAMES if c.startswith(prefix)]
            else:
                parent = parts[0].lower()
                subs = self._CMD_SUBS.get(parent, [])
                prefix = text.lower()
                candidates = [s + ' ' for s in subs if s.startswith(prefix)]
            return candidates[state] if state < len(candidates) else None
        except Exception:
            return None

    # ── Submission ──────────────────────────────────────────────

    def _submit(self, module, fn, params, signer='alice', sudo=False):
        if not self._ensure():
            return None
        for attempt in range(2):
            try:
                call = self.substrate.compose_call(module, fn, params)
                if sudo:
                    call = self.substrate.compose_call('Sudo', 'sudo', {'call': call})
                    signer = 'alice'
                kp = self.keypairs[signer]
                ext = self.substrate.create_signed_extrinsic(call=call, keypair=kp)
                tag = f"{C.DIM}[sudo]{C.R} " if sudo else ""
                self._info(f"{tag}{C.W}{module}.{fn}{C.R} {C.DIM}as{C.R} {C.Y}{signer}{C.R}")
                receipt = self.substrate.submit_extrinsic(ext, wait_for_inclusion=True)
                if receipt.is_success:
                    # Resolve block number for cleaner output
                    blk_num = ""
                    try:
                        hdr = self.substrate.get_block_header(block_hash=receipt.block_hash)
                        blk_num = f"#{hdr['header']['number']}"
                    except Exception:
                        blk_num = str(receipt.block_hash)[:16]
                    # Collect pallet events (skip system noise)
                    pallet_events = []
                    for ev in receipt.triggered_events:
                        ev_val = ev.value
                        if isinstance(ev_val, dict) and 'event' in ev_val:
                            edata = ev_val['event']
                            mid = edata.get('module_id', edata.get('event_index', '?'))
                            eid = edata.get('event_id', '')
                            if mid == 'System' and eid == 'ExtrinsicFailed':
                                pallet_events.append(f"{C.RED}{mid}.{eid}{C.R}")
                            elif mid not in ('System', 'TransactionPayment', 'Balances', 0):
                                pallet_events.append(f"{mid}.{eid}")
                    ev_str = f" {C.DIM}({', '.join(pallet_events)}){C.R}" if pallet_events else ""
                    self._ok(f"Block {blk_num}{ev_str}")
                else:
                    self._err(f"{receipt.error_message}")
                    if hasattr(receipt, 'error_message') and receipt.error_message:
                        err = receipt.error_message
                        if isinstance(err, dict):
                            print(f"       {C.RED}Detail: {json.dumps(err, indent=2)}{C.R}")
                    hint = self._error_hint(receipt.error_message)
                    if hint:
                        print(f"       {C.Y}Hint: {hint}{C.R}")
                return receipt
            except (ConnectionError, BrokenPipeError, OSError) as e:
                if attempt == 0:
                    self._info("Connection lost, reconnecting...")
                    if self._reconnect():
                        continue
                self._err(str(e))
                return None
            except Exception as e:
                err_msg = str(e).lower()
                if attempt == 0 and ('connection' in err_msg or 'lost' in err_msg
                                     or 'closed' in err_msg or 'websocket' in err_msg):
                    self._info("Connection lost, reconnecting...")
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
                if attempt == 0 and ('connection' in err_msg or 'lost' in err_msg
                                     or 'closed' in err_msg or 'websocket' in err_msg):
                    if self._reconnect():
                        continue
                self._err(f"{module}.{fn}: {e}")
                return None

    def _query_map(self, module, fn):
        if not self._ensure():
            return []
        for attempt in range(2):
            try:
                entries = list(self.substrate.query_map(module, fn))
                self._info(f"{C.DIM}query_map{C.R} {module}.{fn} {C.DIM}({len(entries)} entries){C.R}")
                return entries
            except Exception as e:
                err_msg = str(e).lower()
                if attempt == 0 and ('connection' in err_msg or 'lost' in err_msg
                                     or 'closed' in err_msg or 'websocket' in err_msg):
                    if self._reconnect():
                        continue
                self._err(f"{module}.{fn}: {e}")
            return []

    def _show(self, result, label=None):
        """Pretty-print a query result (handles 2-level nesting)."""
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
                        print(f"      {C.DIM}{k2:>20}{C.R}: {C.W}{v2}{C.R}")
                elif isinstance(v, list):
                    print(f"    {C.CY}{k:>24}{C.R}: {C.W}[{len(v)} items]{C.R}")
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

    # ── ID Derivation ──────────────────────────────────────────────

    def _actor_id(self, name):
        kp = self.keypairs[name]
        return '0x' + hashlib.blake2b(kp.public_key, digest_size=32).hexdigest()

    def _validator_id(self, name):
        return self._actor_id(name)

    # ── Prompt Helpers ─────────────────────────────────────────────

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
        print(f"  {C.DIM}Accounts: {', '.join(names)}  (active: {default}){C.R}")
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

    def _prompt_h256(self, label="H256"):
        val = self._prompt(label, "0x" + "00" * 32)
        if not val.startswith("0x"):
            val = "0x" + val
        return val

    def _prompt_actor(self, label="Actor"):
        use_name = self._prompt_bool("Derive ID from account name?")
        if use_name:
            name = self._prompt_account(label)
            aid = self._actor_id(name)
            print(f"  {C.DIM}ID: {aid[:20]}...{C.R}")
            return aid
        return self._prompt_h256(f"{label} ID (H256)")

    def _prompt_enum(self, label, options):
        for i, opt in enumerate(options, 1):
            print(f"    {C.Y}{i}{C.R} {opt}")
        idx = self._prompt_int(label, 1) - 1
        return options[max(0, min(idx, len(options) - 1))]

    # ── Print Helpers ──────────────────────────────────────────────

    def _ok(self, msg):
        print(f"  {C.G}[OK]{C.R} {msg}")

    def _err(self, msg):
        print(f"  {C.RED}[ERR]{C.R} {msg}")

    def _info(self, msg):
        print(f"  {C.CY}[..]{C.R} {msg}")

    def _val(self, key, val):
        v = val.value if hasattr(val, 'value') else val
        if isinstance(v, dict):
            print(f"  {C.CY}{key}{C.R}:")
            for k2, v2 in v.items():
                print(f"    {C.DIM}{k2:>22}:{C.R} {C.W}{v2}{C.R}")
        else:
            print(f"  {C.CY}{key}:{C.R} {C.W}{v}{C.R}")

    def _table(self, headers, rows):
        """Print a formatted table with aligned columns."""
        if not rows:
            print(f"  {C.DIM}(no data){C.R}")
            return
        # Calculate column widths from headers and data
        widths = [len(str(h)) for h in headers]
        for row in rows:
            for i, cell in enumerate(row):
                if i < len(widths):
                    widths[i] = max(widths[i], len(str(cell)))
        # Cap columns at 32 chars
        widths = [min(w, 32) for w in widths]
        # Header
        hdr = "  "
        sep = "  "
        for i, h in enumerate(headers):
            w = widths[i] if i < len(widths) else 10
            hdr += f"{C.BB}{str(h):<{w}}{C.R}  "
            sep += f"{C.DIM}{'─' * w}{C.R}  "
        print(hdr)
        print(sep)
        # Rows
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

    def _header(self, title):
        print(f"\n  {C.BB}{title}{C.R}")
        print(f"  {C.DIM}{'─' * min(52, len(title) + 4)}{C.R}")

    def _menu(self, title, options):
        print(f"\n  {C.BB}{title}{C.R}")
        print(f"  {C.DIM}{'─' * min(52, len(title) + 4)}{C.R}")
        for key, label in options:
            if key == "─":
                if label:
                    print(f"  {label}")
                else:
                    print()
            elif key == "?":
                print(f"  {C.DIM} ?{C.R}  {label}")
            elif key == "0":
                print(f"  {C.DIM} 0  {label}{C.R}")
            else:
                print(f"  {C.Y}{key:>2}{C.R}  {label}")
        print()
        return self._prompt("", "0")

    def _pause(self):
        pass  # no-op: output flows continuously

    # ══════════════════════════════════════════════════════════════
    #  1. CHAIN STATUS
    # ══════════════════════════════════════════════════════════════

    def menu_chain(self, _direct=None):
        self._nav_stack.append('chain')
        _opts = [
                ("1", "Node health & info"),
                ("2", "Latest block"),
                ("3", "Runtime version"),
                ("4", "Account balances"),
                ("5", "Recent events"),
                ("6", "List pallets"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("CHAIN STATUS", _opts)
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
                self._menu("CHAIN STATUS", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                if c == "1":
                    r = self.substrate.rpc_request("system_health", [])['result']
                    self._val("Peers", r.get('peers', 0))
                    self._val("Syncing", r.get('isSyncing', False))
                    self._val("Chain", self.substrate.rpc_request("system_chain", [])['result'])
                    self._val("Version", self.substrate.rpc_request("system_version", [])['result'])
                elif c == "2":
                    h = self.substrate.get_block_header()
                    self._val("Block", h['header']['number'])
                    self._val("Hash", self.substrate.get_block_hash())
                elif c == "3":
                    rv = self.substrate.rpc_request("state_getRuntimeVersion", [])['result']
                    for k in ['specName', 'specVersion', 'implVersion', 'transactionVersion']:
                        self._val(k, rv.get(k))
                elif c == "4":
                    for name, kp in self.keypairs.items():
                        r = self.substrate.query('System', 'Account', [kp.ss58_address])
                        free = r.value['data']['free'] if r else 0
                        self._val(f"{name:>8}", f"{free / 1e12:.4f} UNIT")
                elif c == "5":
                    events = self.substrate.query('System', 'Events')
                    if events and events.value:
                        for ev in events.value[-15:]:
                            mid = ev.get('event', {}).get('module_id', '?')
                            eid = ev.get('event', {}).get('event_id', '?')
                            print(f"    {C.DIM}{mid}.{eid}{C.R}")
                elif c == "6":
                    md = self.substrate.get_metadata()
                    for p in md.pallets:
                        nc = len(p.calls) if p.calls else 0
                        ns = len(p.storage) if p.storage else 0
                        print(f"    {C.B}{p.name:>20}{C.R}  calls={nc}  storage={ns}")
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  2. PRESENCE PROTOCOL
    # ══════════════════════════════════════════════════════════════

    def menu_presence(self, _direct=None):
        self._check_epoch()
        self._nav_stack.append('presence')
        _opts = [
                ("1", "Declare Presence"),
                ("2", "Declare with Commitment"),
                ("3", "Reveal Commitment"),
                ("4", "Vote on Presence"),
                ("5", "Finalize Presence"),
                ("6", f"{C.RED}Slash Presence [sudo]{C.R}"),
                ("7", "Set Quorum Config [sudo]"),
                ("8", "Set Validator Status [sudo]"),
                ("9", "Set Epoch Active [sudo]"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Current Epoch"),
                ("b", "Presence Record"),
                ("c", "Vote Count"),
                ("d", "Active Validators"),
                ("e", "Commitment / Reveal Count"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("PRESENCE PROTOCOL", _opts)
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
                self._menu("PRESENCE PROTOCOL", _opts)
                continue
            elif c == "1":
                e = self._prompt_epoch()
                a = self._prompt_account("Signer")
                self._submit("Presence", "declare_presence", {"epoch": e}, a)
            elif c == "2":
                e = self._prompt_epoch()
                a = self._prompt_account("Signer")
                sec = secrets.token_hex(32)
                rnd = secrets.token_hex(32)
                h = hashlib.blake2b(bytes.fromhex(sec + rnd), digest_size=32).hexdigest()
                print(f"  {C.DIM}Secret:     {sec[:32]}...{C.R}")
                print(f"  {C.DIM}Randomness: {rnd[:32]}...{C.R}")
                print(f"  {C.DIM}Commitment: 0x{h[:32]}...{C.R}")
                self._submit("Presence", "declare_presence_with_commitment",
                             {"epoch": e, "commitment": "0x" + h}, a)
            elif c == "3":
                e   = self._prompt_epoch()
                sec = self._prompt("Secret (hex from step 2)")
                rnd = self._prompt("Randomness (hex from step 2)")
                a   = self._prompt_account("Signer")
                self._submit("Presence", "reveal_commitment",
                             {"epoch": e, "secret": sec, "randomness": rnd}, a)
            elif c == "4":
                actor   = self._prompt_actor("Target actor")
                e       = self._prompt_epoch()
                approve = self._prompt_bool("Approve?")
                a       = self._prompt_account("Voter")
                self._submit("Presence", "vote_presence",
                             {"actor": actor, "epoch": e, "approve": approve}, a)
            elif c == "5":
                actor = self._prompt_actor("Target actor")
                e     = self._prompt_epoch()
                a     = self._prompt_account("Signer")
                self._submit("Presence", "finalize_presence",
                             {"actor": actor, "epoch": e}, a)
            elif c == "6":
                actor = self._prompt_actor("Target actor")
                e     = self._prompt_epoch()
                self._submit("Presence", "slash_presence",
                             {"actor": actor, "epoch": e}, sudo=True)
            elif c == "7":
                t = self._prompt_int("Threshold", 2)
                n = self._prompt_int("Total", 3)
                self._submit("Presence", "set_quorum_config",
                             {"threshold": t, "total": n}, sudo=True)
            elif c == "8":
                vid = self._prompt_actor("Validator")
                act = self._prompt_bool("Active?")
                self._submit("Presence", "set_validator_status",
                             {"validator": vid, "active": act}, sudo=True)
            elif c == "9":
                e   = self._prompt_epoch()
                act = self._prompt_bool("Active?")
                self._submit("Presence", "set_epoch_active",
                             {"epoch": e, "active": act}, sudo=True)
            elif c == "a":
                self._val("Current Epoch", self._query("Presence", "CurrentEpoch"))
            elif c == "b":
                e = self._prompt_epoch()
                actor = self._prompt_actor("Actor")
                self._val("Presence", self._query("Presence", "Presences", [e, actor]))
            elif c == "c":
                e = self._prompt_epoch()
                actor = self._prompt_actor("Actor")
                self._val("Votes", self._query("Presence", "VoteCount", [e, actor]))
            elif c == "d":
                for k, v in self._query_map("Presence", "ActiveValidators")[:10]:
                    print(f"    {C.DIM}{k.value[:20] if hasattr(k,'value') else k}... = {v.value if hasattr(v,'value') else v}{C.R}")
            elif c == "e":
                e = self._prompt_epoch()
                self._val("Commitments", self._query("Presence", "CommitmentCount", [e]))
                self._val("Reveals", self._query("Presence", "RevealCount", [e]))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  3. EPOCH MANAGEMENT
    # ══════════════════════════════════════════════════════════════

    def menu_epoch(self, _direct=None):
        self._nav_stack.append('epoch')
        _opts = [
                ("1", "Schedule Epoch [sudo]"),
                ("2", "Start Epoch [sudo]"),
                ("3", "Close Epoch [sudo]"),
                ("4", "Finalize Epoch [sudo]"),
                ("5", "Register Participant"),
                ("6", "Update Schedule [sudo]"),
                ("7", "Force Transition [sudo]"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Current Epoch"),
                ("b", "Epoch Info"),
                ("c", "Epoch Count"),
                ("d", "Epoch Schedule"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("EPOCH MANAGEMENT", _opts)
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
                self._menu("EPOCH MANAGEMENT", _opts)
                continue
            elif c == "1":
                s = self._prompt_int("Start block", 10)
                d = self._prompt_int("Duration (blocks)", 60)
                self._submit("Epoch", "schedule_epoch",
                             {"start_block": s, "duration": d}, sudo=True)
            elif c == "2":
                self._submit("Epoch", "start_epoch",
                             {"epoch_id": self._prompt_int("Epoch ID", 1)}, sudo=True)
            elif c == "3":
                self._submit("Epoch", "close_epoch",
                             {"epoch_id": self._prompt_int("Epoch ID", 1)}, sudo=True)
            elif c == "4":
                self._submit("Epoch", "finalize_epoch",
                             {"epoch_id": self._prompt_int("Epoch ID", 1)}, sudo=True)
            elif c == "5":
                e = self._prompt_int("Epoch ID", 1)
                a = self._prompt_account()
                self._submit("Epoch", "register_participant", {"epoch_id": e}, a)
            elif c == "6":
                d = self._prompt_int("Duration", 60)
                g = self._prompt_int("Grace period", 2)
                t = self._prompt_bool("Auto-transition?")
                self._submit("Epoch", "update_schedule",
                             {"duration": d, "grace_period": g, "auto_transition": t}, sudo=True)
            elif c == "7":
                e = self._prompt_int("Epoch ID", 1)
                s = self._prompt_enum("State:", ["Scheduled","Active","Closed","Finalized"])
                self._submit("Epoch", "force_transition",
                             {"epoch_id": e, "new_state": s}, sudo=True)
            elif c == "a":
                self._val("Current Epoch", self._query("Epoch", "CurrentEpoch"))
            elif c == "b":
                self._val("Info", self._query("Epoch", "EpochInfo",
                          [self._prompt_epoch()]))
            elif c == "c":
                self._val("Total", self._query("Epoch", "EpochCount"))
            elif c == "d":
                self._val("Schedule", self._query("Epoch", "EpochSchedule"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  4. VALIDATOR OPERATIONS
    # ══════════════════════════════════════════════════════════════

    def menu_validator(self, _direct=None):
        self._nav_stack.append('validator')
        _opts = [
                ("1", "Register Validator"),
                ("2", "Activate Validator"),
                ("3", "Deactivate Validator"),
                ("4", "Withdraw Stake"),
                ("5", "Increase Stake"),
                ("6", f"{C.RED}Slash Validator [sudo]{C.R}"),
                ("7", "Apply Slash [sudo]"),
                ("8", "Report Evidence"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Validator Info"),
                ("b", "Validator Count / Total Stake"),
                ("c", "Pending Slashes"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("VALIDATOR OPERATIONS", _opts)
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
                self._menu("VALIDATOR OPERATIONS", _opts)
                continue
            elif c == "1":
                s = self._prompt_int("Stake", 1000000)
                a = self._prompt_account()
                self._submit("Validator", "register_validator", {"stake": s}, a)
            elif c == "2":
                self._submit("Validator", "activate_validator", {},
                             self._prompt_account())
            elif c == "3":
                self._submit("Validator", "deactivate_validator", {},
                             self._prompt_account())
            elif c == "4":
                self._submit("Validator", "withdraw_stake", {},
                             self._prompt_account())
            elif c == "5":
                s = self._prompt_int("Additional stake", 100000)
                a = self._prompt_account()
                self._submit("Validator", "increase_stake", {"additional": s}, a)
            elif c == "6":
                vid = self._prompt_actor("Validator")
                v = self._prompt_enum("Violation:", ["Minor","Moderate","Severe","Critical"])
                self._submit("Validator", "slash_validator",
                             {"validator": vid, "violation": v}, sudo=True)
            elif c == "7":
                self._submit("Validator", "apply_slash",
                             {"slash_id": self._prompt_int("Slash ID", 0)}, sudo=True)
            elif c == "8":
                vid = self._prompt_actor("Validator")
                v = self._prompt_enum("Violation:", ["Minor","Moderate","Severe","Critical"])
                a = self._prompt_account()
                self._submit("Validator", "report_evidence",
                             {"validator": vid, "violation": v}, a)
            elif c == "a":
                self._val("Validator", self._query("Validator", "Validators",
                          [self._prompt_actor("Validator")]))
            elif c == "b":
                self._val("Count", self._query("Validator", "ValidatorCount"))
                self._val("Total Stake", self._query("Validator", "TotalStake"))
            elif c == "c":
                for k, v in self._query_map("Validator", "PendingSlashes")[:10]:
                    print(f"    {C.DIM}#{k.value if hasattr(k,'value') else k}: {v.value if hasattr(v,'value') else v}{C.R}")
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  5. POSITION-BASED TRIANGULATION (PBT)
    # ══════════════════════════════════════════════════════════════

    def menu_pbt(self, _direct=None):
        self._check_epoch()
        self._nav_stack.append('pbt')
        _opts = [
                ("1", "Set Validator Position"),
                ("2", "Claim Position"),
                ("3", "Submit Witness Attestation"),
                ("4", "Verify Position"),
                ("─", f"{C.DIM}── Automated ──{C.R}"),
                ("5", f"{C.G}Setup All Validators (auto){C.R}"),
                ("6", f"{C.G}Full PBT Test Flow (auto){C.R}"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Position Claim"),
                ("b", "Attestation Count"),
                ("c", "Validator Positions"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("POSITION-BASED TRIANGULATION", _opts)
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
                self._menu("POSITION-BASED TRIANGULATION", _opts)
                continue
            elif c == "1":
                name = self._prompt_account("Validator")
                vid = self._validator_id(name)
                pos = self._prompt_position("Validator position")
                self._submit("Presence", "set_validator_position",
                             {"validator": vid, "position": pos}, name)
            elif c == "2":
                e   = self._prompt_epoch()
                pos = self._prompt_position("Claimed position")
                a   = self._prompt_account("Claimer")
                self._submit("Presence", "claim_position",
                             {"epoch": e, "position": pos}, a)
            elif c == "3":
                target  = self._prompt_actor("Target actor")
                e       = self._prompt_epoch()
                lat     = self._prompt_int("Latency ms", 5)
                direct  = self._prompt_bool("Direct connection?")
                w       = self._prompt_account("Witness")
                self._submit("Presence", "submit_witness_attestation",
                             {"target": target, "epoch": e,
                              "latency_ms": lat, "direct_connection": direct}, w)
            elif c == "4":
                target = self._prompt_actor("Target")
                e      = self._prompt_epoch()
                a      = self._prompt_account("Caller")
                self._submit("Presence", "verify_position",
                             {"target": target, "epoch": e}, a)
            elif c == "5":
                self._auto_setup_validators()
            elif c == "6":
                self._auto_pbt_test()
            elif c == "a":
                e = self._prompt_epoch()
                actor = self._prompt_actor("Actor")
                self._val("Claim", self._query("Presence", "PositionClaims", [e, actor]))
            elif c == "b":
                e = self._prompt_epoch()
                actor = self._prompt_actor("Actor")
                self._val("Count", self._query("Presence", "AttestationCount", [e, actor]))
            elif c == "c":
                for k, v in self._query_map("Presence", "ValidatorPositions")[:10]:
                    kv = k.value if hasattr(k, 'value') else str(k)
                    vv = v.value if hasattr(v, 'value') else str(v)
                    kid = str(kv)[:20] if kv else '?'
                    print(f"    {C.DIM}{kid}... = {vv}{C.R}")
            self._pause()

    def _auto_setup_validators(self):
        if not self._ensure(): return
        positions = {
            'alice':   {"x": 0,      "y": 0,      "z": 0},
            'bob':     {"x": 50000,  "y": 0,      "z": 0},
            'charlie': {"x": 25000,  "y": 43301,  "z": 0},
            'dave':    {"x": -25000, "y": 43301,  "z": 0},
            'eve':     {"x": -50000, "y": 0,      "z": 0},
            'ferdie':  {"x": -25000, "y": -43301, "z": 0},
        }
        total = 1 + len(positions) * 2  # 1 epoch + 6 register + 6 position
        step = 0
        step += 1
        print(f"  {C.DIM}[{step}/{total}]{C.R} Activating epoch 1")
        self._submit("Presence", "set_epoch_active",
                     {"epoch": 1, "active": True}, sudo=True)
        for name, pos in positions.items():
            vid = self._validator_id(name)
            step += 1
            print(f"  {C.DIM}[{step}/{total}]{C.R} Register {C.W}{name}{C.R}")
            self._submit("Presence", "set_validator_status",
                         {"validator": vid, "active": True}, sudo=True)
            step += 1
            print(f"  {C.DIM}[{step}/{total}]{C.R} Position {C.W}{name}{C.R} ({pos['x']}, {pos['y']}, {pos['z']})")
            self._submit("Presence", "set_validator_position",
                         {"validator": vid, "position": pos}, name)
        self._ok(f"Bootstrap complete — 6 validators in hexagonal formation")

    def _auto_pbt_test(self):
        """PBT test with geometrically consistent triangulation.

        The triangulation algorithm uses weighted-centroid:
            weight_i = 1000 / (max_distance_km_i + 1)
            triangulated = Σ(witness_pos_i × weight_i) / Σ(weight_i)
        where max_distance_km = (latency_ms / 2) × 150.

        With equal latency across witnesses, all weights are equal and the
        triangulated position equals the unweighted centroid of the witnesses.

        Witnesses (from bootstrap hexagonal layout):
            bob(50000, 0, 0)  charlie(25000, 43301, 0)  dave(-25000, 43301, 0)
        Centroid = (16666, 28867, 0)

        Alice claims exactly this centroid → deviation = 0 → verified = True.
        """
        if not self._ensure(): return
        self._check_epoch()
        epoch = self._next_test_epoch()
        alice_id = self._actor_id('alice')

        # Claim at centroid of witnesses for geometrically exact match
        claim = {"x": 16666, "y": 28867, "z": 0}

        self._header("PBT TEST FLOW")
        self._info(f"Epoch {epoch} — Alice claims ({claim['x']}, {claim['y']}, {claim['z']})")
        self._info(f"  = centroid of bob, charlie, dave (equal-weight triangulation)")
        self._submit("Presence", "claim_position",
                     {"epoch": epoch, "position": claim}, "alice")

        # Equal latency → equal weight → centroid matches claim
        for w in ['bob', 'charlie', 'dave']:
            self._info(f"{w} attesting (10ms RTT → 750km → weight 1)...")
            self._submit("Presence", "submit_witness_attestation",
                         {"target": alice_id, "epoch": epoch,
                          "latency_ms": 10, "direct_connection": True}, w)

        self._info("Verifying position via triangulation...")
        self._submit("Presence", "verify_position",
                     {"target": alice_id, "epoch": epoch}, "bob")

        r = self._query("Presence", "PositionClaims", [epoch, alice_id])
        self._val("Result", r)
        if r and hasattr(r, 'value'):
            rv = r.value if hasattr(r, 'value') else r
            if isinstance(rv, dict):
                v = rv.get('verified', False)
                c = rv.get('confidence', 0)
                self._val("Verified", v)
                self._val("Confidence", f"{c}%")
        self._ok("PBT test complete!")

    # ── Error Hints ───────────────────────────────────────────────

    ERROR_HINTS = {
        'EpochNotActive': 'Run "bootstrap" to set up the devnet first',
        'NotAValidator': 'Run "bootstrap" to register validators',
        'NotAnActiveValidator': 'Run "bootstrap" to register validators',
        'PositionAlreadyClaimed': 'Already claimed this epoch — try "use epoch <N>" with a fresh epoch',
        'DuplicateAttestation': 'This witness already attested this epoch',
        'DuplicatePresence': 'Already declared this epoch — try "use epoch <N>" with a fresh epoch',
        'DuplicateVote': 'Already voted this epoch — try "use epoch <N>" with a fresh epoch',
        'PresenceImmutable': 'Presence already finalized — cannot modify in this epoch',
        'SelfAttestation': 'Validators cannot self-attest — use a different witness',
        'InsufficientAttestations': 'Need 3+ witness attestations first',
        'InsufficientWitnesses': 'Need 3+ witness attestations before verify_position',
        'AlreadyDeclared': 'Already declared presence this epoch',
        'QuorumNotReached': 'Need 3+ validator votes to finalize',
    }

    def _error_hint(self, err):
        """Return a user-friendly hint for a known pallet error, or None."""
        err_str = str(err)
        for key, hint in self.ERROR_HINTS.items():
            if key in err_str:
                return hint
        return None

    # ── Context Commands ────────────────────────────────────────────

    def _cmd_use(self, args):
        """Handle the 'use' command for setting context."""
        if not args:
            self._info(f"epoch={C.W}{self._ctx_epoch or 'auto'}{C.R}  "
                       f"account={C.W}{self._ctx_account}{C.R}")
            self._info(f"Usage: use epoch <N> | use <account> | use clear")
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
            self._err(f"Unknown: '{args[0]}'. Try: use epoch 5, use bob, use clear")

    def _show_status(self):
        """Show compact chain status."""
        parts = [f"{C.BB}laud{C.R}"]
        if self.connected:
            parts.append(f"{C.DIM}{self.url}{C.R}")
            try:
                blk = self.substrate.get_block_header()['header']['number']
                parts.append(f"{C.G}block #{blk}{C.R}")
            except Exception:
                parts.append(f"{C.G}connected{C.R}")
        else:
            parts.append(f"{C.RED}offline{C.R}")
        if self._ctx_epoch is not None:
            parts.append(f"{C.Y}epoch {self._ctx_epoch}{C.R}")
        acct = self._ctx_account
        sudo_tag = f" {C.DIM}(sudo){C.R}" if acct == 'alice' else ""
        parts.append(f"account: {C.W}{acct}{C.R}{sudo_tag}")
        print(f"  {'  '.join(parts)}")

    # ── Bootstrap ─────────────────────────────────────────────────

    def bootstrap(self):
        """Bootstrap devnet: activate epoch 1, register 6 validators, hexagonal positions."""
        self._auto_setup_validators()

    def _check_epoch(self):
        """Check if epoch 1 is active. If not, offer to bootstrap."""
        if not self._ensure():
            return False
        try:
            result = self.substrate.query("Presence", "EpochActive", [1])
            if result and result.value:
                return True
        except Exception:
            pass
        self._info("Epoch 1 is not active yet.")
        if self._prompt_bool("Run bootstrap? (activates epoch + validators + positions)"):
            self.bootstrap()
            return True
        return False

    def _next_test_epoch(self):
        """Find and activate a fresh epoch for testing.

        Scans from epoch 2 upward to find one that is not yet active,
        activates it via sudo, and returns the epoch number.
        Validators remain active from bootstrap — no re-registration needed.
        """
        for e in range(2, 1000):
            try:
                result = self.substrate.query("Presence", "EpochActive", [e])
                if not (result and result.value):
                    self._info(f"Activating fresh epoch {e}")
                    self._submit("Presence", "set_epoch_active",
                                 {"epoch": e, "active": True}, sudo=True)
                    return e
            except Exception:
                self._info(f"Activating fresh epoch {e}")
                self._submit("Presence", "set_epoch_active",
                             {"epoch": e, "active": True}, sudo=True)
                return e
        return 2

    # ══════════════════════════════════════════════════════════════
    #  6. DISPUTE RESOLUTION
    # ══════════════════════════════════════════════════════════════

    def menu_dispute(self, _direct=None):
        self._nav_stack.append('dispute')
        _opts = [
                ("1", "Open Dispute"),
                ("2", "Submit Evidence"),
                ("3", "Resolve Dispute [sudo]"),
                ("4", "Reject Dispute [sudo]"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Dispute Info"),
                ("b", "Open Disputes"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("DISPUTE RESOLUTION", _opts)
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
                self._menu("DISPUTE RESOLUTION", _opts)
                continue
            elif c == "1":
                t = self._prompt_actor("Target validator")
                v = self._prompt_enum("Violation:", ["Minor","Moderate","Severe","Critical"])
                a = self._prompt_account()
                self._submit("Dispute", "open_dispute",
                             {"target": t, "violation": v}, a)
            elif c == "2":
                d = self._prompt_int("Dispute ID", 0)
                h = self._prompt_h256("Evidence hash")
                a = self._prompt_account()
                self._submit("Dispute", "submit_evidence",
                             {"dispute_id": d, "data_hash": h}, a)
            elif c == "3":
                d = self._prompt_int("Dispute ID", 0)
                o = self._prompt_enum("Outcome:", [
                    "ValidatorSlashed","DisputeRejected","InsufficientEvidence"])
                self._submit("Dispute", "resolve_dispute",
                             {"dispute_id": d, "outcome": o}, sudo=True)
            elif c == "4":
                d = self._prompt_int("Dispute ID", 0)
                r = self._prompt("Reason", "InsufficientEvidence")
                self._submit("Dispute", "reject_dispute",
                             {"dispute_id": d, "reason": r}, sudo=True)
            elif c == "a":
                self._val("Dispute", self._query("Dispute", "Disputes",
                          [self._prompt_int("Dispute ID", 0)]))
            elif c == "b":
                self._val("Open", self._query("Dispute", "OpenDisputes"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  7. SIGNAL TRIANGULATION
    # ══════════════════════════════════════════════════════════════

    def menu_triangulation(self, _direct=None):
        self._nav_stack.append('triangulation')
        _opts = [
                ("1", "Register Reporter"),
                ("2", "Deregister Reporter"),
                ("3", "Report Signal"),
                ("4", "Update Reporter Position"),
                ("5", "Submit Fraud Proof"),
                ("6", "Resolve Fraud Case [sudo]"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Reporter Info"),
                ("b", "Device / Ghost Count"),
                ("c", "Fraud Cases"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("SIGNAL TRIANGULATION", _opts)
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
                self._menu("SIGNAL TRIANGULATION", _opts)
                continue
            elif c == "1":
                pos = self._prompt_position("Reporter pos")
                a   = self._prompt_account()
                self._submit("Triangulation", "register_reporter",
                             {"position": pos}, a)
            elif c == "2":
                r = self._prompt_int("Reporter ID", 0)
                a = self._prompt_account()
                self._submit("Triangulation", "deregister_reporter",
                             {"reporter_id": r}, a)
            elif c == "3":
                rid  = self._prompt_int("Reporter ID", 0)
                mac  = self._prompt_h256("MAC hash")
                rssi = self._prompt_int("RSSI (dBm)", -60)
                st   = self._prompt_enum("Signal:", [
                    "NetworkLatency","PeerTopology","BlockPropagation",
                    "IPGeolocation","GPSConsent","ConsensusWitness"])
                freq = self._prompt_int("Freq MHz (0=none)", 0)
                a    = self._prompt_account()
                self._submit("Triangulation", "report_signal",
                             {"reporter_id": rid, "mac_hash": mac, "rssi": rssi,
                              "signal_type": st, "frequency": None if freq == 0 else freq}, a)
            elif c == "4":
                rid = self._prompt_int("Reporter ID", 0)
                pos = self._prompt_position("New pos")
                a   = self._prompt_account()
                self._submit("Triangulation", "update_reporter_position",
                             {"reporter_id": rid, "new_position": pos}, a)
            elif c == "5":
                sub  = self._prompt_int("Submitter reporter ID", 0)
                acc  = self._prompt_int("Accused reporter ID", 1)
                z    = self._prompt_int("Z-score x100", 350)
                n    = self._prompt_int("Sample size", 10)
                a    = self._prompt_account()
                self._submit("Triangulation", "submit_fraud_proof",
                             {"submitter_id": sub, "proof": {
                                 "accused_reporter": acc,
                                 "conflicting_readings": [],
                                 "z_score_scaled": z, "sample_size": n}}, a)
            elif c == "6":
                rid = self._prompt_int("Reporter ID", 0)
                g   = self._prompt_bool("Guilty?")
                self._submit("Triangulation", "resolve_fraud_case",
                             {"reporter_id": rid, "guilty": g}, sudo=True)
            elif c == "a":
                self._val("Reporter", self._query("Triangulation", "Reporters",
                          [self._prompt_int("ID", 0)]))
            elif c == "b":
                self._val("Devices", self._query("Triangulation", "DeviceCount"))
                self._val("Ghosts", self._query("Triangulation", "GhostCount"))
            elif c == "c":
                for k, v in self._query_map("Triangulation", "FraudCases")[:10]:
                    print(f"    {C.DIM}#{k.value if hasattr(k,'value') else k}{C.R}")
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  8. DEVICE MANAGEMENT
    # ══════════════════════════════════════════════════════════════

    def menu_device(self, _direct=None):
        self._nav_stack.append('device')
        _opts = [
                ("1", "Register Device"),
                ("2", "Activate / Reactivate Device"),
                ("3", "Suspend Device"),
                ("4", "Revoke / Mark Compromised"),
                ("5", "Submit Attestation"),
                ("6", "Record Heartbeat"),
                ("7", "Update Trust Score"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Device Info"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("DEVICE MANAGEMENT", _opts)
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
                self._menu("DEVICE MANAGEMENT", _opts)
                continue
            elif c == "1":
                owner = self._prompt_actor("Owner")
                dt    = self._prompt_enum("Type:", ["Mobile","Desktop","Server","IoT","Hardware","Virtual"])
                pk    = self._prompt_h256("Public key hash")
                at    = self._prompt("Attestation type", "SelfSigned")
                a     = self._prompt_account()
                self._submit("Device", "register_device",
                             {"owner": owner, "device_type": dt,
                              "public_key_hash": pk, "attestation_type": at}, a)
            elif c == "2":
                did = self._prompt_int("Device ID", 0)
                a   = self._prompt_account()
                act = self._prompt_enum("Action:", ["activate_device","reactivate_device"])
                self._submit("Device", act, {"device_id": did}, a)
            elif c == "3":
                did    = self._prompt_int("Device ID", 0)
                reason = self._prompt_h256("Reason hash")
                a      = self._prompt_account()
                self._submit("Device", "suspend_device",
                             {"device_id": did, "reason": reason}, a)
            elif c == "4":
                did = self._prompt_int("Device ID", 0)
                act = self._prompt_enum("Action:", ["revoke_device","mark_compromised"])
                a   = self._prompt_account()
                self._submit("Device", act, {"device_id": did}, a)
            elif c == "5":
                did = self._prompt_int("Device ID", 0)
                ah  = self._prompt_h256("Attestation hash")
                a   = self._prompt_account()
                self._submit("Device", "submit_attestation",
                             {"device_id": did, "attestation_hash": ah, "attester": None}, a)
            elif c == "6":
                did = self._prompt_int("Device ID", 0)
                seq = self._prompt_int("Sequence", 1)
                a   = self._prompt_account()
                self._submit("Device", "record_heartbeat",
                             {"device_id": did, "sequence": seq}, a)
            elif c == "7":
                did = self._prompt_int("Device ID", 0)
                sc  = self._prompt_int("Score (0-100)", 50)
                a   = self._prompt_account()
                self._submit("Device", "update_trust_score",
                             {"device_id": did, "new_score": sc}, a)
            elif c == "a":
                self._val("Device", self._query("Device", "Devices",
                          [self._prompt_int("ID", 0)]))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  9. LIFECYCLE MANAGEMENT
    # ══════════════════════════════════════════════════════════════

    def menu_lifecycle(self, _direct=None):
        self._nav_stack.append('lifecycle')
        _opts = [
                ("1", "Register Actor"),
                ("2", "Activate Actor [sudo]"),
                ("3", "Suspend / Reactivate [sudo]"),
                ("4", "Initiate Destruction"),
                ("5", "Attest Destruction"),
                ("6", "Cancel Destruction"),
                ("7", "Initiate Key Rotation"),
                ("8", "Complete Key Rotation"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Actor Info"),
                ("b", "Actor Count"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("LIFECYCLE MANAGEMENT", _opts)
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
                self._menu("LIFECYCLE MANAGEMENT", _opts)
                continue
            elif c == "1":
                kh = self._prompt_h256("Key hash")
                a  = self._prompt_account()
                self._submit("Lifecycle", "register_actor", {"key_hash": kh}, a)
            elif c == "2":
                actor = self._prompt_actor("Actor")
                self._submit("Lifecycle", "activate_actor", {"actor": actor}, sudo=True)
            elif c == "3":
                actor = self._prompt_actor("Actor")
                act   = self._prompt_enum("Action:", ["suspend_actor","reactivate_actor"])
                self._submit("Lifecycle", act, {"actor": actor}, sudo=True)
            elif c == "4":
                reason = self._prompt_enum("Reason:", [
                    "OwnerRequest","SecurityBreach","Expiration",
                    "ProtocolViolation","Administrative"])
                a = self._prompt_account()
                self._submit("Lifecycle", "initiate_destruction", {"reason": reason}, a)
            elif c == "5":
                target = self._prompt_actor("Target")
                sig    = self._prompt_h256("Signature hash")
                a      = self._prompt_account()
                self._submit("Lifecycle", "attest_destruction",
                             {"target_actor": target, "signature_hash": sig}, a)
            elif c == "6":
                self._submit("Lifecycle", "cancel_destruction", {},
                             self._prompt_account())
            elif c == "7":
                nk = self._prompt_h256("New key hash")
                a  = self._prompt_account()
                self._submit("Lifecycle", "initiate_rotation", {"new_key_hash": nk}, a)
            elif c == "8":
                self._submit("Lifecycle", "complete_rotation", {},
                             self._prompt_account())
            elif c == "a":
                self._val("Actor", self._query("Lifecycle", "Actors",
                          [self._prompt_actor("Actor")]))
            elif c == "b":
                self._val("Count", self._query("Lifecycle", "ActorCount"))
                self._val("Active", self._query("Lifecycle", "ActiveActors"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  10. CRYPTOGRAPHIC VAULT
    # ══════════════════════════════════════════════════════════════

    def menu_vault(self, _direct=None):
        self._nav_stack.append('vault')
        _opts = [
                ("1", "Create Vault"),
                ("2", "Add Member"),
                ("3", "Activate Vault"),
                ("4", "Commit Share"),
                ("5", "Reveal Share"),
                ("6", "Initiate Recovery"),
                ("7", "Lock Vault"),
                ("8", "Dissolve Vault"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Vault Info"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("CRYPTOGRAPHIC VAULT (Shamir t-of-n)", _opts)
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
                self._menu("CRYPTOGRAPHIC VAULT (Shamir t-of-n)", _opts)
                continue
            elif c == "1":
                owner = self._prompt_actor("Owner")
                t = self._prompt_int("Threshold (t)", 2)
                n = self._prompt_int("Ring size (n)", 3)
                sh = self._prompt_h256("Secret hash")
                a = self._prompt_account()
                self._submit("Vault", "create_vault",
                             {"owner": owner, "threshold": t, "ring_size": n,
                              "secret_hash": sh}, a)
            elif c == "2":
                vid = self._prompt_int("Vault ID", 0)
                mem = self._prompt_actor("Member")
                role = self._prompt("Role", "Member")
                a = self._prompt_account()
                self._submit("Vault", "add_member",
                             {"vault_id": vid, "member": mem, "role": role}, a)
            elif c in ("3","6","7","8"):
                vid = self._prompt_int("Vault ID", 0)
                a = self._prompt_account()
                fn = {"3":"activate_vault","6":"initiate_recovery",
                      "7":"lock_vault","8":"dissolve_vault"}[c]
                self._submit("Vault", fn, {"vault_id": vid}, a)
            elif c == "4":
                vid = self._prompt_int("Vault ID", 0)
                cm  = self._prompt_h256("Commitment")
                a   = self._prompt_account()
                self._submit("Vault", "commit_share",
                             {"vault_id": vid, "commitment": cm}, a)
            elif c == "5":
                sid = self._prompt_int("Share ID", 0)
                a   = self._prompt_account()
                self._submit("Vault", "reveal_share", {"share_id": sid}, a)
            elif c == "a":
                self._val("Vault", self._query("Vault", "Vaults",
                          [self._prompt_int("ID", 0)]))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  11. ZERO-KNOWLEDGE PROOFS
    # ══════════════════════════════════════════════════════════════

    def menu_zk(self, _direct=None):
        self._nav_stack.append('zk')
        _opts = [
                ("1", "Verify Share Proof"),
                ("2", "Verify Presence Proof"),
                ("3", "Verify Access Proof"),
                ("4", "Register SNARK Circuit [sudo]"),
                ("5", "Verify SNARK"),
                ("6", "Consume Nullifier"),
                ("7", "Add/Remove Trusted Verifier [sudo]"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Verification Count"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("ZERO-KNOWLEDGE PROOFS", _opts)
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
                self._menu("ZERO-KNOWLEDGE PROOFS", _opts)
                continue
            elif c == "1":
                cm = self._prompt_h256("Commitment hash")
                pr = self._prompt("Proof hex", "00" * 32)
                a  = self._prompt_account()
                self._submit("Zk", "verify_share_proof",
                             {"statement": {"commitment_hash": cm},
                              "proof": "0x" + pr}, a)
            elif c == "2":
                actor = self._prompt_actor("Actor")
                e     = self._prompt_epoch()
                pr    = self._prompt("Proof hex", "00" * 32)
                a     = self._prompt_account()
                self._submit("Zk", "verify_presence_proof",
                             {"statement": {"actor": actor, "epoch": e},
                              "proof": "0x" + pr}, a)
            elif c == "3":
                actor = self._prompt_actor("Actor")
                res   = self._prompt_h256("Resource ID")
                pr    = self._prompt("Proof hex", "00" * 32)
                a     = self._prompt_account()
                self._submit("Zk", "verify_access_proof",
                             {"statement": {"actor": actor, "resource": res},
                              "proof": "0x" + pr}, a)
            elif c == "4":
                cid = self._prompt_h256("Circuit ID")
                pt  = self._prompt_enum("Type:", ["Groth16","PlonK","Halo2"])
                vk  = self._prompt("VK hex", "00" * 32)
                self._submit("Zk", "register_circuit",
                             {"circuit_id": cid, "proof_type": pt,
                              "vk": "0x" + vk}, sudo=True)
            elif c == "5":
                cid = self._prompt_h256("Circuit ID")
                pr  = self._prompt("Proof hex", "00" * 64)
                a   = self._prompt_account()
                self._submit("Zk", "verify_snark",
                             {"circuit_id": cid, "proof": "0x" + pr, "inputs": []}, a)
            elif c == "6":
                nl = self._prompt_h256("Nullifier")
                a  = self._prompt_account()
                self._submit("Zk", "consume_nullifier", {"nullifier": nl}, a)
            elif c == "7":
                act = self._prompt_enum("Action:", ["add_trusted_verifier","remove_trusted_verifier"])
                v   = self._prompt_actor("Verifier")
                self._submit("Zk", act, {"verifier": v}, sudo=True)
            elif c == "a":
                self._val("Count", self._query("Zk", "VerificationCount"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  12. GOVERNANCE
    # ══════════════════════════════════════════════════════════════

    def menu_governance(self, _direct=None):
        self._nav_stack.append('governance')
        _opts = [
                ("1", "Grant Capability"),
                ("2", "Revoke Capability"),
                ("3", "Delegate Capability"),
                ("4", "Update Permissions"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Capability Info"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("GOVERNANCE & CAPABILITIES", _opts)
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
                self._menu("GOVERNANCE & CAPABILITIES", _opts)
                continue
            elif c == "1":
                grantee = self._prompt_actor("Grantee")
                res     = self._prompt_h256("Resource ID")
                perms   = self._prompt_int("Permissions bitmask (R=1 W=2 X=4 D=8 A=16)", 7)
                deleg   = self._prompt_bool("Delegatable?")
                a       = self._prompt_account()
                self._submit("Governance", "grant_capability",
                             {"grantee": grantee, "resource": res,
                              "permissions": perms, "expires_at": None,
                              "delegatable": deleg}, a)
            elif c == "2":
                cid = self._prompt_int("Capability ID", 0)
                a   = self._prompt_account()
                self._submit("Governance", "revoke_capability",
                             {"capability_id": cid}, a)
            elif c == "3":
                cid  = self._prompt_int("Capability ID", 0)
                dele = self._prompt_actor("Delegatee")
                p    = self._prompt_int("Permissions", 1)
                a    = self._prompt_account()
                self._submit("Governance", "delegate_capability",
                             {"capability_id": cid, "delegatee": dele,
                              "permissions": p, "expires_at": None}, a)
            elif c == "4":
                cid = self._prompt_int("Capability ID", 0)
                p   = self._prompt_int("New permissions", 7)
                a   = self._prompt_account()
                self._submit("Governance", "update_capability",
                             {"capability_id": cid, "new_permissions": p}, a)
            elif c == "a":
                self._val("Cap", self._query("Governance", "Capabilities",
                          [self._prompt_int("ID", 0)]))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  13. SEMANTIC RELATIONSHIPS
    # ══════════════════════════════════════════════════════════════

    def menu_semantic(self, _direct=None):
        self._nav_stack.append('semantic')
        _opts = [
                ("1", "Create Relationship"),
                ("2", "Accept Relationship"),
                ("3", "Revoke Relationship"),
                ("4", "Update Trust Level"),
                ("5", "Request Discovery"),
                ("6", "Update Profile"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Relationship Info"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("SEMANTIC RELATIONSHIPS", _opts)
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
                self._menu("SEMANTIC RELATIONSHIPS", _opts)
                continue
            elif c == "1":
                to    = self._prompt_actor("To actor")
                rtype = self._prompt("Relationship type", "Trust")
                trust = self._prompt_int("Trust (0-100)", 50)
                bidir = self._prompt_bool("Bidirectional?")
                a     = self._prompt_account()
                self._submit("Semantic", "create_relationship",
                             {"to_actor": to, "relationship_type": rtype,
                              "trust_level": trust, "expires_at": None,
                              "bidirectional": bidir}, a)
            elif c == "2":
                rid = self._prompt_int("Relationship ID", 0)
                a   = self._prompt_account()
                self._submit("Semantic", "accept_relationship",
                             {"relationship_id": rid}, a)
            elif c == "3":
                rid = self._prompt_int("Relationship ID", 0)
                a   = self._prompt_account()
                self._submit("Semantic", "revoke_relationship",
                             {"relationship_id": rid}, a)
            elif c == "4":
                rid = self._prompt_int("Relationship ID", 0)
                t   = self._prompt_int("New trust (0-100)", 50)
                a   = self._prompt_account()
                self._submit("Semantic", "update_trust_level",
                             {"relationship_id": rid, "new_trust_level": t}, a)
            elif c == "5":
                a = self._prompt_account()
                self._submit("Semantic", "request_discovery",
                             {"criteria": {}}, a)
            elif c == "6":
                en = self._prompt_bool("Discovery enabled?")
                a  = self._prompt_account()
                self._submit("Semantic", "update_profile",
                             {"discovery_enabled": en}, a)
            elif c == "a":
                self._val("Rel", self._query("Semantic", "Relationships",
                          [self._prompt_int("ID", 0)]))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  14. BOOMERANG ROUTING
    # ══════════════════════════════════════════════════════════════

    def menu_boomerang(self, _direct=None):
        self._nav_stack.append('boomerang')
        _opts = [
                ("1", "Initiate Path"),
                ("2", "Record Hop"),
                ("3", "Extend Timeout"),
                ("4", "Fail Path"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Path Info"),
                ("b", "Active Paths"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("BOOMERANG ROUTING", _opts)
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
                self._menu("BOOMERANG ROUTING", _opts)
                continue
            elif c == "1":
                target = self._prompt_actor("Target")
                a      = self._prompt_account()
                self._submit("Boomerang", "initiate_path", {"target": target}, a)
            elif c == "2":
                pid = self._prompt_int("Path ID", 0)
                to  = self._prompt_actor("To actor")
                sig = self._prompt_h256("Signature hash")
                a   = self._prompt_account()
                self._submit("Boomerang", "record_hop",
                             {"path_id": pid, "to_actor": to, "signature_hash": sig}, a)
            elif c == "3":
                pid = self._prompt_int("Path ID", 0)
                a   = self._prompt_account()
                self._submit("Boomerang", "extend_timeout", {"path_id": pid}, a)
            elif c == "4":
                pid = self._prompt_int("Path ID", 0)
                r   = self._prompt("Reason", "Timeout")
                a   = self._prompt_account()
                self._submit("Boomerang", "fail_path",
                             {"path_id": pid, "reason": r}, a)
            elif c == "a":
                self._val("Path", self._query("Boomerang", "Paths",
                          [self._prompt_int("ID", 0)]))
            elif c == "b":
                self._val("Active", self._query("Boomerang", "ActivePaths"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  15. AUTONOMOUS BEHAVIORS
    # ══════════════════════════════════════════════════════════════

    def menu_autonomous(self, _direct=None):
        self._nav_stack.append('autonomous')
        BTYPES = ["PresencePattern","InteractionPattern","TemporalPattern",
                  "TransactionPattern","NetworkPattern"]
        _opts = [
                ("1", "Create Profile"),
                ("2", "Record Behavior"),
                ("3", "Register Pattern"),
                ("4", "Match Behavior"),
                ("5", "Classify Pattern"),
                ("6", "Update Status"),
                ("7", "Flag Actor"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Actor Profile"),
                ("b", "Pattern Count"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("AUTONOMOUS BEHAVIORS", _opts)
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
                self._menu("AUTONOMOUS BEHAVIORS", _opts)
                continue
            elif c == "1":
                actor = self._prompt_actor("Actor")
                a     = self._prompt_account()
                self._submit("Autonomous", "create_profile", {"actor": actor}, a)
            elif c == "2":
                actor = self._prompt_actor("Actor")
                bt    = self._prompt_enum("Behavior:", BTYPES)
                dh    = self._prompt_h256("Data hash")
                a     = self._prompt_account()
                self._submit("Autonomous", "record_behavior",
                             {"actor": actor, "behavior_type": bt, "data_hash": dh}, a)
            elif c == "3":
                bt = self._prompt_enum("Behavior:", BTYPES)
                sh = self._prompt_h256("Signature hash")
                cl = self._prompt("Classification", "Normal")
                a  = self._prompt_account()
                self._submit("Autonomous", "register_pattern",
                             {"behavior_type": bt, "signature_hash": sh,
                              "classification": cl}, a)
            elif c == "4":
                bid = self._prompt_int("Behavior ID", 0)
                actor = self._prompt_actor("Actor")
                pid = self._prompt_int("Pattern ID", 0)
                a   = self._prompt_account()
                self._submit("Autonomous", "match_behavior",
                             {"behavior_id": bid, "actor": actor, "pattern_id": pid}, a)
            elif c == "5":
                pid  = self._prompt_int("Pattern ID", 0)
                cl   = self._prompt("Classification", "Normal")
                conf = self._prompt_int("Confidence (0-100)", 80)
                a    = self._prompt_account()
                self._submit("Autonomous", "classify_pattern",
                             {"pattern_id": pid, "classification": cl,
                              "confidence_score": conf}, a)
            elif c == "6":
                actor  = self._prompt_actor("Actor")
                status = self._prompt("Status", "Active")
                a      = self._prompt_account()
                self._submit("Autonomous", "update_status",
                             {"actor": actor, "new_status": status}, a)
            elif c == "7":
                actor  = self._prompt_actor("Actor")
                reason = self._prompt_h256("Reason hash")
                a      = self._prompt_account()
                self._submit("Autonomous", "flag_actor",
                             {"actor": actor, "reason": reason}, a)
            elif c == "a":
                self._val("Profile", self._query("Autonomous", "ActorProfiles",
                          [self._prompt_actor("Actor")]))
            elif c == "b":
                self._val("Patterns", self._query("Autonomous", "PatternCount"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  16. OCTOPUS CLUSTERS
    # ══════════════════════════════════════════════════════════════

    def menu_octopus(self, _direct=None):
        self._nav_stack.append('octopus')
        _opts = [
                ("1",  "Create Cluster"),
                ("2",  "Register Subnode"),
                ("3",  "Activate Subnode"),
                ("4",  "Start Deactivation"),
                ("5",  "Update Cluster Throughput"),
                ("6",  "Evaluate Scaling"),
                ("7",  "Update Subnode Throughput"),
                ("8",  "Record Heartbeat"),
                ("9",  "Record Device Observation"),
                ("10", "Record Position Confirmation"),
                ("11", "Heartbeat with Device Proof"),
                ("12", "Set Fusion Weights"),
                ("─",  f"{C.DIM}── Queries ──{C.R}"),
                ("a",  "Cluster Info"),
                ("b",  "Subnode Info"),
                ("c",  "Cluster Count"),
                ("?",  "Show options"),
                ("0",  "Back"),
            ]
        if not _direct:
            self._menu("OCTOPUS CLUSTERS", _opts)
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
                self._menu("OCTOPUS CLUSTERS", _opts)
                continue
            a = self._prompt_account() if c not in ("a","b","c","0","─","?") else "alice"
            if c == "1":
                owner = self._prompt_actor("Owner")
                self._submit("Octopus", "create_cluster", {"owner": owner}, a)
            elif c == "2":
                cid = self._prompt_int("Cluster ID", 0)
                op  = self._prompt_actor("Operator")
                self._submit("Octopus", "register_subnode",
                             {"cluster_id": cid, "operator": op}, a)
            elif c == "3":
                self._submit("Octopus", "activate_subnode",
                             {"subnode_id": self._prompt_int("Subnode ID", 0)}, a)
            elif c == "4":
                self._submit("Octopus", "start_deactivation",
                             {"subnode_id": self._prompt_int("Subnode ID", 0)}, a)
            elif c == "5":
                cid = self._prompt_int("Cluster ID", 0)
                tp  = self._prompt_int("Throughput (parts per billion)", 450000000)
                self._submit("Octopus", "update_throughput",
                             {"cluster_id": cid, "throughput": tp}, a)
            elif c == "6":
                self._submit("Octopus", "evaluate_scaling",
                             {"cluster_id": self._prompt_int("Cluster ID", 0)}, a)
            elif c == "7":
                sid = self._prompt_int("Subnode ID", 0)
                tp  = self._prompt_int("Throughput (ppb)", 500000000)
                pr  = self._prompt_int("Processed", 100)
                self._submit("Octopus", "update_subnode_throughput",
                             {"subnode_id": sid, "throughput": tp, "processed": pr}, a)
            elif c == "8":
                self._submit("Octopus", "record_heartbeat",
                             {"subnode_id": self._prompt_int("Subnode ID", 0)}, a)
            elif c == "9":
                sid = self._prompt_int("Subnode ID", 0)
                dc  = self._prompt_int("Device count", 5)
                cm  = self._prompt_h256("Commitment hash")
                self._submit("Octopus", "record_device_observation",
                             {"subnode_id": sid, "device_count": dc, "commitment": cm}, a)
            elif c == "10":
                sid = self._prompt_int("Subnode ID", 0)
                x   = self._prompt_int("X", 0)
                y   = self._prompt_int("Y", 0)
                z   = self._prompt_int("Z", 0)
                self._submit("Octopus", "record_position_confirmation",
                             {"subnode_id": sid, "position_x": x,
                              "position_y": y, "position_z": z}, a)
            elif c == "11":
                sid = self._prompt_int("Subnode ID", 0)
                dc  = self._prompt_int("Device count", 5)
                cm  = self._prompt_h256("Commitment")
                self._submit("Octopus", "heartbeat_with_device_proof",
                             {"subnode_id": sid, "device_count": dc, "commitment": cm}, a)
            elif c == "12":
                hw = self._prompt_int("Heartbeat weight", 40)
                dw = self._prompt_int("Device weight", 40)
                pw = self._prompt_int("Position weight", 20)
                self._submit("Octopus", "set_fusion_weights",
                             {"heartbeat_weight": hw, "device_weight": dw,
                              "position_weight": pw}, a)
            elif c == "a":
                self._val("Cluster", self._query("Octopus", "Clusters",
                          [self._prompt_int("ID", 0)]))
            elif c == "b":
                self._val("Subnode", self._query("Octopus", "Subnodes",
                          [self._prompt_int("ID", 0)]))
            elif c == "c":
                self._val("Count", self._query("Octopus", "ClusterCount"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  17. STORAGE OPERATIONS
    # ══════════════════════════════════════════════════════════════

    def menu_storage(self, _direct=None):
        self._nav_stack.append('storage')
        _opts = [
                ("1", "Store Data"),
                ("2", "Update Data"),
                ("3", "Delete Data"),
                ("4", "Set Quota [sudo]"),
                ("5", "Finalize Epoch Storage [sudo]"),
                ("─", f"{C.DIM}── Queries ──{C.R}"),
                ("a", "Entry Count"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("ON-CHAIN STORAGE", _opts)
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
                self._menu("ON-CHAIN STORAGE", _opts)
                continue
            elif c == "1":
                e    = self._prompt_epoch()
                key  = self._prompt_h256("Data key")
                dh   = self._prompt_h256("Data hash")
                dt   = self._prompt_enum("Type:", ["Presence","Commitment","Proof","Metadata","Temporary"])
                sz   = self._prompt_int("Size (bytes)", 256)
                ret  = self._prompt("Retention", "KeepForever")
                a    = self._prompt_account()
                self._submit("Storage", "store_data",
                             {"epoch": e, "key": key, "data_hash": dh,
                              "data_type": dt, "size_bytes": sz, "retention": ret}, a)
            elif c == "2":
                e   = self._prompt_epoch()
                key = self._prompt_h256("Data key")
                dh  = self._prompt_h256("New data hash")
                sz  = self._prompt_int("New size", 256)
                a   = self._prompt_account()
                self._submit("Storage", "update_data",
                             {"epoch": e, "key": key, "new_data_hash": dh,
                              "new_size": sz}, a)
            elif c == "3":
                e   = self._prompt_epoch()
                key = self._prompt_h256("Data key")
                a   = self._prompt_account()
                self._submit("Storage", "delete_data", {"epoch": e, "key": key}, a)
            elif c == "4":
                actor = self._prompt_actor("Actor")
                me    = self._prompt_int("Max entries", 100)
                mb    = self._prompt_int("Max bytes", 1000000)
                self._submit("Storage", "set_quota",
                             {"actor": actor, "max_entries": me, "max_bytes": mb}, sudo=True)
            elif c == "5":
                self._submit("Storage", "finalize_epoch",
                             {"epoch": self._prompt_epoch()}, sudo=True)
            elif c == "a":
                self._val("Entries", self._query("Storage", "EntryCount"))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  19. BLOCK EXPLORER
    # ══════════════════════════════════════════════════════════════

    def menu_block_explorer(self, _direct=None):
        self._nav_stack.append('blocks')
        _opts = [
                ("1", "Get block by number"),
                ("2", "Get block by hash"),
                ("3", "Latest block detail"),
                ("4", "Decode extrinsic in block"),
                ("5", "Block events"),
                ("6", "Finalized head"),
                ("7", "Compare blocks"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("BLOCK EXPLORER", _opts)
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
                self._menu("BLOCK EXPLORER", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                if c == "1":
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
                elif c == "2":
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
                elif c == "3":
                    header = self.substrate.get_block_header()['header']
                    bh = self.substrate.get_block_hash()
                    self._val("Number", header['number'])
                    self._val("Hash", bh)
                    self._val("Parent", header['parentHash'])
                    self._val("State Root", header['stateRoot'])
                    self._val("Extrinsics Root", header['extrinsicsRoot'])
                    if 'digest' in header and 'logs' in header['digest']:
                        for i, log in enumerate(header['digest']['logs']):
                            print(f"    {C.DIM}Digest[{i}]: {str(log)[:80]}{C.R}")
                elif c == "4":
                    num = self._prompt_int("Block number (0=latest)", 0)
                    bh = self.substrate.get_block_hash(num) if num > 0 else self.substrate.get_block_hash()
                    block = self.substrate.get_block(block_hash=bh)
                    exts = block.get('extrinsics', [])
                    if not exts:
                        self._info("No extrinsics in this block")
                    else:
                        rows = []
                        for i, ext in enumerate(exts):
                            call = ext.value if hasattr(ext, 'value') else ext
                            call_data = call.get('call', {}) if isinstance(call, dict) else {}
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
                                self._val("Function", call_data.get('call_function', '?'))
                                args = call_data.get('call_args', [])
                                if args:
                                    print(f"  {C.CY}Arguments:{C.R}")
                                    for arg in args:
                                        name = arg.get('name', '?')
                                        value = arg.get('value', '?')
                                        print(f"    {C.DIM}{name}:{C.R} {C.W}{value}{C.R}")
                            else:
                                print(f"    {C.DIM}{val}{C.R}")
                elif c == "5":
                    num = self._prompt_int("Block number (0=latest)", 0)
                    bh = self.substrate.get_block_hash(num) if num > 0 else self.substrate.get_block_hash()
                    events = self.substrate.query("System", "Events", block_hash=bh)
                    if events and events.value:
                        for i, ev in enumerate(events.value):
                            mid = ev.get('event', {}).get('module_id', '?')
                            eid = ev.get('event', {}).get('event_id', '?')
                            attrs = ev.get('event', {}).get('attributes', '')
                            attr_str = f" {C.DIM}{str(attrs)[:60]}{C.R}" if attrs else ""
                            print(f"    {C.DIM}[{i:>3}]{C.R} {C.W}{mid}.{eid}{C.R}{attr_str}")
                    else:
                        self._info("No events at this block")
                elif c == "6":
                    fh = self.substrate.rpc_request("chain_getFinalizedHead", [])['result']
                    self._val("Finalized Hash", fh)
                    header = self.substrate.get_block_header(block_hash=fh)
                    self._val("Finalized Block", header['header']['number'])
                elif c == "7":
                    n1 = self._prompt_int("Block number A", 1)
                    n2 = self._prompt_int("Block number B", 2)
                    h1 = self.substrate.get_block_hash(n1)
                    h2 = self.substrate.get_block_hash(n2)
                    b1 = self.substrate.get_block(block_hash=h1)
                    b2 = self.substrate.get_block(block_hash=h2)
                    hdr1, hdr2 = b1['header'], b2['header']
                    print(f"\n  {C.W}{'Field':>20}  {'Block '+str(n1):>30}  {'Block '+str(n2):>30}{C.R}")
                    print(f"  {C.DIM}{'─'*84}{C.R}")
                    for field in ['stateRoot', 'extrinsicsRoot', 'parentHash']:
                        v1 = str(hdr1.get(field, ''))[:28]
                        v2 = str(hdr2.get(field, ''))[:28]
                        diff = " *" if hdr1.get(field) != hdr2.get(field) else ""
                        print(f"  {C.CY}{field:>20}{C.R}  {v1:>30}  {v2:>30}{C.Y}{diff}{C.R}")
                    ext1 = len(b1.get('extrinsics', []))
                    ext2 = len(b2.get('extrinsics', []))
                    diff = " *" if ext1 != ext2 else ""
                    print(f"  {C.CY}{'extrinsicCount':>20}{C.R}  {ext1:>30}  {ext2:>30}{C.Y}{diff}{C.R}")
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  20. STORAGE INSPECTOR
    # ══════════════════════════════════════════════════════════════

    def menu_storage_inspector(self, _direct=None):
        self._nav_stack.append('inspect')
        _opts = [
                ("1", "Query storage by pallet + item"),
                ("2", "Raw storage key lookup"),
                ("3", "Enumerate keys by prefix"),
                ("4", "Storage size"),
                ("5", "Storage diff between blocks"),
                ("6", "Storage proof (Merkle)"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("STORAGE INSPECTOR", _opts)
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
                self._menu("STORAGE INSPECTOR", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                if c == "1":
                    md = self.substrate.get_metadata()
                    pallets = [p.name for p in md.pallets if p.storage]
                    print(f"  {C.DIM}Pallets with storage:{C.R}")
                    for i, name in enumerate(pallets):
                        print(f"    {C.Y}{i+1:>3}{C.R} {name}")
                    idx = self._prompt_int("Pallet #", 1) - 1
                    if 0 <= idx < len(pallets):
                        pallet_name = pallets[idx]
                        pallet = [p for p in md.pallets if p.name == pallet_name][0]
                        items = [s.name for s in pallet.storage]
                        print(f"  {C.DIM}Storage items in {pallet_name}:{C.R}")
                        for i, name in enumerate(items):
                            print(f"    {C.Y}{i+1:>3}{C.R} {name}")
                        sidx = self._prompt_int("Item #", 1) - 1
                        if 0 <= sidx < len(items):
                            item_name = items[sidx]
                            params_str = self._prompt("Parameters (comma-separated, or empty)", "")
                            params = [p.strip() for p in params_str.split(",") if p.strip()] if params_str else []
                            converted = []
                            for p in params:
                                try:
                                    converted.append(int(p))
                                except ValueError:
                                    converted.append(p)
                            result = self.substrate.query(pallet_name, item_name, converted or None)
                            self._val(f"{pallet_name}.{item_name}", result)
                elif c == "2":
                    key = self._prompt("Storage key (hex)", "")
                    if key:
                        result = self.substrate.rpc_request("state_getStorage", [key])
                        raw = result.get('result')
                        self._val("Raw value", raw if raw else "(empty)")
                elif c == "3":
                    prefix = self._prompt("Hex prefix or pallet name", "")
                    if prefix and not prefix.startswith("0x"):
                        try:
                            import xxhash
                            h = xxhash.xxh64(prefix.encode(), seed=0).hexdigest()
                            h += xxhash.xxh64(prefix.encode(), seed=1).hexdigest()
                            prefix = "0x" + h
                            self._info(f"Prefix: {prefix}")
                        except ImportError:
                            self._err("xxhash not available for pallet name conversion")
                    count = self._prompt_int("Max keys", 20)
                    result = self.substrate.rpc_request("state_getKeysPaged", [prefix, count, prefix])
                    keys = result.get('result', [])
                    self._val("Keys found", len(keys))
                    for i, k in enumerate(keys[:count]):
                        print(f"    {C.DIM}[{i}]{C.R} {k}")
                elif c == "4":
                    key = self._prompt("Storage key (hex)", "")
                    if key:
                        result = self.substrate.rpc_request("state_getStorageSize", [key])
                        size = result.get('result')
                        self._val("Size (bytes)", size if size is not None else "key not found")
                elif c == "5":
                    key = self._prompt("Storage key (hex)", "")
                    n1 = self._prompt_int("Block number A", 1)
                    n2 = self._prompt_int("Block number B (0=latest)", 0)
                    h1 = self.substrate.get_block_hash(n1)
                    h2 = self.substrate.get_block_hash(n2) if n2 > 0 else self.substrate.get_block_hash()
                    r1 = self.substrate.rpc_request("state_getStorage", [key, h1]).get('result')
                    r2 = self.substrate.rpc_request("state_getStorage", [key, h2]).get('result')
                    self._val(f"Block {n1}", r1 if r1 else "(empty)")
                    n2_label = n2 if n2 > 0 else "latest"
                    self._val(f"Block {n2_label}", r2 if r2 else "(empty)")
                    if r1 == r2:
                        self._info("Values are identical")
                    else:
                        print(f"  {C.Y}Values differ{C.R}")
                elif c == "6":
                    key = self._prompt("Storage key (hex)", "")
                    if key:
                        bh = self.substrate.get_block_hash()
                        result = self.substrate.rpc_request("state_getReadProof", [[key], bh])
                        proof = result.get('result', {})
                        self._val("At block", proof.get('at', '?'))
                        nodes = proof.get('proof', [])
                        self._val("Proof nodes", len(nodes))
                        for i, node in enumerate(nodes[:10]):
                            print(f"    {C.DIM}[{i}] {node[:80]}...{C.R}")
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  21. RUNTIME INSPECTOR
    # ══════════════════════════════════════════════════════════════

    def menu_runtime_inspector(self, _direct=None):
        self._nav_stack.append('runtime')
        _opts = [
                ("1", "List all pallets"),
                ("2", "Pallet detail"),
                ("3", "Runtime version"),
                ("4", "Search call by name"),
                ("5", "Search storage by name"),
                ("6", "Search error by name"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("RUNTIME INSPECTOR", _opts)
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
                self._menu("RUNTIME INSPECTOR", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                md = self.substrate.get_metadata()
                if c == "1":
                    print(f"\n  {C.W}{'Pallet':>20}  {'Calls':>6}  {'Storage':>8}  {'Events':>7}  {'Errors':>7}  {'Consts':>7}{C.R}")
                    print(f"  {C.DIM}{'─'*62}{C.R}")
                    for p in md.pallets:
                        nc = len(p.calls) if p.calls else 0
                        ns = len(p.storage) if p.storage else 0
                        ne = len(p.events) if p.events else 0
                        nerr = len(p.errors) if p.errors else 0
                        nconst = len(p.constants) if p.constants else 0
                        print(f"  {C.B}{p.name:>20}{C.R}  {nc:>6}  {ns:>8}  {ne:>7}  {nerr:>7}  {nconst:>7}")
                elif c == "2":
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
                                    args = ", ".join(f"{a.name}: {a.type}" for a in call.args)
                                print(f"    {C.G}{call.name}{C.R}({C.DIM}{args}{C.R})")
                        if p.storage:
                            print(f"\n  {C.W}Storage ({len(p.storage)}):{C.R}")
                            for s in p.storage:
                                stype = str(s.type) if hasattr(s, 'type') else '?'
                                print(f"    {C.CY}{s.name}{C.R} {C.DIM}{stype[:60]}{C.R}")
                        if p.events:
                            print(f"\n  {C.W}Events ({len(p.events)}):{C.R}")
                            for ev in p.events:
                                print(f"    {C.Y}{ev.name}{C.R}")
                        if p.errors:
                            print(f"\n  {C.W}Errors ({len(p.errors)}):{C.R}")
                            for err in p.errors:
                                doc = ""
                                if hasattr(err, 'docs') and err.docs:
                                    doc = f" {C.DIM}— {' '.join(err.docs)[:60]}{C.R}"
                                print(f"    {C.RED}{err.name}{C.R}{doc}")
                        if p.constants:
                            print(f"\n  {C.W}Constants ({len(p.constants)}):{C.R}")
                            for const in p.constants:
                                val = const.value if hasattr(const, 'value') else '?'
                                print(f"    {C.CY}{const.name}{C.R} = {C.W}{val}{C.R}")
                elif c == "3":
                    rv = self.substrate.rpc_request("state_getRuntimeVersion", [])['result']
                    for k in ['specName', 'specVersion', 'implVersion', 'authoringVersion',
                              'transactionVersion', 'stateVersion']:
                        self._val(k, rv.get(k, 'n/a'))
                elif c == "4":
                    q = self._prompt("Call name (substring)", "").lower()
                    if q:
                        found = 0
                        for p in md.pallets:
                            if p.calls:
                                for call in p.calls:
                                    if q in call.name.lower():
                                        args = ""
                                        if hasattr(call, 'args') and call.args:
                                            args = ", ".join(f"{a.name}: {a.type}" for a in call.args)
                                        print(f"    {C.B}{p.name}{C.R}.{C.G}{call.name}{C.R}({C.DIM}{args}{C.R})")
                                        found += 1
                        self._info(f"{found} calls matched '{q}'")
                elif c == "5":
                    q = self._prompt("Storage name (substring)", "").lower()
                    if q:
                        found = 0
                        for p in md.pallets:
                            if p.storage:
                                for s in p.storage:
                                    if q in s.name.lower():
                                        stype = str(s.type) if hasattr(s, 'type') else '?'
                                        print(f"    {C.B}{p.name}{C.R}.{C.CY}{s.name}{C.R} {C.DIM}{stype[:50]}{C.R}")
                                        found += 1
                        self._info(f"{found} storage items matched '{q}'")
                elif c == "6":
                    q = self._prompt("Error name (substring)", "").lower()
                    if q:
                        found = 0
                        for p in md.pallets:
                            if p.errors:
                                for err in p.errors:
                                    if q in err.name.lower():
                                        doc = ""
                                        if hasattr(err, 'docs') and err.docs:
                                            doc = f" — {' '.join(err.docs)[:60]}"
                                        print(f"    {C.B}{p.name}{C.R}.{C.RED}{err.name}{C.R}{C.DIM}{doc}{C.R}")
                                        found += 1
                        self._info(f"{found} errors matched '{q}'")
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  22. NETWORK & PEERS
    # ══════════════════════════════════════════════════════════════

    def menu_network(self, _direct=None):
        self._nav_stack.append('network')
        _opts = [
                ("1", "Connected peers"),
                ("2", "Node identity"),
                ("3", "Sync state"),
                ("4", "Node health"),
                ("5", "Node roles"),
                ("6", "Chain type"),
                ("7", "Pending extrinsics"),
                ("8", "Add/Remove reserved peer"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("NETWORK & PEERS", _opts)
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
                self._menu("NETWORK & PEERS", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                if c == "1":
                    peers = self.substrate.rpc_request("system_peers", [])['result']
                    if not peers:
                        self._info("No connected peers (single-node devnet)")
                    else:
                        print(f"\n  {C.W}{'Peer ID':>20}  {'Best #':>8}  {'Roles':>10}{C.R}")
                        print(f"  {C.DIM}{'─'*44}{C.R}")
                        for p in peers:
                            pid = p.get('peerId', '?')[:16]
                            best = p.get('bestNumber', '?')
                            roles = p.get('roles', '?')
                            print(f"  {C.DIM}{pid}...{C.R}  {best:>8}  {roles:>10}")
                        self._val("Total peers", len(peers))
                elif c == "2":
                    pid = self.substrate.rpc_request("system_localPeerId", [])['result']
                    addrs = self.substrate.rpc_request("system_localListenAddresses", [])['result']
                    self._val("Peer ID", pid)
                    self._val("Listen Addresses", len(addrs))
                    for addr in addrs:
                        print(f"    {C.DIM}{addr}{C.R}")
                elif c == "3":
                    state = self.substrate.rpc_request("system_syncState", [])['result']
                    self._val("Starting Block", state.get('startingBlock', '?'))
                    self._val("Current Block", state.get('currentBlock', '?'))
                    self._val("Highest Block", state.get('highestBlock', '?'))
                elif c == "4":
                    h = self.substrate.rpc_request("system_health", [])['result']
                    self._val("Peers", h.get('peers', 0))
                    self._val("Is Syncing", h.get('isSyncing', False))
                    self._val("Should Have Peers", h.get('shouldHavePeers', False))
                elif c == "5":
                    roles = self.substrate.rpc_request("system_nodeRoles", [])['result']
                    self._val("Node Roles", roles)
                elif c == "6":
                    chain_type = self.substrate.rpc_request("system_chainType", [])['result']
                    self._val("Chain Type", chain_type)
                elif c == "7":
                    pending = self.substrate.rpc_request("author_pendingExtrinsics", [])['result']
                    self._val("Pending Count", len(pending))
                    for i, ext in enumerate(pending[:10]):
                        print(f"    {C.DIM}[{i}] {str(ext)[:80]}{C.R}")
                elif c == "8":
                    action = self._prompt_enum("Action:", ["Add reserved peer", "Remove reserved peer"])
                    addr = self._prompt("Multiaddr", "")
                    if addr:
                        if "Add" in action:
                            r = self.substrate.rpc_request("system_addReservedPeer", [addr])
                        else:
                            r = self.substrate.rpc_request("system_removeReservedPeer", [addr])
                        self._ok(f"Result: {r.get('result', r)}")
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  23. CRYPTO TOOLBOX
    # ══════════════════════════════════════════════════════════════

    def menu_crypto(self, _direct=None):
        self._nav_stack.append('crypto')
        _opts = [
                ("1",  "Generate keypair"),
                ("2",  "Derive from URI"),
                ("3",  "SS58 encode/decode"),
                ("4",  "Blake2b-256 hash"),
                ("5",  "Keccak-256 hash"),
                ("6",  "TwoX128 hash"),
                ("7",  "Build storage key"),
                ("8",  "SCALE encode"),
                ("9",  "SCALE decode"),
                ("10", "Sign message"),
                ("11", "Verify signature"),
                ("12", "Random H256"),
                ("?",  "Show options"),
                ("0",  "Back"),
            ]
        if not _direct:
            self._menu("CRYPTO TOOLBOX", _opts)
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
                self._menu("CRYPTO TOOLBOX", _opts)
                continue
            try:
                if c == "1":
                    scheme = self._prompt_enum("Scheme:", ["sr25519", "ed25519"])
                    mnemonic = Keypair.generate_mnemonic()
                    crypto = 1 if scheme == "sr25519" else 2
                    kp = Keypair.create_from_mnemonic(mnemonic, crypto_type=crypto)
                    self._val("Mnemonic", mnemonic)
                    self._val("Public Key", f"0x{kp.public_key.hex()}")
                    self._val("SS58 Address", kp.ss58_address)
                elif c == "2":
                    uri = self._prompt("URI (e.g. //Alice or mnemonic)", "//Alice")
                    try:
                        kp = Keypair.create_from_uri(uri)
                    except Exception:
                        kp = Keypair.create_from_mnemonic(uri)
                    self._val("Public Key", f"0x{kp.public_key.hex()}")
                    self._val("SS58 Address", kp.ss58_address)
                    self._val("AccountId", f"0x{kp.public_key.hex()}")
                elif c == "3":
                    direction = self._prompt_enum("Direction:", ["Hex to SS58", "SS58 to Hex"])
                    if "SS58 to" in direction:
                        ss58 = self._prompt("SS58 address", "")
                        if ss58:
                            kp = Keypair(ss58_address=ss58)
                            self._val("Public Key (hex)", f"0x{kp.public_key.hex()}")
                    else:
                        hex_key = self._prompt("Public key (hex)", "")
                        prefix = self._prompt_int("SS58 prefix", 42)
                        if hex_key:
                            if hex_key.startswith("0x"):
                                hex_key = hex_key[2:]
                            kp = Keypair(public_key=bytes.fromhex(hex_key), ss58_format=prefix)
                            self._val("SS58 Address", kp.ss58_address)
                elif c == "4":
                    data = self._prompt("Input (hex or text)", "")
                    if data:
                        raw = bytes.fromhex(data[2:]) if data.startswith("0x") else data.encode()
                        digest = hashlib.blake2b(raw, digest_size=32).hexdigest()
                        self._val("Blake2b-256", f"0x{digest}")
                elif c == "5":
                    data = self._prompt("Input (hex or text)", "")
                    if data:
                        raw = bytes.fromhex(data[2:]) if data.startswith("0x") else data.encode()
                        try:
                            from Crypto.Hash import keccak as _keccak
                            kh = _keccak.new(digest_bits=256, data=raw)
                            self._val("Keccak-256", f"0x{kh.hexdigest()}")
                        except ImportError:
                            digest = hashlib.sha3_256(raw).hexdigest()
                            self._val("SHA3-256", f"0x{digest}")
                            self._info("Note: install pycryptodome for true Keccak-256")
                elif c == "6":
                    data = self._prompt("Input string", "")
                    if data:
                        try:
                            import xxhash
                            h0 = xxhash.xxh64(data.encode(), seed=0).hexdigest()
                            h1 = xxhash.xxh64(data.encode(), seed=1).hexdigest()
                            self._val("TwoX128", f"0x{h0}{h1}")
                        except ImportError:
                            self._err("xxhash not installed (pip install xxhash)")
                elif c == "7":
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
                elif c == "8":
                    if not self._ensure():
                        self._pause(); continue
                    type_str = self._prompt("SCALE type (e.g. u32, AccountId, Vec<u8>)", "u32")
                    value = self._prompt("Value", "42")
                    try:
                        val = int(value) if value.isdigit() else value
                    except Exception:
                        val = value
                    try:
                        obj = self.substrate.runtime_config.create_scale_object(type_str)
                        obj.encode(val)
                        self._val("Encoded", f"0x{obj.data.to_hex()}")
                    except Exception as e:
                        self._err(f"SCALE encode: {e}")
                elif c == "9":
                    if not self._ensure():
                        self._pause(); continue
                    type_str = self._prompt("SCALE type (e.g. u32, AccountId)", "u32")
                    hex_data = self._prompt("Hex data", "0x2a000000")
                    try:
                        from scalecodec import ScaleBytes
                        obj = self.substrate.runtime_config.create_scale_object(type_str)
                        obj.decode(ScaleBytes(hex_data))
                        self._val("Decoded", obj.value)
                    except Exception as e:
                        self._err(f"SCALE decode: {e}")
                elif c == "10":
                    uri = self._prompt("Keypair URI (e.g. //Alice)", "//Alice")
                    message = self._prompt("Message", "hello")
                    kp = Keypair.create_from_uri(uri)
                    raw = message.encode() if not message.startswith("0x") else bytes.fromhex(message[2:])
                    sig = kp.sign(raw)
                    self._val("Signature", f"0x{sig.hex()}")
                    self._val("Signer", kp.ss58_address)
                elif c == "11":
                    pub = self._prompt("Public key (hex or SS58)", "")
                    message = self._prompt("Message", "hello")
                    sig_hex = self._prompt("Signature (hex)", "")
                    try:
                        if pub.startswith("0x"):
                            kp = Keypair(public_key=bytes.fromhex(pub[2:]))
                        else:
                            kp = Keypair(ss58_address=pub)
                        raw = message.encode() if not message.startswith("0x") else bytes.fromhex(message[2:])
                        sig = bytes.fromhex(sig_hex[2:]) if sig_hex.startswith("0x") else bytes.fromhex(sig_hex)
                        valid = kp.verify(raw, sig)
                        if valid:
                            self._ok("Signature is VALID")
                        else:
                            self._err("Signature is INVALID")
                    except Exception as e:
                        self._err(f"Verify failed: {e}")
                elif c == "12":
                    h = "0x" + secrets.token_hex(32)
                    self._val("Random H256", h)
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  24. ACCOUNT INSPECTOR
    # ══════════════════════════════════════════════════════════════

    def menu_account_inspector(self, _direct=None):
        self._nav_stack.append('accounts')
        _opts = [
                ("1", "Full account info"),
                ("2", "Account nonce"),
                ("3", "All balances"),
                ("4", "Fee estimation"),
                ("5", "Dry run extrinsic"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("ACCOUNT INSPECTOR", _opts)
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
                self._menu("ACCOUNT INSPECTOR", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                if c == "1":
                    name = self._prompt_account("Account")
                    kp = self.keypairs[name]
                    r = self.substrate.query('System', 'Account', [kp.ss58_address])
                    if r and r.value:
                        self._val("Nonce", r.value.get('nonce', 0))
                        data = r.value.get('data', {})
                        self._val("Free", f"{data.get('free', 0) / 1e12:.6f} UNIT")
                        self._val("Reserved", f"{data.get('reserved', 0) / 1e12:.6f} UNIT")
                        self._val("Frozen", f"{data.get('frozen', 0) / 1e12:.6f} UNIT")
                        self._val("Flags", data.get('flags', 0))
                    else:
                        self._info("Account not found or empty")
                elif c == "2":
                    name = self._prompt_account("Account")
                    kp = self.keypairs[name]
                    r = self.substrate.rpc_request("system_accountNextIndex", [kp.ss58_address])
                    self._val("Next Nonce", r.get('result', '?'))
                elif c == "3":
                    rows = []
                    for name, kp in self.keypairs.items():
                        r = self.substrate.query('System', 'Account', [kp.ss58_address])
                        if r and r.value:
                            data = r.value.get('data', {})
                            free = data.get('free', 0)
                            reserved = data.get('reserved', 0)
                            total = free + reserved
                            rows.append([name, f"{free/1e12:.4f}", f"{reserved/1e12:.4f}", f"{total/1e12:.4f}"])
                        else:
                            rows.append([name, "—", "—", "—"])
                    self._table(["Account", "Free", "Reserved", "Total"], rows)
                elif c == "4":
                    mod = self._prompt("Call module", "Presence")
                    fn = self._prompt("Call function", "declare_presence")
                    params_str = self._prompt("Params JSON (or empty for {})", "{}")
                    try:
                        params = json.loads(params_str) if params_str else {}
                    except json.JSONDecodeError:
                        params = {}
                    name = self._prompt_account("Signer")
                    kp = self.keypairs[name]
                    call = self.substrate.compose_call(mod, fn, params)
                    ext = self.substrate.create_signed_extrinsic(call=call, keypair=kp)
                    info = self.substrate.rpc_request("payment_queryInfo", [ext.value])
                    result = info.get('result', {})
                    self._val("Weight", result.get('weight', '?'))
                    self._val("Partial Fee", result.get('partialFee', '?'))
                    self._val("Class", result.get('class', '?'))
                elif c == "5":
                    mod = self._prompt("Call module", "Presence")
                    fn = self._prompt("Call function", "declare_presence")
                    params_str = self._prompt("Params JSON (or empty for {})", "{}")
                    try:
                        params = json.loads(params_str) if params_str else {}
                    except json.JSONDecodeError:
                        params = {}
                    name = self._prompt_account("Signer")
                    kp = self.keypairs[name]
                    call = self.substrate.compose_call(mod, fn, params)
                    ext = self.substrate.create_signed_extrinsic(call=call, keypair=kp)
                    result = self.substrate.rpc_request("system_dryRun", [ext.value])
                    dry = result.get('result', '?')
                    if isinstance(dry, str) and 'Ok' in dry:
                        self._ok(f"Dry run: {dry}")
                    else:
                        self._err(f"Dry run: {dry}")
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  25. EVENT DECODER
    # ══════════════════════════════════════════════════════════════

    def menu_events(self, _direct=None):
        self._nav_stack.append('events')
        _opts = [
                ("1", "Events at latest block"),
                ("2", "Events at block N"),
                ("3", "Filter by pallet"),
                ("4", "Event history (last N blocks)"),
                ("5", "List all event types"),
                ("?", "Show options"),
                ("0", "Back"),
            ]
        if not _direct:
            self._menu("EVENT DECODER", _opts)
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
                self._menu("EVENT DECODER", _opts)
                continue
            if not self._ensure():
                self._pause(); continue
            try:
                if c == "1":
                    events = self.substrate.query("System", "Events")
                    if events and events.value:
                        for i, ev in enumerate(events.value):
                            mid = ev.get('event', {}).get('module_id', '?')
                            eid = ev.get('event', {}).get('event_id', '?')
                            attrs = ev.get('event', {}).get('attributes', '')
                            attr_str = f" {C.DIM}{str(attrs)[:60]}{C.R}" if attrs else ""
                            print(f"    {C.DIM}[{i:>3}]{C.R} {C.W}{mid}.{eid}{C.R}{attr_str}")
                        self._val("Total events", len(events.value))
                    else:
                        self._info("No events at latest block")
                elif c == "2":
                    num = self._prompt_int("Block number", 1)
                    bh = self.substrate.get_block_hash(num)
                    events = self.substrate.query("System", "Events", block_hash=bh)
                    if events and events.value:
                        for i, ev in enumerate(events.value):
                            mid = ev.get('event', {}).get('module_id', '?')
                            eid = ev.get('event', {}).get('event_id', '?')
                            attrs = ev.get('event', {}).get('attributes', '')
                            attr_str = f" {C.DIM}{str(attrs)[:60]}{C.R}" if attrs else ""
                            print(f"    {C.DIM}[{i:>3}]{C.R} {C.W}{mid}.{eid}{C.R}{attr_str}")
                        self._val("Total events", len(events.value))
                    else:
                        self._info(f"No events at block {num}")
                elif c == "3":
                    pallet = self._prompt("Pallet name", "Presence")
                    events = self.substrate.query("System", "Events")
                    if events and events.value:
                        filtered = [ev for ev in events.value
                                    if ev.get('event', {}).get('module_id', '') == pallet]
                        if filtered:
                            for i, ev in enumerate(filtered):
                                eid = ev.get('event', {}).get('event_id', '?')
                                attrs = ev.get('event', {}).get('attributes', '')
                                attr_str = f" {C.DIM}{str(attrs)[:60]}{C.R}" if attrs else ""
                                print(f"    {C.DIM}[{i:>3}]{C.R} {C.W}{pallet}.{eid}{C.R}{attr_str}")
                            self._val(f"{pallet} events", len(filtered))
                        else:
                            self._info(f"No {pallet} events at latest block")
                    else:
                        self._info("No events at latest block")
                elif c == "4":
                    pallet = self._prompt("Pallet name (or empty for all)", "")
                    n = self._prompt_int("Last N blocks", 5)
                    header = self.substrate.get_block_header()['header']
                    current = header['number']
                    total = 0
                    for blk in range(max(1, current - n + 1), current + 1):
                        bh = self.substrate.get_block_hash(blk)
                        events = self.substrate.query("System", "Events", block_hash=bh)
                        if events and events.value:
                            evts = events.value
                            if pallet:
                                evts = [ev for ev in evts
                                        if ev.get('event', {}).get('module_id', '') == pallet]
                            if evts:
                                print(f"  {C.B}Block {blk}{C.R}")
                                for ev in evts:
                                    mid = ev.get('event', {}).get('module_id', '?')
                                    eid = ev.get('event', {}).get('event_id', '?')
                                    print(f"    {C.DIM}{mid}.{eid}{C.R}")
                                total += len(evts)
                    self._val("Total events found", total)
                elif c == "5":
                    md = self.substrate.get_metadata()
                    rows = []
                    for p in md.pallets:
                        if p.events:
                            for ev in p.events:
                                fields = ""
                                if hasattr(ev, 'args') and ev.args:
                                    fields = ", ".join(str(a) for a in ev.args)
                                elif hasattr(ev, 'value') and isinstance(ev.value, dict):
                                    fields = ", ".join(ev.value.get('args', []))
                                rows.append([p.name, ev.name, fields])
                    self._table(["Pallet", "Event", "Fields"], rows)
                    self._val("Total event types", len(rows))
            except Exception as e:
                self._err(str(e))
            self._pause()

    # ══════════════════════════════════════════════════════════════
    #  AUTOMATED TEST: FULL PoP LIFECYCLE
    # ══════════════════════════════════════════════════════════════

    def test_full_lifecycle(self):
        """Full Proof-of-Presence lifecycle: declare → vote → finalize.

        Uses a fresh epoch each run so the test is idempotent — no
        DuplicatePresence / DuplicateVote / PresenceImmutable errors.
        """
        if not self._ensure(): return
        self._check_epoch()
        self._header("FULL PoP LIFECYCLE TEST")
        epoch = self._next_test_epoch()

        # 1. Validators already active from bootstrap
        self._info(f"Step 1: Using epoch {epoch} (validators active from bootstrap)")

        # 2. Declare presence
        self._info("Step 2: Eve declares presence")
        self._submit("Presence", "declare_presence", {"epoch": epoch}, "eve")

        # 3. Validators vote
        eve_id = self._actor_id('eve')
        self._info("Step 3: Validators vote on Eve (3 of 6 → quorum)")
        for voter in ['alice', 'bob', 'charlie']:
            self._submit("Presence", "vote_presence",
                         {"actor": eve_id, "epoch": epoch, "approve": True}, voter)

        # 4. Check vote count
        vc = self._query("Presence", "VoteCount", [epoch, eve_id])
        self._val("Eve votes", vc)

        # 5. Finalize
        self._info("Step 4: Finalize Eve's presence")
        self._submit("Presence", "finalize_presence",
                     {"actor": eve_id, "epoch": epoch}, "alice")

        # 6. Verify on-chain state
        r = self._query("Presence", "Presences", [epoch, eve_id])
        self._val("Final state", r)
        self._ok("Full lifecycle test complete!")
        self._pause()

    # ══════════════════════════════════════════════════════════════
    #  AUTOMATED TEST: COMMIT-REVEAL
    # ══════════════════════════════════════════════════════════════

    def test_commit_reveal(self):
        """Commit-reveal test: commitment = blake2b(secret ‖ randomness).

        Uses a fresh epoch each run so re-runs don't hit DuplicatePresence.
        Secret and randomness are passed as 0x-prefixed hex for SCALE encoding.
        """
        if not self._ensure(): return
        self._check_epoch()
        self._header("COMMIT-REVEAL TEST")
        epoch = self._next_test_epoch()

        # Generate 32-byte secret and 32-byte randomness
        sec = secrets.token_hex(32)
        rnd = secrets.token_hex(32)
        # Commitment = blake2b-256(secret ‖ randomness)
        h = hashlib.blake2b(bytes.fromhex(sec + rnd), digest_size=32).hexdigest()

        self._info(f"Committing (hash: 0x{h[:16]}...)")
        self._submit("Presence", "declare_presence_with_commitment",
                     {"epoch": epoch, "commitment": "0x" + h}, "ferdie")

        self._val("Commitments", self._query("Presence", "CommitmentCount", [epoch]))

        self._info("Revealing...")
        self._submit("Presence", "reveal_commitment",
                     {"epoch": epoch, "secret": "0x" + sec, "randomness": "0x" + rnd},
                     "ferdie")

        self._val("Reveals", self._query("Presence", "RevealCount", [epoch]))
        self._ok("Commit-reveal test complete!")
        self._pause()

    # ══════════════════════════════════════════════════════════════
    #  MAIN MENU
    # ══════════════════════════════════════════════════════════════

    # ══════════════════════════════════════════════════════════════
    #  COMPACT MENU & HELP
    # ══════════════════════════════════════════════════════════════

    def _show_compact_menu(self):
        print(f"""
  {C.BB}COMMANDS{C.R}  {C.DIM}type command or number, Tab to complete{C.R}

  {C.B}CORE{C.R}                    {C.B}POSITIONING{C.R}             {C.B}SECURITY{C.R}
  {C.Y} 2{C.R} presence    {C.DIM}p{C.R}       {C.Y} 5{C.R} pbt                 {C.Y} 7{C.R} dispute     {C.DIM}dis{C.R}
  {C.Y} 3{C.R} epoch       {C.DIM}e{C.R}       {C.Y} 6{C.R} triangulation       {C.Y} 8{C.R} zk
  {C.Y} 4{C.R} validator   {C.DIM}val{C.R}                             {C.Y} 9{C.R} vault

  {C.B}IDENTITY{C.R}                {C.B}INTELLIGENCE{C.R}            {C.B}DEV TOOLS{C.R}
  {C.Y}10{C.R} device      {C.DIM}dev{C.R}     {C.Y}13{C.R} semantic    {C.DIM}sem{C.R}    {C.Y}19{C.R} blocks      {C.DIM}blk{C.R}
  {C.Y}11{C.R} lifecycle   {C.DIM}life{C.R}    {C.Y}14{C.R} boomerang   {C.DIM}boom{C.R}   {C.Y}20{C.R} inspect     {C.DIM}si{C.R}
  {C.Y}12{C.R} governance  {C.DIM}gov{C.R}     {C.Y}15{C.R} autonomous  {C.DIM}auto{C.R}   {C.Y}21{C.R} runtime     {C.DIM}rt{C.R}
                          {C.Y}16{C.R} octopus     {C.DIM}oct{C.R}    {C.Y}22{C.R} network     {C.DIM}net{C.R}
                          {C.Y}17{C.R} storage     {C.DIM}store{C.R}  {C.Y}23{C.R} crypto      {C.DIM}cr{C.R}
                                                {C.Y}24{C.R} accounts    {C.DIM}acct{C.R}
  {C.B}TESTS{C.R}                   {C.B}STATUS{C.R}                  {C.Y}25{C.R} events      {C.DIM}ev{C.R}
  {C.Y}t1{C.R} test pop            {C.Y}18{C.R} chain status
  {C.Y}t2{C.R} test pbt
  {C.Y}t3{C.R} test commit

  {C.DIM}Other: status  use epoch/account  bootstrap (b)  connect (1)  help  ?  exit{C.R}
""")

    def _cmd_help(self, args=None):
        if not args:
            print(f"""
  {C.BB}LAUD CLI{C.R}  {C.DIM}PoP Protocol Testing Suite{C.R}

  {C.W}Navigation{C.R}
    menu              Show all commands with numbers
    <command>         Enter submenu (e.g. 'presence' or '2')
    <cmd> <action>    Direct action (e.g. 'presence declare' or 'p d')
    back              Return to parent menu
    0                 Back / exit current submenu

  {C.W}Context{C.R}
    use epoch <N>     Set default epoch for all commands
    use <name>        Set default account (alice, bob, ...)
    use clear         Reset to defaults
    status            Show chain / epoch / account status

  {C.W}Quick Actions{C.R}
    b / bootstrap     Bootstrap devnet (epoch + validators + positions)
    t1 / test pop     Full PoP lifecycle test
    t2 / test pbt     PBT triangulation test
    t3 / test commit  Commit-reveal test
    1 / connect       Connect to node

  {C.W}Tips{C.R}
    Tab               Autocomplete commands
    Up/Down           Command history
    Ctrl+C            Cancel / back to root
    ?                 Quick start guide (inside submenu: show options)

  {C.DIM}Type 'help <topic>' for details, e.g. 'help presence', 'help pbt'{C.R}
""")
            return
        topic = args[0].lower()
        topic_map = {
            'p': 'presence', '2': 'presence',
            'e': 'epoch', '3': 'epoch',
            'val': 'validator', '4': 'validator',
            '5': 'pbt',
            'tri': 'triangulation', '6': 'triangulation',
            'dis': 'dispute', '7': 'dispute',
            '8': 'zk',
            '9': 'vault',
            'dev': 'device', '10': 'device',
            'life': 'lifecycle', '11': 'lifecycle',
            'gov': 'governance', '12': 'governance',
            'sem': 'semantic', '13': 'semantic',
            'boom': 'boomerang', '14': 'boomerang',
            'auto': 'autonomous', '15': 'autonomous',
            'oct': 'octopus', '16': 'octopus',
            'store': 'storage', '17': 'storage',
            'blk': 'blocks', '19': 'blocks',
            'si': 'inspect', '20': 'inspect',
            'rt': 'runtime', '21': 'runtime',
            'net': 'network', '22': 'network',
            'cr': 'crypto', '23': 'crypto',
            'acct': 'accounts', '24': 'accounts',
            'ev': 'events', '25': 'events',
        }
        topic = topic_map.get(topic, topic)
        help_data = {
            'presence': ('Presence Protocol', [
                ('declare', '1/d', 'Declare presence for an epoch'),
                ('commit',  '2/cm', 'Declare with commitment hash'),
                ('reveal',  '3/rv', 'Reveal a commitment'),
                ('vote',    '4/v', 'Vote on an actor\'s presence'),
                ('finalize','5/f', 'Finalize presence after quorum'),
                ('slash',   '6', 'Slash presence [sudo]'),
                ('quorum',  '7', 'Set quorum config [sudo]'),
            ]),
            'epoch': ('Epoch Management', [
                ('schedule', '1', 'Schedule a new epoch [sudo]'),
                ('start',    '2', 'Start an epoch [sudo]'),
                ('close',    '3', 'Close an epoch [sudo]'),
                ('finalize', '4', 'Finalize an epoch [sudo]'),
                ('register', '5', 'Register participant'),
            ]),
            'validator': ('Validator Operations', [
                ('register',   '1', 'Register with stake'),
                ('activate',   '2', 'Activate validator'),
                ('deactivate', '3', 'Deactivate validator'),
                ('withdraw',   '4', 'Withdraw stake'),
                ('stake',      '5', 'Increase stake'),
                ('slash',      '6', 'Slash validator [sudo]'),
            ]),
            'pbt': ('Position-Based Triangulation', [
                ('position', '1', 'Set validator position'),
                ('claim',    '2', 'Claim a position'),
                ('attest',   '3', 'Submit witness attestation'),
                ('verify',   '4', 'Verify position via triangulation'),
                ('setup',    '5', 'Auto-setup 6 validators'),
                ('test',     '6', 'Full PBT test flow'),
            ]),
        }
        if topic in help_data:
            title, cmds = help_data[topic]
            print(f"\n  {C.BB}{title}{C.R}  {C.DIM}({topic}){C.R}\n")
            for name, alias, desc in cmds:
                print(f"    {C.Y}{name:16}{C.R} {C.DIM}({alias}){C.R}  {desc}")
            print(f"\n  {C.DIM}Usage: {topic} <action>  or type '{topic}' for interactive menu{C.R}\n")
        else:
            self._err(f"No help for '{topic}'. Type 'help' for general help.")

    def show_guide(self):
        self._header("QUICK START GUIDE")
        print(f"""  {C.W}GLOSSARY{C.R}
  {C.DIM}  Epoch     = time period for presence proofs
    Validator = node that votes on presence claims
    Actor     = identity identified by blake2b(pubkey)
    PBT       = position-based triangulation{C.R}

  {C.W}1. Start the devnet{C.R}  {C.DIM}(instant-seal: blocks only on your txns){C.R}
     {C.Y}cd devnet && ./scripts/dev.sh{C.R}
     {C.DIM}Or multi-node Aura:  docker compose up -d --build{C.R}

  {C.W}2. Connect + bootstrap{C.R}
     {C.DIM}CLI auto-connects on start. Type {C.Y}bootstrap{C.DIM} or {C.Y}b{C.DIM}:
     activates epoch 1, registers 6 validators, sets positions.{C.R}

  {C.W}3. Run automated tests{C.R}  {C.DIM}(type bootstrap first){C.R}
     {C.Y}t1{C.R}  {C.DIM}Full PoP lifecycle    {C.Y}test pop{C.R}
     {C.Y}t2{C.R}  {C.DIM}PBT flow             {C.Y}test pbt{C.R}
     {C.Y}t3{C.R}  {C.DIM}Commit-reveal        {C.Y}test commit{C.R}

  {C.W}4. Set context{C.R}  {C.DIM}(avoid re-typing epoch/account){C.R}
     {C.Y}use epoch 5{C.R}   {C.DIM}all subsequent commands use epoch 5{C.R}
     {C.Y}use bob{C.R}       {C.DIM}all subsequent commands sign as bob{C.R}
     {C.Y}use clear{C.R}     {C.DIM}reset to defaults{C.R}

  {C.W}5. Direct commands{C.R}  {C.DIM}(skip menus){C.R}
     {C.Y}presence declare{C.R}   {C.DIM}or{C.R}  {C.Y}p d{C.R}
     {C.Y}presence vote{C.R}      {C.DIM}or{C.R}  {C.Y}p v{C.R}
     {C.Y}pbt test{C.R}           {C.DIM}full PBT test flow{C.R}

  {C.W}6. Inspect results{C.R}
     {C.Y}events{C.R} {C.DIM}or{C.R} {C.Y}25{C.R}   {C.DIM}Event Decoder{C.R}
     {C.Y}blocks{C.R} {C.DIM}or{C.R} {C.Y}19{C.R}   {C.DIM}Block Explorer{C.R}
     {C.Y}status{C.R}           {C.DIM}Quick chain status{C.R}

  {C.W}7. Accounts{C.R}
     {C.DIM}alice {C.Y}(sudo){C.DIM}, bob, charlie, dave, eve, ferdie
     All pre-funded with 10M UNIT on devnet{C.R}
""")

    # ══════════════════════════════════════════════════════════════
    #  COMMAND DISPATCH
    # ══════════════════════════════════════════════════════════════

    # Maps aliases/numbers to canonical menu handler names
    _MENU_ALIASES = {
        '1': '_cmd_connect', 'connect': '_cmd_connect', 'reconnect': '_cmd_connect',
        'b': 'bootstrap', 'boot': 'bootstrap', 'bootstrap': 'bootstrap',
        '2': 'menu_presence', 'presence': 'menu_presence', 'p': 'menu_presence',
        '3': 'menu_epoch', 'epoch': 'menu_epoch', 'e': 'menu_epoch',
        '4': 'menu_validator', 'validator': 'menu_validator', 'val': 'menu_validator',
        '5': 'menu_pbt', 'pbt': 'menu_pbt',
        '6': 'menu_triangulation', 'triangulation': 'menu_triangulation', 'tri': 'menu_triangulation',
        '7': 'menu_dispute', 'dispute': 'menu_dispute', 'dis': 'menu_dispute',
        '8': 'menu_zk', 'zk': 'menu_zk',
        '9': 'menu_vault', 'vault': 'menu_vault',
        '10': 'menu_device', 'device': 'menu_device', 'dev': 'menu_device',
        '11': 'menu_lifecycle', 'lifecycle': 'menu_lifecycle', 'life': 'menu_lifecycle',
        '12': 'menu_governance', 'governance': 'menu_governance', 'gov': 'menu_governance',
        '13': 'menu_semantic', 'semantic': 'menu_semantic', 'sem': 'menu_semantic',
        '14': 'menu_boomerang', 'boomerang': 'menu_boomerang', 'boom': 'menu_boomerang',
        '15': 'menu_autonomous', 'autonomous': 'menu_autonomous', 'auto': 'menu_autonomous',
        '16': 'menu_octopus', 'octopus': 'menu_octopus', 'oct': 'menu_octopus',
        '17': 'menu_storage', 'storage': 'menu_storage', 'store': 'menu_storage',
        '18': 'menu_chain', 'chain': 'menu_chain',
        '19': 'menu_block_explorer', 'blocks': 'menu_block_explorer', 'blk': 'menu_block_explorer',
        '20': 'menu_storage_inspector', 'inspect': 'menu_storage_inspector', 'si': 'menu_storage_inspector',
        '21': 'menu_runtime_inspector', 'runtime': 'menu_runtime_inspector', 'rt': 'menu_runtime_inspector',
        '22': 'menu_network', 'network': 'menu_network', 'net': 'menu_network',
        '23': 'menu_crypto', 'crypto': 'menu_crypto', 'cr': 'menu_crypto',
        '24': 'menu_account_inspector', 'accounts': 'menu_account_inspector', 'acct': 'menu_account_inspector',
        '25': 'menu_events', 'events': 'menu_events', 'ev': 'menu_events',
        't1': 'test_full_lifecycle', 't2': '_auto_pbt_test', 't3': 'test_commit_reveal',
        '?': 'show_guide',
    }

    # Maps parent command → { sub-alias → submenu-number }
    _SUB_ALIASES = {
        'menu_presence': {
            'declare': '1', 'd': '1', 'commit': '2', 'cm': '2', 'reveal': '3', 'rv': '3',
            'vote': '4', 'v': '4', 'finalize': '5', 'f': '5', 'slash': '6',
            'quorum': '7', 'validator-status': '8', 'epoch-active': '9',
        },
        'menu_epoch': {
            'schedule': '1', 'start': '2', 'close': '3', 'finalize': '4',
            'register': '5', 'update': '6', 'force': '7',
        },
        'menu_validator': {
            'register': '1', 'activate': '2', 'deactivate': '3',
            'withdraw': '4', 'stake': '5', 'slash': '6',
        },
        'menu_pbt': {
            'position': '1', 'claim': '2', 'attest': '3', 'verify': '4',
            'setup': '5', 'test': '6',
        },
    }

    def _cmd_connect(self):
        url = self._prompt("URL", self.url)
        self.connect(url)

    def _build_prompt(self):
        path = "/".join(["laud"] + self._nav_stack)
        extras = []
        if self._ctx_account != 'alice':
            extras.append(f"{C.Y}{self._ctx_account}{C.R}")
        if self._ctx_epoch is not None:
            extras.append(f"{C.DIM}epoch:{self._ctx_epoch}{C.R}")
        extra = " " + " ".join(extras) if extras else ""
        return f"  {C.B}{path}{C.R}{extra} > "

    def _dispatch(self, line):
        """Route user input to the right handler."""
        parts = line.strip().split()
        if not parts:
            return
        cmd = parts[0].lower()

        # Special commands
        if cmd in ('exit', 'quit', '0'):
            raise SystemExit
        if cmd == 'help' or cmd == 'h':
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
        if cmd == 'back':
            if self._nav_stack:
                self._nav_stack.pop()
            return

        # Test aliases: "test pop", "test pbt", "test commit"
        if cmd == 'test' and len(parts) > 1:
            sub = parts[1].lower()
            test_map = {'pop': 'test_full_lifecycle', '1': 'test_full_lifecycle',
                        'pbt': '_auto_pbt_test', '2': '_auto_pbt_test',
                        'commit': 'test_commit_reveal', '3': 'test_commit_reveal'}
            handler_name = test_map.get(sub)
            if handler_name:
                getattr(self, handler_name)()
                return

        # Two-word commands: "presence declare", "pbt test", etc.
        handler_name = self._MENU_ALIASES.get(cmd)
        if handler_name and len(parts) > 1:
            sub_map = self._SUB_ALIASES.get(handler_name, {})
            sub_alias = parts[1].lower()
            sub_num = sub_map.get(sub_alias)
            if sub_num:
                # Call the menu method with a pre-set choice
                handler = getattr(self, handler_name, None)
                if handler:
                    handler(_direct=sub_num)
                    return

        # Single-word command / number
        if handler_name:
            handler = getattr(self, handler_name, None)
            if handler:
                handler()
                return

        self._err(f"Unknown: '{line}'. Type 'help' or 'menu'.")

    # ══════════════════════════════════════════════════════════════
    #  RUN
    # ══════════════════════════════════════════════════════════════

    def _print_welcome(self):
        print(f"""
  {C.BB}LAUD NETWORKS{C.R}  {C.DIM}PoP Protocol Testing Suite v1.0.0{C.R}
  {C.DIM}Type{C.R} help {C.DIM}for commands,{C.R} menu {C.DIM}for full menu,{C.R} ? {C.DIM}for guide{C.R}
""")

    def run(self):
        self._print_welcome()
        self._setup_readline()

        if not SUBSTRATE_OK:
            print(f"  {C.RED}substrate-interface not found.{C.R}")
            print(f"  Run: {C.Y}pip install substrate-interface{C.R}")
            print(f"  Or:  {C.Y}source .venv/bin/activate{C.R}\n")
        else:
            self.connect(self.url)
            if not self.connected:
                print(f"  {C.DIM}Tip: run ./scripts/dev.sh then type 'connect'{C.R}\n")

        while True:
            try:
                line = input(self._build_prompt()).strip()
                if not line:
                    continue
                self._dispatch(line)
            except SystemExit:
                print(f"\n  {C.DIM}LAUD NETWORKS{C.R}\n")
                break
            except KeyboardInterrupt:
                print()
                if self._nav_stack:
                    self._nav_stack.clear()
                    continue
                print(f"  {C.DIM}(Ctrl+C again or type 'exit' to quit){C.R}")
            except EOFError:
                print(f"\n  {C.DIM}LAUD NETWORKS{C.R}\n")
                break


# ═══════════════════════════════════════════════════════════════════
#  Entry Point
# ═══════════════════════════════════════════════════════════════════

if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description="LAUD NETWORKS - PoP Protocol Testing Suite")
    parser.add_argument('--url', default='ws://127.0.0.1:9944',
                        help='WebSocket endpoint (default: ws://127.0.0.1:9944)')
    args = parser.parse_args()

    cli = LaudCLI(url=args.url)
    cli.run()
