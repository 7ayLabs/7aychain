"""
LAUD NETWORKS 7ayLabs - Vault File Management
Handles encrypted file storage, share management, and index tracking.
Files are encrypted with AES-256-GCM; encryption keys are split via Shamir.
"""

import ctypes
import hashlib
import json
import os
import pathlib
import stat
from datetime import datetime

try:
    from cryptography.hazmat.primitives.ciphers.aead import AESGCM
    CRYPTO_OK = True
except ImportError:
    CRYPTO_OK = False


# ── Constants ────────────────────────────────────────────────────

VAULT_FILES_DIR = pathlib.Path.home() / '.laud' / 'vault_files'
VAULT_SHARES_DIR = pathlib.Path.home() / '.laud' / 'vault_shares'

FILE_MAGIC = b"7VLT"
FILE_VERSION = (0).to_bytes(1, 'big') + (1).to_bytes(1, 'big')
FILE_RESERVED = b"\x00\x00"
HEADER_SIZE = 4 + 2 + 2 + 12 + 16  # magic + version + reserved + nonce + tag = 36

MAX_FILE_SIZE = 100 * 1024 * 1024  # 100 MB


# ── Directory helpers ────────────────────────────────────────────

def ensure_vault_dir(vault_id):
    """Create and return the vault directory for a given vault_id."""
    d = VAULT_FILES_DIR / str(vault_id)
    d.mkdir(parents=True, exist_ok=True)
    return d


def ensure_shares_dir(vault_id):
    """Create and return the shares directory for a given vault_id."""
    d = VAULT_SHARES_DIR / str(vault_id)
    d.mkdir(parents=True, exist_ok=True)
    return d


# ── Hashing ──────────────────────────────────────────────────────

def hash_file_blake2b(file_path):
    """Compute Blake2-256 hash of a file. Returns hex string (no 0x prefix)."""
    h = hashlib.blake2b(digest_size=32)
    with open(file_path, 'rb') as f:
        while True:
            chunk = f.read(8192)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def hash_bytes_blake2b(data):
    """Compute Blake2-256 hash of bytes. Returns hex string (no 0x prefix)."""
    return hashlib.blake2b(data, digest_size=32).hexdigest()


# ── AES-256-GCM encryption ──────────────────────────────────────

def encrypt_file(source_path, fek):
    """Encrypt a file with AES-256-GCM.

    Format (.enc):
        [4]  Magic: b"7VLT"
        [2]  Version: 0x0001
        [2]  Reserved: 0x0000
        [12] Nonce (random)
        [16] GCM authentication tag
        [..] Ciphertext

    Args:
        source_path: path to plaintext file
        fek: 32-byte File Encryption Key

    Returns:
        (ciphertext_bytes, enc_hash_hex, plaintext_hash_hex)
    """
    if not CRYPTO_OK:
        raise RuntimeError("cryptography package required: pip install cryptography")

    source = pathlib.Path(source_path).expanduser().resolve()
    if not source.is_file():
        raise FileNotFoundError(f"Source file not found: {source}")

    plaintext = source.read_bytes()
    if len(plaintext) > MAX_FILE_SIZE:
        raise ValueError(f"File too large: {len(plaintext)} bytes (max {MAX_FILE_SIZE})")

    plaintext_hash = hash_bytes_blake2b(plaintext)

    nonce = os.urandom(12)
    aesgcm = AESGCM(bytes(fek))
    ct_and_tag = aesgcm.encrypt(nonce, plaintext, None)
    # cryptography library appends 16-byte tag to ciphertext
    ciphertext = ct_and_tag[:-16]
    tag = ct_and_tag[-16:]

    enc_blob = FILE_MAGIC + FILE_VERSION + FILE_RESERVED + nonce + tag + ciphertext
    enc_hash = hash_bytes_blake2b(enc_blob)

    return enc_blob, enc_hash, plaintext_hash


def decrypt_file(enc_blob, fek):
    """Decrypt an AES-256-GCM encrypted file.

    Args:
        enc_blob: raw bytes of the .enc file
        fek: 32-byte File Encryption Key

    Returns:
        plaintext bytes

    Raises:
        ValueError: on invalid format or authentication failure
    """
    if not CRYPTO_OK:
        raise RuntimeError("cryptography package required: pip install cryptography")

    if len(enc_blob) < HEADER_SIZE:
        raise ValueError("File too short to be a valid .enc file")

    magic = enc_blob[0:4]
    if magic != FILE_MAGIC:
        raise ValueError(f"Invalid magic: {magic!r} (expected {FILE_MAGIC!r})")

    version = enc_blob[4:6]
    if version != FILE_VERSION:
        raise ValueError(f"Unsupported version: {version.hex()}")

    nonce = enc_blob[8:20]
    tag = enc_blob[20:36]
    ciphertext = enc_blob[36:]

    aesgcm = AESGCM(bytes(fek))
    # Reconstruct ct+tag format expected by cryptography library
    ct_and_tag = ciphertext + tag
    try:
        plaintext = aesgcm.decrypt(nonce, ct_and_tag, None)
    except Exception:
        raise ValueError("Decryption failed: invalid key or corrupted file")

    return plaintext


# ── Encrypted file storage ───────────────────────────────────────

def store_encrypted_file(vault_id, source_path, fek):
    """Encrypt and store a file in the vault directory.

    Returns:
        (enc_hash_hex, plaintext_hash_hex, size_bytes, dest_path)
    """
    source = pathlib.Path(source_path).expanduser().resolve()
    size_bytes = source.stat().st_size

    enc_blob, enc_hash, plaintext_hash = encrypt_file(source_path, fek)

    vault_dir = ensure_vault_dir(vault_id)
    dest = vault_dir / f"{enc_hash}.enc"
    dest.write_bytes(enc_blob)

    return enc_hash, plaintext_hash, size_bytes, dest


