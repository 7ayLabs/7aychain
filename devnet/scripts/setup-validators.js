#!/usr/bin/env node
/**
 * Setup script to register validators in the presence pallet.
 * This must be run before the PBT tests.
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsHex } = require('@polkadot/util-crypto');

const WS_ENDPOINT = process.env.WS_ENDPOINT || 'ws://127.0.0.1:9944';

async function main() {
    console.log('\n========================================');
    console.log('  Setting up Validators for PBT');
    console.log('========================================\n');

    const provider = new WsProvider(WS_ENDPOINT);
    const api = await ApiPromise.create({ provider });

    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');  // Alice is sudo
    const bob = keyring.addFromUri('//Bob');
    const charlie = keyring.addFromUri('//Charlie');
    const dave = keyring.addFromUri('//Dave');
    const eve = keyring.addFromUri('//Eve');
    const ferdie = keyring.addFromUri('//Ferdie');

    const validators = [
        { name: 'alice', signer: alice },
        { name: 'bob', signer: bob },
        { name: 'charlie', signer: charlie },
        { name: 'dave', signer: dave },
        { name: 'eve', signer: eve },
        { name: 'ferdie', signer: ferdie },
    ];

    console.log('Step 1: Register validators using sudo...\n');

    // Build batch of calls to set validators as active
    const calls = [];
    for (const v of validators) {
        // Create validator ID from public key hash
        const validatorId = api.createType('H256', blake2AsHex(v.signer.publicKey, 256));
        console.log(`  ${v.name}: ${validatorId.toHex().slice(0, 20)}...`);

        // This requires a dispatchable to set validators - let's check what's available
        // For now, we'll set their status directly via sudo
    }

    // Check if presence.setValidatorStatus exists
    if (api.tx.presence.setValidatorStatus) {
        console.log('\nUsing presence.setValidatorStatus...\n');

        for (const v of validators) {
            const validatorId = api.createType('H256', blake2AsHex(v.signer.publicKey, 256));

            try {
                // Use sudo to call setValidatorStatus
                const call = api.tx.presence.setValidatorStatus(validatorId, true);
                const sudoCall = api.tx.sudo.sudo(call);

                console.log(`Setting ${v.name} as active validator...`);
                await signAndSend(sudoCall, alice, api);
                console.log(`  ✓ ${v.name} registered\n`);
            } catch (e) {
                console.log(`  ✗ Failed: ${e.message}\n`);
            }
        }
    } else {
        console.log('\npresence.setValidatorStatus not found. Checking available methods...');
        console.log('Available presence methods:', Object.keys(api.tx.presence || {}));
    }

    // Verify validators are registered
    console.log('\nStep 2: Verifying validators...\n');

    const activeValidators = await api.query.presence.activeValidators.entries();
    console.log(`Active validators: ${activeValidators.length}`);

    for (const [key, value] of activeValidators) {
        console.log(`  ${key.args[0].toHex().slice(0, 20)}...: ${value.toString()}`);
    }

    // Check current epoch status
    const epoch = await api.query.presence.currentEpoch();
    const epochActive = await api.query.presence.epochActive(epoch);
    console.log(`\nCurrent epoch: ${epoch.toNumber()}, active: ${epochActive.toString()}`);

    // If epoch is not active, try to start it
    if (!epochActive.isTrue && api.tx.presence.startEpoch) {
        console.log('Starting epoch...');
        try {
            const call = api.tx.presence.startEpoch(epoch);
            const sudoCall = api.tx.sudo.sudo(call);
            await signAndSend(sudoCall, alice, api);
            console.log('  ✓ Epoch started\n');
        } catch (e) {
            console.log(`  ✗ Failed: ${e.message}\n`);
        }
    }

    console.log('\n========================================');
    console.log('  Setup Complete!');
    console.log('========================================\n');

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
