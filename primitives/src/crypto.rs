//! Cryptographic primitives for presence verification and state proofs.

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sha2::Digest;
use sp_core::{blake2_256, H256};
use sp_runtime::RuntimeDebug;

use crate::traits::{ConstantTimeEq, CryptoHash, DomainSeparatedHash};

// Domain separators for hash functions
pub const DOMAIN_PRESENCE: &[u8] = b"7ay:presence:v1";
pub const DOMAIN_EPOCH: &[u8] = b"7ay:epoch:v1";
pub const DOMAIN_COMMITMENT: &[u8] = b"7ay:commit:v1";
pub const DOMAIN_MERKLE: &[u8] = b"7ay:merkle:v1";
pub const DOMAIN_NULLIFIER: &[u8] = b"7ay:nullifier:v1";
pub const DOMAIN_BOOMERANG: &[u8] = b"7ay:boomerang:v1";
pub const DOMAIN_STORAGE_KEY: &[u8] = b"7ay:storage:key:v1";
pub const DOMAIN_ENTROPY_MIX: &[u8] = b"7ay:entropy:mix:v1";
pub const DOMAIN_VRF_EPOCH: &[u8] = b"7ay:vrf:epoch:v1";

// NIST SHA-256 domain separators (v0.9.0 dual-hash layer)
// These use a `nist:` prefix to ensure domain separation between Blake2 and SHA-256
// hash families per FIPS 180-4 compliance requirements.
pub const NIST_DOMAIN_PRESENCE: &[u8] = b"7ay:nist:presence:v1";
pub const NIST_DOMAIN_COMMITMENT: &[u8] = b"7ay:nist:commit:v1";
pub const NIST_DOMAIN_MERKLE: &[u8] = b"7ay:nist:merkle:v1";
pub const NIST_DOMAIN_NULLIFIER: &[u8] = b"7ay:nist:nullifier:v1";
pub const NIST_DOMAIN_FINGERPRINT: &[u8] = b"7ay:nist:fingerprint:v1";

/// SHA-256 with domain separation (NIST FIPS 180-4).
///
/// Mirrors `hash_with_domain` but uses SHA-256 instead of Blake2-256.
/// Length-prefixed domain prevents ambiguity between domain and data.
#[inline]
pub fn sha256_with_domain(domain: &[u8], data: &[u8]) -> H256 {
    let domain_len = (domain.len() as u32).to_le_bytes();
    let mut hasher = sha2::Sha256::new();
    hasher.update(domain_len);
    hasher.update(domain);
    hasher.update(data);
    let result = hasher.finalize();
    H256::from_slice(&result)
}

/// Raw SHA-256 digest without domain separation.
#[inline]
pub fn sha256_raw(data: &[u8]) -> [u8; 32] {
    let result = sha2::Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// SHA-256 hash pair for NIST-compliant Merkle tree nodes.
///
/// Uses `NIST_DOMAIN_MERKLE` for internal node separation, mirroring
/// the Blake2-based `hash_pair` function.
#[inline]
pub fn sha256_hash_pair(left: &H256, right: &H256) -> H256 {
    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(left.as_bytes());
    data.extend_from_slice(right.as_bytes());
    sha256_with_domain(NIST_DOMAIN_MERKLE, &data)
}

/// NIST-compliant key fingerprint using SHA-256.
///
/// Produces a deterministic fingerprint of a cryptographic key for
/// identification without revealing the key material.
#[inline]
pub fn nist_key_fingerprint(key: &[u8; 32]) -> H256 {
    sha256_with_domain(NIST_DOMAIN_FINGERPRINT, key)
}

/// Hash with domain separation.
///
/// Prefixes the domain length to prevent ambiguity when domain and data
/// are concatenated (e.g., domain="ab" + data="cd" vs domain="a" + data="bcd").
#[inline]
pub fn hash_with_domain(domain: &[u8], data: &[u8]) -> H256 {
    let domain_len = (domain.len() as u32).to_le_bytes();
    let mut input = Vec::with_capacity(4 + domain.len() + data.len());
    input.extend_from_slice(&domain_len);
    input.extend_from_slice(domain);
    input.extend_from_slice(data);
    H256(blake2_256(&input))
}

/// Hash two values together (for Merkle trees).
/// Uses DOMAIN_MERKLE via hash_with_domain to separate internal nodes
/// from leaf hashes with consistent length-prefixed domain separation.
#[inline]
pub fn hash_pair(left: &H256, right: &H256) -> H256 {
    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(left.as_bytes());
    data.extend_from_slice(right.as_bytes());
    hash_with_domain(DOMAIN_MERKLE, &data)
}

/// Pedersen-style commitment: C = H(domain || value || randomness)
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct PresenceCommitment(pub H256);

impl PresenceCommitment {
    pub fn new<V: Encode>(value: &V, randomness: &[u8; 32]) -> Self {
        let value_bytes = value.encode();
        let mut data = Vec::with_capacity(value_bytes.len() + 32);
        data.extend_from_slice(&value_bytes);
        data.extend_from_slice(randomness);
        Self(hash_with_domain(DOMAIN_COMMITMENT, &data))
    }

    pub fn verify<V: Encode>(&self, value: &V, randomness: &[u8; 32]) -> bool {
        let expected = Self::new(value, randomness);
        self.0.ct_eq(&expected.0)
    }

    pub const fn as_h256(&self) -> &H256 {
        &self.0
    }
}

/// Merkle proof for membership verification.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct MerkleProof {
    pub leaf_index: u64,
    pub siblings: Vec<H256>,
}

impl MerkleProof {
    /// Verify membership against a root.
    pub fn verify(&self, root: &H256, leaf: &H256) -> bool {
        let mut current = *leaf;
        let mut index = self.leaf_index;

        for sibling in &self.siblings {
            current = if index & 1 == 0 {
                hash_pair(&current, sibling)
            } else {
                hash_pair(sibling, &current)
            };
            index >>= 1;
        }

        if index != 0 {
            return false;
        }

        current.ct_eq(root)
    }

    /// Compute the root from a leaf and proof.
    pub fn compute_root(&self, leaf: &H256) -> H256 {
        let mut current = *leaf;
        let mut index = self.leaf_index;

        for sibling in &self.siblings {
            current = if index & 1 == 0 {
                hash_pair(&current, sibling)
            } else {
                hash_pair(sibling, &current)
            };
            index >>= 1;
        }

        current
    }
}

