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
2. registering an operators now is being done using two pairing equations, it could all be done by only one by merging the two equations.
3. Remove weight table since it is only one vault, no need to init and set weights every epoch.
4. since it is only one vault, the vault registry is not needed, consider removing it.
5. you can't update the operator snapshots when a new epoch comes before creating the epoch state account first, consider removing it or merging it with the epoch_snapshot account.
6. CLI: run-keeper command is not going to work well, it need to change a bit, it will try to init an epoch_snapshot every epoch, but it should not, epoch_snapshot account init should happen only once at the start of the NCN
7. CLI: Vote command need to be re-written in a way that supports multi-sig aggregation.
8. CLI: registering and operator now will give random G1, G2 pubkeys and a random BN128 privkey, it will log these keys to a file, but you might want to consider giving the operator the options to pass them as params

## 🚧 Known Issues & Limitations

## 🎯 Future Roadmap

### Short-term Improvements (1-3 months)

1. **Enhanced Vault Support**: Multiple vaults per NCN
2. **Improved CLI UX**: Better error messages and help text
3. **Performance Optimizations**: Reduce compute unit costs
4. **Documentation**: Complete API documentation
5. **Security Audit**: Third-party security review

### Medium-term Features (3-6 months)

1. **Automated Rewards**: Keeper-driven fee distribution
2. **Slashing Implementation**: Economic security mechanisms
3. **Multi-Token Support**: Multiple stake token mints
4. **Governance Integration**: On-chain parameter updates
5. **Monitoring Dashboard**: Real-time consensus metrics

### Long-term Vision (6-12 months)

1. **Cross-Chain Consensus**: Bridge to other blockchain networks
2. **Advanced Cryptography**: Post-quantum signature schemes
3. **Layer 2 Integration**: High-frequency consensus on L2
4. **Decentralized Governance**: Community-driven development
5. **Enterprise Features**: SLA guarantees and support

### Resources

- [Jito Restaking Documentation](https://docs.restaking.jito.network)
- [NCN Implementation Guide](https://docs.restaking.jito.network/ncn/00_implementation)
- [Solana Program Development](https://docs.solana.com/developing/on-chain-programs)
- [BLS Signature Specification](https://tools.ietf.org/html/draft-irtf-cfrg-bls-signature)
