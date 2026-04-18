# ma-world-core

Shared core logic for ma-world services.

Current scope:

- `/ma/ipfs/0.0.1` request validation
- DID/message signature checks
- publish flow against configured Kubo RPC
- normalized machine-readable reply payload mapping
- encrypted identity bundle support (encrypt/decrypt)
- world config loading and startup identity material resolution

Module layout:

- `src/kubo.rs`: Kubo/IPNS publish helpers
- `src/bundle.rs`: identity bundle encryption/decryption
- `src/ipfs.rs`: `/ma/ipfs/0.0.1` request handling
- `src/config.rs`: shared world config loading and startup identity material lookup/decrypt

This crate is intended to be reused by world implementations that want the same IPFS publish behavior without duplicating protocol logic.
