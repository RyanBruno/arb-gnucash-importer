use std::env;
use std::fs;
use std::error::Error;

use ethers::providers::{Provider, Http};
use serde::Deserialize;

/// Configuration for connecting to the Arbitrum network.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub rpc_url: String,
}

impl Config {
    /// Load configuration from the `ARBITRUM_RPC_URL` environment variable or
    /// from the given YAML file path. If `path` is `None`, `config.yml` will be
    /// attempted.
    pub fn load(path: Option<&str>) -> Result<Self, Box<dyn Error>> {
        if let Ok(url) = env::var("ARBITRUM_RPC_URL") {
            return Ok(Self { rpc_url: url });
        }

        let path = path.unwrap_or("config.yml");
        let contents = fs::read_to_string(path)?;
        let cfg: Self = serde_yaml::from_str(&contents)?;
        Ok(cfg)
    }
}

/// Create an ethers HTTP provider using the supplied configuration.
pub async fn provider(cfg: &Config) -> Result<Provider<Http>, Box<dyn Error>> {
    let provider = Provider::<Http>::try_from(cfg.rpc_url.as_str())?;
    Ok(provider)
}
