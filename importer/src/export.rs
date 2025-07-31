use chrono::{NaiveDate, NaiveDateTime};
use csv::Writer;
use ethers::types::Address;
use ethers::utils::format_units;
use std::error::Error;
use std::fs::File;
use std::path::Path;

use crate::{blockchain, price, tokens};

/// A single split in a transaction for GnuCash CSV exports
#[derive(Debug)]
pub struct Split {
    pub date: NaiveDate,
    pub description: String,
    pub account: String,
    pub commodity: String,
    pub value: f64,
    pub amount: f64,
}

fn value_to_f64(value: ethers::types::U256, decimals: u32) -> f64 {
    format_units(value, decimals)
        .unwrap_or_else(|_| "0".to_string())
        .parse::<f64>()
        .unwrap_or(0.0)
}

/// Convert blockchain transactions into GnuCash CSV transactions
pub async fn from_chain(
    address: Address,
    txs: &[blockchain::Transaction],
    cache: &mut price::Cache,
) -> Result<Vec<Split>, Box<dyn Error>> {
    let mut res = Vec::new();
    for tx in txs {
        let dt = NaiveDateTime::from_timestamp_opt(tx.timestamp as i64, 0)
            .unwrap_or_else(|| NaiveDateTime::from_timestamp(tx.timestamp as i64, 0));
        let date = dt.date();
        let eth_amount = value_to_f64(tx.value, 18);

        let (description, account) = if tx.to == Some(address) {
            let desc = tx
                .from_tag
                .as_deref()
                .map(|t| format!("from {t}"))
                .unwrap_or_else(|| "deposit".to_string());
            let acc = tx.from_tag.clone().unwrap_or_else(|| "Unknown".to_string());
            (desc, acc)
        } else {
            let desc = tx
                .to_tag
                .as_deref()
                .map(|t| format!("to {t}"))
                .unwrap_or_else(|| "withdrawal".to_string());
            let acc = tx.to_tag.clone().unwrap_or_else(|| "Unknown".to_string());
            (desc, acc)
        };

        if eth_amount != 0.0 {
            let mut amount = eth_amount;
            if tx.from == address {
                amount = -amount;
            }
            let price = cache.price(None, date).await?;
            res.push(Split {
                date,
                description: description.clone(),
                account: account.clone(),
                commodity: "ETH".to_string(),
                value: amount * price,
                amount,
            });
        }

        for tr in &tx.transfers {
            if let Some(sym) = tokens::get_symbol(&tr.token_contract) {
                let decimals = tr.token_decimal.parse::<u32>().unwrap_or(18);
                let mut amount = value_to_f64(tr.value, decimals);
                if tr.from == address {
                    amount = -amount;
                }
                let price = cache.price(Some(tr.token_contract), date).await?;
                res.push(Split {
                    date,
                    description: description.clone(),
                    account: account.clone(),
                    commodity: sym.to_string(),
                    value: amount * price,
                    amount,
                });
            }
        }
    }
    Ok(res)
}

/// Write the provided transactions to `path` in CSV format compatible with GnuCash
pub fn write_csv(path: &Path, txs: &[Split]) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);
    wtr.write_record([
        "Date",
        "Description",
        "Account",
        "Commodity",
        "Value",
        "Amount",
    ])?;
    for tx in txs {
        wtr.write_record([
            tx.date.to_string(),
            tx.description.clone(),
            tx.account.clone(),
            tx.commodity.clone(),
            tx.value.to_string(),
            tx.amount.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blockchain::{Erc20Transfer, Transaction as ChainTx},
        price,
    };
    use chrono::NaiveDate;
    use ethers::types::{Address, H256, U256};
    use std::str::FromStr;

    #[tokio::test]
    async fn conversion_sets_fields() {
        let transfer = Erc20Transfer {
            token_contract: Address::from_str("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8")
                .unwrap(),
            from: Address::repeat_byte(0x11),
            to: Some(Address::repeat_byte(0x22)),
            value: U256::from(5u64),
            token_name: "TEST".to_string(),
            token_symbol: "TST".to_string(),
            token_decimal: "18".to_string(),
        };

        let chain_tx = ChainTx {
            hash: H256::zero(),
            block_number: 1,
            timestamp: 0,
            from: Address::repeat_byte(0x11),
            to: Some(Address::repeat_byte(0x22)),
            value: U256::from(10u64.pow(18)),
            from_tag: Some("alice".to_string()),
            to_tag: Some("bob".to_string()),
            transfers: vec![transfer],
        };
        let mut cache = price::Cache::new(None);
        cache.insert_price(None, NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(), 1.0);
        cache.insert_price(
            Some(Address::from_str("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8").unwrap()),
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            1.0,
        );
        let res = from_chain(Address::repeat_byte(0x11), &[chain_tx], &mut cache)
            .await
            .unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].commodity, "ETH");
        assert!(res[0].value < 0.0);
        assert_eq!(res[1].commodity, "USDC");
        assert!(res[1].value < 0.0);
        assert_eq!(res[0].account, "bob");
    }
}
