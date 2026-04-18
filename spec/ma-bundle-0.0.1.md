# ma.bundle/0.0.1

Status: draft

Purpose: define a portable encrypted bundle format for local world bootstrap data.

This specification describes the bundle created by `ma-world --gen-headless-config` and consumed during startup when a world needs to publish its DID document to Kubo/IPNS.

## 1. Scope

`ma.bundle/0.0.1` defines:

- plaintext payload shape
- encrypted JSON envelope shape
- required KDF and cipher parameters
- validation rules for readers

This version is for local trusted storage and bootstrap flow.

This version does not define:

- remote transport of bundle files
- key escrow or recovery flows
- bundle signing
- multi-identity bundle sets

## 2. Plaintext Payload

The decrypted payload is JSON with this shape:

```json
{
  "did_document_json": "{...DID Document JSON...}",
  "ipns_private_key_base64": "BASE64-ENCODED-IPNS-PRIVATE-KEY-BYTES"
}
```

Field definitions:

- `did_document_json`: UTF-8 JSON string for the DID document to publish
- `ipns_private_key_base64`: base64 string for the raw IPNS private key bytes

## 3. Encrypted Bundle Object

The encrypted file is JSON with this shape:

```json
{
  "version": 2,
  "kdf": "argon2id",
  "cipher": "xchacha20poly1305",
  "salt_b64": "...",
  "nonce_b64": "...",
  "ciphertext_b64": "..."
}
```

Field definitions:

- `version`: integer format version. This specification requires `2`.
- `kdf`: key derivation algorithm. This specification requires `argon2id`.
- `cipher`: AEAD algorithm. This specification requires `xchacha20poly1305`.
- `salt_b64`: base64-encoded KDF salt bytes.
- `nonce_b64`: base64-encoded XChaCha20-Poly1305 nonce bytes.
- `ciphertext_b64`: base64-encoded AEAD ciphertext including authentication tag.

## 4. Cryptographic Parameters

This version is fully parameterized as follows:

- KDF: Argon2id
- Argon2 memory cost: 65536 KiB
- Argon2 time cost: 3
- Argon2 parallelism: 1
- Derived key length: 32 bytes
- Salt length: 16 bytes
- Cipher: XChaCha20-Poly1305
- Nonce length: 24 bytes
- AAD (associated data): `ma-world-core/bundle:v2`

## 5. Encoding Rules

For `*_b64` fields:

- Base64 alphabet: RFC 4648 standard alphabet
- Padding: standard base64 padding is allowed and expected
- Input must decode cleanly to the required byte lengths

Bundle files MUST be UTF-8 JSON.

## 6. Decryption Validation Rules

A conforming reader MUST reject a bundle unless all of the following hold:

1. JSON parses successfully.
2. `version` equals `2`.
3. `kdf` equals `argon2id`.
4. `cipher` equals `xchacha20poly1305`.
5. `salt_b64` decodes to exactly 16 bytes.
6. `nonce_b64` decodes to exactly 24 bytes.
7. `ciphertext_b64` decodes successfully.
8. AEAD authentication succeeds with AAD `ma-world-core/bundle:v2`.
9. Decrypted payload parses as plaintext payload object.
10. `did_document_json` is non-empty.
11. `ipns_private_key_base64` is non-empty.

## 7. Generator Behavior

`ma-world --gen-headless-config` SHOULD:

1. produce an encrypted bundle object conforming to this specification
2. generate a fresh random salt and nonce per bundle
3. store the encrypted bundle in `${XDG_CONFIG_HOME}/ma/<slug>_bundle.json`
4. write `unlock_bundle_file` and `unlock_passphrase` into `${XDG_CONFIG_HOME}/ma/<slug>.yaml`

## 8. Startup Usage

World startup logic MAY read `unlock_bundle_file` and decrypt it with `unlock_passphrase`.

When successful, the world obtains:

- DID document JSON to publish
- IPNS private key material for Kubo publish

## 9. Security Notes

Implementations:

- MUST NOT log decrypted key material
- MUST NOT log passphrases
- SHOULD avoid logging full decrypted payload
- SHOULD keep decrypted data in memory only as long as needed
- SHOULD use OS file permissions appropriate for local secret files

## 10. Versioning

Future versions MAY change KDF/cipher parameters or payload fields.

Readers MUST treat unknown `version` values as unsupported.
