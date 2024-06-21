use crate::{crypto::{assert_cid, get_hasher}, Cid};
use std::{fmt::Display, sync::Arc};
use crate::errors::VerificationError;
use multihash_codetable::Code;

pub trait TwineBlock where Self: Sized {
  fn cid(&self) -> &Cid;
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, VerificationError>;

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError>;

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError>;

  /// Encode to DAG-JSON
  fn dag_json(&self) -> String;

  /// Encode to raw bytes
  fn bytes(&self) -> Arc<[u8]>;

  fn content_bytes(&self) -> Arc<[u8]>;

  /// Encode to pretty dag-json
  fn dag_json_pretty(&self) -> String {
    let json = self.dag_json();
    let j: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();
    serde_json::to_string_pretty(&j).unwrap()
  }

  fn verify_cid(&self, expected: &Cid) -> Result<(), VerificationError> {
    assert_cid(expected, self.cid())
  }

  fn hasher(&self) -> Code {
    get_hasher(self.cid()).unwrap()
  }

  fn content_hash(&self) -> Vec<u8> {
    use multihash_codetable::MultihashDigest;
    let bytes = self.content_bytes();
    self.hasher().digest(&bytes).to_bytes()
  }
}

// impl Display for dyn TwineBlock {
//   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//     write!(f, "{}", self.to_dag_json_pretty())
//   }
// }
