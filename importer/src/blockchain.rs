use std::env;
use std::error::Error;
use std::fs;

use async_trait::async_trait;
use ethers::{
    etherscan::{
        account::{ERC20TokenTransferEvent, NormalTransaction, TokenQueryOption, TxListParams},
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

/// Trait abstracting the subset of [`EtherscanClient`] functionality used by
/// [`fetch_transactions`].
#[async_trait]
pub trait TxSource {
    async fn get_transactions(
        &self,
        address: &Address,
        params: Option<TxListParams>,
    ) -> Result<Vec<NormalTransaction>, Box<dyn Error>>;

    async fn get_erc20_token_transfer_events(
        &self,
        option: TokenQueryOption,
        params: Option<TxListParams>,
    ) -> Result<Vec<ERC20TokenTransferEvent>, Box<dyn Error>>;
}

#[async_trait]
impl TxSource for EtherscanClient {
    async fn get_transactions(
        &self,
        address: &Address,
        params: Option<TxListParams>,
    ) -> Result<Vec<NormalTransaction>, Box<dyn Error>> {
        Ok(EtherscanClient::get_transactions(self, address, params).await?)
    }

    async fn get_erc20_token_transfer_events(
        &self,
        option: TokenQueryOption,
        params: Option<TxListParams>,
    ) -> Result<Vec<ERC20TokenTransferEvent>, Box<dyn Error>> {
        Ok(EtherscanClient::get_erc20_token_transfer_events(self, option, params).await?)
    }
}

/// Retrieve all normal transactions for the given address using the provided [`EtherscanClient`].
pub async fn fetch_transactions<C>(
    client: &C,
    address: Address,
) -> Result<Vec<Transaction>, Box<dyn Error>>
where
    C: TxSource + Sync,
{
    let mut page = 1u64;
    let mut txs = Vec::new();
    loop {
        let params = TxListParams {
            page,
            offset: 100,
            ..Default::default()
        };
        let mut batch = client.get_transactions(&address, Some(params)).await?;
        if batch.is_empty() {
            break;
        }
        txs.append(&mut batch);
        page += 1;
    }

    page = 1;
    let mut events_all = Vec::new();
    loop {
        let params = TxListParams {
            page,
            offset: 100,
            ..Default::default()
        };
        let mut ev = client
            .get_erc20_token_transfer_events(TokenQueryOption::ByAddress(address), Some(params))
            .await?;
        if ev.is_empty() {
            break;
        }
        events_all.append(&mut ev);
        page += 1;
    }
    let mut transfers = group_transfers(events_all);
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

    use ethers::etherscan::account::GenesisOption;
    use ethers::etherscan::Client as EtherscanClient;
    use ethers::types::{BlockNumber, Bytes};

    struct MockClient {
        tx_pages: Vec<Vec<NormalTransaction>>,
        event_pages: Vec<Vec<ERC20TokenTransferEvent>>,
    }

    #[async_trait]
    impl TxSource for MockClient {
        async fn get_transactions(
            &self,
            _address: &Address,
            params: Option<TxListParams>,
        ) -> Result<Vec<NormalTransaction>, Box<dyn Error>> {
            let page = params.map(|p| p.page).unwrap_or(1) as usize;
            Ok(self.tx_pages.get(page - 1).cloned().unwrap_or_default())
        }

        async fn get_erc20_token_transfer_events(
            &self,
            _option: TokenQueryOption,
            params: Option<TxListParams>,
        ) -> Result<Vec<ERC20TokenTransferEvent>, Box<dyn Error>> {
            let page = params.map(|p| p.page).unwrap_or(1) as usize;
            Ok(self.event_pages.get(page - 1).cloned().unwrap_or_default())
        }
    }

    fn make_tx(hash: H256) -> NormalTransaction {
        NormalTransaction {
            is_error: "0".to_string(),
            block_number: BlockNumber::Number(1u64.into()),
            time_stamp: "1".to_string(),
            hash: GenesisOption::Some(hash),
            nonce: None,
            block_hash: None,
            transaction_index: None,
            from: GenesisOption::Some(Address::zero()),
            to: Some(Address::zero()),
            value: U256::zero(),
            gas: U256::zero(),
            gas_price: None,
            tx_receipt_status: "1".to_string(),
            input: Bytes::new(),
            contract_address: None,
            gas_used: U256::zero(),
            cumulative_gas_used: U256::zero(),
            confirmations: 0,
            method_id: None,
            function_name: None,
        }
    }

    fn make_event(hash: H256) -> ERC20TokenTransferEvent {
        ERC20TokenTransferEvent {
            block_number: BlockNumber::Number(1u64.into()),
            time_stamp: "1".to_string(),
            hash,
            nonce: U256::zero(),
            block_hash: H256::zero(),
            from: Address::zero(),
            contract_address: Address::zero(),
            to: Some(Address::zero()),
            value: U256::one(),
            token_name: "T".to_string(),
            token_symbol: "T".to_string(),
            token_decimal: "18".to_string(),
            transaction_index: 0,
            gas: U256::zero(),
            gas_price: None,
            gas_used: U256::zero(),
            cumulative_gas_used: U256::zero(),
            input: String::new(),
            confirmations: 0,
        }
    }

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

    #[tokio::test]
    async fn fetch_transactions_paginates() {
        let tx1 = make_tx(H256::from_low_u64_be(1));
        let tx2 = make_tx(H256::from_low_u64_be(2));
        let mock = MockClient {
            tx_pages: vec![vec![tx1], vec![tx2]],
            event_pages: vec![],
        };

        let res = fetch_transactions(&mock, Address::zero()).await.unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].hash, H256::from_low_u64_be(1));
        assert_eq!(res[1].hash, H256::from_low_u64_be(2));
    }

    #[tokio::test]
    async fn fetch_transactions_paginates_events() {
        let hash = H256::from_low_u64_be(1);
        let tx = make_tx(hash);
        let ev1 = make_event(hash);
        let mut ev2 = make_event(hash);
        ev2.value = U256::from(2u64);
        let mock = MockClient {
            tx_pages: vec![vec![tx]],
            event_pages: vec![vec![ev1], vec![ev2]],
        };

        let res = fetch_transactions(&mock, Address::zero()).await.unwrap();
        assert_eq!(res[0].transfers.len(), 2);
    }
    #[test]
    fn config_loads_toml_and_yaml() {
        let toml_cfg = Config::load(Some("../examples/config.sample.toml")).expect("load toml");
        let yaml_cfg = Config::load(Some("../examples/config.sample.yml")).expect("load yaml");
        assert_eq!(toml_cfg.rpc_url, yaml_cfg.rpc_url);
    }
}
