//! EC-based Feldman Verifiable Secret Sharing using BN254 G1 points.
//!
//! Replaces the hash-based VSS in `seveny-primitives` (C07) with proper
//! elliptic-curve commitments that enable polynomial consistency verification.
//!
//! # Scheme
//!
//! A dealer splits a secret `s` into `n` shares with threshold `t`:
//!
//! 1. Choose random polynomial `f(x) = a_0 + a_1*x + ... + a_{t-1}*x^{t-1}`
//!    where `a_0 = s` (the secret as a BN254 scalar).
//! 2. Compute commitments: `C_i = a_i * G` for each coefficient.
//! 3. Compute shares: `s_j = f(j)` for each participant `j` in `[1..n]`.
//! 4. Publish commitments `[C_0, ..., C_{t-1}]` on-chain.
//!
//! # Verification
//!
//! Any participant `j` can verify their share `s_j` against the public
//! commitments using the homomorphic property:
//!
//! ```text
//! s_j * G == C_0 + j*C_1 + j^2*C_2 + ... + j^{t-1}*C_{t-1}
//! ```
//!
//! This detects a malicious dealer who distributes inconsistent shares
//! (unlike hash-based VSS which only detects individual share tampering).
//!
//! # Security (INV69)
//!
//! - Hiding: Commitments are EC points; recovering `a_i` requires solving DLP.
//! - Binding: The polynomial is uniquely determined by the commitments.
//! - Consistency: Each share is verifiable against the same polynomial.
//! - Dealer corruption detection: Inconsistent shares are detectable by any participant.

use alloc::vec::Vec;
use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::{Field, PrimeField, Zero};
use sp_core::blake2_256;

/// Compressed G1 point size (arkworks BN254).
pub const G1_COMPRESSED_SIZE: usize = 32;

/// EC-based commitment to polynomial coefficients.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcVssCommitment {
    /// Commitments C_i = a_i * G for each polynomial coefficient.
    pub points: Vec<G1Affine>,
}

/// An EC-VSS share: evaluation of the polynomial at a specific index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcVssShare {
    /// Share index (1-based, corresponds to evaluation point).
    pub index: u8,
    /// Share value: f(index) as a BN254 scalar.
    pub value: Fr,
}

/// EC-based Feldman VSS using BN254 G1 curve.
pub struct EcFeldmanVSS;

impl EcFeldmanVSS {
    /// Split a secret into shares with EC-based polynomial commitments.
    ///
    /// `secret_bytes`: 32-byte secret (interpreted as BN254 scalar via mod reduction)
    /// `threshold`: minimum shares needed for reconstruction
    /// `total`: total number of shares to create
    /// `entropy`: randomness seed for polynomial coefficient generation
    ///
    /// Returns `(shares, commitment)` or `None` if parameters are invalid.
    pub fn share_with_commitments(
        secret_bytes: &[u8; 32],
        threshold: u8,
        total: u8,
        entropy: &[u8; 32],
    ) -> Option<(Vec<EcVssShare>, EcVssCommitment)> {
        if threshold < 2 || total < threshold || total == 0 {
            return None;
        }

        let secret = Fr::from_be_bytes_mod_order(secret_bytes);
        let generator = G1Projective::generator();

        // Build polynomial coefficients: a_0 = secret, a_1..a_{t-1} random
        let mut coefficients = Vec::with_capacity(threshold as usize);
        coefficients.push(secret);

        for i in 1..threshold {
            let seed_input = [&entropy[..], &[i][..], b"7ay:ec-vss:coeff:v1"].concat();
            let hash = blake2_256(&seed_input);
            coefficients.push(Fr::from_be_bytes_mod_order(&hash));
        }

        // Compute EC commitments: C_i = a_i * G
        let points: Vec<G1Affine> = coefficients
            .iter()
            .map(|a| (generator * a).into_affine())
            .collect();

        // Compute shares: s_j = f(j) for j in 1..=total
        let shares: Vec<EcVssShare> = (1..=total)
            .map(|j| {
                let x = Fr::from(j as u64);
                let value = eval_polynomial_fr(&coefficients, x);
                EcVssShare { index: j, value }
            })
            .collect();

        Some((shares, EcVssCommitment { points }))
    }

