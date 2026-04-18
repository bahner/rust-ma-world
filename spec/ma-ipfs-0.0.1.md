# /ma/ipfs/0.0.1

Status: draft

Purpose: allow a trusted world to publish a signed did:ma document to a locally configured Kubo node on behalf of a remote actor that cannot access Kubo directly.

## 1. Scope

The `/ma/ipfs/0.0.1` service is a privileged publish service.

It exists for one reason: an actor, especially a browser-based actor, may hold a valid `did:ma:` identity and a corresponding IPNS private key, but it may not have direct access to a trusted local Kubo API. A world may expose `/ma/ipfs/0.0.1` and act as a trusted publisher for that actor.

This service only covers publication of a signed DID document to IPFS/IPNS.

Pinning of the published DID document in IPFS is recommended by this version of the protocol, but not required for interoperability.

This service does not define:

- general file pinning
- arbitrary content publication
- identity creation
- key generation
- key escrow semantics beyond the request lifetime

## 2. Service Identifier

- Protocol identifier: `/ma/ipfs/0.0.1`
- Request content type: `application/x-ma-ipfs-request`
- Reply content type: `application/x-ma-ipfs-request-reply`
- Embedded DID document media type: `application/vnd.ipld.dag-cbor`

The service is intended to be exposed as a named service on a `did:ma:` endpoint.

## 3. Trust Model

The caller gives the service a secret capable of publishing under the caller's IPNS name.

Because of that, the service is inherently trust-sensitive.

An implementation of `/ma/ipfs/0.0.1`:

- MUST be treated as a trusted home or world service
- MUST NOT persist the submitted IPNS private key beyond what is required to complete the publish operation
- MUST protect request material from logs, metrics, panic output, and other accidental disclosure paths
- SHOULD be explicit in configuration and UI that enabling this service grants publish authority

## 4. Transport Envelope

Requests to `/ma/ipfs/0.0.1` are sent as signed ma messages.

The outer message:

- MUST be a valid signed ma message encoded in the normal ma wire format
- MUST have `content_type = application/x-ma-ipfs-request`
- MUST be signed by the same identity namespace as the DID document being published

The world validates the outer signed message before attempting any publish.

Replies from this service are also normal signed ma messages.

A reply from this service:

- MUST be a new message with its own unique `id`
- MUST set `reply_to` to the `id` of the request message being answered
- MUST have `content_type = application/x-ma-ipfs-request-reply`
- SHOULD be sent to the sender's `currentInbox` if that can be resolved from the sender document
- SHOULD otherwise be sent to the sender's `/ma/inbox/0.0.1`

## 5. Request Body

The message body for `application/x-ma-ipfs-request` is JSON with this shape:

```json
{
  "did_document_dag_cbor_base64": "BASE64-ENCODED-DAG-CBOR-BYTES",
  "ipns_private_key_base64": "BASE64-ENCODED-PRIVATE-KEY"
}
```

Field definitions:

- `did_document_dag_cbor_base64`: a signed `did:ma:` DID document encoded as DAG-CBOR bytes and then base64 encoded
- `ipns_private_key_base64`: the caller's IPNS private key, base64 encoded

The DID document represented by `did_document_dag_cbor_base64` is logically of media type `application/vnd.ipld.dag-cbor`, even though it is embedded here as a base64 string field in JSON.

## 6. Validation Rules

An implementation MUST reject the request unless all of the following are true:

1. The outer message can be decoded as a valid signed ma message.
2. The outer message content type is exactly `application/x-ma-ipfs-request`.
3. The message body parses as the request object defined above.
4. `did_document_dag_cbor_base64` decodes from base64 and parses as a valid DAG-CBOR DID document.
5. The DID document passes structural validation.
6. The DID document signature verifies successfully.
7. The sender DID in the outer message and the DID document refer to the same IPNS identity.
8. The outer message signature verifies against the DID document provided in the request.
9. `ipns_private_key_base64` is present and not blank.
10. The decoded private key imports successfully into the configured Kubo node.
11. The imported key resolves to the same IPNS identifier as the DID document subject.

Validation rule 7 is critical: a caller may only ask the world to publish a DID document for the same IPNS identity that signed the request.

## 7. Kubo Key Alias Derivation

