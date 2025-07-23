use std::env;
use std::error::Error;
use std::fs;

use ethers::{
    etherscan::{
        account::{ERC20TokenTransferEvent, TokenQueryOption},
        Client as EtherscanClient,
    },
    providers::{Http, Provider},
    types::{Address, Chain, H256, U256},
};
use serde::Deserialize;
use std::collections::HashMap;

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
#[derive(Clone, Debug, serde::Serialize)]
pub struct Transaction {
    pub hash: H256,
    pub block_number: u64,
    pub timestamp: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    /// ERC-20 token transfers associated with this transaction
    pub transfers: Vec<Erc20Transfer>,
}

/// Details for a single ERC-20 token transfer
#[derive(Clone, Debug, serde::Serialize)]
pub struct Erc20Transfer {
    pub token_contract: Address,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimal: String,
}

fn group_transfers(events: Vec<ERC20TokenTransferEvent>) -> HashMap<H256, Vec<Erc20Transfer>> {
    let mut map: HashMap<H256, Vec<Erc20Transfer>> = HashMap::new();
    for ev in events {
        let transfer = Erc20Transfer {
            token_contract: ev.contract_address,
            from: ev.from,
            to: ev.to,
            value: ev.value,
            token_name: ev.token_name,
            token_symbol: ev.token_symbol,
            token_decimal: ev.token_decimal,
        };
        map.entry(ev.hash).or_default().push(transfer);
    }
    map
}

/// Retrieve all normal transactions for the given address using the Etherscan API.
pub async fn fetch_transactions(address: Address) -> Result<Vec<Transaction>, Box<dyn Error>> {
    // use optional API key from environment if provided
    let client = EtherscanClient::new_from_opt_env(Chain::Arbitrum)?;
    let txs = client.get_transactions(&address, None).await?;
    let events = client
        .get_erc20_token_transfer_events(TokenQueryOption::ByAddress(address), None)
        .await?;
    let mut transfers = group_transfers(events);
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
            transfers: transfers.remove(&hash).unwrap_or_default(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_with_transfer() {
        let transfer = Erc20Transfer {
            token_contract: Address::zero(),
            from: Address::zero(),
            to: Some(Address::zero()),
            value: U256::from(1u64),
            token_name: "TEST".to_string(),
            token_symbol: "TST".to_string(),
            token_decimal: "18".to_string(),
        };

        let tx = Transaction {
            hash: H256::zero(),
            block_number: 1,
            timestamp: 0,
            from: Address::zero(),
            to: None,
            value: U256::zero(),
            transfers: vec![transfer.clone()],
        };

        assert_eq!(tx.transfers.len(), 1);
        assert_eq!(tx.transfers[0].token_symbol, transfer.token_symbol);
    }
}
