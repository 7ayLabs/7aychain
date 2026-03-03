"""
Tests for per-file share storage and flat-to-namespaced migration.
"""

import os
import shutil
import sys
import tempfile

import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import laud_files


@pytest.fixture(autouse=True)
def temp_share_dirs(tmp_path, monkeypatch):
    """Override share dirs to temp directories for each test."""
    shares_dir = tmp_path / 'vault_shares'
    monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', shares_dir)
    return shares_dir


class TestPerFileShares:
    """Validate per-file (enc_hash) namespaced share storage."""

    def test_store_with_enc_hash(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'abc123'
        path = laud_files.store_share(vault_id, 1, b'\x01' * 32,
                                      enc_hash=enc_hash)
        assert path.exists()
        assert enc_hash in str(path)
        assert path.name == 'share_1.bin'

    def test_store_flat_without_enc_hash(self, temp_share_dirs):
        vault_id = 0
        path = laud_files.store_share(vault_id, 1, b'\x01' * 32)
        assert path.exists()
        expected = temp_share_dirs / '0' / 'share_1.bin'
        assert path == expected

    def test_load_namespaced(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_a'
        data = b'\xab' * 32
        laud_files.store_share(vault_id, 2, data, enc_hash=enc_hash)
        result = laud_files.load_share(vault_id, 2, enc_hash=enc_hash)
        assert result is not None
        idx, value = result
        assert idx == 2
        assert value == data

    def test_load_falls_back_to_flat(self, temp_share_dirs):
        vault_id = 0
        data = b'\xcd' * 32
        laud_files.store_share(vault_id, 3, data)
        # Load with an enc_hash that has no namespaced shares
        result = laud_files.load_share(vault_id, 3,
                                       enc_hash='nonexistent')
        assert result is not None
        _, value = result
        assert value == data

    def test_load_all_namespaced(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_b'
        laud_files.store_share(vault_id, 1, b'\x01' * 32,
                               enc_hash=enc_hash)
        laud_files.store_share(vault_id, 2, b'\x02' * 32,
                               enc_hash=enc_hash)
        shares = laud_files.load_all_shares(vault_id,
                                            enc_hash=enc_hash)
        assert len(shares) == 2
        indices = [s[0] for s in shares]
        assert 1 in indices
        assert 2 in indices


class TestTwoFilesIndependentShares:
    """Two files in the same vault have independent share sets."""

    def test_independent_share_sets(self, temp_share_dirs):
        vault_id = 0
        enc_a = 'file_alpha'
        enc_b = 'file_beta'

        data_a = b'\xaa' * 32
        data_b = b'\xbb' * 32

        laud_files.store_share(vault_id, 1, data_a, enc_hash=enc_a)
        laud_files.store_share(vault_id, 1, data_b, enc_hash=enc_b)

        shares_a = laud_files.load_all_shares(vault_id,
                                              enc_hash=enc_a)
        shares_b = laud_files.load_all_shares(vault_id,
                                              enc_hash=enc_b)

        assert len(shares_a) == 1
        assert len(shares_b) == 1
        assert shares_a[0][1] == data_a
        assert shares_b[0][1] == data_b
        assert shares_a[0][1] != shares_b[0][1]


class TestMigration:
    """Test flat share migration to per-file namespace."""

    def test_migrate_copies_flat_to_namespaced(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_migrated'

        # Create flat shares
        laud_files.store_share(vault_id, 1, b'\x01' * 32)
        laud_files.store_share(vault_id, 2, b'\x02' * 32)

        count = laud_files.migrate_shares_to_per_file(vault_id,
                                                      enc_hash)
        assert count == 2

        # Namespaced shares should exist
        shares = laud_files.load_all_shares(vault_id,
                                            enc_hash=enc_hash)
        assert len(shares) == 2

    def test_migration_is_idempotent(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_idem'

        laud_files.store_share(vault_id, 1, b'\x01' * 32)

        first = laud_files.migrate_shares_to_per_file(vault_id,
                                                      enc_hash)
        assert first == 1

        second = laud_files.migrate_shares_to_per_file(vault_id,
                                                       enc_hash)
        assert second == 0  # already migrated

    def test_flat_shares_remain_after_migration(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_keep'

        laud_files.store_share(vault_id, 1, b'\x01' * 32)

        laud_files.migrate_shares_to_per_file(vault_id, enc_hash)

        # Flat shares should still exist (copy, not move)
        flat_path = temp_share_dirs / '0' / 'share_1.bin'
        assert flat_path.exists()

    def test_migrate_no_flat_shares(self, temp_share_dirs):
        vault_id = 99
        count = laud_files.migrate_shares_to_per_file(vault_id,
                                                      'no_shares')
        assert count == 0


class TestExportImportNamespaced:
    """Export/import with namespaced shares."""

    def test_export_namespaced(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_export'
        data = b'\x42' * 32

        laud_files.store_share(vault_id, 5, data, enc_hash=enc_hash)

        hex_str = laud_files.export_share_hex(vault_id, 5,
                                              enc_hash=enc_hash)
        assert hex_str is not None
        assert hex_str.startswith('05:')

    def test_import_roundtrip(self, temp_share_dirs):
        vault_id = 0
        enc_hash = 'file_roundtrip'
        data = b'\x77' * 32

        laud_files.store_share(vault_id, 3, data, enc_hash=enc_hash)

        hex_str = laud_files.export_share_hex(vault_id, 3,
                                              enc_hash=enc_hash)
        parsed = laud_files.import_share_hex(hex_str)
        assert parsed is not None
        idx, value = parsed
        assert idx == 3
        assert value == data