    /// Verify a share against the public EC commitments.
    ///
    /// Checks: `s_j * G == C_0 + j*C_1 + j^2*C_2 + ... + j^{t-1}*C_{t-1}`
    ///
    /// This verifies polynomial consistency, detecting a malicious dealer.
    pub fn verify_share(share: &EcVssShare, commitment: &EcVssCommitment) -> bool {
        if share.index == 0 || commitment.points.is_empty() {
            return false;
        }

        let generator = G1Projective::generator();

        // LHS: share_value * G
        let lhs = generator * share.value;

        // RHS: SUM(C_i * j^i)
        let j = Fr::from(share.index as u64);
        let mut rhs = G1Projective::zero();
        let mut j_pow = Fr::from(1u64);

        for point in &commitment.points {
            let point_proj: G1Projective = (*point).into();
            rhs += point_proj * j_pow;
            j_pow *= j;
        }

        lhs == rhs
    }

    /// Reconstruct the secret from `threshold` shares using Lagrange interpolation.
    ///
    /// Operates in BN254 Fr (scalar field), not GF(2^8).
    pub fn reconstruct(shares: &[EcVssShare], threshold: u8) -> Option<Fr> {
        if shares.len() < threshold as usize {
            return None;
        }

        let subset = &shares[..threshold as usize];
        let mut secret = Fr::zero();

        for (i, share_i) in subset.iter().enumerate() {
            let xi = Fr::from(share_i.index as u64);
            let yi = share_i.value;

            // Lagrange basis polynomial evaluated at 0
            let mut li = Fr::from(1u64);
            for (j, share_j) in subset.iter().enumerate() {
                if i != j {
                    let xj = Fr::from(share_j.index as u64);
                    let denom = xj - xi;
                    if denom.is_zero() {
                        return None; // Duplicate indices
                    }
                    li *= xj * denom.inverse()?;
                }
            }

            secret += yi * li;
        }

        Some(secret)
    }

    /// Check that a set of shares has enough for reconstruction.
    pub fn verify_share_count(shares: &[EcVssShare], threshold: u8) -> bool {
        shares.len() >= threshold as usize
    }
}