When importing the caller's IPNS key into the local Kubo node for publishing, the implementation MUST use a deterministic, collision-resistant alias derived from the caller's IPNS identity. A human-readable vanity alias (such as a slug or username) MUST NOT be used for keys submitted through this protocol.

The REQUIRED derivation is:

```
alias = "ma-" || lower_hex(blake3(ipns_id))[0..16]
```

> **Note:** The reference implementation in `ma-core` prior to 0.6.0 uses `_ma_` as the prefix. As of `ma-core` 0.6.0 the canonical `ma-` prefix is exposed via the `MA_IPNS_ALIAS_HASH_PREFIX` constant and implementations MUST use that constant rather than a hardcoded string.

Where:

- `ipns_id` is the IPNS identifier from the DID document subject (the `did:ma:` key component, which is the IPNS public key multibase string), encoded as UTF-8 bytes
- `blake3(...)` is the BLAKE3 hash of those bytes
- `[0..16]` takes the first 16 hex characters (8 bytes) of the hash
- The result is prefixed with `ma-` to namespace it within Kubo and avoid collisions with user-created keys

Example: for the IPNS id `k51qzi5uqu5di1d2dhwk9o98d96j9q389u3gmpo91g6knh5tnl4t7ph0fngt0e`, the alias would be `ma-` followed by the first 16 hex characters of its BLAKE3 hash.

This scheme ensures:

- The same identity always maps to the same key alias (idempotent, no cleanup required after repeated publishes)
- Imported keys are namespaced and distinguishable from world-owned keys (e.g. slug-based aliases like `world`)
- Alias collisions across distinct identities are computationally infeasible
- Repeated publishes for the same identity reuse the existing Kubo key entry rather than creating a new one, preventing resource exhaustion in Kubo's key store from repeated or adversarial publish requests

The world SHOULD NOT expose the derived alias to the caller. It is an internal implementation detail of the publish step.

