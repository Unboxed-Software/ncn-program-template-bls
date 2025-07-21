# NCN Program Template

## Program Summary

The NCN (Network Consensus Node) Program is a Solana program designed for reaching consensus on weather status in a decentralized network. It manages the collection, voting, and consensus mechanisms across vaults and operators in the ecosystem, leveraging Jito's restaking infrastructure.

Key features:

- Stake-weighted voting mechanism (66% consensus threshold)
- Epoch-based consensus cycles
- Support for multiple stake token mints with configurable weights
- Weather status system (Sunny, Cloudy, Rainy)
- Admin controls for configuration and tie-breaking
- Fee distribution to stakeholders after consensus

## NCN Guides and Tutorials

For more information about Jito (Re)Staking, take a look at the [docs](https://docs.restaking.jito.network). You can also find an in-depth tutorial on NCN programs (and this NCN program, specifically) [here](https://docs.restaking.jito.network/ncn/00_implementation).

## Testing Setup

### Prerequisites

1. Build the ncn program: `cargo build-sbf --manifest-path program/Cargo.toml --sbf-out-dir integration_tests/tests/fixtures`
2. Run tests: `SBF_OUT_DIR=integration_tests/tests/fixtures cargo test`

## Usage Flow

1. **Initialize** the program with configuration, vault registry, and core accounts
2. **Setup Epochs** by creating epoch state and weight tables for each consensus period
3. **Create Snapshots** of operators and vaults to establish voting weights
4. **Cast Votes** on weather status with influence based on stake weight
5. **Achieve Consensus** when votes for a status reach ≥66% of total stake weight
6. **Clean Up** accounts after sufficient time has passed to reclaim rent

## Customization

While this implementation uses weather status as the consensus target, the framework can be adapted for various applications:

- Replace weather status with other vote data
- Modify consensus thresholds
- Adjust epoch and timing parameters

## Deploy

- build .so file: `cargo-build-sbf`

- create a new keypair: `solana-keygen new -o target/tmp/buffer.json`

- Deploy: `solana program deploy --use-rpc --buffer target/tmp/buffer.json --with-compute-unit-price 10000 --max-sign-attempts 10000 target/deploy/ncn_program.so`

## Upgrade

- (Pre Upgrade) Write to buffer: `solana program write-buffer --use-rpc --buffer target/tmp/buffer.json --with-compute-unit-price 10000 --max-sign-attempts 10000 target/deploy/ncn_program.so`

- Upgrade: `solana program upgrade $(solana address --keypair target/tmp/buffer.json) $(solana address --keypair target/deploy/ncn_program-keypair.json)`

- Close Buffers: `solana program close --buffers`

- Upgrade Program Size: `solana program extend $(solana address --keypair target/deploy/ncn_program_program-keypair.json) 100000`

## More info

You can check the docs for more info in the `cli/` directory. See [getting_started.md](cli/getting_started.md) and [api-docs.md](cli/api-docs.md) for details.
