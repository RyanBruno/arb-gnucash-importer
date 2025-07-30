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
    m.insert(
        Address::from_str("0xaf88d065e77c8cc2239327c5edb3a432268e5831").unwrap(),
        "USDC",
    );
    m.insert(
        Address::from_str("0x724dc807b04555b71ed48a6896b6f41593b8c637").unwrap(),
        "USDC",
    );
    m.insert(
        Address::from_str("0x078f358208685046a11c85e8ad32895ded33a249").unwrap(),
        "WBTC",
    );
    m.insert(
        Address::from_str("0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f").unwrap(),
        "WBTC",
    );
    m.insert(
        Address::from_str("0xf611aeb5013fd2c0511c9cd55c7dc5c1140741a6").unwrap(),
        "Debt USDC",
    );
    m.insert(
        Address::from_str("0x92b42c66840c7ad907b4bf74879ff3ef7c529473").unwrap(),
        "Debt WBTC",
    );
    m.insert(
        Address::from_str("0x912ce59144191c1204e64559fe8253a0e49e6548").unwrap(),
        "ARB",
    );
    m.insert(
        Address::from_str("0xe50fa9b3c56ffb159cb0fca61f5c9d750e8128c8").unwrap(),
        "WETH",
    );
    m.insert(
        Address::from_str("0x6533afac2e7bccb20dca161449a13a32d391fb00").unwrap(),
        "ARB",
    );
    m.insert(
        Address::from_str("0x0c84331e39d6658cd6e6b9ba04736cc4c4734351").unwrap(),
        "Debt WETH",
    );
    m.insert(
        Address::from_str("0x953a573793604af8d41f306feb8274190db4ae0e").unwrap(),
        "Debt LINK",
    );
    m.insert(
        Address::from_str("0x18248226c16bf76c032817854e7c83a2113b4f06").unwrap(),
        "Debt GHO",
    );
    m.insert(
        Address::from_str("0x191c10aa4af7c30e871e70c95db0e4eb77237530").unwrap(),
        "LINK",
    );
    m.insert(
        Address::from_str("0x44705f578135cc5d703b4c9c122528c73eb87145").unwrap(),
        "Debt ARB",
    );
    m
});

/// Return the canonical symbol for a token if it exists in the whitelist.
pub fn get_symbol(addr: &Address) -> Option<&'static str> {
    GOOD_TOKENS.get(addr).copied()
}
