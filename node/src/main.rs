#![warn(missing_docs)]
// Allow Substrate-specific patterns that trigger these lints
#![allow(clippy::result_large_err)]
#![allow(clippy::type_complexity)]

//! 7aychain Node - Substrate-based Proof of Presence Protocol

mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
    command::run()
}
