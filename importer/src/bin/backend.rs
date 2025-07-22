use clap::Parser;
use std::path::PathBuf;
use std::error::Error;

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

fn main() -> Result<(), Box<dyn Error>> {
    // initialize logging from log4rs config file
    log4rs::init_file("log4rs.yml", Default::default()).expect("failed to init logger");

    let _args = Args::parse();
    Ok(())
}
