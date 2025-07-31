use chrono::NaiveDate;
use ethers::types::Address;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub struct Cache {
    #[serde(skip)]
    pub api_key: Option<String>,
    #[serde(skip)]
    path: Option<PathBuf>,
    #[serde(default)]
    prices: HashMap<String, f64>,
}

impl Cache {
    pub fn load(path: impl AsRef<Path>, api_key: Option<String>) -> Self {
        let p = path.as_ref().to_path_buf();
        let prices = fs::read_to_string(&p)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();
        Self {
            api_key,
            path: Some(p),
            prices,
        }
    }

    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            path: None,
            prices: HashMap::new(),
        }
    }

    pub fn save(&self) {
        if let Some(ref p) = self.path {
            if let Ok(s) = serde_json::to_string(&self.prices) {
                let _ = fs::write(p, s);
            }
        }
    }

    pub fn insert_price(&mut self, address: Option<Address>, date: NaiveDate, price: f64) {
        let key = Self::key(address, date);
        self.prices.insert(key, price);
    }

    fn key(address: Option<Address>, date: NaiveDate) -> String {
        match address {
            Some(a) => format!("{a:?}_{}", date),
            None => format!("eth_{}", date),
        }
    }

    pub async fn price(
        &mut self,
        address: Option<Address>,
        date: NaiveDate,
    ) -> Result<f64, Box<dyn Error>> {
        let key = Self::key(address, date);
        if let Some(p) = self.prices.get(&key).copied() {
            return Ok(p);
        }
        let price = fetch_price(self.api_key.as_deref(), address, date).await?;
        self.prices.insert(key, price);
        Ok(price)
    }
}

async fn fetch_price(
    api_key: Option<&str>,
    address: Option<Address>,
    date: NaiveDate,
) -> Result<f64, Box<dyn Error>> {
    let client = Client::new();
    let mut params: Vec<(String, String)> = vec![("module".into(), "stats".into())];
    if let Some(addr) = address {
        params.push(("action".into(), "tokenpricehistory".into()));
        params.push(("contractaddress".into(), format!("{addr:?}")));
    } else {
        params.push(("action".into(), "ethdailyprice".into()));
    }
    params.push(("date".into(), date.format("%Y-%m-%d").to_string()));
    if let Some(key) = api_key {
        params.push(("apikey".into(), key.to_string()));
    }
    let resp: Value = client
        .get("https://api.arbiscan.io/api")
        .query(&params)
        .send()
        .await?
        .json()
        .await?;
    let price = resp["result"]
        .get(0)
        .and_then(|v| v.get("ethusd").or_else(|| v.get("tokenPriceUSD")))
        .and_then(|v| v.as_str())
        .or_else(|| resp["result"].get("ethusd").and_then(|v| v.as_str()))
        .or_else(|| resp["result"].get("tokenPriceUSD").and_then(|v| v.as_str()))
        .unwrap_or("0");
    Ok(price.parse::<f64>().unwrap_or(0.0))
}
