//! Unified Address generation and management
//!
//! Unified addresses (u1...) can contain receivers for multiple pools:
//! - Orchard (newest, Halo 2)
//! - Sapling (Groth16)
//! - Transparent (P2PKH)

use super::{keys::OrchardViewingKey, OrchardError, OrchardResult};
use serde::{Deserialize, Serialize};

/// Information about a unified address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAddressInfo {
    /// The unified address string (u1...)
    pub address: String,

    /// Whether it contains an Orchard receiver
    pub has_orchard: bool,

    /// Whether it contains a Sapling receiver
    pub has_sapling: bool,

    /// Whether it contains a transparent receiver
    pub has_transparent: bool,

    /// The transparent address component (if present)
    pub transparent_address: Option<String>,

    /// Address index in the HD derivation path
    pub address_index: u32,

    /// The account this address belongs to
    pub account_index: u32,
}

/// Receiver type in a unified address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiverType {
    Orchard,
    Sapling,
    Transparent,
}

/// Manager for Orchard/Unified address operations
pub struct OrchardAddressManager {
    /// The viewing key used for address derivation
    viewing_key: OrchardViewingKey,
    /// Next address index to use
    next_index: u32,
}

impl OrchardAddressManager {
    /// Create a new address manager from a viewing key
    pub fn new(viewing_key: OrchardViewingKey) -> Self {
        Self {
            viewing_key,
            next_index: 0,
        }
    }

    /// Generate a new unified address with all receivers
    ///
    /// The address will contain Orchard, Sapling, and transparent receivers
    /// for maximum compatibility.
    pub fn generate_unified_address(&mut self) -> OrchardResult<UnifiedAddressInfo> {
        let index = self.next_index;
        self.next_index += 1;

        self.generate_address_at_index(index)
    }

    /// Generate a unified address at a specific index
    pub fn generate_address_at_index(&self, index: u32) -> OrchardResult<UnifiedAddressInfo> {
        // Derive diversifier from index
        let diversifier = self.derive_diversifier(index)?;

        // Generate receivers for each pool
        let orchard_receiver = self.derive_orchard_receiver(&diversifier)?;
        let sapling_receiver = self.derive_sapling_receiver(&diversifier)?;
        let transparent_address = self.derive_transparent_address(index)?;

        // Encode as unified address (F4Jumble encoding)
        let unified_address =
            self.encode_unified_address(&orchard_receiver, &sapling_receiver, &transparent_address)?;

        Ok(UnifiedAddressInfo {
            address: unified_address,
            has_orchard: true,
            has_sapling: true,
            has_transparent: true,
            transparent_address: Some(transparent_address),
            address_index: index,
            account_index: self.viewing_key.account_index,
        })
    }

    /// Generate an Orchard-only unified address (maximum privacy)
    pub fn generate_orchard_only_address(&self, index: u32) -> OrchardResult<UnifiedAddressInfo> {
        let diversifier = self.derive_diversifier(index)?;
        let orchard_receiver = self.derive_orchard_receiver(&diversifier)?;

        // Encode with only Orchard receiver
        let unified_address = self.encode_unified_address_single(&orchard_receiver, ReceiverType::Orchard)?;

        Ok(UnifiedAddressInfo {
            address: unified_address,
            has_orchard: true,
            has_sapling: false,
            has_transparent: false,
            transparent_address: None,
            address_index: index,
            account_index: self.viewing_key.account_index,
        })
    }

    /// Parse a unified address to extract its components
    pub fn parse_unified_address(address: &str) -> OrchardResult<UnifiedAddressInfo> {
        if !address.starts_with("u1") {
            return Err(OrchardError::InvalidUnifiedAddress(
                "Unified address must start with 'u1'".to_string(),
            ));
        }

        // Decode and parse the unified address
        // In a real implementation, this would use F4Jumble decoding
        let decoded = Self::decode_unified_address(address)?;

        let has_orchard = decoded.iter().any(|(t, _)| *t == ReceiverType::Orchard);
        let has_sapling = decoded.iter().any(|(t, _)| *t == ReceiverType::Sapling);
        let has_transparent = decoded.iter().any(|(t, _)| *t == ReceiverType::Transparent);

        let transparent_address = decoded
            .iter()
            .find(|(t, _)| *t == ReceiverType::Transparent)
            .map(|(_, data)| Self::encode_transparent_address(data));

        Ok(UnifiedAddressInfo {
            address: address.to_string(),
            has_orchard,
            has_sapling,
            has_transparent,
            transparent_address,
            address_index: 0, // Unknown when parsing external address
            account_index: 0,
        })
    }

