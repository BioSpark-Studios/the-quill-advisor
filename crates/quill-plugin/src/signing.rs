//! Cryptographic module signing (ed25519).
//!
//! A [`SignedPlugin`] wraps a manifest with an optional detached signature over
//! its canonical bytes and the signer's public key. On install the app checks
//! the signature against a [`TrustStore`] of publisher keys and assigns a
//! [`TrustLevel`] used for the store's "Verified / Untrusted / Unsigned" badge.
//! This is what lets a self-hosted platform accept third-party plugins without
//! trusting arbitrary code blindly.

use crate::error::{PluginError, Result};
use crate::manifest::PluginManifest;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// How much a plugin's provenance can be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// Signed by a key in the trust store; provenance confirmed.
    Verified,
    /// No signature attached.
    Unsigned,
    /// Signed, but the signature is invalid or from an unknown publisher.
    Untrusted,
}

/// A manifest plus its (optional) detached signature and signer public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedPlugin {
    /// The plugin manifest.
    pub manifest: PluginManifest,
    /// Hex-encoded ed25519 signature over `manifest.signing_bytes()`.
    #[serde(default)]
    pub signature: Option<String>,
    /// Hex-encoded ed25519 public key of the signer.
    #[serde(default)]
    pub signer: Option<String>,
}

impl SignedPlugin {
    /// Wrap an unsigned manifest.
    #[must_use]
    pub fn unsigned(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            signature: None,
            signer: None,
        }
    }

    /// Sign a manifest with `key`, producing a self-describing signed plugin.
    ///
    /// # Errors
    /// Fails if the manifest cannot be serialized.
    pub fn sign(manifest: PluginManifest, key: &SigningKey) -> Result<Self> {
        let sig: Signature = key.sign(&manifest.signing_bytes()?);
        Ok(Self {
            manifest,
            signature: Some(hex::encode(sig.to_bytes())),
            signer: Some(hex::encode(key.verifying_key().to_bytes())),
        })
    }

    /// Determine the trust level against a set of trusted publisher keys.
    #[must_use]
    pub fn trust_level(&self, trust: &TrustStore) -> TrustLevel {
        let (Some(sig_hex), Some(signer_hex)) = (&self.signature, &self.signer) else {
            return TrustLevel::Unsigned;
        };
        let Ok(vk) = parse_verifying_key(signer_hex) else {
            return TrustLevel::Untrusted;
        };
        let Ok(sig) = parse_signature(sig_hex) else {
            return TrustLevel::Untrusted;
        };
        let Ok(bytes) = self.manifest.signing_bytes() else {
            return TrustLevel::Untrusted;
        };
        // Signature must verify AND the signer must be a trusted publisher.
        if vk.verify(&bytes, &sig).is_ok() && trust.contains(&vk) {
            TrustLevel::Verified
        } else {
            TrustLevel::Untrusted
        }
    }
}

/// The set of publisher public keys the app trusts (e.g. the BioSpark key).
#[derive(Debug, Clone, Default)]
pub struct TrustStore {
    keys: Vec<VerifyingKey>,
}

impl TrustStore {
    /// An empty trust store.
    #[must_use]
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Add a trusted publisher key.
    pub fn trust(&mut self, key: VerifyingKey) {
        if !self.contains(&key) {
            self.keys.push(key);
        }
    }

    /// Add a trusted publisher key from its hex encoding.
    ///
    /// # Errors
    /// Fails if the hex is malformed or not a valid ed25519 public key.
    pub fn trust_hex(&mut self, hex_key: &str) -> Result<()> {
        self.trust(parse_verifying_key(hex_key)?);
        Ok(())
    }

    /// Whether a key is trusted.
    #[must_use]
    pub fn contains(&self, key: &VerifyingKey) -> bool {
        self.keys.iter().any(|k| k.to_bytes() == key.to_bytes())
    }
}

fn parse_verifying_key(hex_key: &str) -> Result<VerifyingKey> {
    let bytes = hex::decode(hex_key)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| PluginError::Crypto("public key must be 32 bytes".into()))?;
    VerifyingKey::from_bytes(&arr).map_err(|e| PluginError::Crypto(e.to_string()))
}

fn parse_signature(hex_sig: &str) -> Result<Signature> {
    let bytes = hex::decode(hex_sig)?;
    let arr: [u8; 64] = bytes
        .try_into()
        .map_err(|_| PluginError::Crypto("signature must be 64 bytes".into()))?;
    Ok(Signature::from_bytes(&arr))
}
