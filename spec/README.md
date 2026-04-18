# ma-realms specifications

This directory contains protocol specifications for worlds and actors in the ma-realms ecosystem.

## Protocols

- [/ma/ipfs/0.0.1](./ma-ipfs-0.0.1.md) - trusted DID document publish via a world-backed local Kubo node
- [ma.fields/0.0.1](./ma-fields-0.0.1.md) - language metadata fields in DID `ma.fields`
- [ma.bundle/0.0.1](./ma-bundle-0.0.1.md) - encrypted identity bundle format for world bootstrap and startup publish

## Notes

- These documents are intended to be stable inputs for parallel implementation work.
- The first implementation target is the world-side trusted publish service.
- The actor web application is being developed in parallel and should follow these specifications.