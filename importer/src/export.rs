use chrono::{NaiveDate, NaiveDateTime};
use csv::Writer;
use ethers::types::Address;
use ethers::utils::format_units;
use std::error::Error;
use std::fs::File;
use std::path::Path;

use crate::{blockchain, tokens};

/// A single split in a transaction for GnuCash CSV exports
#[derive(Debug)]
pub struct Split {
    pub date: NaiveDate,
    pub description: String,
    pub account: String,
    pub commodity: String,
    pub amount: f64,
}

fn value_to_f64(value: ethers::types::U256, decimals: u32) -> f64 {
    format_units(value, decimals)
        .unwrap_or_else(|_| "0".to_string())
        .parse::<f64>()
        .unwrap_or(0.0)
}

/// Convert blockchain transactions into GnuCash CSV transactions
pub fn from_chain(address: Address, txs: &[blockchain::Transaction]) -> Vec<Split> {
    let mut res = Vec::new();
    for tx in txs {
        let dt = NaiveDateTime::from_timestamp_opt(tx.timestamp as i64, 0)
            .unwrap_or_else(|| NaiveDateTime::from_timestamp(tx.timestamp as i64, 0));
        let date = dt.date();
        let eth_amount = value_to_f64(tx.value, 18);

        let default_desc = if tx.to == Some(address) {
            "deposit".to_string()
        } else {
            "withdrawal".to_string()
        };
        let description = tx
            .description
            .clone()
            .or_else(|| tx.category.clone())
            .unwrap_or_else(|| default_desc.clone());
        let account = tx.category.clone().unwrap_or_else(|| "Unknown".to_string());

        if eth_amount != 0.0 {
            let mut amount = eth_amount;
            if tx.from == address {
                amount = -amount;
            }
            res.push(Split {
                date,
                description: description.clone(),
                account: account.clone(),
                commodity: "ETH".to_string(),
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
                res.push(Split {
                    date,
                    description: description.clone(),
                    account: account.clone(),
                    commodity: sym.to_string(),
                    amount,
                });
            }
        }
    }
    res
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
            tx.amount.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::{Erc20Transfer, Transaction as ChainTx};
    use ethers::types::{H256, U256};
    use std::str::FromStr;

    #[test]
    fn conversion_sets_fields() {
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
            category: Some("Trade".to_string()),
            description: None,
            transfers: vec![transfer],
        };
        let res = from_chain(Address::repeat_byte(0x11), &[chain_tx]);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].commodity, "ETH");
        assert!(res[0].amount < 0.0);
        assert_eq!(res[1].commodity, "USDC");
        assert!(res[1].amount < 0.0);
        assert_eq!(res[0].account, "Trade");
    }
}
