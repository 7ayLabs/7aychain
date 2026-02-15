#!/usr/bin/env node
/**
 * Presence-Based Triangulation (PBT) Test Script
 *
 * Tests the new PBT architecture:
 * 1. Validators set their positions
 * 2. Actors claim positions
 * 3. Validators submit witness attestations
 * 4. Positions get verified through triangulation
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

const WS_ENDPOINT = process.env.WS_ENDPOINT || 'ws://127.0.0.1:9944';

// Test positions (in meters, centered around origin)
const VALIDATOR_POSITIONS = {
    alice: { x: 0, y: 0, z: 0 },           // Origin
    bob: { x: 50000, y: 0, z: 0 },         // 50km east
    charlie: { x: 25000, y: 43301, z: 0 }, // 50km northeast (equilateral triangle)
};

// Simulated latencies (in milliseconds) based on distances
// ~150km per ms, so 50km = ~0.33ms, but add network overhead
const SIMULATED_LATENCIES = {
    // Alice measuring others
    alice_to_bob: 5,      // ~750km max distance
    alice_to_charlie: 5,  // ~750km max distance
    // Bob measuring others
    bob_to_alice: 5,
    bob_to_charlie: 5,
    // Charlie measuring others
    charlie_to_alice: 5,
    charlie_to_bob: 5,
};

async function main() {
    console.log('\n========================================');
    console.log('  Presence-Based Triangulation Test');
    console.log('========================================\n');

    // Connect to node
    console.log(`Connecting to ${WS_ENDPOINT}...`);
    const provider = new WsProvider(WS_ENDPOINT);
    const api = await ApiPromise.create({ provider });

    const [chain, nodeName, nodeVersion] = await Promise.all([
        api.rpc.system.chain(),
        api.rpc.system.name(),
        api.rpc.system.version(),
    ]);
    console.log(`Connected to ${chain} using ${nodeName} v${nodeVersion}\n`);

    // Setup keyring
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    const bob = keyring.addFromUri('//Bob');
    const charlie = keyring.addFromUri('//Charlie');

    // Get current epoch
    let currentEpoch;
    try {
        currentEpoch = await api.query.presence.currentEpoch();
        console.log(`Current epoch: ${currentEpoch.toNumber()}\n`);
    } catch (e) {
        console.log('Could not get epoch, using default 1');
        currentEpoch = { toNumber: () => 1 };
    }
    const epochId = currentEpoch.toNumber();

    // Check if presence pallet has the new methods
    if (!api.tx.presence.claimPosition) {
        console.error('ERROR: presence.claimPosition not found!');
        console.error('Make sure you are running the updated node with PBT support.');
        process.exit(1);
    }

    console.log('--- Step 1: Set Validator Positions ---\n');

    // Set validator positions
    for (const [name, signer] of [['alice', alice], ['bob', bob], ['charlie', charlie]]) {
        const pos = VALIDATOR_POSITIONS[name];
        console.log(`Setting ${name}'s position to (${pos.x}, ${pos.y}, ${pos.z})...`);

        try {
            // Get validator ID (hash of account)
            const validatorId = api.createType('H256', signer.publicKey);

            const tx = api.tx.presence.setValidatorPosition(
                validatorId,
                { x: pos.x, y: pos.y, z: pos.z }
            );

            await signAndSend(tx, signer, api);
            console.log(`  ✓ ${name} position set\n`);
        } catch (e) {
            console.log(`  ✗ Failed: ${e.message}\n`);
        }
    }

    console.log('--- Step 2: Claim Positions ---\n');

    // Each validator claims their own position
    for (const [name, signer] of [['alice', alice], ['bob', bob], ['charlie', charlie]]) {
        const pos = VALIDATOR_POSITIONS[name];
        console.log(`${name} claiming position (${pos.x}, ${pos.y}, ${pos.z})...`);

        try {
            const tx = api.tx.presence.claimPosition(
                epochId,
                { x: pos.x, y: pos.y, z: pos.z }
            );

            await signAndSend(tx, signer, api);
            console.log(`  ✓ ${name} position claimed\n`);
        } catch (e) {
            if (e.message.includes('PositionAlreadyClaimed')) {
                console.log(`  ⚠ ${name} already claimed position this epoch\n`);
            } else {
                console.log(`  ✗ Failed: ${e.message}\n`);
            }
        }
    }

    console.log('--- Step 3: Submit Witness Attestations ---\n');

    // Each validator attests to the others
    const attestations = [
        { witness: alice, target: 'bob', latency: SIMULATED_LATENCIES.alice_to_bob },
        { witness: alice, target: 'charlie', latency: SIMULATED_LATENCIES.alice_to_charlie },
        { witness: bob, target: 'alice', latency: SIMULATED_LATENCIES.bob_to_alice },
        { witness: bob, target: 'charlie', latency: SIMULATED_LATENCIES.bob_to_charlie },
        { witness: charlie, target: 'alice', latency: SIMULATED_LATENCIES.charlie_to_alice },
        { witness: charlie, target: 'bob', latency: SIMULATED_LATENCIES.charlie_to_bob },
    ];

    const signers = { alice, bob, charlie };

    for (const att of attestations) {
        const witnessName = Object.keys(signers).find(k => signers[k] === att.witness);
        const targetSigner = signers[att.target];
        const targetActorId = api.createType('H256', targetSigner.publicKey);

        console.log(`${witnessName} attesting to ${att.target}'s presence (RTT: ${att.latency}ms)...`);

        try {
            const tx = api.tx.presence.submitWitnessAttestation(
                targetActorId,
                epochId,
                att.latency,
                true // direct connection
            );

            await signAndSend(tx, att.witness, api);
            console.log(`  ✓ Attestation submitted\n`);
        } catch (e) {
            if (e.message.includes('DuplicateAttestation')) {
                console.log(`  ⚠ Already attested this epoch\n`);
            } else if (e.message.includes('SelfAttestation')) {
                console.log(`  ⚠ Cannot self-attest\n`);
            } else {
                console.log(`  ✗ Failed: ${e.message}\n`);
            }
        }
    }

    console.log('--- Step 4: Verify Positions ---\n');

    // Verify each actor's position
    for (const [name, signer] of [['alice', alice], ['bob', bob], ['charlie', charlie]]) {
        const actorId = api.createType('H256', signer.publicKey);
        console.log(`Verifying ${name}'s position...`);

        try {
            // Check attestation count first
            const count = await api.query.presence.attestationCount(epochId, actorId);
            console.log(`  Attestation count: ${count.toNumber()}`);

            if (count.toNumber() < 3) {
                console.log(`  ⚠ Need at least 3 attestations, skipping verification\n`);
                continue;
            }

            const tx = api.tx.presence.verifyPosition(actorId, epochId);
            await signAndSend(tx, alice, api);

            // Check the result
            const claim = await api.query.presence.positionClaims(epochId, actorId);
            if (claim.isSome) {
                const claimData = claim.unwrap();
                console.log(`  Claimed: (${claimData.claimedPosition.x}, ${claimData.claimedPosition.y}, ${claimData.claimedPosition.z})`);
                if (claimData.triangulatedPosition.isSome) {
                    const tri = claimData.triangulatedPosition.unwrap();
                    console.log(`  Triangulated: (${tri.x}, ${tri.y}, ${tri.z})`);
                }
                console.log(`  Confidence: ${claimData.confidence}%`);
                console.log(`  Verified: ${claimData.verified}`);
            }
            console.log(`  ✓ Verification complete\n`);
        } catch (e) {
            console.log(`  ✗ Failed: ${e.message}\n`);
        }
    }

    console.log('--- Step 5: Query Final State ---\n');

    // Show all position claims
    console.log('Position Claims:');
    for (const [name, signer] of [['alice', alice], ['bob', bob], ['charlie', charlie]]) {
        const actorId = api.createType('H256', signer.publicKey);
        const claim = await api.query.presence.positionClaims(epochId, actorId);

        if (claim.isSome) {
            const data = claim.unwrap();
            console.log(`  ${name}:`);
            console.log(`    Claimed: (${data.claimedPosition.x}, ${data.claimedPosition.y}, ${data.claimedPosition.z})`);
            console.log(`    Witnesses: ${data.witnessCount}`);
            console.log(`    Verified: ${data.verified}`);
        } else {
            console.log(`  ${name}: No claim`);
        }
    }

    console.log('\n========================================');
    console.log('  Test Complete!');
    console.log('========================================\n');

    await api.disconnect();
}

async function signAndSend(tx, signer, api) {
    return new Promise((resolve, reject) => {
        let unsub;
        tx.signAndSend(signer, { nonce: -1 }, (result) => {
            if (result.status.isInBlock) {
                // Check for errors
                const failed = result.events.find(({ event }) =>
                    api.events.system.ExtrinsicFailed.is(event)
                );
                if (failed) {
                    const [dispatchError] = failed.event.data;
                    let errorMsg = 'Unknown error';
                    if (dispatchError.isModule) {
                        const decoded = api.registry.findMetaError(dispatchError.asModule);
                        errorMsg = `${decoded.section}.${decoded.name}: ${decoded.docs.join(' ')}`;
                    }
                    if (unsub) unsub();
                    reject(new Error(errorMsg));
                } else {
                    if (unsub) unsub();
                    resolve(result);
                }
            }
        }).then(u => { unsub = u; }).catch(reject);
    });
}

main().catch(console.error);
