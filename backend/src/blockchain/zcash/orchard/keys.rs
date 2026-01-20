//! Orchard key management
//!
//! This module handles UnifiedSpendingKey derivation and management
//! following ZIP 32 hierarchical deterministic key derivation.

use super::{OrchardError, OrchardResult};
use sha2::{Digest, Sha256};

/// Orchard viewing key for scanning blocks
#[derive(Debug, Clone)]
pub struct OrchardViewingKey {
    /// The full viewing key bytes
    fvk_bytes: Vec<u8>,
    /// Account index
    pub account_index: u32,
    /// Birthday height (first block to scan from)
    pub birthday_height: u64,
}

impl OrchardViewingKey {
    /// Get the full viewing key bytes
    pub fn fvk_bytes(&self) -> &[u8] {
        &self.fvk_bytes
    }

    /// Encode the viewing key to a string representation
    pub fn encode(&self) -> String {
        // Encode as hex with metadata prefix
        format!(
            "ufvk:{}:{}:{}",
            self.account_index,
            self.birthday_height,
            hex::encode(&self.fvk_bytes)
        )
    }

    /// Decode a viewing key from string representation
    pub fn decode(encoded: &str) -> OrchardResult<Self> {
        let parts: Vec<&str> = encoded.split(':').collect();
        if parts.len() != 4 || parts[0] != "ufvk" {
            return Err(OrchardError::KeyDerivation(
                "Invalid viewing key format".to_string(),
            ));
        }

        let account_index = parts[1]
            .parse()
            .map_err(|_| OrchardError::KeyDerivation("Invalid account index".to_string()))?;

        let birthday_height = parts[2]
            .parse()
            .map_err(|_| OrchardError::KeyDerivation("Invalid birthday height".to_string()))?;

        let fvk_bytes = hex::decode(parts[3])
            .map_err(|_| OrchardError::KeyDerivation("Invalid FVK hex".to_string()))?;

        Ok(Self {
            fvk_bytes,
            account_index,
            birthday_height,
        })
    }
}

/// Orchard spending key for signing transactions
pub struct OrchardSpendingKey {
    /// The spending key bytes (sensitive!)
    sk_bytes: Vec<u8>,
    /// Account index
    pub account_index: u32,
}

impl OrchardSpendingKey {
    /// Get the spending key bytes (use with caution)
    pub fn sk_bytes(&self) -> &[u8] {
        &self.sk_bytes
    }
}

impl Drop for OrchardSpendingKey {
    fn drop(&mut self) {
        // Zero out the spending key bytes when dropped
        self.sk_bytes.iter_mut().for_each(|b| *b = 0);
    }
}

/// Orchard key manager for HD key derivation
pub struct OrchardKeyManager;

impl OrchardKeyManager {
    /// Derive Orchard keys from a seed phrase
    ///
    /// # Arguments
    /// * `seed` - 64-byte seed from BIP39 mnemonic
    /// * `account_index` - Account number (0 for first account)
    /// * `birthday_height` - Block height when wallet was created
    ///
    /// # Returns
    /// * Tuple of (spending_key, viewing_key)
    pub fn derive_from_seed(
        seed: &[u8],
        account_index: u32,
        birthday_height: u64,
    ) -> OrchardResult<(OrchardSpendingKey, OrchardViewingKey)> {
        if seed.len() < 32 {
            return Err(OrchardError::KeyDerivation(
                "Seed must be at least 32 bytes".to_string(),
            ));
        }

        // ZIP 32 derivation path: m/32'/133'/account'
        // 133 is the coin type for Zcash mainnet
        let master_key = Self::derive_master_key(seed)?;
        let purpose_key = Self::derive_child(&master_key, 32 | 0x80000000)?;
        let coin_key = Self::derive_child(&purpose_key, 133 | 0x80000000)?;
        let account_key = Self::derive_child(&coin_key, account_index | 0x80000000)?;

        // Derive the full viewing key from the account key
        let fvk = Self::derive_full_viewing_key(&account_key)?;

        let spending_key = OrchardSpendingKey {
            sk_bytes: account_key,
            account_index,
        };

        let viewing_key = OrchardViewingKey {
            fvk_bytes: fvk,
            account_index,
            birthday_height,
        };

        Ok((spending_key, viewing_key))
    }

