use std::time::Duration;

use anyhow::{anyhow, Result};
use ma_core::ipfs_publish::publish_did_document_to_kubo;
use ma_core::{Did, Document};
use serde::Serialize;

use ma_core::kubo::{
    dag_put, generate_key, list_keys, name_publish_with_retry, wait_for_api, IpnsPublishOptions,
    KuboKey,
};

/// Publish a signed DID document to Kubo/IPNS.
///
/// Peeks at the Kubo keystore first: if the IPNS key matching `expected_ipns_id`
/// is already imported, publishes directly using the existing alias.
/// Otherwise imports the key (via `publish_did_document_to_kubo`) then publishes.
pub async fn publish_identity_document(
    kubo_rpc_api: &str,
    expected_ipns_id: &str,
    did_document_json: &str,
    ipns_private_key_base64: &str,
) -> Result<Option<String>> {
    wait_for_api(kubo_rpc_api, 10).await?;

    let keys = list_keys(kubo_rpc_api).await?;
    if let Some(existing) = keys.into_iter().find(|k| k.id == expected_ipns_id) {
        // Key already in Kubo — publish without re-importing
        let document = Document::unmarshal(did_document_json)
            .map_err(|err| anyhow!("invalid DID document JSON: {err}"))?;
        document
            .validate()
            .map_err(|err| anyhow!("invalid DID document: {err}"))?;
        document
            .verify()
            .map_err(|err| anyhow!("DID document signature verification failed: {err}"))?;

        let cid = dag_put(kubo_rpc_api, &document).await?;
        let publish_options = IpnsPublishOptions::default();
        name_publish_with_retry(
            kubo_rpc_api,
            &existing.name,
            &cid,
            &publish_options,
            3,
            Duration::from_millis(1000),
        )
        .await?;

        return Ok(Some(existing.name));
    }

    // Key not yet in Kubo — import and publish
    publish_did_document_to_kubo(kubo_rpc_api, did_document_json, ipns_private_key_base64).await
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentityPublishResult {
    pub did: String,
    pub cid: String,
}

pub async fn ensure_kubo_key_alias(kubo_rpc_api: &str, alias: &str) -> Result<KuboKey> {
    wait_for_api(kubo_rpc_api, 10).await?;

    let alias = alias.trim();
    if alias.is_empty() {
        return Err(anyhow!("kubo key alias must not be empty"));
    }

    let keys = list_keys(kubo_rpc_api).await?;
    if let Some(existing) = keys.into_iter().find(|key| key.name == alias) {
        return Ok(existing);
    }

    generate_key(kubo_rpc_api, alias).await?;
    let keys = list_keys(kubo_rpc_api).await?;
    keys.into_iter()
        .find(|key| key.name == alias)
        .ok_or_else(|| anyhow!("generated kubo key alias '{}' not found in key list", alias))
}

pub async fn publish_identity_with_kubo_alias(
    kubo_rpc_api: &str,
    kubo_key_alias: &str,
    did_document_json: &str,
) -> Result<IdentityPublishResult> {
    let document = Document::unmarshal(did_document_json)
        .map_err(|err| anyhow!("invalid DID document JSON: {err}"))?;

    document
        .validate()
        .map_err(|err| anyhow!("invalid DID document: {err}"))?;
    document
        .verify()
        .map_err(|err| anyhow!("DID document signature verification failed: {err}"))?;

    let document_did = Did::try_from(document.id.as_str())
        .map_err(|err| anyhow!("invalid document DID '{}': {err}", document.id))?;

    let alias_key = ensure_kubo_key_alias(kubo_rpc_api, kubo_key_alias).await?;
    if alias_key.id != document_did.ipns {
        return Err(anyhow!(
            "kubo alias '{}' has IPNS id '{}' but document DID expects '{}'",
            kubo_key_alias,
            alias_key.id,
            document_did.ipns
        ));
    }

    let cid = dag_put(kubo_rpc_api, &document).await?;
    let publish_options = IpnsPublishOptions::default();
    name_publish_with_retry(
        kubo_rpc_api,
        kubo_key_alias,
        &cid,
        &publish_options,
        3,
        std::time::Duration::from_millis(1000),
    )
    .await?;

    Ok(IdentityPublishResult {
        did: document_did.id(),
        cid,
    })
}
