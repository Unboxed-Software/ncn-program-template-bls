# NCN Program Template - BLS Implementation

### 🎯 Core Purpose

This program serves as a template for building decentralized consensus networks where multiple operators can collectively agree on shared state through:

- **BLS signature aggregation**: Efficient cryptographic proof of consensus
- **Economic incentives**: Fee distribution and slashing mechanisms

## 🏛️ Architecture Overview

### System Components Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                    Jito Restaking Infrastructure            │
├─────────────────────────────────────────────────────────────┤
│  NCN Program (Core Consensus Logic)                         │
│  ├── Config Management                                      │
│  ├── Registry Management (Vaults & Operators)               │
│  ├── Epoch State Management                                 │
│  ├── Snapshot System                                        │
│  ├── BLS Voting System                                      │
│  └── Account Lifecycle Management                           │
├─────────────────────────────────────────────────────────────┤
│  Client Libraries                                           │
│  ├── Rust Client (Auto-generated)                           │
│  └── JavaScript Client (Auto-generated)                     │
├─────────────────────────────────────────────────────────────┤
│  CLI Tools                                                  │
│  ├── NCN Program CLI                                        │
│  └── Keepr                                                  │
├─────────────────────────────────────────────────────────────┤
│  Testing & Integration                                      │
│  ├── Unit Tests                                             │
│  ├── Integration Tests                                      │
│  └── Simulation Tests                                       │
└─────────────────────────────────────────────────────────────┘
```

### 📁 Directory Structure Deep Dive

```
ncn-program-template-bls/
├── 📋 Program Core
│   ├── program/                    # Main Solana program entry point
│   │   ├── src/lib.rs             # 21 instruction handlers (initialize→vote→close)
│   │   └── src/*.rs               # Individual instruction implementations
│   └── core/                      # Shared core functionality
│       ├── src/lib.rs             # 24 core modules (crypto, accounts, utils)
│       ├── schemes/               # BLS signature schemes (SHA256, normalized)
│       ├── g1_point.rs           # G1 elliptic curve operations
│       ├── g2_point.rs           # G2 elliptic curve operations
│       └── error.rs              # 64 custom error types
│
├── 🔧 Client SDKs
│   ├── clients/rust/              # Auto-generated Rust client
│   │   └── ncn_program/src/       # Generated account/instruction types
│   └── clients/js/                # Auto-generated JavaScript client
│       └── ncn_program/           # TypeScript definitions & helpers
│
├── 🛠️ CLI Tools
│   └── cli/                       # Comprehensive CLI tooling
│       ├── src/instructions.rs   # CLI instruction wrappers
│       ├── getting_started.md    # CLI usage documentation
│       └── api-docs.md          # Complete API reference
│
├── 🧪 Testing Infrastructure
│   └── integration_tests/
│       ├── tests/fixtures/       # Test fixtures and programs
│       ├── tests/ncn_program/    # 15+ comprehensive test modules
│       └── src/main.rs          # Test harness entry point
│
├── ⚙️ Configuration & Scripts
│   ├── .cargo/config.toml       # Program ID environment variables
│   ├── scripts/generate-clients.js  # Client generation automation
│   ├── format.sh               # Code formatting and testing pipeline
│   ├── generate_client.sh      # Quick client regeneration
│   └── idl/ncn_program.json    # Interface definition (2307 lines)
│
└── 📚 Documentation
    ├── README.md               # This comprehensive guide
    ├── cli/getting_started.md  # CLI quick start guide
    └── cli/api-docs.md        # Complete API documentation
```

## 🔧 Core Components Deep Dive

### 1. Program Instructions (21 Total)

#### **Global Setup Instructions**

- `InitializeConfig`: Creates program configuration with consensus parameters
- `InitializeVaultRegistry`: Sets up vault tracking system
- `RegisterVault`: Adds vaults to the registry (permissionless after handshake)
- `InitializeOperatorRegistry`: Creates operator tracking system
- `RegisterOperator`: Adds operators with BLS public keys
- `UpdateOperatorBN128Keys`: Updates operator cryptographic keys
- `ReallocOperatorRegistry`: Expands operator storage capacity
- `InitializeEpochSnapshot`: Creates immutable epoch state snapshot
- `ReallocEpochSnapshot`: Expands snapshot storage
- `InitializeOperatorSnapshot`: Captures individual operator state

#### **Epoch Management Instructions**

- `InitializeEpochState`: Creates new epoch with voting parameters
- `InitializeWeightTable`: Sets up stake weight calculations
- `SetEpochWeights`: Finalizes voting weights for the epoch
- `SnapshotVaultOperatorDelegation`: Records delegation relationships

#### **Consensus Voting Instructions**

- `CastVote`: Submits BLS aggregated signatures for consensus

#### **Administrative Instructions**

- `AdminSetParameters`: Updates consensus parameters
- `AdminSetNewAdmin`: Changes administrative roles
- `AdminSetWeight`: Manually adjusts stake weights
- `AdminRegisterStMint`: Adds supported stake token mints
- `AdminSetStMint`: Updates stake token configurations

#### **Cleanup Instructions**

- `CloseEpochAccount`: Reclaims rent from finalized accounts

### 2. Account Types (8 Primary Accounts)

#### **Config** - Global program parameters

```rust
pub struct Config {
    ncn: Pubkey,                              // NCN identifier
    tie_breaker_admin: Pubkey,                // Admin for tie-breaking votes
    valid_slots_after_consensus: PodU64,     // Voting window after consensus
    epochs_before_stall: PodU64,             // Epochs before system stalls
    epochs_after_consensus_before_close: PodU64, // Cleanup timing
    starting_valid_epoch: PodU64,            // First valid epoch
    fee_config: FeeConfig,                   // Fee distribution settings
    minimum_stake_weight: StakeWeights,      // Minimum participation threshold
}
```

#### **EpochState** - Per-epoch consensus tracking

```rust
pub struct EpochState {
    ncn: Pubkey,                             // NCN reference
    epoch: PodU64,                           // Epoch number
    vault_count: PodU64,                     // Number of participating vaults
    account_status: EpochAccountStatus,      // State machine status
    set_weight_progress: Progress,           // Weight setting progress
    operator_snapshot_progress: [Progress; 256], // Per-operator progress
    is_closing: PodBool,                     // Cleanup flag
}
```

#### **EpochSnapshot** - mutable epoch state

```rust
pub struct EpochSnapshot {
    ncn: Pubkey,                             // NCN reference
    epoch: PodU64,                           // Epoch number
    operator_count: PodU64,                  // Total operators
    operators_registered: PodU64,           // Active operators
    operators_can_vote_count: PodU64,       // Eligible voters
    total_agg_g1_pubkey: [u8; 32],          // Aggregated public key
    operator_snapshots: [OperatorSnapshot; 256], // Operator states
    minimum_stake_weight: StakeWeights,     // Participation threshold
}
```

#### **OperatorRegistry** - Operator management

```rust
pub struct OperatorRegistry {
    ncn: Pubkey,                             // NCN reference
    operator_list: [OperatorEntry; 256],     // Operator data array
}

pub struct OperatorEntry {
    operator_pubkey: Pubkey,                 // Operator identifier
    g1_pubkey: [u8; 32],                    // BLS G1 public key
    g2_pubkey: [u8; 64],                    // BLS G2 public key
    operator_index: PodU64,                  // Registry index
    slot_registered: PodU64,                 // Registration timestamp
}
```

#### **VaultRegistry** - Vault and token management

```rust
pub struct VaultRegistry {
    ncn: Pubkey,                             // NCN reference
    st_mint_list: [StMintEntry; 1],         // Supported stake tokens
    vault_list: [VaultEntry; 1],            // Registered vaults
}
```

#### **WeightTable** - Stake weight calculations

```rust
pub struct WeightTable {
    ncn: Pubkey,                             // NCN reference
    epoch: PodU64,                           // Target epoch
    vault_count: PodU64,                     // Number of vaults
    table: [WeightEntry; 1],                // Weight entries per vault
}
```

### 3. BLS Cryptography Implementation

#### **Elliptic Curve Operations**

The system uses the BN254 (alt_bn128) elliptic curve for BLS signatures:

- **G1 Points**: 32-byte compressed format for signatures
- **G2 Points**: 64-byte compressed format for public keys
- **Pairing Operations**: Verification through bilinear pairings

#### **Signature Schemes**

Located in `core/src/schemes/`:

- `Sha256Normalized`: Standard SHA-256 based message hashing
- `traits.rs`: Generic signing/verification interfaces

#### **Key Management**

- Private keys: 32-byte scalars for signature generation
- G1 Public keys: For signature aggregation
- G2 Public keys: For verification operations
- Signature verification: Uses Solana's alt_bn128 precompiles

## 🔄 Consensus Workflow

### Phase 1: Initialization (Per NCN)

```
1. Admin creates Config with consensus parameters
2. Initialize VaultRegistry for supported tokens
3. Initialize OperatorRegistry for participant tracking
4. Register supported stake token mints with weights
5. Register vaults (permissionless after NCN approval)
6. Register operators with BLS keypairs
7. Create EpochSnapshot: mutable state checkpoint
8. Initialize operator snapshots for each participant
```

### Phase 2: Epoch Setup (Per Epoch)

```
1. Create EpochState for new consensus round
2. Initialize WeightTable with current vault count
3. SetEpochWeights: Calculate voting power per vault
4. Snapshot vault-operator delegations
```

### Phase 3: Consensus Voting

```
1. Operators generate BLS signatures on consensus data
2. Signatures are aggregated off-chain
3. CastVote instruction submits aggregated signature
4. Program verifies signature against operator set
6. Consensus reached at 66% threshold
```

### Phase 4: Cleanup

```
1. Wait for epochs_after_consensus_before_close epochs
2. CloseEpochAccount reclaims rent from old accounts
3. Fee distribution to stakeholders
4. Prepare for next epoch cycle
```

## 🛠️ CLI Tools & Automation

### NCN Program CLI (`ncn-program-bls-cli`)

#### **Command Categories**

1. **Admin Commands**: Configuration management

   - `admin-create-config`: Initialize program parameters
   - `admin-register-st-mint`: Add supported tokens
   - `admin-set-parameters`: Update consensus settings

2. **Crank Functions**: State maintenance

   - `crank-register-vaults`: Register pending vaults
   - `crank-close-epoch-accounts`: Cleanup finalized epochs

3. **Instructions**: Core program interactions

   - `create-epoch-state`: Start new epoch
   - `cast-vote`: Submit consensus votes
   - `snapshot-vault-operator-delegation`: Capture delegations

4. **Getters**: State queries
   - Query any on-chain account state
   - Inspect epoch progress and voting status

### Keeper Service (`run-keeper`)

The keeper automates epoch management through state transitions:

#### **Keeper States**

1. **SetWeight**: Establish stake weights for epoch
2. **Snapshot**: Capture operator and vault states
3. **Vote**: Monitor and process consensus votes
4. **PostVoteCooldown**: Wait period after consensus
5. **Distribute**: Reward distribution to stakeholders
6. **Close**: Account cleanup and rent reclamation

#### **Keeper Configuration**

- Loop timeout: 10 minutes (configurable)
- Error timeout: 10 seconds
- Automatic state progression
- Error recovery and retry logic

### Operator Service

Manages operator-specific functionality:

- BLS key generation and management
- Vote preparation and submission
- Delegation monitoring
- Reward claiming

## 🧪 Testing Infrastructure

### Integration Tests (15+ Test Modules)

#### **Core Test Coverage**

- `simulation_test.rs`: Complete end-to-end consensus workflow
- `initialize_config.rs`: Configuration testing
- `register_operator.rs`: Operator registration flows
- `cast_vote.rs`: Voting mechanism testing
- `epoch_state.rs`: Epoch lifecycle management

#### **Test Builder Pattern**

The `test_builder.rs` provides a comprehensive testing framework:

```rust
let mut fixture = TestBuilder::new().await;
fixture.initialize_restaking_and_vault_programs().await?;
fixture.create_test_ncn().await?;
fixture.add_vaults_to_test_ncn(&mut test_ncn, 1, Some(mint_keypair)).await?;
```

#### **Simulation Test Workflow**

The main simulation test demonstrates:

1. Setting up 13 test operators with BLS keypairs
2. Creating vault-operator delegations
3. Running complete epoch consensus cycle
4. BLS signature aggregation and verification
5. Consensus achievement and cleanup

### Test Fixtures

- Pre-built program binaries in `integration_tests/tests/fixtures/`
- Mock restaking and vault programs
- Pre-generated keypairs and test data
- Configurable test scenarios

## 🏗️ Local Test Validator

### Overview

The `local-test-validator/` submodule provides a complete testing environment that automatically sets up the Jito Restaking Protocol with all dependencies. This is the fastest way to get a working development environment.

### Quick Start

```bash
# Navigate to the local test validator directory
cd local-test-validator

# Make scripts executable
chmod +x *.sh

# Run the complete setup (one command does everything)
./run.sh
```

This single command will:

1. Start Solana test validator with pre-loaded programs
2. Initialize the complete restaking network
3. Set up 3 operators with proper handshakes
4. Create a vault and delegate tokens
5. Advance validator by epochs for connection warm-up

### What's Included

The local test validator provides:

- **Pre-built Programs**: Jito restaking, vault, and SPL programs
- **Automated Setup**: Complete network initialization script
- **Test Operators**: 3 pre-configured operators with BLS keypairs
- **Time Simulation**: Scripts to advance validator time for epoch testing
- **Clean State**: Fresh ledger on each run with `--reset`

### Key Scripts

- `run.sh`: Main orchestration script that sets up everything
- `validator.sh`: Starts Solana test validator with required programs
- `setup-testing-env.sh`: Initializes the complete restaking network
- `rerun-validator.sh`: Advances validator time for epoch testing

### Generated Assets

After setup, you'll have:

- **Keypairs**: NCN admin, vault admin, operator admins in `./keys/`
- **Addresses**: All important addresses saved in `setup_summary.txt`

For detailed setup instructions and troubleshooting, see the [local-test-validator README](local-test-validator/README.md).

## 🔧 Development Workflow

### Building the Project

```bash
# Build all workspace components
cargo build --release

# Build Solana program
cargo build-sbf --manifest-path program/Cargo.toml

# Install CLI tools
cargo install --path ./cli --bin ncn-program-bls-cli --locked
```

### Code Generation Pipeline

```bash
# 1. Build shank CLI for IDL generation
cargo b && ./target/debug/ncn-program-shank-cli

# 2. Install Node.js dependencies
yarn install

# 3. Generate Rust and JavaScript clients
yarn generate-clients

# 4. Rebuild with new clients
cargo b
```

## 🚀 Deployment Guide

### Environment Configuration

Set up environment variables in `.env` or shell:

```bash
export RPC_URL=http://127.0.0.1:8899
export COMMITMENT="confirmed"
export NCN_PROGRAM_ID="3fKQSi6VzzDUJSmeksS8qK6RB3Gs3UoZWtsQD3xagy45"
export RESTAKING_PROGRAM_ID="RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q"
export VAULT_PROGRAM_ID="Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8"
export KEYPAIR_PATH="~/.config/solana/id.json"
```

### Program Deployment

```bash
# Create buffer account
solana-keygen new -o target/tmp/buffer.json

# Deploy program
solana program deploy --use-rpc --buffer target/tmp/buffer.json \
  --with-compute-unit-price 10000 --max-sign-attempts 10000 \
  target/deploy/ncn_program.so

# Upgrade existing program
solana program write-buffer --use-rpc --buffer target/tmp/buffer.json \
  --with-compute-unit-price 10000 --max-sign-attempts 10000 \
  target/deploy/ncn_program.so

solana program upgrade $(solana address --keypair target/tmp/buffer.json) \
  $(solana address --keypair target/deploy/ncn_program-keypair.json)

# Clean up buffers
solana program close --buffers
```

## 🔐 Security Considerations

### Key Security Features

1. **BLS Signature Verification**: Cryptographic proof of operator consensus
2. **Minimum Stake-weighted Voting**: Economic security through skin-in-the-game
3. **Time-locked Operations**: Prevents hasty state changes
4. **Role-based Access Control**: Admin separation and permissions
5. **Account Rent Protection**: Economic incentives for proper cleanup

### Potential Security Risks

1. **Key Management**: BLS private keys must be securely stored

## 📈 Performance & Scalability

### Current Limitations

- **Maximum Operators**: 256 operators per NCN
- **Maximum Vaults**: Currently limited to 1 vault per registry
- **Signature Verification**: On-chain BLS verification costs
- **Storage Costs**: Large account sizes for snapshots
- **Compute Units**: Complex cryptographic operations

## Optimization Opportunities and Development TODOs

1. Split Operator_Registry into multiple accounts, one PDA per operator to be able to add as much metadata as needed.
1. You should only init the epoch_snapshot account once, but to do that the first time you will need to init the epoch_state and the weight_table first, So consider uncoupling the epoch_snapshot account from the epoch_state account and the weight_table account.
1. instead of having two Instuctions (`ResgiterOperator` and `InitOperatorSnapshot`) they could be only one
1. registering an operators now is being done using two pairing equations, it could all be done by only one by merging the two equations.
1. Remove weight table since it is only one vault, no need to init and set weights every epoch.
1. since it is only one vault, the vault registry is not needed, consider removing it.
1. you can't update the operator snapshots when a new epoch comes before creating the epoch state account first, consider removing it or merging it with the epoch_snapshot account.
1. CLI: run-keeper command is not going to work well, it need to change a bit, it will try to init an epoch_snapshot every epoch, but it should not, epoch_snapshot account init should happen only once at the start of the NCN
1. CLI: Vote command need to be re-written in a way that supports multi-sig aggregation.
1. CLI: registering and operator now will give random G1, G2 pubkeys and a random BN128 privkey, it will log these keys to a file, but you might want to consider giving the operator the options to pass them as params
1. CLI: crank-update-all-vaults are updating

## The command that you need to run to get started

- Build the program and the cli:

```bash
cargo build-sbf
```

- Deploy the program:

```bash
solana program deploy --program-id ./ncn_program-keypair.json target/deploy/ncn_program.so
```

- build and Configure the CLI: refer to the [cli/getting_started.md](cli/getting_started.md) file for more details

- Configure the NCN program:

```bash
# Fund the payer account with 20 SOL for transaction fees
./target/debug/ncn-program-bls-cli admin-fund-account-payer --amount-in-sol 20
sleep 2
# Create and initialize the NCN program configuration with fee wallet, fee bps, consensus slots, and minimum stake weight
./target/debug/ncn-program-bls-cli admin-create-config --ncn-fee-wallet 3ogGQ7nFX6nCa9bkkZ6hwud6VaEQCekCCmNj6ZoWh8MF --ncn-fee-bps 100 --valid-slots-after-consensus 10000 --minimum-stake-weight 100
sleep 2
# Create the vault registry to track supported stake vaults
./target/debug/ncn-program-bls-cli create-vault-registry
sleep 2
# Register vaults that are pending approval and add them to the registry
./target/debug/ncn-program-bls-cli crank-register-vaults
sleep 2
# Register a supported stake token mint and set its weight
./target/debug/ncn-program-bls-cli admin-register-st-mint --weight 10
sleep 2
# Create the operator registry to track BLS operators
./target/debug/ncn-program-bls-cli create-operator-registry
```

- Register all the Operators: Repeat the command for all operators

```bash
./target/debug/ncn-program-bls-cli register-operator --operator <Operator Pubkey> --keypair-path <operator-admin-keypair>
sleep 2
```

- init the epoch_snapshot account:
  Notice that for now you will need to init the epoch_state and the weight table before initing the epoch_snapshot, but this should change later

```bash

./target/debug/ncn-program-bls-cli create-epoch-state
sleep 2
./target/debug/ncn-program-bls-cli create-weight-table
sleep 2
./target/debug/ncn-program-bls-cli set-epoch-weights
sleep 2
./target/debug/ncn-program-bls-cli create-epoch-snapshot
```

- init the operator_snapshot for each operator:

```bash
./target/debug/ncn-program-bls-cli create-operator-snapshot --operator <Operator Pubkey>
```

- Snapshot the vault-operator delegations: and to do so, you will need to make sure that the vault are up to date first:

```bash
./target/debug/ncn-program-bls-cli full-update-vault
sleep 2
./target/debug/ncn-program-bls-cli snapshot-vault-operator-delegation --operator <Operator Pubkey>
```
