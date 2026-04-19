use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ma_core::{
    ipfs_publish::publish_did_document_to_kubo, Did, Document, Message, CONTENT_TYPE_IPFS_REQUEST,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

pub const IPFS_REPLY_CONTENT_TYPE: &str = "application/x-ipfs-request-reply";

#[derive(Debug, Serialize)]
pub struct IpfsRequestReply {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IpfsPublishRequest {
    did_document_dag_cbor_base64: String,
    #[serde(default)]
    ipns_private_key_base64: String,
}

pub async fn handle_ipfs_publish_message(
    kubo_rpc_api: &str,
    message: &Message,
) -> IpfsRequestReply {
    if message.content_type != CONTENT_TYPE_IPFS_REQUEST {
        warn!(
            message_id = %message.id,
            from = %message.from,
            to = %message.to,
            content_type = %message.content_type,
            expected_content_type = CONTENT_TYPE_IPFS_REQUEST,
            "invalid ipfs publish message"
        );
        return IpfsRequestReply {
            status: 400,
            code: "bad-request",
            message: format!(
                "unexpected content_type '{}', expected '{}'",
                message.content_type, CONTENT_TYPE_IPFS_REQUEST
            ),
            upstream_detail: None,
            did: None,
            cid: None,
        };
    }

    match process_ipfs_publish_request(kubo_rpc_api, message).await {
        Ok((did, cid)) => IpfsRequestReply {
            status: 200,
            code: "ok",
            message: "did document published via ma/ipfs/0.0.1".to_string(),
            upstream_detail: None,
            did: Some(did),
            cid: Some(cid),
        },
        Err(err) => {
            let err_text = err.to_string();
            let (status, code) = map_ipfs_error(&err_text);
            warn!(
                message_id = %message.id,
                from = %message.from,
                to = %message.to,
                code = code,
                status = status,
                error = %err_text,
                "ipfs publish request rejected"
            );
            IpfsRequestReply {
                status,
                code,
                message: ipfs_error_summary(code),
                upstream_detail: Some(err_text),
                did: None,
                cid: None,
            }
        }
    }
}

async fn process_ipfs_publish_request(
    kubo_rpc_api: &str,
    message: &Message,
) -> Result<(String, String)> {
    let request: IpfsPublishRequest = serde_json::from_slice(&message.content)
        .map_err(|err| anyhow!("invalid IPFS publish payload: {err}"))?;

    let sender_did = Did::try_from(message.from.as_str())
        .map_err(|err| anyhow!("invalid sender did '{}': {err}", message.from))?;

    let document_cbor = B64
        .decode(request.did_document_dag_cbor_base64.trim())
        .map_err(|err| anyhow!("invalid dag-cbor base64 payload: {err}"))?;

    let document = Document::from_cbor(&document_cbor)
        .map_err(|err| anyhow!("invalid DID document DAG-CBOR: {err}"))?;

    document
        .validate()
        .map_err(|err| anyhow!("invalid DID document: {err}"))?;
    document
        .verify()
        .map_err(|err| anyhow!("DID document signature verification failed: {err}"))?;

    let document_did = Did::try_from(document.id.as_str())
        .map_err(|err| anyhow!("invalid document DID '{}': {err}", document.id))?;

    if document_did.ipns != sender_did.ipns {
        return Err(anyhow!(
            "sender IPNS '{}' does not match document IPNS '{}'",
            sender_did.ipns,
            document_did.ipns
        ));
    }

    message
        .verify_with_document(&document)
        .map_err(|err| anyhow!("request signature verification failed: {err}"))?;

    if request.ipns_private_key_base64.trim().is_empty() {
        return Err(anyhow!("ipns_private_key_base64 is required"));
    }

    let document_json = document
        .marshal()
        .map_err(|err| anyhow!("failed to marshal DID document to JSON for publish: {err}"))?;

    let cid = publish_did_document_to_kubo(
        kubo_rpc_api,
        &document_json,
        &request.ipns_private_key_base64,
    )
    .await?
    .ok_or_else(|| anyhow!("publish succeeded without cid"))?;

    let document_did = Did::try_from(document.id.as_str())
        .map_err(|err| anyhow!("invalid document DID '{}': {err}", document.id))?;

    Ok((document_did.id(), cid))
}

fn map_ipfs_error(error_text: &str) -> (u16, &'static str) {
    let text = error_text.to_lowercase();

    if text.contains("expected application/x-ma-ipfs-request")
        || text.contains("invalid ipfs publish payload")
        || text.contains("invalid signed message")
        || text.contains("invalid sender did")
        || text.contains("invalid dag-cbor base64 payload")
    {
        return (400, "bad-request");
    }

    if text.contains("signature verification failed") || text.contains("request signature") {
        return (401, "auth-failed");
    }

    if text.contains("forbidden") || text.contains("not allowed") || text.contains("denied") {
        return (403, "forbidden");
    }

    if text.contains("does not match") {
        return (409, "identity-conflict");
    }

    if text.contains("invalid base64 key") || text.contains("ipns_private_key_base64 is required") {
        return (422, "invalid-key");
    }

    if text.contains("invalid did document")
        || text.contains("invalid document did")
        || text.contains("invalid did document dag-cbor")
    {
        return (422, "invalid-document");
    }

    if text.contains("connection")
        || text.contains("refused")
        || text.contains("unavailable")
        || text.contains("temporarily")
        || text.contains("kubo")
        || text.contains("timeout")
    {
        return (503, "service-unavailable");
    }

    if text.contains("name_publish") || text.contains("dag_put") || text.contains("import_key") {
        return (500, "publish-failed");
    }

    if text.contains("publish succeeded without cid") {
        return (500, "publish-failed");
    }

    (500, "internal-error")
}

fn ipfs_error_summary(code: &str) -> String {
    match code {
        "bad-request" => "invalid ipfs publish request".to_string(),
        "auth-failed" => "request authentication failed".to_string(),
        "forbidden" => "request rejected by local policy".to_string(),
        "identity-conflict" => "request identity conflict".to_string(),
        "invalid-key" => "invalid or missing ipns private key".to_string(),
        "invalid-document" => "invalid did document".to_string(),
        "service-unavailable" => "kubo service unavailable".to_string(),
        "publish-failed" => "publish operation failed".to_string(),
        _ => "internal processing failure".to_string(),
    }
}
