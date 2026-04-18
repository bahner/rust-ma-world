use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::bundle::{decrypt_identity_bundle_json, parse_plain_identity_bundle_json};

#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub kubo_rpc_api: String,
    pub kubo_key_alias: Option<String>,
    pub owner: String,
    pub locale: Option<String>,
    pub iroh_secret: String,
    pub log_level: Option<String>,
    pub log_file: Option<String>,
    pub unlock_passphrase: Option<String>,
    pub unlock_passphrase_file: Option<String>,
    pub unlock_bundle_file: Option<String>,
    pub status_api_bind: String,
    pub publish_identity_on_startup: bool,
    pub identity_ipns_private_key_base64_file: Option<String>,
    pub identity_signing_private_key_hex_file: Option<String>,
    pub identity_encryption_private_key_hex_file: Option<String>,
    pub identity_document_json_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StartupIdentityMaterial {
    pub ipns_private_key_base64: String,
    pub signing_private_key_hex: String,
    pub encryption_private_key_hex: String,
    pub source: String,
}

#[derive(Debug, Deserialize)]
struct WorldConfigFile {
    kubo_rpc_api: Option<String>,
    kubo_key_alias: Option<String>,
    owner: Option<String>,
    locale: Option<String>,
    iroh_secret: Option<String>,
    log_level: Option<String>,
    log_file: Option<String>,
    unlock_passphrase: Option<String>,
    unlock_passphrase_file: Option<String>,
    unlock_bundle_file: Option<String>,
    status_api_bind: Option<String>,
    publish_identity_on_startup: Option<bool>,
    identity_ipns_private_key_base64_file: Option<String>,
    identity_signing_private_key_hex_file: Option<String>,
    identity_encryption_private_key_hex_file: Option<String>,
    identity_document_json_file: Option<String>,
}

pub fn ma_config_dir() -> PathBuf {
    #[cfg(unix)]
    {
        if let Ok(xdg_dirs) = xdg::BaseDirectories::with_prefix("ma") {
            let config_home = xdg_dirs.get_config_home();
            if !config_home.as_os_str().is_empty() {
                return config_home;
            }
        }
    }

    let base = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));

    base.join("ma")
}

pub fn write_secret_file_secure(path: &Path, content: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).truncate(false);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options
        .open(path)
        .with_context(|| format!("create secret file {}", path.display()))?;

    use std::io::Write;
    file.write_all(content)
        .with_context(|| format!("write secret file {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("finalize secret file {}", path.display()))?;

    #[cfg(windows)]
    {
        apply_windows_private_dacl(path)?;
    }

    Ok(())
}

#[cfg(windows)]
fn apply_windows_private_dacl(path: &Path) -> Result<()> {
    use std::process::Command;

    let path_arg = path.display().to_string();
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "Users".to_string());
    let user_grant = format!("{}:F", username);

    let strip_inheritance = Command::new("icacls")
        .args([&path_arg, "/inheritance:r"])
        .status()
        .with_context(|| format!("run icacls inheritance strip for {}", path.display()))?;
    if !strip_inheritance.success() {
        return Err(anyhow!(
            "icacls failed to remove inherited ACL entries for {}",
            path.display()
        ));
    }

    let grant_acl = Command::new("icacls")
        .args([&path_arg, "/grant:r", &user_grant, "SYSTEM:F", "Administrators:F"])
        .status()
        .with_context(|| format!("run icacls grant for {}", path.display()))?;
    if !grant_acl.success() {
        return Err(anyhow!(
            "icacls failed to set private ACL for {}",
            path.display()
        ));
    }

    Ok(())
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let display = path.to_string_lossy();
    if !display.starts_with("~/") {
        return path.to_path_buf();
    }

    let Some(home) = dirs::home_dir() else {
        return path.to_path_buf();
    };

    home.join(display.trim_start_matches("~/"))
}

pub fn load_world_config(slug: &str) -> Result<(WorldConfig, PathBuf)> {
    let config_dir = ma_config_dir();
    let path =
        find_slug_config_path(&config_dir, slug).unwrap_or_else(|| config_dir.join(format!("{slug}.yaml")));

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("read config file {}", path.display()))?;

    let file: WorldConfigFile = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => {
            serde_json::from_str(&raw).with_context(|| format!("parse json {}", path.display()))?
        }
        _ => serde_yaml::from_str(&raw).with_context(|| format!("parse yaml {}", path.display()))?,
    };

    let kubo_rpc_api = file
        .kubo_rpc_api
        .ok_or_else(|| anyhow!("missing kubo_rpc_api in {}", path.display()))?;
    let owner = file
        .owner
        .ok_or_else(|| anyhow!("missing owner in {}", path.display()))?;

    let iroh_secret = file
        .iroh_secret
        .or_else(|| {
            find_slug_related_file(&config_dir, slug, "iroh.bin").map(|p| p.display().to_string())
        })
        .ok_or_else(|| {
            anyhow!(
                "missing iroh_secret in {} and could not find {}_iroh.bin or {}.iroh.bin in {}",
                path.display(),
                slug,
                slug,
                config_dir.display()
            )
        })?;

    let unlock_bundle_file = file.unlock_bundle_file.or_else(|| {
        find_slug_related_file(&config_dir, slug, "bundle.json").map(|p| p.display().to_string())
    });

    let config = WorldConfig {
        kubo_rpc_api,
        kubo_key_alias: file.kubo_key_alias,
        owner,
        locale: file.locale,
        iroh_secret,
        log_level: file.log_level,
        log_file: file.log_file,
        unlock_passphrase: file.unlock_passphrase,
        unlock_passphrase_file: file.unlock_passphrase_file,
        unlock_bundle_file,
        status_api_bind: file
            .status_api_bind
            .unwrap_or_else(|| "127.0.0.1:5002".to_string()),
        publish_identity_on_startup: file.publish_identity_on_startup.unwrap_or(true),
        identity_ipns_private_key_base64_file: file.identity_ipns_private_key_base64_file,
        identity_signing_private_key_hex_file: file.identity_signing_private_key_hex_file,
        identity_encryption_private_key_hex_file: file.identity_encryption_private_key_hex_file,
        identity_document_json_file: file.identity_document_json_file,
    };

    Ok((config, path))
}

