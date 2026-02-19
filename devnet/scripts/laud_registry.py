"""
LAUD NETWORKS - Command Registry
Data-driven domain and command definitions for the LAUD CLI.
"""

from dataclasses import dataclass, field
from typing import Any


@dataclass
class Param:
    """Describes a single parameter for a CLI command."""
    name: str
    label: str
    kind: str  # int|str|bool|account|actor|epoch|h256|position|enum
    default: Any = None
    options: list = field(default_factory=list)


@dataclass
class Command:
    """A single CLI command (submit, query, or custom handler)."""
    key: str
    label: str
    action: str  # submit|query|query_map|custom|separator
    pallet: str = ""
    function: str = ""
    params: list = field(default_factory=list)
    sudo: bool = False
    aliases: list = field(default_factory=list)
    help_text: str = ""
    instructions: str = ""
    custom_handler: str = ""
    fixed_params: dict = field(default_factory=dict)
    mode: str = "both"    # "normal"|"dev"|"both"


@dataclass
class Domain:
    """A feature area containing its commands."""
    name: str
    title: str
    number: str
    shortcut: str
    group: str  # core|positioning|security|identity|intelligence|devtools|status
    commands: list = field(default_factory=list)
    check_epoch: bool = False
    help_summary: str = ""
    instructions: str = ""
    mode: str = "both"           # "normal"|"dev"|"both"
    normal_title: str = ""       # title override in normal mode
    normal_group: str = ""       # group override in normal mode


# ---------------------------------------------------------------------------
# Domain definitions
# ---------------------------------------------------------------------------

