# NCN Program Template - BLS Implementation

## Table of Contents

- [🎯 Core Purpose](#-core-purpose)
- [🏛️ Architecture Overview](#️-architecture-overview)
  - [System Components Hierarchy](#system-components-hierarchy)
  - [📁 Directory Structure Deep Dive](#-directory-structure-deep-dive)
- [🔧 Core Components Deep Dive](#-core-components-deep-dive)
  - [1. Program Instructions (22 Total)](#1-program-instructions-22-total)
  - [2. Account Types (9 Primary Accounts)](#2-account-types-9-primary-accounts)
  - [3. Vote Counter System](#3-vote-counter-system)
  - [4. BLS Cryptography Implementation](#4-bls-cryptography-implementation)
- [🧮 BN128 Mathematics and Cryptographic Foundations](#-bn128-mathematics-and-cryptographic-foundations)
  - [Core Mathematical Concepts](#core-mathematical-concepts)
  - [BLS Signature Mathematics](#bls-signature-mathematics)
  - [Voting Implementation Mathematics](#voting-implementation-mathematics)
  - [Security Properties](#security-properties)
  - [Performance Considerations](#performance-considerations)
- [🔄 Consensus Workflow](#-consensus-workflow)
  - [Phase 1: Initialization (Per NCN)](#phase-1-initialization-per-ncn)
  - [Phase 2: Epoch Setup (Per Epoch)](#phase-2-epoch-setup-per-epoch)
  - [Phase 3: Consensus Voting](#phase-3-consensus-voting)
  - [Phase 4: Cleanup](#phase-4-cleanup)
- [🛠️ CLI Tools & Automation](#️-cli-tools--automation)
  - [NCN Program CLI](#ncn-program-cli-ncn-program-bls-cli)
  - [Keeper Service](#keeper-service-run-keeper)
  - [Operator Service](#operator-service)
- [🧪 Testing Infrastructure](#-testing-infrastructure)
  - [Integration Tests (15+ Test Modules)](#integration-tests-15-test-modules)
  - [Test Fixtures](#test-fixtures)
- [🏗️ Local Test Validator](#️-local-test-validator)
  - [Overview](#overview)
  - [Quick Start](#quick-start)
  - [What's Included](#whats-included)
  - [Key Scripts](#key-scripts)
  - [Generated Assets](#generated-assets)
- [🔧 Development Workflow](#-development-workflow)
  - [Building the Project](#building-the-project)
  - [Code Generation Pipeline](#code-generation-pipeline)
- [🚀 Deployment Guide](#-deployment-guide)
  - [Environment Configuration](#environment-configuration)
  - [Program Deployment](#program-deployment)
- [🔐 Security Considerations](#-security-considerations)
  - [Key Security Features](#key-security-features)
  - [Potential Security Risks](#potential-security-risks)
- [📈 Performance & Scalability](#-performance--scalability)
  - [Current Limitations](#current-limitations)
- [Optimization Opportunities and Development TODOs](#optimization-opportunities-and-development-todos)
- [The command that you need to run to get started](#the-command-that-you-need-to-run-to-get-started)

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
│   │   ├── src/lib.rs             # 22 instruction handlers (initialize→vote→close)
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

### 1. Program Instructions (22 Total)

#### **Global Setup Instructions**

- `InitializeConfig`: Creates program configuration with consensus parameters
- `InitializeVaultRegistry`: Sets up vault tracking system
- `RegisterVault`: Adds vaults to the registry (permissionless after handshake)
- `InitializeOperatorRegistry`: Creates operator tracking system
- `RegisterOperator`: Adds operators with BLS public keys
- `UpdateOperatorBN128Keys`: Updates operator cryptographic keys
- `ReallocOperatorRegistry`: Expands operator storage capacity
- `InitializeSnapshot`: Creates immutable epoch state snapshot
- `ReallocSnapshot`: Expands snapshot storage
- `InitializeVoteCounter`: Creates vote counter for replay attack prevention
- `InitializeOperatorSnapshot`: Captures individual operator state

#### **Epoch Management Instructions**

- `InitializeEpochState`: Creates new epoch with voting parameters
- `InitializeWeightTable`: Sets up stake weight calculations
- `SetEpochWeights`: Finalizes voting weights for the epoch
- `SnapshotVaultOperatorDelegation`: Records delegation relationships

#### **Consensus Voting Instructions**

- `CastVote`: Submits BLS aggregated signatures for consensus (uses counter for message)

#### **Administrative Instructions**

- `AdminSetParameters`: Updates consensus parameters
- `AdminSetNewAdmin`: Changes administrative roles
- `AdminSetWeight`: Manually adjusts stake weights
- `AdminRegisterStMint`: Adds supported stake token mints
- `AdminSetStMint`: Updates stake token configurations

#### **Cleanup Instructions**

- `CloseEpochAccount`: Reclaims rent from finalized accounts

### 2. Account Types (9 Primary Accounts)

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
    minimum_stake: StakeWeights,      // Minimum participation threshold
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

#### **Snapshot** - mutable epoch state

```rust
pub struct Snapshot {
    ncn: Pubkey,                             // NCN reference
    epoch: PodU64,                           // Epoch number
    operator_count: PodU64,                  // Total operators
    operators_registered: PodU64,           // Active operators
    operators_can_vote_count: PodU64,       // Eligible voters
    total_aggregated_g1_pubkey: [u8; 32],          // Aggregated public key
    operator_snapshots: [OperatorSnapshot; 256], // Operator states
    minimum_stake: StakeWeights,     // Participation threshold
}
```

#### **OperatorRegistry** - Operator management

```rust
pub struct NCNOperatorAccount {
    ncn: Pubkey,                             // NCN reference
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

#### **VoteCounter** - Replay attack prevention

```rust
pub struct VoteCounter {
    ncn: Pubkey,                             // NCN reference
    count: PodU64,                           // Current vote counter
    bump: u8,                                // PDA bump seed
    reserved: [u8; 7],                       // Reserved for future use
}
```

The vote counter tracks the number of successful votes and provides automatic replay attack protection by using the counter value as the message for BLS signature verification.

### 3. Vote Counter System

#### **Purpose & Security Model**

The vote counter is a critical security component that prevents replay attacks by ensuring each vote uses a unique, sequential message:

```rust
// Vote counter provides the message for signature verification
let current_count = vote_counter.count();
let message = current_count.to_le_bytes(); // Padded to 32 bytes
```

#### **Key Properties**

1. **Sequential Uniqueness**: Each vote increments the counter, making old signatures invalid
2. **Deterministic**: No external dependencies - counter value is the message
3. **Atomic Updates**: Counter only increments after successful signature verification
4. **Replay Prevention**: Previous signatures cannot be reused due to counter advancement

#### **Workflow Integration**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Read Counter  │ -> │  Sign Counter   │ -> │ Verify & Update │
│   (N = current) │    │  (Message = N)  │    │   (N = N + 1)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

#### **Security Benefits**

- **No Message Collisions**: Sequential counter ensures unique messages
- **Automatic Protection**: No manual nonce management required
- **Resistant to Attacks**: Replay, precomputation, and signature reuse attacks are prevented
- **Simple Verification**: Anyone can verify counter progression

### 4. BLS Cryptography Implementation

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

## 🧮 BN128 Mathematics and Cryptographic Foundations

### Overview

This section details the mathematical foundations underlying the BLS signature aggregation system implemented in the NCN program. The system uses the BN254 (alt_bn128) elliptic curve to enable efficient signature aggregation and verification through bilinear pairings.

### Core Mathematical Concepts

#### **1. BN254 Elliptic Curve**

The BN254 curve is defined over a finite field with the equation:

```
y² = x³ + 3 (mod p)
```

Where:

- `p = 21888242871839275222246405745257275088696311157297823662689037894645226208583` (prime modulus)
- G1: Points on the base curve over Fp
- G2: Points on the twisted curve over Fp²

#### **2. Curve Points and Generators**

**G1 Generator Point:**

```rust:48:55:core/src/constants.rs
pub const G1_GENERATOR: [u8; 64] = [
    // x coordinate: 1
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    // y coordinate: 2
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
];
```

**Curve Order (MODULUS):**

```rust:37:44:core/src/constants.rs
pub static MODULUS: UBig = unsafe {
    UBig::from_static_words(&[
        0x3c208c16d87cfd47,
        0x97816a916871ca8d,
        0xb85045b68181585d,
        0x30644e72e131a029,
    ])
};
```

### BLS Signature Mathematics

#### **1. Hash-to-Curve Implementation**

The system implements a deterministic hash-to-curve function using SHA-256 with normalization:

```rust:23:47:core/src/schemes/sha256_normalized.rs
impl HashToCurve for Sha256Normalized {
    fn try_hash_to_curve<T: AsRef<[u8]>>(message: T) -> Result<G1Point, NCNProgramError> {
        (0..255)
            .find_map(|n: u8| {
                // Create a hash
                let hash = solana_nostd_sha256::hashv(&[message.as_ref(), &[n]]);

                // Convert hash to a Ubig for Bigint operations
                let hash_ubig = UBig::from_be_bytes(&hash);

                // Check if the hash is higher than our normalization modulus of Fq * 5
                if hash_ubig >= NORMALIZE_MODULUS {
                    return None;
                }

                let modulus_ubig = hash_ubig % &MODULUS;

                // Decompress the point
                match alt_bn128_g1_decompress(&modulus_ubig.to_be_bytes()) {
                    Ok(p) => Some(G1Point(p)),
                    Err(_) => None,
                }
            })
            .ok_or(NCNProgramError::HashToCurveError)
    }
}
```

**Mathematical Process:**

1. **Domain Separation**: `H(message || n)` where `n` is a counter (0-254)
2. **Normalization**: Ensure hash < `NORMALIZE_MODULUS` to avoid bias
3. **Modular Reduction**: `hash mod p` to get field element
4. **Curve Mapping**: Attempt to decompress as G1 point until valid point found

#### **2. BLS Signature Verification**

The system implements two verification modes: single signature and aggregated signature verification.

**Single Signature Verification:**

The verification equation is:

```
e(H(m), PK2) = e(σ, G2_GENERATOR)
```

Which is implemented as:

```
e(H(m), PK2) * e(σ, -G2_GENERATOR) = 1
```

where:

- `H(m)` is the hash of the message
- `PK2` is the G2 public key
- `σ` is the signature
- `G2_GENERATOR` is the generator point for the G2 curve
- `e` is the pairing function

```rust:31:58:core/src/g2_point.rs
pub fn verify_signature<H: HashToCurve, T: AsRef<[u8]>, S: BLSSignature>(
    self,
    signature: S,
    message: T,
) -> Result<(), NCNProgramError> {
    let mut input = [0u8; 384];

    // 1) Hash message to curve
    input[..64].clone_from_slice(&H::try_hash_to_curve(message)?.0);
    // 2) Decompress our public key
    input[64..192].clone_from_slice(&self.0);
    // 3) Decompress our signature
    input[192..256].clone_from_slice(&signature.to_bytes()?);
    // 4) Pair with -G2::one()
    input[256..].clone_from_slice(&G2_MINUS_ONE);

    // Calculate result
    if let Ok(r) = alt_bn128_pairing(&input) {
        msg!("Pairing result: {:?}", r);
        if r.eq(&BN128_ADDITION_SUCESS_RESULT) {
            Ok(())
        } else {
            Err(NCNProgramError::BLSVerificationError)
        }
    } else {
        Err(NCNProgramError::AltBN128PairingError)
    }
}
```

#### **3. Aggregated Signature Verification with Anti-Rogue Key Protection**

The aggregated signature verification uses a sophisticated scheme to prevent rogue key attacks:

**Core Equation:**

```
e(H(m) + α·G1, APK2) = e(σ + α·APK1, G2_GENERATOR)
```

Implemented as:

```
e(H(m) + α·G1, APK2) * e(σ + α·APK1, -G2_GENERATOR) = 1
```

Where:

- `α = H(H(m) || σ || APK1 || APK2)` (anti-rogue key factor)
- `APK1 = Σ(PK1_i)` (aggregated G1 public keys)
- `APK2` = aggregated G2 public key
- `σ` = aggregated signature
- `G2_GENERATOR` = generator point for the G2 curve
- `e` = pairing function

```rust:60:100:core/src/g2_point.rs
pub fn verify_aggregated_signature<H: HashToCurve, T: AsRef<[u8]>, S: BLSSignature>(
    self,
    aggregated_signature: G1Point,
    message: T,
    apk1: G1Point,
) -> Result<(), NCNProgramError> {
    let message_hash = H::try_hash_to_curve(message)?.0;
    let alpha = compute_alpha(&message_hash, &aggregated_signature.0, &apk1.0, &self.0);

    let scaled_g1 = G1Point::from(G1_GENERATOR).mul(alpha)?;
    let scaled_aggregated_g1 = apk1.mul(alpha)?;

    let msg_hash_plus_g1 = G1Point::from(message_hash) + scaled_g1;
    let aggregated_signature_plus_aggregated_g1 = aggregated_signature + scaled_aggregated_g1;

    let mut input = [0u8; 384];

    // Pairing equation is:
    // e(H(m) + G1_Generator * alpha, aggregated_g2) = e(aggregated_signature + aggregated_g1 * alpha, G2_MINUS_ONE)

    // 1) Hash message to curve
    input[..64].clone_from_slice(&msg_hash_plus_g1.0);
    // 2) Decompress our public key
    input[64..192].clone_from_slice(&self.0);
    // 3) Decompress our signature
    input[192..256].clone_from_slice(&aggregated_signature_plus_aggregated_g1.0);
    // 4) Pair with -G2::one()
    input[256..].clone_from_slice(&G2_MINUS_ONE);

    // Calculate result
    if let Ok(r) = alt_bn128_pairing(&input) {
        msg!("Pairing result: {:?}", r);
        if r.eq(&BN128_ADDITION_SUCESS_RESULT) {
            Ok(())
        } else {
            Err(NCNProgramError::BLSVerificationError)
        }
    } else {
        Err(NCNProgramError::AltBN128PairingError)
    }
}
```

**Anti-Rogue Key Factor Computation:**

```rust:55:84:core/src/utils.rs
pub fn compute_alpha(
    message: &[u8; 64],
    signature: &[u8; 64],
    apk1: &[u8; 64],
    apk2: &[u8; 128],
) -> [u8; 32] {
    // Concatenate all inputs
    let mut input = Vec::with_capacity(message.len() + signature.len() + apk1.len() + apk2.len());
    input.extend_from_slice(message);
    input.extend_from_slice(signature);
    input.extend_from_slice(apk1);
    input.extend_from_slice(apk2);

    // Hash the concatenated input
    let hash = solana_nostd_sha256::hashv(&[&input]);

    // Convert hash to UBig and reduce modulo MODULUS
    let hash_ubig = UBig::from_be_bytes(&hash) % MODULUS.clone();
    let mut alpha_bytes = [0u8; 32];
    let hash_bytes = hash_ubig.to_be_bytes();
    // Copy to 32 bytes, pad with zeros if needed
    let pad = 32usize.saturating_sub(hash_bytes.len());
    if pad > 0 {
        alpha_bytes[..pad].fill(0);
        alpha_bytes[pad..].copy_from_slice(&hash_bytes);
    } else {
        alpha_bytes.copy_from_slice(&hash_bytes[hash_bytes.len() - 32..]);
    }
    alpha_bytes
}
```

### Voting Implementation Mathematics

#### **1. Signature Aggregation Logic**

In the `cast_vote` instruction, the system handles partial signature aggregation:

```rust:105:154:program/src/cast_vote.rs
// Aggregate the G1 public keys of operators who signed
let mut aggregated_nonsigners_pubkey: Option<G1Point> = None;
let mut non_signers_count: u64 = 0;

for (i, operator_snapshot) in snapshot.operator_snapshots().iter().enumerate() {
    if i >= operator_count as usize {
        break;
    }

    let byte_index = i / 8;
    let bit_index = i % 8;
    let signed = (operators_signature_bitmap[byte_index] >> bit_index) & 1 == 1;

    if signed {
        let snapshot_epoch =
            get_epoch(operator_snapshot.last_snapshot_slot(), ncn_epoch_length)?;
        let current_epoch = get_epoch(current_slot, ncn_epoch_length)?;
        let has_minimum_stake =
            operator_snapshot.has_minimum_stake_now(current_epoch, snapshot_epoch)?;
        if !has_minimum_stake {
            msg!(
                "The operator {} does not have enough stake to vote",
                operator_snapshot.operator()
            );
            return Err(NCNProgramError::OperatorHasNoMinimumStake.into());
        }
    } else {
        // Convert bytes to G1Point
        let g1_compressed = G1CompressedPoint::from(operator_snapshot.g1_pubkey());
        let g1_point = G1Point::try_from(&g1_compressed)
            .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

        if aggregated_nonsigners_pubkey.is_none() {
            aggregated_nonsigners_pubkey = Some(g1_point);
        } else {
            // Add this G1 pubkey to the aggregate using G1Point addition
            let current = aggregated_nonsigners_pubkey.unwrap();
            aggregated_nonsigners_pubkey = Some(
                current
                    .checked_add(&g1_point)
                    .ok_or(NCNProgramError::AltBN128AddError)?,
            );
        }

        non_signers_count = non_signers_count
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?
    }
}
```

#### **2. Public Key Adjustment for Partial Signatures**

When not all operators sign, the system computes the effective aggregate public key:

```rust:165:207:program/src/cast_vote.rs
let total_aggregate_g1_pubkey_compressed =
    G1CompressedPoint::from(snapshot.total_aggregated_g1_pubkey());
let total_aggregated_g1_pubkey = G1Point::try_from(&total_aggregate_g1_pubkey_compressed)
    .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

let signature_compressed = G1CompressedPoint(aggregated_signature);
let signature = G1Point::try_from(&signature_compressed)
    .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

// If there are no non-signers, we should verify the aggregate signature with the total G1
// pubkey because adding to the initial non-signers pubkey would result in error since it is
// initialized to all zeros and this is not a valid point of the curve BN128
if non_signers_count == 0 {
    msg!("All operators signed, verifying aggregate signature with total G1 pubkey");
    aggregated_g2_point
        .verify_aggregated_signature::<Sha256Normalized, &[u8], G1Point>(
            signature,
            &message_32,
            total_aggregated_g1_pubkey,
        )
        .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
} else {
    msg!("Total non signers: {}", non_signers_count);
    let aggregated_nonsigners_pubkey =
        aggregated_nonsigners_pubkey.ok_or(NCNProgramError::NoNonSignersAggregatedPubkey)?;

    let apk1 = total_aggregated_g1_pubkey
        .checked_add(&aggregated_nonsigners_pubkey.negate())
        .ok_or(NCNProgramError::AltBN128AddError)?;

    msg!("Aggregated non-signers G1 pubkey {:?}", apk1.0);
    msg!("Aggregated G2 pubkey {:?}", aggregated_g2_point.0);

    // One Pairing attempt
    msg!("Verifying aggregate signature one pairing");
    aggregated_g2_point
        .verify_aggregated_signature::<Sha256Normalized, &[u8], G1Point>(
            signature,
            &message_32,
            apk1,
        )
        .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
}
```

**Mathematical Logic:**

- If all operators sign: `APK1 = Σ(PK1_i)` for all i
- If some don't sign: `APK1 = Σ(PK1_i) - Σ(PK1_j)` where j are non-signers
- This is computed as: `APK1 = total_aggregated_g1_pubkey + (-aggregated_nonsigners_pubkey)`

#### **3. Point Negation Implementation**

The point negation operation is crucial for signature aggregation:

```rust:240:264:core/src/g1_point.rs
/// Returns the negation of the point: (x, -y mod p)
pub fn negate(&self) -> Self {
    // x: first 32 bytes, y: last 32 bytes
    let x_bytes = &self.0[0..32];
    let y_bytes = &self.0[32..64];
    let y = UBig::from_be_bytes(y_bytes);
    let neg_y = if y == UBig::ZERO {
        UBig::ZERO
    } else {
        (MODULUS.clone() - y) % MODULUS.clone()
    };
    let mut neg_point = [0u8; 64];
    neg_point[0..32].copy_from_slice(x_bytes);
    let neg_y_bytes = neg_y.to_be_bytes();
    // pad to 32 bytes if needed
    let pad = 32 - neg_y_bytes.len();
    if pad > 0 {
        for i in 0..pad {
            neg_point[32 + i] = 0;
        }
        neg_point[32 + pad..64].copy_from_slice(&neg_y_bytes);
    } else {
        neg_point[32..64].copy_from_slice(&neg_y_bytes);
    }
    G1Point(neg_point)
}
```

### Security Properties

#### **1. Replay Attack Prevention**

The system uses a vote counter as the message:

```rust:62:70:program/src/cast_vote.rs
// Get the current counter value to use as the message for signature verification
let vote_counter_data = vote_counter.data.borrow();
let vote_counter_account = VoteCounter::try_from_slice_unchecked(&vote_counter_data)?;
let current_count = vote_counter_account.count();
let message = current_count.to_le_bytes();
// Pad to 32 bytes for signature verification
let mut message_32 = [0u8; 32];
message_32[..8].copy_from_slice(&message);
```

#### **2. Quorum Requirements**

The system enforces a 2/3 majority for consensus:

```rust:155:163:program/src/cast_vote.rs
// If non_signers_count is more than 1/3 of registered operators, throw an error because quorum didn't meet
if non_signers_count > operator_count / 3 {
    msg!(
        "Quorum not met: non-signers count ({}) exceeds 1/3 of registered operators ({})",
        non_signers_count,
        operator_count
    );
    return Err(NCNProgramError::QuorumNotMet.into());
}
```

#### **3. Stake Weight Validation**

Only operators with sufficient stake can participate:

```rust:118:131:program/src/cast_vote.rs
if signed {
    let snapshot_epoch =
        get_epoch(operator_snapshot.last_snapshot_slot(), ncn_epoch_length)?;
    let current_epoch = get_epoch(current_slot, ncn_epoch_length)?;
    let has_minimum_stake =
        operator_snapshot.has_minimum_stake_now(current_epoch, snapshot_epoch)?;
    if !has_minimum_stake {
        msg!(
            "The operator {} does not have enough stake to vote",
            operator_snapshot.operator()
        );
        return Err(NCNProgramError::OperatorHasNoMinimumStake.into());
    }
}
```

## 🔄 Consensus Workflow

### Phase 1: Initialization (Per NCN)

```
1. Admin creates Config with consensus parameters
2. Initialize VoteCounter for replay attack prevention
3. Initialize VaultRegistry for supported tokens
4. Initialize OperatorRegistry for participant tracking
5. Register supported stake token mints with weights
6. Register vaults (permissionless after NCN approval)
7. Register operators with BLS keypairs
8. Create Snapshot: mutable state checkpoint
9. Initialize operator snapshots for each participant
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
1. System reads current vote counter value as message
2. Operators generate BLS signatures on the counter message
3. Signatures are aggregated off-chain
4. CastVote instruction submits aggregated signature
5. Program verifies signature against current counter value
6. Vote counter is incremented after successful verification
7. Consensus reached at 66% threshold
```

#### **Vote Counter Security Model**

The vote counter provides automatic replay attack protection:

- **Unique Messages**: Each vote uses a sequential counter value as the message
- **Automatic Advancement**: Counter increments after each successful vote
- **Replay Prevention**: Old signatures become invalid after counter advancement
- **No External Dependencies**: Message generation is deterministic and internal

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
- `cast_vote.rs`: Voting mechanism and vote counter testing
  - `test_cast_vote_counter_advancement`: Verifies counter increments correctly
  - `test_cast_vote_duplicate_signature_fails`: Tests replay attack prevention
  - `test_cast_vote_sequential_voting_with_counter_tracking`: Multi-round counter validation
  - `test_cast_vote_wrong_counter_message_fails`: Invalid counter value rejection
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
2. **Vote Counter Replay Protection**: Automatic prevention of signature replay attacks
3. **Minimum Stake Voting**: Economic security through skin-in-the-game
4. **Time-locked Operations**: Prevents hasty state changes
5. **Role-based Access Control**: Admin separation and permissions
6. **Account Rent Protection**: Economic incentives for proper cleanup

### Potential Security Risks

1. **Key Management**: BLS private keys must be securely stored
2. **Vote Counter Synchronization**: Operators must coordinate to sign the correct counter value
3. **Counter Overflow**: Theoretical risk after 2^64 votes (astronomically unlikely)

## 📈 Performance & Scalability

### Current Limitations

- **Maximum Operators**: 256 operators per NCN
- **Maximum Vaults**: Currently limited to 1 vault per registry
- **Signature Verification**: On-chain BLS verification costs
- **Storage Costs**: Large account sizes for snapshots
- **Compute Units**: Complex cryptographic operations

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
./target/debug/ncn-program-bls-cli admin-create-config --ncn-fee-wallet 3ogGQ7nFX6nCa9bkkZ6hwud6VaEQCekCCmNj6ZoWh8MF --ncn-fee-bps 100 --valid-slots-after-consensus 10000 --minimum-stake 100
sleep 2
# Initialize the vote counter for replay attack prevention
./target/debug/ncn-program-bls-cli create-vote-counter
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

- init the snapshot account:
  Notice that for now you will need to init the epoch_state and the weight table before initing the snapshot, but this should change later

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

## Optimization Opportunities and Development TODOs

[x]. Split Operator_Registry into multiple accounts, one PDA per operator to be able to add as much metadata as needed.
[x]. Remove weight table since it is only one vault, no need to init and set weights every epoch.
[x]. You should only init the epoch_snapshot account once, but to do that the first time you will need to init the epoch_state and the weight_table first, So consider uncoupling the epoch_snapshot account from the epoch_state account and the weight_table account.
[x]. CLI: crank-update-all-vaults are updating
[]. since it is only one vault, the vault registry is not needed, consider removing it.
[]. you can't update the operator snapshots when a new epoch comes before creating the epoch state account first, consider removing it or merging it with the epoch_snapshot account.
[]. instead of having two Instructions (`ResgiterOperator` and `InitOperatorSnapshot`) they could be only one
[]. check to see if the operator change its G1 pubkey, are we changing that in the operator_snapshot account or not, if not then we should change it.
[]. registering an operators now is being done using two pairing equations, it could all be done by only one by merging the two equations.
[]. CLI: run-keeper command is not going to work well, it need to change a bit, it will try to init an epoch_snapshot every epoch, but it should not, epoch_snapshot account init should happen only once at the start of the NCN
[]. CLI: Vote command need to be re-written in a way that supports multi-sig aggregation.
[]. CLI: registering and operator now will give random G1, G2 pubkeys and a random BN128 privkey, it will log these keys to a file, but you might want to consider giving the operator the options to pass them as params
