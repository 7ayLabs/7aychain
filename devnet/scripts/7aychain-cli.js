#!/usr/bin/env node
/**
 * 7aychain CLI Test Tool
 *
 * A comprehensive command-line interface for testing and interacting
 * with the 7aychain blockchain.
 *
 * Usage: ./7aychain-cli.js <command> [options]
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsHex } = require('@polkadot/util-crypto');

// ============================================================================
// Configuration
// ============================================================================

const CONFIG = {
    wsEndpoint: process.env.WS_ENDPOINT || 'ws://127.0.0.1:9944',
    accounts: ['alice', 'bob', 'charlie', 'dave', 'eve', 'ferdie'],
};

// ============================================================================
// Helper Functions
// ============================================================================

let api = null;
let keyring = null;

async function connect() {
    if (api && api.isConnected) return api;

    const provider = new WsProvider(CONFIG.wsEndpoint);
    api = await ApiPromise.create({ provider });
    keyring = new Keyring({ type: 'sr25519' });
    return api;
}

async function disconnect() {
    if (api) {
        await api.disconnect();
        api = null;
    }
}

function getSigner(name) {
    const normalized = name.charAt(0).toUpperCase() + name.slice(1).toLowerCase();
    return keyring.addFromUri(`//${normalized}`);
}

function deriveId(signer) {
    return api.createType('H256', blake2AsHex(signer.publicKey, 256));
}

async function signAndSend(tx, signer, silent = false) {
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
                        errorMsg = `${decoded.section}.${decoded.name}`;
                    }
                    if (unsub) unsub();
                    reject(new Error(errorMsg));
                } else {
                    if (!silent) {
                        console.log(`  Block: ${result.status.asInBlock.toHex().slice(0, 18)}...`);
                    }
                    if (unsub) unsub();
                    resolve(result);
                }
            }
        }).then(u => { unsub = u; }).catch(reject);
    });
}

function formatPosition(pos) {
    if (!pos) return 'N/A';
    return `(${pos.x}, ${pos.y}, ${pos.z})`;
}

function printHeader(title) {
    console.log('\n' + '='.repeat(50));
    console.log(`  ${title}`);
    console.log('='.repeat(50) + '\n');
}

function printSubHeader(title) {
    console.log(`\n--- ${title} ---\n`);
}

// ============================================================================
// Commands
// ============================================================================

const commands = {
    // ------------------------------------------------------------------------
    // Status Commands
    // ------------------------------------------------------------------------

    async status() {
        printHeader('7aychain Status');

        const [chain, nodeName, nodeVersion, health] = await Promise.all([
            api.rpc.system.chain(),
            api.rpc.system.name(),
            api.rpc.system.version(),
            api.rpc.system.health(),
        ]);

        const header = await api.rpc.chain.getHeader();
        const finalizedHash = await api.rpc.chain.getFinalizedHead();
        const finalizedHeader = await api.rpc.chain.getHeader(finalizedHash);

        console.log(`Chain:          ${chain}`);
        console.log(`Node:           ${nodeName} v${nodeVersion}`);
        console.log(`Peers:          ${health.peers}`);
        console.log(`Syncing:        ${health.isSyncing}`);
        console.log(`Best Block:     #${header.number} (${header.hash.toHex().slice(0, 18)}...)`);
        console.log(`Finalized:      #${finalizedHeader.number} (${finalizedHash.toHex().slice(0, 18)}...)`);

        // Presence pallet status
        printSubHeader('Presence Pallet');
        try {
            const epoch = await api.query.presence.currentEpoch();
            const epochActive = await api.query.presence.epochActive(epoch);
            const validators = await api.query.presence.activeValidators.entries();

            console.log(`Current Epoch:  ${epoch.toNumber()}`);
            console.log(`Epoch Active:   ${epochActive.toString()}`);
            console.log(`Validators:     ${validators.length}`);
        } catch (e) {
            console.log(`Presence pallet not available: ${e.message}`);
        }
    },

    async validators() {
        printHeader('Active Validators');

        const validators = await api.query.presence.activeValidators.entries();

        if (validators.length === 0) {
            console.log('No active validators found.');
            console.log('Run: ./7aychain-cli.js setup-validators');
            return;
        }

        console.log(`Found ${validators.length} validators:\n`);

        for (const [key, value] of validators) {
            const id = key.args[0].toHex();
            const pos = await api.query.presence.validatorPositions(key.args[0]);

            console.log(`ID: ${id.slice(0, 20)}...`);
            console.log(`  Active: ${value.toString()}`);
            if (pos.isSome) {
                const p = pos.unwrap();
                console.log(`  Position: (${p.x}, ${p.y}, ${p.z})`);
            } else {
                console.log(`  Position: Not set`);
            }
            console.log();
        }
    },

    async accounts() {
        printHeader('Test Accounts');

        for (const name of CONFIG.accounts) {
            const signer = getSigner(name);
            const id = deriveId(signer);
            const balance = await api.query.system.account(signer.address);

            console.log(`${name.toUpperCase()}`);
            console.log(`  Address:  ${signer.address}`);
            console.log(`  ID:       ${id.toHex().slice(0, 30)}...`);
            console.log(`  Balance:  ${(BigInt(balance.data.free) / BigInt(10**12)).toString()} UNIT`);
            console.log();
        }
    },

    // ------------------------------------------------------------------------
    // Setup Commands
    // ------------------------------------------------------------------------

    async 'setup-validators'(accounts = CONFIG.accounts) {
        printHeader('Setup Validators');

        const alice = getSigner('alice'); // sudo account

        for (const name of accounts) {
            const signer = getSigner(name);
            const validatorId = deriveId(signer);

            process.stdout.write(`Registering ${name}... `);

            try {
                const call = api.tx.presence.setValidatorStatus(validatorId, true);
                const sudoCall = api.tx.sudo.sudo(call);
                await signAndSend(sudoCall, alice, true);
                console.log('OK');
            } catch (e) {
                console.log(`FAILED: ${e.message}`);
            }
        }

        console.log('\nVerifying...');
        const validators = await api.query.presence.activeValidators.entries();
        console.log(`Active validators: ${validators.length}`);
    },

    async 'start-epoch'(epochNum) {
        printHeader('Start Epoch');

        const epoch = epochNum ? parseInt(epochNum) : (await api.query.presence.currentEpoch()).toNumber();
        const alice = getSigner('alice');

        console.log(`Starting epoch ${epoch}...`);

        try {
            const call = api.tx.presence.startEpoch(epoch);
            const sudoCall = api.tx.sudo.sudo(call);
            await signAndSend(sudoCall, alice);
            console.log('Epoch started successfully.');
        } catch (e) {
            console.log(`Failed: ${e.message}`);
        }
    },

    // ------------------------------------------------------------------------
    // Position Commands
    // ------------------------------------------------------------------------

    async 'set-position'(account, x, y, z = 0) {
        printHeader('Set Validator Position');

        if (!account || x === undefined || y === undefined) {
            console.log('Usage: set-position <account> <x> <y> [z]');
            console.log('Example: set-position alice 0 0 0');
            return;
        }

        const signer = getSigner(account);
        const id = deriveId(signer);
        const pos = { x: parseInt(x), y: parseInt(y), z: parseInt(z) };

        console.log(`Account: ${account}`);
        console.log(`Position: (${pos.x}, ${pos.y}, ${pos.z})`);

        try {
            const tx = api.tx.presence.setValidatorPosition(id, pos);
            await signAndSend(tx, signer);
            console.log('Position set successfully.');
        } catch (e) {
            console.log(`Failed: ${e.message}`);
        }
    },

    async 'claim-position'(account, x, y, z = 0) {
        printHeader('Claim Position');

        if (!account || x === undefined || y === undefined) {
            console.log('Usage: claim-position <account> <x> <y> [z]');
            console.log('Example: claim-position alice 0 0 0');
            return;
        }

        const signer = getSigner(account);
        const epoch = (await api.query.presence.currentEpoch()).toNumber();
        const pos = { x: parseInt(x), y: parseInt(y), z: parseInt(z) };

        console.log(`Account: ${account}`);
        console.log(`Epoch: ${epoch}`);
        console.log(`Position: (${pos.x}, ${pos.y}, ${pos.z})`);

        try {
            const tx = api.tx.presence.claimPosition(epoch, pos);
            await signAndSend(tx, signer);
            console.log('Position claimed successfully.');
        } catch (e) {
            console.log(`Failed: ${e.message}`);
        }
    },

    async 'get-position'(account) {
        printHeader('Get Position');

        if (!account) {
            console.log('Usage: get-position <account>');
            return;
        }

        const signer = getSigner(account);
        const id = deriveId(signer);
        const epoch = (await api.query.presence.currentEpoch()).toNumber();

        console.log(`Account: ${account}`);
        console.log(`ID: ${id.toHex().slice(0, 30)}...`);

        const validatorPos = await api.query.presence.validatorPositions(id);
        console.log(`\nValidator Position: ${validatorPos.isSome ? formatPosition(validatorPos.unwrap()) : 'Not set'}`);

        const claim = await api.query.presence.positionClaims(epoch, id);
        if (claim.isSome) {
            const c = claim.unwrap();
            console.log(`\nClaim (Epoch ${epoch}):`);
            console.log(`  Claimed Position: ${formatPosition(c.claimedPosition)}`);
            if (c.triangulatedPosition.isSome) {
                console.log(`  Triangulated:     ${formatPosition(c.triangulatedPosition.unwrap())}`);
            }
            console.log(`  Witnesses:        ${c.witnessCount}`);
            console.log(`  Confidence:       ${c.confidence}%`);
            console.log(`  Verified:         ${c.verified}`);
        } else {
            console.log(`\nNo claim for epoch ${epoch}`);
        }
    },

    // ------------------------------------------------------------------------
    // Attestation Commands
    // ------------------------------------------------------------------------

    async attest(witness, target, latency = 5) {
        printHeader('Submit Attestation');

        if (!witness || !target) {
            console.log('Usage: attest <witness> <target> [latency_ms]');
            console.log('Example: attest alice bob 5');
            return;
        }

        const witnessSigner = getSigner(witness);
        const targetSigner = getSigner(target);
        const targetId = deriveId(targetSigner);
        const epoch = (await api.query.presence.currentEpoch()).toNumber();

        console.log(`Witness: ${witness}`);
        console.log(`Target: ${target}`);
        console.log(`Latency: ${latency}ms`);
        console.log(`Epoch: ${epoch}`);

        try {
            const tx = api.tx.presence.submitWitnessAttestation(
                targetId,
                epoch,
                parseInt(latency),
                true
            );
            await signAndSend(tx, witnessSigner);
            console.log('Attestation submitted successfully.');
        } catch (e) {
            console.log(`Failed: ${e.message}`);
        }
    },

    async 'get-attestations'(account) {
        printHeader('Get Attestations');

        if (!account) {
            console.log('Usage: get-attestations <account>');
            return;
        }

        const signer = getSigner(account);
        const id = deriveId(signer);
        const epoch = (await api.query.presence.currentEpoch()).toNumber();

        console.log(`Account: ${account}`);
        console.log(`Epoch: ${epoch}`);

        const count = await api.query.presence.attestationCount(epoch, id);
        console.log(`\nAttestation Count: ${count.toNumber()}`);

        // Try to get individual attestations
        const attestations = await api.query.presence.attestations.entries(epoch, id);
        if (attestations.length > 0) {
            console.log('\nAttestations:');
            for (const [key, value] of attestations) {
                const witnessId = key.args[2].toHex();
                console.log(`  From: ${witnessId.slice(0, 20)}...`);
                if (value.isSome) {
                    const att = value.unwrap();
                    console.log(`    RTT: ${att.rttMs}ms, Direct: ${att.directConnection}`);
                }
            }
        }
    },

    // ------------------------------------------------------------------------
    // Verification Commands
    // ------------------------------------------------------------------------

    async verify(account) {
        printHeader('Verify Position');

        if (!account) {
            console.log('Usage: verify <account>');
            return;
        }

        const signer = getSigner(account);
        const id = deriveId(signer);
        const alice = getSigner('alice');
        const epoch = (await api.query.presence.currentEpoch()).toNumber();

        console.log(`Account: ${account}`);
        console.log(`Epoch: ${epoch}`);

        const countBefore = await api.query.presence.attestationCount(epoch, id);
        console.log(`Attestations: ${countBefore.toNumber()}`);

        if (countBefore.toNumber() < 3) {
            console.log('\nNeed at least 3 attestations for verification.');
            return;
        }

        try {
            const tx = api.tx.presence.verifyPosition(id, epoch);
            await signAndSend(tx, alice);

            const claim = await api.query.presence.positionClaims(epoch, id);
            if (claim.isSome) {
                const c = claim.unwrap();
                console.log('\nVerification Result:');
                console.log(`  Claimed:      ${formatPosition(c.claimedPosition)}`);
                if (c.triangulatedPosition.isSome) {
                    console.log(`  Triangulated: ${formatPosition(c.triangulatedPosition.unwrap())}`);
                }
                console.log(`  Confidence:   ${c.confidence}%`);
                console.log(`  Verified:     ${c.verified}`);
            }
        } catch (e) {
            console.log(`Failed: ${e.message}`);
        }
    },

    // ------------------------------------------------------------------------
    // Test Scenarios
    // ------------------------------------------------------------------------

    async 'test-pbt'() {
        printHeader('PBT Full Test');

        const positions = {
            alice: { x: 0, y: 0, z: 0 },
            bob: { x: 50000, y: 0, z: 0 },
            charlie: { x: 25000, y: 43301, z: 0 },
            dave: { x: -25000, y: 43301, z: 0 },
            eve: { x: -50000, y: 0, z: 0 },
            ferdie: { x: -25000, y: -43301, z: 0 },
        };

        const accounts = Object.keys(positions);
        const epoch = (await api.query.presence.currentEpoch()).toNumber();

        console.log(`Epoch: ${epoch}`);
        console.log(`Testing ${accounts.length} validators\n`);

        // Step 1: Set positions
        printSubHeader('Step 1: Set Validator Positions');
        for (const name of accounts) {
            const signer = getSigner(name);
            const id = deriveId(signer);
            const pos = positions[name];

            process.stdout.write(`${name}: ${formatPosition(pos)}... `);
            try {
                const tx = api.tx.presence.setValidatorPosition(id, pos);
                await signAndSend(tx, signer, true);
                console.log('OK');
            } catch (e) {
                console.log(`FAILED: ${e.message}`);
            }
        }

        // Step 2: Claim positions
        printSubHeader('Step 2: Claim Positions');
        for (const name of accounts) {
            const signer = getSigner(name);
            const pos = positions[name];

            process.stdout.write(`${name}: ${formatPosition(pos)}... `);
            try {
                const tx = api.tx.presence.claimPosition(epoch, pos);
                await signAndSend(tx, signer, true);
                console.log('OK');
            } catch (e) {
                if (e.message.includes('AlreadyClaimed')) {
                    console.log('Already claimed');
                } else {
                    console.log(`FAILED: ${e.message}`);
                }
            }
        }

        // Step 3: Submit attestations
        printSubHeader('Step 3: Submit Attestations');
        for (const witness of accounts) {
            const witnessSigner = getSigner(witness);
            let attested = 0;

            for (const target of accounts) {
                if (witness === target) continue;

                const targetSigner = getSigner(target);
                const targetId = deriveId(targetSigner);
                const latency = Math.floor(Math.random() * 10) + 2;

                try {
                    const tx = api.tx.presence.submitWitnessAttestation(targetId, epoch, latency, true);
                    await signAndSend(tx, witnessSigner, true);
                    attested++;
                } catch (e) {
                    // Skip errors (duplicates, etc.)
                }
            }
            console.log(`${witness}: ${attested} attestations sent`);
        }

        // Step 4: Check counts
        printSubHeader('Step 4: Attestation Counts');
        for (const name of accounts) {
            const signer = getSigner(name);
            const id = deriveId(signer);
            const count = await api.query.presence.attestationCount(epoch, id);
            console.log(`${name}: ${count.toNumber()} attestations`);
        }

        // Step 5: Verify
        printSubHeader('Step 5: Verify Positions');
        const alice = getSigner('alice');
        let verified = 0;

        for (const name of accounts) {
            const signer = getSigner(name);
            const id = deriveId(signer);
            const count = await api.query.presence.attestationCount(epoch, id);

            process.stdout.write(`${name}: `);

            if (count.toNumber() < 3) {
                console.log('Not enough attestations');
                continue;
            }

            try {
                const tx = api.tx.presence.verifyPosition(id, epoch);
                await signAndSend(tx, alice, true);

                const claim = await api.query.presence.positionClaims(epoch, id);
                if (claim.isSome) {
                    const c = claim.unwrap();
                    const v = c.verified.isTrue || c.verified === true;
                    if (v) verified++;
                    console.log(`${v ? 'VERIFIED' : 'NOT VERIFIED'} (${c.confidence}% confidence)`);
                }
            } catch (e) {
                console.log(`FAILED: ${e.message}`);
            }
        }

        // Summary
        printSubHeader('Summary');
        console.log(`Validators tested: ${accounts.length}`);
        console.log(`Verified: ${verified}/${accounts.length}`);
    },

    async 'test-quick'() {
        printHeader('Quick PBT Test (3 validators)');

        const accounts = ['alice', 'bob', 'charlie'];
        const epoch = (await api.query.presence.currentEpoch()).toNumber();

        console.log(`Testing with: ${accounts.join(', ')}`);
        console.log(`Epoch: ${epoch}\n`);

        // Each attests to the others
        for (const witness of accounts) {
            const witnessSigner = getSigner(witness);
            for (const target of accounts) {
                if (witness === target) continue;

                const targetSigner = getSigner(target);
                const targetId = deriveId(targetSigner);

                process.stdout.write(`${witness} -> ${target}: `);
                try {
                    const tx = api.tx.presence.submitWitnessAttestation(targetId, epoch, 5, true);
                    await signAndSend(tx, witnessSigner, true);
                    console.log('OK');
                } catch (e) {
                    console.log(e.message.includes('Duplicate') ? 'Already done' : `FAILED`);
                }
            }
        }

        // Show counts
        console.log('\nAttestation counts:');
        for (const name of accounts) {
            const signer = getSigner(name);
            const id = deriveId(signer);
            const count = await api.query.presence.attestationCount(epoch, id);
            console.log(`  ${name}: ${count.toNumber()}`);
        }
    },

    // ------------------------------------------------------------------------
    // Query Commands
    // ------------------------------------------------------------------------

    async epoch() {
        printHeader('Epoch Info');

        const current = await api.query.presence.currentEpoch();
        const active = await api.query.presence.epochActive(current);

        console.log(`Current Epoch: ${current.toNumber()}`);
        console.log(`Active: ${active.toString()}`);

        // Count claims for this epoch
        const claims = await api.query.presence.positionClaims.entries(current);
        console.log(`Position Claims: ${claims.length}`);

        // Count presence declarations
        try {
            const presenceCount = await api.query.presence.presenceCount(current);
            console.log(`Presence Count: ${presenceCount.toNumber()}`);
        } catch (e) {
            // Not available
        }
    },

    async claims() {
        printHeader('Position Claims');

        const epoch = (await api.query.presence.currentEpoch()).toNumber();
        const claims = await api.query.presence.positionClaims.entries(epoch);

        console.log(`Epoch: ${epoch}`);
        console.log(`Total claims: ${claims.length}\n`);

        for (const [key, value] of claims) {
            if (value.isSome) {
                const c = value.unwrap();
                const actorId = key.args[1].toHex();

                console.log(`Actor: ${actorId.slice(0, 20)}...`);
                console.log(`  Claimed: ${formatPosition(c.claimedPosition)}`);
                if (c.triangulatedPosition.isSome) {
                    console.log(`  Triangulated: ${formatPosition(c.triangulatedPosition.unwrap())}`);
                }
                console.log(`  Witnesses: ${c.witnessCount}`);
                console.log(`  Confidence: ${c.confidence}%`);
                console.log(`  Verified: ${c.verified}`);
                console.log();
            }
        }
    },

    async events(count = 20) {
        printHeader('Recent Events');

        const events = await api.query.system.events();
        const recent = events.slice(-parseInt(count));

        console.log(`Showing last ${recent.length} events:\n`);

        for (const record of recent) {
            const { event } = record;
            // Skip common noise
            if (event.section === 'system' && event.method === 'ExtrinsicSuccess') continue;
            if (event.section === 'transactionPayment') continue;

            console.log(`${event.section}.${event.method}`);
        }
    },

    // ------------------------------------------------------------------------
    // Transfer Commands
    // ------------------------------------------------------------------------

    async transfer(from, to, amount) {
        printHeader('Transfer');

        if (!from || !to || !amount) {
            console.log('Usage: transfer <from> <to> <amount>');
            console.log('Example: transfer alice bob 100');
            return;
        }

        const fromSigner = getSigner(from);
        const toSigner = getSigner(to);
        const value = BigInt(amount) * BigInt(10**12);

        console.log(`From: ${from} (${fromSigner.address})`);
        console.log(`To: ${to} (${toSigner.address})`);
        console.log(`Amount: ${amount} UNIT`);

        try {
            const tx = api.tx.balances.transferKeepAlive(toSigner.address, value);
            await signAndSend(tx, fromSigner);
            console.log('Transfer successful.');
        } catch (e) {
            console.log(`Failed: ${e.message}`);
        }
    },

    // ------------------------------------------------------------------------
    // Help
    // ------------------------------------------------------------------------

    async help() {
        console.log(`
7aychain CLI Test Tool
======================

Usage: ./7aychain-cli.js <command> [args...]

STATUS COMMANDS:
  status                    Show chain and pallet status
  validators                List active validators
  accounts                  Show test account details
  epoch                     Show current epoch info
  claims                    List all position claims
  events [count]            Show recent events

SETUP COMMANDS:
  setup-validators          Register all test validators (requires sudo)
  start-epoch [num]         Start an epoch (requires sudo)

POSITION COMMANDS:
  set-position <account> <x> <y> [z]    Set validator position (cm)
  claim-position <account> <x> <y> [z]  Claim position for epoch
  get-position <account>                Get position info

ATTESTATION COMMANDS:
  attest <witness> <target> [latency]   Submit witness attestation
  get-attestations <account>            Get attestation info
  verify <account>                      Verify position

TEST SCENARIOS:
  test-pbt                  Run full PBT test (6 validators)
  test-quick                Quick test (3 validators)

OTHER:
  transfer <from> <to> <amount>  Transfer tokens
  help                           Show this help

EXAMPLES:
  ./7aychain-cli.js status
  ./7aychain-cli.js setup-validators
  ./7aychain-cli.js set-position alice 0 0 0
  ./7aychain-cli.js claim-position alice 0 0 0
  ./7aychain-cli.js attest bob alice 5
  ./7aychain-cli.js verify alice
  ./7aychain-cli.js test-pbt

ENVIRONMENT:
  WS_ENDPOINT    WebSocket endpoint (default: ws://127.0.0.1:9944)
`);
    },
};

// ============================================================================
// Main
// ============================================================================

async function main() {
    const args = process.argv.slice(2);
    const command = args[0] || 'help';
    const params = args.slice(1);

    if (command === 'help' || command === '--help' || command === '-h') {
        await commands.help();
        return;
    }

    try {
        await connect();

        if (commands[command]) {
            await commands[command](...params);
        } else {
            console.error(`Unknown command: ${command}`);
            console.error('Run ./7aychain-cli.js help for usage.');
            process.exit(1);
        }
    } catch (e) {
        console.error(`Error: ${e.message}`);
        process.exit(1);
    } finally {
        await disconnect();
    }
}

main();