DOMAINS = [
    # ------------------------------------------------------------------
    # GETTING STARTED
    # ------------------------------------------------------------------
    Domain(
        name="dashboard", title="DASHBOARD",
        number="26", shortcut="dash", group="getting-started",
        mode="normal",
        normal_group="getting-started",
        help_summary="Quick overview of network status and your identity",
        instructions="""
  The Dashboard gives you a bird's-eye view of the network:
  current time period, your presence status, validator count,
  and recent activity. Start here to see what is happening.
""",
        commands=[
            Command("1", "Network Overview", "custom",
                    custom_handler="_dashboard_overview",
                    help_text="See chain status, block height, validators, active epoch"),
            Command("2", "My Status", "custom",
                    custom_handler="_dashboard_my_status",
                    help_text="Check your presence, device, and validator status"),
            Command("3", "Recent Activity", "custom",
                    custom_handler="_dashboard_activity",
                    help_text="Show recent events relevant to you"),
        ],
    ),

    # ------------------------------------------------------------------
    # CORE
    # ------------------------------------------------------------------
    Domain(
        name="presence", title="PRESENCE PROTOCOL",
        number="2", shortcut="p", group="core", check_epoch=True,
        mode="both", normal_title="PRESENCE", normal_group="core",
        help_summary="Declare, vote on, and finalize proof-of-presence claims",
        instructions="""
  The Presence Protocol lets participants prove they are active on the
  network during a given time period.

  TYPICAL FLOW:
    1. Make sure a time period is active (use "bootstrap" to set up)
    2. Declare your presence (option 1)
    3. Validators vote to confirm (option 4)
    4. After enough votes, finalize (option 5)

  PREREQUISITES:
    - An active time period (type "bootstrap" if first time)
    - A funded account (all test accounts are pre-funded)
""",
        commands=[
            Command("1", "Declare Presence", "submit",
                    pallet="Presence", function="declare_presence",
                    params=[Param("epoch", "Time period", "epoch")],
                    aliases=["declare", "d"],
                    help_text="Tell the network you are present in this time period",
                    instructions="""
  DECLARE PRESENCE

  What this does:
    Tells the network you are present during the chosen time period.

  What you need:
    - An active time period number
    - A funded account

  What happens next:
    - Validators can now vote on your claim
    - Once enough votes arrive, you can finalize
"""),
            Command("2", "Declare with Commitment", "custom",
                    custom_handler="_presence_commit",
                    aliases=["commit", "cm"],
                    help_text="Declare presence using a secret (commit-reveal scheme)"),
            Command("3", "Reveal Commitment", "submit",
                    pallet="Presence", function="reveal_commitment",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("secret", "Secret (hex from commit step)", "h256"),
                        Param("randomness", "Randomness (hex from commit step)", "h256"),
                    ],
                    aliases=["reveal", "rv"],
                    help_text="Reveal a previously submitted secret commitment"),
            Command("4", "Vote on Presence", "submit",
                    pallet="Presence", function="vote_presence",
                    params=[
                        Param("actor", "Target identity", "actor"),
                        Param("epoch", "Time period", "epoch"),
                        Param("approve", "Approve?", "bool", True),
                    ],
                    aliases=["vote", "v"],
                    help_text="Cast your vote on someone's presence claim",
                    instructions="""
  VOTE ON PRESENCE

  What this does:
    As a validator, cast your vote approving or rejecting
    someone's presence claim.

  What you need:
    - You must be an active validator
    - The target identity must have declared presence
    - You haven't already voted for this identity this period

  What happens next:
    - Once enough validators vote, the presence can be finalized
"""),
            Command("5", "Finalize Presence", "submit",
                    pallet="Presence", function="finalize_presence",
                    params=[
                        Param("actor", "Target identity", "actor"),
                        Param("epoch", "Time period", "epoch"),
                    ],
                    aliases=["finalize", "f"],
                    help_text="Lock in a presence claim after enough votes",
                    instructions="""
  FINALIZE PRESENCE

  What this does:
    Locks in a presence claim permanently after enough votes.

  What you need:
    - Enough validator votes (check with option c)
    - The presence must be in "Validated" state

  After this:
    - The presence is permanently recorded
    - It cannot be changed or reversed
"""),
            Command("6", "Penalize Presence [admin]", "submit",
                    pallet="Presence", function="slash_presence",
                    params=[
                        Param("actor", "Target identity", "actor"),
                        Param("epoch", "Time period", "epoch"),
                    ],
                    sudo=True, aliases=["slash", "penalize"],
                    help_text="Penalize a fraudulent presence claim",
                    mode="dev"),
            Command("7", "Set Vote Threshold [admin]", "submit",
                    pallet="Presence", function="set_quorum_config",
                    params=[
                        Param("threshold", "Minimum votes needed", "int", 2),
                        Param("total", "Total voters", "int", 3),
                    ],
                    sudo=True, aliases=["quorum", "threshold"],
                    help_text="Set how many votes are needed to confirm presence",
                    mode="dev"),
            Command("8", "Set Validator Status [admin]", "submit",
                    pallet="Presence", function="set_validator_status",
                    params=[
                        Param("validator", "Validator identity", "actor"),
                        Param("active", "Active?", "bool", True),
                    ],
                    sudo=True, aliases=["validator-status"],
                    help_text="Enable or disable a validator",
                    mode="dev"),
            Command("9", "Set Time Period Active [admin]", "submit",
                    pallet="Presence", function="set_epoch_active",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("active", "Active?", "bool", True),
                    ],
                    sudo=True, aliases=["epoch-active"],
                    help_text="Activate or deactivate a time period",
                    mode="dev"),
            Command("---", "Lookups", "separator"),
            Command("a", "Current Time Period", "query",
                    pallet="Presence", function="CurrentEpoch",
                    help_text="Check what time period the network is in"),
            Command("b", "Presence Record", "query",
                    pallet="Presence", function="Presences",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("actor", "Identity", "actor"),
                    ],
                    help_text="Look up presence status for an identity"),
            Command("c", "Vote Count", "query",
                    pallet="Presence", function="VoteCount",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("actor", "Identity", "actor"),
                    ],
                    help_text="Check how many votes an identity has received"),
            Command("d", "Active Validators", "query_map",
                    pallet="Presence", function="ActiveValidators",
                    help_text="List all currently active validators"),
            Command("e", "Commitment / Reveal Count", "custom",
                    custom_handler="_presence_commitment_count",
                    help_text="Check how many commitments and reveals exist"),
        ],
    ),

    Domain(
        name="epoch", title="TIME PERIODS (Epochs)",
        number="3", shortcut="e", group="core",
        mode="both", normal_title="SESSIONS", normal_group="core",
        help_summary="Schedule, start, close, and finalize time periods",
        instructions="""
  Time Periods (Epochs) are windows during which presence proofs
  happen. Each period moves through: Scheduled -> Active -> Closed -> Finalized.

  TYPICAL FLOW:
    1. Schedule a new time period (option 1)
    2. Start it when the block arrives (option 2)
    3. Let participants declare and vote
    4. Close and finalize when done (options 3-4)

  TIP: Use "bootstrap" to automatically set up time period 1.
""",
        commands=[
            Command("1", "Schedule Time Period [admin]", "submit",
                    pallet="Epoch", function="schedule_epoch",
                    params=[
                        Param("start_block", "Start at block #", "int", 10),
                        Param("duration", "Duration (in blocks)", "int", 60),
                    ],
                    sudo=True, aliases=["schedule"],
                    help_text="Plan a new time period to start at a future block"),
            Command("2", "Start Time Period [admin]", "submit",
                    pallet="Epoch", function="start_epoch",
                    params=[Param("epoch_id", "Time period ID", "int", 1)],
                    sudo=True, aliases=["start"],
                    help_text="Begin a scheduled time period"),
            Command("3", "Close Time Period [admin]", "submit",
                    pallet="Epoch", function="close_epoch",
                    params=[Param("epoch_id", "Time period ID", "int", 1)],
                    sudo=True, aliases=["close"],
                    help_text="Stop accepting new claims for this period"),
            Command("4", "Finalize Time Period [admin]", "submit",
                    pallet="Epoch", function="finalize_epoch",
                    params=[Param("epoch_id", "Time period ID", "int", 1)],
                    sudo=True, aliases=["finalize"],
                    help_text="Permanently close and archive this time period"),
            Command("5", "Register as Participant", "submit",
                    pallet="Epoch", function="register_participant",
                    params=[Param("epoch_id", "Time period ID", "int", 1)],
                    aliases=["register"],
                    help_text="Sign up to participate in a time period"),
            Command("6", "Update Schedule [admin]", "submit",
                    pallet="Epoch", function="update_schedule",
                    params=[
                        Param("duration", "Duration (blocks)", "int", 60),
                        Param("grace_period", "Grace period (blocks)", "int", 2),
                        Param("auto_transition", "Auto-transition?", "bool", True),
                    ],
                    sudo=True, aliases=["update"],
                    help_text="Change the default scheduling parameters",
                    mode="dev"),
            Command("7", "Force State Change [admin]", "submit",
                    pallet="Epoch", function="force_transition",
                    params=[
                        Param("epoch_id", "Time period ID", "int", 1),
                        Param("new_state", "New state", "enum",
                              options=["Scheduled", "Active", "Closed", "Finalized"]),
                    ],
                    sudo=True, aliases=["force"],
                    help_text="Override the current state of a time period",
                    mode="dev"),
            Command("---", "Lookups", "separator"),
            Command("a", "Current Time Period", "query",
                    pallet="Epoch", function="CurrentEpoch",
                    help_text="See which time period is active right now"),
            Command("b", "Time Period Info", "query",
                    pallet="Epoch", function="EpochInfo",
                    params=[Param("epoch", "Time period", "epoch")],
                    help_text="View details about a specific time period"),
            Command("c", "Total Time Periods", "query",
                    pallet="Epoch", function="EpochCount",
                    help_text="How many time periods have been created"),
            Command("d", "Schedule Settings", "query",
                    pallet="Epoch", function="EpochSchedule",
                    help_text="View the current scheduling configuration"),
        ],
    ),

    Domain(
        name="validator", title="VALIDATORS",
        number="4", shortcut="val", group="core",
        mode="both", normal_title="STAKING", normal_group="core",
        help_summary="Register, stake, activate, and manage network validators",
        instructions="""
  Validators are network participants who vote on presence claims.
  They must stake tokens to participate and can be penalized for misbehavior.

  TYPICAL FLOW:
    1. Register with a stake (option 1)
    2. Activate your validator (option 2)
    3. Vote on presence claims (via Presence menu)
    4. Withdraw stake when done (option 4)

  TIP: "bootstrap" registers 6 test validators automatically.
""",
        commands=[
            Command("1", "Register as Validator", "submit",
                    pallet="Validator", function="register_validator",
                    params=[Param("stake", "Amount to stake", "int", 1000000)],
                    aliases=["register"],
                    help_text="Join the network as a validator with an initial stake"),
            Command("2", "Activate Validator", "submit",
                    pallet="Validator", function="activate_validator",
                    aliases=["activate"],
                    help_text="Start participating in validation"),
            Command("3", "Deactivate Validator", "submit",
                    pallet="Validator", function="deactivate_validator",
                    aliases=["deactivate"],
                    help_text="Stop participating (stake remains locked)"),
            Command("4", "Withdraw Stake", "submit",
                    pallet="Validator", function="withdraw_stake",
                    aliases=["withdraw"],
                    help_text="Reclaim your staked funds after unbonding"),
            Command("5", "Increase Stake", "submit",
                    pallet="Validator", function="increase_stake",
                    params=[Param("additional", "Amount to add", "int", 100000)],
                    aliases=["stake"],
                    help_text="Add more funds to your validator stake"),
            Command("6", "Penalize Validator [admin]", "submit",
                    pallet="Validator", function="slash_validator",
                    params=[
                        Param("validator", "Validator identity", "actor"),
                        Param("violation", "Severity level", "enum",
                              options=["Minor", "Moderate", "Severe", "Critical"]),
                    ],
                    sudo=True, aliases=["slash", "penalize"],
                    help_text="Penalize a validator for misbehavior",
                    mode="dev"),
            Command("7", "Apply Pending Penalty [admin]", "submit",
                    pallet="Validator", function="apply_slash",
                    params=[Param("slash_id", "Penalty ID", "int", 0)],
                    sudo=True,
                    help_text="Execute a pending penalty against a validator",
                    mode="dev"),
            Command("8", "Report Misbehavior", "submit",
                    pallet="Validator", function="report_evidence",
                    params=[
                        Param("validator", "Validator identity", "actor"),
                        Param("violation", "Severity level", "enum",
                              options=["Minor", "Moderate", "Severe", "Critical"]),
                    ],
                    help_text="Report evidence of validator misbehavior",
                    mode="dev"),
            Command("---", "Lookups", "separator"),
            Command("a", "Validator Info", "query",
                    pallet="Validator", function="Validators",
                    params=[Param("validator", "Validator identity", "actor")],
                    help_text="View details about a specific validator"),
            Command("b", "Validator Count / Total Stake", "custom",
                    custom_handler="_validator_count_stake",
                    help_text="See how many validators and total staked amount"),
            Command("c", "Pending Penalties", "query_map",
                    pallet="Validator", function="PendingSlashes",
                    help_text="List penalties waiting to be applied",
                    mode="dev"),
        ],
    ),

    # ------------------------------------------------------------------
    # POSITIONING
    # ------------------------------------------------------------------
    Domain(
        name="pbt", title="POSITION-BASED TRIANGULATION",
        number="5", shortcut="", group="positioning", check_epoch=True,
        mode="both", normal_title="LOCATION PROOF", normal_group="core",
        help_summary="Claim and verify physical positions via witnesses",
        instructions="""
  Position-Based Triangulation lets participants prove their physical
  location using witness attestations from nearby validators.

  TYPICAL FLOW:
    1. Set up validators at known positions (option 1 or 5 for auto)
    2. Claim your position (option 2)
    3. Nearby witnesses attest to your location (option 3)
    4. Verify the position once enough attestations arrive (option 4)

  TIP: Option 6 runs a complete test automatically.
""",
        commands=[
            Command("1", "Set Validator Position", "custom",
                    custom_handler="_pbt_set_position",
                    aliases=["position"],
                    help_text="Set the physical position of a validator node",
                    mode="dev"),
            Command("2", "Claim Position", "submit",
                    pallet="Presence", function="claim_position",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("position", "Claimed position", "position"),
                    ],
                    aliases=["claim"],
                    help_text="Declare your physical position for this time period"),
            Command("3", "Submit Witness Attestation", "submit",
                    pallet="Presence", function="submit_witness_attestation",
                    params=[
                        Param("target", "Target identity", "actor"),
                        Param("epoch", "Time period", "epoch"),
                        Param("latency_ms", "Latency ms", "int", 5),
                        Param("direct_connection", "Direct connection?", "bool", True),
                    ],
                    aliases=["attest"],
                    help_text="Confirm you witnessed another node at a position"),
            Command("4", "Verify Position", "submit",
                    pallet="Presence", function="verify_position",
                    params=[
                        Param("target", "Target identity", "actor"),
                        Param("epoch", "Time period", "epoch"),
                    ],
                    aliases=["verify"],
                    help_text="Check if a claimed position has enough attestations"),
            Command("---", "Automated", "separator"),
            Command("5", "Setup All Validators (auto)", "custom",
                    custom_handler="_auto_setup_validators",
                    aliases=["setup"],
                    help_text="Register 6 validators in hexagonal formation",
                    mode="dev"),
            Command("6", "Full PBT Test Flow (auto)", "custom",
                    custom_handler="_auto_pbt_test",
                    aliases=["test"],
                    help_text="Run a complete position verification test"),
            Command("---", "Lookups", "separator"),
            Command("a", "Position Claim", "query",
                    pallet="Presence", function="PositionClaims",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("actor", "Identity", "actor"),
                    ],
                    help_text="Look up a position claim for an identity"),
            Command("b", "Attestation Count", "query",
                    pallet="Presence", function="AttestationCount",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("actor", "Identity", "actor"),
                    ],
                    help_text="Check how many attestations an identity has received"),
            Command("c", "Validator Positions", "query_map",
                    pallet="Presence", function="ValidatorPositions",
                    help_text="List all known validator positions"),
        ],
    ),

    Domain(
        name="triangulation", title="SIGNAL TRIANGULATION",
        number="6", shortcut="tri", group="positioning",
        mode="dev",
        help_summary="Signal-based location reporting and fraud detection",
        instructions="""
  Signal Triangulation uses signal reports from multiple reporters
  to estimate device locations and detect fraudulent signals.

  TYPICAL FLOW:
    1. Register a reporter at a known position (option 1)
    2. Submit signal observations (option 3)
    3. The system estimates positions from multiple reports
    4. Submit fraud proofs if you detect anomalies (option 5)
""",
        commands=[
            Command("1", "Register Reporter", "submit",
                    pallet="Triangulation", function="register_reporter",
                    params=[Param("position", "Reporter position", "position")],
                    help_text="Register a new signal reporter at a position"),
            Command("2", "Deregister Reporter", "submit",
                    pallet="Triangulation", function="deregister_reporter",
                    params=[Param("reporter_id", "Reporter ID", "int", 0)],
                    help_text="Remove a signal reporter from the network"),
            Command("3", "Report Signal", "custom",
                    custom_handler="_triangulation_report_signal",
                    help_text="Submit a signal observation from a reporter"),
            Command("4", "Update Reporter Position", "submit",
                    pallet="Triangulation", function="update_reporter_position",
                    params=[
                        Param("reporter_id", "Reporter ID", "int", 0),
                        Param("new_position", "New position", "position"),
                    ],
                    help_text="Move a reporter to a new position"),
            Command("5", "Submit Fraud Proof", "custom",
                    custom_handler="_triangulation_fraud_proof",
                    help_text="Report evidence of fraudulent signal data"),
            Command("6", "Resolve Fraud Case [admin]", "submit",
                    pallet="Triangulation", function="resolve_fraud_case",
                    params=[
                        Param("reporter_id", "Reporter ID", "int", 0),
                        Param("guilty", "Guilty?", "bool", True),
                    ],
                    sudo=True,
                    help_text="Decide the outcome of a fraud investigation"),
            Command("---", "Lookups", "separator"),
            Command("a", "Reporter Info", "query",
                    pallet="Triangulation", function="Reporters",
                    params=[Param("reporter_id", "ID", "int", 0)],
                    help_text="View details about a signal reporter"),
            Command("b", "Device / Ghost Count", "custom",
                    custom_handler="_triangulation_counts",
                    help_text="See how many devices and ghost signals exist"),
            Command("c", "Fraud Cases", "query_map",
                    pallet="Triangulation", function="FraudCases",
                    help_text="List all open fraud investigations"),
        ],
    ),

    # ------------------------------------------------------------------
    # SECURITY
    # ------------------------------------------------------------------
    Domain(
        name="dispute", title="DISPUTE RESOLUTION",
        number="7", shortcut="dis", group="security",
        mode="both", normal_title="DISPUTES", normal_group="security",
        help_summary="Open disputes, submit evidence, resolve cases",
        instructions="""
  Dispute Resolution handles disagreements about validator behavior.
  Anyone can open a dispute and submit evidence for review.

  TYPICAL FLOW:
    1. Open a dispute against a validator (option 1)
    2. Submit supporting evidence (option 2)
    3. An admin resolves the dispute (option 3)
""",
        commands=[
            Command("1", "Open Dispute", "submit",
                    pallet="Dispute", function="open_dispute",
                    params=[
                        Param("target", "Target validator", "actor"),
                        Param("violation", "Violation", "enum",
                              options=["Minor", "Moderate", "Severe", "Critical"]),
                    ],
                    help_text="Start a dispute against a validator for misbehavior"),
            Command("2", "Submit Evidence", "submit",
                    pallet="Dispute", function="submit_evidence",
                    params=[
                        Param("dispute_id", "Dispute ID", "int", 0),
                        Param("data_hash", "Evidence ID (32-byte hex)", "h256"),
                    ],
                    help_text="Add evidence to support an open dispute"),
            Command("3", "Resolve Dispute [admin]", "submit",
                    pallet="Dispute", function="resolve_dispute",
                    params=[
                        Param("dispute_id", "Dispute ID", "int", 0),
                        Param("outcome", "Outcome", "enum",
                              options=["ValidatorSlashed", "DisputeRejected",
                                       "InsufficientEvidence"]),
                    ],
                    sudo=True,
                    help_text="Decide the outcome of a dispute case",
                    mode="dev"),
            Command("4", "Reject Dispute [admin]", "submit",
                    pallet="Dispute", function="reject_dispute",
                    params=[
                        Param("dispute_id", "Dispute ID", "int", 0),
                        Param("reason", "Reason", "enum",
                              options=["InsufficientEvidence",
                                       "ResolutionPeriodExpired",
                                       "InvalidTarget"]),
                    ],
                    sudo=True,
                    help_text="Dismiss a dispute as unfounded",
                    mode="dev"),
            Command("---", "Lookups", "separator"),
            Command("a", "Dispute Info", "query",
                    pallet="Dispute", function="Disputes",
                    params=[Param("dispute_id", "Dispute ID", "int", 0)],
                    help_text="View details about a specific dispute"),
            Command("b", "Open Disputes", "query",
                    pallet="Dispute", function="OpenDisputes",
                    help_text="List all disputes that are still open"),
        ],
    ),

    Domain(
        name="zk", title="PRIVACY PROOFS (Zero-Knowledge)",
        number="8", shortcut="", group="security",
        mode="dev",
        help_summary="Verify claims without revealing private data",
        instructions="""
  Privacy Proofs use zero-knowledge cryptography to verify claims
  without revealing the underlying private data.

  PROOF TYPES:
    - Share proofs: verify a secret share is valid
    - Presence proofs: verify someone was present without details
    - Access proofs: verify authorization without credentials
    - SNARK proofs: verify using registered circuits
""",
        commands=[
            Command("1", "Verify Share Proof", "custom",
                    custom_handler="_zk_share_proof",
                    help_text="Verify a proof that a secret share is valid"),
            Command("2", "Verify Presence Proof", "custom",
                    custom_handler="_zk_presence_proof",
                    help_text="Verify a presence claim without revealing details"),
            Command("3", "Verify Access Proof", "custom",
                    custom_handler="_zk_access_proof",
                    help_text="Verify someone has access without revealing credentials"),
            Command("4", "Register SNARK Circuit [admin]", "custom",
                    custom_handler="_zk_register_circuit",
                    help_text="Add a new proof circuit to the registry"),
            Command("5", "Verify SNARK", "custom",
                    custom_handler="_zk_verify_snark",
                    help_text="Check a SNARK proof against a registered circuit"),
            Command("6", "Consume Unique-Use Token", "submit",
                    pallet="Zk", function="consume_nullifier",
                    params=[Param("nullifier", "Unique-use token (32-byte hex)", "h256")],
                    help_text="Mark a one-time token as used to prevent replay"),
            Command("7", "Add/Remove Trusted Verifier [admin]", "custom",
                    custom_handler="_zk_trusted_verifier",
                    help_text="Manage which accounts can verify proofs"),
            Command("---", "Lookups", "separator"),
            Command("a", "Verification Count", "query",
                    pallet="Zk", function="VerificationCount",
                    help_text="See how many proofs have been verified"),
        ],
    ),

    Domain(
        name="vault", title="SECURE VAULT (Shared Keys)",
        number="9", shortcut="", group="security",
        mode="both", normal_title="VAULT", normal_group="security",
        help_summary="Secure shared key management with split secrets",
        instructions="""
  The Secure Vault uses threshold cryptography (t-of-n) so that
  a secret is split among multiple members and can only be
  reconstructed when enough members cooperate.

  TYPICAL FLOW:
    1. Create a vault with a threshold (option 1)
    2. Add members (option 2)
    3. Activate the vault (option 3)
    4. Members commit and reveal their shares (options 4-5)

  EXAMPLE: A 2-of-3 vault needs any 2 of 3 members to reconstruct.
""",
        commands=[
            Command("1", "Create Vault", "submit",
                    pallet="Vault", function="create_vault",
                    params=[
                        Param("owner", "Owner identity", "actor"),
                        Param("threshold", "Minimum signers needed (t)", "int", 2),
                        Param("ring_size", "Total members (n)", "int", 3),
                        Param("secret_hash", "Secret ID (32-byte hex)", "h256"),
                    ],
                    help_text="Create a new shared vault that requires multiple signers"),
            Command("2", "Add Member", "submit",
                    pallet="Vault", function="add_member",
                    params=[
                        Param("vault_id", "Vault ID", "int", 0),
                        Param("member", "Member identity", "actor"),
                        Param("role", "Role", "enum",
                              options=["Participant", "Guardian", "Owner"]),
                    ],
                    help_text="Add a new member to a vault"),
            Command("3", "Activate Vault", "submit",
                    pallet="Vault", function="activate_vault",
                    params=[Param("vault_id", "Vault ID", "int", 0)],
                    help_text="Activate a vault once all members are added"),
            Command("4", "Commit Share", "submit",
                    pallet="Vault", function="commit_share",
                    params=[
                        Param("vault_id", "Vault ID", "int", 0),
                        Param("commitment", "Commitment ID (32-byte hex)", "h256"),
                    ],
                    help_text="Submit your secret share commitment to the vault"),
            Command("5", "Reveal Share", "submit",
                    pallet="Vault", function="reveal_share",
                    params=[Param("share_id", "Share ID", "int", 0)],
                    help_text="Reveal your secret share for reconstruction"),
            Command("6", "Initiate Recovery", "submit",
                    pallet="Vault", function="initiate_recovery",
                    params=[Param("vault_id", "Vault ID", "int", 0)],
                    help_text="Start the vault recovery process"),
            Command("7", "Lock Vault", "submit",
                    pallet="Vault", function="lock_vault",
                    params=[Param("vault_id", "Vault ID", "int", 0)],
                    help_text="Lock a vault to prevent access"),
            Command("8", "Dissolve Vault", "submit",
                    pallet="Vault", function="dissolve_vault",
                    params=[Param("vault_id", "Vault ID", "int", 0)],
                    help_text="Permanently dissolve a vault and release its keys"),
            Command("---", "Lookups", "separator"),
            Command("a", "Vault Info", "query",
                    pallet="Vault", function="Vaults",
                    params=[Param("vault_id", "ID", "int", 0)],
                    help_text="View details about a specific vault"),
            Command("---", "Secure Documents", "separator"),
            Command("9", "Secure a Document", "custom",
                    custom_handler="_vault_secure_file",
                    aliases=["secure", "up"],
                    help_text="Encrypt a file with threshold protection"),
            Command("10", "Unlock Document", "custom",
                    custom_handler="_vault_unlock_file",
                    aliases=["unlock"],
                    help_text="Reconstruct key and decrypt a vault file"),
            Command("11", "Vault Files", "custom",
                    custom_handler="_vault_list_files",
                    aliases=["files", "ls"],
                    help_text="List encrypted files in a vault"),
            Command("12", "Verify File Integrity", "custom",
                    custom_handler="_vault_verify_file",
                    aliases=["verify"],
                    help_text="Re-hash and verify encrypted file integrity"),
            Command("13", "Export Share (hex)", "custom",
                    custom_handler="_vault_export_share",
                    aliases=["export-share"],
                    help_text="Export a key share as hex for manual transfer"),
            Command("14", "Import Share (hex)", "custom",
                    custom_handler="_vault_import_share",
                    aliases=["import-share"],
                    help_text="Import a key share from hex string"),
            Command("---", "Dev Tools", "separator", mode="dev"),
            Command("d1", "Split Secret (Shamir)", "custom",
                    custom_handler="_vault_dev_split",
                    mode="dev",
                    help_text="Split a raw secret into Shamir shares"),
            Command("d2", "Reconstruct Secret", "custom",
                    custom_handler="_vault_dev_reconstruct",
                    mode="dev",
                    help_text="Reconstruct a secret from Shamir shares"),
            Command("d3", "Encrypt File (raw)", "custom",
                    custom_handler="_vault_dev_encrypt",
                    mode="dev",
                    help_text="Encrypt a file with a raw FEK"),
            Command("d4", "Decrypt File (raw)", "custom",
                    custom_handler="_vault_dev_decrypt",
                    mode="dev",
                    help_text="Decrypt a .enc file with a raw FEK"),
        ],
    ),

    # ------------------------------------------------------------------
    # IDENTITY
    # ------------------------------------------------------------------
    Domain(
        name="device", title="DEVICES",
        number="10", shortcut="dev", group="identity",
        mode="both", normal_title="MY DEVICES", normal_group="identity",
        help_summary="Register, activate, suspend, and monitor devices",
        instructions="""
  Devices represent physical hardware registered on the network.
  Each device has a trust score and must send heartbeats to stay active.

  TYPICAL FLOW:
    1. Register a device with its type and key (option 1)
    2. Activate the device (option 2)
    3. Send periodic heartbeats (option 6)
    4. Submit attestations to build trust (option 5)
""",
        commands=[
            Command("1", "Register Device", "submit",
                    pallet="Device", function="register_device",
                    params=[
                        Param("owner", "Owner identity", "actor"),
                        Param("device_type", "Type", "enum",
                              options=["Mobile", "Desktop", "Server",
                                       "IoT", "Hardware", "Virtual"]),
                        Param("public_key_hash", "Public key ID (32-byte hex)", "h256"),
                        Param("attestation_type", "Attestation type", "str",
                              "SelfSigned"),
                    ],
                    help_text="Register a new device on the network"),
            Command("2", "Activate / Reactivate Device", "custom",
                    custom_handler="_device_activate",
                    help_text="Turn on a device or bring it back online"),
            Command("3", "Suspend Device", "submit",
                    pallet="Device", function="suspend_device",
                    params=[
                        Param("device_id", "Device ID", "int", 0),
                        Param("reason", "Reason ID (32-byte hex)", "h256"),
                    ],
                    help_text="Temporarily disable a device"),
            Command("4", "Revoke / Mark Compromised", "custom",
                    custom_handler="_device_revoke",
                    help_text="Permanently revoke a device or flag it as compromised",
                    mode="dev"),
            Command("5", "Submit Attestation", "custom",
                    custom_handler="_device_attestation",
                    help_text="Submit a device attestation for trust verification",
                    mode="dev"),
            Command("6", "Record Heartbeat", "submit",
                    pallet="Device", function="record_heartbeat",
                    params=[
                        Param("device_id", "Device ID", "int", 0),
                        Param("sequence", "Sequence", "int", 1),
                    ],
                    help_text="Send a heartbeat to show the device is alive"),
            Command("7", "Update Trust Score [admin]", "submit",
                    pallet="Device", function="update_trust_score",
                    params=[
                        Param("device_id", "Device ID", "int", 0),
                        Param("new_score", "Score (0-100)", "int", 50),
                    ],
                    sudo=True,
                    help_text="Set the trust score for a device",
                    mode="dev"),
            Command("---", "Lookups", "separator"),
            Command("a", "Device Info", "query",
                    pallet="Device", function="Devices",
                    params=[Param("device_id", "ID", "int", 0)],
                    help_text="View details about a registered device"),
        ],
    ),

    Domain(
        name="lifecycle", title="IDENTITY LIFECYCLE",
        number="11", shortcut="life", group="identity",
        mode="dev",
        help_summary="Register identities, manage key rotation and destruction",
        instructions="""
  Identity Lifecycle manages the full life of network identities:
  registration, activation, key rotation, and destruction.

  TYPICAL FLOW:
    1. Register a new identity (option 1)
    2. Admin activates it (option 2)
    3. Rotate keys periodically (options 7-8)
    4. Destroy when no longer needed (options 4-6)

  KEY ROTATION: Always initiate first, then complete.
  DESTRUCTION: Requires multiple attestations for safety.
""",
        commands=[
            Command("1", "Register Identity", "submit",
                    pallet="Lifecycle", function="register_actor",
                    params=[Param("key_hash", "Key ID (32-byte hex)", "h256")],
                    help_text="Create a new identity on the network"),
            Command("2", "Activate Identity [admin]", "submit",
                    pallet="Lifecycle", function="activate_actor",
                    params=[Param("actor", "Identity", "actor")],
                    sudo=True,
                    help_text="Activate an identity so it can participate"),
            Command("3", "Suspend / Reactivate [admin]", "custom",
                    custom_handler="_lifecycle_suspend_reactivate",
                    help_text="Temporarily suspend or bring back an identity"),
            Command("4", "Initiate Destruction", "submit",
                    pallet="Lifecycle", function="initiate_destruction",
                    params=[
                        Param("reason", "Reason", "enum",
                              options=["OwnerRequest", "SecurityBreach",
                                       "Expiration", "ProtocolViolation",
                                       "Administrative"]),
                    ],
                    help_text="Start the process of permanently destroying an identity"),
            Command("5", "Attest Destruction", "submit",
                    pallet="Lifecycle", function="attest_destruction",
                    params=[
                        Param("target_actor", "Target identity", "actor"),
                        Param("signature_hash", "Signature ID (32-byte hex)", "h256"),
                    ],
                    help_text="Confirm you agree with an identity's destruction"),
            Command("6", "Cancel Destruction", "submit",
                    pallet="Lifecycle", function="cancel_destruction",
                    help_text="Stop an in-progress identity destruction"),
            Command("7", "Initiate Key Rotation", "submit",
                    pallet="Lifecycle", function="initiate_rotation",
                    params=[Param("new_key_hash", "New key ID (32-byte hex)", "h256")],
                    help_text="Start rotating your identity key to a new one"),
            Command("8", "Complete Key Rotation", "submit",
                    pallet="Lifecycle", function="complete_rotation",
                    help_text="Finish the key rotation and activate the new key"),
            Command("---", "Lookups", "separator"),
            Command("a", "Identity Info", "query",
                    pallet="Lifecycle", function="Actors",
                    params=[Param("actor", "Identity", "actor")],
                    help_text="View details about a registered identity"),
            Command("b", "Identity Count", "custom",
                    custom_handler="_lifecycle_count",
                    help_text="See how many identities are registered"),
        ],
    ),

    Domain(
        name="governance", title="PERMISSIONS & ACCESS",
        number="12", shortcut="gov", group="identity",
        mode="both", normal_title="PERMISSIONS", normal_group="identity",
        help_summary="Grant, revoke, and delegate access permissions",
        instructions="""
  Permissions & Access controls what identities can do on the network.
  Capabilities are fine-grained permissions that can be delegated.

  PERMISSIONS BITMASK:
    R=1  W=2  X=4  D=8  A=16
    Example: 7 = Read + Write + Execute

  TYPICAL FLOW:
    1. Grant a capability to someone (option 1)
    2. They can delegate it further (option 3)
    3. Revoke when no longer needed (option 2)
""",
        commands=[
            Command("1", "Grant Capability", "custom",
                    custom_handler="_gov_grant",
                    help_text="Give someone a new access permission"),
            Command("2", "Revoke Capability", "submit",
                    pallet="Governance", function="revoke_capability",
                    params=[Param("capability_id", "Capability ID", "int", 0)],
                    help_text="Remove an access permission"),
            Command("3", "Delegate Capability", "custom",
                    custom_handler="_gov_delegate",
                    help_text="Pass one of your permissions to someone else"),
            Command("4", "Update Permissions", "submit",
                    pallet="Governance", function="update_capability",
                    params=[
                        Param("capability_id", "Capability ID", "int", 0),
                        Param("new_permissions", "New permissions", "int", 7),
                    ],
                    help_text="Change what a capability allows"),
            Command("---", "Lookups", "separator"),
            Command("a", "Capability Info", "query",
                    pallet="Governance", function="Capabilities",
                    params=[Param("capability_id", "ID", "int", 0)],
                    help_text="View details about an access permission"),
        ],
    ),

    # ------------------------------------------------------------------
    # INTELLIGENCE
    # ------------------------------------------------------------------
    Domain(
        name="semantic", title="TRUST RELATIONSHIPS",
        number="13", shortcut="sem", group="intelligence",
        mode="both", normal_title="TRUST", normal_group="identity",
        help_summary="Create and manage trust relationships between identities",
        instructions="""
  Trust Relationships create verifiable connections between identities.
  Each relationship has a trust level (0-100) and can be bidirectional.

  TYPICAL FLOW:
    1. Create a relationship request (option 1)
    2. The other party accepts (option 2)
    3. Adjust trust levels over time (option 4)
    4. Revoke if the relationship ends (option 3)
""",
        commands=[
            Command("1", "Create Relationship", "custom",
                    custom_handler="_semantic_create",
                    help_text="Start a new trust relationship with another identity"),
            Command("2", "Accept Relationship", "submit",
                    pallet="Semantic", function="accept_relationship",
                    params=[Param("relationship_id", "Relationship ID", "int", 0)],
                    help_text="Accept an incoming relationship request"),
            Command("3", "Revoke Relationship", "submit",
                    pallet="Semantic", function="revoke_relationship",
                    params=[Param("relationship_id", "Relationship ID", "int", 0)],
                    help_text="End an existing relationship"),
            Command("4", "Update Trust Level", "submit",
                    pallet="Semantic", function="update_trust_level",
                    params=[
                        Param("relationship_id", "Relationship ID", "int", 0),
                        Param("new_trust_level", "New trust (0-100)", "int", 50),
                    ],
                    help_text="Change how much you trust someone"),
            Command("5", "Request Discovery", "submit",
                    pallet="Semantic", function="request_discovery",
                    fixed_params={"criteria": {
                        "min_trust_level": 0,
                        "relationship_type": None,
                        "max_hops": 2,
                        "include_pending": False,
                    }},
                    help_text="Search for new identities to connect with"),
            Command("6", "Update Profile", "submit",
                    pallet="Semantic", function="update_profile",
                    params=[Param("discovery_enabled", "Discovery enabled?",
                                  "bool", True)],
                    help_text="Toggle whether others can find you"),
            Command("---", "Lookups", "separator"),
            Command("a", "Relationship Info", "query",
                    pallet="Semantic", function="Relationships",
                    params=[Param("relationship_id", "ID", "int", 0)],
                    help_text="View details about a trust relationship"),
        ],
    ),

    Domain(
        name="boomerang", title="ROUND-TRIP VERIFICATION",
        number="14", shortcut="boom", group="intelligence",
        mode="dev",
        help_summary="Round-trip path verification between identities",
        instructions="""
  Round-Trip Verification tests network paths by sending a signal
  through a chain of identities and verifying it returns.

  TYPICAL FLOW:
    1. Initiate a path to a target (option 1)
    2. Each intermediate identity records a hop (option 2)
    3. The path completes when the signal returns
    4. Extend timeout if needed (option 3)

  TIMEOUT: Default 30 seconds, max 60 second extension.
""",
        commands=[
            Command("1", "Initiate Path", "submit",
                    pallet="Boomerang", function="initiate_path",
                    params=[Param("target", "Target identity", "actor")],
                    help_text="Start a round-trip verification to a target"),
            Command("2", "Record Hop", "submit",
                    pallet="Boomerang", function="record_hop",
                    params=[
                        Param("path_id", "Path ID", "int", 0),
                        Param("to_actor", "Next identity", "actor"),
                        Param("signature_hash", "Signature ID (32-byte hex)", "h256"),
                    ],
                    help_text="Record a hop along the verification path"),
            Command("3", "Extend Timeout", "submit",
                    pallet="Boomerang", function="extend_timeout",
                    params=[Param("path_id", "Path ID", "int", 0)],
                    help_text="Give a path more time to complete"),
            Command("4", "Fail Path [admin]", "submit",
                    pallet="Boomerang", function="fail_path",
                    params=[
                        Param("path_id", "Path ID", "int", 0),
                        Param("reason", "Failure reason", "enum",
                              options=["InvalidHop", "MismatchedReturn",
                                       "VerificationFailed", "MaxHopsExceeded"]),
                    ],
                    sudo=True,
                    help_text="Mark a verification path as failed"),
            Command("---", "Lookups", "separator"),
            Command("a", "Path Info", "query",
                    pallet="Boomerang", function="Paths",
                    params=[Param("path_id", "ID", "int", 0)],
                    help_text="View details about a verification path"),
            Command("b", "Active Paths", "query",
                    pallet="Boomerang", function="ActivePaths",
                    help_text="List all paths currently in progress"),
        ],
    ),

    Domain(
        name="autonomous", title="BEHAVIOR PATTERNS",
        number="15", shortcut="auto", group="intelligence",
        mode="dev",
        help_summary="Track behavior patterns and anomaly detection",
        instructions="""
  Behavior Patterns tracks and classifies identity behaviors
  to detect anomalies and build reputation profiles.

  TYPICAL FLOW:
    1. Create a profile for an identity (option 1)
    2. Record behaviors as they happen (option 2)
    3. Register known patterns (option 3)
    4. Match behaviors against patterns (option 4)
    5. Flag suspicious identities (option 7)
""",
        commands=[
            Command("1", "Create Profile", "submit",
                    pallet="Autonomous", function="create_profile",
                    params=[Param("actor", "Identity", "actor")],
                    help_text="Create a behavior profile for an identity"),
            Command("2", "Record Behavior", "submit",
                    pallet="Autonomous", function="record_behavior",
                    params=[
                        Param("actor", "Identity", "actor"),
                        Param("behavior_type", "Behavior", "enum",
                              options=["PresencePattern", "InteractionPattern",
                                       "TemporalPattern", "TransactionPattern",
                                       "NetworkPattern"]),
                        Param("data_hash", "Data ID (32-byte hex)", "h256"),
                    ],
                    help_text="Log a behavior observation for an identity"),
            Command("3", "Register Pattern [admin]", "submit",
                    pallet="Autonomous", function="register_pattern",
                    params=[
                        Param("behavior_type", "Behavior", "enum",
                              options=["PresencePattern", "InteractionPattern",
                                       "TemporalPattern", "TransactionPattern",
                                       "NetworkPattern"]),
                        Param("signature_hash", "Signature ID (32-byte hex)", "h256"),
                        Param("classification", "Classification", "enum",
                              options=["Normal", "PotentiallyAutomated",
                                       "Automated", "Anomalous", "Malicious"]),
                    ],
                    sudo=True,
                    help_text="Register a known behavior pattern for matching"),
            Command("4", "Match Behavior [admin]", "submit",
                    pallet="Autonomous", function="match_behavior",
                    params=[
                        Param("behavior_id", "Behavior ID", "int", 0),
                        Param("actor", "Identity", "actor"),
                        Param("pattern_id", "Pattern ID", "int", 0),
                    ],
                    sudo=True,
                    help_text="Check if a behavior matches a known pattern"),
            Command("5", "Classify Pattern [admin]", "submit",
                    pallet="Autonomous", function="classify_pattern",
                    params=[
                        Param("pattern_id", "Pattern ID", "int", 0),
                        Param("classification", "Classification", "enum",
                              options=["Normal", "PotentiallyAutomated",
                                       "Automated", "Anomalous", "Malicious"]),
                        Param("confidence_score", "Confidence (0-100)", "int", 80),
                    ],
                    sudo=True,
                    help_text="Label a pattern as normal or anomalous"),
            Command("6", "Update Status [admin]", "submit",
                    pallet="Autonomous", function="update_status",
                    params=[
                        Param("actor", "Identity", "actor"),
                        Param("new_status", "Status", "enum",
                              options=["Unknown", "Human", "Suspected",
                                       "Confirmed", "UnderReview", "Flagged"]),
                    ],
                    sudo=True,
                    help_text="Change the behavior monitoring status of an identity"),
            Command("7", "Flag Identity [admin]", "submit",
                    pallet="Autonomous", function="flag_actor",
                    params=[
                        Param("actor", "Identity", "actor"),
                        Param("reason", "Reason ID (32-byte hex)", "h256"),
                    ],
                    sudo=True,
                    help_text="Flag an identity for suspicious behavior"),
            Command("---", "Lookups", "separator"),
            Command("a", "Identity Profile", "query",
                    pallet="Autonomous", function="ActorProfiles",
                    params=[Param("actor", "Identity", "actor")],
                    help_text="View the behavior profile of an identity"),
            Command("b", "Pattern Count", "query",
                    pallet="Autonomous", function="PatternCount",
                    help_text="See how many patterns have been registered"),
        ],
    ),

    Domain(
        name="octopus", title="MULTI-NODE CLUSTERS",
        number="16", shortcut="oct", group="intelligence",
        mode="dev",
        help_summary="Multi-node orchestration with sub-node management",
        instructions="""
  Multi-Node Clusters let multiple sub-nodes work together as a
  single logical entity for distributed processing.

  TYPICAL FLOW:
    1. Create a cluster (option 1)
    2. Register and activate sub-nodes (options 2-3)
    3. Sub-nodes send heartbeats (option 8)
    4. Monitor throughput (options 5, 7)
    5. Evaluate if scaling is needed (option 6)

  MAX SUB-NODES: 8 per cluster.
""",
        commands=[
            Command("1", "Create Cluster", "custom",
                    custom_handler="_octopus_create_cluster",
                    help_text="Create a new multi-node cluster"),
            Command("2", "Register Subnode", "custom",
                    custom_handler="_octopus_register_subnode",
                    help_text="Add a sub-node to a cluster"),
            Command("3", "Activate Subnode", "custom",
                    custom_handler="_octopus_activate_subnode",
                    help_text="Bring a sub-node online"),
            Command("4", "Start Deactivation", "custom",
                    custom_handler="_octopus_start_deactivation",
                    help_text="Begin shutting down a sub-node gracefully"),
            Command("5", "Update Cluster Throughput", "custom",
                    custom_handler="_octopus_update_throughput",
                    help_text="Report the cluster's throughput score"),
            Command("6", "Evaluate Scaling", "custom",
                    custom_handler="_octopus_evaluate_scaling",
                    help_text="Check if the cluster should scale up or down"),
            Command("7", "Update Subnode Throughput", "custom",
                    custom_handler="_octopus_update_subnode_throughput",
                    help_text="Report a sub-node's throughput score"),
            Command("8", "Record Heartbeat", "custom",
                    custom_handler="_octopus_record_heartbeat",
                    help_text="Send a heartbeat for a sub-node"),
            Command("9", "Record Device Observation", "custom",
                    custom_handler="_octopus_device_observation",
                    help_text="Log a device observation from a sub-node"),
            Command("10", "Record Position Confirmation", "custom",
                    custom_handler="_octopus_position_confirmation",
                    help_text="Confirm a sub-node's physical position"),
            Command("11", "Heartbeat with Device Proof", "custom",
                    custom_handler="_octopus_heartbeat_device_proof",
                    help_text="Send a heartbeat with attached device proof"),
            Command("12", "Set Fusion Weights", "custom",
                    custom_handler="_octopus_set_fusion_weights",
                    help_text="Configure how signals are combined in the cluster"),
            Command("---", "Lookups", "separator"),
            Command("a", "Cluster Info", "query",
                    pallet="Octopus", function="Clusters",
                    params=[Param("cluster_id", "ID", "int", 0)],
                    help_text="View details about a cluster"),
            Command("b", "Subnode Info", "query",
                    pallet="Octopus", function="Subnodes",
                    params=[Param("subnode_id", "ID", "int", 0)],
                    help_text="View details about a sub-node"),
            Command("c", "Cluster Count", "query",
                    pallet="Octopus", function="ClusterCount",
                    help_text="See how many clusters exist"),
        ],
    ),

    Domain(
        name="storage", title="DATA STORAGE",
        number="17", shortcut="store", group="intelligence",
        mode="dev",
        help_summary="Time-period-bound encrypted data storage",
        instructions="""
  Data Storage provides encrypted, time-period-bound storage.
  Data is tied to a specific time period and can be archived
  when the period ends.

  TYPICAL FLOW:
    1. Store data with a key and type (option 1)
    2. Update or delete as needed (options 2-3)
    3. Admin finalizes storage when the period ends (option 5)

  DATA TYPES: Presence, Commitment, Proof, Metadata, Temporary
""",
        commands=[
            Command("1", "Store Data", "submit",
                    pallet="Storage", function="store_data",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("key", "Data key (32-byte hex)", "h256"),
                        Param("data_hash", "Data ID (32-byte hex)", "h256"),
                        Param("data_type", "Type", "enum",
                              options=["Presence", "Commitment", "Proof",
                                       "Metadata", "Temporary"]),
                        Param("size_bytes", "Size (bytes)", "int", 256),
                        Param("retention", "Retention policy", "enum",
                              options=["EpochBound", "TimeBound",
                                       "Persistent", "OneTime"]),
                    ],
                    help_text="Save encrypted data tied to a time period"),
            Command("2", "Update Data", "submit",
                    pallet="Storage", function="update_data",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("key", "Data key (32-byte hex)", "h256"),
                        Param("new_data_hash", "New data ID (32-byte hex)", "h256"),
                        Param("new_size", "New size", "int", 256),
                    ],
                    help_text="Replace existing stored data"),
            Command("3", "Delete Data", "submit",
                    pallet="Storage", function="delete_data",
                    params=[
                        Param("epoch", "Time period", "epoch"),
                        Param("key", "Data key (32-byte hex)", "h256"),
                    ],
                    help_text="Remove stored data"),
            Command("4", "Set Quota [admin]", "submit",
                    pallet="Storage", function="set_quota",
                    params=[
                        Param("actor", "Identity", "actor"),
                        Param("max_entries", "Max entries", "int", 100),
                        Param("max_bytes", "Max bytes", "int", 1000000),
                    ],
                    sudo=True,
                    help_text="Set storage limits for an identity"),
            Command("5", "Finalize Period Storage [admin]", "submit",
                    pallet="Storage", function="finalize_epoch",
                    params=[Param("epoch", "Time period", "epoch")],
                    sudo=True,
                    help_text="Lock storage for a completed time period"),
            Command("---", "Lookups", "separator"),
            Command("a", "Entry Count", "query",
                    pallet="Storage", function="EntryCount",
                    help_text="See how many data entries are stored"),
        ],
    ),

    # ------------------------------------------------------------------
    # STATUS
    # ------------------------------------------------------------------
    Domain(
        name="chain", title="CHAIN STATUS",
        number="18", shortcut="", group="status",
        mode="both", normal_title="STATUS", normal_group="tools",
        help_summary="Node health, blocks, balances, and pallets",
        instructions="""
  Chain Status shows the health and state of the connected node.
  Use this to verify the node is running and check balances.
""",
        commands=[
            Command("1", "Node health & info", "custom",
                    custom_handler="_chain_health",
                    help_text="Check if the node is running and healthy"),
            Command("2", "Latest block", "custom",
                    custom_handler="_chain_latest_block",
                    help_text="See the most recent block on the chain"),
            Command("3", "Runtime version", "custom",
                    custom_handler="_chain_runtime_version",
                    help_text="Check the runtime and spec version"),
            Command("4", "Account balances", "custom",
                    custom_handler="_chain_balances",
                    help_text="View token balances for an account"),
            Command("5", "Recent events", "custom",
                    custom_handler="_chain_events",
                    help_text="Show events from the latest block"),
            Command("6", "List pallets", "custom",
                    custom_handler="_chain_pallets",
                    help_text="List all pallets available in the runtime"),
        ],
    ),

    # ------------------------------------------------------------------
    # DEV TOOLS
    # ------------------------------------------------------------------
    Domain(
        name="blocks", title="BLOCK EXPLORER",
        number="19", shortcut="blk", group="devtools",
        mode="dev",
        help_summary="Inspect blocks, transactions, and finalization",
        instructions="""
  Block Explorer lets you inspect individual blocks, decode
  transactions, and view events. Useful for debugging what
  happened on-chain.
""",
        commands=[
            Command("1", "Get block by number", "custom",
                    custom_handler="_blocks_by_number",
                    help_text="Look up a block by its number"),
            Command("2", "Get block by hash", "custom",
                    custom_handler="_blocks_by_hash",
                    help_text="Look up a block by its hash"),
            Command("3", "Latest block detail", "custom",
                    custom_handler="_blocks_latest",
                    help_text="View full details of the latest block"),
            Command("4", "Decode transaction in block", "custom",
                    custom_handler="_blocks_decode_ext",
                    help_text="Decode and display a transaction from a block"),
            Command("5", "Block events", "custom",
                    custom_handler="_blocks_events",
                    help_text="View events emitted by a specific block"),
            Command("6", "Finalized head", "custom",
                    custom_handler="_blocks_finalized",
                    help_text="See the latest finalized block"),
            Command("7", "Compare blocks", "custom",
                    custom_handler="_blocks_compare",
                    help_text="Compare two blocks side by side"),
        ],
    ),

    Domain(
        name="inspect", title="STORAGE INSPECTOR",
        number="20", shortcut="si", group="devtools",
        mode="dev",
        help_summary="Query raw storage, enumerate keys, view proofs",
        instructions="""
  Storage Inspector provides low-level access to the chain's
  key-value storage. Advanced tool for debugging storage state.
""",
        commands=[
            Command("1", "Query storage by pallet + item", "custom",
                    custom_handler="_si_query_pallet",
                    help_text="Read a storage value by pallet and item name"),
            Command("2", "Raw storage key lookup", "custom",
                    custom_handler="_si_raw_key",
                    help_text="Look up a value using a raw storage key"),
            Command("3", "Enumerate keys by prefix", "custom",
                    custom_handler="_si_enum_keys",
                    help_text="List all storage keys matching a prefix"),
            Command("4", "Storage size", "custom",
                    custom_handler="_si_storage_size",
                    help_text="Check how much storage a key uses"),
            Command("5", "Storage diff between blocks", "custom",
                    custom_handler="_si_diff",
                    help_text="See what changed in storage between two blocks"),
            Command("6", "Storage proof (Merkle)", "custom",
                    custom_handler="_si_proof",
                    help_text="Get a Merkle proof for a storage value"),
        ],
    ),

    Domain(
        name="runtime", title="RUNTIME EXPLORER",
        number="21", shortcut="rt", group="devtools",
        mode="dev",
        help_summary="Explore pallets, calls, storage, events, errors",
        instructions="""
  Runtime Explorer lets you browse the runtime metadata: pallets,
  callable functions, storage items, events, and errors. Useful
  for discovering what the chain can do.
""",
        commands=[
            Command("1", "List all pallets", "custom",
                    custom_handler="_rt_list_pallets",
                    help_text="Show every pallet in the runtime"),
            Command("2", "Pallet detail", "custom",
                    custom_handler="_rt_pallet_detail",
                    help_text="View calls, storage, events, and errors for a pallet"),
            Command("3", "Runtime version", "custom",
                    custom_handler="_rt_version",
                    help_text="Check the current runtime version"),
            Command("4", "Search call by name", "custom",
                    custom_handler="_rt_search_call",
                    help_text="Find a callable function by name"),
            Command("5", "Search storage by name", "custom",
                    custom_handler="_rt_search_storage",
                    help_text="Find a storage item by name"),
            Command("6", "Search error by name", "custom",
                    custom_handler="_rt_search_error",
                    help_text="Find an error type by name"),
        ],
    ),

    Domain(
        name="network", title="NETWORK & PEERS",
        number="22", shortcut="net", group="devtools",
        mode="dev",
        help_summary="View peers, sync status, and manage connections",
        instructions="""
  Network & Peers shows connection status, peer information, and
  sync state. Useful for monitoring the network health.
""",
        commands=[
            Command("1", "Connected peers", "custom",
                    custom_handler="_net_peers",
                    help_text="List all peers this node is connected to"),
            Command("2", "Node identity", "custom",
                    custom_handler="_net_identity",
                    help_text="Show this node's network identity"),
            Command("3", "Sync state", "custom",
                    custom_handler="_net_sync",
                    help_text="Check if the node is synced with the network"),
            Command("4", "Node health", "custom",
                    custom_handler="_net_health",
                    help_text="Check the health of this node"),
            Command("5", "Node roles", "custom",
                    custom_handler="_net_roles",
                    help_text="See what roles this node has (full, authority, etc.)"),
            Command("6", "Chain type", "custom",
                    custom_handler="_net_chain_type",
                    help_text="Check whether this is a devnet, testnet, or mainnet"),
            Command("7", "Pending transactions", "custom",
                    custom_handler="_net_pending",
                    help_text="Show transactions waiting in the pool"),
            Command("8", "Add/Remove reserved peer", "custom",
                    custom_handler="_net_reserved_peer",
                    help_text="Manage the reserved peer list"),
        ],
    ),

    Domain(
        name="crypto", title="CRYPTO TOOLKIT",
        number="23", shortcut="cr", group="devtools",
        mode="dev",
        help_summary="Keypairs, hashing, signing, SCALE encoding",
        instructions="""
  Crypto Toolkit provides cryptographic utilities: key generation,
  hashing, signing, and SCALE encoding/decoding. These are local
  operations that don't interact with the chain.
""",
        commands=[
            Command("1", "Generate keypair", "custom",
                    custom_handler="_crypto_generate",
                    help_text="Create a new public/private key pair"),
            Command("2", "Derive from URI", "custom",
                    custom_handler="_crypto_derive",
                    help_text="Derive a key from a secret URI or mnemonic"),
            Command("3", "SS58 encode/decode", "custom",
                    custom_handler="_crypto_ss58",
                    help_text="Convert between SS58 addresses and raw bytes"),
            Command("4", "Blake2b-256 hash", "custom",
                    custom_handler="_crypto_blake2b",
                    help_text="Hash data using Blake2b-256"),
            Command("5", "Keccak-256 hash", "custom",
                    custom_handler="_crypto_keccak",
                    help_text="Hash data using Keccak-256"),
            Command("6", "TwoX128 hash", "custom",
                    custom_handler="_crypto_twox128",
                    help_text="Hash data using TwoX128"),
            Command("7", "Build storage key", "custom",
                    custom_handler="_crypto_storage_key",
                    help_text="Construct a storage key from pallet and item names"),
            Command("8", "SCALE encode", "custom",
                    custom_handler="_crypto_scale_encode",
                    help_text="Encode a value in SCALE binary format"),
            Command("9", "SCALE decode", "custom",
                    custom_handler="_crypto_scale_decode",
                    help_text="Decode a SCALE-encoded value"),
            Command("10", "Sign message", "custom",
                    custom_handler="_crypto_sign",
                    help_text="Sign a message with a private key"),
            Command("11", "Verify signature", "custom",
                    custom_handler="_crypto_verify",
                    help_text="Check if a signature is valid"),
            Command("12", "Random 32-byte hex", "custom",
                    custom_handler="_crypto_random",
                    help_text="Generate a random 32-byte hex value"),
        ],
    ),

    Domain(
        name="accounts", title="ACCOUNTS",
        number="24", shortcut="acct", group="devtools",
        mode="both", normal_title="MY ACCOUNT", normal_group="tools",
        help_summary="Account info, balances, nonces, fee estimation",
        instructions="""
  Accounts shows detailed information about test accounts including
  balances, nonces, and fee estimation. Useful for checking account
  state before and after transactions.
""",
        commands=[
            Command("1", "Full account info", "custom",
                    custom_handler="_acct_full_info",
                    help_text="View complete details for an account"),
            Command("2", "Account nonce", "custom",
                    custom_handler="_acct_nonce",
                    help_text="Check the current transaction count for an account"),
            Command("3", "All balances", "custom",
                    custom_handler="_acct_balances",
                    help_text="View free, reserved, and total balances"),
            Command("4", "Fee estimation", "custom",
                    custom_handler="_acct_fee",
                    help_text="Estimate the fee for a transaction",
                    mode="dev"),
            Command("5", "Dry run transaction", "custom",
                    custom_handler="_acct_dry_run",
                    help_text="Test a transaction without submitting it",
                    mode="dev"),
        ],
    ),

    Domain(
        name="events", title="EVENTS",
        number="25", shortcut="ev", group="devtools",
        mode="dev",
        help_summary="Decode and filter blockchain events",
        instructions="""
  Events are emitted by the chain when state changes happen.
  Use this tool to decode events, filter by pallet, and browse
  recent history.
""",
        commands=[
            Command("1", "Events at latest block", "custom",
                    custom_handler="_ev_latest",
                    help_text="Show all events from the most recent block"),
            Command("2", "Events at block N", "custom",
                    custom_handler="_ev_at_block",
                    help_text="Show events from a specific block number"),
            Command("3", "Filter by pallet", "custom",
                    custom_handler="_ev_filter",
                    help_text="Show only events from a specific pallet"),
            Command("4", "Event history (last N blocks)", "custom",
                    custom_handler="_ev_history",
                    help_text="Browse events across recent blocks"),
            Command("5", "List all event types", "custom",
                    custom_handler="_ev_types",
                    help_text="Show every event type the runtime can emit"),
        ],
    ),

    # ------------------------------------------------------------------
    # DEV EXTENSIONS
    # ------------------------------------------------------------------
    Domain(
        name="devx", title="DEV EXTENSIONS",
        number="27", shortcut="dx", group="devtools",
        mode="dev",
        help_summary="Raw extrinsic builder, batch mode, benchmarks",
        commands=[
            Command("1", "Raw Extrinsic Builder", "custom",
                    custom_handler="_devx_raw_extrinsic",
                    help_text="Compose and submit any extrinsic manually"),
            Command("2", "Batch Transactions", "custom",
                    custom_handler="_devx_batch",
                    help_text="Send multiple transactions in a batch"),
            Command("3", "Storage Key Calculator", "custom",
                    custom_handler="_devx_storage_key_calc",
                    help_text="Calculate storage keys with full hash breakdown"),
            Command("4", "Weight Estimation", "custom",
                    custom_handler="_devx_weight_estimate",
                    help_text="Estimate weight for any call"),
            Command("5", "Metadata Explorer", "custom",
                    custom_handler="_devx_metadata_explorer",
                    help_text="Deep dive into runtime type registry"),
            Command("6", "Performance Benchmark", "custom",
                    custom_handler="_devx_benchmark",
                    help_text="Measure tx throughput and block time"),
            Command("7", "Chain State Snapshot", "custom",
                    custom_handler="_devx_snapshot",
                    help_text="Save/restore chain state for specific keys"),
            Command("8", "Event Stream (poll)", "custom",
                    custom_handler="_devx_event_stream",
                    help_text="Poll for new events every N seconds"),
        ],
    ),
]


