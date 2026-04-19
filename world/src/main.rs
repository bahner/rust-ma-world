use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use api::run_status_api;
use config::{generate_headless_config, parse_args};
use i18n::Localizer;
use ma_core::{
    identity::load_secret_key_bytes, Did, IrohEndpoint, MaEndpoint, Message, INBOX_PROTOCOL,
    IPFS_PROTOCOL,
};
use ma_world_core::{
    config::{expand_tilde, load_startup_identity_material, load_world_config, WorldConfig},
    ensure_kubo_key_alias, generate_world_did_document_ephemeral,
    generate_world_did_document_from_keys, handle_ipfs_publish_message, IpfsRequestReply,
    IPFS_REPLY_CONTENT_TYPE,
};
use startup_publish::{retry_publish_identity, retry_publish_identity_alias};
use status::{
    configure_startup_publish, mark_inbox, mark_ipfs, mark_startup_publish_skipped,
    new_shared_status, set_endpoint_metadata,
};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

mod api;
mod config;
mod i18n;
mod startup_publish;
mod status;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_args()?;
    let boot_i18n = Localizer::new(None)?;

    if cli.gen_headless_config {
        let config_path = generate_headless_config(&cli).await?;
        println!(
            "{}",
            boot_i18n.generated_headless_config(&config_path.display().to_string())
        );
        return Ok(());
    }

    let slug = cli.slug;

    let (mut config, config_path) = load_world_config(&slug)?;
    if let Some(bind) = cli.status_api_bind {
        config.status_api_bind = bind;
    }

    let _log_guard = init_logging(&slug, &config)?;
    let i18n = Localizer::new(config.locale.as_deref())?;
    info!(slug = %slug, config = %config_path.display(), "world startup");
    info!(owner = %config.owner, "configured owner");
    info!(bind = %config.status_api_bind, "status api configured");
    info!(locale = %i18n.locale(), "console locale configured");

    if config.unlock_bundle_file.is_some() || config.unlock_passphrase.is_some() {
        info!("unlock bundle/passphrase found in config (not used yet in this first slice)");
    }

    let owner_did = Did::try_from(config.owner.as_str())
        .map_err(|err| anyhow!("invalid owner did '{}' in config: {err}", config.owner))?;
    let ipns_endpoint = format!("/ipns/{}", owner_did.ipns);

    let iroh_secret = expand_tilde(Path::new(&config.iroh_secret));
    let secret = load_secret_key_bytes(&iroh_secret)
        .with_context(|| format!("load iroh secret from {}", iroh_secret.display()))?
        .ok_or_else(|| anyhow!("missing iroh secret file: {}", iroh_secret.display()))?;

    let mut endpoint = IrohEndpoint::new(secret).await?;
    info!(
        target: "ma_event",
        "router booted: endpoint={}", endpoint.id()
    );

    let mut inbox_messages = endpoint.service(INBOX_PROTOCOL_STR);
    info!(
        target: "ma_event",
        "router service attached: {}", INBOX_PROTOCOL_STR
    );

    let mut ipfs_messages = endpoint.service(IPFS_PROTOCOL_STR);
    info!(
        target: "ma_event",
        "router service attached: {}", IPFS_PROTOCOL_STR
    );

    endpoint.start_router();
    info!(
        target: "ma_event",
        "router listener started"
    );

    info!(endpoint_id = %endpoint.id(), "endpoint started");
    info!(services = ?endpoint.services(), "registered services");
    info!(
        target: "ma_event",
        "{}",
        i18n.world_online(&config.owner, &endpoint.id().to_string(), &endpoint.services().join(","))
    );

    let status = new_shared_status(
        slug.clone(),
        config.owner.clone(),
        config_path.display().to_string(),
        config.status_api_bind.clone(),
        Some(ipns_endpoint),
        now_unix_secs(),
    );

    // Spawn background task for identity publication (non-blocking startup).
    let publish_task = if let Some(identity) = load_startup_identity_material(&config)? {
        configure_startup_publish(
            &status,
            "identity-material",
            10,
            Some(identity.source.clone()),
            None,
        )
        .await;

        let did_document_json = generate_world_did_document_from_keys(
            &owner_did.ipns,
            &endpoint,
            &identity.signing_private_key_hex,
            &identity.encryption_private_key_hex,
        )
        .with_context(|| format!("generate startup did document for '{}'", owner_did.ipns))?;

        let kubo_rpc_api = config.kubo_rpc_api.clone();
        let ipns_id = owner_did.ipns.clone();
        let ipns_key_base64 = identity.ipns_private_key_base64.clone();
        let source_label = identity.source.clone();
        let publish_status = status.clone();

        Some(tokio::spawn(async move {
            match retry_publish_identity(
                publish_status,
                &kubo_rpc_api,
                &ipns_id,
                &did_document_json,
                &ipns_key_base64,
                10,
            )
            .await
            {
                Ok(published_did) => {
                    info!(
                        target: "ma_event",
                        "identity published: {} from source: {}",
                        published_did,
                        source_label
                    );
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        source = %source_label,
                        "startup identity publish failed after retries"
                    );
                }
            }
        }))
    } else if let Some(alias) = config.kubo_key_alias.as_deref() {
        configure_startup_publish(&status, "kubo-key-alias", 10, None, Some(alias.to_string()))
            .await;

        let alias_key = ensure_kubo_key_alias(&config.kubo_rpc_api, alias)
            .await
            .with_context(|| format!("ensure kubo key alias '{}'", alias))?;

        let did_document_json = generate_world_did_document_ephemeral(&alias_key.id, &endpoint)
            .with_context(|| format!("marshal startup did document for alias '{}'", alias))?;

        let kubo_rpc_api = config.kubo_rpc_api.clone();
        let alias_name = alias.to_string();
        let publish_status = status.clone();

        Some(tokio::spawn(async move {
            match retry_publish_identity_alias(
                publish_status,
                &kubo_rpc_api,
                &alias_name,
                &did_document_json,
                10,
            )
            .await
            {
                Ok((did, cid)) => {
                    info!(
                        target: "ma_event",
                        "identity published: did={} cid={} alias={}",
                        did,
                        cid,
                        alias_name
                    );
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        alias = %alias_name,
                        "alias identity publish failed after retries"
                    );
                }
            }
        }))
    } else {
        warn!("startup identity publish skipped: no identity material found and no kubo_key_alias configured");
        mark_startup_publish_skipped(
            &status,
            "no identity material found and no kubo_key_alias configured".to_string(),
        )
        .await;
        None
    };
    drop(publish_task); // Background task runs independently

    let mut status_api_task = tokio::spawn(run_status_api(
        config.status_api_bind.clone(),
        status.clone(),
    ));

    set_endpoint_metadata(&status, endpoint.id().to_string(), endpoint.services()).await;

    let mut ticker = tokio::time::interval(Duration::from_millis(100));
    let mut heartbeat_at = Instant::now();
    let mut inbox_total: u64 = 0;
    let mut ipfs_total: u64 = 0;

    info!(
        target: "ma_event",
        "router loop active: polling every {}ms", 100
    );

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                for message in drain_messages(&mut inbox_messages) {
                    inbox_total += 1;
                    mark_inbox(&status, now_unix_secs()).await;
                    log_inbox_message(&message, &i18n);
                    log_trace_message(INBOX_PROTOCOL_STR, &message);
                }

                for message in drain_messages(&mut ipfs_messages) {
                    ipfs_total += 1;
                    mark_ipfs(&status, now_unix_secs()).await;
                    log_trace_message(IPFS_PROTOCOL_STR, &message);
                    handle_ipfs_message(&config, &message, &i18n).await;
                }

                if heartbeat_at.elapsed() >= Duration::from_secs(5) {
                    debug!(
                        endpoint_id = %endpoint.id(),
                        inbox_total = inbox_total,
                        ipfs_total = ipfs_total,
                        "router heartbeat"
                    );
                    heartbeat_at = Instant::now();
                }
            }
            status_result = &mut status_api_task => {
                match status_result {
                    Ok(Ok(())) => {
                        warn!("status api exited");
                    }
                    Ok(Err(err)) => {
                        error!(error = %err, "status api failed");
                    }
                    Err(err) => {
                        error!(error = %err, "status api task join failure");
                    }
                }
                break;
            }
            signal = tokio::signal::ctrl_c() => {
                if let Err(err) = signal {
                    error!(error = %err, "ctrl-c handler failed");
                }
                info!("shutdown requested");
                status_api_task.abort();
                break;
            }
        }
    }

    Ok(())
}

