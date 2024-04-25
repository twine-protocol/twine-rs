use core::str;
/// Structs and traits common to both Chain's and Pulses

use std::fmt::Display;
use libipld::multihash::Code;
use libipld::store::StoreParams;
use libipld::{Block, Cid};
use serde::{Serialize, Deserialize};
use crate::crypto::{assert_cid, get_hasher};
use super::{Strand, Tixel};
use super::TwineBlock;
use crate::errors::VerificationError;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Twine {
  Strand(Strand),
  Tixel(Tixel),
}

impl Twine {
  pub fn cid(&self) -> Cid {
    match self {
      Twine::Strand(s) => s.cid(),
      Twine::Tixel(t) => t.cid(),
    }
  }

  pub fn content_hash(&self) -> Vec<u8> {
    match self {
      Twine::Strand(s) => s.content_hash(),
      Twine::Tixel(t) => t.content_hash(),
    }
  }

  pub fn signature(&self) -> &str {
    match self {
      Twine::Strand(s) => s.signature(),
      Twine::Tixel(t) => t.signature(),
    }
  }

  /// Is this twine a Strand?
  pub fn is_strand(&self) -> bool {
    matches!(self, Twine::Strand(_))
  }

  /// Is this twine a Tixel?
  pub fn is_tixel(&self) -> bool {
    matches!(self, Twine::Tixel(_))
  }

  fn assert_cid(&self, expected: Cid) -> Result<(), VerificationError> {
    assert_cid(expected, self.cid())
  }
}

impl From<Twine> for Cid {
  fn from(t: Twine) -> Self {
    match t {
      Twine::Strand(s) => s.cid(),
      Twine::Tixel(t) => t.cid(),
    }
  }
}

impl<S: StoreParams> From<Twine> for Block<S> {
  fn from(t: Twine) -> Self {
    Block::new_unchecked(t.cid(), t.bytes())
  }
}

impl TwineBlock for Twine {
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let str_json = json.to_string();
    // assume it's a Tixel first
    let tixel = Tixel::from_dag_json(&str_json);
    if tixel.is_ok() {
      return Ok(Twine::Tixel(tixel.unwrap()));
    }
    // assume it's a Strand next
    let strand = Strand::from_dag_json(&str_json);
    if strand.is_ok() {
      return Ok(Twine::Strand(strand.unwrap()));
    }
    Err(VerificationError::InvalidTwineFormat)
  }

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let tixel = Tixel::from_bytes_unchecked(hasher, bytes.clone());
    if tixel.is_ok() {
      return Ok(Twine::Tixel(tixel.unwrap()));
    }
    let strand = Strand::from_bytes_unchecked(hasher, bytes);
    if strand.is_ok() {
      return Ok(Twine::Strand(strand.unwrap()));
    }
    Err(VerificationError::InvalidTwineFormat)
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
      Twine::Strand(s) => s.dag_json(),
      Twine::Tixel(t) => t.dag_json(),
    }
  }

  /// Encode to raw bytes
  fn bytes(&self) -> Vec<u8> {
    match self {
      Twine::Strand(s) => s.bytes(),
      Twine::Tixel(t) => t.bytes(),
    }
  }
}

impl Display for Twine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Twine::Strand(s) => write!(f, "{}", s),
      Twine::Tixel(t) => write!(f, "{}", t),
    }
  }
}
