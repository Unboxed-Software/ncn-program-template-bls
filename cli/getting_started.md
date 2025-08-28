# NCN Program CLI - Getting Started Guide

## 🚀 Quick Start

The NCN Program CLI (`ncn-program-bls-cli`) is a comprehensive command-line interface for interacting with the NCN (Network Consensus Node) Program. This tool enables operators, administrators, and developers to manage consensus networks built on Jito's restaking infrastructure with BLS signature aggregation.

## 📦 Installation & Setup

### 1. Build and Install the CLI

```bash
# Clone the repository (if not already done)
git clone <repository-url>
cd ncn-program-template-bls

# Build the entire workspace
cargo build --release

# Install the CLI globally
cargo install --path ./cli --bin ncn-program-bls-cli --locked
```

Verify installation:

```bash
ncn-program-bls-cli --help
```

### 2. Install Dependencies

Install the Jito Restaking CLI for integration workflows:

```bash
# Clone in a separate directory
cd ..
git clone https://github.com/jito-foundation/restaking.git
cd restaking

# Build and install the restaking CLI
cargo build --release
cargo install --path ./cli --bin jito-restaking-cli
```

Verify installation:

```bash
jito-restaking-cli --help
```

## ⚙️ Configuration

### Environment Variables

The CLI supports both environment variables and command-line arguments. Set up your environment:

```bash
# Network Configuration
export RPC_URL="https://api.mainnet-beta.solana.com"  # or devnet/testnet
export COMMITMENT="confirmed"  # confirmed, finalized, or processed

# Program IDs
export NCN_PROGRAM_ID="3fKQSi6VzzDUJSmeksS8qK6RB3Gs3UoZWtsQD3xagy45"
export RESTAKING_PROGRAM_ID="RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q"
export VAULT_PROGRAM_ID="Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8"
export TOKEN_PROGRAM_ID="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"

# Account Settings
export KEYPAIR_PATH="~/.config/solana/id.json"
export NCN="<YOUR_NCN_ADDRESS>"  # Set after NCN creation
export VAULT="<YOUR_VAULT_ADDRESS>"  # Set after vault creation
```

### Using .env File

Create a `.env` file in the CLI directory:

```bash
# Copy example configuration
cp .env.example .env

# Edit with your settings
nano .env
```

### Network Selection

**Mainnet** (Production):

```bash
export RPC_URL="https://api.mainnet-beta.solana.com"
```

**Devnet** (Testing):

```bash
export RPC_URL="https://api.devnet.solana.com"
```

**Testnet** (Validation):

```bash
export RPC_URL="https://api.testnet.solana.com"
```

**Local** (Development):

```bash
export RPC_URL="http://localhost:8899"
```

## 🔧 Command Structure

The CLI is organized into logical command groups:

### **Keeper Command** - Automated epoch management

- `run-keeper`: Automated state transitions and maintenance
- Handles complete epoch lifecycle management
- Monitors and responds to network state changes

### **Admin Commands** - Network administration

- Configuration management
- Parameter updates
- Admin role management
- Account funding

### **Instruction Commands** - Core program interactions

- Registry creation and management
- Operator and vault registration
- Epoch state management
- Snapshot operations
- Voting processes

### **Crank Functions** - Automated maintenance

- Batch vault updates
- Registry synchronization
- Account cleanup
- Epoch transitions

### **Getter Commands** - State queries

- Account information retrieval
- Network state inspection
- Progress monitoring
- Relationship queries

## 🤖 Automated Operation with Keeper

### The Keeper Service

The keeper is the **primary operational tool** for NCN networks. It automates the complete epoch lifecycle through intelligent state management.

```bash
# Start automated keeper service
ncn-program-bls-cli run-keeper

# Custom timeout settings
ncn-program-bls-cli run-keeper \
  --loop-timeout-ms 300000 \     # 5 minutes between checks
  --error-timeout-ms 5000        # 5 seconds on errors
```

### Keeper State Machine

The keeper automatically progresses through these states:

