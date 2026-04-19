use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use tracing::{info, warn};

use crate::status::{
    mark_startup_publish_attempt,
    mark_startup_publish_failed,
    mark_startup_publish_retry,
    mark_startup_publish_succeeded,
    SharedWorldStatus,
};
use ma_world_core::{publish_identity_document, publish_identity_with_kubo_alias};

pub async fn retry_publish_identity(
    status: SharedWorldStatus,
    kubo_rpc_api: &str,
    expected_ipns_id: &str,
    did_document_json: &str,
    ipns_private_key_base64: &str,
    max_retries: u32,
) -> Result<String> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let now = now_unix_secs();
        mark_startup_publish_attempt(&status, now, attempt).await;

        match publish_identity_document(
            kubo_rpc_api,
            expected_ipns_id,
            did_document_json,
            ipns_private_key_base64,
        )
        .await
        {
            Ok(Some(did)) => {
                mark_startup_publish_succeeded(&status, now, did.clone(), None).await;
                info!("startup identity published successfully on attempt {}", attempt);
                return Ok(did);
            }
            Ok(None) => {
                let error_text = format!("startup identity publish returned None on attempt {}", attempt);
                warn!("{}", error_text);

                if attempt >= max_retries {
                    mark_startup_publish_failed(&status, error_text.clone()).await;
                    return Err(anyhow!(
                        "startup identity publish failed after {} attempts",
                        max_retries
                    ));
                }

                let backoff_secs = fibonacci_backoff_secs(attempt);
                mark_startup_publish_retry(&status, error_text, now.saturating_add(backoff_secs)).await;
                warn!(attempt = attempt, next_retry_in_secs = backoff_secs, "retrying startup identity publish after backoff");
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            }
            Err(err) => {
                let error_text = err.to_string();
                warn!(error = %err, attempt = attempt, max_retries = max_retries, "startup identity publish failed");

                if attempt >= max_retries {
                    mark_startup_publish_failed(&status, error_text).await;
                    return Err(anyhow!(
                        "startup identity publish failed after {} attempts",
                        max_retries
                    ));
                }

                let backoff_secs = fibonacci_backoff_secs(attempt);
                mark_startup_publish_retry(&status, error_text, now.saturating_add(backoff_secs)).await;
                warn!(attempt = attempt, next_retry_in_secs = backoff_secs, "retrying startup identity publish after backoff");
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            }
        }
    }
}

pub async fn retry_publish_identity_alias(
    status: SharedWorldStatus,
    kubo_rpc_api: &str,
    kubo_key_alias: &str,
    did_document_json: &str,
    max_retries: u32,
) -> Result<(String, String)> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let now = now_unix_secs();
        mark_startup_publish_attempt(&status, now, attempt).await;

        match publish_identity_with_kubo_alias(kubo_rpc_api, kubo_key_alias, did_document_json).await {
            Ok(result) => {
                mark_startup_publish_succeeded(
                    &status,
                    now,
                    result.did.clone(),
                    Some(result.cid.clone()),
                )
                .await;
                info!("alias identity published successfully on attempt {}", attempt);
                return Ok((result.did, result.cid));
            }
            Err(err) => {
                let error_text = err.to_string();
                warn!(error = %err, attempt = attempt, max_retries = max_retries, "alias identity publish failed");

                if attempt >= max_retries {
                    mark_startup_publish_failed(&status, error_text).await;
                    return Err(anyhow!(
                        "alias identity publish failed after {} attempts",
                        max_retries
                    ));
                }

                let backoff_secs = fibonacci_backoff_secs(attempt);
                mark_startup_publish_retry(&status, error_text, now.saturating_add(backoff_secs)).await;
                warn!(attempt = attempt, next_retry_in_secs = backoff_secs, "retrying alias identity publish after backoff");
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            }
        }
    }
}

fn fibonacci_backoff_secs(attempt: u32) -> u64 {
    match attempt {
        0 | 1 => 1,
        _ => {
            let mut prev = 1u64;
            let mut curr = 1u64;
            for _ in 2..attempt {
                let next = prev.saturating_add(curr);
                prev = curr;
                curr = next;
            }
            curr
        }
    }
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}