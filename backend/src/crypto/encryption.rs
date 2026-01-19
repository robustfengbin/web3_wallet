use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;

use crate::error::{AppError, AppResult};

const NONCE_SIZE: usize = 12;

/// Encrypts data using AES-256-GCM
/// Returns base64 encoded string with nonce prepended
pub fn encrypt(data: &str, key: &str) -> AppResult<String> {
    if key.len() != 32 {
        return Err(AppError::EncryptionError(
            "Encryption key must be 32 bytes".to_string(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| AppError::EncryptionError(format!("Failed to create cipher: {}", e)))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the data
    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| AppError::EncryptionError(format!("Encryption failed: {}", e)))?;

    // Prepend nonce to ciphertext and encode as base64
    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);

    Ok(STANDARD.encode(result))
}

/// Decrypts AES-256-GCM encrypted data
/// Expects base64 encoded string with nonce prepended
pub fn decrypt(encrypted_data: &str, key: &str) -> AppResult<String> {
    if key.len() != 32 {
        return Err(AppError::EncryptionError(
            "Encryption key must be 32 bytes".to_string(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| AppError::EncryptionError(format!("Failed to create cipher: {}", e)))?;

    // Decode from base64
    let data = STANDARD
        .decode(encrypted_data)
        .map_err(|e| AppError::EncryptionError(format!("Base64 decode failed: {}", e)))?;

    if data.len() < NONCE_SIZE {
        return Err(AppError::EncryptionError(
            "Invalid encrypted data: too short".to_string(),
        ));
    }

    // Extract nonce and ciphertext
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt the data
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::EncryptionError(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::EncryptionError(format!("UTF-8 decode failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = "32-byte-encryption-key-here!!!!";
        let data = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let encrypted = encrypt(data, key).unwrap();
        let decrypted = decrypt(&encrypted, key).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_different_encryptions() {
        let key = "32-byte-encryption-key-here!!!!";
        let data = "test data";

        let encrypted1 = encrypt(data, key).unwrap();
        let encrypted2 = encrypt(data, key).unwrap();

        // Each encryption should produce different output due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        assert_eq!(decrypt(&encrypted1, key).unwrap(), data);
        assert_eq!(decrypt(&encrypted2, key).unwrap(), data);
    }

    #[test]
    fn test_invalid_key_length() {
        let key = "short-key";
        let data = "test data";

        assert!(encrypt(data, key).is_err());
    }
}
