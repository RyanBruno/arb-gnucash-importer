# arb-gnucash-importer

A Rust tool for importing transactions from the Arbitrum network into GnuCash. At the moment the code base provides the skeleton for the backend binary with planned support for retrieving chain data and writing it to GnuCash compatible formats.

## Prerequisites

This project uses the nightly Rust toolchain. Ensure you have [rustup](https://rustup.rs/) installed and the nightly toolchain available. The repo includes a `rust-toolchain.toml` file which will automatically configure the correct toolchain when running Cargo commands.

## Running the binary

The workspace contains a single crate with a binary called `backend`. You can run it with:

```bash
cargo run -p arb-gnucash-importer --bin backend
```

The output JSON contains normal transactions along with any ERC-20 token transfers.

## Configuration

RPC endpoint information is read from a small configuration file. Examples are
provided at [examples/config.sample.toml](examples/config.sample.toml) and
[examples/config.sample.yml](examples/config.sample.yml). Copy one of these
files to `config.toml` or `config.yml` (or another path of your choice) and
adjust the `rpc_url` value to point at your preferred Arbitrum RPC provider.

## API key

Some API requests require an Arbiscan API key. Register for a free account at
[arbiscan.io](https://arbiscan.io) and create a new key from the *API Keys*
section of your profile. Export the key before running the binary:

```bash
export ETHERSCAN_API_KEY=YOUR_KEY
cargo run -p arb-gnucash-importer --bin backend
```

You may also specify `etherscan_api_key` in your configuration file instead of
using the environment variable.

## Planned features

- Fetch transactions from the Arbitrum blockchain.
- Export retrieved data into a format that can be imported by GnuCash.
- Track ERC-20 token transfers associated with each transaction.

## Transaction categories

You can provide a mapping of addresses to transaction categories using the `--categories` option. The file may be TOML, JSON or YAML. Example `categories.yml`:

```yaml
0x1111111111111111111111111111111111111111: Deposit
0x2222222222222222222222222222222222222222: Withdrawal
```
See [examples/categories.sample.toml](examples/categories.sample.toml) for a TOML example.
