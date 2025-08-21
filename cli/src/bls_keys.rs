use anyhow::{anyhow, Result};
use chrono::Utc;
use ncn_program_core::{
    g1_point::G1CompressedPoint, g2_point::G2CompressedPoint, privkey::PrivKey,
    schemes::Sha256Normalized,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use solana_sdk::pubkey::Pubkey;
use std::{fs, path::Path};

/// BLS key set for an operator
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlsKeySet {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub private_key: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub g1_pubkey: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub g2_pubkey: [u8; 64],
    pub operator: String,
    pub created_at: String,
}

/// Container for multiple operator key sets
#[derive(Debug, Serialize, Deserialize)]
pub struct BlsKeyStorage {
    pub operators: std::collections::HashMap<String, BlsKeySet>,
}

impl BlsKeyStorage {
    pub fn new() -> Self {
        Self {
            operators: std::collections::HashMap::new(),
        }
    }
}

/// Generate a new BLS keypair for an operator
pub fn generate_bls_keypair(operator: &Pubkey) -> Result<BlsKeySet> {
    // Generate random private key
    let privkey = PrivKey::from_random();

    // Derive G1 compressed public key (32 bytes)
    let g1_compressed = G1CompressedPoint::try_from(privkey)
        .map_err(|e| anyhow!("Failed to generate G1 public key: {:?}", e))?;

    // Derive G2 compressed public key (64 bytes)
    let g2_compressed = G2CompressedPoint::try_from(&privkey)
        .map_err(|e| anyhow!("Failed to generate G2 public key: {:?}", e))?;

    Ok(BlsKeySet {
        private_key: privkey.0,
        g1_pubkey: g1_compressed.0,
        g2_pubkey: g2_compressed.0,
        operator: operator.to_string(),
        created_at: Utc::now().to_rfc3339(),
    })
}

/// Generate BLS signature by signing the G1 public key
pub fn generate_signature(key_set: &BlsKeySet) -> Result<[u8; 64]> {
    let privkey = PrivKey(key_set.private_key);

    let signature = privkey
        .sign::<Sha256Normalized, &[u8; 32]>(&key_set.g1_pubkey)
        .map_err(|e| anyhow!("Failed to generate signature: {:?}", e))?;

    Ok(signature.0)
}

/// Generate BLS signature from private key and message
pub fn generate_signature_from_private_key(
    private_key: &[u8; 32],
    message: &[u8; 32],
) -> Result<[u8; 64]> {
    let privkey = PrivKey(*private_key);

    let signature = privkey
        .sign::<Sha256Normalized, &[u8; 32]>(message)
        .map_err(|e| anyhow!("Failed to generate signature: {:?}", e))?;

    Ok(signature.0)
}

/// Load BLS keys from file
pub fn load_keys_from_file<P: AsRef<Path>>(file_path: P) -> Result<BlsKeyStorage> {
    let file_path = file_path.as_ref();

    if !file_path.exists() {
        return Ok(BlsKeyStorage::new());
    }

    let content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read keys file {}: {}", file_path.display(), e))?;

    let storage: BlsKeyStorage = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse keys file {}: {}", file_path.display(), e))?;

    Ok(storage)
}

/// Save BLS keys to file
pub fn save_keys_to_file<P: AsRef<Path>>(storage: &BlsKeyStorage, file_path: P) -> Result<()> {
    let file_path = file_path.as_ref();

    let content = serde_json::to_string_pretty(storage)
        .map_err(|e| anyhow!("Failed to serialize keys: {}", e))?;

    fs::write(file_path, content)
        .map_err(|e| anyhow!("Failed to write keys file {}: {}", file_path.display(), e))?;

    println!("BLS keys saved to {}", file_path.display());

    Ok(())
}

/// Get or generate BLS keys for an operator
pub fn get_or_generate_keys(operator: &Pubkey, keys_file: &str) -> Result<BlsKeySet> {
    let mut storage = load_keys_from_file(keys_file)?;
    let operator_str = operator.to_string();

    // Check if keys already exist for this operator
    if let Some(existing_keys) = storage.operators.get(&operator_str) {
        println!("Using existing BLS keys for operator {}", operator);
        return Ok(existing_keys.clone());
    }

    // Generate new keys
    println!("Generating new BLS keys for operator {}", operator);
    let key_set = generate_bls_keypair(operator)?;

    // Save to storage
    storage
        .operators
        .insert(operator_str.clone(), key_set.clone());
    save_keys_to_file(&storage, keys_file)?;

    Ok(key_set)
}

/// Generate BLS keys with optional G1 and G2 pubkeys
pub fn generate_or_use_keys(
    operator: &Pubkey,
    keys_file: &str,
    g1_pubkey: Option<&str>,
    g2_pubkey: Option<&str>,
) -> Result<BlsKeySet> {
    let mut storage = load_keys_from_file(keys_file)?;
    let operator_str = operator.to_string();

    // Check if keys already exist for this operator
    if let Some(existing_keys) = storage.operators.get(&operator_str) {
        println!("Using existing BLS keys for operator {}", operator);

        // If G1 or G2 pubkeys are provided, update the existing keys
        if let (Some(g1), Some(g2)) = (g1_pubkey, g2_pubkey) {
            println!("Updating existing keys with provided G1 and G2 pubkeys");
            let mut updated_keys = existing_keys.clone();
            updated_keys.g1_pubkey = hex_to_bytes::<32>(g1)?;
            updated_keys.g2_pubkey = hex_to_bytes::<64>(g2)?;

            // Save updated keys
            storage
                .operators
                .insert(operator_str.clone(), updated_keys.clone());
            save_keys_to_file(&storage, keys_file)?;

            return Ok(updated_keys);
        }

        return Ok(existing_keys.clone());
    }

    // Generate new keys
    println!("Generating new BLS keys for operator {}", operator);
    let mut key_set = generate_bls_keypair(operator)?;

    // If G1 and G2 pubkeys are provided, use them instead of generated ones
    if let (Some(g1), Some(g2)) = (g1_pubkey, g2_pubkey) {
        println!("Using provided G1 and G2 pubkeys");
        key_set.g1_pubkey = hex_to_bytes::<32>(g1)?;
        key_set.g2_pubkey = hex_to_bytes::<64>(g2)?;
    }

    // Save to storage
    storage
        .operators
        .insert(operator_str.clone(), key_set.clone());
    save_keys_to_file(&storage, keys_file)?;

    // Log the keys to console
    log_generated_keys(&key_set, operator);

    Ok(key_set)
}

/// Log generated keys to console
pub fn log_generated_keys(key_set: &BlsKeySet, operator: &Pubkey) {
    println!("\n=== Generated BLS Keys for Operator {} ===", operator);
    println!("BN128 Private Key: {}", hex::encode(key_set.private_key));
    println!("G1 Public Key: {}", hex::encode(key_set.g1_pubkey));
    println!("G2 Public Key: {}", hex::encode(key_set.g2_pubkey));
    println!("Created At: {}", key_set.created_at);
    println!("==========================================\n");
}

/// Parse hex string to byte array of specified length
pub fn hex_to_bytes<const N: usize>(hex_str: &str) -> Result<[u8; N]> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| anyhow!("Error parsing hex string '{}': {}", hex_str, e))?;

    if bytes.len() != N {
        return Err(anyhow!("Expected {} bytes, got {}", N, bytes.len()));
    }

    let mut array = [0u8; N];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Result of signature aggregation
