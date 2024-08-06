/*!
This module provides the configuration setup and command-line argument parsing for a server application. It utilizes the `clap` crate for parsing command-line arguments and the `config` crate for managing configuration through a combination of command-line arguments, environment variables, and default values.

# Dependencies
- `clap::{Parser, ValueEnum}`
- `serde::Deserialize`
- `std::net::Ipv4Addr`
- `config::{Config, Environment, ConfigError}`

# Components
1. `LogMethod`: An enum representing the different logging methods.
2. `ServerConfig`: A struct representing the server configuration.
3. `Args`: A struct representing the command-line arguments.
4. `parse_args`: A function to parse command-line arguments and merge them with other configuration sources.
*/
use clap::{Parser, ValueEnum};
use config::{Config, ConfigError, Environment};
use serde::Deserialize;
use core::fmt;
use std::net::Ipv4Addr;

/// Defines the logging methods available for the server.
#[derive(ValueEnum, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogMethod {
    /// Log to a file
    File,
    /// Log to a Loki server
    Loki,
    /// Log to stdout
    Stdout,
}

impl fmt::Display for LogMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogMethod::File => write!(f, "file"),
            LogMethod::Loki => write!(f, "loki"),
            LogMethod::Stdout => write!(f, "stdout"),
        }
    }
}

// impl ToString for LogMethod {
//     fn to_string(&self) -> String {
//         match self {
//             LogMethod::File => "file".to_string(),
//             LogMethod::Loki => "loki".to_string(),
//             LogMethod::Stdout => "stdout".to_string(),
//         }
//     }
// }

/// Configuration structure for the server.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// Port number for the API server
    pub api_port: u16,
    /// Network interface IP address for the API server
    pub api_network: Ipv4Addr,
    /// Enable or disable metrics collection
    pub metrics: bool,
    /// Port number for the metrics server (not yet implemented)
    pub metrics_port: u16,
    /// Network interface IP address for the metrics server (not yet implemented)
    pub metrics_network: Ipv4Addr,
    /// Logging method to use (Not yet implemented)
    pub subscriber: LogMethod,
    /// Log level verbosity
    pub verbose: u8,
}
/// Command-line arguments structure
#[derive(Parser, Debug, Deserialize)]
#[command(version, about, long_about= None)]
struct Args {
    /// Port number for the API server
    ///
    /// Must be between 1 and 65535
    #[arg(short = 'p', long, value_parser=clap::value_parser!(u16).range(1..),)]
    pub api_port: Option<u16>,

    /// Network interface IP address for the API server
    ///
    /// Default is 0.0.0.0 (all interfaces)
    #[arg(short = 'n', long)]
    pub api_network: Option<Ipv4Addr>,

    /// Enable or disable metrics collection
    #[arg(short, long)]
    pub metrics: Option<bool>,

    /// Port number for the metrics server
    ///
    /// Must be between 1 and 65535
    #[arg(long, value_parser=clap::value_parser!(u16).range(1..),)]
    pub metrics_port: Option<u16>,

    /// Network interface IP address for the metrics server
    ///
    /// Default is 0.0.0.0 (all interfaces)
    #[arg(long)]
    pub metrics_network: Option<Ipv4Addr>,

    /// Logging method to use
    #[arg(short, long, value_enum)]
    pub subscriber: Option<LogMethod>,

    ///log level
    ///
    /// Default is INFO (3), equivalent to "-verbose -verbose -verbose" or "-vvv"
    /// Values are:
    ///  - ERROR (1): -v
    ///  - WARN  (2): -vv
    ///  - INFO  (3): -vvv
    ///  - DEBUG (4): -vvvv
    ///  - Trace (5): -vvvvv
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

/// Parses command-line arguments and merges them with configuration from environment variables and defaults.
pub fn parse_args() -> Result<ServerConfig, ConfigError> {
    let cli_args = Args::parse();

    let mut config_builder = Config::builder()
        .set_default("api_network", Ipv4Addr::new(0, 0, 0, 0).to_string())?
        .set_default("api_port", 3200)?
        .set_default("metrics", false)?
        .set_default("metrics_network", Ipv4Addr::new(127, 0, 0, 1).to_string())?
        .set_default("metrics_port", 3201)?
        .set_default("subscriber", LogMethod::Stdout.to_string())?
        .set_default("verbose", 3)?
        .add_source(Environment::with_prefix("TIMETALLY"))
        .set_override_option("api_network", cli_args.api_network.map(|v| v.to_string()))?
        .set_override_option("api_port", cli_args.api_port.map(|v| v.to_string()))?
        .set_override_option("metrics", cli_args.metrics.map(|v| v.to_string()))?
        .set_override_option(
            "metrics_network",
            cli_args.metrics_network.map(|v| v.to_string()),
        )?
        .set_override_option("metrics_port", cli_args.metrics_port.map(|v| v.to_string()))?
        .set_override_option("subscriber", cli_args.subscriber.map(|v| v.to_string()))?;

    if cli_args.verbose > 0 {
        config_builder = config_builder.set_override("verbose", cli_args.verbose.to_string())?;
    }
    let config = config_builder.build()?.try_deserialize::<ServerConfig>()?;
    Ok(config)
}