    /// Validate a unified address
    pub fn validate_unified_address(address: &str) -> bool {
        if !address.starts_with("u1") {
            return false;
        }

        if address.len() < 100 {
            return false;
        }

        // Try to decode - if successful, it's valid
        Self::decode_unified_address(address).is_ok()
    }

    /// Derive diversifier from index
    fn derive_diversifier(&self, index: u32) -> OrchardResult<[u8; 11]> {
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(11)
            .personal(b"ZcashOrchardDiv")
            .to_state();
        hasher.update(self.viewing_key.fvk_bytes());
        hasher.update(&index.to_le_bytes());
        let result = hasher.finalize();

        let mut diversifier = [0u8; 11];
        diversifier.copy_from_slice(result.as_bytes());
        Ok(diversifier)
    }

    /// Derive Orchard receiver from diversifier
    fn derive_orchard_receiver(&self, diversifier: &[u8; 11]) -> OrchardResult<Vec<u8>> {
        // In real implementation, this would:
        // 1. Convert diversifier to point on Pallas curve
        // 2. Derive receiver using the viewing key
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(43) // Orchard receiver is 43 bytes
            .personal(b"ZcashOrchardRcv")
            .to_state();
        hasher.update(self.viewing_key.fvk_bytes());
        hasher.update(diversifier);
        let result = hasher.finalize();

        Ok(result.as_bytes().to_vec())
    }

    /// Derive Sapling receiver from diversifier
    fn derive_sapling_receiver(&self, diversifier: &[u8; 11]) -> OrchardResult<Vec<u8>> {
        // Sapling receiver is 43 bytes
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(43)
            .personal(b"ZcashSaplingRcv")
            .to_state();
        hasher.update(self.viewing_key.fvk_bytes());
        hasher.update(diversifier);
        let result = hasher.finalize();

        Ok(result.as_bytes().to_vec())
    }

    /// Derive transparent address from index
    fn derive_transparent_address(&self, index: u32) -> OrchardResult<String> {
        use ripemd::Ripemd160;
        use sha2::{Digest, Sha256};

        // Derive public key hash for transparent address
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(33) // Compressed public key size
            .personal(b"ZcashOrchardTPK")
            .to_state();
        hasher.update(self.viewing_key.fvk_bytes());
        hasher.update(&index.to_le_bytes());
        let pubkey = hasher.finalize();

        // Hash160 (SHA256 + RIPEMD160)
        let sha256_hash = Sha256::digest(pubkey.as_bytes());
        let hash160 = Ripemd160::digest(&sha256_hash);

        // Encode as t-address
        let mut payload = vec![0x1C, 0xB8]; // Zcash mainnet t1 prefix
        payload.extend_from_slice(&hash160);

        // Add checksum
        let checksum = Sha256::digest(&Sha256::digest(&payload));
        payload.extend_from_slice(&checksum[..4]);

        Ok(bs58::encode(payload).into_string())
    }

    /// Encode unified address from receivers
    fn encode_unified_address(
        &self,
        orchard_receiver: &[u8],
        sapling_receiver: &[u8],
        transparent_address: &str,
    ) -> OrchardResult<String> {
        // Build the unified address payload
        // Format: typecode || length || data for each receiver
        let mut payload = Vec::new();

        // Add Orchard receiver (typecode 0x03)
        payload.push(0x03);
        payload.push(orchard_receiver.len() as u8);
        payload.extend_from_slice(orchard_receiver);

        // Add Sapling receiver (typecode 0x02)
        payload.push(0x02);
        payload.push(sapling_receiver.len() as u8);
        payload.extend_from_slice(sapling_receiver);

        // Add transparent receiver (typecode 0x00 for P2PKH)
        // Decode the t-address to get the pubkey hash
        if let Ok(decoded) = bs58::decode(transparent_address).into_vec() {
            if decoded.len() >= 22 {
                let pubkey_hash = &decoded[2..22];
                payload.push(0x00);
                payload.push(20);
                payload.extend_from_slice(pubkey_hash);
            }
        }

        // Apply F4Jumble encoding and Bech32m
        let jumbled = self.f4_jumble(&payload)?;
        let address = self.bech32m_encode("u", &jumbled)?;

        Ok(address)
    }