1. **SetWeight** - Establish stake weights for the epoch
2. **Snapshot** - Capture operator and vault state snapshots
3. **Vote** - Monitor and process consensus votes
4. **PostVoteCooldown** - Wait period after voting completes
5. **Close** - Account cleanup and rent reclamation

### Keeper Features

- **Automatic State Detection**: Monitors on-chain state and determines appropriate actions
- **Error Recovery**: Retries failed operations with exponential backoff
- **Epoch Progression**: Automatically moves to next epoch when current epoch stalls
- **Resource Management**: Handles account creation, reallocation, and cleanup
- **Comprehensive Logging**: Detailed logs of all operations and state changes

## 📋 Manual Operation Workflows

### 1. Initial Network Setup (Admin)

#### Step 1: Fund Your Account

```bash
# Ensure your keypair has sufficient SOL
solana balance

# Fund account payer if needed
ncn-program-bls-cli admin-fund-account-payer --amount-in-sol 10
```

#### Step 2: Create NCN Configuration

```bash
# Initialize the NCN program configuration
ncn-program-bls-cli admin-create-config \
  --ncn-fee-wallet <FEE_WALLET_PUBKEY> \
  --ncn-fee-bps 100 \
  --tie-breaker-admin <ADMIN_PUBKEY> \
  --minimum-stake 1000000000 \
  --epochs-before-stall 10 \
  --valid-slots-after-consensus 43200 \
  --epochs-after-consensus-before-close 5
```

#### Step 3: Initialize Registries

```bash
# Create vault registry for managing supported tokens
ncn-program-bls-cli create-vault-registry

# Create operator registry for managing consensus participants
ncn-program-bls-cli create-operator-registry
```

#### Step 4: Register Supported Assets

```bash
# Register a stake token mint with voting weight
ncn-program-bls-cli admin-register-st-mint \
  --weight 1000000000
```

### 2. Vault Integration

#### Register Vaults

```bash
# Register individual vault
ncn-program-bls-cli register-vault

# Bulk register all eligible vaults
ncn-program-bls-cli crank-register-vaults
```

### 3. Operator Onboarding

#### Register New Operator

```bash
# Register operator with auto-generated BLS keys
ncn-program-bls-cli register-operator \
  --operator <OPERATOR_PUBKEY>

# Register with custom BLS keys
ncn-program-bls-cli register-operator \
  --operator <OPERATOR_PUBKEY> \
  --g1-pubkey <G1_PUBKEY_HEX> \
  --g2-pubkey <G2_PUBKEY_HEX> \
  --signature <SIGNATURE_HEX> \
  --keys-file "custom-keys.json"
```

### 4. Manual Epoch Consensus Cycle (Advanced)

> **⚠️ Note**: Manual epoch management is complex. Use the keeper service for production deployments.

#### Phase 1: Epoch Initialization

```bash
# Create new epoch state
ncn-program-bls-cli create-epoch-state

# Initialize weight table for stake calculations
ncn-program-bls-cli create-weight-table

# Set final weights for voting power
ncn-program-bls-cli set-epoch-weights
```

#### Phase 2: State Snapshots

```bash
# Create immutable snapshot
ncn-program-bls-cli create-epoch-snapshot

# Snapshot individual operators
ncn-program-bls-cli create-operator-snapshot \
  --operator <OPERATOR_PUBKEY>

# Snapshot vault-operator delegations
ncn-program-bls-cli snapshot-vault-operator-delegation \
  --operator <OPERATOR_PUBKEY>
```

#### Phase 3: Consensus Voting

```bash
# Cast operator vote (with BLS signature)
ncn-program-bls-cli operator-cast-vote \
  --operator <OPERATOR_PUBKEY> \
  --weather-status 1  # Example: 1=Sunny, 2=Cloudy, 3=Rainy
```

#### Phase 4: Cleanup

```bash
# Close finalized epoch accounts to reclaim rent
ncn-program-bls-cli crank-close-epoch-accounts
```

## 🔍 Monitoring & Inspection

### Network State Queries