#[derive(Debug)]
pub struct AggregationResult {
    pub aggregated_signature: [u8; 32],
    pub aggregated_g2: [u8; 64],
    pub signers_bitmap: Vec<u8>,
}

/// Aggregate multiple BLS signatures and public keys
pub fn aggregate_signatures_and_keys(
    signatures: &str,
    g1_public_keys: &str,
    g2_public_keys: &str,
    signers_bitmap: &str,
) -> Result<AggregationResult> {
    // Parse comma-separated hex strings
    let signature_list: Vec<&str> = signatures.split(',').collect();
    let g1_list: Vec<&str> = g1_public_keys.split(',').collect();
    let g2_list: Vec<&str> = g2_public_keys.split(',').collect();

    if signature_list.len() != g1_list.len() || signature_list.len() != g2_list.len() {
        return Err(anyhow!(
            "Number of signatures, G1 keys, and G2 keys must match"
        ));
    }

    // Parse bitmap
    let bitmap_bytes =
        hex::decode(signers_bitmap).map_err(|e| anyhow!("Error parsing signers bitmap: {}", e))?;

    // Parse signatures
    let mut signatures_vec = Vec::new();
    for sig_hex in signature_list {
        let sig_bytes = hex::decode(sig_hex.trim())
            .map_err(|e| anyhow!("Error parsing signature '{}': {}", sig_hex, e))?;
        if sig_bytes.len() != 64 {
            return Err(anyhow!(
                "Signature must be 64 bytes, got {} for '{}'",
                sig_bytes.len(),
                sig_hex
            ));
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&sig_bytes);
        signatures_vec.push(sig_array);
    }

    // Parse G1 public keys
    let mut g1_keys_vec = Vec::new();
    for g1_hex in g1_list {
        let g1_bytes = hex::decode(g1_hex.trim())
            .map_err(|e| anyhow!("Error parsing G1 key '{}': {}", g1_hex, e))?;
        if g1_bytes.len() != 32 {
            return Err(anyhow!(
                "G1 key must be 32 bytes, got {} for '{}'",
                g1_bytes.len(),
                g1_hex
            ));
        }
        let mut g1_array = [0u8; 32];
        g1_array.copy_from_slice(&g1_bytes);
        g1_keys_vec.push(g1_array);
    }

    // Parse G2 public keys
    let mut g2_keys_vec = Vec::new();
    for g2_hex in g2_list {
        let g2_bytes = hex::decode(g2_hex.trim())
            .map_err(|e| anyhow!("Error parsing G2 key '{}': {}", g2_hex, e))?;
        if g2_bytes.len() != 64 {
            return Err(anyhow!(
                "G2 key must be 64 bytes, got {} for '{}'",
                g2_bytes.len(),
                g2_hex
            ));
        }
        let mut g2_array = [0u8; 64];
        g2_array.copy_from_slice(&g2_bytes);
        g2_keys_vec.push(g2_array);
    }

    // Aggregate signatures (simple addition for now - in production you'd want proper BLS aggregation)
    let mut aggregated_signature = [0u8; 32];
    for sig in signatures_vec {
        for i in 0..32 {
            aggregated_signature[i] = aggregated_signature[i].wrapping_add(sig[i]);
        }
    }

    // Aggregate G2 public keys (simple addition for now - in production you'd want proper BLS aggregation)
    let mut aggregated_g2 = [0u8; 64];
    for g2_key in g2_keys_vec {
        for i in 0..64 {
            aggregated_g2[i] = aggregated_g2[i].wrapping_add(g2_key[i]);
        }
    }

    Ok(AggregationResult {
        aggregated_signature,
        aggregated_g2,
        signers_bitmap: bitmap_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_bls_keypair() {
        let operator = Pubkey::new_unique();
        let key_set = generate_bls_keypair(&operator).unwrap();

        // Verify operator matches
        assert_eq!(key_set.operator, operator.to_string());

        // Verify keys are not all zeros
        assert_ne!(key_set.private_key, [0u8; 32]);
        assert_ne!(key_set.g1_pubkey, [0u8; 32]);
        assert_ne!(key_set.g2_pubkey, [0u8; 64]);

        // Verify we can generate signature
        let signature = generate_signature(&key_set).unwrap();
        assert_ne!(signature, [0u8; 64]);
    }

    #[test]
    fn test_save_and_load_keys() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        let operator1 = Pubkey::new_unique();
        let operator2 = Pubkey::new_unique();

        // Generate keys for two operators
        let keys1 = generate_bls_keypair(&operator1).unwrap();
        let keys2 = generate_bls_keypair(&operator2).unwrap();

        // Create storage and save
        let mut storage = BlsKeyStorage::new();
        storage
            .operators
            .insert(operator1.to_string(), keys1.clone());
        storage
            .operators
            .insert(operator2.to_string(), keys2.clone());

        save_keys_to_file(&storage, file_path).unwrap();

        // Load and verify
        let loaded_storage = load_keys_from_file(file_path).unwrap();
        assert_eq!(loaded_storage.operators.len(), 2);

        let loaded_keys1 = &loaded_storage.operators[&operator1.to_string()];
        assert_eq!(loaded_keys1.private_key, keys1.private_key);
        assert_eq!(loaded_keys1.g1_pubkey, keys1.g1_pubkey);
        assert_eq!(loaded_keys1.g2_pubkey, keys1.g2_pubkey);
    }

    #[test]
    fn test_hex_to_bytes() {
        let hex = "216f05b464d2cab272954c660dd45cf8ab0b2613654dccc74c1155febaafb5c9";
        let bytes: [u8; 32] = hex_to_bytes(hex).unwrap();

        assert_eq!(bytes[0], 0x21);
        assert_eq!(bytes[1], 0x6f);
        assert_eq!(bytes[31], 0xc9);

        // Test wrong length
        assert!(hex_to_bytes::<16>(hex).is_err());
    }
}
