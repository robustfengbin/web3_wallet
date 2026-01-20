use ethers::signers::{LocalWallet, Signer};
use rand::thread_rng;

use crate::error::{AppError, AppResult};

/// Generate a new Ethereum wallet
/// Returns (address, private_key_hex)
pub fn generate_ethereum_wallet() -> AppResult<(String, String)> {
    let wallet = LocalWallet::new(&mut thread_rng());
    let address = format!("{:?}", wallet.address());
    let private_key = hex::encode(wallet.signer().to_bytes());
    Ok((address, private_key))
}

/// Import an Ethereum wallet from private key
/// Returns the address derived from the private key
pub fn import_ethereum_wallet(private_key_hex: &str) -> AppResult<String> {
    let key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);

    let wallet: LocalWallet = key_hex
        .parse()
        .map_err(|e| AppError::ValidationError(format!("Invalid private key: {}", e)))?;

    Ok(format!("{:?}", wallet.address()))
}

/// Validate an Ethereum address format
#[allow(dead_code)]
pub(crate) fn validate_ethereum_address(address: &str) -> bool {
    if address.is_empty() {
        return false;
    }

    // Must start with 0x
    if !address.starts_with("0x") && !address.starts_with("0X") {
        return false;
    }

    // Must be 42 characters (0x + 40 hex chars)
    if address.len() != 42 {
        return false;
    }

    // All characters after 0x must be valid hex
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ethereum_wallet() {
        let (address, private_key) = generate_ethereum_wallet().unwrap();

        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
        assert_eq!(private_key.len(), 64); // 32 bytes = 64 hex chars
        assert!(validate_ethereum_address(&address));
    }

    #[test]
    fn test_import_ethereum_wallet() {
        // Generate a wallet first
        let (original_address, private_key) = generate_ethereum_wallet().unwrap();

        // Import the same private key
        let imported_address = import_ethereum_wallet(&private_key).unwrap();

        assert_eq!(original_address.to_lowercase(), imported_address.to_lowercase());
    }

    #[test]
    fn test_import_ethereum_wallet_with_0x_prefix() {
        let (original_address, private_key) = generate_ethereum_wallet().unwrap();

        // Import with 0x prefix
        let prefixed_key = format!("0x{}", private_key);
        let imported_address = import_ethereum_wallet(&prefixed_key).unwrap();

        assert_eq!(original_address.to_lowercase(), imported_address.to_lowercase());
    }

    #[test]
    fn test_validate_ethereum_address() {
        // Valid addresses
        assert!(validate_ethereum_address("0x742d35Cc6634C0532925a3b844Bc9e7595f1bEaB"));
        assert!(validate_ethereum_address("0x0000000000000000000000000000000000000000"));

        // Invalid addresses
        assert!(!validate_ethereum_address("")); // Empty
        assert!(!validate_ethereum_address("invalid")); // No 0x prefix
        assert!(!validate_ethereum_address("0x")); // Too short
        assert!(!validate_ethereum_address("0x742d35Cc6634C0532925a3b844Bc9e7595f1bEa")); // 41 chars
        assert!(!validate_ethereum_address("0x742d35Cc6634C0532925a3b844Bc9e7595f1bEaB1")); // 43 chars
        assert!(!validate_ethereum_address("0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG")); // Invalid hex
    }
}