const INBOX_PROTOCOL_STR: &str = "/ma/inbox/0.0.1";
const IPFS_PROTOCOL_STR: &str = "/ma/ipfs/0.0.1";

fn init_logging(
    slug: &str,
    config: &WorldConfig,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let level = config.log_level.as_deref().unwrap_or("info");
    let file_filter = tracing_subscriber::EnvFilter::try_new(level)
        .with_context(|| format!("invalid log level/filter: {level}"))?;

    let log_path = config
        .log_file
        .as_deref()
        .map(|path| expand_tilde(Path::new(path)))
        .unwrap_or_else(|| {
            let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            base.join(".local")
                .join("share")
                .join("ma")
                .join("worlds")
                .join(slug)
                .join(format!("{slug}.log"))
        });

    let parent = log_path
        .parent()
        .ok_or_else(|| anyhow!("invalid log_file path: {}", log_path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create log directory {}", parent.display()))?;

    let file_name = log_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("invalid log file name in {}", log_path.display()))?;

    let appender = tracing_appender::rolling::never(parent, file_name);
    let (file_writer, guard) = tracing_appender::non_blocking(appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .compact()
        .without_time()
        .with_level(false)
        .with_target(false)
        .with_writer(std::io::stdout)
        .with_filter(
            tracing_subscriber::filter::Targets::new().with_target("ma_event", LevelFilter::TRACE),
        );

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(file_writer);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer.with_filter(file_filter))
        .init();

    Ok(guard)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn drain_messages(inbox: &mut ma_core::Inbox<Message>) -> Vec<Message> {
    let now = now_unix_secs();
    let mut messages = Vec::new();
    while let Some(message) = inbox.pop(now) {
        messages.push(message);
    }
    messages
}

fn log_inbox_message(message: &Message, i18n: &Localizer) {
    info!(
        target: "ma_event",
        "{}",
        i18n.inbox_received(&message.from, &message.to, &message.content_type, &message.id)
    );
    info!(
        protocol = INBOX_PROTOCOL,
        message_id = %message.id,
        from = %message.from,
        to = %message.to,
        content_type = %message.content_type,
        payload_len = message.content.len(),
        "received inbox message"
    );
}

fn log_trace_message(protocol: &str, message: &Message) {
    let payload_hex = hex::encode(&message.content);
    let payload_utf8 = String::from_utf8_lossy(&message.content);

    trace!(
        protocol = protocol,
        message_id = %message.id,
        from = %message.from,
        to = %message.to,
        content_type = %message.content_type,
        payload_len = message.content.len(),
        payload_hex = %payload_hex,
        payload_utf8 = %payload_utf8,
        "received ma message payload"
    );
}

async fn handle_ipfs_message(config: &WorldConfig, message: &Message, i18n: &Localizer) {
    info!(
        target: "ma_event",
        "{}",
        i18n.ipfs_received(&message.from, &message.to, &message.content_type, &message.id)
    );
    info!(
        protocol = IPFS_PROTOCOL,
        message_id = %message.id,
        from = %message.from,
        content_type = %message.content_type,
        "received ipfs publish message"
    );

    let reply = handle_ipfs_publish_message(&config.kubo_rpc_api, message).await;
    log_ipfs_reply(message, &reply, i18n);
}

fn log_ipfs_reply(request: &Message, reply: &IpfsRequestReply, i18n: &Localizer) {
    match serde_json::to_string(reply) {
        Ok(reply_json) => {
            info!(
                target: "ma_event",
                "{}",
                i18n.ipfs_reply(
                    &request.from,
                    reply.status,
                    reply.code,
                    &request.id,
                    IPFS_REPLY_CONTENT_TYPE,
                )
            );
            info!(
                message_id = %request.id,
                reply_to = %request.id,
                reply_content_type = IPFS_REPLY_CONTENT_TYPE,
                to = %request.from,
                status = reply.status,
                code = reply.code,
                reply = %reply_json,
                "ipfs request processed"
            );
            warn!(
                "reply transport is not wired yet in this first slice; reply payload is emitted to logs"
            );
        }
        Err(err) => {
            error!(error = %err, "failed to serialize ipfs reply payload");
        }
    }
}
