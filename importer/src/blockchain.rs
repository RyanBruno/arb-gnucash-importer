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
    #[serde(default)]
    pub etherscan_api_key: Option<String>,
}

impl Config {
    /// Load configuration from the `ARBITRUM_RPC_URL` environment variable or
    /// from the provided config file. The file format is inferred from the
    /// extension and may be TOML, YAML or JSON. If `path` is `None`,
    /// `config.toml` will be attempted.
    pub fn load(path: Option<&str>) -> Result<Self, Box<dyn Error>> {
        if let Ok(url) = env::var("ARBITRUM_RPC_URL") {
            return Ok(Self {
                rpc_url: url,
                etherscan_api_key: env::var("ETHERSCAN_API_KEY").ok(),
            });
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
pub fn etherscan_client(cfg: &Config) -> Result<EtherscanClient, Box<dyn Error>> {
    if let Some(ref key) = cfg.etherscan_api_key {
        Ok(EtherscanClient::new(Chain::Arbitrum, key)?)
    } else {
        Ok(EtherscanClient::new_from_opt_env(Chain::Arbitrum)?)
    }
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
    /// Optional category for the transaction
    pub category: Option<String>,
    /// Optional description for the transaction
    pub description: Option<String>,
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

/// Category information associated with an address
#[derive(Clone, Debug, Deserialize)]
pub struct CategoryEntry {
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Mapping from addresses to transaction categories and descriptions
#[derive(Debug, Deserialize)]
pub struct Categories(pub HashMap<Address, CategoryEntry>);

impl Categories {
    /// Load categories from the given file path. The format is inferred from the
    /// extension and may be TOML, JSON, or YAML.
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        // first try parsing the new structure with descriptions
        let map: Result<HashMap<Address, CategoryEntry>, Box<dyn Error>> =
            match path.extension().and_then(|e| e.to_str()) {
                Some("json") => Ok(serde_json::from_str(&contents)?),
                Some("toml") => Ok(toml::from_str(&contents)?),
                _ => Ok(serde_yaml::from_str(&contents)?),
            };

        let map = match map {
            Ok(m) => m,
            Err(_) => {
                // fall back to legacy format mapping address -> String
                let legacy: HashMap<Address, String> =
                    match path.extension().and_then(|e| e.to_str()) {
                        Some("json") => serde_json::from_str(&contents)?,
                        Some("toml") => toml::from_str(&contents)?,
                        _ => serde_yaml::from_str(&contents)?,
                    };
                legacy
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            CategoryEntry {
                                category: v,
                                description: None,
                            },
                        )
                    })
                    .collect()
            }
        };
        Ok(Self(map))
    }

    fn entry_for(&self, addr: &Address) -> Option<&CategoryEntry> {
        self.0.get(addr)
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

/// Assign categories to transactions by looking up the from and to addresses in the
/// provided [`Categories`] mapping.
pub fn apply_categories(txs: &mut [Transaction], categories: &Categories) {
    for tx in txs {
        if let Some(to) = tx.to {
            if let Some(entry) = categories.entry_for(&to) {
                tx.category = Some(entry.category.clone());
                tx.description = entry.description.clone();
                continue;
            }
        }
        if let Some(entry) = categories.entry_for(&tx.from) {
            tx.category = Some(entry.category.clone());
            tx.description = entry.description.clone();
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
            category: None,
            description: None,
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
            category: None,
            description: None,
            transfers: vec![transfer.clone()],
        };

        assert_eq!(tx.transfers.len(), 1);
        assert_eq!(tx.transfers[0].token_symbol, transfer.token_symbol);
    }

    #[test]
    fn apply_categories_assigns_values() {
        let mut txs = vec![Transaction {
            hash: H256::zero(),
            block_number: 0,
            timestamp: 0,
            from: Address::repeat_byte(0x11),
            to: Some(Address::repeat_byte(0x22)),
            value: U256::zero(),
            category: None,
            description: None,
            transfers: Vec::new(),
        }];

        let mut map = HashMap::new();
        map.insert(
            Address::repeat_byte(0x11),
            CategoryEntry {
                category: "Deposit".to_string(),
                description: None,
            },
        );
        map.insert(
            Address::repeat_byte(0x22),
            CategoryEntry {
                category: "Withdrawal".to_string(),
                description: Some("Foo".to_string()),
            },
        );
        let cats = Categories(map);

        apply_categories(&mut txs, &cats);

        assert_eq!(txs[0].category.as_deref(), Some("Withdrawal"));
        assert_eq!(txs[0].description.as_deref(), Some("Foo"));
    }

    #[tokio::test]
    async fn fetch_transactions_uses_client() {
        let client = EtherscanClient::builder()
            .chain(Chain::Arbitrum)
            .unwrap()
            .with_api_url("http://127.0.0.1:0/api")
            .unwrap()
            .with_url("http://127.0.0.1:0")
            .unwrap()
            .build()
            .unwrap();

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
