pub mod bundle;
pub mod config;
pub mod identity;
mod ipfs;
pub mod kubo;

pub use identity::{generate_world_did_document_ephemeral, generate_world_did_document_from_keys};
pub use ipfs::{handle_ipfs_publish_message, IpfsRequestReply, IPFS_REPLY_CONTENT_TYPE};
pub use kubo::{
    ensure_kubo_key_alias, publish_identity_document, publish_identity_with_kubo_alias,
    IdentityPublishResult,
};
