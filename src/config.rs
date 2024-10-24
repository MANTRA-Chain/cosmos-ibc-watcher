//! Chain configuration
use std::collections::HashMap;
use std::{fs, fs::File, io::Write, path::Path, time::Duration};

use serde_derive::{Deserialize, Serialize};

use crate::error::Error;

pub mod default {
    use super::*;

    pub fn refresh() -> Duration {
        Duration::from_secs(120)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub prometheus: PrometheusConfig,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub chains: Vec<ChainConfig>,
}

impl Config {
    pub fn chains_map(&self) -> HashMap<&String, &ChainConfig> {
        self.chains.iter().map(|c| (&c.id, c)).collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusConfig {
    pub host: String,
    pub port: i32,
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    pub reset: Option<Duration>,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 9090,
            reset: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainConfig {
    pub id: String,
    pub grpc_addr: tendermint_rpc::Url,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Channel>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub port_id: String,
    pub channel_id: String,
    pub destination_chain_id: String,
    pub min_total: String,
    #[serde(default = "default::refresh", with = "humantime_serde")]
    pub refresh: Duration,
}

/// Attempt to load and parse the TOML config file as a `Config`.
pub fn load(path: impl AsRef<Path>) -> Result<Config, Error> {
    let config_toml = fs::read_to_string(&path).map_err(Error::config_io)?;

    let config = toml::from_str::<Config>(&config_toml[..]).map_err(Error::config_decode)?;
    check_parse_u64(config.clone())?;
    Ok(config)
}

// Attempt to parse min_total to u64
pub fn check_parse_u64(config: Config) -> Result<(), Error> {
    for chain_config in config.chains.iter() {
        for chain_channel in chain_config.channels.iter() {
            chain_channel
                .min_total
                .parse::<u64>()
                .map_err(Error::config_parse_u64)?;
        }
    }
    Ok(())
}

/// Serialize the given `Config` as TOML to the given config file.
pub fn store(config: &Config, path: impl AsRef<Path>) -> Result<(), Error> {
    let mut file = if path.as_ref().exists() {
        fs::OpenOptions::new().write(true).truncate(true).open(path)
    } else {
        File::create(path)
    }
    .map_err(Error::config_io)?;

    store_writer(config, &mut file)
}

/// Serialize the given `Config` as TOML to the given writer.
pub(crate) fn store_writer(config: &Config, mut writer: impl Write) -> Result<(), Error> {
    let toml_config = toml::to_string_pretty(&config).map_err(Error::config_encode)?;

    writeln!(writer, "{toml_config}").map_err(Error::config_io)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{load, store_writer};
    use test_log::test;

    #[test]
    fn parse_valid_config() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/config/fixtures/chains.toml"
        );

        let config = load(path);
        println!("{:?}", config);
        assert!(config.is_ok());
    }

    #[test]
    fn parse_invalid_config() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/config/fixtures/chains-fail.toml"
        );

        let config = load(path);
        println!("{:?}", config);
        assert!(config.is_err());
    }

    #[test]
    fn serialize_valid_config() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/config/fixtures/chains.toml"
        );

        let config = load(path).expect("could not parse config");

        let mut buffer = Vec::new();
        store_writer(&config, &mut buffer).unwrap();
    }
}
