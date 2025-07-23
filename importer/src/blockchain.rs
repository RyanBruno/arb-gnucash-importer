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
use std::path::Path;

/// Configuration for connecting to the Arbitrum network.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub rpc_url: String,
}

impl Config {
    /// Load configuration from the `ARBITRUM_RPC_URL` environment variable or
    /// from the provided config file. The file format is inferred from the
    /// extension and may be TOML, YAML or JSON. If `path` is `None`,
    /// `config.toml` will be attempted.
    pub fn load(path: Option<&str>) -> Result<Self, Box<dyn Error>> {
        if let Ok(url) = env::var("ARBITRUM_RPC_URL") {
            return Ok(Self { rpc_url: url });
        }

        let path = path.unwrap_or("config.yml");
        let contents = fs::read_to_string(path)?;
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yml");
        let cfg = match ext {
            "json" => serde_json::from_str(&contents)?,
            "yaml" | "yml" => serde_yaml::from_str(&contents)?,
            _ => toml::from_str(&contents)?,
        };
        Ok(cfg)
    }
}

/// Create an ethers HTTP provider using the supplied configuration.
pub async fn provider(cfg: &Config) -> Result<Provider<Http>, Box<dyn Error>> {
    let provider = Provider::<Http>::try_from(cfg.rpc_url.as_str())?;
    Ok(provider)
}

/// Create an [`EtherscanClient`] for the Arbitrum network using an optional API key.
pub fn etherscan_client(_cfg: &Config) -> Result<EtherscanClient, Box<dyn Error>> {
    Ok(EtherscanClient::new_from_opt_env(Chain::Arbitrum)?)
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
    /// Optional tag for the from address
    pub from_tag: Option<String>,
    /// Optional tag for the to address
    pub to_tag: Option<String>,
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

/// Mapping from addresses to service names for tagging transactions
#[derive(Debug, Deserialize)]
pub struct Tags(pub HashMap<Address, String>);

impl Tags {
    /// Load tags from the given file path. The format is inferred from the
    /// extension and may be TOML, JSON, or YAML.
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let map: HashMap<Address, String> = match path.extension().and_then(|e| e.to_str()) {
            Some("json") => serde_json::from_str(&contents)?,
            Some("toml") => toml::from_str(&contents)?,
            _ => serde_yaml::from_str(&contents)?,
        };
        Ok(Self(map))
    }

    fn tag_for(&self, addr: &Address) -> Option<String> {
        self.0.get(addr).cloned()
    }
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

/// Apply tags to transactions by looking up the from and to addresses in the
/// provided [`Tags`] mapping.
pub fn apply_tags(txs: &mut [Transaction], tags: &Tags) {
    for tx in txs {
        tx.from_tag = tags.tag_for(&tx.from);
        if let Some(to) = tx.to {
            tx.to_tag = tags.tag_for(&to);
        }
    }
}

/// Retrieve all normal transactions for the given address using the provided [`EtherscanClient`].
pub async fn fetch_transactions(
    client: &EtherscanClient,
    address: Address,
) -> Result<Vec<Transaction>, Box<dyn Error>> {
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
            from_tag: None,
            to_tag: None,
            transfers: transfers.remove(&hash).unwrap_or_default(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ethers::etherscan::Client as EtherscanClient;

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
            from_tag: None,
            to_tag: None,
            transfers: vec![transfer.clone()],
        };

        assert_eq!(tx.transfers.len(), 1);
        assert_eq!(tx.transfers[0].token_symbol, transfer.token_symbol);
    }

    #[test]
    fn apply_tags_assigns_values() {
        let mut txs = vec![Transaction {
            hash: H256::zero(),
            block_number: 0,
            timestamp: 0,
            from: Address::repeat_byte(0x11),
            to: Some(Address::repeat_byte(0x22)),
            value: U256::zero(),
            from_tag: None,
            to_tag: None,
            transfers: Vec::new(),
        }];

        let mut map = HashMap::new();
        map.insert(Address::repeat_byte(0x11), "alice".to_string());
        map.insert(Address::repeat_byte(0x22), "bob".to_string());
        let tags = Tags(map);

        apply_tags(&mut txs, &tags);

        assert_eq!(txs[0].from_tag.as_deref(), Some("alice"));
        assert_eq!(txs[0].to_tag.as_deref(), Some("bob"));
    }

    #[tokio::test]
    async fn fetch_transactions_uses_client() {
        let client = EtherscanClient::builder()
            .chain(Chain::Arbitrum).unwrap()
            .with_api_url("http://127.0.0.1:0/api").unwrap()
            .with_url("http://127.0.0.1:0").unwrap()
            .build().unwrap();

        let res = fetch_transactions(&client, Address::zero()).await;
        assert!(res.is_err());
    }
    #[test]
    fn config_loads_toml_and_yaml() {
        let toml_cfg = Config::load(Some("../examples/config.sample.toml")).expect("load toml");
        let yaml_cfg = Config::load(Some("../examples/config.sample.yml")).expect("load yaml");
        assert_eq!(toml_cfg.rpc_url, yaml_cfg.rpc_url);
    }
}
