/// Cryptographic operations for secret encryption/decryption
/// Uses AES-256-GCM for authenticated encryption
/// Implements PBKDF2 and Argon2id for key derivation

use crate::errors::{StateStoreError, StateStoreResult};
use crate::secrets::{EncryptedSecretData, EncryptionContext, KeyDerivationParams};

/// Constants for cryptographic operations
pub mod constants {
    /// AES-256-GCM requires 32-byte (256-bit) keys
    pub const KEY_SIZE: usize = 32;

    /// GCM nonce size - 96 bits (12 bytes) is standard for GCM
    pub const NONCE_SIZE: usize = 12;

    /// GCM authentication tag size - 128 bits (16 bytes)
    pub const TAG_SIZE: usize = 16;

    /// Current encryption scheme version
    pub const ENCRYPTION_VERSION: u8 = 1;

    /// PBKDF2 default iteration count (NIST recommends >= 100,000 for 2024)
    pub const PBKDF2_ITERATIONS: u32 = 480_000;

    /// Argon2id default memory cost in KiB
    pub const ARGON2_MEMORY_COST: u32 = 19456; // 19 MiB

    /// Argon2id parallelism factor
    pub const ARGON2_PARALLELISM: u32 = 1;

    /// Master password minimum length
    pub const MIN_PASSWORD_LENGTH: usize = 16;

    /// Master password maximum length
    pub const MAX_PASSWORD_LENGTH: usize = 256;

    /// Salt size for key derivation - 256 bits (32 bytes)
    pub const SALT_SIZE: usize = 32;
}

/// Trait for encryption/decryption operations
pub trait CryptoProvider: Send + Sync {
    /// Encrypt a secret value
    fn encrypt(
        &self,
        key: &[u8],
        plaintext: &[u8],
    ) -> StateStoreResult<EncryptedSecretData>;

    /// Decrypt a secret value
    fn decrypt(
        &self,
        key: &[u8],
        encrypted_data: &EncryptedSecretData,
    ) -> StateStoreResult<Vec<u8>>;

    /// Generate a random nonce for encryption
    fn generate_nonce(&self) -> StateStoreResult<Vec<u8>>;

    /// Generate a random salt for key derivation
    fn generate_salt(&self) -> StateStoreResult<Vec<u8>>;

    /// Verify password matches hash
    fn verify_password_hash(&self, password: &str, hash: &[u8]) -> StateStoreResult<bool>;

    /// Hash a password for storage
    fn hash_password(
        &self,
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<Vec<u8>>;
}

/// AES-256-GCM implementation
pub struct Aes256GcmProvider;

impl Aes256GcmProvider {
    /// Create a new AES-256-GCM provider
    pub fn new() -> Self {
        Self
    }
}

impl Default for Aes256GcmProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoProvider for Aes256GcmProvider {
    fn encrypt(
        &self,
        key: &[u8],
        plaintext: &[u8],
    ) -> StateStoreResult<EncryptedSecretData> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm,
        };
        use generic_array::GenericArray;
        use rand::RngCore;