/// Nullifier to prevent double-spending/double-presence.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Hash,
)]
pub struct Nullifier(pub H256);

impl Nullifier {
    /// Derive a nullifier from a secret, epoch, and chain context.
    ///
    /// INV1: One nullifier per (secret, epoch, chain) tuple — no nonce
    /// parameter, ensuring the same actor cannot produce different nullifiers
    /// for the same epoch, which would defeat double-presence prevention.
    ///
    /// M12: Including `genesis_hash` prevents cross-chain replay of
    /// nullifiers. A nullifier valid on one chain is invalid on any other
    /// chain with a different genesis block.
    pub fn derive(secret: &[u8; 32], epoch_id: u64, genesis_hash: &[u8; 32]) -> Self {
        let mut data = Vec::with_capacity(32 + 8 + 32);
        data.extend_from_slice(secret);
        data.extend_from_slice(&epoch_id.to_le_bytes());
        data.extend_from_slice(genesis_hash);
        Self(hash_with_domain(DOMAIN_NULLIFIER, &data))
    }

    /// Legacy derivation without chain binding (for migration/testing).
    #[cfg(test)]
    pub fn derive_legacy(secret: &[u8; 32], epoch_id: u64) -> Self {
        let mut data = Vec::with_capacity(32 + 8);
        data.extend_from_slice(secret);
        data.extend_from_slice(&epoch_id.to_le_bytes());
        Self(hash_with_domain(DOMAIN_NULLIFIER, &data))
    }
}

/// State root representing a snapshot of all presence data.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Default,
)]
pub struct StateRoot(pub H256);

impl StateRoot {
    pub const EMPTY: Self = Self(H256([0u8; 32]));

    pub fn from_leaves(leaves: &[H256]) -> Self {
        if leaves.is_empty() {
            return Self::EMPTY;
        }

        let mut layer: Vec<H256> = leaves.to_vec();

        // Pad to power of 2
        let next_pow2 = layer.len().next_power_of_two();
        while layer.len() < next_pow2 {
            layer.push(H256::zero());
        }

        // Build tree bottom-up
        while layer.len() > 1 {
            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for chunk in layer.chunks(2) {
                next_layer.push(hash_pair(&chunk[0], &chunk[1]));
            }
            layer = next_layer;
        }

        Self(layer[0])
    }
}

/// Presence proof combining commitment and Merkle proof.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct PresenceProof {
    pub commitment: PresenceCommitment,
    pub merkle_proof: MerkleProof,
    pub nullifier: Nullifier,
}

impl PresenceProof {
    pub fn verify(&self, state_root: &StateRoot, commitment_leaf: &H256) -> bool {
        // Verify the commitment is included in the state
        self.merkle_proof.verify(&state_root.0, commitment_leaf)
    }
}

/// ZK statement for presence verification.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct PresenceStatement {
    pub epoch_id: u64,
    pub state_root: StateRoot,
    pub nullifier: Nullifier,
}

/// ZK witness (private inputs) for presence proof generation.
pub struct PresenceWitness {
    pub secret: [u8; 32],
    pub randomness: [u8; 32],
    pub merkle_path: Vec<H256>,
    pub leaf_index: u64,
}

/// Shamir secret sharing types for key distribution.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct ShareIndex(pub u8);

#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct Share {
    pub index: ShareIndex,
    pub value: [u8; 32],
}

pub mod gf256 {
    const RIJNDAEL_POLY: u16 = 0x11B;

    #[inline]
    pub fn mul(a: u8, b: u8) -> u8 {
        let mut result: u8 = 0;
        let mut a = a;
        let mut b = b;

        while b != 0 {
            if b & 1 != 0 {
                result ^= a;
            }
            let high_bit = a & 0x80;
            a <<= 1;
            if high_bit != 0 {
                a ^= (RIJNDAEL_POLY & 0xFF) as u8;
            }
            b >>= 1;
        }

        result
    }

    #[inline]
    pub fn add(a: u8, b: u8) -> u8 {
        a ^ b
    }

    #[inline]
    pub fn sub(a: u8, b: u8) -> u8 {
        a ^ b
    }

    pub fn inv(a: u8) -> u8 {
        if a == 0 {
            return 0;
        }

        let mut result = a;
        for _ in 0..6 {
            result = mul(result, result);
            result = mul(result, a);
        }
        mul(result, result)
    }

    #[inline]
    pub fn div(a: u8, b: u8) -> Option<u8> {
        if b == 0 {
            return None;
        }
        Some(mul(a, inv(b)))
    }

    pub fn pow(base: u8, exp: u8) -> u8 {
        if exp == 0 {
            return 1;
        }
        let mut result = 1u8;
        let mut base = base;
        let mut exp = exp;
        while exp > 0 {
            if exp & 1 != 0 {
                result = mul(result, base);
            }
            base = mul(base, base);
            exp >>= 1;
        }
        result
    }
}

pub fn eval_polynomial(coeffs: &[[u8; 32]], x: u8) -> [u8; 32] {
    let mut result = [0u8; 32];

    for byte_idx in 0..32 {
        let mut value = 0u8;
        for coeff in coeffs.iter().rev() {
            value = gf256::add(gf256::mul(value, x), coeff[byte_idx]);
        }
        result[byte_idx] = value;
    }

    result
}

impl CryptoHash for PresenceCommitment {
    fn crypto_hash(&self) -> H256 {
        self.0
    }
}

impl DomainSeparatedHash for PresenceCommitment {
    const DOMAIN: &'static [u8] = DOMAIN_COMMITMENT;

    fn domain_hash(&self) -> H256 {
        hash_with_domain(Self::DOMAIN, self.0.as_bytes())
    }
}

pub const DOMAIN_ACTOR: &[u8] = b"7ay:actor:v1";
pub const DOMAIN_VALIDATOR_ID: &[u8] = b"7ay:validator:v1";

/// Derive an ActorId from SCALE-encoded account bytes using domain separation.
pub fn derive_actor_id(account_bytes: &[u8]) -> crate::types::ActorId {
    let hash = hash_with_domain(DOMAIN_ACTOR, account_bytes);
    crate::types::ActorId::from_raw(hash.0)
}

