#!/usr/bin/env node
/**
 * Presence-Based Triangulation (PBT) Test Script v2
 *
 * This version correctly matches how the pallet derives actor/validator IDs.
 * The pallet uses blake2_256(account_id) to create ActorId/ValidatorId.
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsHex } = require('@polkadot/util-crypto');

const WS_ENDPOINT = process.env.WS_ENDPOINT || 'ws://127.0.0.1:9944';

// Test positions (in centimeters) - 6 validators in hexagonal pattern
const POSITIONS = {
    alice: { x: 0, y: 0, z: 0 },           // center
    bob: { x: 50000, y: 0, z: 0 },         // 500m east
    charlie: { x: 25000, y: 43301, z: 0 }, // 500m northeast
    dave: { x: -25000, y: 43301, z: 0 },   // 500m northwest
    eve: { x: -50000, y: 0, z: 0 },        // 500m west
    ferdie: { x: -25000, y: -43301, z: 0 },// 500m southwest
};

async function main() {
    console.log('\n========================================');
    console.log('  Presence-Based Triangulation Test v2');
    console.log('========================================\n');

    const provider = new WsProvider(WS_ENDPOINT);
    const api = await ApiPromise.create({ provider });

    const [chain, nodeName] = await Promise.all([
        api.rpc.system.chain(),
        api.rpc.system.name(),
    ]);
    console.log(`Connected to ${chain} using ${nodeName}\n`);

    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    const bob = keyring.addFromUri('//Bob');
    const charlie = keyring.addFromUri('//Charlie');
    const dave = keyring.addFromUri('//Dave');
    const eve = keyring.addFromUri('//Eve');
    const ferdie = keyring.addFromUri('//Ferdie');

    // Derive IDs the same way the pallet does (blake2_256 of public key)
    const deriveId = (signer) => api.createType('H256', blake2AsHex(signer.publicKey, 256));

    const aliceId = deriveId(alice);
    const bobId = deriveId(bob);
    const charlieId = deriveId(charlie);
    const daveId = deriveId(dave);
    const eveId = deriveId(eve);
    const ferdieId = deriveId(ferdie);

    // All validators as array for easier iteration
    const validators = [
        { name: 'alice', signer: alice, id: aliceId },
        { name: 'bob', signer: bob, id: bobId },
        { name: 'charlie', signer: charlie, id: charlieId },
        { name: 'dave', signer: dave, id: daveId },
        { name: 'eve', signer: eve, id: eveId },
        { name: 'ferdie', signer: ferdie, id: ferdieId },
    ];

    console.log('Derived IDs:');
    for (const v of validators) {
        console.log(`  ${v.name}: ${v.id.toHex().slice(0, 20)}...`);
    }
    console.log();

    // Check current epoch
    const currentEpoch = await api.query.presence.currentEpoch();
    console.log(`Current epoch: ${currentEpoch.toNumber()}`);

    // Check if validators are active
    console.log(`\nValidator status:`);
    let allActive = true;
    for (const v of validators) {
        const active = await api.query.presence.activeValidators(v.id);
        console.log(`  ${v.name} active: ${active.toString()}`);
        if (!active.isTrue) allActive = false;
    }
    console.log();

    if (!allActive) {
        console.log('ERROR: Not all validators active. Run setup-validators.js first!');
        await api.disconnect();
        return;
    }

    const epoch = currentEpoch.toNumber();

    // Step 1: Set validator positions (using the validator's own key)
    console.log('--- Step 1: Set Validator Positions ---\n');

    for (const v of validators) {
        const pos = POSITIONS[v.name];
        console.log(`${v.name} setting their position to (${pos.x}, ${pos.y}, ${pos.z})...`);

        try {
            const tx = api.tx.presence.setValidatorPosition(v.id, pos);
            await signAndSend(tx, v.signer, api);
            console.log(`  ✓ Position set\n`);
        } catch (e) {
            console.log(`  ✗ Failed: ${e.message}\n`);
        }
    }

    // Verify positions were set
    console.log('Verifying validator positions...');
    for (const v of validators) {
        const pos = await api.query.presence.validatorPositions(v.id);
        if (pos.isSome) {
            const p = pos.unwrap();
            console.log(`  ${v.name}: (${p.x}, ${p.y}, ${p.z})`);
        } else {
            console.log(`  ${v.name}: NOT SET`);
        }
    }

    // Step 2: Claim positions
    console.log('\n--- Step 2: Claim Positions ---\n');

    for (const v of validators) {
        const pos = POSITIONS[v.name];
        console.log(`${v.name} claiming position (${pos.x}, ${pos.y}, ${pos.z})...`);

        try {
            const tx = api.tx.presence.claimPosition(epoch, pos);
            await signAndSend(tx, v.signer, api);
            console.log(`  ✓ Claimed\n`);
        } catch (e) {
            if (e.message.includes('PositionAlreadyClaimed')) {
                console.log(`  ⚠ Already claimed\n`);
            } else {
                console.log(`  ✗ Failed: ${e.message}\n`);
            }
        }
    }

    // Verify claims
    console.log('Verifying position claims...');
    for (const v of validators) {
        const claim = await api.query.presence.positionClaims(epoch, v.id);
        if (claim.isSome) {
            const c = claim.unwrap();
            console.log(`  ${v.name}: claimed (${c.claimedPosition.x}, ${c.claimedPosition.y}, ${c.claimedPosition.z})`);
        } else {
            console.log(`  ${v.name}: NO CLAIM`);
        }
    }

    // Step 3: Submit witness attestations
    // Each validator attests to all other validators (5 attestations per target)
    console.log('\n--- Step 3: Submit Witness Attestations ---\n');
    console.log('Each validator attests to all others (5 attestations per target)...\n');

    for (const witness of validators) {
        for (const target of validators) {
            // Skip self-attestation
            if (witness.name === target.name) continue;

            // Simulate varying latencies based on "distance"
            const latency = Math.floor(Math.random() * 10) + 2; // 2-12ms RTT

            console.log(`${witness.name} -> ${target.name} (RTT: ${latency}ms)...`);

            try {
                const tx = api.tx.presence.submitWitnessAttestation(
                    target.id,
                    epoch,
                    latency,
                    true // direct connection
                );
                await signAndSend(tx, witness.signer, api);
                console.log(`  ✓ Attested`);
            } catch (e) {
                if (e.message.includes('DuplicateAttestation')) {
                    console.log(`  ⚠ Already attested`);
                } else if (e.message.includes('SelfAttestation')) {
                    console.log(`  ⚠ Cannot self-attest`);
                } else {
                    console.log(`  ✗ Failed: ${e.message}`);
                }
            }
        }
        console.log(); // blank line between witnesses
    }

    // Check attestation counts
    console.log('Attestation counts:');
    for (const v of validators) {
        const count = await api.query.presence.attestationCount(epoch, v.id);
        console.log(`  ${v.name}: ${count.toNumber()} attestations`);
    }

    // Step 4: Verify positions (if enough attestations)
    console.log('\n--- Step 4: Verify Positions ---\n');

    for (const v of validators) {
        const count = await api.query.presence.attestationCount(epoch, v.id);
        console.log(`Verifying ${v.name}'s position (${count.toNumber()} attestations)...`);

        if (count.toNumber() < 3) {
            console.log(`  ⚠ Need at least 3 attestations\n`);
            continue;
        }

        try {
            const tx = api.tx.presence.verifyPosition(v.id, epoch);
            await signAndSend(tx, alice, api);

            const claim = await api.query.presence.positionClaims(epoch, v.id);
            if (claim.isSome) {
                const c = claim.unwrap();
                console.log(`  Claimed: (${c.claimedPosition.x}, ${c.claimedPosition.y}, ${c.claimedPosition.z})`);
                if (c.triangulatedPosition.isSome) {
                    const t = c.triangulatedPosition.unwrap();
                    console.log(`  Triangulated: (${t.x}, ${t.y}, ${t.z})`);
                }
                console.log(`  Confidence: ${c.confidence}%`);
                console.log(`  Verified: ${c.verified}`);
            }
            console.log(`  ✓ Verification complete\n`);
        } catch (e) {
            console.log(`  ✗ Failed: ${e.message}\n`);
        }
    }

    // Final state
    console.log('--- Final State ---\n');

    let verifiedCount = 0;
    for (const v of validators) {
        const claim = await api.query.presence.positionClaims(epoch, v.id);
        if (claim.isSome) {
            const c = claim.unwrap();
            console.log(`${v.name}:`);
            console.log(`  Position: (${c.claimedPosition.x}, ${c.claimedPosition.y}, ${c.claimedPosition.z})`);
            console.log(`  Witnesses: ${c.witnessCount}`);
            console.log(`  Verified: ${c.verified}`);
            if (c.verified.isTrue || c.verified === true) verifiedCount++;
            console.log();
        } else {
            console.log(`${v.name}: No claim\n`);
        }
    }

    console.log(`========================================`);
    console.log(`  Test Complete!`);
    console.log(`  ${verifiedCount}/${validators.length} validators verified`);
    console.log(`========================================\n`);

    await api.disconnect();
}

async function signAndSend(tx, signer, api) {
    return new Promise((resolve, reject) => {
        let unsub;
        tx.signAndSend(signer, { nonce: -1 }, (result) => {
            if (result.status.isInBlock) {
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