```bash
# View NCN configuration
ncn-program-bls-cli get-ncn-program-config

# List all operators in the network
ncn-program-bls-cli get-all-operators-in-ncn

# List all registered vaults
ncn-program-bls-cli get-all-vaults-in-ncn

# Check current epoch state
ncn-program-bls-cli get-epoch-state

# View consensus progress
ncn-program-bls-cli get-epoch-snapshot
```

### Account-Specific Queries

```bash
# Check operator status
ncn-program-bls-cli get-operator-snapshot \
  --operator <OPERATOR_PUBKEY>

# View vault-operator relationship
ncn-program-bls-cli get-vault-operator-delegation \
  --operator <OPERATOR_PUBKEY>

# Check consensus result
ncn-program-bls-cli get-consensus-result

# Monitor system health
ncn-program-bls-cli get-account-payer
```

### System Analytics

```bash
# View total rent costs for current epoch
ncn-program-bls-cli get-total-epoch-rent-cost

# Examine stake distributions
ncn-program-bls-cli get-operator-stakes
ncn-program-bls-cli get-vault-stakes
ncn-program-bls-cli get-vault-operator-stakes
```

## 🛠️ Advanced Operations

### Admin Management

```bash
# Update consensus parameters
ncn-program-bls-cli admin-set-parameters \
  --epochs-before-stall 15 \
  --valid-slots-after-consensus 21600 \
  --starting-valid-epoch 1000

# Transfer admin roles
ncn-program-bls-cli admin-set-new-admin \
  --new-admin <NEW_ADMIN_PUBKEY> \
  --set-tie-breaker-admin

# Manually adjust stake weights
ncn-program-bls-cli admin-set-weight \
  --weight 2000000000
```

### Maintenance Operations

```bash
# Comprehensive vault updates
ncn-program-bls-cli full-update-vaults

# Batch snapshot operations
ncn-program-bls-cli crank-snapshot
```

## 🔑 BLS Key Management

The CLI automatically manages BLS (Boneh-Lynn-Shacham) cryptographic keys for operators:

### Key Generation

- **G1 Keys**: 32-byte compressed points for signatures
- **G2 Keys**: 64-byte compressed points for public keys
- **Signatures**: 64-byte BLS signatures for authentication

### Key Storage

```bash
# Default key file location
./bls-keys.json

# Custom key file
ncn-program-bls-cli register-operator \
  --operator <OPERATOR_PUBKEY> \
  --keys-file "operator-keys.json"
```

### Key File Format

```json
{
  "private_key": "hex_encoded_32_bytes",
  "g1_pubkey": "hex_encoded_32_bytes",
  "g2_pubkey": "hex_encoded_64_bytes",
  "signature": "hex_encoded_64_bytes"
}
```

## 📊 Output Formats & Verbose Mode

### Standard Output

Most commands provide structured JSON output for programmatic parsing.

### Verbose Mode

Enable detailed logging and progress information:

```bash
ncn-program-bls-cli --verbose <command>
```

### Progress Indicators

Long-running operations show progress bars and status updates.

## ⚠️ Common Issues & Troubleshooting

### Keeper Service Issues

```bash
# Check keeper state and logs
ncn-program-bls-cli --verbose run-keeper

# Reduce timeout for faster debugging
ncn-program-bls-cli run-keeper \
  --loop-timeout-ms 60000 \
  --error-timeout-ms 5000
```

### Transaction Failures

```bash
# Increase priority fees for congested networks
export PRIORITY_FEE_MICRO_LAMPORTS="5000"

# Add retry attempts for unreliable networks
export TRANSACTION_RETRIES="5"
```

### Account Not Found

```bash
# Verify environment variables are set
ncn-program-bls-cli --verbose get-ncn-program-config

# Check network connectivity
solana cluster-version
```

### Insufficient Balance

```bash
# Check account balance
solana balance

# Fund account if needed
solana airdrop 2  # On devnet/testnet only
```

### BLS Key Issues

```bash
# Regenerate keys if corrupted
rm bls-keys.json
ncn-program-bls-cli register-operator --operator <OPERATOR_PUBKEY>

# Verify key format
cat bls-keys.json | jq '.'
```

## 🔗 Production Deployment Examples

### Keeper Service Deployment

