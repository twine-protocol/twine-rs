#![doc = include_str!("../README.md")]

/// A bytes type that serializes using serde_bytes
#[derive(Debug, Clone, PartialEq, Eq, Hash, ::serde::Serialize, ::serde::Deserialize)]
#[serde(transparent)]
pub struct Bytes(#[serde(with = "serde_bytes")] pub Vec<u8>);

impl Bytes {
  /// Gets a copy of the bytes
  pub fn to_vec(&self) -> Vec<u8> {
    self.0.clone()
  }
}

impl Deref for Bytes {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl From<Vec<u8>> for Bytes {
  fn from(v: Vec<u8>) -> Self {
    Self(v)
  }
}

impl From<&[u8]> for Bytes {
  fn from(v: &[u8]) -> Self {
    Self(v.to_vec())
  }
}

impl From<Bytes> for Vec<u8> {
  fn from(v: Bytes) -> Self {
    v.0
  }
}

impl AsRef<[u8]> for Bytes {
  fn as_ref(&self) -> &[u8] {
    &self.0
  }
}

pub mod as_cid;
pub mod car;
pub mod crypto;
pub mod errors;
pub mod resolver;
pub mod schemas;
pub mod serde;
pub mod skiplist;
pub mod specification;
pub mod store;
pub mod twine;
pub mod verify;

use std::ops::Deref;

pub use semver;

pub use ipld_core::cid::{self, Cid};
pub use ipld_core::{self, ipld::Ipld};
pub use multihash_codetable;
pub use serde_ipld_dagcbor;
pub use serde_ipld_dagjson;

#[cfg(test)]
mod test;