def retrieve_and_decrypt(vault_id, enc_hash, fek, dest_path):
    """Load an encrypted vault file, decrypt, and write to destination.

    Returns:
        resolved destination path
    """
    vault_dir = ensure_vault_dir(vault_id)
    enc_path = vault_dir / f"{enc_hash}.enc"
    if not enc_path.exists():
        raise FileNotFoundError(f"Encrypted file not found: {enc_path}")

    enc_blob = enc_path.read_bytes()
    plaintext = decrypt_file(enc_blob, fek)

    dest = pathlib.Path(dest_path).expanduser().resolve()
    dest.write_bytes(plaintext)
    return dest


# ── Share storage (local filesystem, 0600 perms) ────────────────

def store_share(vault_id, share_index, share_value):
    """Write a share to the local share directory with restricted permissions.

    File: ~/.laud/vault_shares/<vault_id>/share_<index>.bin
    """
    shares_dir = ensure_shares_dir(vault_id)
    share_path = shares_dir / f"share_{share_index}.bin"
    share_path.write_bytes(bytes(share_value))
    share_path.chmod(stat.S_IRUSR | stat.S_IWUSR)  # 0600
    return share_path


def load_share(vault_id, share_index):
    """Read a share from the local share directory.

    Returns:
        (index, value_bytes) or None if not found
    """
    shares_dir = VAULT_SHARES_DIR / str(vault_id)
    share_path = shares_dir / f"share_{share_index}.bin"
    if not share_path.exists():
        return None
    value = share_path.read_bytes()
    return (share_index, value)


def load_all_shares(vault_id):
    """Load all locally available shares for a vault.

    Returns:
        list of (index, value_bytes) tuples
    """
    shares_dir = VAULT_SHARES_DIR / str(vault_id)
    if not shares_dir.exists():
        return []

    shares = []
    for p in sorted(shares_dir.glob("share_*.bin")):
        try:
            idx = int(p.stem.split("_")[1])
            value = p.read_bytes()
            if len(value) == 32:
                shares.append((idx, value))
        except (ValueError, IndexError):
            continue
    return shares


def export_share_hex(vault_id, share_index):
    """Export a share as a hex string for manual transfer.

    Format: <index_byte_hex>:<value_hex> (e.g., "01:abcd...")
    """
    result = load_share(vault_id, share_index)
    if result is None:
        return None
    idx, value = result
    return f"{idx:02x}:{value.hex()}"


def import_share_hex(hex_string):
    """Parse a hex-encoded share string.

    Format: <index_byte_hex>:<value_hex>
    Returns (index, value_bytes) or None on parse error.
    """
    try:
        parts = hex_string.strip().split(":")
        if len(parts) != 2:
            return None
        idx = int(parts[0], 16)
        value = bytes.fromhex(parts[1])
        if len(value) != 32 or idx < 1 or idx > 255:
            return None
        return (idx, value)
    except (ValueError, IndexError):
        return None


# ── Memory safety ────────────────────────────────────────────────

def secure_zero(buf):
    """Best-effort zeroing of a mutable buffer (bytearray).

    Uses ctypes.memset to avoid Python optimizer skipping dead writes.
    """
    if isinstance(buf, bytearray) and len(buf) > 0:
        ctypes.memset((ctypes.c_char * len(buf)).from_buffer(buf), 0, len(buf))


# ── Index management ─────────────────────────────────────────────

def load_index():
    """Load the vault file index."""
    index_path = VAULT_FILES_DIR / 'index.json'
    try:
        with open(index_path, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def save_index(index):
    """Save the vault file index."""
    VAULT_FILES_DIR.mkdir(parents=True, exist_ok=True)
    index_path = VAULT_FILES_DIR / 'index.json'
    with open(index_path, 'w') as f:
        json.dump(index, f, indent=2)


def add_to_index(vault_id, enc_hash, plaintext_hash, original_name,
                 size_bytes, uploader, epoch, key_fingerprint_hex,
                 threshold, ring_size):
    """Add an encrypted file entry to the index."""
    index = load_index()
    key = f"{vault_id}:{enc_hash}"
    index[key] = {
        "vault_id": vault_id,
        "enc_hash": enc_hash,
        "plaintext_hash": plaintext_hash,
        "original_name": original_name,
        "size_bytes": size_bytes,
        "uploaded_at": datetime.now().isoformat(),
        "uploader": uploader,
        "on_chain_key": f"0x{enc_hash}",
        "key_fingerprint": key_fingerprint_hex,
        "threshold": threshold,
        "ring_size": ring_size,
        "encrypted": True,
        "verified": True,
    }
    save_index(index)
    return key


def get_vault_files(vault_id):
    """Return all file entries for a given vault_id."""
    index = load_index()
    return [v for v in index.values() if v.get('vault_id') == vault_id]


def verify_file(vault_id, enc_hash):
    """Verify a local encrypted file still matches its recorded hash.

    Returns:
        (True, hash) if matches
        (False, current_hash) if mismatch
        (None, None) if file not found
    """
    local_path = ensure_vault_dir(vault_id) / f"{enc_hash}.enc"
    if not local_path.exists():
        return None, None
    current_hash = hash_file_blake2b(local_path)
    return current_hash == enc_hash, current_hash
