//! Twine Core Library
//!
//! Docs...
//!
// pub(crate) mod serde_utils;

pub type Bytes = Vec<u8>;

pub mod errors;
pub mod crypto;
pub mod as_cid;
pub mod twine;
pub mod verify;
pub mod specification;
pub mod schemas;
pub mod resolver;
pub mod store;
pub mod car;
pub mod skiplist;

pub use semver;
pub use twine::dag_json;

pub use ipld_core::cid::{self, Cid};
pub use ipld_core::{self, ipld::Ipld};
pub use serde_ipld_dagcbor;
pub use serde_ipld_dagjson;
pub use multihash_codetable;

#[cfg(test)]
mod test;