```bash
#!/bin/bash
# production-keeper.sh

# Set production configuration
export RPC_URL="https://api.mainnet-beta.solana.com"
export COMMITMENT="finalized"
export PRIORITY_FEE_MICRO_LAMPORTS="2000"
export TRANSACTION_RETRIES="10"

# Enhanced keeper settings for production
export LOOP_TIMEOUT_MS="300000"    # 5 minutes
export ERROR_TIMEOUT_MS="30000"    # 30 seconds on errors

# Start keeper with logging
ncn-program-bls-cli --verbose run-keeper \
  --loop-timeout-ms $LOOP_TIMEOUT_MS \
  --error-timeout-ms $ERROR_TIMEOUT_MS \
  2>&1 | tee keeper.log
```

### Automated Setup Script

```bash
#!/bin/bash
# ncn-setup.sh

set -e
echo "Setting up NCN consensus network..."

# Initialize configuration
ncn-program-bls-cli admin-create-config \
  --ncn-fee-wallet $NCN_FEE_WALLET \
  --ncn-fee-bps 100 \
  --minimum-stake 1000000000

# Create registries
ncn-program-bls-cli create-vault-registry
ncn-program-bls-cli create-operator-registry

# Register supported assets
ncn-program-bls-cli admin-register-st-mint --weight 1000000000

echo "✅ NCN setup completed!"
echo "Start the keeper service with: ncn-program-bls-cli run-keeper"
```

### Monitoring Script

```bash
#!/bin/bash
# monitor-ncn.sh

while true; do
  echo "=== NCN Network Status $(date) ==="

  # Check epoch state
  EPOCH_STATE=$(ncn-program-bls-cli get-epoch-state 2>/dev/null || echo "null")
  if [ "$EPOCH_STATE" != "null" ]; then
    CURRENT_EPOCH=$(echo $EPOCH_STATE | jq -r '.epoch')
    echo "Current epoch: $CURRENT_EPOCH"
  fi

  # Check operator count
  OPERATORS=$(ncn-program-bls-cli get-all-operators-in-ncn 2>/dev/null || echo "[]")
  OPERATOR_COUNT=$(echo $OPERATORS | jq '. | length')
  echo "Registered operators: $OPERATOR_COUNT"

  # Check consensus status
  CONSENSUS=$(ncn-program-bls-cli get-consensus-result 2>/dev/null || echo "null")
  if [ "$CONSENSUS" != "null" ]; then
    echo "Consensus: $CONSENSUS"
  fi

  echo "---"
  sleep 30
done
```

## 📚 Additional Resources

### Help & Documentation

```bash
# General help
ncn-program-bls-cli --help

# Command-specific help
ncn-program-bls-cli run-keeper --help
ncn-program-bls-cli admin-create-config --help

# Markdown documentation
ncn-program-bls-cli --markdown-help > cli-reference.md
```

### Related Tools

- **Solana CLI**: Basic blockchain operations
- **Jito Restaking CLI**: Infrastructure management
- **Anchor CLI**: Program development and deployment

### External Links

- [NCN Program Template Repository](https://github.com/jito-foundation/ncn-program-template)
- [Jito Restaking Documentation](https://docs.restaking.jito.network)
- [BLS Signature Specification](https://tools.ietf.org/html/draft-irtf-cfrg-bls-signature)
- [Solana Program Development](https://docs.solana.com/developing/on-chain-programs)

---

## 🚀 Recommended Getting Started Path

1. **Setup Environment**: Configure all required environment variables
2. **Install Tools**: Build and install NCN CLI and Jito Restaking CLI
3. **Initialize Network**: Run admin setup commands to create NCN configuration
4. **Register Participants**: Add operators and vaults to the network
5. **Start Keeper**: Launch the automated keeper service for ongoing operations
6. **Monitor**: Use getter commands to monitor network health and consensus progress

**For Production**: Always use the keeper service (`run-keeper`) for automated epoch management rather than manual commands.

**Next Steps**: After completing the basic setup, refer to the [API Documentation](api-docs.md) for detailed command reference and advanced usage patterns.

**Support**: For issues or questions, consult the main project README.md or reach out to the Jito community on Discord.
