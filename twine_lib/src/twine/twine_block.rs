use crate::errors::VerificationError;
use crate::{
  crypto::{assert_cid, get_hasher},
  Cid,
};
use multihash_codetable::Code;
use std::{fmt::Display, sync::Arc};

/// A trait providing methods for twine data structures
pub trait TwineBlock
where
  Self: Sized,
{
  /// Get the CID
  fn cid(&self) -> &Cid;
  /// Decode from DAG-JSON
  ///
  /// Tagged dag json is a JSON object with a CID and a data object.
  /// CID is verified against the data.
  fn from_tagged_dag_json<S: Display>(json: S) -> Result<Self, VerificationError>;

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError>;

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError>;

  /// Encode a `Tagged` version to DAG-JSON
  fn tagged_dag_json(&self) -> String;

  /// Encode to raw bytes
  fn bytes(&self) -> Arc<[u8]>;

  /// The serialized bytes of the content field
  fn content_bytes(&self) -> Arc<[u8]>;

  /// Encode a `Tagged` version to pretty dag-json
  fn tagged_dag_json_pretty(&self) -> String {
    let json = self.tagged_dag_json();
    let j: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();
    serde_json::to_string_pretty(&j).unwrap()
  }

  /// Verify the CID against the expected CID
  fn verify_cid(&self, expected: &Cid) -> Result<(), VerificationError> {
    assert_cid(expected, self.cid())
  }

  /// Get the hasher (Code) for this data structure
  fn hasher(&self) -> Code {
    get_hasher(self.cid()).unwrap()
  }

  /// Get the hash of the content field
  fn content_hash(&self) -> Vec<u8> {
    use multihash_codetable::MultihashDigest;
    let bytes = self.content_bytes();
    self.hasher().digest(&bytes).to_bytes()
  }
}
