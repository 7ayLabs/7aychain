#!/usr/bin/env node
const { ApiPromise, WsProvider } = require('@polkadot/api');

async function main() {
    const provider = new WsProvider('ws://127.0.0.1:9944');
    const api = await ApiPromise.create({ provider });

    console.log('\n=== Chain State Check ===\n');

    // Check current block
    const header = await api.rpc.chain.getHeader();
    console.log(`Current block: #${header.number}`);

    // Check current epoch
    try {
        const epoch = await api.query.presence.currentEpoch();
        console.log(`Current epoch: ${epoch.toNumber()}`);

        const epochActive = await api.query.presence.epochActive(epoch);
        console.log(`Epoch active: ${epochActive.toString()}`);
    } catch (e) {
        console.log(`Epoch query failed: ${e.message}`);
    }

    // Check position claims count
    try {
        const presenceCount = await api.query.presence.presenceCount(0);
        console.log(`Presence count (epoch 0): ${presenceCount.toNumber()}`);

        const presenceCount1 = await api.query.presence.presenceCount(1);
        console.log(`Presence count (epoch 1): ${presenceCount1.toNumber()}`);
    } catch (e) {
        console.log(`Presence count query failed: ${e.message}`);
    }

    // List all storage entries in presence pallet for debugging
    console.log('\n--- Active Validators ---');
    try {
        const validators = await api.query.presence.activeValidators.entries();
        console.log(`Found ${validators.length} validators`);
        for (const [key, value] of validators.slice(0, 5)) {
            console.log(`  ${key.args[0].toHex().slice(0, 20)}...: ${value.toString()}`);
        }
    } catch (e) {
        console.log(`Active validators query failed: ${e.message}`);
    }

    // Check validator positions
    console.log('\n--- Validator Positions ---');
    try {
        const positions = await api.query.presence.validatorPositions.entries();
        console.log(`Found ${positions.length} validator positions`);
        for (const [key, value] of positions.slice(0, 5)) {
            if (value.isSome) {
                const pos = value.unwrap();
                console.log(`  ${key.args[0].toHex().slice(0, 20)}...: (${pos.x}, ${pos.y}, ${pos.z})`);
            }
        }
    } catch (e) {
        console.log(`Validator positions query failed: ${e.message}`);
    }

    // Check recent events
    console.log('\n--- Recent System Events ---');
    try {
        const events = await api.query.system.events();
        const recentEvents = events.slice(-20);
        for (const record of recentEvents) {
            const { event } = record;
            if (event.section !== 'system' || event.method !== 'ExtrinsicSuccess') {
                console.log(`  ${event.section}.${event.method}`);
            }
        }
    } catch (e) {
        console.log(`Events query failed: ${e.message}`);
    }

    await api.disconnect();
}

main().catch(console.error);
