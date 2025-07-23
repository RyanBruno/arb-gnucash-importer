use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

use arb_gnucash_importer::blockchain::{self, apply_tags, Config, Tags};
use ethers::types::Address;
use std::fs;

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

    /// Optional config file for tagged addresses
    #[arg(long)]
    tags: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // initialize logging from log4rs config file
    log4rs::init_file("log4rs.yml", Default::default()).expect("failed to init logger");

    let args = Args::parse();
    let cfg = Config::load(None)?;
    let _provider = blockchain::provider(&cfg).await?;

    let address: Address = args.address.parse()?;
    let mut txs = blockchain::fetch_transactions(address).await?;
    if let Some(tags_path) = args.tags.as_deref() {
        let tags = Tags::load(tags_path)?;
        apply_tags(&mut txs, &tags);
    }
    let json = serde_json::to_string_pretty(&txs)?;
    fs::write(args.output, json)?;
    Ok(())
}
