use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use crate::i18n::{Localizer, DEFAULT_LOCALE};
use ma_core::{identity::generate_secret_key_file, Document};
use ma_did::generate_identity;
use ma_world_core::config::ma_config_dir;
use ma_world_core::kubo::ensure_kubo_key_alias;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub slug: String,
    pub status_api_bind: Option<String>,
    pub gen_headless_config: bool,
    pub kubo_rpc_api: Option<String>,
}

#[derive(Debug, Serialize)]
struct HeadlessConfigFile {
    kubo_rpc_api: String,
    kubo_key_alias: String,
    owner: String,
    locale: String,
    iroh_secret: String,
    log_level: String,
    log_file: String,
    status_api_bind: String,
    publish_identity_on_startup: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    unlock_passphrase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unlock_bundle_file: Option<String>,
}

pub fn parse_args() -> Result<CliArgs> {
    let i18n = Localizer::new(None)?;
    let mut slug = "world".to_string();
    let mut status_api_bind = None;
    let mut gen_headless_config = false;
    let mut kubo_rpc_api = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--slug" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!(i18n.cli_missing_value("--slug")))?;
                slug = value;
            }
            "--status-api-bind" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!(i18n.cli_missing_value("--status-api-bind")))?;
                status_api_bind = Some(value);
            }
            "--gen-headles-config" | "--gen-headless-config" => {
                gen_headless_config = true;
            }
            "--kubo-rpc-api" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!(i18n.cli_missing_value("--kubo-rpc-api")))?;
                kubo_rpc_api = Some(value);
            }
            "-h" | "--help" => {
                println!("{}", i18n.cli_usage());
                std::process::exit(0);
            }
            value if !value.starts_with('-') => {
                slug = value.to_string();
            }
            _ => {
                return Err(anyhow!(i18n.cli_unknown_argument(&arg)));
            }
        }
    }

    Ok(CliArgs {
        slug,
        status_api_bind,
        gen_headless_config,
        kubo_rpc_api,
    })
}

pub async fn generate_headless_config(cli: &CliArgs) -> Result<PathBuf> {
    let config_dir = ma_config_dir();
    fs::create_dir_all(&config_dir)
        .with_context(|| format!("create config dir {}", config_dir.display()))?;

    let slug = cli.slug.trim();
    if slug.is_empty() {
        return Err(anyhow!("slug must not be empty"));
    }

    let kubo_rpc_api = cli
        .kubo_rpc_api
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:5001".to_string());
    let kubo_key_alias = slug.to_string();

    let kubo_key = ensure_kubo_key_alias(&kubo_rpc_api, &kubo_key_alias).await?;
    let generated_identity = generate_identity(&kubo_key.id)
        .map_err(|err| anyhow!("generate did document from ipns id '{}': {err}", kubo_key.id))?;
    let did_document_json = generated_identity
        .document
        .marshal()
        .map_err(|err| anyhow!("marshal generated did document: {err}"))?;

    {
    }

    let iroh_secret_path = config_dir.join(format!("{slug}_iroh.bin"));
    if !iroh_secret_path.exists() {
        generate_secret_key_file(&iroh_secret_path)
            .map_err(|err| anyhow!("generate iroh secret {}: {err}", iroh_secret_path.display()))?;
    }

    let doc = Document::unmarshal(&did_document_json)
        .map_err(|err| anyhow!("invalid did document json for headless config: {err}"))?;
    let owner = format!("{}#world", doc.id);

    let config = HeadlessConfigFile {
        kubo_rpc_api,
        kubo_key_alias,
        owner,
        locale: DEFAULT_LOCALE.to_string(),
        iroh_secret: iroh_secret_path.display().to_string(),
        log_level: "info".to_string(),
        log_file: format!("~/.local/share/ma/worlds/{slug}/{slug}.log"),
        status_api_bind: "127.0.0.1:5002".to_string(),
        publish_identity_on_startup: true,
        unlock_passphrase: None,
        unlock_bundle_file: None,
    };

    let config_path = config_dir.join(format!("{slug}.yaml"));
    let yaml = serde_yaml::to_string(&config)
        .with_context(|| format!("serialize yaml for {}", config_path.display()))?;
    fs::write(&config_path, yaml)
        .with_context(|| format!("write config file {}", config_path.display()))?;

    Ok(config_path)
}