# ---------------------------------------------------------------------------
# Lookup helpers
# ---------------------------------------------------------------------------

_DOMAIN_INDEX = {}  # built lazily


def _build_index():
    """Build lookup index from DOMAINS list."""
    global _DOMAIN_INDEX
    if _DOMAIN_INDEX:
        return
    for d in DOMAINS:
        _DOMAIN_INDEX[d.name] = d
        _DOMAIN_INDEX[d.number] = d
        if d.shortcut:
            _DOMAIN_INDEX[d.shortcut] = d


def find_domain(key):
    """Find a domain by name, number, or shortcut."""
    _build_index()
    return _DOMAIN_INDEX.get(key.lower())


def find_command(domain, key):
    """Find a command in a domain by key or alias."""
    for cmd in domain.commands:
        if cmd.key == key:
            return cmd
        if key in cmd.aliases:
            return cmd
    return None


def build_menu_aliases():
    """Auto-generate the menu alias map from registry."""
    aliases = {}
    for d in DOMAINS:
        aliases[d.number] = d.name
        aliases[d.name] = d.name
        if d.shortcut:
            aliases[d.shortcut] = d.name
    return aliases


def build_sub_aliases():
    """Auto-generate sub-command aliases from registry."""
    subs = {}
    for d in DOMAINS:
        sa = {}
        for cmd in d.commands:
            for alias in cmd.aliases:
                sa[alias] = cmd.key
        if sa:
            subs[d.name] = sa
    return subs


