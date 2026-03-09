"""
Tests for laud_registry.py — Domain/command lookup, alias resolution,
mode filtering, and structural integrity.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from laud_registry import DOMAINS, Domain, Command, Param


# ── Domain structure ─────────────────────────────────────────────

class TestDomainStructure:
    def test_all_domains_have_names(self):
        for d in DOMAINS:
            assert d.name, f"Domain missing name: {d}"

    def test_all_domains_have_titles(self):
        for d in DOMAINS:
            assert d.title, f"Domain {d.name} missing title"

    def test_all_domains_have_numbers(self):
        for d in DOMAINS:
            assert d.number, f"Domain {d.name} missing number"

    def test_unique_domain_names(self):
        names = [d.name for d in DOMAINS]
        assert len(names) == len(set(names)), \
            f"Duplicate domain names: {names}"

    def test_unique_domain_numbers(self):
        numbers = [d.number for d in DOMAINS]
        assert len(numbers) == len(set(numbers)), \
            f"Duplicate domain numbers: {numbers}"

    def test_valid_modes(self):
        valid = {"normal", "dev", "both"}
        for d in DOMAINS:
            assert d.mode in valid, \
                f"Domain {d.name} has invalid mode: {d.mode}"

    def test_valid_groups(self):
        valid = {"core", "security", "identity", "devtools",
                 "getting-started", "positioning", "intelligence",
                 "status", ""}
        for d in DOMAINS:
            assert d.group in valid, \
                f"Domain {d.name} has invalid group: {d.group}"


# ── Command structure ────────────────────────────────────────────

class TestCommandStructure:
    def test_all_commands_have_keys(self):
        for d in DOMAINS:
            for c in d.commands:
                assert c.key, f"Command missing key in {d.name}"

    def test_all_commands_have_labels(self):
        for d in DOMAINS:
            for c in d.commands:
                assert c.label, f"Command {c.key} missing label in {d.name}"

    def test_all_commands_have_valid_action(self):
        valid_actions = {"submit", "query", "custom", "separator",
                         "query_map"}
        for d in DOMAINS:
            for c in d.commands:
                assert c.action in valid_actions, \
                    f"Command {c.key} in {d.name} has invalid " \
                    f"action: {c.action}"

    def test_submit_commands_have_pallet(self):
        for d in DOMAINS:
            for c in d.commands:
                if c.action == "submit":
                    assert c.pallet, \
                        f"Submit command {c.key} in {d.name} missing pallet"
                    assert c.function, \
                        f"Submit command {c.key} in {d.name} missing function"

    def test_query_commands_have_pallet(self):
        for d in DOMAINS:
            for c in d.commands:
                if c.action == "query":
                    assert c.pallet, \
                        f"Query command {c.key} in {d.name} missing pallet"
                    assert c.function, \
                        f"Query command {c.key} in {d.name} missing function"

    def test_custom_commands_have_handler(self):
        for d in DOMAINS:
            for c in d.commands:
                if c.action == "custom":
                    assert c.custom_handler, \
                        f"Custom command {c.key} in {d.name} " \
                        f"missing handler"

    def test_command_modes_valid(self):
        valid = {"normal", "dev", "both"}
        for d in DOMAINS:
            for c in d.commands:
                assert c.mode in valid, \
                    f"Command {c.key} in {d.name} has " \
                    f"invalid mode: {c.mode}"


# ── Unique keys within domains ───────────────────────────────────

class TestUniqueKeys:
    def test_unique_command_keys_per_domain(self):
        for d in DOMAINS:
            keys = [c.key for c in d.commands
                    if c.action != "separator"]
            assert len(keys) == len(set(keys)), \
                f"Duplicate keys in {d.name}: {keys}"


# ── Alias resolution ────────────────────────────────────────────

class TestAliases:
    def test_no_alias_conflicts_within_domain(self):
        for d in DOMAINS:
            all_identifiers = set()
            for c in d.commands:
                if c.action == "separator":
                    continue
                all_identifiers.add(c.key)
                for alias in c.aliases:
                    assert alias not in all_identifiers, \
                        f"Alias '{alias}' conflicts in {d.name}"
                    all_identifiers.add(alias)

    def test_aliases_are_lowercase(self):
        for d in DOMAINS:
            for c in d.commands:
                for alias in c.aliases:
                    assert alias == alias.lower(), \
                        f"Alias '{alias}' not lowercase in " \
                        f"{d.name}/{c.key}"


# ── Domain lookup helpers ────────────────────────────────────────

class TestDomainLookup:
    def test_find_by_name(self):
        found = [d for d in DOMAINS if d.name == "presence"]
        assert len(found) == 1
        assert found[0].title == "PRESENCE PROTOCOL"

    def test_find_by_number(self):
        found = [d for d in DOMAINS if d.number == "2"]
        assert len(found) == 1
        assert found[0].name == "presence"

    def test_find_by_shortcut(self):
        found = [d for d in DOMAINS
                 if d.shortcut and d.shortcut == "p"]
        assert len(found) == 1
        assert found[0].name == "presence"


# ── Mode filtering ───────────────────────────────────────────────

class TestModeFiltering:
    def test_normal_mode_domains(self):
        normal = [d for d in DOMAINS
                  if d.mode in ("normal", "both")]
        assert len(normal) > 0
        names = [d.name for d in normal]
        assert "presence" in names

    def test_dev_only_domains(self):
        dev_only = [d for d in DOMAINS if d.mode == "dev"]
        names = [d.name for d in dev_only]
        assert "zk" in names or "lifecycle" in names

    def test_normal_titles_set(self):
        # Dashboard uses its own title directly, no normal_title needed
        skip = {"dashboard"}
        for d in DOMAINS:
            if d.mode in ("normal", "both") and d.name not in skip:
                assert d.normal_title, \
                    f"Domain {d.name} visible in normal mode " \
                    f"but missing normal_title"


# ── Specific domain checks ──────────────────────────────────────

class TestCoreDomains:
    def test_presence_domain_exists(self):
        d = next(d for d in DOMAINS if d.name == "presence")
        assert d.check_epoch is True
        keys = [c.key for c in d.commands
                if c.action != "separator"]
        assert "1" in keys

    def test_epoch_domain_exists(self):
        d = next(d for d in DOMAINS if d.name == "epoch")
        assert d.number == "3"

    def test_validator_domain_exists(self):
        d = next(d for d in DOMAINS if d.name == "validator")
        assert d.number == "4"

    def test_vault_domain_exists(self):
        d = next(d for d in DOMAINS if d.name == "vault")
        keys = [c.key for c in d.commands
                if c.action != "separator"]
        assert "1" in keys
        assert "r" in keys
        assert "u" in keys
        assert "v" in keys
        # New vault UX commands
        for k in ("15", "16", "17", "c", "d", "e", "f"):
            assert k in keys, f"New vault key '{k}' missing"

    def test_zk_domain_has_new_commands(self):
        d = next(d for d in DOMAINS if d.name == "zk")
        keys = [c.key for c in d.commands
                if c.action != "separator"]
        assert "8" in keys
        assert "9" in keys
        assert "10" in keys
        assert "11" in keys
        assert "b" in keys
        assert "c" in keys
        assert "d" in keys


# ── Normal mode labels ───────────────────────────────────────────

class TestNormalLabels:
    def test_presence_has_normal_labels(self):
        d = next(d for d in DOMAINS if d.name == "presence")
        labeled = [c for c in d.commands if c.normal_label]
        assert len(labeled) >= 3
        labels = [c.normal_label for c in labeled]
        assert "Check In Now" in labels

    def test_vault_has_normal_labels(self):
        d = next(d for d in DOMAINS if d.name == "vault")
        labeled = [c for c in d.commands if c.normal_label]
        assert len(labeled) >= 3

    def test_vault_new_commands_have_normal_labels(self):
        d = next(d for d in DOMAINS if d.name == "vault")
        new_keys = {"15", "16", "17", "c", "d", "e", "f"}
        cmds = {c.key: c for c in d.commands
                if c.key in new_keys}
        for k in new_keys:
            assert k in cmds, f"New vault key '{k}' missing"
            assert cmds[k].normal_label, \
                f"Vault key '{k}' missing normal_label"
            assert "_" not in cmds[k].normal_label, \
                f"Vault key '{k}' normal_label not friendly: " \
                f"{cmds[k].normal_label}"

    def test_normal_titles_friendly(self):
        d = next(d for d in DOMAINS if d.name == "presence")
        assert d.normal_title == "CHECK-IN"
        d = next(d for d in DOMAINS if d.name == "vault")
        assert d.normal_title == "DOCUMENT SAFE"
        d = next(d for d in DOMAINS if d.name == "dispute")
        assert d.normal_title == "CHALLENGES"


# ── New vault UX commands ────────────────────────────────────────

class TestVaultNewCommands:
    """Validates the 8 new vault UX commands added in Phase 1."""

    EXPECTED = {
        "15": "_vault_browse",
        "16": "_vault_health",
        "17": "_vault_approvals",
        "c":  "_vault_my_vaults",
        "d":  "_vault_enter",
        "e":  "_vault_share_status",
        "f":  "_vault_unlock_requests",
    }

    @staticmethod
    def _vault():
        return next(d for d in DOMAINS if d.name == "vault")

    def test_all_new_keys_exist(self):
        keys = [c.key for c in self._vault().commands
                if c.action != "separator"]
        for k in self.EXPECTED:
            assert k in keys, f"Key '{k}' not in vault domain"

    def test_all_new_commands_are_custom(self):
        cmds = {c.key: c for c in self._vault().commands}
        for k in self.EXPECTED:
            assert cmds[k].action == "custom", \
                f"Key '{k}' should be custom, got {cmds[k].action}"

    def test_correct_handlers(self):
        cmds = {c.key: c for c in self._vault().commands}
        for k, handler in self.EXPECTED.items():
            assert cmds[k].custom_handler == handler, \
                f"Key '{k}' handler: expected {handler}, " \
                f"got {cmds[k].custom_handler}"

    def test_all_mode_both(self):
        cmds = {c.key: c for c in self._vault().commands}
        for k in self.EXPECTED:
            assert cmds[k].mode == "both", \
                f"Key '{k}' mode should be 'both', " \
                f"got {cmds[k].mode}"

    def test_separators_present(self):
        seps = [c.label for c in self._vault().commands
                if c.action == "separator"]
        assert "Browse & Status" in seps, \
            "Missing 'Browse & Status' separator"
        assert "Vault Detail" in seps, \
            "Missing 'Vault Detail' separator"

    def test_no_key_d_prefix_collision(self):
        """Verify 'd' doesn't collide with 'd1', 'd2', 'd3', 'd4'."""
        cmds = {c.key: c for c in self._vault().commands
                if c.action != "separator"}
        # 'd' and 'd1'-'d4' are distinct string keys
        assert "d" in cmds
        assert "d1" in cmds
        assert cmds["d"].custom_handler != cmds["d1"].custom_handler


# ── Param structure ──────────────────────────────────────────────

class TestParams:
    def test_params_have_names(self):
        for d in DOMAINS:
            for c in d.commands:
                for p in c.params:
                    assert p.name, \
                        f"Param missing name in {d.name}/{c.key}"

    def test_params_have_labels(self):
        for d in DOMAINS:
            for c in d.commands:
                for p in c.params:
                    assert p.label, \
                        f"Param {p.name} missing label in " \
                        f"{d.name}/{c.key}"

    def test_param_kinds_valid(self):
        valid_kinds = {"str", "int", "h256", "actor", "self_actor",
                       "epoch", "enum", "bool", "hex", "bytes",
                       "float", "account", "position"}
        for d in DOMAINS:
            for c in d.commands:
                for p in c.params:
                    assert p.kind in valid_kinds, \
                        f"Param {p.name} in {d.name}/{c.key} " \
                        f"has invalid kind: {p.kind}"

    def test_enum_params_have_options(self):
        for d in DOMAINS:
            for c in d.commands:
                for p in c.params:
                    if p.kind == "enum":
                        assert p.options, \
                            f"Enum param {p.name} in " \
                            f"{d.name}/{c.key} missing options"
