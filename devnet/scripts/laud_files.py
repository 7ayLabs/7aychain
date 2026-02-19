"""
LAUD NETWORKS - Vault File Management
Handles local file storage, hashing, and index management.
"""

import hashlib
import json
import pathlib
import shutil
from datetime import datetime


VAULT_FILES_DIR = pathlib.Path.home() / '.laud' / 'vault_files'


def ensure_vault_dir(vault_id):
    """Create and return the vault directory for a given vault_id."""
    d = VAULT_FILES_DIR / str(vault_id)
    d.mkdir(parents=True, exist_ok=True)
    return d


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


def add_to_index(vault_id, file_hash, original_name, size_bytes,
                 uploader, epoch):
    """Add a file entry to the index."""
    index = load_index()
    key = f"{vault_id}:{file_hash}"
    index[key] = {
        "vault_id": vault_id,
        "file_hash": file_hash,
        "original_name": original_name,
        "size_bytes": size_bytes,
        "uploaded_at": datetime.now().isoformat(),
        "uploader": uploader,
        "on_chain_epoch": epoch,
        "on_chain_key": f"0x{file_hash}",
        "verified": True,
    }
    save_index(index)
    return key


def get_vault_files(vault_id):
    """Return all file entries for a given vault_id."""
    index = load_index()
    return [v for v in index.values() if v.get('vault_id') == vault_id]


def verify_file(vault_id, file_hash):
    """Verify a local file still matches its recorded hash.

    Returns:
        (True, hash) if matches
        (False, current_hash) if mismatch
        (None, None) if file not found
    """
    local_path = ensure_vault_dir(vault_id) / file_hash
    if not local_path.exists():
        return None, None
    current_hash = hash_file_blake2b(local_path)
    return current_hash == file_hash, current_hash


def store_file(vault_id, source_path):
    """Copy a file into the vault directory and return its hash.

    Returns:
        (file_hash, dest_path) tuple
    """
    source = pathlib.Path(source_path).expanduser().resolve()
    if not source.is_file():
        raise FileNotFoundError(f"Source file not found: {source}")
    file_hash = hash_file_blake2b(source)
    vault_dir = ensure_vault_dir(vault_id)
    dest = vault_dir / file_hash
    shutil.copy2(str(source), str(dest))
    return file_hash, dest


def export_file(vault_id, file_hash, dest_path):
    """Export a vault file to an external location.

    Returns:
        The resolved destination path.
    """
    local_path = ensure_vault_dir(vault_id) / file_hash
    if not local_path.exists():
        raise FileNotFoundError(f"Vault file not found: {local_path}")
    dest = pathlib.Path(dest_path).expanduser().resolve()
    shutil.copy2(str(local_path), str(dest))
    return dest
