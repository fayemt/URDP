//! URDP control message definitions and helper functions.
//!
//! This crate provides minimal data structures for URDP control
//! messages (`CODEX_OFFER`, `CODEX_SELECT` and `CODEX_COMMIT`) along
//! with helper functions to serialise them to canonical CBOR and
//! sign/verify messages.  The focus here is on clarity and
//! correctness rather than performance.  It is intended to serve as
//! a reference implementation for other languages.

use blake3::Hasher;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

/// Error type for control message operations.
#[derive(Debug, Error)]
pub enum ControlError {
    #[error("CBOR serialization error: {0}")]
    Cbor(#[from] serde_cbor::Error),
    #[error("Signature verification failed")]
    Signature,
}

/// Description of a codex pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexDescriptor {
    /// The 32‑byte BLAKE3 hash of the codex pack.
    pub codex_id: [u8; 32],
    /// Human friendly name of the codex.
    pub name: String,
    /// Semantic version (e.g. "v1.0").
    pub semver: String,
    /// Identifier of the vendor or publisher.
    pub vendor_id: String,
    /// List of domains supported by this codex (e.g. "texture/BC7").
    pub domains: Vec<String>,
    /// Expected bits per byte for each supported domain (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_bpb: Option<u64>,
    /// Expected decode time in microseconds per KiB (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_decode_us: Option<u64>,
    /// Size of the codex pack in bytes (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_size_bytes: Option<u64>,
}

/// `CODEX_OFFER` message: the sender proposes one or more codexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexOffer {
    /// Monotonic identifier for correlating offers and selects.
    pub offer_id: u64,
    /// A list of available codex descriptors.
    pub codex_list: Vec<CodexDescriptor>,
}

/// `CODEX_SELECT` message: the receiver chooses a codex per domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexSelect {
    /// The offer being responded to.
    pub offer_id: u64,
    /// Mapping from domain to codex id.  Keys must be sorted for canonical CBOR.
    pub mapping: Vec<(String, [u8; 32])>,
    /// Policy hint (e.g. "Balanced").  Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
}

/// `CODEX_COMMIT` message: both parties commit to a mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCommit {
    /// Monotonic identifier of the offer/selection.
    pub offer_id: u64,
    /// Final mapping from domain to codex id.
    pub mapping: Vec<(String, [u8; 32])>,
    /// Fallback flags indicating which domains may be downgraded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Vec<String>>,
}

/// Serialise a control message to deterministic CBOR.
pub fn to_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, ControlError> {
    // Use serde_cbor with indefinite items disabled and canonical ordering.
    let mut buf = Vec::new();
    let mut ser = serde_cbor::ser::Serializer::new(&mut buf);
    ser.self_describe()?;
    value.serialize(&mut ser)?;
    Ok(buf)
}

/// Compute a BLAKE3 hash of a canonical mapping.
pub fn compute_codex_map_id(mapping: &[(String, [u8; 32])]) -> [u8; 32] {
    // Sort mapping by domain name for deterministic hashing.
    let mut sorted = mapping.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Hasher::new();
    for (domain, id) in sorted.iter() {
        hasher.update(domain.as_bytes());
        hasher.update(id);
    }
    *hasher.finalize().as_bytes()
}

/// Sign a message (CBOR) using Ed25519.
pub fn sign_message(data: &[u8], keypair: &Keypair) -> Signature {
    keypair.sign(data)
}

/// Verify a message signature with the corresponding public key.
pub fn verify_message(data: &[u8], signature: &Signature, public_key: &PublicKey) -> Result<(), ControlError> {
    public_key
        .verify_strict(data, signature)
        .map_err(|_| ControlError::Signature)
}

/// Derive an 8‑byte session tag from a codex map id and a secret.
pub fn derive_session_tag(map_id: &[u8], secret: &[u8]) -> [u8; 8] {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(map_id);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 8];
    tag.copy_from_slice(&result[..8]);
    tag
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    #[test]
    fn test_cbor_roundtrip() {
        let offer = CodexOffer {
            offer_id: 1,
            codex_list: vec![CodexDescriptor {
                codex_id: [0u8; 32],
                name: "Test".into(),
                semver: "v0.1".into(),
                vendor_id: "vendor".into(),
                domains: vec!["test/domain".into()],
                exp_bpb: None,
                exp_decode_us: None,
                pack_size_bytes: None,
            }],
        };
        let bytes = to_cbor(&offer).unwrap();
        let de: CodexOffer = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(de.offer_id, 1);
        assert_eq!(de.codex_list[0].name, "Test");
    }
}
