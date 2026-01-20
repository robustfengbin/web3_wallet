use rand::RngCore;
use ripemd::Ripemd160;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Zcash mainnet transparent address prefix (t1)
const ZCASH_T_ADDR_PREFIX: [u8; 2] = [0x1C, 0xB8];

/// Generate a new Zcash transparent address and private key
/// Returns (address, private_key_hex)
pub fn generate_zcash_wallet() -> AppResult<(String, String)> {
    let secp = Secp256k1::new();

    // Generate random 32-byte private key
    let mut rng = rand::thread_rng();
    let mut key_bytes = [0u8; 32];
    rng.fill_bytes(&mut key_bytes);

    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|e| AppError::InternalError(format!("Failed to generate secret key: {}", e)))?;

    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Generate address from public key
    let address = public_key_to_t_address(&public_key)?;
    let private_key_hex = hex::encode(key_bytes);

    Ok((address, private_key_hex))
}

/// Import a Zcash wallet from private key
/// Returns the address derived from the private key
pub fn import_zcash_wallet(private_key_hex: &str) -> AppResult<String> {
    let secp = Secp256k1::new();

    // Parse private key
    let key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| AppError::ValidationError(format!("Invalid private key hex: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(AppError::ValidationError(
            "Private key must be 32 bytes".to_string(),
        ));
    }

    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|e| AppError::ValidationError(format!("Invalid private key: {}", e)))?;

    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Generate address from public key
    public_key_to_t_address(&public_key)
}

/// Convert a secp256k1 public key to a Zcash transparent address (t-address)
fn public_key_to_t_address(public_key: &PublicKey) -> AppResult<String> {
    // Get compressed public key bytes
    let pubkey_bytes = public_key.serialize();

    // SHA256 hash
    let sha256_hash = Sha256::digest(&pubkey_bytes);

    // RIPEMD160 hash
    let ripemd_hash = Ripemd160::digest(&sha256_hash);

    // Build payload: prefix + ripemd160 hash
    let mut payload = Vec::with_capacity(22);
    payload.extend_from_slice(&ZCASH_T_ADDR_PREFIX);
    payload.extend_from_slice(&ripemd_hash);

    // Double SHA256 for checksum
    let checksum1 = Sha256::digest(&payload);
    let checksum2 = Sha256::digest(&checksum1);

    // Take first 4 bytes of checksum
    let checksum = &checksum2[..4];

    // Build final address: payload + checksum
    let mut address_bytes = payload;
    address_bytes.extend_from_slice(checksum);

    // Base58 encode
    let address = bs58::encode(address_bytes).into_string();

    Ok(address)
}

/// Validate a Zcash address format
pub fn validate_zcash_address(address: &str) -> bool {
    if address.is_empty() {
        return false;
    }

    // Transparent addresses (t1 or t3)
    if address.starts_with("t1") || address.starts_with("t3") {
        // Try to decode and verify checksum
        if let Ok(decoded) = bs58::decode(address).into_vec() {
            if decoded.len() == 26 {
                // 2 byte prefix + 20 byte hash + 4 byte checksum
                let payload = &decoded[..22];
                let checksum = &decoded[22..];

                let checksum1 = Sha256::digest(payload);
                let checksum2 = Sha256::digest(&checksum1);

                return &checksum2[..4] == checksum;
            }
        }
        return false;
    }

    // Sapling shielded addresses (zs)
    if address.starts_with("zs") && address.len() >= 78 {
        return true; // Basic length check for Sapling addresses
    }

    // Sprout shielded addresses (zc) - legacy
    if address.starts_with("zc") && address.len() >= 95 {
        return true;
    }

    // Unified addresses (u1)
    if address.starts_with("u1") && address.len() >= 100 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_zcash_wallet() {
        let (address, private_key) = generate_zcash_wallet().unwrap();

        assert!(address.starts_with("t1"));
        assert_eq!(private_key.len(), 64); // 32 bytes = 64 hex chars
        assert!(validate_zcash_address(&address));
    }

    #[test]
    fn test_import_zcash_wallet() {
        // Generate a wallet first
        let (original_address, private_key) = generate_zcash_wallet().unwrap();

        // Import the same private key
        let imported_address = import_zcash_wallet(&private_key).unwrap();

        assert_eq!(original_address, imported_address);
    }

    #[test]
    fn test_validate_zcash_address() {
        // Valid t1 address format (example)
        assert!(!validate_zcash_address("")); // Empty
        assert!(!validate_zcash_address("invalid")); // Random string

        // Generate and validate
        let (address, _) = generate_zcash_wallet().unwrap();
        assert!(validate_zcash_address(&address));
    }
}