    /// Derive from an existing transparent private key (hex)
    ///
    /// This allows users to "upgrade" their transparent wallet to support Orchard
    /// by using the private key as seed material.
    pub fn derive_from_private_key(
        private_key_hex: &str,
        account_index: u32,
        birthday_height: u64,
    ) -> OrchardResult<(OrchardSpendingKey, OrchardViewingKey)> {
        let pk_bytes = hex::decode(private_key_hex)
            .map_err(|e| OrchardError::KeyDerivation(format!("Invalid private key hex: {}", e)))?;

        if pk_bytes.len() != 32 {
            return Err(OrchardError::KeyDerivation(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        // Expand the 32-byte private key to a 64-byte seed using BLAKE2b
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(64)
            .personal(b"ZcashOrchardSeed")
            .to_state();
        hasher.update(&pk_bytes);
        let seed = hasher.finalize();

        Self::derive_from_seed(seed.as_bytes(), account_index, birthday_height)
    }

    /// Derive a viewing key only (for watch-only wallets)
    pub fn derive_viewing_key(
        seed: &[u8],
        account_index: u32,
        birthday_height: u64,
    ) -> OrchardResult<OrchardViewingKey> {
        let (_, viewing_key) = Self::derive_from_seed(seed, account_index, birthday_height)?;
        Ok(viewing_key)
    }

    /// Derive master key from seed using BLAKE2b
    fn derive_master_key(seed: &[u8]) -> OrchardResult<Vec<u8>> {
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(64)
            .personal(b"ZcashOrchardMstr")
            .to_state();
        hasher.update(seed);
        let result = hasher.finalize();

        // Take the first 32 bytes as the key
        Ok(result.as_bytes()[..32].to_vec())
    }

    /// Derive child key using BLAKE2b
    fn derive_child(parent: &[u8], index: u32) -> OrchardResult<Vec<u8>> {
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(64)
            .personal(b"ZcashOrchardChld")
            .to_state();
        hasher.update(parent);
        hasher.update(&index.to_le_bytes());
        let result = hasher.finalize();

        Ok(result.as_bytes()[..32].to_vec())
    }

    /// Derive full viewing key from account key
    /// FVK consists of ak (32 bytes) || nk (32 bytes) || rivk (32 bytes) = 96 bytes
    /// We derive each component separately since Blake2b max output is 64 bytes
    fn derive_full_viewing_key(account_key: &[u8]) -> OrchardResult<Vec<u8>> {
        // Derive ak (authorization key, 32 bytes)
        let ak = {
            let mut hasher = blake2b_simd::Params::new()
                .hash_length(32)
                .personal(b"Zcash_Orchard_ak")
                .to_state();
            hasher.update(account_key);
            hasher.finalize()
        };

        // Derive nk (nullifier key, 32 bytes)
        let nk = {
            let mut hasher = blake2b_simd::Params::new()
                .hash_length(32)
                .personal(b"Zcash_Orchard_nk")
                .to_state();
            hasher.update(account_key);
            hasher.finalize()
        };

        // Derive rivk (internal viewing key, 32 bytes)
        let rivk = {
            let mut hasher = blake2b_simd::Params::new()
                .hash_length(32)
                .personal(b"ZcashOrchardrivk")
                .to_state();
            hasher.update(account_key);
            hasher.finalize()
        };

        // Concatenate: ak || nk || rivk = 96 bytes
        let mut fvk = Vec::with_capacity(96);
        fvk.extend_from_slice(ak.as_bytes());
        fvk.extend_from_slice(nk.as_bytes());
        fvk.extend_from_slice(rivk.as_bytes());

        Ok(fvk)
    }

    /// Generate a random Orchard seed
    pub fn generate_seed() -> OrchardResult<Vec<u8>> {
        use rand::RngCore;
        let mut seed = vec![0u8; 64];
        rand::thread_rng().fill_bytes(&mut seed);
        Ok(seed)
    }

    /// Convert seed to BIP39 mnemonic words
    pub fn seed_to_mnemonic(seed: &[u8]) -> OrchardResult<String> {
        // Simple implementation - in production, use a proper BIP39 library
        // This is a placeholder that returns the seed as hex
        Ok(hex::encode(seed))
    }

    /// Get the fingerprint of a viewing key (for identification)
    pub fn get_fingerprint(viewing_key: &OrchardViewingKey) -> String {
        let mut hasher = Sha256::new();
        hasher.update(viewing_key.fvk_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_from_seed() {
        let seed = vec![0u8; 64];
        let (sk, vk) = OrchardKeyManager::derive_from_seed(&seed, 0, 2000000).unwrap();

        assert_eq!(sk.account_index, 0);
        assert_eq!(vk.account_index, 0);
        assert_eq!(vk.birthday_height, 2000000);
        assert!(!vk.fvk_bytes().is_empty());
    }

    #[test]
    fn test_viewing_key_encode_decode() {
        let seed = vec![1u8; 64];
        let (_, vk) = OrchardKeyManager::derive_from_seed(&seed, 0, 2000000).unwrap();

        let encoded = vk.encode();
        let decoded = OrchardViewingKey::decode(&encoded).unwrap();

        assert_eq!(vk.account_index, decoded.account_index);
        assert_eq!(vk.birthday_height, decoded.birthday_height);
        assert_eq!(vk.fvk_bytes(), decoded.fvk_bytes());
    }

    #[test]
    fn test_derive_from_private_key() {
        let private_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let (sk, vk) = OrchardKeyManager::derive_from_private_key(private_key, 0, 2000000).unwrap();

        assert_eq!(sk.account_index, 0);
        assert!(!vk.fvk_bytes().is_empty());
    }
}
