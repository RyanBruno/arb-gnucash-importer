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

## Planned features

- Fetch transactions from the Arbitrum blockchain.
- Export retrieved data into a format that can be imported by GnuCash.
- Track ERC-20 token transfers associated with each transaction.

## Address tags

You can provide a mapping of addresses to service names using the `--tags` option. The file may be TOML, JSON or YAML. Example `tags.yml`:

```yaml
0x1111111111111111111111111111111111111111: Alice
0x2222222222222222222222222222222222222222: Bob
```
