use ethers::types::Address;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::str::FromStr;

/// Mapping of known good token contract addresses to canonical symbols
pub static GOOD_TOKENS: Lazy<HashMap<Address, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        Address::from_str("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8").unwrap(),
        "USDC",
    );
    m.insert(
        Address::from_str("0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9").unwrap(),
        "USDT",
    );
    m.insert(
        Address::from_str("0xda10009cbd5d07dd0cecc66161fc93d7c9000da1").unwrap(),
        "DAI",
    );
    m.insert(
        Address::from_str("0x2f2a2543b76a4166549f7aab2e75bef0aefc5b63").unwrap(),
        "WBTC",
    );
    m.insert(
        Address::from_str("0x82af49447d8a07e3bd95bd0d56f35241523fbab1").unwrap(),
        "WETH",
    );
    m
});

/// Return the canonical symbol for a token if it exists in the whitelist.
pub fn get_symbol(addr: &Address) -> Option<&'static str> {
    GOOD_TOKENS.get(addr).copied()
}
