use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct StartupPublishStatus {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    pub attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_attempt_at_unix: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_retry_at_unix: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success_at_unix: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_cid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldStatus {
    pub slug: String,
    pub owner: String,
    pub config_path: String,
    pub status_api_bind: String,
    pub ipns_endpoint: Option<String>,
    pub started_at_unix: u64,
    pub endpoint_id: Option<String>,
    pub services: Vec<String>,
    pub inbox_messages: u64,
    pub ipfs_messages: u64,
    pub last_inbox_at_unix: Option<u64>,
    pub last_ipfs_at_unix: Option<u64>,
    pub startup_publish: StartupPublishStatus,
}

pub type SharedWorldStatus = Arc<RwLock<WorldStatus>>;

pub fn new_shared_status(
    slug: String,
    owner: String,
    config_path: String,
    status_api_bind: String,
    ipns_endpoint: Option<String>,
    started_at_unix: u64,
) -> SharedWorldStatus {
    Arc::new(RwLock::new(WorldStatus {
        slug,
        owner,
        config_path,
        status_api_bind,
        ipns_endpoint,
        started_at_unix,
        endpoint_id: None,
        services: Vec::new(),
        inbox_messages: 0,
        ipfs_messages: 0,
        last_inbox_at_unix: None,
        last_ipfs_at_unix: None,
        startup_publish: StartupPublishStatus {
            state: "not-started".to_string(),
            mode: None,
            attempts: 0,
            max_retries: None,
            last_attempt_at_unix: None,
            next_retry_at_unix: None,
            last_success_at_unix: None,
            last_error: None,
            published_did: None,
            published_cid: None,
            source: None,
            alias: None,
            detail: None,
        },
    }))
}

pub async fn set_endpoint_metadata(
    status: &SharedWorldStatus,
    endpoint_id: String,
    services: Vec<String>,
) {
    let mut state = status.write().await;
    state.endpoint_id = Some(endpoint_id);
    state.services = services;
}

pub async fn mark_inbox(status: &SharedWorldStatus, now_unix: u64) {
    let mut state = status.write().await;
    state.inbox_messages += 1;
    state.last_inbox_at_unix = Some(now_unix);
}

pub async fn mark_ipfs(status: &SharedWorldStatus, now_unix: u64) {
    let mut state = status.write().await;
    state.ipfs_messages += 1;
    state.last_ipfs_at_unix = Some(now_unix);
}

pub async fn configure_startup_publish(
    status: &SharedWorldStatus,
    mode: &str,
    max_retries: u32,
    source: Option<String>,
    alias: Option<String>,
) {
    let mut state = status.write().await;
    state.startup_publish.state = "running".to_string();
    state.startup_publish.mode = Some(mode.to_string());
    state.startup_publish.attempts = 0;
    state.startup_publish.max_retries = Some(max_retries);
    state.startup_publish.last_attempt_at_unix = None;
    state.startup_publish.next_retry_at_unix = None;
    state.startup_publish.last_success_at_unix = None;
    state.startup_publish.last_error = None;
    state.startup_publish.published_did = None;
    state.startup_publish.published_cid = None;
    state.startup_publish.source = source;
    state.startup_publish.alias = alias;
    state.startup_publish.detail = None;
}

pub async fn mark_startup_publish_attempt(
    status: &SharedWorldStatus,
    now_unix: u64,
    attempt: u32,
) {
    let mut state = status.write().await;
    state.startup_publish.state = "running".to_string();
    state.startup_publish.attempts = attempt;
    state.startup_publish.last_attempt_at_unix = Some(now_unix);
    state.startup_publish.next_retry_at_unix = None;
}

pub async fn mark_startup_publish_retry(
    status: &SharedWorldStatus,
    error: String,
    next_retry_at_unix: u64,
) {
    let mut state = status.write().await;
    state.startup_publish.state = "retrying".to_string();
    state.startup_publish.last_error = Some(error);
    state.startup_publish.next_retry_at_unix = Some(next_retry_at_unix);
}

pub async fn mark_startup_publish_succeeded(
    status: &SharedWorldStatus,
    now_unix: u64,
    did: String,
    cid: Option<String>,
) {
    let mut state = status.write().await;
    state.startup_publish.state = "succeeded".to_string();
    state.startup_publish.last_success_at_unix = Some(now_unix);
    state.startup_publish.next_retry_at_unix = None;
    state.startup_publish.last_error = None;
    state.startup_publish.published_did = Some(did);
    state.startup_publish.published_cid = cid;
}

pub async fn mark_startup_publish_failed(status: &SharedWorldStatus, error: String) {
    let mut state = status.write().await;
    state.startup_publish.state = "failed".to_string();
    state.startup_publish.last_error = Some(error);
    state.startup_publish.next_retry_at_unix = None;
}

pub async fn mark_startup_publish_skipped(status: &SharedWorldStatus, detail: String) {
    let mut state = status.write().await;
    state.startup_publish.state = "skipped".to_string();
    state.startup_publish.detail = Some(detail);
    state.startup_publish.next_retry_at_unix = None;
}