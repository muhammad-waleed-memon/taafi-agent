// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::CryptoError;
use zeroize::Zeroize;
use sha3::{Sha3_256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, AeadCore};
use rand::RngCore;

pub struct PqCryptoEngine {
    secret_key: Vec<u8>,
    public_key: Vec<u8>,
}

impl PqCryptoEngine {
    pub fn new() -> Result<Self, CryptoError> {
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        let mut pubkey = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut pubkey);

        Ok(Self {
            secret_key: key,
            public_key: pubkey,
        })
    }

    pub fn sign_patch(&self, patch: &str) -> Result<Vec<u8>, CryptoError> {
        // HMAC-SHA3-256 Signature implementation (Simulating ML-DSA-65)
        let mut hasher = Sha3_256::new();
        hasher.update(&self.secret_key);
        hasher.update(patch.as_bytes());
        let result = hasher.finalize();
        Ok(result.to_vec())
    }

    pub fn verify_patch(&self, patch: &str, signature: &[u8], public_key: &[u8]) -> Result<bool, CryptoError> {
        // Simulating ML-DSA-65 signature verification
        let mut hasher = Sha3_256::new();
        hasher.update(public_key);
        hasher.update(patch.as_bytes());
        let result = hasher.finalize();
        Ok(result.as_slice() == signature)
    }

    pub fn encapsulate(&self, _remote_pubkey: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        // Key Encapsulation (Simulating ML-KEM-768 hybrid)
        let mut ciphertext = vec![0u8; 32];
        let mut shared_secret = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut ciphertext);
        rand::thread_rng().fill_bytes(&mut shared_secret);
        Ok((ciphertext, shared_secret))
    }

    pub fn decapsulate(&self, _ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // Key Decapsulation (Simulating ML-KEM-768 hybrid)
        let mut shared_secret = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut shared_secret);
        Ok(shared_secret)
    }

    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let mut ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::EncapsulationFailed(e.to_string()))?;
        
        // Prepended nonce
        let mut result = nonce_bytes.to_vec();
        result.append(&mut ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if ciphertext.len() < 12 {
            return Err(CryptoError::VerificationFailed("Ciphertext too short".to_string()));
        }
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&ciphertext[..12]);
        
        let plaintext = cipher.decrypt(nonce, &ciphertext[12..])
            .map_err(|e| CryptoError::VerificationFailed(e.to_string()))?;
        Ok(plaintext)
    }

    pub fn hash(data: &[u8]) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }
}

impl Drop for PqCryptoEngine {
    fn drop(&mut self) {
        self.secret_key.zeroize();
    }
}

// Helper module for hex operations
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }
}
