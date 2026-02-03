#![warn(missing_docs)]

//! 7aychain Node - Substrate-based Proof of Presence Protocol

mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
    command::run()
}
