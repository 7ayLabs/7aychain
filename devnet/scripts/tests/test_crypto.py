"""
Tests for laud_crypto.py — GF(2^8) arithmetic, Shamir secret sharing,
domain-separated hashing, and commitment verification.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from laud_crypto import (
    gf256_mul, gf256_inv, gf256_div,
    hash_with_domain, key_fingerprint, share_commitment,
    eval_polynomial, ShamirScheme, generate_fek,
    DOMAIN_SHARE, DOMAIN_VAULT_FEK,
    _ct_eq,
)


# ── GF(2^8) arithmetic ──────────────────────────────────────────

class TestGF256Mul:
    def test_identity(self):
        for a in range(256):
            assert gf256_mul(a, 1) == a

    def test_zero(self):
        for a in range(256):
            assert gf256_mul(a, 0) == 0
            assert gf256_mul(0, a) == 0

    def test_commutative(self):
        for a in range(0, 256, 17):
            for b in range(0, 256, 19):
                assert gf256_mul(a, b) == gf256_mul(b, a)

    def test_known_values(self):
        assert gf256_mul(0x57, 0x83) == 0xC1

    def test_result_in_field(self):
        for a in range(0, 256, 7):
            for b in range(0, 256, 11):
                result = gf256_mul(a, b)
                assert 0 <= result <= 255


class TestGF256Inv:
    def test_inv_zero(self):
        assert gf256_inv(0) == 0

    def test_inv_one(self):
        assert gf256_inv(1) == 1

    def test_inverse_property(self):
        for a in range(1, 256):
            inv = gf256_inv(a)
            assert gf256_mul(a, inv) == 1, f"inv({a}) = {inv} failed"

    def test_double_inverse(self):
        for a in range(1, 256):
            assert gf256_inv(gf256_inv(a)) == a


class TestGF256Div:
    def test_div_by_zero(self):
        assert gf256_div(42, 0) is None

    def test_div_identity(self):
        for a in range(256):
            assert gf256_div(a, 1) == a

    def test_self_div(self):
        for a in range(1, 256):
            assert gf256_div(a, a) == 1

    def test_div_roundtrip(self):
        for a in range(0, 256, 13):
            for b in range(1, 256, 17):
                q = gf256_div(a, b)
                assert gf256_mul(q, b) == a


# ── Hash utilities ───────────────────────────────────────────────

class TestHashWithDomain:
    def test_deterministic(self):
        h1 = hash_with_domain(b"test", b"data")
        h2 = hash_with_domain(b"test", b"data")
        assert h1 == h2

    def test_length(self):
        h = hash_with_domain(b"domain", b"data")
        assert len(h) == 32

    def test_domain_separation(self):
        h1 = hash_with_domain(b"domain_a", b"data")
        h2 = hash_with_domain(b"domain_b", b"data")
        assert h1 != h2

    def test_data_sensitivity(self):
        h1 = hash_with_domain(b"domain", b"data_a")
        h2 = hash_with_domain(b"domain", b"data_b")
        assert h1 != h2


class TestKeyFingerprint:
    def test_deterministic(self):
        fek = os.urandom(32)
        fp1 = key_fingerprint(fek)
        fp2 = key_fingerprint(fek)
        assert fp1 == fp2

    def test_length(self):
        fek = os.urandom(32)
        fp = key_fingerprint(fek)
        assert len(fp) == 32

    def test_different_keys_different_fingerprints(self):
        fek1 = os.urandom(32)
        fek2 = os.urandom(32)
        assert key_fingerprint(fek1) != key_fingerprint(fek2)

    def test_uses_correct_domain(self):
        fek = b'\x00' * 32
        expected = hash_with_domain(DOMAIN_VAULT_FEK, fek)
        assert key_fingerprint(fek) == expected


class TestShareCommitment:
    def test_deterministic(self):
        value = os.urandom(32)
        c1 = share_commitment(1, value)
        c2 = share_commitment(1, value)
        assert c1 == c2

    def test_different_index(self):
        value = os.urandom(32)
        c1 = share_commitment(1, value)
        c2 = share_commitment(2, value)
        assert c1 != c2

    def test_different_value(self):
        v1 = os.urandom(32)
        v2 = os.urandom(32)
        c1 = share_commitment(1, v1)
        c2 = share_commitment(1, v2)
        assert c1 != c2

    def test_uses_correct_domain(self):
        value = b'\xAB' * 32
        expected = hash_with_domain(
            DOMAIN_SHARE, bytes([3]) + value)
        assert share_commitment(3, value) == expected


# ── Shamir Secret Sharing ────────────────────────────────────────

class TestShamirSplit:
    def test_invalid_secret_length(self):
        assert ShamirScheme.split(b'\x00' * 16, 2, 3) is None
        assert ShamirScheme.split(b'\x00' * 64, 2, 3) is None

    def test_invalid_secret_type(self):
        assert ShamirScheme.split("not bytes", 2, 3) is None
        assert ShamirScheme.split(42, 2, 3) is None

    def test_invalid_threshold(self):
        secret = os.urandom(32)
        assert ShamirScheme.split(secret, 1, 3) is None
        assert ShamirScheme.split(secret, 0, 3) is None

    def test_threshold_exceeds_total(self):
        secret = os.urandom(32)
        assert ShamirScheme.split(secret, 4, 3) is None

    def test_zero_total(self):
        secret = os.urandom(32)
        assert ShamirScheme.split(secret, 2, 0) is None

    def test_produces_correct_count(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 5)
        assert shares is not None
        assert len(shares) == 5

    def test_share_indices(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 3, 5)
        indices = [s[0] for s in shares]
        assert indices == [1, 2, 3, 4, 5]

    def test_share_values_are_32_bytes(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 3)
        for _, value in shares:
            assert len(value) == 32

    def test_deterministic(self):
        secret = b'\x42' * 32
        shares1 = ShamirScheme.split(secret, 2, 3)
        shares2 = ShamirScheme.split(secret, 2, 3)
        assert shares1 == shares2

    def test_different_secrets_different_shares(self):
        s1 = os.urandom(32)
        s2 = os.urandom(32)
        shares1 = ShamirScheme.split(s1, 2, 3)
        shares2 = ShamirScheme.split(s2, 2, 3)
        assert shares1 != shares2


class TestShamirReconstruct:
    def test_basic_2_of_3(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 3)
        recovered = ShamirScheme.reconstruct(shares[:2], 2)
        assert recovered == secret

    def test_basic_3_of_5(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 3, 5)
        recovered = ShamirScheme.reconstruct(shares[:3], 3)
        assert recovered == secret

    def test_any_subset(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 5)
        import itertools
        for combo in itertools.combinations(shares, 2):
            recovered = ShamirScheme.reconstruct(list(combo), 2)
            assert recovered == secret

    def test_insufficient_shares(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 3, 5)
        result = ShamirScheme.reconstruct(shares[:2], 3)
        assert result is None

    def test_excess_shares(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 5)
        recovered = ShamirScheme.reconstruct(shares, 2)
        assert recovered == secret

    def test_all_zeros_secret(self):
        secret = b'\x00' * 32
        shares = ShamirScheme.split(secret, 2, 3)
        recovered = ShamirScheme.reconstruct(shares[:2], 2)
        assert recovered == secret

    def test_all_ones_secret(self):
        secret = b'\xFF' * 32
        shares = ShamirScheme.split(secret, 2, 3)
        recovered = ShamirScheme.reconstruct(shares[:2], 2)
        assert recovered == secret

    def test_roundtrip_various_thresholds(self):
        secret = os.urandom(32)
        for t in range(2, 6):
            n = t + 2
            shares = ShamirScheme.split(secret, t, n)
            recovered = ShamirScheme.reconstruct(shares[:t], t)
            assert recovered == secret, f"Failed for t={t}, n={n}"


class TestShamirCommitments:
    def test_create_and_verify(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 3)
        for idx, value in shares:
            commitment = ShamirScheme.create_commitment(idx, value)
            assert ShamirScheme.verify_share(idx, value, commitment)

    def test_wrong_value_fails(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 3)
        idx, value = shares[0]
        commitment = ShamirScheme.create_commitment(idx, value)
        wrong_value = os.urandom(32)
        assert not ShamirScheme.verify_share(idx, wrong_value, commitment)

    def test_wrong_index_fails(self):
        secret = os.urandom(32)
        shares = ShamirScheme.split(secret, 2, 3)
        idx, value = shares[0]
        commitment = ShamirScheme.create_commitment(idx, value)
        assert not ShamirScheme.verify_share(idx + 1, value, commitment)


# ── Constant-time comparison ─────────────────────────────────────

class TestConstantTimeEq:
    def test_equal(self):
        a = b'\x42' * 32
        assert _ct_eq(a, a)

    def test_not_equal(self):
        a = b'\x42' * 32
        b = b'\x43' * 32
        assert not _ct_eq(a, b)

    def test_different_length(self):
        assert not _ct_eq(b'\x00' * 31, b'\x00' * 32)

    def test_single_bit_diff(self):
        a = b'\x00' * 32
        b_arr = bytearray(32)
        b_arr[15] = 0x01
        assert not _ct_eq(a, bytes(b_arr))


# ── FEK generation ───────────────────────────────────────────────

class TestGenerateFek:
    def test_length(self):
        fek = generate_fek()
        assert len(fek) == 32

    def test_randomness(self):
        fek1 = generate_fek()
        fek2 = generate_fek()
        assert fek1 != fek2

    def test_returns_bytes(self):
        fek = generate_fek()
        assert isinstance(fek, bytes)


# ── Polynomial evaluation ────────────────────────────────────────

class TestEvalPolynomial:
    def test_constant_polynomial(self):
        secret = os.urandom(32)
        result = eval_polynomial([secret], 5)
        assert result == secret

    def test_zero_polynomial(self):
        zero = b'\x00' * 32
        result = eval_polynomial([zero, zero], 42)
        assert result == zero
