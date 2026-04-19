use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use ma_core::{Did, EncryptionKey, IrohEndpoint, MaEndpoint, SigningKey, VerificationMethod};
use ma_did::{Document, Ipld};

pub fn generate_world_did_document_from_keys(
    ipns: &str,
    endpoint: &IrohEndpoint,
    signing_private_key_hex: &str,
    encryption_private_key_hex: &str,
) -> Result<String> {
    let root_did = Did::new_url(ipns, None::<String>)
        .with_context(|| format!("build root did for '{}'", ipns))?;
    let signing_did = Did::new_url(ipns, Some("signing".to_string()))
        .with_context(|| format!("build signing did for '{}'", ipns))?;
    let encryption_did = Did::new_url(ipns, Some("encryption".to_string()))
        .with_context(|| format!("build encryption did for '{}'", ipns))?;

    let signing_key_bytes: [u8; 32] = hex::decode(signing_private_key_hex.trim())
        .with_context(|| format!("decode signing key hex for '{}'", ipns))?
        .try_into()
        .map_err(|_| anyhow!("invalid signing key length for '{}'", ipns))?;
    let encryption_key_bytes: [u8; 32] = hex::decode(encryption_private_key_hex.trim())
        .with_context(|| format!("decode encryption key hex for '{}'", ipns))?
        .try_into()
        .map_err(|_| anyhow!("invalid encryption key length for '{}'", ipns))?;

    let signing_key = SigningKey::from_private_key_bytes(signing_did, signing_key_bytes)
        .with_context(|| format!("reconstruct signing key for '{}'", ipns))?;
    let encryption_key =
        EncryptionKey::from_private_key_bytes(encryption_did, encryption_key_bytes)
            .with_context(|| format!("reconstruct encryption key for '{}'", ipns))?;

    let signing_vm = VerificationMethod::try_from(&signing_key)
        .with_context(|| format!("build signing verification method for '{}'", ipns))?;
    let encryption_vm = VerificationMethod::try_from(&encryption_key)
        .with_context(|| format!("build encryption verification method for '{}'", ipns))?;

    let mut document = Document::new(&root_did, &root_did);
    document
        .add_verification_method(signing_vm.clone())
        .with_context(|| format!("add signing verification method for '{}'", ipns))?;
    document
        .add_verification_method(encryption_vm.clone())
        .with_context(|| format!("add encryption verification method for '{}'", ipns))?;
    document.assertion_method = vec![signing_vm.id.clone()];
    document.key_agreement = vec![encryption_vm.id.clone()];

    document.set_ma(build_ma_namespace(&endpoint.services()));
    endpoint
        .reconcile_document_ma_iroh(&mut document)
        .with_context(|| format!("reconcile endpoint metadata for '{}'", ipns))?;

    document
        .sign(&signing_key, &signing_vm)
        .with_context(|| format!("sign did document for '{}'", ipns))?;
    document
        .validate()
        .with_context(|| format!("validate did document for '{}'", ipns))?;
    document
        .verify()
        .with_context(|| format!("verify did document for '{}'", ipns))?;

    document
        .marshal()
        .with_context(|| format!("marshal did document for '{}'", ipns))
}

pub fn generate_world_did_document_ephemeral(
    ipns: &str,
    endpoint: &IrohEndpoint,
) -> Result<String> {
    let generated_identity = ma_did::generate_identity(ipns)
        .map_err(|err| anyhow!("generate world did document for '{}': {err}", ipns))?;

    generate_world_did_document_from_keys(
        ipns,
        endpoint,
        &generated_identity.signing_private_key_hex,
        &generated_identity.encryption_private_key_hex,
    )
}

fn build_ma_namespace(services: &[String]) -> Ipld {
    Ipld::Map(BTreeMap::from([(
        "services".to_string(),
        Ipld::List(services.iter().cloned().map(Ipld::String).collect()),
    )]))
}
