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


# ---------------------------------------------------------------------------
# Domain definitions
# ---------------------------------------------------------------------------

DOMAINS = [
    # ------------------------------------------------------------------
    # CORE
    # ------------------------------------------------------------------
    Domain(
        name="presence", title="PRESENCE PROTOCOL",
        number="2", shortcut="p", group="core", check_epoch=True,
        help_summary="Declare, vote on, and finalize proof-of-presence claims",
        commands=[
            Command("1", "Declare Presence", "submit",
                    pallet="Presence", function="declare_presence",
                    params=[Param("epoch", "Epoch", "epoch")],
                    aliases=["declare", "d"],
                    help_text="Declare your presence for a given time period"),
            Command("2", "Declare with Commitment", "custom",
                    custom_handler="_presence_commit",
                    aliases=["commit", "cm"],
                    help_text="Declare presence with a secret commitment"),
            Command("3", "Reveal Commitment", "submit",
                    pallet="Presence", function="reveal_commitment",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("secret", "Secret (hex from commit step)", "str"),
                        Param("randomness", "Randomness (hex from commit step)", "str"),
                    ],
                    aliases=["reveal", "rv"],
                    help_text="Reveal a previously submitted commitment"),
            Command("4", "Vote on Presence", "submit",
                    pallet="Presence", function="vote_presence",
                    params=[
                        Param("actor", "Target actor", "actor"),
                        Param("epoch", "Epoch", "epoch"),
                        Param("approve", "Approve?", "bool", True),
                    ],
                    aliases=["vote", "v"],
                    help_text="Cast your vote on a presence claim"),
            Command("5", "Finalize Presence", "submit",
                    pallet="Presence", function="finalize_presence",
                    params=[
                        Param("actor", "Target actor", "actor"),
                        Param("epoch", "Epoch", "epoch"),
                    ],
                    aliases=["finalize", "f"],
                    help_text="Lock in a presence claim after enough votes"),
            Command("6", "Slash Presence [sudo]", "submit",
                    pallet="Presence", function="slash_presence",
                    params=[
                        Param("actor", "Target actor", "actor"),
                        Param("epoch", "Epoch", "epoch"),
                    ],
                    sudo=True, aliases=["slash"],
                    help_text="Penalize a fraudulent presence claim"),
            Command("7", "Set Quorum Config [sudo]", "submit",
                    pallet="Presence", function="set_quorum_config",
                    params=[
                        Param("threshold", "Threshold", "int", 2),
                        Param("total", "Total", "int", 3),
                    ],
                    sudo=True, aliases=["quorum"],
                    help_text="Configure how many votes are needed"),
            Command("8", "Set Validator Status [sudo]", "submit",
                    pallet="Presence", function="set_validator_status",
                    params=[
                        Param("validator", "Validator", "actor"),
                        Param("active", "Active?", "bool", True),
                    ],
                    sudo=True, aliases=["validator-status"]),
            Command("9", "Set Epoch Active [sudo]", "submit",
                    pallet="Presence", function="set_epoch_active",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("active", "Active?", "bool", True),
                    ],
                    sudo=True, aliases=["epoch-active"]),
            Command("---", "Queries", "separator"),
            Command("a", "Current Epoch", "query",
                    pallet="Presence", function="CurrentEpoch",
                    help_text="Check what epoch the network is in"),
            Command("b", "Presence Record", "query",
                    pallet="Presence", function="Presences",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("actor", "Actor", "actor"),
                    ],
                    help_text="Look up presence status for an identity"),
            Command("c", "Vote Count", "query",
                    pallet="Presence", function="VoteCount",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("actor", "Actor", "actor"),
                    ]),
            Command("d", "Active Validators", "query_map",
                    pallet="Presence", function="ActiveValidators"),
            Command("e", "Commitment / Reveal Count", "custom",
                    custom_handler="_presence_commitment_count"),
        ],
    ),

    Domain(
        name="epoch", title="EPOCH MANAGEMENT",
        number="3", shortcut="e", group="core",
        help_summary="Schedule, start, close, and finalize time periods",
        commands=[
            Command("1", "Schedule Epoch [sudo]", "submit",
                    pallet="Epoch", function="schedule_epoch",
                    params=[
                        Param("start_block", "Start block", "int", 10),
                        Param("duration", "Duration (blocks)", "int", 60),
                    ],
                    sudo=True, aliases=["schedule"]),
            Command("2", "Start Epoch [sudo]", "submit",
                    pallet="Epoch", function="start_epoch",
                    params=[Param("epoch_id", "Epoch ID", "int", 1)],
                    sudo=True, aliases=["start"]),
            Command("3", "Close Epoch [sudo]", "submit",
                    pallet="Epoch", function="close_epoch",
                    params=[Param("epoch_id", "Epoch ID", "int", 1)],
                    sudo=True, aliases=["close"]),
            Command("4", "Finalize Epoch [sudo]", "submit",
                    pallet="Epoch", function="finalize_epoch",
                    params=[Param("epoch_id", "Epoch ID", "int", 1)],
                    sudo=True, aliases=["finalize"]),
            Command("5", "Register Participant", "submit",
                    pallet="Epoch", function="register_participant",
                    params=[Param("epoch_id", "Epoch ID", "int", 1)],
                    aliases=["register"]),
            Command("6", "Update Schedule [sudo]", "submit",
                    pallet="Epoch", function="update_schedule",
                    params=[
                        Param("duration", "Duration", "int", 60),
                        Param("grace_period", "Grace period", "int", 2),
                        Param("auto_transition", "Auto-transition?", "bool", True),
                    ],
                    sudo=True, aliases=["update"]),
            Command("7", "Force Transition [sudo]", "submit",
                    pallet="Epoch", function="force_transition",
                    params=[
                        Param("epoch_id", "Epoch ID", "int", 1),
                        Param("new_state", "State", "enum",
                              options=["Scheduled", "Active", "Closed", "Finalized"]),
                    ],
                    sudo=True, aliases=["force"]),
            Command("---", "Queries", "separator"),
            Command("a", "Current Epoch", "query",
                    pallet="Epoch", function="CurrentEpoch"),
            Command("b", "Epoch Info", "query",
                    pallet="Epoch", function="EpochInfo",
                    params=[Param("epoch", "Epoch", "epoch")]),
            Command("c", "Epoch Count", "query",
                    pallet="Epoch", function="EpochCount"),
            Command("d", "Epoch Schedule", "query",
                    pallet="Epoch", function="EpochSchedule"),
        ],
    ),

    Domain(
        name="validator", title="VALIDATOR OPERATIONS",
        number="4", shortcut="val", group="core",
        help_summary="Register, stake, activate, and manage validators",
        commands=[
            Command("1", "Register Validator", "submit",
                    pallet="Validator", function="register_validator",
                    params=[Param("stake", "Stake", "int", 1000000)],
                    aliases=["register"]),
            Command("2", "Activate Validator", "submit",
                    pallet="Validator", function="activate_validator",
                    aliases=["activate"]),
            Command("3", "Deactivate Validator", "submit",
                    pallet="Validator", function="deactivate_validator",
                    aliases=["deactivate"]),
            Command("4", "Withdraw Stake", "submit",
                    pallet="Validator", function="withdraw_stake",
                    aliases=["withdraw"]),
            Command("5", "Increase Stake", "submit",
                    pallet="Validator", function="increase_stake",
                    params=[Param("additional", "Additional stake", "int", 100000)],
                    aliases=["stake"]),
            Command("6", "Slash Validator [sudo]", "submit",
                    pallet="Validator", function="slash_validator",
                    params=[
                        Param("validator", "Validator", "actor"),
                        Param("violation", "Violation", "enum",
                              options=["Minor", "Moderate", "Severe", "Critical"]),
                    ],
                    sudo=True, aliases=["slash"]),
            Command("7", "Apply Slash [sudo]", "submit",
                    pallet="Validator", function="apply_slash",
                    params=[Param("slash_id", "Slash ID", "int", 0)],
                    sudo=True),
            Command("8", "Report Evidence", "submit",
                    pallet="Validator", function="report_evidence",
                    params=[
                        Param("validator", "Validator", "actor"),
                        Param("violation", "Violation", "enum",
                              options=["Minor", "Moderate", "Severe", "Critical"]),
                    ]),
            Command("---", "Queries", "separator"),
            Command("a", "Validator Info", "query",
                    pallet="Validator", function="Validators",
                    params=[Param("validator", "Validator", "actor")]),
            Command("b", "Validator Count / Total Stake", "custom",
                    custom_handler="_validator_count_stake"),
            Command("c", "Pending Slashes", "query_map",
                    pallet="Validator", function="PendingSlashes"),
        ],
    ),

    # ------------------------------------------------------------------
    # POSITIONING
    # ------------------------------------------------------------------
    Domain(
        name="pbt", title="POSITION-BASED TRIANGULATION",
        number="5", shortcut="", group="positioning", check_epoch=True,
        help_summary="Claim and verify physical positions via witnesses",
        commands=[
            Command("1", "Set Validator Position", "custom",
                    custom_handler="_pbt_set_position",
                    aliases=["position"]),
            Command("2", "Claim Position", "submit",
                    pallet="Presence", function="claim_position",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("position", "Claimed position", "position"),
                    ],
                    aliases=["claim"]),
            Command("3", "Submit Witness Attestation", "submit",
                    pallet="Presence", function="submit_witness_attestation",
                    params=[
                        Param("target", "Target actor", "actor"),
                        Param("epoch", "Epoch", "epoch"),
                        Param("latency_ms", "Latency ms", "int", 5),
                        Param("direct_connection", "Direct connection?", "bool", True),
                    ],
                    aliases=["attest"]),
            Command("4", "Verify Position", "submit",
                    pallet="Presence", function="verify_position",
                    params=[
                        Param("target", "Target", "actor"),
                        Param("epoch", "Epoch", "epoch"),
                    ],
                    aliases=["verify"]),
            Command("---", "Automated", "separator"),
            Command("5", "Setup All Validators (auto)", "custom",
                    custom_handler="_auto_setup_validators",
                    aliases=["setup"],
                    help_text="Register 6 validators in hexagonal formation"),
            Command("6", "Full PBT Test Flow (auto)", "custom",
                    custom_handler="_auto_pbt_test",
                    aliases=["test"],
                    help_text="Run a complete position verification test"),
            Command("---", "Queries", "separator"),
            Command("a", "Position Claim", "query",
                    pallet="Presence", function="PositionClaims",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("actor", "Actor", "actor"),
                    ]),
            Command("b", "Attestation Count", "query",
                    pallet="Presence", function="AttestationCount",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("actor", "Actor", "actor"),
                    ]),
            Command("c", "Validator Positions", "query_map",
                    pallet="Presence", function="ValidatorPositions"),
        ],
    ),

    Domain(
        name="triangulation", title="SIGNAL TRIANGULATION",
        number="6", shortcut="tri", group="positioning",
        help_summary="RSSI-based signal reporting and fraud detection",
        commands=[
            Command("1", "Register Reporter", "submit",
                    pallet="Triangulation", function="register_reporter",
                    params=[Param("position", "Reporter position", "position")]),
            Command("2", "Deregister Reporter", "submit",
                    pallet="Triangulation", function="deregister_reporter",
                    params=[Param("reporter_id", "Reporter ID", "int", 0)]),
            Command("3", "Report Signal", "custom",
                    custom_handler="_triangulation_report_signal"),
            Command("4", "Update Reporter Position", "submit",
                    pallet="Triangulation", function="update_reporter_position",
                    params=[
                        Param("reporter_id", "Reporter ID", "int", 0),
                        Param("new_position", "New position", "position"),
                    ]),
            Command("5", "Submit Fraud Proof", "custom",
                    custom_handler="_triangulation_fraud_proof"),
            Command("6", "Resolve Fraud Case [sudo]", "submit",
                    pallet="Triangulation", function="resolve_fraud_case",
                    params=[
                        Param("reporter_id", "Reporter ID", "int", 0),
                        Param("guilty", "Guilty?", "bool", True),
                    ],
                    sudo=True),
            Command("---", "Queries", "separator"),
            Command("a", "Reporter Info", "query",
                    pallet="Triangulation", function="Reporters",
                    params=[Param("reporter_id", "ID", "int", 0)]),
            Command("b", "Device / Ghost Count", "custom",
                    custom_handler="_triangulation_counts"),
            Command("c", "Fraud Cases", "query_map",
                    pallet="Triangulation", function="FraudCases"),
        ],
    ),

    # ------------------------------------------------------------------
    # SECURITY
    # ------------------------------------------------------------------
    Domain(
        name="dispute", title="DISPUTE RESOLUTION",
        number="7", shortcut="dis", group="security",
        help_summary="Open disputes, submit evidence, resolve cases",
        commands=[
            Command("1", "Open Dispute", "submit",
                    pallet="Dispute", function="open_dispute",
                    params=[
                        Param("target", "Target validator", "actor"),
                        Param("violation", "Violation", "enum",
                              options=["Minor", "Moderate", "Severe", "Critical"]),
                    ]),
            Command("2", "Submit Evidence", "submit",
                    pallet="Dispute", function="submit_evidence",
                    params=[
                        Param("dispute_id", "Dispute ID", "int", 0),
                        Param("data_hash", "Evidence hash", "h256"),
                    ]),
            Command("3", "Resolve Dispute [sudo]", "submit",
                    pallet="Dispute", function="resolve_dispute",
                    params=[
                        Param("dispute_id", "Dispute ID", "int", 0),
                        Param("outcome", "Outcome", "enum",
                              options=["ValidatorSlashed", "DisputeRejected",
                                       "InsufficientEvidence"]),
                    ],
                    sudo=True),
            Command("4", "Reject Dispute [sudo]", "submit",
                    pallet="Dispute", function="reject_dispute",
                    params=[
                        Param("dispute_id", "Dispute ID", "int", 0),
                        Param("reason", "Reason", "str", "InsufficientEvidence"),
                    ],
                    sudo=True),
            Command("---", "Queries", "separator"),
            Command("a", "Dispute Info", "query",
                    pallet="Dispute", function="Disputes",
                    params=[Param("dispute_id", "Dispute ID", "int", 0)]),
            Command("b", "Open Disputes", "query",
                    pallet="Dispute", function="OpenDisputes"),
        ],
    ),

    Domain(
        name="zk", title="ZERO-KNOWLEDGE PROOFS",
        number="8", shortcut="", group="security",
        help_summary="Verify zero-knowledge proofs and manage circuits",
        commands=[
            Command("1", "Verify Share Proof", "custom",
                    custom_handler="_zk_share_proof"),
            Command("2", "Verify Presence Proof", "custom",
                    custom_handler="_zk_presence_proof"),
            Command("3", "Verify Access Proof", "custom",
                    custom_handler="_zk_access_proof"),
            Command("4", "Register SNARK Circuit [sudo]", "custom",
                    custom_handler="_zk_register_circuit"),
            Command("5", "Verify SNARK", "custom",
                    custom_handler="_zk_verify_snark"),
            Command("6", "Consume Nullifier", "submit",
                    pallet="Zk", function="consume_nullifier",
                    params=[Param("nullifier", "Nullifier", "h256")]),
            Command("7", "Add/Remove Trusted Verifier [sudo]", "custom",
                    custom_handler="_zk_trusted_verifier"),
            Command("---", "Queries", "separator"),
            Command("a", "Verification Count", "query",
                    pallet="Zk", function="VerificationCount"),
        ],
    ),

    Domain(
        name="vault", title="CRYPTOGRAPHIC VAULT (Shamir t-of-n)",
        number="9", shortcut="", group="security",
        help_summary="Threshold key management with secret sharing",
        commands=[
            Command("1", "Create Vault", "submit",
                    pallet="Vault", function="create_vault",
                    params=[
                        Param("owner", "Owner", "actor"),
                        Param("threshold", "Threshold (t)", "int", 2),
                        Param("ring_size", "Ring size (n)", "int", 3),
                        Param("secret_hash", "Secret hash", "h256"),
                    ]),
            Command("2", "Add Member", "submit",
                    pallet="Vault", function="add_member",
                    params=[
                        Param("vault_id", "Vault ID", "int", 0),
                        Param("member", "Member", "actor"),
                        Param("role", "Role", "str", "Member"),
                    ]),
            Command("3", "Activate Vault", "submit",
                    pallet="Vault", function="activate_vault",
                    params=[Param("vault_id", "Vault ID", "int", 0)]),
            Command("4", "Commit Share", "submit",
                    pallet="Vault", function="commit_share",
                    params=[
                        Param("vault_id", "Vault ID", "int", 0),
                        Param("commitment", "Commitment", "h256"),
                    ]),
            Command("5", "Reveal Share", "submit",
                    pallet="Vault", function="reveal_share",
                    params=[Param("share_id", "Share ID", "int", 0)]),
            Command("6", "Initiate Recovery", "submit",
                    pallet="Vault", function="initiate_recovery",
                    params=[Param("vault_id", "Vault ID", "int", 0)]),
            Command("7", "Lock Vault", "submit",
                    pallet="Vault", function="lock_vault",
                    params=[Param("vault_id", "Vault ID", "int", 0)]),
            Command("8", "Dissolve Vault", "submit",
                    pallet="Vault", function="dissolve_vault",
                    params=[Param("vault_id", "Vault ID", "int", 0)]),
            Command("---", "Queries", "separator"),
            Command("a", "Vault Info", "query",
                    pallet="Vault", function="Vaults",
                    params=[Param("vault_id", "ID", "int", 0)]),
        ],
    ),

    # ------------------------------------------------------------------
    # IDENTITY
    # ------------------------------------------------------------------
    Domain(
        name="device", title="DEVICE MANAGEMENT",
        number="10", shortcut="dev", group="identity",
        help_summary="Register, activate, suspend, and monitor devices",
        commands=[
            Command("1", "Register Device", "submit",
                    pallet="Device", function="register_device",
                    params=[
                        Param("owner", "Owner", "actor"),
                        Param("device_type", "Type", "enum",
                              options=["Mobile", "Desktop", "Server",
                                       "IoT", "Hardware", "Virtual"]),
                        Param("public_key_hash", "Public key hash", "h256"),
                        Param("attestation_type", "Attestation type", "str",
                              "SelfSigned"),
                    ]),
            Command("2", "Activate / Reactivate Device", "custom",
                    custom_handler="_device_activate"),
            Command("3", "Suspend Device", "submit",
                    pallet="Device", function="suspend_device",
                    params=[
                        Param("device_id", "Device ID", "int", 0),
                        Param("reason", "Reason hash", "h256"),
                    ]),
            Command("4", "Revoke / Mark Compromised", "custom",
                    custom_handler="_device_revoke"),
            Command("5", "Submit Attestation", "custom",
                    custom_handler="_device_attestation"),
            Command("6", "Record Heartbeat", "submit",
                    pallet="Device", function="record_heartbeat",
                    params=[
                        Param("device_id", "Device ID", "int", 0),
                        Param("sequence", "Sequence", "int", 1),
                    ]),
            Command("7", "Update Trust Score", "submit",
                    pallet="Device", function="update_trust_score",
                    params=[
                        Param("device_id", "Device ID", "int", 0),
                        Param("new_score", "Score (0-100)", "int", 50),
                    ]),
            Command("---", "Queries", "separator"),
            Command("a", "Device Info", "query",
                    pallet="Device", function="Devices",
                    params=[Param("device_id", "ID", "int", 0)]),
        ],
    ),

    Domain(
        name="lifecycle", title="LIFECYCLE MANAGEMENT",
        number="11", shortcut="life", group="identity",
        help_summary="Register actors, manage key rotation and destruction",
        commands=[
            Command("1", "Register Actor", "submit",
                    pallet="Lifecycle", function="register_actor",
                    params=[Param("key_hash", "Key hash", "h256")]),
            Command("2", "Activate Actor [sudo]", "submit",
                    pallet="Lifecycle", function="activate_actor",
                    params=[Param("actor", "Actor", "actor")],
                    sudo=True),
            Command("3", "Suspend / Reactivate [sudo]", "custom",
                    custom_handler="_lifecycle_suspend_reactivate"),
            Command("4", "Initiate Destruction", "submit",
                    pallet="Lifecycle", function="initiate_destruction",
                    params=[
                        Param("reason", "Reason", "enum",
                              options=["OwnerRequest", "SecurityBreach",
                                       "Expiration", "ProtocolViolation",
                                       "Administrative"]),
                    ]),
            Command("5", "Attest Destruction", "submit",
                    pallet="Lifecycle", function="attest_destruction",
                    params=[
                        Param("target_actor", "Target", "actor"),
                        Param("signature_hash", "Signature hash", "h256"),
                    ]),
            Command("6", "Cancel Destruction", "submit",
                    pallet="Lifecycle", function="cancel_destruction"),
            Command("7", "Initiate Key Rotation", "submit",
                    pallet="Lifecycle", function="initiate_rotation",
                    params=[Param("new_key_hash", "New key hash", "h256")]),
            Command("8", "Complete Key Rotation", "submit",
                    pallet="Lifecycle", function="complete_rotation"),
            Command("---", "Queries", "separator"),
            Command("a", "Actor Info", "query",
                    pallet="Lifecycle", function="Actors",
                    params=[Param("actor", "Actor", "actor")]),
            Command("b", "Actor Count", "custom",
                    custom_handler="_lifecycle_count"),
        ],
    ),

    Domain(
        name="governance", title="GOVERNANCE & CAPABILITIES",
        number="12", shortcut="gov", group="identity",
        help_summary="Grant, revoke, and delegate access capabilities",
        commands=[
            Command("1", "Grant Capability", "custom",
                    custom_handler="_gov_grant"),
            Command("2", "Revoke Capability", "submit",
                    pallet="Governance", function="revoke_capability",
                    params=[Param("capability_id", "Capability ID", "int", 0)]),
            Command("3", "Delegate Capability", "custom",
                    custom_handler="_gov_delegate"),
            Command("4", "Update Permissions", "submit",
                    pallet="Governance", function="update_capability",
                    params=[
                        Param("capability_id", "Capability ID", "int", 0),
                        Param("new_permissions", "New permissions", "int", 7),
                    ]),
            Command("---", "Queries", "separator"),
            Command("a", "Capability Info", "query",
                    pallet="Governance", function="Capabilities",
                    params=[Param("capability_id", "ID", "int", 0)]),
        ],
    ),

    # ------------------------------------------------------------------
    # INTELLIGENCE
    # ------------------------------------------------------------------
    Domain(
        name="semantic", title="SEMANTIC RELATIONSHIPS",
        number="13", shortcut="sem", group="intelligence",
        help_summary="Create and manage trust relationships between actors",
        commands=[
            Command("1", "Create Relationship", "custom",
                    custom_handler="_semantic_create"),
            Command("2", "Accept Relationship", "submit",
                    pallet="Semantic", function="accept_relationship",
                    params=[Param("relationship_id", "Relationship ID", "int", 0)]),
            Command("3", "Revoke Relationship", "submit",
                    pallet="Semantic", function="revoke_relationship",
                    params=[Param("relationship_id", "Relationship ID", "int", 0)]),
            Command("4", "Update Trust Level", "submit",
                    pallet="Semantic", function="update_trust_level",
                    params=[
                        Param("relationship_id", "Relationship ID", "int", 0),
                        Param("new_trust_level", "New trust (0-100)", "int", 50),
                    ]),
            Command("5", "Request Discovery", "submit",
                    pallet="Semantic", function="request_discovery",
                    fixed_params={"criteria": {}}),
            Command("6", "Update Profile", "submit",
                    pallet="Semantic", function="update_profile",
                    params=[Param("discovery_enabled", "Discovery enabled?",
                                  "bool", True)]),
            Command("---", "Queries", "separator"),
            Command("a", "Relationship Info", "query",
                    pallet="Semantic", function="Relationships",
                    params=[Param("relationship_id", "ID", "int", 0)]),
        ],
    ),

    Domain(
        name="boomerang", title="BOOMERANG ROUTING",
        number="14", shortcut="boom", group="intelligence",
        help_summary="Round-trip path verification between actors",
        commands=[
            Command("1", "Initiate Path", "submit",
                    pallet="Boomerang", function="initiate_path",
                    params=[Param("target", "Target", "actor")]),
            Command("2", "Record Hop", "submit",
                    pallet="Boomerang", function="record_hop",
                    params=[
                        Param("path_id", "Path ID", "int", 0),
                        Param("to_actor", "To actor", "actor"),
                        Param("signature_hash", "Signature hash", "h256"),
                    ]),
            Command("3", "Extend Timeout", "submit",
                    pallet="Boomerang", function="extend_timeout",
                    params=[Param("path_id", "Path ID", "int", 0)]),
            Command("4", "Fail Path", "submit",
                    pallet="Boomerang", function="fail_path",
                    params=[
                        Param("path_id", "Path ID", "int", 0),
                        Param("reason", "Reason", "str", "Timeout"),
                    ]),
            Command("---", "Queries", "separator"),
            Command("a", "Path Info", "query",
                    pallet="Boomerang", function="Paths",
                    params=[Param("path_id", "ID", "int", 0)]),
            Command("b", "Active Paths", "query",
                    pallet="Boomerang", function="ActivePaths"),
        ],
    ),

    Domain(
        name="autonomous", title="AUTONOMOUS BEHAVIORS",
        number="15", shortcut="auto", group="intelligence",
        help_summary="Track behavior patterns and anomaly detection",
        commands=[
            Command("1", "Create Profile", "submit",
                    pallet="Autonomous", function="create_profile",
                    params=[Param("actor", "Actor", "actor")]),
            Command("2", "Record Behavior", "submit",
                    pallet="Autonomous", function="record_behavior",
                    params=[
                        Param("actor", "Actor", "actor"),
                        Param("behavior_type", "Behavior", "enum",
                              options=["PresencePattern", "InteractionPattern",
                                       "TemporalPattern", "TransactionPattern",
                                       "NetworkPattern"]),
                        Param("data_hash", "Data hash", "h256"),
                    ]),
            Command("3", "Register Pattern", "submit",
                    pallet="Autonomous", function="register_pattern",
                    params=[
                        Param("behavior_type", "Behavior", "enum",
                              options=["PresencePattern", "InteractionPattern",
                                       "TemporalPattern", "TransactionPattern",
                                       "NetworkPattern"]),
                        Param("signature_hash", "Signature hash", "h256"),
                        Param("classification", "Classification", "str", "Normal"),
                    ]),
            Command("4", "Match Behavior", "submit",
                    pallet="Autonomous", function="match_behavior",
                    params=[
                        Param("behavior_id", "Behavior ID", "int", 0),
                        Param("actor", "Actor", "actor"),
                        Param("pattern_id", "Pattern ID", "int", 0),
                    ]),
            Command("5", "Classify Pattern", "submit",
                    pallet="Autonomous", function="classify_pattern",
                    params=[
                        Param("pattern_id", "Pattern ID", "int", 0),
                        Param("classification", "Classification", "str", "Normal"),
                        Param("confidence_score", "Confidence (0-100)", "int", 80),
                    ]),
            Command("6", "Update Status", "submit",
                    pallet="Autonomous", function="update_status",
                    params=[
                        Param("actor", "Actor", "actor"),
                        Param("new_status", "Status", "str", "Active"),
                    ]),
            Command("7", "Flag Actor", "submit",
                    pallet="Autonomous", function="flag_actor",
                    params=[
                        Param("actor", "Actor", "actor"),
                        Param("reason", "Reason hash", "h256"),
                    ]),
            Command("---", "Queries", "separator"),
            Command("a", "Actor Profile", "query",
                    pallet="Autonomous", function="ActorProfiles",
                    params=[Param("actor", "Actor", "actor")]),
            Command("b", "Pattern Count", "query",
                    pallet="Autonomous", function="PatternCount"),
        ],
    ),

    Domain(
        name="octopus", title="OCTOPUS CLUSTERS",
        number="16", shortcut="oct", group="intelligence",
        help_summary="Multi-node orchestration with sub-node management",
        commands=[
            Command("1", "Create Cluster", "custom",
                    custom_handler="_octopus_create_cluster"),
            Command("2", "Register Subnode", "custom",
                    custom_handler="_octopus_register_subnode"),
            Command("3", "Activate Subnode", "custom",
                    custom_handler="_octopus_activate_subnode"),
            Command("4", "Start Deactivation", "custom",
                    custom_handler="_octopus_start_deactivation"),
            Command("5", "Update Cluster Throughput", "custom",
                    custom_handler="_octopus_update_throughput"),
            Command("6", "Evaluate Scaling", "custom",
                    custom_handler="_octopus_evaluate_scaling"),
            Command("7", "Update Subnode Throughput", "custom",
                    custom_handler="_octopus_update_subnode_throughput"),
            Command("8", "Record Heartbeat", "custom",
                    custom_handler="_octopus_record_heartbeat"),
            Command("9", "Record Device Observation", "custom",
                    custom_handler="_octopus_device_observation"),
            Command("10", "Record Position Confirmation", "custom",
                    custom_handler="_octopus_position_confirmation"),
            Command("11", "Heartbeat with Device Proof", "custom",
                    custom_handler="_octopus_heartbeat_device_proof"),
            Command("12", "Set Fusion Weights", "custom",
                    custom_handler="_octopus_set_fusion_weights"),
            Command("---", "Queries", "separator"),
            Command("a", "Cluster Info", "query",
                    pallet="Octopus", function="Clusters",
                    params=[Param("cluster_id", "ID", "int", 0)]),
            Command("b", "Subnode Info", "query",
                    pallet="Octopus", function="Subnodes",
                    params=[Param("subnode_id", "ID", "int", 0)]),
            Command("c", "Cluster Count", "query",
                    pallet="Octopus", function="ClusterCount"),
        ],
    ),

    Domain(
        name="storage", title="ON-CHAIN STORAGE",
        number="17", shortcut="store", group="intelligence",
        help_summary="Epoch-bound encrypted data storage",
        commands=[
            Command("1", "Store Data", "submit",
                    pallet="Storage", function="store_data",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("key", "Data key", "h256"),
                        Param("data_hash", "Data hash", "h256"),
                        Param("data_type", "Type", "enum",
                              options=["Presence", "Commitment", "Proof",
                                       "Metadata", "Temporary"]),
                        Param("size_bytes", "Size (bytes)", "int", 256),
                        Param("retention", "Retention", "str", "KeepForever"),
                    ]),
            Command("2", "Update Data", "submit",
                    pallet="Storage", function="update_data",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("key", "Data key", "h256"),
                        Param("new_data_hash", "New data hash", "h256"),
                        Param("new_size", "New size", "int", 256),
                    ]),
            Command("3", "Delete Data", "submit",
                    pallet="Storage", function="delete_data",
                    params=[
                        Param("epoch", "Epoch", "epoch"),
                        Param("key", "Data key", "h256"),
                    ]),
            Command("4", "Set Quota [sudo]", "submit",
                    pallet="Storage", function="set_quota",
                    params=[
                        Param("actor", "Actor", "actor"),
                        Param("max_entries", "Max entries", "int", 100),
                        Param("max_bytes", "Max bytes", "int", 1000000),
                    ],
                    sudo=True),
            Command("5", "Finalize Epoch Storage [sudo]", "submit",
                    pallet="Storage", function="finalize_epoch",
                    params=[Param("epoch", "Epoch", "epoch")],
                    sudo=True),
            Command("---", "Queries", "separator"),
            Command("a", "Entry Count", "query",
                    pallet="Storage", function="EntryCount"),
        ],
    ),

    # ------------------------------------------------------------------
    # STATUS
    # ------------------------------------------------------------------
    Domain(
        name="chain", title="CHAIN STATUS",
        number="18", shortcut="", group="status",
        help_summary="Node health, blocks, balances, and pallets",
        commands=[
            Command("1", "Node health & info", "custom",
                    custom_handler="_chain_health"),
            Command("2", "Latest block", "custom",
                    custom_handler="_chain_latest_block"),
            Command("3", "Runtime version", "custom",
                    custom_handler="_chain_runtime_version"),
            Command("4", "Account balances", "custom",
                    custom_handler="_chain_balances"),
            Command("5", "Recent events", "custom",
                    custom_handler="_chain_events"),
            Command("6", "List pallets", "custom",
                    custom_handler="_chain_pallets"),
        ],
    ),

    # ------------------------------------------------------------------
    # DEV TOOLS
    # ------------------------------------------------------------------
    Domain(
        name="blocks", title="BLOCK EXPLORER",
        number="19", shortcut="blk", group="devtools",
        help_summary="Inspect blocks, extrinsics, and finalization",
        commands=[
            Command("1", "Get block by number", "custom",
                    custom_handler="_blocks_by_number"),
            Command("2", "Get block by hash", "custom",
                    custom_handler="_blocks_by_hash"),
            Command("3", "Latest block detail", "custom",
                    custom_handler="_blocks_latest"),
            Command("4", "Decode extrinsic in block", "custom",
                    custom_handler="_blocks_decode_ext"),
            Command("5", "Block events", "custom",
                    custom_handler="_blocks_events"),
            Command("6", "Finalized head", "custom",
                    custom_handler="_blocks_finalized"),
            Command("7", "Compare blocks", "custom",
                    custom_handler="_blocks_compare"),
        ],
    ),

    Domain(
        name="inspect", title="STORAGE INSPECTOR",
        number="20", shortcut="si", group="devtools",
        help_summary="Query raw storage, enumerate keys, view proofs",
        commands=[
            Command("1", "Query storage by pallet + item", "custom",
                    custom_handler="_si_query_pallet"),
            Command("2", "Raw storage key lookup", "custom",
                    custom_handler="_si_raw_key"),
            Command("3", "Enumerate keys by prefix", "custom",
                    custom_handler="_si_enum_keys"),
            Command("4", "Storage size", "custom",
                    custom_handler="_si_storage_size"),
            Command("5", "Storage diff between blocks", "custom",
                    custom_handler="_si_diff"),
            Command("6", "Storage proof (Merkle)", "custom",
                    custom_handler="_si_proof"),
        ],
    ),

    Domain(
        name="runtime", title="RUNTIME INSPECTOR",
        number="21", shortcut="rt", group="devtools",
        help_summary="Explore pallets, calls, storage, events, errors",
        commands=[
            Command("1", "List all pallets", "custom",
                    custom_handler="_rt_list_pallets"),
            Command("2", "Pallet detail", "custom",
                    custom_handler="_rt_pallet_detail"),
            Command("3", "Runtime version", "custom",
                    custom_handler="_rt_version"),
            Command("4", "Search call by name", "custom",
                    custom_handler="_rt_search_call"),
            Command("5", "Search storage by name", "custom",
                    custom_handler="_rt_search_storage"),
            Command("6", "Search error by name", "custom",
                    custom_handler="_rt_search_error"),
        ],
    ),

    Domain(
        name="network", title="NETWORK & PEERS",
        number="22", shortcut="net", group="devtools",
        help_summary="View peers, sync status, and manage connections",
        commands=[
            Command("1", "Connected peers", "custom",
                    custom_handler="_net_peers"),
            Command("2", "Node identity", "custom",
                    custom_handler="_net_identity"),
            Command("3", "Sync state", "custom",
                    custom_handler="_net_sync"),
            Command("4", "Node health", "custom",
                    custom_handler="_net_health"),
            Command("5", "Node roles", "custom",
                    custom_handler="_net_roles"),
            Command("6", "Chain type", "custom",
                    custom_handler="_net_chain_type"),
            Command("7", "Pending extrinsics", "custom",
                    custom_handler="_net_pending"),
            Command("8", "Add/Remove reserved peer", "custom",
                    custom_handler="_net_reserved_peer"),
        ],
    ),

    Domain(
        name="crypto", title="CRYPTO TOOLBOX",
        number="23", shortcut="cr", group="devtools",
        help_summary="Keypairs, hashing, signing, SCALE encoding",
        commands=[
            Command("1", "Generate keypair", "custom",
                    custom_handler="_crypto_generate"),
            Command("2", "Derive from URI", "custom",
                    custom_handler="_crypto_derive"),
            Command("3", "SS58 encode/decode", "custom",
                    custom_handler="_crypto_ss58"),
            Command("4", "Blake2b-256 hash", "custom",
                    custom_handler="_crypto_blake2b"),
            Command("5", "Keccak-256 hash", "custom",
                    custom_handler="_crypto_keccak"),
            Command("6", "TwoX128 hash", "custom",
                    custom_handler="_crypto_twox128"),
            Command("7", "Build storage key", "custom",
                    custom_handler="_crypto_storage_key"),
            Command("8", "SCALE encode", "custom",
                    custom_handler="_crypto_scale_encode"),
            Command("9", "SCALE decode", "custom",
                    custom_handler="_crypto_scale_decode"),
            Command("10", "Sign message", "custom",
                    custom_handler="_crypto_sign"),
            Command("11", "Verify signature", "custom",
                    custom_handler="_crypto_verify"),
            Command("12", "Random H256", "custom",
                    custom_handler="_crypto_random"),
        ],
    ),

    Domain(
        name="accounts", title="ACCOUNT INSPECTOR",
        number="24", shortcut="acct", group="devtools",
        help_summary="Account info, balances, nonces, fee estimation",
        commands=[
            Command("1", "Full account info", "custom",
                    custom_handler="_acct_full_info"),
            Command("2", "Account nonce", "custom",
                    custom_handler="_acct_nonce"),
            Command("3", "All balances", "custom",
                    custom_handler="_acct_balances"),
            Command("4", "Fee estimation", "custom",
                    custom_handler="_acct_fee"),
            Command("5", "Dry run extrinsic", "custom",
                    custom_handler="_acct_dry_run"),
        ],
    ),

    Domain(
        name="events", title="EVENT DECODER",
        number="25", shortcut="ev", group="devtools",
        help_summary="Decode and filter blockchain events",
        commands=[
            Command("1", "Events at latest block", "custom",
                    custom_handler="_ev_latest"),
            Command("2", "Events at block N", "custom",
                    custom_handler="_ev_at_block"),
            Command("3", "Filter by pallet", "custom",
                    custom_handler="_ev_filter"),
            Command("4", "Event history (last N blocks)", "custom",
                    custom_handler="_ev_history"),
            Command("5", "List all event types", "custom",
                    custom_handler="_ev_types"),
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
