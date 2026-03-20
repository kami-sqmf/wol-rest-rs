mod cli;
mod config;
mod server;
mod wol;

use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser};

use crate::{
    cli::{Cli, Command, DeviceCommand},
    config::{Config, DeviceConfig, config_path},
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli).await
}

async fn run(cli: Cli) -> Result<()> {
    let path = config_path()?;
    let mut config = Config::load_or_default(&path)?;

    match cli.command {
        Command::Init(args) => {
            config.server = server::init_server_config(args.bind, args.cert, args.key, args.token);
            config.save(&path)?;
            println!("saved server configuration to {}", path.display());
        }
        Command::Device(command) => match command {
            DeviceCommand::Add(args) => {
                validate_name(&args.name)?;
                let device = DeviceConfig {
                    mac: args.mac,
                    host: args.host,
                    port: args.port,
                };
                config.devices.insert(args.name.clone(), device);
                config.save(&path)?;
                println!("saved device `{}` to {}", args.name, path.display());
            }
            DeviceCommand::List => {
                if config.devices.is_empty() {
                    println!("no devices configured");
                } else {
                    for (name, device) in &config.devices {
                        println!("{}", server::device_summary(name, device));
                    }
                }
            }
            DeviceCommand::Remove(args) => {
                if config.devices.remove(&args.name).is_some() {
                    config.save(&path)?;
                    println!("removed device `{}`", args.name);
                } else {
                    return Err(anyhow!("device `{}` not found", args.name));
                }
            }
        },
        Command::Wake(args) => {
            let device = config
                .devices
                .get(&args.name)
                .cloned()
                .ok_or_else(|| anyhow!("device `{}` not found", args.name))?;
            wol::wake_device(&device).await?;
            println!("sent WOL packet to `{}`", args.name);
        }
        Command::Serve(args) => {
            let override_config =
                server::build_override_server_config(args.bind, args.cert, args.key, args.token);
            server::run(config, override_config).await?;
        }
    }

    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("device name cannot be empty"));
    }
    if name.contains('/') {
        return Err(anyhow!("device name cannot contain `/`"));
    }
    Ok(())
}

#[allow(dead_code)]
fn _clap_debug() {
    let _ = Cli::command().debug_assert();
}