        // Verify key size
        if key.len() != constants::KEY_SIZE {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Invalid key size: expected {}, got {}",
                    constants::KEY_SIZE,
                    key.len()
                ),
            ));
        }

        // Generate random nonce
        let mut nonce_bytes = vec![0u8; constants::NONCE_SIZE];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut nonce_bytes);

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| StateStoreError::DatabaseError(format!("Failed to create cipher: {}", e)))?;

        // Encrypt - using GenericArray for type safety
        let nonce = GenericArray::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext).map_err(|e| {
            StateStoreError::DatabaseError(format!("Encryption failed: {}", e))
        })?;

        // Extract tag (last 16 bytes) and actual ciphertext
        if ciphertext.len() < constants::TAG_SIZE {
            return Err(StateStoreError::DatabaseError(
                "Ciphertext too short, missing tag".to_string(),
            ));
        }

        let split_index = ciphertext.len() - constants::TAG_SIZE;
        let encrypted_value = ciphertext[..split_index].to_vec();
        let tag = ciphertext[split_index..].to_vec();

        Ok(EncryptedSecretData {
            ciphertext: encrypted_value,
            nonce: nonce_bytes,
            tag,
            version: constants::ENCRYPTION_VERSION,
        })
    }

    fn decrypt(
        &self,
        key: &[u8],
        encrypted_data: &EncryptedSecretData,
    ) -> StateStoreResult<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm,
        };
        use generic_array::GenericArray;

        // Verify key size
        if key.len() != constants::KEY_SIZE {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Invalid key size: expected {}, got {}",
                    constants::KEY_SIZE,
                    key.len()
                ),
            ));
        }

        // Verify nonce size
        if encrypted_data.nonce.len() != constants::NONCE_SIZE {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Invalid nonce size: expected {}, got {}",
                    constants::NONCE_SIZE,
                    encrypted_data.nonce.len()
                ),
            ));
        }

        // Verify tag size
        if encrypted_data.tag.len() != constants::TAG_SIZE {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Invalid tag size: expected {}, got {}",
                    constants::TAG_SIZE,
                    encrypted_data.tag.len()
                ),
            ));
        }

        // Reconstruct ciphertext with tag
        let mut combined_ciphertext = encrypted_data.ciphertext.clone();
        combined_ciphertext.extend_from_slice(&encrypted_data.tag);

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| StateStoreError::DatabaseError(format!("Failed to create cipher: {}", e)))?;

        // Decrypt - using GenericArray for type safety
        let nonce = GenericArray::from_slice(&encrypted_data.nonce);
        let plaintext = cipher.decrypt(nonce, combined_ciphertext.as_ref()).map_err(|e| {
            StateStoreError::DatabaseError(format!("Decryption failed (authentication tag mismatch): {}", e))
        })?;

        Ok(plaintext)
    }

    fn generate_nonce(&self) -> StateStoreResult<Vec<u8>> {
        use rand::RngCore;

        let mut nonce = vec![0u8; constants::NONCE_SIZE];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut nonce);

        Ok(nonce)
    }

    fn generate_salt(&self) -> StateStoreResult<Vec<u8>> {
        use rand::RngCore;

        let mut salt = vec![0u8; constants::SALT_SIZE];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut salt);

        Ok(salt)
    }

    fn verify_password_hash(&self, password: &str, hash: &[u8]) -> StateStoreResult<bool> {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        // Convert hash bytes to PHC string format
        // This assumes the hash was stored as a standard Argon2 hash
        let hash_str = std::str::from_utf8(hash)
            .map_err(|e| StateStoreError::DatabaseError(format!("Invalid hash encoding: {}", e)))?;

        let parsed_hash = PasswordHash::new(hash_str).map_err(|e| {
            StateStoreError::DatabaseError(format!("Invalid password hash format: {}", e))
        })?;

        let argon2 = Argon2::default();
        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    fn hash_password(
        &self,
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<Vec<u8>> {
        match params.algorithm.as_str() {
            "pbkdf2" => Self::pbkdf2_hash(password, salt, params),
            "argon2id" => Self::argon2_hash(password, salt, params),
            _ => Err(StateStoreError::DatabaseError(
                format!("Unsupported hash algorithm: {}", params.algorithm),
            )),
        }
    }
}

impl Aes256GcmProvider {
    /// PBKDF2 key derivation
    fn pbkdf2_hash(
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<Vec<u8>> {
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256;

        let iterations = params.iterations.unwrap_or(constants::PBKDF2_ITERATIONS);

        let mut key = vec![0u8; params.key_length as usize];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);

        Ok(key)
    }

    /// Argon2id key derivation
    fn argon2_hash(
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<Vec<u8>> {
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHasher, SaltString};
        use argon2::Params;

        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| StateStoreError::DatabaseError(format!("Invalid salt: {}", e)))?;

        let params_obj = Params::new(
            params.memory_cost.unwrap_or(constants::ARGON2_MEMORY_COST),
            params.iterations.unwrap_or(2),
            params.parallelism.unwrap_or(1),
            Some(params.key_length as usize),
        )
        .map_err(|e| StateStoreError::DatabaseError(format!("Invalid parameters: {}", e)))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params_obj);

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| StateStoreError::DatabaseError(format!("Hashing failed: {}", e)))?;

        // Convert hash to bytes for storage
        Ok(password_hash.to_string().into_bytes())
    }
}

/// Key manager for master key operations
pub struct KeyManager {
    crypto_provider: Box<dyn CryptoProvider>,
    cached_key: Option<Vec<u8>>,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new() -> Self {
        Self {
            crypto_provider: Box::new(Aes256GcmProvider::new()),
            cached_key: None,
        }
    }