/// Derive a ValidatorId from SCALE-encoded account bytes using domain separation.
pub fn derive_validator_id(account_bytes: &[u8]) -> crate::types::ValidatorId {
    let hash = hash_with_domain(DOMAIN_VALIDATOR_ID, account_bytes);
    crate::types::ValidatorId::from(H256(hash.0))
}

pub const DOMAIN_SHARE: &[u8] = b"7ay:share:v1";
pub const DOMAIN_VSS: &[u8] = b"7ay:vss:v1";
pub const DOMAIN_VAULT_FEK: &[u8] = b"7ay:vault:fek:v1";
pub const DOMAIN_VAULT_FILE: &[u8] = b"7ay:vault:file:v1";
pub const DOMAIN_UNLOCK: &[u8] = b"7ay:unlock:v1";

/// Domain separator for Proactive Secret Sharing refresh polynomials.
///
/// Used to derive deterministic polynomial coefficients for share
/// refreshing. The zero-constant-term property ensures the underlying
/// secret is unchanged after refresh.
pub const DOMAIN_PSS_REFRESH: &[u8] = b"7ay:pss:refresh:v1";

/// Domain separator for Schnorr-style share knowledge proofs.
///
/// Used in the Fiat-Shamir challenge derivation to bind proofs
/// to a specific share index and commitment.
pub const DOMAIN_SCHNORR_SHARE: &[u8] = b"7ay:schnorr:share:v1";

/// Compute a fingerprint of a File Encryption Key.
/// The FEK itself is never stored on-chain; only this fingerprint is.
pub fn key_fingerprint(fek: &[u8; 32]) -> H256 {
    hash_with_domain(DOMAIN_VAULT_FEK, fek)
}

pub struct ShamirScheme;

impl ShamirScheme {
    /// Split a secret into shares using (threshold, total) Shamir scheme.
    ///
    /// `entropy` provides randomness for polynomial coefficients. The raw
    /// entropy is mixed with the secret and scheme parameters via
    /// domain-separated hashing (H06) so that an attacker who controls
    /// `entropy` alone cannot predict the resulting polynomial coefficients.
    pub fn split(
        secret: &[u8; 32],
        threshold: u8,
        total: u8,
        entropy: &[u8; 32],
    ) -> Option<Vec<Share>> {
        if threshold < 2 || total < threshold || total == 0 {
            return None;
        }

        // H06: mix caller-provided entropy with secret and scheme parameters
        // so that controlling entropy alone is insufficient to predict shares
        let mut mix_input = Vec::with_capacity(32 + 32 + 2);
        mix_input.extend_from_slice(entropy);
        mix_input.extend_from_slice(secret);
        mix_input.push(threshold);
        mix_input.push(total);
        let mixed_entropy = hash_with_domain(DOMAIN_ENTROPY_MIX, &mix_input);

        let mut shares = Vec::with_capacity(total as usize);
        let mut coefficients = Vec::with_capacity(threshold as usize);
        coefficients.push(*secret);

        for i in 1..threshold {
            let seed_input = [mixed_entropy.as_bytes(), &[i][..]].concat();
            let coeff = hash_with_domain(b"7ay:shamir:coeff:v1", &seed_input).0;
            coefficients.push(coeff);
        }

        for idx in 1..=total {
            let share_value = eval_polynomial(&coefficients, idx);
            shares.push(Share {
                index: ShareIndex(idx),
                value: share_value,
            });
        }

        Some(shares)
    }

    pub fn reconstruct(shares: &[Share], threshold: u8) -> Option<[u8; 32]> {
        if shares.len() < threshold as usize {
            return None;
        }

        Self::reconstruct_inner(&shares[..threshold as usize])
    }

    fn reconstruct_inner(shares: &[Share]) -> Option<[u8; 32]> {
        let mut secret = [0u8; 32];

        for (byte_idx, slot) in secret.iter_mut().enumerate() {
            let mut result: u8 = 0;

            for (i, share_i) in shares.iter().enumerate() {
                let xi = share_i.index.0;
                let yi = share_i.value[byte_idx];
                let li = Self::compute_lagrange_basis_at_zero(shares, i, xi)?;
                result = gf256::add(result, gf256::mul(yi, li));
            }

            *slot = result;
        }

        Some(secret)
    }

    fn compute_lagrange_basis_at_zero(shares: &[Share], i: usize, xi: u8) -> Option<u8> {
        let mut result: u8 = 1;

        for (j, share_j) in shares.iter().enumerate() {
            if i != j {
                let xj = share_j.index.0;
                let denominator = gf256::sub(xi, xj);

                if denominator == 0 {
                    return None;
                }

                let div_result = gf256::div(xj, denominator)?;
                result = gf256::mul(result, div_result);
            }
        }

        Some(result)
    }

    pub fn verify_share(share: &Share, commitment: &H256) -> bool {
        let share_hash = Self::hash_share(share);
        share_hash.ct_eq(commitment)
    }

    pub fn hash_share(share: &Share) -> H256 {
        let mut data = Vec::with_capacity(33);
        data.push(share.index.0);
        data.extend_from_slice(&share.value);
        hash_with_domain(DOMAIN_SHARE, &data)
    }