    /// Encode unified address with single receiver
    fn encode_unified_address_single(
        &self,
        receiver: &[u8],
        receiver_type: ReceiverType,
    ) -> OrchardResult<String> {
        let mut payload = Vec::new();

        let typecode = match receiver_type {
            ReceiverType::Orchard => 0x03,
            ReceiverType::Sapling => 0x02,
            ReceiverType::Transparent => 0x00,
        };

        payload.push(typecode);
        payload.push(receiver.len() as u8);
        payload.extend_from_slice(receiver);

        let jumbled = self.f4_jumble(&payload)?;
        let address = self.bech32m_encode("u", &jumbled)?;

        Ok(address)
    }

    /// Decode unified address to receivers
    fn decode_unified_address(address: &str) -> OrchardResult<Vec<(ReceiverType, Vec<u8>)>> {
        if !address.starts_with("u1") {
            return Err(OrchardError::InvalidUnifiedAddress(
                "Invalid prefix".to_string(),
            ));
        }

        // Bech32m decode
        let data = Self::bech32m_decode(address)?;

        // F4Jumble decode
        let payload = Self::f4_jumble_inv(&data)?;

        // Parse receivers
        let mut receivers = Vec::new();
        let mut pos = 0;

        while pos < payload.len() {
            if pos + 2 > payload.len() {
                break;
            }

            let typecode = payload[pos];
            let length = payload[pos + 1] as usize;
            pos += 2;

            if pos + length > payload.len() {
                break;
            }

            let data = payload[pos..pos + length].to_vec();
            pos += length;

            let receiver_type = match typecode {
                0x00 | 0x01 => ReceiverType::Transparent,
                0x02 => ReceiverType::Sapling,
                0x03 => ReceiverType::Orchard,
                _ => continue, // Unknown receiver type
            };

            receivers.push((receiver_type, data));
        }

        if receivers.is_empty() {
            return Err(OrchardError::InvalidUnifiedAddress(
                "No valid receivers found".to_string(),
            ));
        }

        Ok(receivers)
    }

    /// Encode transparent address from pubkey hash
    fn encode_transparent_address(pubkey_hash: &[u8]) -> String {
        use sha2::{Digest, Sha256};

        let mut payload = vec![0x1C, 0xB8];
        payload.extend_from_slice(pubkey_hash);

        let checksum = Sha256::digest(&Sha256::digest(&payload));
        payload.extend_from_slice(&checksum[..4]);

        bs58::encode(payload).into_string()
    }

    /// F4Jumble encoding (simplified)
    fn f4_jumble(&self, input: &[u8]) -> OrchardResult<Vec<u8>> {
        // Simplified F4Jumble - in production, use proper implementation
        // F4Jumble is a length-preserving pseudorandom permutation
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(input.len().max(48))
            .personal(b"ZcashF4Jumble__")
            .to_state();
        hasher.update(input);
        hasher.update(&[0x00]); // Direction flag
        let hash = hasher.finalize();

        // XOR the input with the hash
        let mut result = input.to_vec();
        for (i, byte) in result.iter_mut().enumerate() {
            *byte ^= hash.as_bytes()[i % hash.as_bytes().len()];
        }

        Ok(result)
    }

    /// F4Jumble inverse
    fn f4_jumble_inv(input: &[u8]) -> OrchardResult<Vec<u8>> {
        // F4Jumble is its own inverse with the direction flag
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(input.len().max(48))
            .personal(b"ZcashF4Jumble__")
            .to_state();

        // For inverse, we need to compute the same hash from the result
        // This is a simplified version - real implementation is more complex
        let mut result = input.to_vec();

        // XOR with hash to get original
        hasher.update(&result);
        hasher.update(&[0x01]); // Inverse direction flag
        let hash = hasher.finalize();

        for (i, byte) in result.iter_mut().enumerate() {
            *byte ^= hash.as_bytes()[i % hash.as_bytes().len()];
        }

        Ok(result)
    }

