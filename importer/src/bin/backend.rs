use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

use arb_gnucash_importer::blockchain::{self, apply_categories, Categories, Config};
use arb_gnucash_importer::export::{self, write_csv, write_transfers_csv};
use ethers::types::Address;

/// Command line arguments for the backend tool
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Arbitrum address
    #[arg(long)]
    address: String,

    /// Output file path
    #[arg(long)]
    output: PathBuf,

    /// Optional config file mapping addresses to transaction categories
    #[arg(long)]
    categories: Option<PathBuf>,

    /// Optional file path to write token transfer details
    #[arg(long)]
    transfers_output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // initialize logging from log4rs config file
    log4rs::init_file("log4rs.yml", Default::default()).expect("failed to init logger");

    let args = Args::parse();
    let cfg = Config::load(None)?;
    let _provider = blockchain::provider(&cfg).await?;
    let client = blockchain::etherscan_client(&cfg)?;

    let address: Address = args.address.parse()?;
    let mut txs = blockchain::fetch_transactions(&client, address).await?;
    if let Some(cat_path) = args.categories.as_deref() {
        let cats = Categories::load(cat_path)?;
        apply_categories(&mut txs, &cats);
    }
    let gnucash_txs = export::from_chain(address, &txs);
    write_csv(&args.output, &gnucash_txs)?;
    if let Some(path) = args.transfers_output.as_deref() {
        write_transfers_csv(path, &txs)?;
    }
    Ok(())
}
