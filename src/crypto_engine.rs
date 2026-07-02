// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::CryptoError;
use zeroize::Zeroize;
use sha3::{Sha3_256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, AeadCore};
use rand::RngCore;

use pqcrypto_traits::kem::{PublicKey as KemPublicKey, SecretKey as KemSecretKey, Ciphertext as KemCiphertext};
use pqcrypto_traits::sign::{PublicKey as DsaPublicKey, SecretKey as DsaSecretKey, Signature as DsaSignature};

pub struct PqCryptoEngine {
    kem_secret: pqcrypto_mlkem::mlkem768::SecretKey,
    pub kem_public: pqcrypto_mlkem::mlkem768::PublicKey,
    dsa_secret: pqcrypto_mldsa::mldsa65::SecretKey,
    pub dsa_public: pqcrypto_mldsa::mldsa65::PublicKey,
}

impl PqCryptoEngine {
    pub fn new() -> Result<Self, CryptoError> {
        let (kem_public, kem_secret) = pqcrypto_mlkem::mlkem768::keypair();
        let (dsa_public, dsa_secret) = pqcrypto_mldsa::mldsa65::keypair();

        Ok(Self {
            kem_secret,
            kem_public,
            dsa_secret,
            dsa_public,
        })
    }

    pub fn sign_patch(&self, patch: &str) -> Result<Vec<u8>, CryptoError> {
        let sig = pqcrypto_mldsa::mldsa65::sign(patch.as_bytes(), &self.dsa_secret);
        Ok(sig.as_bytes().to_vec())
    }

    pub fn verify_patch(&self, patch: &str, signature: &[u8], public_key: &[u8]) -> Result<bool, CryptoError> {
        let pubkey = pqcrypto_mldsa::mldsa65::PublicKey::from_bytes(public_key)
            .map_err(|e| CryptoError::VerificationFailed(format!("Invalid public key bytes: {:?}", e)))?;
        let sig = pqcrypto_mldsa::mldsa65::Signature::from_bytes(signature)
            .map_err(|e| CryptoError::VerificationFailed(format!("Invalid signature bytes: {:?}", e)))?;

        match pqcrypto_mldsa::mldsa65::verify(patch.as_bytes(), &sig, &pubkey) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn encapsulate(&self, remote_pubkey: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let pubkey = pqcrypto_mlkem::mlkem768::PublicKey::from_bytes(remote_pubkey)
            .map_err(|e| CryptoError::EncapsulationFailed(format!("Invalid remote public key bytes: {:?}", e)))?;
        let (ciphertext, shared_secret) = pqcrypto_mlkem::mlkem768::encapsulate(&pubkey);
        Ok((ciphertext.as_bytes().to_vec(), shared_secret.as_bytes().to_vec()))
    }

    pub fn decapsulate(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let ct = pqcrypto_mlkem::mlkem768::Ciphertext::from_bytes(ciphertext)
            .map_err(|e| CryptoError::EncapsulationFailed(format!("Invalid ciphertext bytes: {:?}", e)))?;
        let shared_secret = pqcrypto_mlkem::mlkem768::decapsulate(&ct, &self.kem_secret);
        Ok(shared_secret.as_bytes().to_vec())
    }

    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if key.len() < 32 {
            return Err(CryptoError::EncapsulationFailed("Shared secret key too short for AES-256".to_string()));
        }
        let key_ref = Key::<Aes256Gcm>::from_slice(&key[..32]);
        let cipher = Aes256Gcm::new(key_ref);
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let mut ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::EncapsulationFailed(e.to_string()))?;
        
        let mut result = nonce_bytes.to_vec();
        result.append(&mut ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if ciphertext.len() < 12 {
            return Err(CryptoError::VerificationFailed("Ciphertext too short".to_string()));
        }
        if key.len() < 32 {
            return Err(CryptoError::VerificationFailed("Shared secret key too short for AES-256".to_string()));
        }
        let key_ref = Key::<Aes256Gcm>::from_slice(&key[..32]);
        let cipher = Aes256Gcm::new(key_ref);
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

// Helper module for hex operations
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

