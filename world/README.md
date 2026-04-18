# ma-world

First runnable world slice for ma-realms.

Current behavior:

- starts an iroh-backed endpoint using `ma-core`
- registers `/ma/inbox/0.0.1` and `/ma/ipfs/0.0.1`
- logs inbox messages to console and file
- handles `/ma/ipfs/0.0.1` requests through `ma-world-core`
- emits a machine-readable reply payload to logs using `application/x-ipfs-request-reply`
- exposes local status API on `127.0.0.1:5002` by default (`/status.json`)
- attempts startup identity publish to Kubo/IPNS when identity material is available

Workspace note:

- reusable IPFS request validation/publish logic lives in `ma-world-core`
- reusable world config loading + startup identity material resolution live in `ma-world-core`
- `ma-world` focuses on CLI commands and runtime wiring/logging

Note:

- network transport for sending reply messages is not wired yet in this first slice
- reply payload is generated and logged with `reply_to = request.id`

## Build

From workspace root:

```bash
cargo check -p ma-world
cargo build -p ma-world --release
```

Run with:

```bash
cargo run -p ma-world -- --slug <slug>
```

Example:

```bash
cargo run -p ma-world -- --slug panteia
```

Defaults:

- slug defaults to `world` when `--slug` is omitted
- status API bind defaults to `127.0.0.1:5002`
- bind can be overridden with `--status-api-bind <host:port>`

Status API:

- `GET /status.json` returns runtime status counters and service metadata

## Config

Config path:

- `${XDG_CONFIG_HOME}/ma/<slug>.yaml` (preferred)
- fallback: `~/.config/ma/<slug>.yaml`
- or first match for `${XDG_CONFIG_HOME}/ma/<slug>.*`

Config loading and startup identity material resolution are implemented in `ma-world-core::config` and consumed by the world binary.

Related slug files auto-detected (when unset in config):

- `${XDG_CONFIG_HOME}/ma/<slug>_iroh.bin` or `${XDG_CONFIG_HOME}/ma/<slug>.iroh.bin`
- `${XDG_CONFIG_HOME}/ma/<slug>_bundle.json` or `${XDG_CONFIG_HOME}/ma/<slug>.bundle.json`

Startup identity publish:

- `publish_identity_on_startup` (default: `true`)
- `identity_document_json_file` + `identity_ipns_private_key_base64_file` (recommended)
- encrypted bundle via `unlock_bundle_file` + `unlock_passphrase` (new ma-world-core format)
- plaintext bundle fallback via `unlock_bundle_file` with:
	- `did_document_json`
	- `ipns_private_key_base64`

Encrypted bundle JSON shape (version 2):

- `version`: `2`
- `kdf`: `argon2id`
- `cipher`: `xchacha20poly1305`
- `salt_b64`
- `nonce_b64`
- `ciphertext_b64`

Example config:

```yaml
kubo_rpc_api: http://localhost:5001
owner: did:ma:k51qzi5uqu5dglxrtfnvh2tx1wddufnscvi50zm90wyll9i3k0bkoofrb35uoc#bahner
iroh_secret: /home/lars/.config/ma/panteia_iroh.bin
log_level: info
log_file: ~/.local/share/ma/worlds/panteia/panteia.log
unlock_passphrase: example-passphrase
unlock_bundle_file: /home/lars/.config/ma/panteia_bundle.json
```

Required now:

- `kubo_rpc_api`
- `owner`
- `iroh_secret`

Optional now:

- `log_level`
- `log_file`
- `unlock_passphrase` (reserved)
- `unlock_bundle_file` (reserved)
