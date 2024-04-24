use libipld::Cid;
use std::fmt::Display;
use super::errors::ParseError;
use libipld::multihash::Code;

pub trait TwineEncode where Self: Sized {
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, ParseError>;

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, ParseError>;

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, ParseError>;

  /// Encode to DAG-JSON
  fn to_dag_json(&self) -> String;

  /// Encode to raw bytes
  fn to_bytes(&self) -> Vec<u8>;

  /// Encode to pretty dag-json
  fn to_dag_json_pretty(&self) -> String {
    let json = self.to_dag_json();
    let j: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();
    serde_json::to_string_pretty(&j).unwrap()
  }
}
