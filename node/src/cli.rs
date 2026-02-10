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
    pub run: RunCmd,
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