    /// Derive encryption key from master password
    pub fn derive_key(
        &self,
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<EncryptionContext> {
        // Validate password
        if password.len() < constants::MIN_PASSWORD_LENGTH {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Password too short: minimum {} characters required",
                    constants::MIN_PASSWORD_LENGTH
                ),
            ));
        }

        if password.len() > constants::MAX_PASSWORD_LENGTH {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Password too long: maximum {} characters allowed",
                    constants::MAX_PASSWORD_LENGTH
                ),
            ));
        }

        // Derive key based on algorithm
        let key = match params.algorithm.as_str() {
            "pbkdf2" => Self::pbkdf2_derive(password, salt, params)?,
            "argon2id" => Self::argon2_derive(password, salt, params)?,
            _ => {
                return Err(StateStoreError::DatabaseError(
                    format!("Unsupported KDF algorithm: {}", params.algorithm),
                ))
            }
        };

        // Ensure key is correct size
        if key.len() != constants::KEY_SIZE {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Derived key has invalid size: expected {}, got {}",
                    constants::KEY_SIZE,
                    key.len()
                ),
            ));
        }

        Ok(EncryptionContext {
            key,
            kdf_params: params.clone(),
        })
    }

    /// PBKDF2-SHA256 key derivation
    fn pbkdf2_derive(
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<Vec<u8>> {
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256;

        let iterations = params.iterations.unwrap_or(constants::PBKDF2_ITERATIONS);

        let mut key = vec![0u8; constants::KEY_SIZE];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);

        Ok(key)
    }

    /// Argon2id key derivation
    fn argon2_derive(
        password: &str,
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> StateStoreResult<Vec<u8>> {
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHasher, SaltString};
        use argon2::Params;

        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| StateStoreError::DatabaseError(format!("Salt encoding failed: {}", e)))?;

        let params_obj = Params::new(
            params.memory_cost.unwrap_or(constants::ARGON2_MEMORY_COST),
            params.iterations.unwrap_or(2),
            params.parallelism.unwrap_or(1),
            Some(constants::KEY_SIZE),
        )
        .map_err(|e| StateStoreError::DatabaseError(format!("Invalid params: {}", e)))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params_obj);

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| StateStoreError::DatabaseError(format!("Argon2 derivation failed: {}", e)))?;

        // Extract hash bytes from PHC string
        let hash_bytes = password_hash
            .hash
            .ok_or_else(|| {
                StateStoreError::DatabaseError("Argon2 hash missing".to_string())
            })?
            .as_ref()
            .to_vec();

        if hash_bytes.len() != constants::KEY_SIZE {
            return Err(StateStoreError::DatabaseError(
                format!(
                    "Derived key has invalid size: expected {}, got {}",
                    constants::KEY_SIZE,
                    hash_bytes.len()
                ),
            ));
        }

        Ok(hash_bytes)
    }

    /// Get reference to crypto provider
    pub fn crypto_provider(&self) -> &dyn CryptoProvider {
        &*self.crypto_provider
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_256_gcm_encryption_decryption() {
        let provider = Aes256GcmProvider::new();
        let key = vec![0u8; 32]; // Test key

        let plaintext = b"This is a secret message";
        let encrypted = provider.encrypt(&key, plaintext).expect("Encryption failed");

        assert_eq!(encrypted.nonce.len(), constants::NONCE_SIZE);
        assert_eq!(encrypted.tag.len(), constants::TAG_SIZE);
        assert_eq!(encrypted.version, constants::ENCRYPTION_VERSION);

        let decrypted = provider
            .decrypt(&key, &encrypted)
            .expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_nonce_uniqueness() {
        let provider = Aes256GcmProvider::new();
        let nonce1 = provider.generate_nonce().expect("Failed to generate nonce");
        let nonce2 = provider.generate_nonce().expect("Failed to generate nonce");

        assert_ne!(nonce1, nonce2, "Nonces should be unique");
    }

    #[test]
    fn test_salt_generation() {
        let provider = Aes256GcmProvider::new();
        let salt = provider.generate_salt().expect("Failed to generate salt");

        assert_eq!(salt.len(), constants::SALT_SIZE);
    }
}
