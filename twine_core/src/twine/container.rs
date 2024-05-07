use std::fmt::Display;
use std::sync::Arc;
use ipld_core::codec::Codec;
use libipld::multihash::MultihashDigest;
use libipld::store::StoreParams;
use libipld::Block;
use libipld::{Cid, multihash::Code};
use serde_ipld_dagjson::codec::DagJsonCodec;
use serde::{Serialize, Deserialize};
use serde_ipld_dagcbor::codec::DagCborCodec;
use crate::crypto::{get_hasher, assert_cid, get_cid};
use crate::verify::{Verifiable, Verified};
use super::dag_json::TwineContainerJson;
use super::{Stitch, TwineBlock};
use crate::errors::VerificationError;
use crate::as_cid::AsCid;

pub trait TwineContent: Clone + Verifiable + Send {
  fn back_stitches(&self) -> Vec<Stitch>;
  fn cross_stitches(&self) -> Vec<Stitch>;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TwineContainer<C: TwineContent> {
  #[serde(skip)]
  cid: Cid,

  content: Verified<C>,
  pub(super)
  signature: String,
}

impl<C: TwineContent> TwineContainer<C> {
  pub fn cid(&self) -> Cid {
    self.cid.clone()
  }

  fn verify_cid(&self, expected: Cid) -> Result<(), VerificationError> {
    assert_cid(expected, self.cid)
  }

  pub fn back_stitches(&self) -> Vec<Stitch> {
    self.content().back_stitches()
  }

  pub fn cross_stitches(&self) -> Vec<Stitch> {
    self.content().cross_stitches()
  }

  pub fn content(&self) -> &C {
    self.content.as_inner()
  }

  pub fn hasher(&self) -> Code {
    get_hasher(&self.cid).unwrap()
  }

  pub fn signature(&self) -> &str {
    &self.signature
  }
}

impl<C: TwineContent> From<TwineContainer<C>> for Cid {
  fn from(t: TwineContainer<C>) -> Self {
    t.cid
  }
}

impl<C: TwineContent> AsCid for TwineContainer<C> {
  fn as_cid(&self) -> &Cid {
    &self.cid
  }
}

impl<C: TwineContent, S: StoreParams> From<TwineContainer<C>> for Block<S> where C: Serialize + for<'de> Deserialize<'de> {
  fn from(t: TwineContainer<C>) -> Self {
    Block::new_unchecked(t.cid(), t.bytes().to_vec())
  }
}

impl<C> TwineContainer<C> where C: TwineContent + Serialize + for<'de> Deserialize<'de> {
  /// Instance a Twine from its content and signature
  fn new_from_parts(hasher: Code, content: Verified<C>, signature: String) -> Self {
    let mut twine = Self { cid: Cid::default(), content, signature };
    let dat = DagCborCodec::encode_to_vec(&twine).unwrap();
    twine.cid = get_cid(hasher, dat.as_slice());
    twine
  }

  pub fn content_hash(&self) -> Vec<u8> {
    let bytes = DagCborCodec::encode_to_vec(self.content()).unwrap();
    self.hasher().digest(&bytes).to_bytes()
  }

  fn compute_cid(&mut self, hasher: Code) {
    let dat = DagCborCodec::encode_to_vec(self).unwrap();
    self.cid = get_cid(hasher, dat.as_slice());
  }
}

impl<T> TryFrom<TwineContainerJson<T>> for TwineContainer<T> where T: TwineContent + Serialize + for<'de> Deserialize<'de> {
  type Error = VerificationError;

  fn try_from(j: TwineContainerJson<T>) -> Result<Self, Self::Error> {
    let hasher = get_hasher(&j.cid)?;
    let twine = Self::new_from_parts(hasher, j.data.content, j.data.signature);
    twine.verify_cid(j.cid)?;
    Ok(twine)
  }
}

impl<C> TwineBlock for TwineContainer<C> where C: TwineContent + Serialize + for<'de> Deserialize<'de> {

  fn cid(&self) -> Cid {
    self.cid()
  }
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let j: TwineContainerJson<C> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    let twine = TwineContainer::try_from(j)?;
    Ok(twine)
  }

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let mut twine: Self = DagCborCodec::decode_from_slice(bytes.as_slice())?;
    twine.compute_cid(hasher);
    Ok(twine)
  }

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError> {
    let hasher = get_hasher(&cid)?;
    let twine = Self::from_bytes_unchecked(hasher, bytes.as_ref().to_vec())?;
    twine.verify_cid(cid)?;
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
  fn bytes(&self) -> Arc<[u8]> {
    DagCborCodec::encode_to_vec(self).unwrap().as_slice().into()
  }
}

impl<C> Display for TwineContainer<C> where C: TwineContent + Serialize + for<'de> Deserialize<'de> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.clone().to_dag_json_pretty())
  }
}

#[cfg(test)]
mod test {
  use crate::twine::*;
  use crate::test::*;

  #[test]
  fn test_invalid_signature(){
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let mut tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    tixel.signature = tixel.signature.replace("jvap", "javp");
    let res = strand.verify_tixel(&tixel);
    assert!(res.is_err(), "Signature verification should have failed");
  }
}
