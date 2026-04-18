use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use cid::multibase::Base;
use cid::Cid;
use crate::i18n::{Localizer, DEFAULT_LOCALE};
use libp2p_identity::Keypair;
use ma_core::{identity::generate_secret_key_file, Document};
use ma_did::generate_identity;
use ma_world_core::bundle::{encrypt_identity_bundle_json, parse_plain_identity_bundle_json, PlainIdentityBundle};
use ma_world_core::config::{ma_config_dir, write_secret_file_secure};
use rand::RngCore;
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
    unlock_passphrase_file: Option<String>,
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

    let bundle_path = config_dir.join(format!("{slug}_bundle.json"));
    let passphrase_path = config_dir.join(format!("{slug}_bundle.passphrase"));

    let identity_bundle = load_or_create_identity_bundle(&bundle_path, &passphrase_path)?;
    let doc = Document::unmarshal(
        identity_bundle
            .did_document_json
            .as_deref()
            .ok_or_else(|| anyhow!("generated identity bundle is missing did_document_json"))?,
    )
    .map_err(|err| anyhow!("invalid did document json for headless config: {err}"))?;

    let iroh_secret_path = config_dir.join(format!("{slug}_iroh.bin"));
    if !iroh_secret_path.exists() {
        generate_secret_key_file(&iroh_secret_path)
            .map_err(|err| anyhow!("generate iroh secret {}: {err}", iroh_secret_path.display()))?;
    }

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
        unlock_passphrase_file: Some(passphrase_path.display().to_string()),
        unlock_bundle_file: Some(bundle_path.display().to_string()),
    };

    let config_path = config_dir.join(format!("{slug}.yaml"));
    let yaml = serde_yaml::to_string(&config)
        .with_context(|| format!("serialize yaml for {}", config_path.display()))?;
    fs::write(&config_path, yaml)
        .with_context(|| format!("write config file {}", config_path.display()))?;

    Ok(config_path)
}

fn load_or_create_identity_bundle(bundle_path: &PathBuf, passphrase_path: &PathBuf) -> Result<PlainIdentityBundle> {
    match (bundle_path.exists(), passphrase_path.exists()) {
        (true, true) => {
            let raw = fs::read_to_string(bundle_path)
                .with_context(|| format!("read identity bundle {}", bundle_path.display()))?;
            if let Ok(bundle) = parse_plain_identity_bundle_json(&raw) {
                return Ok(bundle);
            }

            let passphrase = fs::read_to_string(passphrase_path)
                .with_context(|| format!("read bundle passphrase {}", passphrase_path.display()))?;
            return ma_world_core::bundle::decrypt_identity_bundle_json(passphrase.trim(), &raw)
                .with_context(|| format!("decrypt identity bundle {}", bundle_path.display()));
        }
        (false, false) => {}
        _ => {
            return Err(anyhow!(
                "bundle/passphrase files are inconsistent: {} {}",
                bundle_path.display(),
                passphrase_path.display()
            ));
        }
    }

    let ipns_keypair = Keypair::generate_ed25519();
    let ipns_private_key_bytes = ipns_keypair
        .to_protobuf_encoding()
        .map_err(|err| anyhow!("encode ipns private key: {err}"))?;
    let peer_id = ipns_keypair.public().to_peer_id();
    let ipns_id = Cid::new_v1(0x72, peer_id.as_ref().to_owned())
        .to_string_of_base(Base::Base36Lower)
        .map_err(|err| anyhow!("encode ipns id as base36 cidv1: {err}"))?;

    let generated_identity = generate_identity(&ipns_id)
        .map_err(|err| anyhow!("generate did document from ipns id '{}': {err}", ipns_id))?;
    let did_document_json = generated_identity
        .document
        .marshal()
        .map_err(|err| anyhow!("marshal generated did document: {err}"))?;

    let plain = PlainIdentityBundle {
        ipns_private_key_base64: B64.encode(ipns_private_key_bytes),
        signing_private_key_hex: generated_identity.signing_private_key_hex,
        encryption_private_key_hex: generated_identity.encryption_private_key_hex,
        did_document_json: Some(did_document_json),
    };

    let mut passphrase_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut passphrase_bytes);
    let passphrase = hex::encode(passphrase_bytes);
    let encrypted = encrypt_identity_bundle_json(&passphrase, &plain)?;

    write_secret_file_secure(bundle_path, encrypted.as_bytes())?;
    write_secret_file_secure(passphrase_path, passphrase.as_bytes())?;

    Ok(plain)
}
