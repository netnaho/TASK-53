use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// AES-256-GCM encryption service for sensitive fields at rest.
///
/// Encrypted values are stored as: base64(nonce || ciphertext)
/// The nonce is 12 bytes, prepended to the ciphertext.
#[derive(Clone)]
pub struct EncryptionService {
    key: [u8; 32],
}

impl EncryptionService {
    /// Derive a 256-bit key from the provided key material using SHA-256.
    pub fn new(key_material: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key_material.as_bytes());
        let key: [u8; 32] = hasher.finalize().into();
        Self { key }
    }

    /// Encrypt plaintext and return a base64-encoded string (nonce + ciphertext).
    pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| EncryptionError::KeyError)?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| EncryptionError::EncryptFailed)?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// Decrypt a base64-encoded string (nonce + ciphertext) back to plaintext.
    pub fn decrypt(&self, encrypted: &str) -> Result<String, EncryptionError> {
        let combined = BASE64
            .decode(encrypted)
            .map_err(|_| EncryptionError::DecodeFailed)?;

        if combined.len() < 13 {
            return Err(EncryptionError::DecodeFailed);
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| EncryptionError::KeyError)?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| EncryptionError::DecryptFailed)?;

        String::from_utf8(plaintext).map_err(|_| EncryptionError::DecryptFailed)
    }

    /// Mask a value for safe display (e.g., in logs or API responses).
    /// Shows first 2 and last 2 characters, masks the rest.
    pub fn mask(value: &str) -> String {
        if value.len() <= 4 {
            return "****".to_string();
        }
        let first = &value[..2];
        let last = &value[value.len() - 2..];
        format!("{}{}{}",first, "*".repeat(value.len() - 4), last)
    }
}

impl std::fmt::Debug for EncryptionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionService")
            .field("key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Invalid encryption key")]
    KeyError,
    #[error("Encryption failed")]
    EncryptFailed,
    #[error("Decryption failed")]
    DecryptFailed,
    #[error("Base64 decode failed")]
    DecodeFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let svc = EncryptionService::new("test-key-material");
        let plaintext = "SSN-123-45-6789";
        let encrypted = svc.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);
        let decrypted = svc.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces_produce_different_ciphertexts() {
        let svc = EncryptionService::new("test-key");
        let plaintext = "sensitive-data";
        let enc1 = svc.encrypt(plaintext).unwrap();
        let enc2 = svc.encrypt(plaintext).unwrap();
        assert_ne!(enc1, enc2);
        assert_eq!(svc.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(svc.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn test_wrong_key_fails_decrypt() {
        let svc1 = EncryptionService::new("key-one");
        let svc2 = EncryptionService::new("key-two");
        let encrypted = svc1.encrypt("secret").unwrap();
        assert!(svc2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_mask_values() {
        assert_eq!(EncryptionService::mask("123456789"), "12*****89"); // 2 + 5 stars + 2
        assert_eq!(EncryptionService::mask("ab"), "****");             // len ≤ 4 → "****"
        assert_eq!(EncryptionService::mask("abcde"), "ab*de");        // 2 + 1 star + 2 (len=5, stars=5-4=1)
    }
}
