use std::fmt::Display;
use ipld_core::codec::Codec;
use libipld::multihash::MultihashDigest;
use libipld::store::StoreParams;
use libipld::Block;
use libipld::{Cid, multihash::Code};
use serde_ipld_dagjson::codec::DagJsonCodec;
use serde::{Serialize, Deserialize};
use serde_ipld_dagcbor::codec::DagCborCodec;
use crate::twine::get_hasher;

use super::{assert_cid, TwineBlock};
use super::errors::ParseError;
use super::get_cid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TwineContainer<C: Clone> {
  #[serde(skip)]
  cid: Cid,

  content: C,
  pub(super)
  signature: String,
}

impl<C: Clone> TwineContainer<C> {
  pub fn cid(&self) -> Cid {
    self.cid.clone()
  }

  fn assert_cid(&self, expected: Cid) -> Result<(), ParseError> {
    assert_cid(expected, self.cid)
  }

  pub fn content(&self) -> &C {
    &self.content
  }

  pub fn hasher(&self) -> Code {
    get_hasher(&self.cid).unwrap()
  }

  pub fn signature(&self) -> &str {
    &self.signature
  }
}

impl<C: Clone> From<TwineContainer<C>> for Cid {
  fn from(t: TwineContainer<C>) -> Self {
    t.cid()
  }
}

impl<C: Clone, S: StoreParams> From<TwineContainer<C>> for Block<S> where C: Serialize + for<'de> Deserialize<'de> {
  fn from(t: TwineContainer<C>) -> Self {
    Block::new_unchecked(t.cid(), t.bytes())
  }
}

impl<C> TwineContainer<C> where C: Clone + Serialize + for<'de> Deserialize<'de> {
  /// Instance a Twine from its content and signature
  fn new_from_parts(hasher: Code, content: C, signature: String) -> Self {
    let mut twine = Self { cid: Cid::default(), content, signature };
    let dat = DagCborCodec::encode_to_vec(&twine).unwrap();
    twine.cid = get_cid(hasher, dat.as_slice());
    twine
  }

  pub fn content_hash(&self) -> Vec<u8> {
    let bytes = DagCborCodec::encode_to_vec(self.content()).unwrap();
    self.hasher().digest(&bytes).to_bytes()
  }
}

impl<C> TwineBlock for TwineContainer<C> where C: Clone + Serialize + for<'de> Deserialize<'de> {
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, ParseError> {

    #[derive(Serialize, Deserialize)]
    struct TwineContainerJson<T: Clone> {
      cid: Cid,
      data: TwineContainer<T>,
    }

    let j: TwineContainerJson<C> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    let hasher = get_hasher(&j.cid)?;
    let twine = Self::new_from_parts(hasher, j.data.content, j.data.signature);
    twine.assert_cid(j.cid)?;
    Ok(twine)
  }

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, ParseError> {
    let mut twine: Self = DagCborCodec::decode_from_slice(bytes.as_slice())?;
    twine.cid = get_cid(hasher, bytes.as_slice());
    Ok(twine)
  }

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, ParseError> {
    let hasher = get_hasher(&cid)?;
    let twine = Self::from_bytes_unchecked(hasher, bytes.as_ref().to_vec())?;
    twine.assert_cid(cid)?;
    Ok(twine)
  }

  /// Encode to DAG-JSON
  fn dag_json(&self) -> String {
    format!(
      "{{\"cid\":{},\"data\":{}}}",
      String::from_utf8(DagJsonCodec::encode_to_vec(&self.cid).unwrap()).unwrap(),
      String::from_utf8(DagJsonCodec::encode_to_vec(self).unwrap()).unwrap()
    )
  }

  /// Encode to raw bytes
  fn bytes(&self) -> Vec<u8> {
    DagCborCodec::encode_to_vec(self).unwrap()
  }
}

impl<C> Display for TwineContainer<C> where C: Clone + Serialize + for<'de> Deserialize<'de> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.clone().to_dag_json_pretty())
  }
}