pub fn load_startup_identity_material(config: &WorldConfig) -> Result<Option<StartupIdentityMaterial>> {
    if !config.publish_identity_on_startup {
        return Ok(None);
    }

    if let (Some(ipns_key_file), Some(signing_key_file), Some(encryption_key_file)) = (
        config.identity_ipns_private_key_base64_file.as_deref(),
        config.identity_signing_private_key_hex_file.as_deref(),
        config.identity_encryption_private_key_hex_file.as_deref(),
    ) {
        let ipns_private_key_base64 = fs::read_to_string(expand_tilde(Path::new(ipns_key_file)))
            .with_context(|| format!("read identity_ipns_private_key_base64_file {ipns_key_file}"))?;
        let signing_private_key_hex = fs::read_to_string(expand_tilde(Path::new(signing_key_file)))
            .with_context(|| format!("read identity_signing_private_key_hex_file {signing_key_file}"))?;
        let encryption_private_key_hex = fs::read_to_string(expand_tilde(Path::new(encryption_key_file)))
            .with_context(|| format!("read identity_encryption_private_key_hex_file {encryption_key_file}"))?;

        return Ok(Some(StartupIdentityMaterial {
            ipns_private_key_base64: ipns_private_key_base64.trim().to_string(),
            signing_private_key_hex: signing_private_key_hex.trim().to_string(),
            encryption_private_key_hex: encryption_private_key_hex.trim().to_string(),
            source: format!("files: {ipns_key_file}, {signing_key_file}, {encryption_key_file}"),
        }));
    }

    if config.identity_document_json_file.is_some()
        && (config.identity_signing_private_key_hex_file.is_none()
            || config.identity_encryption_private_key_hex_file.is_none())
    {
        return Err(anyhow!(
            "identity_document_json_file without signing/encryption key files is deprecated; provide identity_signing_private_key_hex_file and identity_encryption_private_key_hex_file or use unlock_bundle_file"
        ));
    }

    if let Some(bundle_file) = config.unlock_bundle_file.as_deref() {
        let bundle_path = expand_tilde(Path::new(bundle_file));
        let raw = fs::read_to_string(&bundle_path)
            .with_context(|| format!("read unlock_bundle_file {}", bundle_path.display()))?;

        let file_passphrase = config
            .unlock_passphrase_file
            .as_deref()
            .map(|path| {
                fs::read_to_string(expand_tilde(Path::new(path)))
                    .with_context(|| format!("read unlock_passphrase_file {path}"))
                    .map(|value| value.trim().to_string())
            })
            .transpose()?;
        let env_passphrase = env::var("MA_WORLD_UNLOCK_PASSPHRASE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let passphrase = config
            .unlock_passphrase
            .as_deref()
            .map(str::to_string)
            .or(file_passphrase)
            .or(env_passphrase);

        if let Some(passphrase) = passphrase.as_deref() {
            let bundle = decrypt_identity_bundle_json(passphrase, &raw).with_context(|| {
                format!(
                    "decrypt unlock_bundle_file {} with configured passphrase",
                    bundle_path.display()
                )
            })?;

            return Ok(Some(StartupIdentityMaterial {
                ipns_private_key_base64: bundle.ipns_private_key_base64.trim().to_string(),
                signing_private_key_hex: bundle.signing_private_key_hex.trim().to_string(),
                encryption_private_key_hex: bundle.encryption_private_key_hex.trim().to_string(),
                source: format!("encrypted unlock_bundle_file: {}", bundle_path.display()),
            }));
        }

        if let Ok(bundle) = parse_plain_identity_bundle_json(&raw) {
            return Ok(Some(StartupIdentityMaterial {
                ipns_private_key_base64: bundle.ipns_private_key_base64.trim().to_string(),
                signing_private_key_hex: bundle.signing_private_key_hex.trim().to_string(),
                encryption_private_key_hex: bundle.encryption_private_key_hex.trim().to_string(),
                source: format!("plaintext unlock_bundle_file: {}", bundle_path.display()),
            }));
        }

        return Err(anyhow!(
            "unlock_bundle_file {} is not a supported plaintext bundle and no unlock_passphrase, unlock_passphrase_file, or MA_WORLD_UNLOCK_PASSPHRASE was provided for encrypted bundles",
            bundle_path.display()
        ));
    }

    Ok(None)
}

fn find_slug_config_path(config_dir: &Path, slug: &str) -> Option<PathBuf> {
    let preferred = [
        config_dir.join(format!("{slug}.yaml")),
        config_dir.join(format!("{slug}.yml")),
        config_dir.join(format!("{slug}.json")),
    ];
    for path in preferred {
        if path.exists() {
            return Some(path);
        }
    }

    let prefix = format!("{slug}.");
    let Ok(entries) = fs::read_dir(config_dir) else {
        return None;
    };

    let mut matches = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    matches.sort();
    matches.into_iter().next()
}

fn find_slug_related_file(config_dir: &Path, slug: &str, suffix: &str) -> Option<PathBuf> {
    let candidates = [
        config_dir.join(format!("{slug}_{suffix}")),
        config_dir.join(format!("{slug}.{suffix}")),
        config_dir.join(format!("{slug}-{suffix}")),
    ];

    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }

    None
}
