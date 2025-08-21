# Multi-Signature Aggregation Vote Commands

This document describes the new BLS multi-signature aggregation vote commands that have been implemented to support secure, decentralized voting in the NCN program.

## Overview

The voting system has been completely rewritten to use BLS (Boneh-Lynn-Shacham) multi-signature aggregation instead of the previous weather-based voting. This provides:

- **Cryptographic Security**: BLS signatures provide provable security
- **Multi-Signature Support**: Multiple operators can sign and aggregate their votes
- **Replay Attack Prevention**: Vote counter ensures each signature is unique
- **Efficient Verification**: Single pairing operation verifies all signatures

## Command Structure

### 1. Generate Vote Signature

Generate a BLS signature for a specific message (typically the current vote counter).

```bash
ncn-program-bls-cli generate-vote-signature \
  --private-key <32_BYTE_HEX_PRIVATE_KEY> \
  [--message <32_BYTE_HEX_MESSAGE>]
```

**Parameters:**
- `--private-key`: 32-byte BLS private key in hex format
- `--message`: Optional 32-byte message to sign (defaults to current vote counter)

**Example:**
```bash
# Generate signature using current vote counter as message
ncn-program-bls-cli generate-vote-signature \
  --private-key "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"

# Generate signature for specific message
ncn-program-bls-cli generate-vote-signature \
  --private-key "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" \
  --message "0000000000000001000000000000000000000000000000000000000000000000"
```

### 2. Aggregate Signatures

Combine multiple BLS signatures and public keys into a single aggregated signature.

```bash
ncn-program-bls-cli aggregate-signatures \
  --signatures <COMMA_SEPARATED_64_BYTE_HEX_SIGNATURES> \
  --g1-public-keys <COMMA_SEPARATED_32_BYTE_HEX_G1_KEYS> \
  --g2-public-keys <COMMA_SEPARATED_64_BYTE_HEX_G2_KEYS> \
  --signers-bitmap <HEX_BITMAP>
```

**Parameters:**
- `--signatures`: Comma-separated list of 64-byte BLS signatures in hex
- `--g1-public-keys`: Comma-separated list of 32-byte G1 public keys in hex
- `--g2-public-keys`: Comma-separated list of 64-byte G2 public keys in hex
- `--signers-bitmap`: Hex string indicating which operators signed (1 bit per operator)

**Example:**
```bash
ncn-program-bls-cli aggregate-signatures \
  --signatures "sig1_64_bytes_hex,sig2_64_bytes_hex,sig3_64_bytes_hex" \
  --g1-public-keys "g1_key1_32_bytes_hex,g1_key2_32_bytes_hex,g1_key3_32_bytes_hex" \
  --g2-public-keys "g2_key1_64_bytes_hex,g2_key2_64_bytes_hex,g2_key3_64_bytes_hex" \
  --signers-bitmap "07"  # Binary: 00000111 (operators 0, 1, 2 signed)
```

### 3. Cast Vote

Submit an aggregated vote to the NCN program.

```bash
ncn-program-bls-cli cast-vote \
  --aggregated-signature <32_BYTE_HEX_AGGREGATED_SIGNATURE> \
  --aggregated-g2 <64_BYTE_HEX_AGGREGATED_G2_KEY> \
  --signers-bitmap <HEX_BITMAP> \
  [--message <32_BYTE_HEX_MESSAGE>]
```

**Parameters:**
- `--aggregated-signature`: 32-byte aggregated G1 signature in hex
- `--aggregated-g2`: 64-byte aggregated G2 public key in hex
- `--signers-bitmap`: Hex string indicating which operators signed
- `--message`: Optional 32-byte message that was signed (defaults to current vote counter)

**Example:**
```bash
ncn-program-bls-cli cast-vote \
  --aggregated-signature "agg_sig_32_bytes_hex" \
  --aggregated-g2 "agg_g2_key_64_bytes_hex" \
  --signers-bitmap "07"
```

## Complete Workflow Example

Here's a complete example of how to use the multi-sig aggregation voting system:

### Step 1: Generate Individual Signatures

Each operator generates a signature for the current vote counter:

