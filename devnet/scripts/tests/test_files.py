"""
Tests for laud_files.py — AES-256-GCM encryption/decryption, index
management, share storage, and file verification.
"""

import json
import os
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from laud_files import (
    encrypt_file, decrypt_file,
    hash_file_blake2b, hash_bytes_blake2b,
    store_encrypted_file, retrieve_and_decrypt,
    store_share, load_share, load_all_shares,
    export_share_hex, import_share_hex,
    add_to_index, load_index, save_index, get_vault_files,
    anonymize_existing_index, verify_file,
    secure_zero,
    HEADER_SIZE, FILE_MAGIC, FILE_VERSION, MAX_FILE_SIZE,
)

import pytest


# ── Hashing ──────────────────────────────────────────────────────

class TestHashing:
    def test_blake2b_file_hash(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_bytes(b"hello world")
        h = hash_file_blake2b(str(f))
        assert len(h) == 64
        assert all(c in '0123456789abcdef' for c in h)

    def test_blake2b_file_deterministic(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_bytes(b"deterministic")
        h1 = hash_file_blake2b(str(f))
        h2 = hash_file_blake2b(str(f))
        assert h1 == h2

    def test_blake2b_bytes_hash(self):
        h = hash_bytes_blake2b(b"test data")
        assert len(h) == 64

    def test_blake2b_bytes_deterministic(self):
        h1 = hash_bytes_blake2b(b"data")
        h2 = hash_bytes_blake2b(b"data")
        assert h1 == h2

    def test_different_data_different_hash(self):
        h1 = hash_bytes_blake2b(b"data_a")
        h2 = hash_bytes_blake2b(b"data_b")
        assert h1 != h2


# ── AES-256-GCM encryption ──────────────────────────────────────

class TestEncryption:
    def test_encrypt_decrypt_roundtrip(self, tmp_path):
        plaintext = b"Secret document content"
        f = tmp_path / "doc.txt"
        f.write_bytes(plaintext)
        fek = os.urandom(32)

        enc_blob, enc_hash, pt_hash = encrypt_file(str(f), fek)
        recovered = decrypt_file(enc_blob, fek)
        assert recovered == plaintext

    def test_enc_blob_format(self, tmp_path):
        f = tmp_path / "doc.txt"
        f.write_bytes(b"test")
        fek = os.urandom(32)

        enc_blob, _, _ = encrypt_file(str(f), fek)
        assert enc_blob[:4] == FILE_MAGIC
        assert enc_blob[4:6] == FILE_VERSION
        assert len(enc_blob) >= HEADER_SIZE

    def test_different_key_fails(self, tmp_path):
        f = tmp_path / "doc.txt"
        f.write_bytes(b"secret")
        fek1 = os.urandom(32)
        fek2 = os.urandom(32)

        enc_blob, _, _ = encrypt_file(str(f), fek1)
        with pytest.raises(ValueError, match="Decryption failed"):
            decrypt_file(enc_blob, fek2)

    def test_tampered_ciphertext_fails(self, tmp_path):
        f = tmp_path / "doc.txt"
        f.write_bytes(b"secret data")
        fek = os.urandom(32)

        enc_blob, _, _ = encrypt_file(str(f), fek)
        tampered = bytearray(enc_blob)
        tampered[-1] ^= 0xFF
        with pytest.raises(ValueError, match="Decryption failed"):
            decrypt_file(bytes(tampered), fek)

    def test_short_blob_fails(self):
        with pytest.raises(ValueError, match="too short"):
            decrypt_file(b"\x00" * 10, os.urandom(32))

    def test_wrong_magic_fails(self):
        bad = b"XXXX" + b"\x00" * (HEADER_SIZE + 10)
        with pytest.raises(ValueError, match="Invalid magic"):
            decrypt_file(bad, os.urandom(32))

    def test_wrong_version_fails(self):
        bad = FILE_MAGIC + b"\x99\x99" + b"\x00" * (HEADER_SIZE + 10)
        with pytest.raises(ValueError, match="Unsupported version"):
            decrypt_file(bad, os.urandom(32))

    def test_file_not_found(self):
        with pytest.raises(FileNotFoundError):
            encrypt_file("/nonexistent/path/file.txt", os.urandom(32))

    def test_large_file(self, tmp_path):
        f = tmp_path / "big.bin"
        data = os.urandom(1024 * 1024)  # 1 MB
        f.write_bytes(data)
        fek = os.urandom(32)

        enc_blob, _, _ = encrypt_file(str(f), fek)
        recovered = decrypt_file(enc_blob, fek)
        assert recovered == data

    def test_empty_file(self, tmp_path):
        f = tmp_path / "empty.txt"
        f.write_bytes(b"")
        fek = os.urandom(32)

        enc_blob, _, _ = encrypt_file(str(f), fek)
        recovered = decrypt_file(enc_blob, fek)
        assert recovered == b""

    def test_hash_values(self, tmp_path):
        data = b"hash test data"
        f = tmp_path / "hash_test.txt"
        f.write_bytes(data)
        fek = os.urandom(32)

        enc_blob, enc_hash, pt_hash = encrypt_file(str(f), fek)
        assert pt_hash == hash_bytes_blake2b(data)
        assert enc_hash == hash_bytes_blake2b(enc_blob)


# ── Store and retrieve ───────────────────────────────────────────

class TestStoreRetrieve:
    def test_store_and_retrieve(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path / 'vf')

        src = tmp_path / "source.txt"
        src.write_bytes(b"vault test content")
        fek = os.urandom(32)

        enc_hash, pt_hash, size, dest = store_encrypted_file(
            "test_vault", str(src), fek)

        assert dest.exists()
        assert size == 18

        out = tmp_path / "recovered.txt"
        result = retrieve_and_decrypt("test_vault", enc_hash, fek, str(out))
        assert result.exists()
        assert out.read_bytes() == b"vault test content"


# ── Share storage ────────────────────────────────────────────────

class TestShareStorage:
    def test_store_and_load(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', tmp_path / 'vs')

        value = os.urandom(32)
        store_share("v1", 1, value)
        result = load_share("v1", 1)
        assert result is not None
        assert result == (1, value)

    def test_load_missing(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', tmp_path / 'vs')

        result = load_share("nonexistent", 99)
        assert result is None

    def test_load_all_shares(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', tmp_path / 'vs')

        for i in range(1, 4):
            store_share("v1", i, os.urandom(32))

        shares = load_all_shares("v1")
        assert len(shares) == 3
        indices = [s[0] for s in shares]
        assert sorted(indices) == [1, 2, 3]

    def test_share_permissions(self, tmp_path, monkeypatch):
        import laud_files
        import stat
        monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', tmp_path / 'vs')

        path = store_share("v1", 1, os.urandom(32))
        mode = path.stat().st_mode
        assert mode & stat.S_IRUSR
        assert mode & stat.S_IWUSR
        assert not (mode & stat.S_IRGRP)
        assert not (mode & stat.S_IROTH)


# ── Share hex export/import ──────────────────────────────────────

class TestShareHex:
    def test_export_format(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', tmp_path / 'vs')

        value = bytes(range(32))
        store_share("v1", 3, value)
        hex_str = export_share_hex("v1", 3)
        assert hex_str is not None
        assert hex_str.startswith("03:")
        assert len(hex_str) == 3 + 64

    def test_import_valid(self):
        hex_str = "01:" + "ab" * 32
        result = import_share_hex(hex_str)
        assert result is not None
        assert result[0] == 1
        assert len(result[1]) == 32

    def test_import_invalid_format(self):
        assert import_share_hex("invalid") is None
        assert import_share_hex("01:short") is None
        assert import_share_hex("00:" + "ab" * 32) is None
        assert import_share_hex("") is None

    def test_roundtrip(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_SHARES_DIR', tmp_path / 'vs')

        value = os.urandom(32)
        store_share("v1", 5, value)
        hex_str = export_share_hex("v1", 5)
        idx, recovered = import_share_hex(hex_str)
        assert idx == 5
        assert recovered == value


# ── Index management ─────────────────────────────────────────────

class TestIndex:
    def test_add_and_get(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path / 'vf')

        add_to_index(
            vault_id="v1", enc_hash="abc123", plaintext_hash="def456",
            original_name="document.pdf", size_bytes=1024,
            uploader="alice", epoch=1, key_fingerprint_hex="aabbcc",
            threshold=2, ring_size=3)

        files = get_vault_files("v1")
        assert len(files) == 1
        assert files[0]['original_name'] == "document.pdf"
        assert files[0]['size_bytes'] == 1024

    def test_privacy_mode(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path / 'vf')

        add_to_index(
            vault_id="v1", enc_hash="abc123", plaintext_hash="def456",
            original_name="secret.pdf", size_bytes=512,
            uploader="bob", epoch=1, key_fingerprint_hex="aabbcc",
            threshold=2, ring_size=3, privacy_mode=True)

        files = get_vault_files("v1")
        assert len(files) == 1
        assert files[0]['name_redacted'] is True
        assert files[0]['display_label'] != "secret.pdf"
        assert len(files[0]['display_label']) == 64

    def test_anonymize_existing(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path / 'vf')

        add_to_index(
            vault_id="v1", enc_hash="abc123", plaintext_hash="def456",
            original_name="visible.pdf", size_bytes=256,
            uploader="carol", epoch=1, key_fingerprint_hex="aabbcc",
            threshold=2, ring_size=3, privacy_mode=False)

        count = anonymize_existing_index()
        assert count == 1

        files = get_vault_files("v1")
        assert files[0]['name_redacted'] is True
        assert files[0]['display_label'] != "visible.pdf"

    def test_empty_index(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path / 'vf')

        index = load_index()
        assert index == {}

    def test_multiple_vaults(self, tmp_path, monkeypatch):
        import laud_files
        monkeypatch.setattr(laud_files, 'VAULT_FILES_DIR', tmp_path / 'vf')

        add_to_index("v1", "h1", "ph1", "f1.txt", 100,
                     "alice", 1, "fp1", 2, 3)
        add_to_index("v2", "h2", "ph2", "f2.txt", 200,
                     "bob", 1, "fp2", 2, 3)

        assert len(get_vault_files("v1")) == 1
        assert len(get_vault_files("v2")) == 1
        assert len(get_vault_files("v3")) == 0


# ── Secure zero ──────────────────────────────────────────────────

class TestSecureZero:
    def test_zeros_bytearray(self):
        buf = bytearray(b"sensitive data here!!")
        secure_zero(buf)
        assert all(b == 0 for b in buf)

    def test_empty_bytearray(self):
        buf = bytearray()
        secure_zero(buf)

    def test_non_bytearray_noop(self):
        buf = b"immutable"
        secure_zero(buf)