def build_cmd_names():
    """Build autocomplete command name list from registry."""
    names = ['help', 'use', 'status', 'menu', 'back', 'exit',
             'bootstrap', 'connect', 'test']
    for d in DOMAINS:
        if d.name not in names:
            names.append(d.name)
        if d.shortcut and d.shortcut not in names:
            names.append(d.shortcut)
    return names


def build_cmd_subs():
    """Build autocomplete sub-command map from registry."""
    subs = {'test': ['pop', 'pbt', 'commit'],
            'use': ['epoch', 'alice', 'bob', 'charlie',
                    'dave', 'eve', 'ferdie', 'clear']}
    for d in DOMAINS:
        cmd_aliases = []
        for cmd in d.commands:
            cmd_aliases.extend(cmd.aliases)
        if cmd_aliases:
            subs[d.name] = cmd_aliases
            if d.shortcut:
                subs[d.shortcut] = cmd_aliases
    return subs


GROUP_DISPLAY_ORDER = [
    ("core", "CORE"),
    ("positioning", "POSITIONING"),
    ("security", "SECURITY"),
    ("identity", "IDENTITY"),
    ("intelligence", "INTELLIGENCE"),
    ("status", "STATUS"),
    ("devtools", "DEV TOOLS"),
]

NORMAL_GROUP_ORDER = [
    ("getting-started", "GETTING STARTED"),
    ("core", "CORE PROTOCOL"),
    ("security", "SECURITY"),
    ("identity", "IDENTITY"),
    ("tools", "TOOLS"),
]


