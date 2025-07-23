use chrono::{NaiveDate, NaiveDateTime};
use csv::Writer;
use ethers::types::Address;
use ethers::utils::format_units;
use std::error::Error;
use std::fs::File;
use std::path::Path;

use crate::blockchain;

/// Transaction format used for GnuCash CSV exports
#[derive(Debug)]
pub struct Transaction {
    pub date: NaiveDate,
    pub description: String,
    pub account: String,
    pub deposit: Option<f64>,
    pub withdrawal: Option<f64>,
}

fn value_to_f64(value: ethers::types::U256) -> f64 {
    format_units(value, 18)
        .unwrap_or_else(|_| "0".to_string())
        .parse::<f64>()
        .unwrap_or(0.0)
}

/// Convert blockchain transactions into GnuCash CSV transactions
pub fn from_chain(address: Address, txs: &[blockchain::Transaction]) -> Vec<Transaction> {
    txs.iter()
        .map(|tx| {
            let dt = NaiveDateTime::from_timestamp_opt(tx.timestamp as i64, 0)
                .unwrap_or_else(|| NaiveDateTime::from_timestamp(tx.timestamp as i64, 0));
            let date = dt.date();
            let amount = value_to_f64(tx.value);
            let (deposit, withdrawal, description, account) = if tx.to == Some(address) {
                let desc = tx
                    .from_tag
                    .as_deref()
                    .map(|t| format!("from {t}"))
                    .unwrap_or_else(|| "deposit".to_string());
                let acc = tx.from_tag.clone().unwrap_or_else(|| "Unknown".to_string());
                (Some(amount), None, desc, acc)
            } else {
                let desc = tx
                    .to_tag
                    .as_deref()
                    .map(|t| format!("to {t}"))
                    .unwrap_or_else(|| "withdrawal".to_string());
                let acc = tx.to_tag.clone().unwrap_or_else(|| "Unknown".to_string());
                (None, Some(amount), desc, acc)
            };

            Transaction {
                date,
                description,
                account,
                deposit,
                withdrawal,
            }
        })
        .collect()
}

/// Write the provided transactions to `path` in CSV format compatible with GnuCash
pub fn write_csv(path: &Path, txs: &[Transaction]) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);
    wtr.write_record(["Date", "Description", "Account", "Deposit", "Withdrawal"])?;
    for tx in txs {
        wtr.write_record([
            tx.date.to_string(),
            tx.description.clone(),
            tx.account.clone(),
            tx.deposit.map(|v| v.to_string()).unwrap_or_default(),
            tx.withdrawal.map(|v| v.to_string()).unwrap_or_default(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::Transaction as ChainTx;
    use ethers::types::{H256, U256};

    #[test]
    fn conversion_sets_fields() {
        let chain_tx = ChainTx {
            hash: H256::zero(),
            block_number: 1,
            timestamp: 0,
            from: Address::repeat_byte(0x11),
            to: Some(Address::repeat_byte(0x22)),
            value: U256::from(10u64.pow(18)),
            from_tag: Some("alice".to_string()),
            to_tag: Some("bob".to_string()),
            transfers: Vec::new(),
        };
        let res = from_chain(Address::repeat_byte(0x22), &[chain_tx]);
        assert_eq!(res[0].deposit.unwrap(), 1.0);
        assert!(res[0].withdrawal.is_none());
        assert_eq!(res[0].account, "alice");
    }
}
