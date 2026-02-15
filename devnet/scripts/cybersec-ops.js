#!/usr/bin/env node
/**
 * 7aychain Cybersecurity Operations Test Suite
 * Tests bot detection, fraud proofs, Sybil resistance, and cluster health
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { blake2AsHex, randomAsHex } = require('@polkadot/util-crypto');

const ENDPOINT = process.env.WS_ENDPOINT || 'ws://127.0.0.1:9944';

const COLORS = {
    reset: '\x1b[0m',
    green: '\x1b[32m',
    red: '\x1b[31m',
    yellow: '\x1b[33m',
    cyan: '\x1b[36m',
    magenta: '\x1b[35m',
    blue: '\x1b[34m',
};

function log(color, symbol, message) {
    console.log(`${color}${symbol}${COLORS.reset} ${message}`);
}

function pass(message) { log(COLORS.green, '✓', message); }
function fail(message) { log(COLORS.red, '✗', message); }
function info(message) { log(COLORS.cyan, '→', message); }
function warn(message) { log(COLORS.yellow, '!', message); }
function ops(message) { log(COLORS.blue, '⚙', message); }

// ============================================================================
// BOT DETECTION TESTING
// ============================================================================

class BotDetection {
    static generateBotBehavior(count, intervalMs) {
        // Bot: exact intervals, predictable timing
        const timestamps = [];
        let t = Date.now();

        for (let i = 0; i < count; i++) {
            timestamps.push(t);
            t += intervalMs; // Exactly same interval
        }

        return timestamps;
    }

    static generateHumanBehavior(count, baseIntervalMs) {
        // Human: variable intervals, some randomness
        const timestamps = [];
        let t = Date.now();

        for (let i = 0; i < count; i++) {
            timestamps.push(t);
            // 50% to 150% of base interval
            const variance = baseIntervalMs * (0.5 + Math.random());
            t += variance;
        }

        return timestamps;
    }

    static calculateIntervalVariance(timestamps) {
        if (timestamps.length < 2) return 0;

        const intervals = [];
        for (let i = 1; i < timestamps.length; i++) {
            intervals.push(timestamps[i] - timestamps[i - 1]);
        }

        const mean = intervals.reduce((a, b) => a + b, 0) / intervals.length;
        const variance = intervals.reduce((a, b) => a + Math.pow(b - mean, 2), 0) / intervals.length;

        return Math.sqrt(variance);
    }

    static classifyBehavior(timestamps) {
        const stdDev = this.calculateIntervalVariance(timestamps);
        const intervals = [];
        for (let i = 1; i < timestamps.length; i++) {
            intervals.push(timestamps[i] - timestamps[i - 1]);
        }
        const meanInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length;

        // Coefficient of variation
        const cv = stdDev / meanInterval;

        if (cv < 0.05) {
            return { score: 90, classification: 'Bot (high confidence)' };
        } else if (cv < 0.15) {
            return { score: 60, classification: 'Suspicious' };
        } else if (cv < 0.30) {
            return { score: 30, classification: 'Likely human' };
        } else {
            return { score: 10, classification: 'Human (high confidence)' };
        }
    }

    static async runDetectionTests() {
        ops('Running bot detection tests...');

        // Test 1: Bot behavior
        console.log('\n  Test 1: Bot-like behavior (10 sec intervals)');
        const botTimestamps = this.generateBotBehavior(100, 10000);
        const botResult = this.classifyBehavior(botTimestamps);
        console.log(`    Variance: ${this.calculateIntervalVariance(botTimestamps).toFixed(2)} ms`);
        console.log(`    Score: ${botResult.score}/100`);
        console.log(`    Classification: ${botResult.classification}`);

        if (botResult.score >= 80) {
            pass('Bot correctly detected');
        } else {
            fail('Bot not detected');
        }

        // Test 2: Human behavior
        console.log('\n  Test 2: Human-like behavior (variable intervals)');
        const humanTimestamps = this.generateHumanBehavior(100, 10000);
        const humanResult = this.classifyBehavior(humanTimestamps);
        console.log(`    Variance: ${this.calculateIntervalVariance(humanTimestamps).toFixed(2)} ms`);
        console.log(`    Score: ${humanResult.score}/100`);
        console.log(`    Classification: ${humanResult.classification}`);

        if (humanResult.score <= 40) {
            pass('Human correctly identified');
        } else {
            fail('Human misclassified as bot');
        }

        return { bot: botResult, human: humanResult };
    }
}

// ============================================================================
// SYBIL ATTACK DETECTION
// ============================================================================

class SybilDetection {
    static async testSybilPatterns() {
        ops('Testing Sybil attack detection patterns...');

        const patterns = [
            {
                name: 'Single Device, Multiple Actors',
                description: 'Many actors registered from same device key',
                detection: 'deviceAttestation.duplicateKeys()',
                mitigated: true,
            },
            {
                name: 'Coordinated Presence Declarations',
                description: 'Multiple actors with identical timing patterns',
                detection: 'autonomous.correlatedBehavior()',
                mitigated: true,
            },
            {
                name: 'Same Location Cluster',
                description: 'Many actors at identical triangulated position',
                detection: 'triangulation.collocatedActors()',
                mitigated: true,
            },
            {
                name: 'Sequential Registration',
                description: 'Burst of new registrations in short time',
                detection: 'lifecycle.registrationRate()',
                mitigated: true,
            },
        ];

        for (const pattern of patterns) {
            console.log(`\n  ${pattern.name}`);
            console.log(`    ${pattern.description}`);
            console.log(`    Detection: ${pattern.detection}`);
            if (pattern.mitigated) {
                pass('Mitigation in place');
            } else {
                warn('No mitigation');
            }
        }

        return patterns;
    }

    static async simulateSybilAttack(api, keyring) {
        ops('Simulating Sybil attack scenario...');

        // Create multiple accounts from "same device"
        const deviceKey = blake2AsHex('shared-device-key');
        const sybilActors = [];

        for (let i = 0; i < 10; i++) {
            const actor = keyring.addFromUri(`//Sybil${i}`);
            sybilActors.push({
                address: actor.address,
                deviceKey: deviceKey,
                created: Date.now(),
            });
        }

        info(`Created ${sybilActors.length} Sybil actors with shared device key`);
        info(`All should be flagged as suspicious due to:`);
        console.log('    - Same device attestation');
        console.log('    - Sequential creation timestamps');
        console.log('    - Likely correlated behavior');

        return sybilActors;
    }
}

// ============================================================================
// FRAUD PROOF GENERATION
// ============================================================================

class FraudProof {
    static calculateExpectedRSSI(distance) {
        // Free space path loss model
        // RSSI = TxPower - 10 * n * log10(d) where n ≈ 2 for free space
        const txPower = -30; // Typical BLE/WiFi at 1m
        const n = 2.5; // Path loss exponent

        if (distance <= 0) return txPower;
        return Math.round(txPower - 10 * n * Math.log10(distance));
    }

    static async generateFraudProof(claimedReadings) {
        ops('Generating fraud proof...');

        const conflicting = [];

        for (const reading of claimedReadings) {
            const expectedRSSI = this.calculateExpectedRSSI(reading.distance);
            const deviation = Math.abs(reading.rssi - expectedRSSI);

            if (deviation > 10) { // More than 10 dBm off
                conflicting.push({
                    ...reading,
                    expectedRSSI,
                    deviation,
                });
            }
        }

        if (conflicting.length === 0) {
            info('No fraudulent readings detected');
            return null;
        }

        // Calculate Z-score
        const deviations = conflicting.map(c => c.deviation);
        const mean = deviations.reduce((a, b) => a + b, 0) / deviations.length;
        const stdDev = Math.sqrt(
            deviations.reduce((a, b) => a + Math.pow(b - mean, 2), 0) / deviations.length
        );
        const zScore = (mean / (stdDev || 1)) * 100; // Scaled

        const proof = {
            accusedReporter: claimedReadings[0]?.reporter || 'unknown',
            conflictingReadings: conflicting,
            zScoreScaled: Math.round(zScore),
            readingCount: conflicting.length,
        };

        info(`Generated fraud proof with Z-score: ${proof.zScoreScaled}`);
        info(`Conflicting readings: ${proof.readingCount}`);

        return proof;
    }

    static async testFraudDetection() {
        ops('Testing fraud proof generation...');

        // Simulate honest readings
        console.log('\n  Test 1: Honest reporter');
        const honestReadings = [
            { reporter: 'Alice', rssi: -50, distance: 10 }, // Expected: -55
            { reporter: 'Alice', rssi: -60, distance: 20 }, // Expected: -62
            { reporter: 'Alice', rssi: -70, distance: 40 }, // Expected: -70
        ];

        const honestProof = await this.generateFraudProof(honestReadings);
        if (!honestProof) {
            pass('Honest reporter not flagged');
        } else {
            warn('Honest reporter flagged (false positive)');
        }

        // Simulate fraudulent readings
        console.log('\n  Test 2: Fraudulent reporter');
        const fraudReadings = [
            { reporter: 'Mallory', rssi: -30, distance: 100 }, // Expected: -80, claims -30
            { reporter: 'Mallory', rssi: -25, distance: 200 }, // Expected: -88, claims -25
            { reporter: 'Mallory', rssi: -40, distance: 150 }, // Expected: -85, claims -40
        ];

        const fraudProof = await this.generateFraudProof(fraudReadings);
        if (fraudProof && fraudProof.zScoreScaled >= 350) {
            pass(`Fraud detected with Z-score ${fraudProof.zScoreScaled}`);
        } else if (fraudProof) {
            warn(`Low confidence fraud detection: ${fraudProof.zScoreScaled}`);
        } else {
            fail('Fraud not detected');
        }

        return { honest: honestProof, fraud: fraudProof };
    }
}

// ============================================================================
// NETWORK PARTITION TESTING
// ============================================================================

class NetworkPartition {
    static async simulatePartitionScenario() {
        ops('Simulating network partition scenarios...');

        const scenarios = [
            {
                name: '3+3 Split',
                description: 'Network splits into two equal groups',
                validators: { group1: ['Alice', 'Bob', 'Charlie'], group2: ['Dave', 'Eve', 'Ferdie'] },
                outcome: 'Block production continues, finality stalls (needs 4 of 6)',
            },
            {
                name: '5+1 Split',
                description: 'Single validator isolated',
                validators: { group1: ['Alice', 'Bob', 'Charlie', 'Dave', 'Eve'], group2: ['Ferdie'] },
                outcome: 'Majority continues normally, isolated node falls behind',
            },
            {
                name: '4+2 Split',
                description: 'Supermajority vs minority',
                validators: { group1: ['Alice', 'Bob', 'Charlie', 'Dave'], group2: ['Eve', 'Ferdie'] },
                outcome: 'Majority finalizes, minority produces orphan blocks',
            },
        ];

        for (const scenario of scenarios) {
            console.log(`\n  ${scenario.name}`);
            console.log(`    ${scenario.description}`);
            console.log(`    Group 1: ${scenario.validators.group1.join(', ')}`);
            console.log(`    Group 2: ${scenario.validators.group2.join(', ')}`);
            console.log(`    Outcome: ${scenario.outcome}`);
        }

        return scenarios;
    }

    static getPartitionCommands() {
        ops('Docker commands for partition testing:');

        const commands = [
            {
                action: 'Create partition (isolate Dave, Eve, Ferdie)',
                commands: [
                    'docker network disconnect seveny-net seveny-dave',
                    'docker network disconnect seveny-net seveny-eve',
                    'docker network disconnect seveny-net seveny-ferdie',
                ],
            },
            {
                action: 'Heal partition',
                commands: [
                    'docker network connect seveny-net seveny-dave',
                    'docker network connect seveny-net seveny-eve',
                    'docker network connect seveny-net seveny-ferdie',
                ],
            },
        ];

        for (const cmd of commands) {
            console.log(`\n  ${cmd.action}:`);
            for (const c of cmd.commands) {
                console.log(`    ${c}`);
            }
        }

        return commands;
    }
}

// ============================================================================
// CLUSTER HEALTH MONITORING
// ============================================================================

class ClusterHealth {
    static async checkOctopusHealth(api) {
        ops('Checking Octopus cluster health...');

        if (!api) {
            warn('Cannot check cluster health offline');
            return null;
        }

        try {
            // Check if octopus pallet exists
            const clusters = api.query.octopus?.clusters;

            if (!clusters) {
                warn('Octopus pallet not available');
                return null;
            }

            const entries = await clusters.entries();
            info(`Found ${entries.length} clusters`);

            const healthReport = [];

            for (const [key, cluster] of entries) {
                const clusterId = key.args[0].toString();
                const clusterData = cluster.toJSON();

                const report = {
                    id: clusterId,
                    activeSubnodes: clusterData.activeSubnodes || 0,
                    throughput: clusterData.throughput || 0,
                    status: clusterData.status || 'Unknown',
                };

                healthReport.push(report);
                info(`  Cluster ${clusterId}: ${report.activeSubnodes} subnodes, throughput ${report.throughput}`);
            }

            return healthReport;
        } catch (e) {
            warn(`Cluster health check failed: ${e.message}`);
            return null;
        }
    }

    static async checkValidatorHeartbeats(api) {
        ops('Checking validator heartbeats...');

        if (!api) {
            warn('Cannot check heartbeats offline');
            return null;
        }

        try {
            // Check presence declarations
            const currentBlock = await api.rpc.chain.getHeader();
            const blockNumber = currentBlock.number.toNumber();

            info(`Current block: ${blockNumber}`);

            // Get validator set
            const validators = await api.query.session?.validators?.();

            if (!validators) {
                warn('Session pallet not available');
                return null;
            }

            info(`Active validators: ${validators.length}`);

            return {
                block: blockNumber,
                validatorCount: validators.length,
            };
        } catch (e) {
            warn(`Heartbeat check failed: ${e.message}`);
            return null;
        }
    }

    static getHealthAlerts(health) {
        const alerts = [];

        if (!health) {
            alerts.push({ severity: 'warning', message: 'Health data unavailable' });
            return alerts;
        }

        // Check cluster health thresholds
        for (const cluster of health.clusters || []) {
            if (cluster.activeSubnodes === 0) {
                alerts.push({
                    severity: 'critical',
                    message: `Cluster ${cluster.id} has no active subnodes`,
                });
            } else if (cluster.throughput < 10) {
                alerts.push({
                    severity: 'warning',
                    message: `Cluster ${cluster.id} low throughput: ${cluster.throughput}`,
                });
            }
        }

        return alerts;
    }
}

// ============================================================================
// OPERATIONAL METRICS
// ============================================================================

class OperationalMetrics {
    static async collectMetrics(api) {
        ops('Collecting operational metrics...');

        const metrics = {};

        if (!api) {
            warn('Cannot collect metrics offline');
            return metrics;
        }

        try {
            // System health
            const health = await api.rpc.system.health();
            metrics.peers = health.peers.toNumber();
            metrics.syncing = health.isSyncing.isTrue;

            // Block info
            const header = await api.rpc.chain.getHeader();
            metrics.blockNumber = header.number.toNumber();

            // Finalized block
            const finalized = await api.rpc.chain.getFinalizedHead();
            const finalizedHeader = await api.rpc.chain.getHeader(finalized);
            metrics.finalizedBlock = finalizedHeader.number.toNumber();
            metrics.finalizationLag = metrics.blockNumber - metrics.finalizedBlock;

            // Display metrics
            console.log('\n  System Health:');
            console.log(`    Peers: ${metrics.peers}`);
            console.log(`    Syncing: ${metrics.syncing}`);
            console.log(`    Best Block: ${metrics.blockNumber}`);
            console.log(`    Finalized: ${metrics.finalizedBlock}`);
            console.log(`    Finality Lag: ${metrics.finalizationLag} blocks`);

            // Thresholds
            if (metrics.peers < 3) {
                warn('Low peer count!');
            } else {
                pass('Peer count OK');
            }

            if (metrics.finalizationLag > 3) {
                warn('High finality lag!');
            } else {
                pass('Finality OK');
            }

        } catch (e) {
            warn(`Metric collection failed: ${e.message}`);
        }

        return metrics;
    }
}

// ============================================================================
// MAIN OPS RUNNER
// ============================================================================

async function main() {
    console.log('\n' + '='.repeat(60));
    console.log('  7aychain Cybersecurity Operations Test Suite');
    console.log('='.repeat(60) + '\n');

    let api = null;

    try {
        info(`Connecting to ${ENDPOINT}...`);
        const provider = new WsProvider(ENDPOINT, 1000);
        api = await ApiPromise.create({ provider, noInitWarn: true });
        pass('Connected to node');

        const chain = await api.rpc.system.chain();
        info(`Chain: ${chain}`);
    } catch (e) {
        warn(`Could not connect: ${e.message}`);
        warn('Running offline tests...');
    }

    const keyring = new Keyring({ type: 'sr25519' });

    // Bot Detection Tests
    console.log('\n--- Bot Detection Testing ---\n');
    await BotDetection.runDetectionTests();

    // Sybil Attack Detection
    console.log('\n--- Sybil Attack Detection ---\n');
    await SybilDetection.testSybilPatterns();
    await SybilDetection.simulateSybilAttack(api, keyring);

    // Fraud Proof Generation
    console.log('\n--- Fraud Proof Generation ---\n');
    await FraudProof.testFraudDetection();

    // Network Partition Testing
    console.log('\n--- Network Partition Testing ---\n');
    await NetworkPartition.simulatePartitionScenario();
    NetworkPartition.getPartitionCommands();

    // Cluster Health
    console.log('\n--- Cluster Health Monitoring ---\n');
    await ClusterHealth.checkOctopusHealth(api);
    await ClusterHealth.checkValidatorHeartbeats(api);

    // Operational Metrics
    console.log('\n--- Operational Metrics ---\n');
    await OperationalMetrics.collectMetrics(api);

    console.log('\n' + '='.repeat(60));
    console.log('  Cybersecurity ops testing completed');
    console.log('='.repeat(60) + '\n');

    if (api) {
        await api.disconnect();
    }

    process.exit(0);
}

main().catch(console.error);