    /// Bech32m encode
    fn bech32m_encode(&self, hrp: &str, data: &[u8]) -> OrchardResult<String> {
        // Convert to 5-bit groups
        let mut bits = Vec::new();
        for byte in data {
            bits.push((byte >> 3) & 0x1f);
            bits.push((byte << 2) & 0x1f);
        }

        // Remove trailing bits if necessary
        while bits.len() > data.len() * 8 / 5 {
            bits.pop();
        }

        // Bech32m character set
        const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

        // Compute checksum (simplified - real implementation needs polymod)
        let checksum = self.bech32m_checksum(hrp, &bits);

        // Build the address
        let mut result = String::from(hrp);
        result.push('1'); // Separator

        for &b in &bits {
            result.push(CHARSET[b as usize] as char);
        }

        for &b in &checksum {
            result.push(CHARSET[b as usize] as char);
        }

        Ok(result)
    }

    /// Bech32m decode
    fn bech32m_decode(address: &str) -> OrchardResult<Vec<u8>> {
        const CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

        // Find separator
        let sep_pos = address
            .rfind('1')
            .ok_or_else(|| OrchardError::InvalidUnifiedAddress("No separator".to_string()))?;

        let data_part = &address[sep_pos + 1..];

        // Remove checksum (last 6 characters)
        if data_part.len() < 6 {
            return Err(OrchardError::InvalidUnifiedAddress(
                "Too short".to_string(),
            ));
        }
        let data_without_checksum = &data_part[..data_part.len() - 6];

        // Convert from Bech32m characters to 5-bit values
        let mut bits = Vec::new();
        for c in data_without_checksum.chars() {
            let idx = CHARSET
                .find(c.to_ascii_lowercase())
                .ok_or_else(|| OrchardError::InvalidUnifiedAddress("Invalid character".to_string()))?;
            bits.push(idx as u8);
        }

        // Convert 5-bit groups to 8-bit bytes
        let mut result = Vec::new();
        let mut acc = 0u16;
        let mut acc_bits = 0;

        for &b in &bits {
            acc = (acc << 5) | (b as u16);
            acc_bits += 5;

            if acc_bits >= 8 {
                acc_bits -= 8;
                result.push((acc >> acc_bits) as u8);
            }
        }

        Ok(result)
    }

    /// Compute Bech32m checksum (simplified)
    fn bech32m_checksum(&self, hrp: &str, data: &[u8]) -> [u8; 6] {
        // Simplified checksum - real implementation uses polymod
        let mut hasher = blake2b_simd::Params::new()
            .hash_length(6)
            .personal(b"Bech32mChecksum")
            .to_state();
        hasher.update(hrp.as_bytes());
        hasher.update(data);
        let result = hasher.finalize();

        let mut checksum = [0u8; 6];
        for (i, byte) in result.as_bytes()[..6].iter().enumerate() {
            checksum[i] = byte & 0x1f;
        }
        checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::zcash::orchard::keys::OrchardKeyManager;

    #[test]
    fn test_generate_unified_address() {
        let seed = vec![0u8; 64];
        let (_, vk) = OrchardKeyManager::derive_from_seed(&seed, 0, 2000000).unwrap();

        let mut manager = OrchardAddressManager::new(vk);
        let addr_info = manager.generate_unified_address().unwrap();

        assert!(addr_info.address.starts_with("u1"));
        assert!(addr_info.has_orchard);
        assert!(addr_info.has_sapling);
        assert!(addr_info.has_transparent);
        assert!(addr_info.transparent_address.is_some());
    }

    #[test]
    fn test_generate_multiple_addresses() {
        let seed = vec![1u8; 64];
        let (_, vk) = OrchardKeyManager::derive_from_seed(&seed, 0, 2000000).unwrap();

        let mut manager = OrchardAddressManager::new(vk);

        let addr1 = manager.generate_unified_address().unwrap();
        let addr2 = manager.generate_unified_address().unwrap();

        assert_ne!(addr1.address, addr2.address);
        assert_eq!(addr1.address_index, 0);
        assert_eq!(addr2.address_index, 1);
    }

    #[test]
    fn test_orchard_only_address() {
        let seed = vec![2u8; 64];
        let (_, vk) = OrchardKeyManager::derive_from_seed(&seed, 0, 2000000).unwrap();

        let manager = OrchardAddressManager::new(vk);
        let addr_info = manager.generate_orchard_only_address(0).unwrap();

        assert!(addr_info.address.starts_with("u1"));
        assert!(addr_info.has_orchard);
        assert!(!addr_info.has_sapling);
        assert!(!addr_info.has_transparent);
    }
}
