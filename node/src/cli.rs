use sc_cli::RunCmd;

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

    #[arg(long, default_value = "real")]
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