```bash
# Operator 1
ncn-program-bls-cli generate-vote-signature \
  --private-key "operator1_private_key_32_bytes_hex"

# Operator 2  
ncn-program-bls-cli generate-vote-signature \
  --private-key "operator2_private_key_32_bytes_hex"

# Operator 3
ncn-program-bls-cli generate-vote-signature \
  --private-key "operator3_private_key_32_bytes_hex"
```

### Step 2: Aggregate Signatures

Combine the individual signatures and public keys:

```bash
ncn-program-bls-cli aggregate-signatures \
  --signatures "sig1_64_bytes,sig2_64_bytes,sig3_64_bytes" \
  --g1-public-keys "g1_key1_32_bytes,g1_key2_32_bytes,g1_key3_32_bytes" \
  --g2-public-keys "g2_key1_64_bytes,g2_key2_64_bytes,g2_key3_64_bytes" \
  --signers-bitmap "07"
```

### Step 3: Cast the Vote

Submit the aggregated vote to the blockchain:

```bash
ncn-program-bls-cli cast-vote \
  --aggregated-signature "aggregated_signature_32_bytes" \
  --aggregated-g2 "aggregated_g2_key_64_bytes" \
  --signers-bitmap "07"
```

## Security Considerations

### Vote Counter Protection

The vote counter provides automatic replay attack prevention:

- Each vote increments the counter
- Old signatures become invalid after counter advancement
- No manual nonce management required
- Deterministic message generation

### Quorum Requirements

The system enforces quorum requirements:

- At least 2/3 of operators must sign for consensus
- Non-signers are tracked and excluded from verification
- Partial signature aggregation is supported

### Key Management

- Private keys must be securely stored
- G1 and G2 public keys must be properly registered
- Key rotation is supported through the update mechanism

## Error Handling

Common error scenarios and solutions:

### Invalid Signature Format
```
Error: Signature must be 64 bytes, got 32 for 'invalid_sig'
```
**Solution**: Ensure signatures are properly formatted 64-byte hex strings.

### Quorum Not Met
```
Error: Quorum not met: non-signers count (2) exceeds 1/3 of registered operators (3)
```
**Solution**: Ensure at least 2/3 of operators sign the vote.

### Vote Counter Mismatch
```
Error: Signature verification failed
```
**Solution**: Use the current vote counter value as the message for signing.

### Invalid Bitmap
```
Error: Invalid bitmap size
```
**Solution**: Ensure the bitmap size matches the number of operators.

## Integration with Existing System

The new voting system integrates seamlessly with the existing NCN program:

- **Backward Compatibility**: Old voting commands are deprecated but still available
- **Same Account Structure**: Uses existing vote counter and snapshot accounts
- **Consistent State Management**: Follows the same epoch and state progression
- **Keeper Integration**: Works with existing keeper automation

## Performance Considerations

- **Signature Aggregation**: Off-chain aggregation reduces on-chain computation
- **Single Verification**: One pairing operation verifies all signatures
- **Efficient Storage**: Compressed point formats minimize storage costs
- **Batch Processing**: Multiple votes can be processed efficiently

## Troubleshooting

### Debugging Signature Generation

```bash
# Enable verbose mode for detailed logging
ncn-program-bls-cli --verbose generate-vote-signature \
  --private-key "your_private_key"
```

### Verifying Vote Counter

```bash
# Check current vote counter value
ncn-program-bls-cli get-vote-counter
```

### Checking Operator Registration

```bash
# Verify operator BLS keys are registered
ncn-program-bls-cli get-ncn-operator-state --operator <OPERATOR_PUBKEY>
```

## Advanced Usage

### Custom Message Signing

For advanced use cases, you can sign custom messages:

```bash
# Sign a custom 32-byte message
ncn-program-bls-cli generate-vote-signature \
  --private-key "your_private_key" \
  --message "custom_32_byte_message_hex"
```

### Partial Aggregation

Support for partial signature aggregation when not all operators sign:

```bash
# Aggregate only signatures from operators 0, 2, 4
ncn-program-bls-cli aggregate-signatures \
  --signatures "sig0,sig2,sig4" \
  --g1-public-keys "g1_0,g1_2,g1_4" \
  --g2-public-keys "g2_0,g2_2,g2_4" \
  --signers-bitmap "15"  # Binary: 00010101
```

This new multi-signature aggregation system provides a robust, secure, and efficient voting mechanism for the NCN program while maintaining compatibility with existing infrastructure.