/// Evaluate polynomial at point x over BN254 Fr.
fn eval_polynomial_fr(coefficients: &[Fr], x: Fr) -> Fr {
    let mut result = Fr::zero();
    let mut x_pow = Fr::from(1u64);
    for coeff in coefficients {
        result += *coeff * x_pow;
        x_pow *= x;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> [u8; 32] {
        [42u8; 32]
    }

    fn test_entropy() -> [u8; 32] {
        [0xAAu8; 32]
    }

    #[test]
    fn ec_vss_creates_shares_and_commitments() {
        let (shares, commitment) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 2, 3, &test_entropy())
                .expect("share creation failed");

        assert_eq!(shares.len(), 3);
        assert_eq!(commitment.points.len(), 2); // threshold = 2 coefficients
    }

    #[test]
    fn ec_vss_shares_verify_against_commitments() {
        let (shares, commitment) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 3, 5, &test_entropy())
                .expect("share creation failed");

        for share in &shares {
            assert!(
                EcFeldmanVSS::verify_share(share, &commitment),
                "share {} should verify",
                share.index
            );
        }
    }

    #[test]
    fn ec_vss_detects_tampered_share() {
        let (mut shares, commitment) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 2, 3, &test_entropy())
                .expect("share creation failed");

        // Tamper with share value
        shares[0].value += Fr::from(1u64);

        assert!(
            !EcFeldmanVSS::verify_share(&shares[0], &commitment),
            "tampered share should fail verification"
        );

        // Other shares should still verify
        assert!(EcFeldmanVSS::verify_share(&shares[1], &commitment));
        assert!(EcFeldmanVSS::verify_share(&shares[2], &commitment));
    }

    #[test]
    fn ec_vss_detects_inconsistent_dealer() {
        let (shares1, _) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 2, 3, &test_entropy())
                .expect("share creation failed");

        // Dealer creates a different polynomial (different entropy)
        let (_, commitment2) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 2, 3, &[0xBBu8; 32])
                .expect("share creation failed");

        // Shares from polynomial 1 should NOT verify against commitments from polynomial 2
        for share in &shares1 {
            assert!(
                !EcFeldmanVSS::verify_share(share, &commitment2),
                "share from different polynomial should fail"
            );
        }
    }

    #[test]
    fn ec_vss_reconstruct_secret() {
        let secret_bytes = test_secret();
        let secret_fr = Fr::from_be_bytes_mod_order(&secret_bytes);

        let (shares, _) =
            EcFeldmanVSS::share_with_commitments(&secret_bytes, 3, 5, &test_entropy())
                .expect("share creation failed");

        // Reconstruct from threshold shares
        let reconstructed =
            EcFeldmanVSS::reconstruct(&shares[0..3], 3).expect("reconstruction failed");
        assert_eq!(reconstructed, secret_fr);

        // Reconstruct from different subset
        let subset = vec![shares[0].clone(), shares[2].clone(), shares[4].clone()];
        let reconstructed2 = EcFeldmanVSS::reconstruct(&subset, 3).expect("reconstruction failed");
        assert_eq!(reconstructed2, secret_fr);
    }

    #[test]
    fn ec_vss_reconstruct_insufficient_shares() {
        let (shares, _) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 3, 5, &test_entropy())
                .expect("share creation failed");

        // 2 shares for threshold 3 should fail
        let result = EcFeldmanVSS::reconstruct(&shares[0..2], 3);
        assert!(result.is_none());
    }

    #[test]
    fn ec_vss_reconstruct_duplicate_indices_fails() {
        let (shares, _) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 2, 3, &test_entropy())
                .expect("share creation failed");

        let duplicates = vec![
            EcVssShare {
                index: shares[0].index,
                value: shares[0].value,
            },
            EcVssShare {
                index: shares[0].index, // same index
                value: shares[1].value,
            },
        ];

        let result = EcFeldmanVSS::reconstruct(&duplicates, 2);
        assert!(result.is_none());
    }

    #[test]
    fn ec_vss_invalid_parameters() {
        let secret = test_secret();
        let entropy = test_entropy();

        assert!(EcFeldmanVSS::share_with_commitments(&secret, 1, 3, &entropy).is_none());
        assert!(EcFeldmanVSS::share_with_commitments(&secret, 5, 3, &entropy).is_none());
        assert!(EcFeldmanVSS::share_with_commitments(&secret, 2, 0, &entropy).is_none());
    }

    #[test]
    fn ec_vss_different_entropy_different_shares() {
        let secret = test_secret();
        let (shares1, commitment1) =
            EcFeldmanVSS::share_with_commitments(&secret, 2, 3, &[0x01u8; 32])
                .expect("share creation failed");
        let (shares2, commitment2) =
            EcFeldmanVSS::share_with_commitments(&secret, 2, 3, &[0x02u8; 32])
                .expect("share creation failed");

        // Different entropy = different shares and commitments
        assert_ne!(shares1[0].value, shares2[0].value);
        assert_ne!(commitment1.points[1], commitment2.points[1]);

        // But same secret (C_0 = secret * G should be the same)
        assert_eq!(commitment1.points[0], commitment2.points[0]);

        // Both should reconstruct to the same secret
        let r1 = EcFeldmanVSS::reconstruct(&shares1[0..2], 2).expect("reconstruct");
        let r2 = EcFeldmanVSS::reconstruct(&shares2[0..2], 2).expect("reconstruct");
        assert_eq!(r1, r2);
    }

    #[test]
    fn ec_vss_commitment_binding_same_secret() {
        let secret = test_secret();
        let (_, c1) = EcFeldmanVSS::share_with_commitments(&secret, 2, 3, &[0x01; 32])
            .expect("share creation failed");
        let (_, c2) = EcFeldmanVSS::share_with_commitments(&secret, 2, 3, &[0x02; 32])
            .expect("share creation failed");

        // C_0 = secret * G is identical for the same secret
        assert_eq!(c1.points[0], c2.points[0]);
    }

    #[test]
    fn ec_vss_zero_index_share_rejected() {
        let (_, commitment) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 2, 3, &test_entropy())
                .expect("share creation failed");

        let zero_share = EcVssShare {
            index: 0,
            value: Fr::from(1u64),
        };

        assert!(!EcFeldmanVSS::verify_share(&zero_share, &commitment));
    }

    #[test]
    fn ec_vss_verify_share_count() {
        let (shares, _) =
            EcFeldmanVSS::share_with_commitments(&test_secret(), 3, 5, &test_entropy())
                .expect("share creation failed");

        assert!(EcFeldmanVSS::verify_share_count(&shares, 3));
        assert!(EcFeldmanVSS::verify_share_count(&shares, 5));
        assert!(!EcFeldmanVSS::verify_share_count(&shares[0..2], 3));
    }
}
