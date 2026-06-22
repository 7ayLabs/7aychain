use sc_cli::RunCmd;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(
    author,
    version,
    about = "7aychain Node - Substrate-based Proof of Presence Protocol",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[clap(flatten)]
    pub run: ExtendedRunCmd,
}

#[derive(Debug, clap::Parser)]
pub struct ExtendedRunCmd {
    #[clap(flatten)]
    pub base: RunCmd,

    #[arg(long, default_value = "latency")]
    pub scanner_mode: String,

    #[arg(long, default_value = "15")]
    pub mock_devices: u32,

    #[arg(long, default_value = "0")]
    pub scanner_pos_x: i64,

    #[arg(long, default_value = "0")]
    pub scanner_pos_y: i64,

    #[arg(long, default_value = "0")]
    pub scanner_pos_z: i64,

    #[arg(long, default_value = "6")]
    pub scan_interval: u64,

    #[arg(long, default_value = "42")]
    pub mock_seed: u64,

    #[arg(long)]
    pub external_scan_file: Option<PathBuf>,

    #[arg(long, default_value = "30")]
    pub max_scan_age: u64,

    /// Block sealing mode: "aura" (continuous every 7s) or "instant" (only on extrinsic)
    #[arg(long, default_value = "aura")]
    pub sealing: String,

    // --- Carrier signal (beta) ---------------------------------------
    /// Carrier-signal source mode: "disabled", "file", or "mock".
    /// When enabled, the node periodically samples a local carrier
    /// signal and exposes it via the seveny_currentCarrierSignal RPC
    /// and an atomic status file.
    #[arg(long, default_value = "disabled")]
    pub carrier_signal_mode: String,

    /// JSON file the node polls when carrier-signal-mode=file.
    /// Schema: see node/src/carrier_signal/file.rs.
    #[arg(long)]
    pub carrier_signal_file: Option<PathBuf>,

    /// Status file the node writes after each sample (default:
    /// data/carrier-signal/status.json). Set to "" to disable.
    #[arg(long, default_value = "data/carrier-signal/status.json")]
    pub carrier_signal_status_file: String,

    /// Seconds between samples / file polls.
    #[arg(long, default_value = "10")]
    pub carrier_signal_interval: u64,

    /// Carrier label this node reports under (used by mock mode).
    #[arg(long, default_value = "7AY-Test")]
    pub carrier_id: String,

    /// Region commitment (32-byte hex) this node serves.
    /// Default = 0x1111...1111 to match devnet north-lab region.
    #[arg(
        long,
        default_value = "0x1111111111111111111111111111111111111111111111111111111111111111"
    )]
    pub carrier_region: String,

    /// Network technology label (e.g. "LTE", "5G", "7AY-MESH"). Used by mock mode.
    #[arg(long, default_value = "LTE")]
    pub carrier_network_type: String,

    /// Seed for the mock carrier signal generator.
    #[arg(long, default_value = "42")]
    pub carrier_mock_seed: u64,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    #[command(subcommand)]
    Key(sc_cli::KeySubcommand),

    BuildSpec(sc_cli::BuildSpecCmd),

    CheckBlock(sc_cli::CheckBlockCmd),

    ExportBlocks(sc_cli::ExportBlocksCmd),

    ExportState(sc_cli::ExportStateCmd),

    ImportBlocks(sc_cli::ImportBlocksCmd),

    PurgeChain(sc_cli::PurgeChainCmd),

    Revert(sc_cli::RevertCmd),

    ChainInfo(sc_cli::ChainInfoCmd),
}