Worlds MAY configure their own IPNS key using a vanity slug alias (e.g. matching the world's slug). That slug-based alias is a separate key used only for the world's own identity, not for caller-submitted keys.

## 8. Publish Semantics, the implementation performs these logical steps:

1. Decode the submitted base64 IPNS private key.
2. Import the key into the configured Kubo node.
3. Store the DID document in IPFS and obtain a CID.
4. Optionally pin that CID in IPFS.
5. Publish that CID to IPNS using the imported key.

The exact Kubo RPC sequence is implementation-defined, but the observable outcome MUST be equivalent to:

- storing the supplied DID document in IPFS
- publishing that CID under the caller's IPNS name

Implementations SHOULD pin the resulting CID locally where operationally appropriate.

Implementations SHOULD make repeated publishes idempotent for the same identity where practical.

## 9. Reply Message

This service does not define a separate RPC service.

Instead, the world replies, when practical, by sending a normal ma message with `reply_to` set to the request message `id`.

The sender is RECOMMENDED to watch for this reply and report the result locally.

Reply routing order is:

1. the sender's `currentInbox`, if specified and resolvable from the sender document
2. otherwise the sender's `/ma/inbox/0.0.1`

If neither destination can be resolved, the world MAY omit the reply and SHOULD log the local failure.

## 10. Reply Body

The reply body for `application/x-ma-ipfs-request-reply` is JSON with this shape:

```json
{
  "status": 200,
  "code": "ok",
  "message": "did document published via ma/ipfs/0.0.1",
  "did": "did:ma:...",
  "cid": "bafy..."
}
```

Reply fields:

- `status`: machine-readable status code
- `code`: machine-readable symbolic result code
- `message`: human-readable status text
- `upstream_detail`: optional diagnostics from implementation-specific upstream systems (for example Kubo)
- `did`: optional DID string for the published document subject
- `cid`: CID of the stored DID document (required on success)

Status code meanings for version `0.0.1`:

- `200`: request validated and DID document published successfully
- `400`: malformed request message or malformed request body
- `401`: sender could not be authenticated from the supplied document and signature material
- `403`: request was understood but rejected by local policy
- `409`: request conflicts with the submitted identity or imported key material
- `422`: request is syntactically valid but the DID document or key material is semantically invalid
- `500`: internal processing failure
- `503`: dependent local service, such as Kubo, is unavailable or failed the publish operation

Recommended reply codes for version `0.0.1`:

- `ok`
- `bad-request`
- `auth-failed`
- `forbidden`
- `identity-conflict`
- `invalid-document`
- `invalid-key`
- `publish-failed`
- `service-unavailable`
- `internal-error`

Failure replies MUST keep the same object shape:

```json
{
  "status": 422,
  "code": "invalid-document",
  "message": "reason for rejection or publish failure"
}
```

On failure, `did` and `cid` MAY be omitted.

On success (`status = 200` and `code = ok`), `cid` MUST be present.

The `status` and `code` fields are the machine-readable result. The `message` field is diagnostic and intended for operators and user interfaces.

The `status` and `code` fields are normative in this specification.

Implementations MUST map upstream failures (including Kubo errors) into the stable `status`/`code` space defined here.

The optional `upstream_detail` field MAY carry extra diagnostics for troubleshooting, but senders and clients MUST NOT rely on that field for control flow.

## 10. Errors

The following conditions MUST produce failure:

- malformed signed message
- wrong content type
- malformed request JSON
- malformed or invalid DID document
- DID document signature failure
- request signature failure
- mismatch between sender IPNS identity and document IPNS identity
- missing or undecodable private key material
- mismatch between imported Kubo key and DID subject identity
- Kubo import, DAG put, or IPNS publish failure

Implementations SHOULD return a diagnostic `message` suitable for local debugging, but MUST avoid echoing private key material or full sensitive payloads.

When practical, implementations SHOULD send a reply message carrying one of the error replies defined above.

The following mapping guidance applies:

- malformed request framing and parse failures SHOULD map to `400/bad-request`
- signature/authentication failures SHOULD map to `401/auth-failed`
- local policy rejection SHOULD map to `403/forbidden`
- identity mismatch failures SHOULD map to `409/identity-conflict`
- invalid DID document SHOULD map to `422/invalid-document`
- invalid or missing key material SHOULD map to `422/invalid-key`
- upstream transport unavailability SHOULD map to `503/service-unavailable`
- publish operation errors with available upstream SHOULD map to `500/publish-failed`
- unclassified failures SHOULD map to `500/internal-error`

## 11. Logging and Secret Handling

Because this protocol carries secret material, implementations:

- MUST NOT log `ipns_private_key_base64`
- MUST NOT log decoded private key bytes
- SHOULD avoid logging full request bodies
- SHOULD log only request metadata such as sender DID, target DID, outcome, and resulting CID
- SHOULD erase in-memory secret buffers as far as practical within the implementation language and libraries

## 12. Relationship to Worlds

A world exposing `/ma/ipfs/0.0.1` is expected to:

- run against a locally trusted Kubo RPC endpoint
- publish on behalf of actors that trust that world as home
- expose a normal `/ma/inbox/0.0.1` service alongside this one

The presence of `/ma/ipfs/0.0.1` on a world means the world is willing to accept publish requests, not that every request will be accepted.

Worlds MAY add local authorization rules on top of this specification. For example, a world may choose to only serve known actors or owners. Such authorization is implementation-specific and outside this version of the wire spec.

## 13. Minimal Interoperability Profile

For interoperability in this repository, a conforming implementation of `/ma/ipfs/0.0.1` MUST at minimum:

- accept a signed ma message with `application/x-ma-ipfs-request`
- parse the request JSON fields `did_document_dag_cbor_base64` and `ipns_private_key_base64`
- validate the DID document and request signature as described above
- ensure sender and document share the same IPNS identity
- publish the DID document to the configured Kubo node
- when sending a reply, send a signed ma message with `reply_to` equal to the request `id`
- when sending a reply, use `application/x-ma-ipfs-request-reply`
- when sending a reply, include machine-readable `status` and `code` fields

Pinning is RECOMMENDED for implementations in this repository, but is not part of the minimal interoperability requirement for version `0.0.1`.

The sender is RECOMMENDED to watch for such replies and surface them locally.

## 14. Open Questions for Later Versions

The following topics are intentionally deferred from version `0.0.1`:

- whether actor-to-home authorization should be standardized
- whether key material can be replaced with a delegated capability or one-shot publish token
- whether document payloads should later move from embedded JSON string to a more explicit typed envelope
- whether `currentInbox` needs a more tightly defined DID document encoding
- whether pin retention lifetime, replacement semantics, or garbage-collection behavior should be specified more precisely

These should be handled in future specs without breaking the minimal publish flow defined here.