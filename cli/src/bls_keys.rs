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
