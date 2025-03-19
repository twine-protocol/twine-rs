use super::TwineBlock;
use super::{Strand, Tixel, Twine};
use crate::as_cid::AsCid;
use crate::crypto::{assert_cid, get_hasher};
use crate::errors::VerificationError;
use crate::twine::Tagged;
use crate::Cid;
use core::str;
use ipld_core::codec::Codec;
use multihash_codetable::{Code, Multihash};
use serde_ipld_dagjson::codec::DagJsonCodec;
use std::convert::TryFrom;
/// Structs and traits common to both Chain's and Pulses
use std::fmt::Display;

/// A type that can be either a Strand or a Tixel
///
/// Useful for dealing with the fundamental data structures
/// without needing to know which one you're dealing with.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum AnyTwine {
  /// A [`Strand`]
  Strand(Strand),
  /// A [`Tixel`]
  Tixel(Tixel),
}

impl AnyTwine {
  /// Get the CID
  pub fn cid(&self) -> Cid {
    match self {
      Self::Strand(s) => s.cid(),
      Self::Tixel(t) => t.cid(),
    }
  }

  /// Get the Strand CID
  ///
  /// If this is a Tixel, it will return the CID of its strand property
  /// If this is a Strand, it will return its own CID
  pub fn strand_cid(&self) -> Cid {
    match self {
      Self::Strand(s) => s.cid(),
      Self::Tixel(t) => t.strand_cid(),
    }
  }

  /// Get the hash
  pub fn content_hash(&self) -> Multihash {
    match self {
      Self::Strand(s) => s.content_hash(),
      Self::Tixel(t) => t.content_hash(),
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

  /// Unwrap a Tixel or panic
  pub fn unwrap_tixel(&self) -> Tixel {
    match self {
      Self::Tixel(t) => t.clone(),
      _ => panic!("Expected Tixel, found Strand"),
    }
  }

  /// Unwrap a Strand or panic
  pub fn unwrap_strand(&self) -> Strand {
    match self {
      Self::Strand(s) => s.clone(),
      _ => panic!("Expected Strand, found Tixel"),
    }
  }

  fn assert_cid(&self, expected: &Cid) -> Result<(), VerificationError> {
    assert_cid(expected, &self.cid())
  }

  /// Get a list of AnyTwines from a json string formatted as a tagged DAG-JSON array
  pub fn from_tagged_dag_json_array<S: AsRef<str>>(
    json: S,
  ) -> Result<Vec<Self>, VerificationError> {
    let arr: Vec<Tagged<Self>> = DagJsonCodec::decode_from_slice(json.as_ref().as_bytes())?;
    Ok(arr.into_iter().map(|t| t.unpack()).collect())
  }
}

impl PartialEq<Tixel> for AnyTwine {
  fn eq(&self, other: &Tixel) -> bool {
    match self {
      Self::Tixel(t) => *t == *other,
      _ => false,
    }
  }
}

impl PartialEq<AnyTwine> for Tixel {
  fn eq(&self, other: &AnyTwine) -> bool {
    other == self
  }
}

impl PartialEq<Strand> for AnyTwine {
  fn eq(&self, other: &Strand) -> bool {
    match self {
      Self::Strand(s) => *s == *other,
      _ => false,
    }
  }
}

impl PartialEq<AnyTwine> for Strand {
  fn eq(&self, other: &AnyTwine) -> bool {
    other == self
  }
}

impl TryFrom<AnyTwine> for Tixel {
  type Error = VerificationError;

  fn try_from(t: AnyTwine) -> Result<Self, Self::Error> {
    match t {
      AnyTwine::Tixel(t) => Ok(t),
      _ => Err(VerificationError::WrongType {
        expected: "Tixel".to_string(),
        found: "Strand".to_string(),
      }),
    }
  }
}

impl TryFrom<AnyTwine> for Strand {
  type Error = VerificationError;

  fn try_from(s: AnyTwine) -> Result<Self, Self::Error> {
    match s {
      AnyTwine::Strand(s) => Ok(s),
      _ => Err(VerificationError::WrongType {
        expected: "Strand".to_string(),
        found: "Tixel".to_string(),
      }),
    }
  }
}

impl From<Strand> for AnyTwine {
  fn from(s: Strand) -> Self {
    Self::Strand(s)
  }
}

impl From<Twine> for AnyTwine {
  fn from(t: Twine) -> Self {
    Self::Tixel(t.tixel().clone())
  }
}

impl From<Tixel> for AnyTwine {
  fn from(t: Tixel) -> Self {
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

impl TwineBlock for AnyTwine {
  fn cid(&self) -> &Cid {
    self.as_cid()
  }
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_tagged_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let str_json = json.to_string();
    // assume it's a Tixel first
    let tixel = Tixel::from_tagged_dag_json(&str_json);
    if tixel.is_ok() {
      return Ok(Self::Tixel(tixel.unwrap().into()));
    }
    // assume it's a Strand next
    let strand = Strand::from_tagged_dag_json(&str_json);
    if strand.is_ok() {
      return Ok(Self::Strand(strand.unwrap().into()));
    }
    let msg = format!(
      "Undecodable structure:\n{}\n{}",
      tixel.err().unwrap(),
      strand.err().unwrap()
    );
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
    let msg = format!(
      "Undecodable structure because:\n{}\n{}",
      tixel.err().unwrap(),
      strand.err().unwrap()
    );
    Err(VerificationError::InvalidTwineFormat(msg))
  }

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError> {
    let hasher = get_hasher(&cid)?;
    let twine = Self::from_bytes_unchecked(hasher, bytes.as_ref().to_vec())?;
    twine.assert_cid(&cid)?;
    Ok(twine)
  }

  /// Encode to DAG-JSON
  fn tagged_dag_json(&self) -> String {
    match self {
      Self::Strand(s) => s.tagged_dag_json(),
      Self::Tixel(t) => t.tagged_dag_json(),
    }
  }

  /// Encode to raw bytes
  fn bytes(&self) -> std::sync::Arc<[u8]> {
    match self {
      Self::Strand(s) => s.bytes(),
      Self::Tixel(t) => t.bytes(),
    }
  }

  fn content_bytes(&self) -> std::sync::Arc<[u8]> {
    match self {
      Self::Strand(s) => s.content_bytes(),
      Self::Tixel(t) => t.content_bytes(),
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
