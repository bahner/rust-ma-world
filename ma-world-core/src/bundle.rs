use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

const BUNDLE_AAD: &[u8] = b"ma-world-core/bundle:v2";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlainIdentityBundle {
    pub did_document_json: String,
    pub ipns_private_key_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedIdentityBundle {
    pub version: u32,
    pub kdf: String,
    pub cipher: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

pub fn parse_plain_identity_bundle_json(raw: &str) -> Result<PlainIdentityBundle> {
    let bundle: PlainIdentityBundle =
        serde_json::from_str(raw).map_err(|err| anyhow!("invalid plaintext identity bundle JSON: {err}"))?;
    if bundle.did_document_json.trim().is_empty() {
        return Err(anyhow!("did_document_json is empty in plaintext identity bundle"));
    }
    if bundle.ipns_private_key_base64.trim().is_empty() {
        return Err(anyhow!("ipns_private_key_base64 is empty in plaintext identity bundle"));
    }
    Ok(bundle)
}

pub fn encrypt_identity_bundle(
    passphrase: &str,
    plain: &PlainIdentityBundle,
) -> Result<EncryptedIdentityBundle> {
    if passphrase.is_empty() {
        return Err(anyhow!("passphrase must not be empty"));
    }

    let plaintext = serde_json::to_vec(plain)
        .map_err(|err| anyhow!("serialize plaintext identity bundle: {err}"))?;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);

    let key = derive_key(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), Payload { msg: &plaintext, aad: BUNDLE_AAD })
        .map_err(|err| anyhow!("encrypt identity bundle: {err}"))?;

    Ok(EncryptedIdentityBundle {
        version: 2,
        kdf: "argon2id".to_string(),
        cipher: "xchacha20poly1305".to_string(),
        salt_b64: B64.encode(salt),
        nonce_b64: B64.encode(nonce),
        ciphertext_b64: B64.encode(ciphertext),
    })
}

pub fn encrypt_identity_bundle_json(passphrase: &str, plain: &PlainIdentityBundle) -> Result<String> {
    let encrypted = encrypt_identity_bundle(passphrase, plain)?;
    serde_json::to_string(&encrypted).map_err(|err| anyhow!("serialize encrypted identity bundle: {err}"))
}

pub fn decrypt_identity_bundle(
    passphrase: &str,
    encrypted: &EncryptedIdentityBundle,
) -> Result<PlainIdentityBundle> {
    if passphrase.is_empty() {
        return Err(anyhow!("passphrase must not be empty"));
    }

    if encrypted.version != 2 {
        return Err(anyhow!("unsupported bundle version: {}", encrypted.version));
    }
    if encrypted.kdf != "argon2id" {
        return Err(anyhow!("unsupported bundle kdf: {}", encrypted.kdf));
    }
    if encrypted.cipher != "xchacha20poly1305" {
        return Err(anyhow!("unsupported bundle cipher: {}", encrypted.cipher));
    }

    let salt = B64
        .decode(encrypted.salt_b64.trim())
        .map_err(|err| anyhow!("invalid salt_b64: {err}"))?;
    if salt.len() != SALT_LEN {
        return Err(anyhow!("invalid salt length: expected {SALT_LEN}, got {}", salt.len()));
    }

    let nonce = B64
        .decode(encrypted.nonce_b64.trim())
        .map_err(|err| anyhow!("invalid nonce_b64: {err}"))?;
    if nonce.len() != NONCE_LEN {
        return Err(anyhow!("invalid nonce length: expected {NONCE_LEN}, got {}", nonce.len()));
    }

    let ciphertext = B64
        .decode(encrypted.ciphertext_b64.trim())
        .map_err(|err| anyhow!("invalid ciphertext_b64: {err}"))?;

    let key = derive_key(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let plaintext = cipher
        .decrypt(XNonce::from_slice(&nonce), Payload { msg: &ciphertext, aad: BUNDLE_AAD })
        .map_err(|err| anyhow!("decrypt identity bundle: {err}"))?;

    let plain: PlainIdentityBundle = serde_json::from_slice(&plaintext)
        .map_err(|err| anyhow!("invalid decrypted identity bundle JSON: {err}"))?;

    if plain.did_document_json.trim().is_empty() {
        return Err(anyhow!("did_document_json is empty in decrypted identity bundle"));
    }
    if plain.ipns_private_key_base64.trim().is_empty() {
        return Err(anyhow!("ipns_private_key_base64 is empty in decrypted identity bundle"));
    }

    Ok(plain)
}

pub fn decrypt_identity_bundle_json(passphrase: &str, raw: &str) -> Result<PlainIdentityBundle> {
    let encrypted: EncryptedIdentityBundle =
        serde_json::from_str(raw).map_err(|err| anyhow!("invalid encrypted identity bundle JSON: {err}"))?;
    decrypt_identity_bundle(passphrase, &encrypted)
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = Params::new(65_536, 3, 1, Some(KEY_LEN))
        .map_err(|err| anyhow!("argon2 params: {err}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|err| anyhow!("argon2 key derivation failed: {err}"))?;
    Ok(key)
}