    pub fn create_commitment(share: &Share) -> H256 {
        Self::hash_share(share)
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct VssCommitment {
    pub coefficients: Vec<H256>,
}

pub struct FeldmanVSS;

/// VssCommitment stores per-share commitments for verification.
///
/// NOTE: Hash-based commitments do not support homomorphic verification
/// (unlike EC-based Feldman VSS with g^a_i). Share verification here
/// checks that a share matches its committed hash, NOT that the share
/// is consistent with a specific polynomial. For polynomial consistency
/// guarantees, an EC-based scheme (e.g., using Ristretto points) is
/// required.
impl FeldmanVSS {
    pub fn share_with_commitments(
        secret: &[u8; 32],
        threshold: u8,
        total: u8,
        entropy: &[u8; 32],
    ) -> Option<(Vec<Share>, VssCommitment)> {
        if threshold < 2 || total < threshold {
            return None;
        }

        let shares = ShamirScheme::split(secret, threshold, total, entropy)?;

        // Store per-share commitments: H(DOMAIN_SHARE || index || value)
        let share_commitments: Vec<H256> =
            shares.iter().map(ShamirScheme::create_commitment).collect();

        Some((
            shares,
            VssCommitment {
                coefficients: share_commitments,
            },
        ))
    }

    /// Verify a share against its committed hash.
    ///
    /// Checks that H(DOMAIN_SHARE || index || value) matches the stored
    /// commitment at the share's index position.
    pub fn verify_share_against_commitments(share: &Share, commitments: &VssCommitment) -> bool {
        if share.index.0 == 0 {
            return false;
        }

        let idx = (share.index.0 as usize).saturating_sub(1);
        if idx >= commitments.coefficients.len() {
            return false;
        }

        let share_hash = ShamirScheme::create_commitment(share);
        share_hash.ct_eq(&commitments.coefficients[idx])
    }

    pub fn verify_share_count(shares: &[Share], threshold: u8) -> bool {
        shares.len() >= threshold as usize
    }
}

// =========================================================================
// Proactive Secret Sharing (PSS)
// =========================================================================

/// Generate refresh shares for Proactive Secret Sharing (PSS).
///
/// Creates a degree-(threshold-1) polynomial with a zero constant term,
/// evaluated at each participant index `1..=total`. Because the constant
/// term is zero, adding these deltas to existing Shamir shares does not
/// change the reconstructed secret:
///
/// ```text
///   delta(x) = 0 + b_1*x + b_2*x^2 + ... + b_{t-1}*x^{t-1}
///   delta(0) = 0  =>  secret is preserved after refresh
/// ```
///
/// Each participant generates one such polynomial and distributes its
/// evaluations. The final refresh delta for participant `j` is the
/// GF(2^8) sum of all `delta_i(j)` across all participants `i`. This
/// function generates a single participant's contribution.
///
/// Returns `None` if parameters are invalid (`threshold < 2`, or
/// `total < threshold`, or `total == 0`).
///
/// # Security
///
/// - Entropy is mixed with scheme parameters via domain-separated
///   hashing (`DOMAIN_PSS_REFRESH`) to prevent predictability even
///   if the attacker controls the entropy source.
/// - The constant term is unconditionally set to `[0u8; 32]` after
///   coefficient generation, enforcing the zero-secret invariant
///   regardless of the entropy derivation path.
/// - After applying refresh deltas, old shares become incompatible
///   with new shares, providing forward security.
pub fn pss_generate_refresh(threshold: u8, total: u8, entropy: &[u8; 32]) -> Option<Vec<Share>> {
    if threshold < 2 || total < threshold || total == 0 {
        return None;
    }

    // Mix entropy with scheme parameters for unpredictability
    let mut mix_input = Vec::with_capacity(32 + 2);
    mix_input.extend_from_slice(entropy);
    mix_input.push(threshold);
    mix_input.push(total);
    let mixed = hash_with_domain(DOMAIN_PSS_REFRESH, &mix_input);

    // Build polynomial coefficients: constant term = 0 (zero secret)
    let mut coefficients = Vec::with_capacity(threshold as usize);
    coefficients.push([0u8; 32]); // c_0 = 0 enforces secret preservation

    for i in 1..threshold {
        let seed_input = [mixed.as_bytes() as &[u8], &[i]].concat();
        let coeff = hash_with_domain(DOMAIN_PSS_REFRESH, &seed_input).0;
        coefficients.push(coeff);
    }

    // Evaluate the zero-secret polynomial at each participant index
    let mut refresh_shares = Vec::with_capacity(total as usize);
    for idx in 1..=total {
        let delta = eval_polynomial(&coefficients, idx);
        refresh_shares.push(Share {
            index: ShareIndex(idx),
            value: delta,
        });
    }

    Some(refresh_shares)
}

/// Apply refresh deltas to existing shares for Proactive Secret Sharing.
///
/// Computes new share values by adding (XOR in GF(2^8)) the refresh
/// delta to each existing share:
///
/// ```text
///   new_value[byte] = old_value[byte] XOR delta[byte]
/// ```
///
/// XOR is the addition operation in GF(2^8), matching the field
/// arithmetic used by the Shamir scheme.
///
/// Returns `None` if:
/// - The slice lengths differ
/// - Either slice is empty
/// - Any share index does not match its corresponding delta index
///
/// # Security
///
/// - Index matching is enforced to prevent accidental misapplication
///   of deltas to wrong participants.
/// - After refresh, old shares become incompatible with the new
///   share set, providing forward security for the threshold scheme.
pub fn pss_apply_refresh(old_shares: &[Share], refresh_deltas: &[Share]) -> Option<Vec<Share>> {
    if old_shares.len() != refresh_deltas.len() {
        return None;
    }

    if old_shares.is_empty() {
        return None;
    }

    let mut new_shares = Vec::with_capacity(old_shares.len());

    for (old, delta) in old_shares.iter().zip(refresh_deltas.iter()) {
        // Enforce that delta is applied to the correct participant
        if old.index.0 != delta.index.0 {
            return None;
        }

        let mut new_value = [0u8; 32];
        for (byte_idx, new_byte) in new_value.iter_mut().enumerate() {
            // GF(2^8) addition is XOR
            *new_byte = gf256::add(old.value[byte_idx], delta.value[byte_idx]);
        }

        new_shares.push(Share {
            index: ShareIndex(old.index.0),
            value: new_value,
        });
    }

    Some(new_shares)
}

// =========================================================================
// Schnorr Proof of Share Knowledge
// =========================================================================

/// Schnorr-style proof of knowledge of a share value.
///
/// Proves the statement: "I know a value `s` such that
/// `H(DOMAIN_SHARE || index || s) = commitment`" without revealing `s`.
///
/// The protocol uses the Fiat-Shamir heuristic to make the following
/// Sigma protocol non-interactive:
///
/// 1. **Commit**: Prover picks random nonce `k`, computes
///    `R = H(DOMAIN_SCHNORR_SHARE || index || k)`.
/// 2. **Challenge**: `e = H(DOMAIN_SCHNORR_SHARE || R || commitment)`
///    where `commitment = H(DOMAIN_SHARE || index || s)`.
/// 3. **Response**: `z[i] = k[i] XOR gf256_mul(e_byte, s[i])` for each
///    byte `i` in `0..32`.
///
/// Soundness in the random oracle model: A forger who does not know `s`
/// cannot produce a valid `(R, z)` pair because `R` is committed before
/// the challenge `e` is known, and computing `z` requires knowledge of
/// both `k` (bound by `R`) and `s` (bound by the commitment).
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct ShareKnowledgeProof {
    /// Random commitment `R = H(DOMAIN_SCHNORR_SHARE || index || k)`
    /// where `k` is the random nonce.
    pub commitment_r: H256,
    /// Response `z[i] = k[i] XOR gf256_mul(e_byte, s[i])` per byte,
    /// where `e_byte` is the first byte of the Fiat-Shamir challenge.
    pub response: [u8; 32],
}

/// Generate a Schnorr proof of knowledge for a share value.
///
/// Demonstrates knowledge of `share.value` that hashes to the
/// commitment `H(DOMAIN_SHARE || index || value)` without revealing
/// the share value itself.
///
/// `entropy` provides 32 bytes of randomness used as the seed for
/// nonce derivation. The entropy is mixed with the share index via
/// domain-separated hashing to prevent catastrophic nonce reuse if
/// the same entropy is accidentally supplied for different shares.
///
/// # Security
///
/// - Nonce `k` is derived from `entropy` mixed with the share index
///   to prevent nonce reuse across different share indices.
/// - Challenge `e` is derived via Fiat-Shamir with domain separation,
///   binding the proof to both the nonce commitment and the share
///   commitment.
/// - The response is computed in GF(2^8) per byte, matching the
///   field arithmetic of the underlying Shamir scheme.
pub fn prove_share_knowledge(share: &Share, entropy: &[u8; 32]) -> ShareKnowledgeProof {
    // Derive nonce k from entropy mixed with share context
    let mut nonce_input = Vec::with_capacity(32 + 1);
    nonce_input.extend_from_slice(entropy);
    nonce_input.push(share.index.0);
    let k = hash_with_domain(DOMAIN_SCHNORR_SHARE, &nonce_input).0;

    // R = H(DOMAIN_SCHNORR_SHARE || index || k)
    let mut r_input = Vec::with_capacity(1 + 32);
    r_input.push(share.index.0);
    r_input.extend_from_slice(&k);
    let commitment_r = hash_with_domain(DOMAIN_SCHNORR_SHARE, &r_input);

    // Compute the public commitment to this share
    let share_commitment = ShamirScheme::hash_share(share);

    // Challenge e = H(DOMAIN_SCHNORR_SHARE || R || share_commitment)
    let mut e_input = Vec::with_capacity(32 + 32);
    e_input.extend_from_slice(commitment_r.as_bytes());
    e_input.extend_from_slice(share_commitment.as_bytes());
    let e_hash = hash_with_domain(DOMAIN_SCHNORR_SHARE, &e_input);

    // Use first byte of challenge hash as the GF(2^8) scalar.
    // The soundness of the protocol per-byte is bounded by the
    // field size (2^8), but the overall proof covers all 32 bytes
    // simultaneously -- an adversary must guess correctly for ALL
    // bytes, giving effective security of 256 bits (32 * 8 independent
    // GF(2^8) equations) against brute-force forgery.
    let e_byte = e_hash.0[0];

    // Response: z[i] = k[i] XOR gf256_mul(e_byte, s[i])
    let mut response = [0u8; 32];
    for i in 0..32 {
        response[i] = gf256::add(k[i], gf256::mul(e_byte, share.value[i]));
    }

    ShareKnowledgeProof {
        commitment_r,
        response,
    }
}

/// Verify a share-knowledge proof.
///
/// The current construction proves knowledge relative to a hash commitment
/// `H(DOMAIN_SHARE || index || s)`, but hash-preimage knowledge is not a
/// relation this verifier can soundly check with the transcript alone.
///
/// Until the protocol is upgraded to a real proof system over a verifiable
/// algebraic commitment, this verifier fails closed and rejects all proofs
/// after basic structural validation. This avoids the previous unsafe
/// behavior where malformed proofs could be accepted without checking the
/// claimed witness relation at all.
pub fn verify_share_knowledge(
    proof: &ShareKnowledgeProof,
    share_index: ShareIndex,
    expected_commitment: &H256,
) -> bool {
    // R must not be zero (degenerate proof)
    if proof.commitment_r == H256::zero() {
        return false;
    }

    // Recompute challenge e = H(DOMAIN_SCHNORR_SHARE || R || commitment)
    let mut e_input = Vec::with_capacity(32 + 32);
    e_input.extend_from_slice(proof.commitment_r.as_bytes());
    e_input.extend_from_slice(expected_commitment.as_bytes());
    let e_hash = hash_with_domain(DOMAIN_SCHNORR_SHARE, &e_input);
    let e_byte = e_hash.0[0];

    // If e_byte == 0, the response z = k regardless of s, making
    // the proof trivially forgeable. Reject to maintain soundness.
    if e_byte == 0 {
        return false;
    }

    // Verify the share index is valid (non-zero, matching Shamir convention)
    if share_index.0 == 0 {
        return false;
    }

    let _ = proof.response;
    let _ = expected_commitment;

    false
}

impl Share {
    pub fn new(index: u8, value: [u8; 32]) -> Self {
        Self {
            index: ShareIndex(index),
            value,
        }
    }
}

impl ShareIndex {
    pub fn value(&self) -> u8 {
        self.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn gf256_add_is_xor() {
        assert_eq!(gf256::add(0, 0), 0);
        assert_eq!(gf256::add(0xFF, 0xFF), 0);
        assert_eq!(gf256::add(0xAB, 0xCD), 0xAB ^ 0xCD);
    }

    #[test]
    fn gf256_mul_basic() {
        assert_eq!(gf256::mul(0, 5), 0);
        assert_eq!(gf256::mul(1, 5), 5);
        assert_eq!(gf256::mul(5, 1), 5);
        assert_eq!(gf256::mul(2, 2), 4);
    }

    #[test]
    fn gf256_mul_overflow() {
        let result = gf256::mul(0x80, 2);
        assert_eq!(result, 0x1B);
    }

    #[test]
    fn gf256_inverse() {
        for a in 1..=255u8 {
            let inv = gf256::inv(a);
            assert_eq!(gf256::mul(a, inv), 1);
        }
    }

    #[test]
    fn gf256_div() {
        for a in 1..=255u8 {
            assert_eq!(gf256::div(a, a), Some(1));
        }
        for a in 0..=255u8 {
            assert_eq!(gf256::div(a, 1), Some(a));
        }
    }

    #[test]
    fn gf256_div_by_zero_returns_none() {
        assert!(gf256::div(0, 0).is_none());
        assert!(gf256::div(1, 0).is_none());
        assert!(gf256::div(255, 0).is_none());
    }

    #[test]
    fn gf256_pow() {
        assert_eq!(gf256::pow(2, 0), 1);
        assert_eq!(gf256::pow(2, 1), 2);
        assert_eq!(gf256::pow(2, 8), gf256::mul(gf256::pow(2, 7), 2));
    }

    #[test]
    fn shamir_roundtrip_simple() {
        let secret = [42u8; 32];
        let entropy = [0xAA; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");
        let reconstructed =
            ShamirScheme::reconstruct(&shares[0..2], 2).expect("reconstruct failed");
        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn shamir_roundtrip_random_secret() {
        let secret: [u8; 32] = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ];
        let entropy = [0xBB; 32];
        let shares = ShamirScheme::split(&secret, 3, 5, &entropy).expect("split failed");

        let combos: [(usize, usize, usize); 4] = [(0, 1, 2), (0, 2, 4), (1, 3, 4), (2, 3, 4)];

        for (a, b, c) in combos {
            let subset = vec![shares[a].clone(), shares[b].clone(), shares[c].clone()];
            let reconstructed = ShamirScheme::reconstruct(&subset, 3).expect("reconstruct failed");
            assert_eq!(secret, reconstructed);
        }
    }

    #[test]
    fn shamir_threshold_boundary() {
        let secret = [0xAB; 32];
        let entropy = [0xCC; 32];
        let shares = ShamirScheme::split(&secret, 4, 7, &entropy).expect("split failed");

        let reconstructed =
            ShamirScheme::reconstruct(&shares[0..4], 4).expect("reconstruct failed");
        assert_eq!(secret, reconstructed);

        let reconstructed =
            ShamirScheme::reconstruct(&shares[0..6], 4).expect("reconstruct failed");
        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn shamir_different_entropy_different_shares() {
        let secret = [42u8; 32];
        let entropy1 = [0x01; 32];
        let entropy2 = [0x02; 32];
        let shares1 = ShamirScheme::split(&secret, 2, 3, &entropy1).expect("split failed");
        let shares2 = ShamirScheme::split(&secret, 2, 3, &entropy2).expect("split failed");
        assert_ne!(shares1[0].value, shares2[0].value);

        // Both should reconstruct to the same secret
        let r1 = ShamirScheme::reconstruct(&shares1[0..2], 2).expect("reconstruct failed");
        let r2 = ShamirScheme::reconstruct(&shares2[0..2], 2).expect("reconstruct failed");
        assert_eq!(r1, r2);
        assert_eq!(r1, secret);
    }

    #[test]
    fn commitment_verify() {
        let value = 42u64;
        let randomness = [1u8; 32];

        let commitment = PresenceCommitment::new(&value, &randomness);
        assert!(commitment.verify(&value, &randomness));
        assert!(!commitment.verify(&43u64, &randomness));
    }

    #[test]
    fn merkle_proof_single_leaf() {
        let leaf = H256::repeat_byte(0x01);
        let proof = MerkleProof {
            leaf_index: 0,
            siblings: vec![],
        };

        assert!(proof.verify(&leaf, &leaf));
    }

    #[test]
    fn merkle_proof_two_leaves() {
        let left = H256::repeat_byte(0x01);
        let right = H256::repeat_byte(0x02);
        let root = hash_pair(&left, &right);

        let proof_left = MerkleProof {
            leaf_index: 0,
            siblings: vec![right],
        };
        assert!(proof_left.verify(&root, &left));

        let proof_right = MerkleProof {
            leaf_index: 1,
            siblings: vec![left],
        };
        assert!(proof_right.verify(&root, &right));
    }

    #[test]
    fn nullifier_uniqueness() {
        let secret = [42u8; 32];
        let genesis = [0xAA; 32];
        let n1 = Nullifier::derive(&secret, 1, &genesis);
        let n2 = Nullifier::derive(&secret, 2, &genesis);

        // Same secret + same epoch + same chain = same nullifier (INV1)
        let n1_dup = Nullifier::derive(&secret, 1, &genesis);
        assert_eq!(n1, n1_dup);

        // Different epochs = different nullifiers
        assert_ne!(n1, n2);

        // Different secrets = different nullifiers
        let other_secret = [99u8; 32];
        let n3 = Nullifier::derive(&other_secret, 1, &genesis);
        assert_ne!(n1, n3);

        // M12: Different chains = different nullifiers (cross-chain replay prevention)
        let other_genesis = [0xBB; 32];
        let n4 = Nullifier::derive(&secret, 1, &other_genesis);
        assert_ne!(n1, n4);
    }

    #[test]
    fn state_root_empty() {
        let root = StateRoot::from_leaves(&[]);
        assert_eq!(root, StateRoot::EMPTY);
    }

    #[test]
    fn state_root_deterministic() {
        let leaves = vec![
            H256::repeat_byte(0x01),
            H256::repeat_byte(0x02),
            H256::repeat_byte(0x03),
        ];

        let root1 = StateRoot::from_leaves(&leaves);
        let root2 = StateRoot::from_leaves(&leaves);

        assert_eq!(root1, root2);
    }

    #[test]
    fn shamir_split_creates_shares() {
        let secret = [42u8; 32];
        let entropy = [0xDD; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");
        assert_eq!(shares.len(), 3);
        assert_eq!(shares[0].index.0, 1);
        assert_eq!(shares[1].index.0, 2);
        assert_eq!(shares[2].index.0, 3);
    }

    #[test]
    fn shamir_shares_are_different() {
        let secret = [42u8; 32];
        let entropy = [0xEE; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");
        assert_ne!(shares[0].value, shares[1].value);
        assert_ne!(shares[1].value, shares[2].value);
    }

    #[test]
    fn shamir_reconstruct_returns_result() {
        let secret = [42u8; 32];
        let entropy = [0xFF; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");
        let result = ShamirScheme::reconstruct(&shares[0..2], 2);
        assert!(result.is_some());
    }

    #[test]
    fn shamir_reconstruct_deterministic() {
        let secret = [42u8; 32];
        let entropy = [0x11; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");
        let result1 = ShamirScheme::reconstruct(&shares[0..2], 2);
        let result2 = ShamirScheme::reconstruct(&shares[0..2], 2);
        assert_eq!(result1, result2);
    }

    #[test]
    fn shamir_insufficient_shares() {
        let secret = [1u8; 32];
        let entropy = [0x22; 32];
        let shares = ShamirScheme::split(&secret, 3, 5, &entropy).expect("split failed");

        let result = ShamirScheme::reconstruct(&shares[0..2], 3);
        assert!(result.is_none());
    }

    #[test]
    fn shamir_invalid_parameters() {
        let secret = [1u8; 32];
        let entropy = [0x33; 32];
        assert!(ShamirScheme::split(&secret, 1, 3, &entropy).is_none());
        assert!(ShamirScheme::split(&secret, 5, 3, &entropy).is_none());
        assert!(ShamirScheme::split(&secret, 2, 0, &entropy).is_none());
    }

    #[test]
    fn shamir_share_commitment() {
        let secret = [5u8; 32];
        let entropy = [0x44; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");

        let commitment = ShamirScheme::create_commitment(&shares[0]);
        assert!(ShamirScheme::verify_share(&shares[0], &commitment));
        assert!(!ShamirScheme::verify_share(&shares[1], &commitment));
    }

    #[test]
    fn feldman_vss_creates_shares_and_commitments() {
        let secret = [99u8; 32];
        let entropy = [0x55; 32];
        let result = FeldmanVSS::share_with_commitments(&secret, 2, 3, &entropy);
        assert!(result.is_some());

        let (shares, commitments) = result.expect("vss failed");
        assert_eq!(shares.len(), 3);
        // Now stores per-share commitments (one per share)
        assert_eq!(commitments.coefficients.len(), 3);
    }

    #[test]
    fn feldman_vss_verify_share() {
        let secret = [99u8; 32];
        let entropy = [0x66; 32];
        let (shares, commitments) =
            FeldmanVSS::share_with_commitments(&secret, 2, 3, &entropy).expect("vss failed");

        // Each share should verify against its commitment
        for share in &shares {
            assert!(FeldmanVSS::verify_share_against_commitments(
                share,
                &commitments
            ));
        }

        // Tampered share should not verify
        let mut tampered = shares[0].clone();
        tampered.value[0] ^= 0xFF;
        assert!(!FeldmanVSS::verify_share_against_commitments(
            &tampered,
            &commitments
        ));
    }

    #[test]
    fn feldman_vss_invalid_parameters() {
        let secret = [1u8; 32];
        let entropy = [0x77; 32];
        assert!(FeldmanVSS::share_with_commitments(&secret, 1, 3, &entropy).is_none());
        assert!(FeldmanVSS::share_with_commitments(&secret, 5, 3, &entropy).is_none());
    }

    #[test]
    fn feldman_verify_share_count() {
        let secret = [1u8; 32];
        let entropy = [0x88; 32];
        let (shares, _) =
            FeldmanVSS::share_with_commitments(&secret, 3, 5, &entropy).expect("vss failed");

        assert!(FeldmanVSS::verify_share_count(&shares, 3));
        assert!(FeldmanVSS::verify_share_count(&shares, 5));
        assert!(!FeldmanVSS::verify_share_count(&shares[0..2], 3));
    }

    #[test]
    fn share_index_value() {
        let index = ShareIndex(5);
        assert_eq!(index.value(), 5);
    }

    #[test]
    fn share_new() {
        let share = Share::new(3, [7u8; 32]);
        assert_eq!(share.index.0, 3);
        assert_eq!(share.value, [7u8; 32]);
    }

    #[test]
    fn different_secrets_different_shares() {
        let secret1 = [1u8; 32];
        let secret2 = [2u8; 32];
        let entropy = [0x99; 32];

        let shares1 = ShamirScheme::split(&secret1, 2, 3, &entropy).expect("split failed");
        let shares2 = ShamirScheme::split(&secret2, 2, 3, &entropy).expect("split failed");

        assert_ne!(shares1[0].value, shares2[0].value);
    }

    #[test]
    fn share_indices_are_sequential() {
        let secret = [1u8; 32];
        let entropy = [0xAA; 32];
        let shares = ShamirScheme::split(&secret, 2, 5, &entropy).expect("split failed");

        for (i, share) in shares.iter().enumerate() {
            assert_eq!(share.index.0, (i + 1) as u8);
        }
    }

    #[test]
    fn shamir_duplicate_indices_fails() {
        let secret = [42u8; 32];
        let entropy = [0xBB; 32];
        let shares = ShamirScheme::split(&secret, 2, 3, &entropy).expect("split failed");

        let duplicate_shares = vec![
            Share::new(shares[0].index.0, shares[0].value),
            Share::new(shares[0].index.0, shares[1].value),
        ];

        let result = ShamirScheme::reconstruct(&duplicate_shares, 2);
        assert!(result.is_none());
    }

    #[test]
    fn merkle_proof_incomplete_path_rejected() {
        let left = H256::repeat_byte(0x01);
        let right = H256::repeat_byte(0x02);
        let root = hash_pair(&left, &right);

        let proof = MerkleProof {
            leaf_index: 4,
            siblings: vec![right],
        };
        assert!(!proof.verify(&root, &left));
    }

    #[test]
    fn merkle_proof_extra_siblings_accepted_if_root_matches() {
        let leaf = H256::repeat_byte(0x01);

        let proof = MerkleProof {
            leaf_index: 0,
            siblings: vec![],
        };
        assert!(proof.verify(&leaf, &leaf));

        let proof_extra = MerkleProof {
            leaf_index: 0,
            siblings: vec![H256::repeat_byte(0x02)],
        };
        assert!(!proof_extra.verify(&leaf, &leaf));
    }

    #[test]
    fn key_fingerprint_deterministic() {
        let fek = [0xABu8; 32];
        let fp1 = key_fingerprint(&fek);
        let fp2 = key_fingerprint(&fek);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn key_fingerprint_different_keys() {
        let fek1 = [0xABu8; 32];
        let fek2 = [0xCDu8; 32];
        assert_ne!(key_fingerprint(&fek1), key_fingerprint(&fek2));
    }

    #[test]
    fn vault_domain_separators_unique() {
        let data = [42u8; 32];
        let h_fek = hash_with_domain(DOMAIN_VAULT_FEK, &data);
        let h_file = hash_with_domain(DOMAIN_VAULT_FILE, &data);
        let h_unlock = hash_with_domain(DOMAIN_UNLOCK, &data);
        let h_share = hash_with_domain(DOMAIN_SHARE, &data);
        assert_ne!(h_fek, h_file);
        assert_ne!(h_fek, h_unlock);
        assert_ne!(h_fek, h_share);
        assert_ne!(h_file, h_unlock);
        assert_ne!(h_file, h_share);
        assert_ne!(h_unlock, h_share);
    }

    // =========================================================================
    // SHA-256 NIST Dual-Hash Layer Tests (v0.9.0)
    // =========================================================================

    #[test]
    fn sha256_with_domain_deterministic() {
        let data = b"test data";
        let h1 = sha256_with_domain(NIST_DOMAIN_PRESENCE, data);
        let h2 = sha256_with_domain(NIST_DOMAIN_PRESENCE, data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn sha256_with_domain_different_domains() {
        let data = b"same data";
        let h1 = sha256_with_domain(NIST_DOMAIN_PRESENCE, data);
        let h2 = sha256_with_domain(NIST_DOMAIN_COMMITMENT, data);
        assert_ne!(h1, h2);
    }

    #[test]
    fn sha256_with_domain_different_data() {
        let h1 = sha256_with_domain(NIST_DOMAIN_PRESENCE, b"data1");
        let h2 = sha256_with_domain(NIST_DOMAIN_PRESENCE, b"data2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn sha256_differs_from_blake2() {
        let data = b"cross-hash check";
        let blake = hash_with_domain(DOMAIN_PRESENCE, data);
        let sha = sha256_with_domain(NIST_DOMAIN_PRESENCE, data);
        assert_ne!(blake, sha);
    }

    #[test]
    fn sha256_raw_deterministic() {
        let data = b"raw hash test";
        let h1 = sha256_raw(data);
        let h2 = sha256_raw(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn sha256_raw_known_vector() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let empty_hash = sha256_raw(b"");
        assert_eq!(empty_hash[0], 0xe3);
        assert_eq!(empty_hash[1], 0xb0);
        assert_eq!(empty_hash[31], 0x55);
    }

    #[test]
    fn sha256_hash_pair_deterministic() {
        let left = H256::repeat_byte(0x01);
        let right = H256::repeat_byte(0x02);
        let h1 = sha256_hash_pair(&left, &right);
        let h2 = sha256_hash_pair(&left, &right);
        assert_eq!(h1, h2);
    }

    #[test]
    fn sha256_hash_pair_order_matters() {
        let a = H256::repeat_byte(0x01);
        let b = H256::repeat_byte(0x02);
        let h1 = sha256_hash_pair(&a, &b);
        let h2 = sha256_hash_pair(&b, &a);
        assert_ne!(h1, h2);
    }

    #[test]
    fn sha256_hash_pair_differs_from_blake2_pair() {
        let left = H256::repeat_byte(0x0A);
        let right = H256::repeat_byte(0x0B);
        let blake = hash_pair(&left, &right);
        let sha = sha256_hash_pair(&left, &right);
        assert_ne!(blake, sha);
    }

    #[test]
    fn nist_key_fingerprint_deterministic() {
        let key = [0xABu8; 32];
        let fp1 = nist_key_fingerprint(&key);
        let fp2 = nist_key_fingerprint(&key);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn nist_key_fingerprint_different_keys() {
        let key1 = [0xABu8; 32];
        let key2 = [0xCDu8; 32];
        assert_ne!(nist_key_fingerprint(&key1), nist_key_fingerprint(&key2));
    }

    #[test]
    fn nist_key_fingerprint_differs_from_blake2() {
        let key = [0xABu8; 32];
        let blake = key_fingerprint(&key);
        let nist = nist_key_fingerprint(&key);
        assert_ne!(blake, nist);
    }

    #[test]
    fn nist_domain_separators_unique() {
        let data = [42u8; 32];
        let h_presence = sha256_with_domain(NIST_DOMAIN_PRESENCE, &data);
        let h_commit = sha256_with_domain(NIST_DOMAIN_COMMITMENT, &data);
        let h_merkle = sha256_with_domain(NIST_DOMAIN_MERKLE, &data);
        let h_nullifier = sha256_with_domain(NIST_DOMAIN_NULLIFIER, &data);
        let h_fingerprint = sha256_with_domain(NIST_DOMAIN_FINGERPRINT, &data);

        let all = [h_presence, h_commit, h_merkle, h_nullifier, h_fingerprint];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j]);
            }
        }
    }

    #[test]
    fn sha256_domain_length_prefix_prevents_ambiguity() {
        // "ab" + "cd" should differ from "a" + "bcd"
        let h1 = sha256_with_domain(b"ab", b"cd");
        let h2 = sha256_with_domain(b"a", b"bcd");
        assert_ne!(h1, h2);
    }

    #[test]
    fn share_knowledge_verifier_fails_closed_for_generated_proof() {
        let share = Share::new(1, [0x42; 32]);
        let entropy = [0xAB; 32];
        let proof = prove_share_knowledge(&share, &entropy);
        let commitment = ShamirScheme::hash_share(&share);

        assert!(!verify_share_knowledge(&proof, share.index, &commitment));
    }

    #[test]
    fn share_knowledge_verifier_rejects_degenerate_inputs() {
        let proof = ShareKnowledgeProof {
            commitment_r: H256::zero(),
            response: [0u8; 32],
        };

        assert!(!verify_share_knowledge(
            &proof,
            ShareIndex(0),
            &H256::repeat_byte(0x11)
        ));
    }
}
