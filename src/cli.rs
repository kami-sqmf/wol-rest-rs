use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "wol",
    version,
    about = "Wake-on-LAN CLI with HTTPS control plane"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create or update the server settings in the config file.
    Init(InitArgs),
    /// Manage saved devices.
    #[command(subcommand)]
    Device(DeviceCommand),
    /// Send a WOL packet to a saved device.
    Wake(WakeArgs),
    /// Start the HTTPS API server.
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Address the HTTPS server should bind to.
    #[arg(long, default_value = "0.0.0.0:8443")]
    pub bind: String,
    /// Path to a PEM encoded certificate.
    #[arg(long)]
    pub cert: PathBuf,
    /// Path to a PEM encoded private key.
    #[arg(long)]
    pub key: PathBuf,
    /// Bearer token required by the API.
    #[arg(long, env = "WOL_BEARER_TOKEN")]
    pub token: String,
}

#[derive(Debug, Subcommand)]
pub enum DeviceCommand {
    /// Add or update a device entry.
    Add(DeviceAddArgs),
    /// List saved devices.
    List,
    /// Remove a saved device.
    Remove(DeviceRemoveArgs),
}

#[derive(Debug, Args)]
pub struct DeviceAddArgs {
    pub name: String,
    /// Target MAC address like AA:BB:CC:DD:EE:FF.
    #[arg(long)]
    pub mac: String,
    /// Broadcast host used for the packet.
    #[arg(long, default_value = "255.255.255.255")]
    pub host: String,
    /// UDP port used for the packet.
    #[arg(long, default_value_t = 9)]
    pub port: u16,
}

#[derive(Debug, Args)]
pub struct DeviceRemoveArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct WakeArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Override the bind address saved in config.
    #[arg(long)]
    pub bind: Option<String>,
    /// Override the certificate path saved in config.
    #[arg(long)]
    pub cert: Option<PathBuf>,
    /// Override the key path saved in config.
    #[arg(long)]
    pub key: Option<PathBuf>,
    /// Override the bearer token saved in config.
    #[arg(long, env = "WOL_BEARER_TOKEN")]
    pub token: Option<String>,
}
