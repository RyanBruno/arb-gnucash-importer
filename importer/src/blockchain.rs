use std::env;
use std::error::Error;
use std::fs;

use ethers::{
    etherscan::Client as EtherscanClient,
    providers::{Http, Provider},
    types::{Address, Chain, H256, U256},
};
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

/// Simplified transaction information returned by [`fetch_transactions`].
#[derive(Clone, Debug)]
pub struct Transaction {
    pub hash: H256,
    pub block_number: u64,
    pub timestamp: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
}

/// Retrieve all normal transactions for the given address using the Etherscan API.
pub async fn fetch_transactions(address: Address) -> Result<Vec<Transaction>, Box<dyn Error>> {
    // use optional API key from environment if provided
    let client = EtherscanClient::new_from_opt_env(Chain::Arbitrum)?;
    let txs = client.get_transactions(&address, None).await?;
    let mut result = Vec::new();

    for tx in txs {
        let hash = match tx.hash.value().copied() {
            Some(hash) => hash,
            None => continue, // skip malformed entries
        };
        let from = match tx.from.value().copied() {
            Some(addr) => addr,
            None => continue,
        };

        let block_number = tx
            .block_number
            .as_number()
            .map(|n| n.as_u64())
            .unwrap_or_default();
        let timestamp = tx.time_stamp.parse::<u64>().unwrap_or_default();

        result.push(Transaction {
            hash,
            block_number,
            timestamp,
            from,
            to: tx.to,
            value: tx.value,
        });
    }

    Ok(result)
}
