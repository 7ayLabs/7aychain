"""
LAUD NETWORKS - Cryptographic Primitives
GF(2^8) Shamir secret sharing matching Rust primitives/src/crypto.rs byte-for-byte.
"""

import hashlib
import os

# Domain separators - MUST match Rust constants byte-for-byte
DOMAIN_SHARE = b"7ay:share:v1"
DOMAIN_VSS = b"7ay:vss:v1"
DOMAIN_VAULT_FEK = b"7ay:vault:fek:v1"
DOMAIN_VAULT_FILE = b"7ay:vault:file:v1"
DOMAIN_UNLOCK = b"7ay:unlock:v1"

RIJNDAEL_POLY = 0x11B


# ── GF(2^8) arithmetic (Rijndael polynomial 0x11B) ──────────────

def gf256_mul(a, b):
    """Multiply two bytes in GF(2^8) with Rijndael reduction."""
    result = 0
    a &= 0xFF
    b &= 0xFF
    while b:
        if b & 1:
            result ^= a
        high_bit = a & 0x80
        a = (a << 1) & 0xFF
        if high_bit:
            a ^= RIJNDAEL_POLY & 0xFF
        b >>= 1
    return result


def gf256_inv(a):
    """Multiplicative inverse in GF(2^8). inv(0) = 0."""
    if a == 0:
        return 0
    result = a
    for _ in range(6):
        result = gf256_mul(result, result)
        result = gf256_mul(result, a)
    return gf256_mul(result, result)


def gf256_div(a, b):
    """Divide a by b in GF(2^8). Returns None if b == 0."""
    if b == 0:
        return None
    return gf256_mul(a, gf256_inv(b))


# ── Hash utilities ───────────────────────────────────────────────

def hash_with_domain(domain, data):
    """Blake2-256 with domain separation. Returns 32 bytes.

    Matches Rust: blake2_256(domain || data).
    """
    return hashlib.blake2b(domain + data, digest_size=32).digest()


def key_fingerprint(fek):
    """Compute fingerprint of a File Encryption Key (32 bytes).

    Returns 32-byte digest matching Rust key_fingerprint().
    """
    return hash_with_domain(DOMAIN_VAULT_FEK, bytes(fek))


def share_commitment(index, value):
    """Compute share commitment hash matching Rust ShamirScheme::hash_share.

    commitment = blake2_256(DOMAIN_SHARE || index_byte || value_32bytes)
    """
    return hash_with_domain(
        DOMAIN_SHARE, bytes([index]) + bytes(value)
    )


# ── Shamir Secret Sharing over GF(2^8) ──────────────────────────

def eval_polynomial(coeffs, x):
    """Evaluate polynomial at x, byte-by-byte across 32 coefficients.

    coeffs: list of 32-byte arrays (coefficients, index 0 = constant).
    x: evaluation point (1..255).

    Matches Rust eval_polynomial() exactly: byte_idx-major loop.
    """
    result = bytearray(32)
    for byte_idx in range(32):
        value = 0
        for coeff in reversed(coeffs):
            value = gf256_mul(value, x) ^ coeff[byte_idx]
        result[byte_idx] = value
    return bytes(result)


class ShamirScheme:
    """Shamir secret sharing matching Rust ShamirScheme byte-for-byte."""

    @staticmethod
    def split(secret_32, threshold, total):
        """Split a 32-byte secret into shares.

        Coefficient derivation matches Rust exactly:
            coeff[i] = blake2_256(secret || bytes([i]))

        Returns list of (index, value) tuples where index is 1..total.
        Returns None on invalid parameters.
        """
        if not isinstance(secret_32, (bytes, bytearray)):
            return None
        if len(secret_32) != 32:
            return None
        if threshold < 2 or total < threshold or total == 0:
            return None

        secret = bytes(secret_32)
        coefficients = [secret]

        for i in range(1, threshold):
            seed = secret + bytes([i])
            coeff = hashlib.blake2b(seed, digest_size=32).digest()
            coefficients.append(coeff)

        shares = []
        for idx in range(1, total + 1):
            share_value = eval_polynomial(coefficients, idx)
            shares.append((idx, share_value))

        return shares

    @staticmethod
    def reconstruct(shares, threshold):
        """Reconstruct the 32-byte secret from shares.

        shares: list of (index, value) tuples.
        threshold: minimum shares required.

        Returns 32-byte secret or None.
        """
        if len(shares) < threshold:
            return None

        subset = shares[:threshold]
        secret = bytearray(32)

        for byte_idx in range(32):
            result = 0
            for i, (xi, yi) in enumerate(subset):
                li = _lagrange_basis_at_zero(subset, i, xi)
                if li is None:
                    return None
                result ^= gf256_mul(yi[byte_idx], li)
            secret[byte_idx] = result

        return bytes(secret)

    @staticmethod
    def create_commitment(index, value):
        """Create a share commitment (hash).

        Returns 32-byte hash matching Rust ShamirScheme::create_commitment.
        """
        return share_commitment(index, value)

    @staticmethod
    def verify_share(index, value, commitment):
        """Verify a share against its commitment.

        Returns True if hash(DOMAIN_SHARE || index || value) == commitment.
        """
        computed = share_commitment(index, value)
        # Constant-time comparison
        return _ct_eq(computed, commitment)


def _lagrange_basis_at_zero(shares, i, xi):
    """Compute Lagrange basis polynomial at x=0 for share i.

    Matches Rust compute_lagrange_basis_at_zero().
    """
    result = 1
    for j, (xj, _) in enumerate(shares):
        if i != j:
            denom = xi ^ xj  # GF(2^8) subtraction is XOR
            if denom == 0:
                return None
            div_result = gf256_div(xj, denom)
            if div_result is None:
                return None
            result = gf256_mul(result, div_result)
    return result


def _ct_eq(a, b):
    """Constant-time equality comparison for byte strings."""
    if len(a) != len(b):
        return False
    result = 0
    for x, y in zip(a, b):
        result |= x ^ y
    return result == 0


def generate_fek():
    """Generate a random 32-byte File Encryption Key."""
    return os.urandom(32)
