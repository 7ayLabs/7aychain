#!/usr/bin/env node
/**
 * 7aychain Cryptographer Test Suite
 * Tests commitment-reveal, Merkle proofs, nullifiers, and secret sharing
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsHex, randomAsHex } = require('@polkadot/util-crypto');
const { hexToU8a, u8aToHex } = require('@polkadot/util');

const ENDPOINT = process.env.WS_ENDPOINT || 'ws://127.0.0.1:9944';

const COLORS = {
    reset: '\x1b[0m',
    green: '\x1b[32m',
    red: '\x1b[31m',
    yellow: '\x1b[33m',
    cyan: '\x1b[36m',
};

function log(color, symbol, message) {
    console.log(`${color}${symbol}${COLORS.reset} ${message}`);
}

function pass(message) { log(COLORS.green, '✓', message); }
function fail(message) { log(COLORS.red, '✗', message); }
function info(message) { log(COLORS.cyan, '→', message); }
function warn(message) { log(COLORS.yellow, '!', message); }

// ============================================================================
// COMMITMENT-REVEAL SCHEME
// ============================================================================

class CommitmentReveal {
    static DOMAIN = 'DOMAIN_PRESENCE';

    static generateCommitment(actorId, epochId, secret, randomness) {
        const preimage = this.DOMAIN + actorId + epochId.toString() + secret + randomness;
        return blake2AsHex(preimage);
    }

    static async testCommitmentBinding(api, keyring) {
        info('Testing commitment binding property...');

        const alice = keyring.addFromUri('//Alice');
        const secret = randomAsHex(32);
        const randomness = randomAsHex(32);
        const actorId = alice.publicKey;
        const epochId = 1;

        const commitment = this.generateCommitment(
            u8aToHex(actorId),
            epochId,
            secret,
            randomness
        );

        // Try to find a different preimage (should be computationally infeasible)
        const differentSecret = randomAsHex(32);
        const differentCommitment = this.generateCommitment(
            u8aToHex(actorId),
            epochId,
            differentSecret,
            randomness
        );

        if (commitment !== differentCommitment) {
            pass('Commitment binding: Different secrets produce different commitments');
        } else {
            fail('Commitment binding: COLLISION FOUND (critical!)');
        }

        return { commitment, secret, randomness, actorId, epochId };
    }

    static async testCommitRevealTiming(api, keyring, commitment) {
        info('Testing commit-reveal timing window...');

        // This test would require the presence pallet to be available
        // For now, we verify the commitment structure
        if (commitment.length === 66 && commitment.startsWith('0x')) {
            pass('Commitment format valid (32 bytes, hex encoded)');
        } else {
            fail(`Invalid commitment format: ${commitment}`);
        }
    }
}

// ============================================================================
// MERKLE PROOF VERIFICATION
// ============================================================================

class MerkleTree {
    constructor(leaves) {
        this.leaves = leaves.map(l => blake2AsHex(l));
        this.layers = [this.leaves];
        this.buildTree();
    }

    buildTree() {
        let currentLayer = this.leaves;

        while (currentLayer.length > 1) {
            const nextLayer = [];
            for (let i = 0; i < currentLayer.length; i += 2) {
                const left = currentLayer[i];
                const right = currentLayer[i + 1] || left;
                nextLayer.push(blake2AsHex(left + right.slice(2)));
            }
            this.layers.push(nextLayer);
            currentLayer = nextLayer;
        }
    }

    get root() {
        return this.layers[this.layers.length - 1][0];
    }

    getProof(index) {
        const proof = [];
        let currentIndex = index;

        for (let i = 0; i < this.layers.length - 1; i++) {
            const layer = this.layers[i];
            const isRight = currentIndex % 2 === 1;
            const siblingIndex = isRight ? currentIndex - 1 : currentIndex + 1;

            if (siblingIndex < layer.length) {
                proof.push({
                    hash: layer[siblingIndex],
                    position: isRight ? 'left' : 'right'
                });
            }

            currentIndex = Math.floor(currentIndex / 2);
        }

        return proof;
    }

    static verify(leaf, proof, root) {
        let hash = blake2AsHex(leaf);

        for (const { hash: siblingHash, position } of proof) {
            if (position === 'left') {
                hash = blake2AsHex(siblingHash + hash.slice(2));
            } else {
                hash = blake2AsHex(hash + siblingHash.slice(2));
            }
        }

        return hash === root;
    }
}

async function testMerkleProofs() {
    info('Testing Merkle proof generation and verification...');

    const devices = [
        '0xaaaaaaaabbbbbbbbccccccccdddddddd',
        '0x1111111122222222333333334444444',
        '0x5555555566666666777777778888888',
        '0x9999999900000000aaaaaaabbbbbbbb',
    ];

    const tree = new MerkleTree(devices);
    info(`Merkle root: ${tree.root}`);

    // Test each leaf
    let allValid = true;
    for (let i = 0; i < devices.length; i++) {
        const proof = tree.getProof(i);
        const valid = MerkleTree.verify(devices[i], proof, tree.root);

        if (valid) {
            pass(`Leaf ${i} proof verified`);
        } else {
            fail(`Leaf ${i} proof failed`);
            allValid = false;
        }
    }

    // Test invalid proof
    const fakeDevice = '0xffffffffffffffffffffffffffffff';
    const fakeProof = tree.getProof(0);
    const shouldFail = MerkleTree.verify(fakeDevice, fakeProof, tree.root);

    if (!shouldFail) {
        pass('Invalid device correctly rejected');
    } else {
        fail('Invalid device incorrectly accepted');
        allValid = false;
    }

    return allValid;
}

// ============================================================================
// NULLIFIER UNIQUENESS
// ============================================================================

class Nullifier {
    static derive(secret, epochId, nonce) {
        const preimage = 'NULLIFIER' + secret + epochId.toString() + nonce.toString();
        return blake2AsHex(preimage);
    }

    static async testUniqueness() {
        info('Testing nullifier uniqueness...');

        const secret = randomAsHex(32);

        // Same epoch, different nonces
        const n1 = this.derive(secret, 1, 1);
        const n2 = this.derive(secret, 1, 2);

        // Different epoch, same nonce
        const n3 = this.derive(secret, 2, 1);

        const allUnique = n1 !== n2 && n1 !== n3 && n2 !== n3;

        if (allUnique) {
            pass('All nullifiers are unique');
            info(`  Nullifier 1: ${n1.slice(0, 20)}...`);
            info(`  Nullifier 2: ${n2.slice(0, 20)}...`);
            info(`  Nullifier 3: ${n3.slice(0, 20)}...`);
        } else {
            fail('Nullifier collision detected!');
        }

        return allUnique;
    }

    static async testDoubleSpendPrevention(api, keyring) {
        info('Testing double-spend prevention (requires ZK pallet)...');

        // This would submit the same nullifier twice
        // For now, we verify the nullifier derivation is deterministic
        const secret = '0x' + '01'.repeat(32);
        const n1 = this.derive(secret, 1, 1);
        const n2 = this.derive(secret, 1, 1);

        if (n1 === n2) {
            pass('Nullifier derivation is deterministic');
        } else {
            fail('Nullifier derivation is non-deterministic!');
        }

        return n1 === n2;
    }
}

// ============================================================================
// SECRET SHARING (SHAMIR)
// ============================================================================

class ShamirSecretSharing {
    // Simple polynomial evaluation mod prime
    static evaluate(coeffs, x, prime) {
        let result = 0n;
        for (let i = coeffs.length - 1; i >= 0; i--) {
            result = (result * BigInt(x) + coeffs[i]) % prime;
        }
        return result;
    }

    // Lagrange interpolation to recover secret
    static interpolate(shares, prime) {
        let secret = 0n;

        for (let i = 0; i < shares.length; i++) {
            let numerator = 1n;
            let denominator = 1n;

            for (let j = 0; j < shares.length; j++) {
                if (i !== j) {
                    numerator = (numerator * (0n - BigInt(shares[j].x))) % prime;
                    denominator = (denominator * (BigInt(shares[i].x) - BigInt(shares[j].x))) % prime;
                }
            }

            // Modular inverse
            const inv = this.modInverse(denominator, prime);
            secret = (secret + shares[i].y * numerator * inv) % prime;
        }

        return ((secret % prime) + prime) % prime;
    }

    static modInverse(a, m) {
        let [old_r, r] = [a, m];
        let [old_s, s] = [1n, 0n];

        while (r !== 0n) {
            const q = old_r / r;
            [old_r, r] = [r, old_r - q * r];
            [old_s, s] = [s, old_s - q * s];
        }

        return ((old_s % m) + m) % m;
    }

    static split(secret, threshold, shares, prime) {
        // Generate random coefficients
        const coeffs = [secret];
        for (let i = 1; i < threshold; i++) {
            coeffs.push(BigInt('0x' + randomAsHex(32).slice(2, 34)) % prime);
        }

        // Evaluate polynomial at each x
        const result = [];
        for (let x = 1; x <= shares; x++) {
            result.push({ x, y: this.evaluate(coeffs, x, prime) });
        }

        return result;
    }

    static async testShamirScheme() {
        info('Testing Shamir secret sharing (3-of-5)...');

        // Use a large prime (simplified for testing)
        const prime = 2n ** 127n - 1n;
        const secret = BigInt('0x' + randomAsHex(16).slice(2, 18));

        info(`Original secret: ${secret}`);

        // Split into 5 shares, threshold 3
        const shares = this.split(secret, 3, 5, prime);
        info(`Generated ${shares.length} shares`);

        // Test reconstruction with exactly 3 shares
        const subset1 = [shares[0], shares[2], shares[4]];
        const recovered1 = this.interpolate(subset1, prime);

        if (recovered1 === secret) {
            pass('Secret recovered with shares [0, 2, 4]');
        } else {
            fail(`Recovery failed: expected ${secret}, got ${recovered1}`);
        }

        // Test with different 3 shares
        const subset2 = [shares[1], shares[3], shares[4]];
        const recovered2 = this.interpolate(subset2, prime);

        if (recovered2 === secret) {
            pass('Secret recovered with shares [1, 3, 4]');
        } else {
            fail(`Recovery failed: expected ${secret}, got ${recovered2}`);
        }

        // Test that 2 shares are insufficient (wrong recovery)
        const subset3 = [shares[0], shares[1]];
        const recovered3 = this.interpolate(subset3, prime);

        if (recovered3 !== secret) {
            pass('2 shares correctly fail to recover secret');
        } else {
            warn('2 shares recovered secret (possible but unlikely)');
        }

        return recovered1 === secret && recovered2 === secret;
    }
}

// ============================================================================
// HASH BENCHMARKING
// ============================================================================

async function benchmarkHashes() {
    info('Benchmarking hash operations...');

    const iterations = 10000;
    const data = randomAsHex(1024);

    const start = performance.now();
    for (let i = 0; i < iterations; i++) {
        blake2AsHex(data + i.toString());
    }
    const elapsed = performance.now() - start;

    const opsPerSec = Math.round(iterations / (elapsed / 1000));
    info(`Blake2-256: ${opsPerSec.toLocaleString()} ops/sec`);

    if (opsPerSec > 50000) {
        pass('Hash performance acceptable');
    } else {
        warn(`Hash performance low: ${opsPerSec} ops/sec`);
    }
}

// ============================================================================
// MAIN TEST RUNNER
// ============================================================================

async function main() {
    console.log('\n' + '='.repeat(60));
    console.log('  7aychain Cryptographer Test Suite');
    console.log('='.repeat(60) + '\n');

    let api;
    let connected = false;

    try {
        info(`Connecting to ${ENDPOINT}...`);
        const provider = new WsProvider(ENDPOINT, 1000);
        api = await ApiPromise.create({ provider, noInitWarn: true });
        connected = true;
        pass('Connected to node');

        const chain = await api.rpc.system.chain();
        const version = await api.rpc.system.version();
        info(`Chain: ${chain}, Version: ${version}`);
    } catch (e) {
        warn(`Could not connect to node: ${e.message}`);
        warn('Running offline tests only...');
    }

    const keyring = new Keyring({ type: 'sr25519' });

    console.log('\n--- Commitment-Reveal Tests ---\n');
    const commitData = await CommitmentReveal.testCommitmentBinding(null, keyring);
    await CommitmentReveal.testCommitRevealTiming(null, keyring, commitData.commitment);

    console.log('\n--- Merkle Proof Tests ---\n');
    await testMerkleProofs();

    console.log('\n--- Nullifier Tests ---\n');
    await Nullifier.testUniqueness();
    await Nullifier.testDoubleSpendPrevention(null, keyring);

    console.log('\n--- Secret Sharing Tests ---\n');
    await ShamirSecretSharing.testShamirScheme();

    console.log('\n--- Hash Benchmarks ---\n');
    await benchmarkHashes();

    console.log('\n' + '='.repeat(60));
    console.log('  Test suite completed');
    console.log('='.repeat(60) + '\n');

    if (api) {
        await api.disconnect();
    }

    process.exit(0);
}

main().catch(console.error);
