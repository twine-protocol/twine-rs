use core::str;
/// Structs and traits common to both Chain's and Pulses

use std::fmt::Display;
use std::sync::Arc;
use libipld::multihash::Code;
use libipld::store::StoreParams;
use libipld::{Block, Cid};
use serde::{Serialize, Deserialize};
use crate::as_cid::AsCid;
use crate::crypto::{assert_cid, get_hasher};
use super::{Strand, Tixel};
use super::TwineBlock;
use crate::errors::VerificationError;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum AnyTwine {
  Strand(Arc<Strand>),
  Tixel(Arc<Tixel>),
}

impl AnyTwine {
  pub fn cid(&self) -> Cid {
    match self {
      Self::Strand(s) => s.cid(),
      Self::Tixel(t) => t.cid(),
    }
  }

  pub fn strand_cid(&self) -> Cid {
    match self {
      Self::Strand(s) => s.cid(),
      Self::Tixel(t) => t.strand_cid(),
    }
  }

  pub fn content_hash(&self) -> Vec<u8> {
    match self {
      Self::Strand(s) => s.content_hash(),
      Self::Tixel(t) => t.content_hash(),
    }
  }

  pub fn signature(&self) -> &str {
    match self {
      Self::Strand(s) => s.signature(),
      Self::Tixel(t) => t.signature(),
    }
  }

  /// Is this twine a Strand?
  pub fn is_strand(&self) -> bool {
    matches!(self, Self::Strand(_))
  }

  /// Is this twine a Tixel?
  pub fn is_tixel(&self) -> bool {
    matches!(self, Self::Tixel(_))
  }

  fn assert_cid(&self, expected: Cid) -> Result<(), VerificationError> {
    assert_cid(expected, self.cid())
  }
}

impl From<Strand> for AnyTwine {
  fn from(s: Strand) -> Self {
    Self::Strand(Arc::new(s))
  }
}

impl From<Tixel> for AnyTwine {
  fn from(t: Tixel) -> Self {
    Self::Tixel(Arc::new(t))
  }
}

impl From<Arc<Strand>> for AnyTwine {
  fn from(s: Arc<Strand>) -> Self {
    Self::Strand(s)
  }
}

impl From<Arc<Tixel>> for AnyTwine {
  fn from(t: Arc<Tixel>) -> Self {
    Self::Tixel(t)
  }
}

impl AsCid for AnyTwine {
  fn as_cid(&self) -> &Cid {
    match self {
      Self::Strand(s) => s.as_cid(),
      Self::Tixel(t) => t.as_cid(),
    }
  }
}

impl From<AnyTwine> for Cid {
  fn from(t: AnyTwine) -> Self {
    match t {
      AnyTwine::Strand(s) => s.cid(),
      AnyTwine::Tixel(t) => t.cid(),
    }
  }
}

impl<S: StoreParams> From<AnyTwine> for Block<S> {
  fn from(t: AnyTwine) -> Self {
    Block::new_unchecked(t.cid(), t.bytes().to_vec())
  }
}

impl TwineBlock for AnyTwine {
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let str_json = json.to_string();
    // assume it's a Tixel first
    let tixel = Tixel::from_dag_json(&str_json);
    if tixel.is_ok() {
      return Ok(Self::Tixel(tixel.unwrap().into()));
    }
    // assume it's a Strand next
    let strand = Strand::from_dag_json(&str_json);
    if strand.is_ok() {
      return Ok(Self::Strand(strand.unwrap().into()));
    }
    let msg = format!("Undecodable structure because:\n{}\n{}", tixel.err().unwrap(), strand.err().unwrap());
    Err(VerificationError::InvalidTwineFormat(msg))
  }

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let tixel = Tixel::from_bytes_unchecked(hasher, bytes.clone());
    if tixel.is_ok() {
      return Ok(Self::Tixel(tixel.unwrap().into()));
    }
    let strand = Strand::from_bytes_unchecked(hasher, bytes);
    if strand.is_ok() {
      return Ok(Self::Strand(strand.unwrap().into()));
    }
    let msg = format!("Undecodable structure because:\n{}\n{}", tixel.err().unwrap(), strand.err().unwrap());
    Err(VerificationError::InvalidTwineFormat(msg))
  }

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError> {
    let hasher = get_hasher(&cid)?;
    let twine = Self::from_bytes_unchecked(hasher, bytes.as_ref().to_vec())?;
    twine.assert_cid(cid)?;
    Ok(twine)
  }

  /// Encode to DAG-JSON
  fn dag_json(&self) -> String {
    match self {
      Self::Strand(s) => s.dag_json(),
      Self::Tixel(t) => t.dag_json(),
    }
  }

  /// Encode to raw bytes
  fn bytes(&self) -> Arc<[u8]> {
    match self {
      Self::Strand(s) => s.bytes(),
      Self::Tixel(t) => t.bytes(),
    }
  }
}

impl Display for AnyTwine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Strand(s) => write!(f, "{}", s),
      Self::Tixel(t) => write!(f, "{}", t),
    }
  }
}