def get_domains_for_mode(mode):
    """Return domains visible in the given mode."""
    return [d for d in DOMAINS if d.mode in (mode, "both")]


def get_commands_for_mode(domain, mode):
    """Return commands visible in the given mode."""
    return [c for c in domain.commands if c.mode in (mode, "both")]


def get_group_display_order(mode):
    """Return group display order for the given mode."""
    if mode == "normal":
        return NORMAL_GROUP_ORDER
    return GROUP_DISPLAY_ORDER


def build_menu_aliases_for_mode(mode):
    """Auto-generate the menu alias map for domains visible in mode."""
    aliases = {}
    for d in get_domains_for_mode(mode):
        aliases[d.number] = d.name
        aliases[d.name] = d.name
        if d.shortcut:
            aliases[d.shortcut] = d.name
    return aliases


def build_cmd_names_for_mode(mode):
    """Build autocomplete command name list for the given mode."""
    names = ['help', 'use', 'status', 'menu', 'back', 'exit',
             'bootstrap', 'connect', 'test', 'mode']
    for d in get_domains_for_mode(mode):
        if d.name not in names:
            names.append(d.name)
        if d.shortcut and d.shortcut not in names:
            names.append(d.shortcut)
    return names


def build_cmd_subs_for_mode(mode):
    """Build autocomplete sub-command map for the given mode."""
    subs = {'test': ['pop', 'pbt', 'commit'],
            'use': ['epoch', 'alice', 'bob', 'charlie',
                    'dave', 'eve', 'ferdie', 'clear'],
            'mode': ['dev', 'normal']}
    for d in get_domains_for_mode(mode):
        cmd_aliases = []
        for cmd in get_commands_for_mode(d, mode):
            cmd_aliases.extend(cmd.aliases)
        if cmd_aliases:
            subs[d.name] = cmd_aliases
            if d.shortcut:
                subs[d.shortcut] = cmd_aliases
    return subs
