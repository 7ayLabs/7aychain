use crate::{
    carrier_signal::{CarrierSignalConfig, CarrierSignalMode},
    chain_spec,
    cli::{Cli, Subcommand},
    scanner::{Position, ScannerConfig, ScannerMode},
    service::{self, SealingMode},
};
use sp_core::H256;
use clap::Parser;
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;
use seveny_runtime::Block;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "7aychain Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/7ayLabs/7aychain/issues".into()
    }

    fn copyright_start_year() -> i32 {
        2026
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()?),
            "local" | "" => Box::new(chain_spec::local_testnet_config()?),
            "mainnet" => Box::new(chain_spec::mainnet_config()?),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}

pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = service::new_partial(&config)?;
                let aux_revert = Box::new(|client, _, blocks| {
                    sc_consensus_grandpa::revert(client, blocks)?;
                    Ok(())
                });
                Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
            })
        }
        Some(Subcommand::ChainInfo(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run::<Block>(&config))
        }
        None => {
            fn parse_region_h256(input: &str) -> Result<H256, String> {
                let trimmed = input.strip_prefix("0x").unwrap_or(input);
                if trimmed.len() != 64 {
                    return Err(format!("expected 32 bytes hex, got {} chars", trimmed.len()));
                }
                let mut bytes = [0u8; 32];
                hex::decode_to_slice(trimmed, &mut bytes)
                    .map_err(|e| format!("invalid hex: {e}"))?;
                Ok(H256(bytes))
            }

            let scanner_mode = cli
                .run
                .scanner_mode
                .parse::<ScannerMode>()
                .unwrap_or_default();
            let scanner_config = ScannerConfig {
                mode: scanner_mode,
                mock_device_count: cli.run.mock_devices,
                mock_seed: cli.run.mock_seed,
                scan_interval_secs: cli.run.scan_interval,
                external_scan_file: cli.run.external_scan_file.clone(),
                max_scan_age_secs: cli.run.max_scan_age,
                reporter_position: Position {
                    x: cli.run.scanner_pos_x,
                    y: cli.run.scanner_pos_y,
                    z: cli.run.scanner_pos_z,
                },
                ..ScannerConfig::default()
            };

            let sealing = cli.run.sealing.parse::<SealingMode>().unwrap_or_default();

            let carrier_signal_mode = cli
                .run
                .carrier_signal_mode
                .parse::<CarrierSignalMode>()
                .unwrap_or_default();
            let carrier_region_id = parse_region_h256(&cli.run.carrier_region)
                .unwrap_or_else(|err| {
                    log::warn!("--carrier-region invalid ({err}); using default 0x1111...");
                    H256::repeat_byte(0x11)
                });
            let status_file = if cli.run.carrier_signal_status_file.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(
                    cli.run.carrier_signal_status_file.clone(),
                ))
            };
            let carrier_signal_config = CarrierSignalConfig {
                mode: carrier_signal_mode,
                interval_secs: cli.run.carrier_signal_interval,
                input_file: cli.run.carrier_signal_file.clone(),
                status_file,
                carrier_id: cli.run.carrier_id.clone(),
                region_id: carrier_region_id,
                network_type: cli.run.carrier_network_type.clone(),
                mock_seed: cli.run.carrier_mock_seed,
            };

            let runner = cli.create_runner(&cli.run.base)?;
            runner.run_node_until_exit(|config| async move {
                service::new_full(
                    config,
                    Some(scanner_config),
                    sealing,
                    Some(carrier_signal_config),
                )
                .map_err(sc_cli::Error::Service)
            })
        }
    }
}
