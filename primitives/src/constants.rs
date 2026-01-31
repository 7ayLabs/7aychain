//! Protocol constants derived from the PoP specification (INV1-78).

use sp_arithmetic::Perbill;

// Validator Economics (INV46-49)
pub const MIN_VALIDATORS: u32 = 5;
pub const MAX_STAKE_RATIO: Perbill = Perbill::from_percent(33);
pub const DEFAULT_MIN_STAKE: u128 = 10_000;

// Slashing (INV48)
pub const SLASH_MINOR: Perbill = Perbill::from_percent(5);
pub const SLASH_MODERATE: Perbill = Perbill::from_percent(20);
pub const SLASH_SEVERE: Perbill = Perbill::from_percent(50);
pub const SLASH_CRITICAL: Perbill = Perbill::from_percent(100);

// Evidence Rewards (INV49)
pub const EVIDENCE_REWARD_RATIO: Perbill = Perbill::from_percent(10);
pub const EVIDENCE_REWARD_MAX: u128 = 1_000;

// Recovery & Governance (INV57-60)
pub const RECOVERY_QUORUM: Perbill = Perbill::from_percent(80);
pub const RECOVERY_COOLDOWN_DAYS: u32 = 7;
pub const UPGRADE_DELAY_PARAM_HOURS: u32 = 48;
pub const UPGRADE_DELAY_PROTOCOL_DAYS: u32 = 7;
pub const EMERGENCY_UPGRADE_QUORUM: Perbill = Perbill::from_percent(80);

// Security (INV43-45)
pub const KEY_DESTRUCTION_TIMEOUT_SECS: u64 = 300;
pub const KEY_DESTRUCTION_MIN_ATTESTATIONS: u32 = 3;
pub const DISCOVERY_RATE_LIMIT_PER_MIN: u32 = 60;

// Octopus Scaling (INV38-42, INV63)
pub const OCTOPUS_ACTIVATION_THRESHOLD: Perbill = Perbill::from_percent(45);
pub const OCTOPUS_DEACTIVATION_THRESHOLD: Perbill = Perbill::from_percent(20);
pub const OCTOPUS_DEACTIVATION_DURATION_SECS: u64 = 300;
pub const OCTOPUS_MAX_SUBNODES: u32 = 8;
pub const OCTOPUS_SUBNODE_DIVISOR: u32 = 225; // 22.5 * 10

// Vault (INV66-68)
pub const VAULT_MIN_THRESHOLD: u32 = 2;
pub const VAULT_MIN_RING_SIZE: u32 = 3;
pub const VAULT_MAX_RING_SIZE: u32 = 10;

// Reputation (INV50-53)
pub const REPUTATION_MIN: u32 = 0;
pub const REPUTATION_MAX: u32 = 100;
pub const REPUTATION_INITIAL: u32 = 50;
pub const REPUTATION_PENALTY_MINOR: u32 = 5;
pub const REPUTATION_PENALTY_MODERATE: u32 = 10;
pub const REPUTATION_PENALTY_SEVERE: u32 = 20;
pub const COOLDOWN_MAX_HOURS: u32 = 24;

// Small Network (INV54-56)
pub const SMALL_NETWORK_THRESHOLD: u32 = 5;

// Boomerang (INV30-33)
pub const BOOMERANG_TIMEOUT_SECS: u64 = 30;
pub const BOOMERANG_MAX_EXTENSION_SECS: u64 = 60;

// Autonomous (INV34-37)
pub const AUTONOMOUS_PATTERN_THRESHOLD: u32 = 3;

/// Calculate max sub-nodes based on throughput (INV63).
#[inline]
pub const fn max_subnodes(throughput_pct: u32) -> u32 {
    let scaled = throughput_pct.saturating_mul(10);
    let result =
        scaled.saturating_add(OCTOPUS_SUBNODE_DIVISOR.saturating_sub(1)) / OCTOPUS_SUBNODE_DIVISOR;
    if result > OCTOPUS_MAX_SUBNODES {
        OCTOPUS_MAX_SUBNODES
    } else if result == 0 {
        1
    } else {
        result
    }
}

/// Calculate slash amount from stake and percentage.
#[inline]
pub const fn slash_amount(stake: u128, pct: u8) -> u128 {
    stake.saturating_mul(pct as u128) / 100
}

/// Calculate evidence reward with caps (INV49).
#[inline]
pub const fn evidence_reward(slash: u128) -> u128 {
    let reward = slash.saturating_mul(10) / 100;
    if reward > EVIDENCE_REWARD_MAX {
        EVIDENCE_REWARD_MAX
    } else {
        reward
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_subnodes() {
        assert_eq!(max_subnodes(45), 2);
        assert_eq!(max_subnodes(100), 5);
        assert_eq!(max_subnodes(200), 8);
        assert_eq!(max_subnodes(0), 1);
    }

    #[test]
    fn test_slash_amount() {
        assert_eq!(slash_amount(10_000, 5), 500);
        assert_eq!(slash_amount(10_000, 100), 10_000);
    }

    #[test]
    fn test_evidence_reward() {
        assert_eq!(evidence_reward(5_000), 500);
        assert_eq!(evidence_reward(100_000), EVIDENCE_REWARD_MAX);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_constants_valid() {
        assert!(RECOVERY_QUORUM <= Perbill::from_percent(100));
        assert!(VAULT_MIN_THRESHOLD <= VAULT_MIN_RING_SIZE);
        assert!(REPUTATION_INITIAL <= REPUTATION_MAX);
    }
}
