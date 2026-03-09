"""
Tests for presence-gated vault access.
Tests the position distance helper, presence binding detection,
and the access control logic (without requiring a live chain).
"""

import os
import sys

import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))


# ── Position distance tests ─────────────────────────────────────

class TestPositionDistance:
    """Test _position_distance_sq static method."""

    @staticmethod
    def _dist_sq(a, b):
        """Inline distance calculation matching the CLI helper."""
        dx = a.get('x', 0) - b.get('x', 0)
        dy = a.get('y', 0) - b.get('y', 0)
        dz = a.get('z', 0) - b.get('z', 0)
        return dx * dx + dy * dy + dz * dz

    def test_same_position(self):
        p = {'x': 10, 'y': 20, 'z': 30}
        assert self._dist_sq(p, p) == 0

    def test_unit_distance(self):
        a = {'x': 0, 'y': 0, 'z': 0}
        b = {'x': 1, 'y': 0, 'z': 0}
        assert self._dist_sq(a, b) == 1

    def test_3d_distance(self):
        a = {'x': 1, 'y': 2, 'z': 3}
        b = {'x': 4, 'y': 6, 'z': 3}
        # dx=3, dy=4, dz=0 => 9+16+0=25
        assert self._dist_sq(a, b) == 25

    def test_negative_coordinates(self):
        a = {'x': -5, 'y': -5, 'z': 0}
        b = {'x': 5, 'y': 5, 'z': 0}
        # dx=10, dy=10 => 100+100=200
        assert self._dist_sq(a, b) == 200

    def test_missing_keys_default_zero(self):
        a = {'x': 3}
        b = {'y': 4}
        # dx=3, dy=-4, dz=0 => 9+16=25
        assert self._dist_sq(a, b) == 25


# ── Presence binding detection ──────────────────────────────────

class TestPresenceBinding:
    """Test presence_bound field detection in file entries."""

    def test_unbound_file(self):
        entry = {'enc_hash': 'abc', 'presence_bound': False}
        assert not entry.get('presence_bound', False)

    def test_bound_file(self):
        entry = {
            'enc_hash': 'abc',
            'presence_bound': True,
            'bound_position': {'x': 10, 'y': 20, 'z': 0},
            'position_tolerance': 50,
        }
        assert entry.get('presence_bound', False)
        assert entry['bound_position']['x'] == 10
        assert entry['position_tolerance'] == 50

    def test_missing_presence_bound_defaults_false(self):
        entry = {'enc_hash': 'abc'}
        assert not entry.get('presence_bound', False)


# ── Access control logic ────────────────────────────────────────

class TestAccessControl:
    """Test the access gate logic without a live chain.

    Reproduces the logic from _check_presence_for_file inline.
    """

    @staticmethod
    def _check_access(file_entry, current_pos, verified=True):
        """Simplified check mirroring CLI logic."""
        if not file_entry.get('presence_bound', False):
            return True

        bound_pos = file_entry.get('bound_position')
        tolerance = file_entry.get('position_tolerance', 100)
        if not bound_pos:
            return True

        if current_pos is None or not verified:
            return False

        dx = bound_pos.get('x', 0) - current_pos.get('x', 0)
        dy = bound_pos.get('y', 0) - current_pos.get('y', 0)
        dz = bound_pos.get('z', 0) - current_pos.get('z', 0)
        dist_sq = dx * dx + dy * dy + dz * dz
        tol_sq = tolerance * tolerance

        return dist_sq <= tol_sq

    def test_unbound_always_passes(self):
        entry = {'presence_bound': False}
        assert self._check_access(entry, None, verified=False)

    def test_bound_no_position_denied(self):
        entry = {
            'presence_bound': True,
            'bound_position': {'x': 10, 'y': 20, 'z': 0},
            'position_tolerance': 50,
        }
        assert not self._check_access(entry, None)

    def test_bound_unverified_denied(self):
        entry = {
            'presence_bound': True,
            'bound_position': {'x': 10, 'y': 20, 'z': 0},
            'position_tolerance': 50,
        }
        pos = {'x': 10, 'y': 20, 'z': 0}
        assert not self._check_access(entry, pos, verified=False)

    def test_bound_matching_position_passes(self):
        entry = {
            'presence_bound': True,
            'bound_position': {'x': 100, 'y': 200, 'z': 0},
            'position_tolerance': 50,
        }
        pos = {'x': 100, 'y': 200, 'z': 0}
        assert self._check_access(entry, pos)

    def test_bound_within_tolerance_passes(self):
        entry = {
            'presence_bound': True,
            'bound_position': {'x': 100, 'y': 200, 'z': 0},
            'position_tolerance': 50,
        }
        # Move 30 units in x, 40 in y => dist=50, just at edge
        pos = {'x': 130, 'y': 240, 'z': 0}
        assert self._check_access(entry, pos)

    def test_bound_outside_tolerance_denied(self):
        entry = {
            'presence_bound': True,
            'bound_position': {'x': 100, 'y': 200, 'z': 0},
            'position_tolerance': 50,
        }
        # Move 51 units => clearly out
        pos = {'x': 151, 'y': 200, 'z': 0}
        assert not self._check_access(entry, pos)

    def test_bound_3d_tolerance(self):
        entry = {
            'presence_bound': True,
            'bound_position': {'x': 0, 'y': 0, 'z': 0},
            'position_tolerance': 10,
        }
        # 3D distance: sqrt(6^2 + 6^2 + 6^2) = sqrt(108) ≈ 10.39
        pos = {'x': 6, 'y': 6, 'z': 6}
        assert not self._check_access(entry, pos)

        # sqrt(5^2 + 5^2 + 5^2) = sqrt(75) ≈ 8.66
        pos2 = {'x': 5, 'y': 5, 'z': 5}
        assert self._check_access(entry, pos2)


# ── Index integration ───────────────────────────────────────────

class TestIndexPresenceFields:
    """Verify add_to_index stores presence fields correctly."""

    def test_unbound_file_index(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path)

        key = laud_files.add_to_index(
            vault_id=0, enc_hash='abc', plaintext_hash='def',
            original_name='test.txt', size_bytes=100,
            uploader='alice', epoch=1,
            key_fingerprint_hex='ff' * 32,
            threshold=2, ring_size=3,
        )
        index = laud_files.load_index()
        entry = index[key]
        assert entry['presence_bound'] is False
        assert 'bound_position' not in entry

    def test_bound_file_index(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path)

        pos = {'x': 42, 'y': 84, 'z': 0}
        key = laud_files.add_to_index(
            vault_id=0, enc_hash='xyz', plaintext_hash='uvw',
            original_name='secret.pdf', size_bytes=500,
            uploader='bob', epoch=2,
            key_fingerprint_hex='aa' * 32,
            threshold=2, ring_size=3,
            bound_position=pos, position_tolerance=75,
        )
        index = laud_files.load_index()
        entry = index[key]
        assert entry['presence_bound'] is True
        assert entry['bound_position'] == pos
        assert entry['position_tolerance'] == 75
