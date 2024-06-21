use std::fmt::Display;
use std::sync::Arc;

use crate::as_cid::AsCid;
use crate::crypto::{get_hasher, Signature};
use crate::dag_json::TwineContainerJson;
use crate::schemas::v1::PulseContentV1;
use crate::schemas::v2;
use crate::specification::Subspec;
use crate::verify::{Verifiable, Verified};
use crate::{errors::VerificationError, schemas::v1};
use crate::Cid;
use crate::Ipld;
use ipld_core::serde::{from_ipld, SerdeError};
use multihash_codetable::Code;
use semver::Version;
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use ipld_core::codec::Codec;
use serde_ipld_dagcbor::codec::DagCborCodec;
use serde_ipld_dagjson::codec::DagJsonCodec;
use super::{BackStitches, CrossStitches, Stitch, TwineBlock};
use super::Strand;
use crate::schemas::{TixelContainer, TwineContainer};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
pub enum TixelContainerVersion {
  V1(v1::ContainerV1<PulseContentV1>),
  V2(v2::TixelContainerV2),
}

impl TixelContainerVersion {
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      TixelContainerVersion::V1(v) => {
        v.compute_cid(hasher);
      },
      TixelContainerVersion::V2(_) => unimplemented!(),
    }
  }
}

impl Verifiable for TixelContainerVersion {
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      TixelContainerVersion::V1(v) => v.verify(),
      TixelContainerVersion::V2(_) => Ok(()),
    }
  }
}

impl From<v1::ContainerV1<PulseContentV1>> for TixelContainerVersion {
  fn from(v: v1::ContainerV1<PulseContentV1>) -> Self {
    TixelContainerVersion::V1(v)
  }
}

impl From<v2::TixelContainerV2> for TixelContainerVersion {
  fn from(v: v2::TixelContainerV2) -> Self {
    TixelContainerVersion::V2(v)
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Tixel(Verified<TixelContainerVersion>);

impl PartialOrd for Tixel {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    if self.strand_cid() != other.strand_cid() {
      return None;
    }
    Some(self.index().cmp(&other.index()))
  }
}

impl Tixel {
  pub fn try_new<T: Into<TixelContainerVersion>>(container: T) -> Result<Self, VerificationError> {
    let verified = Verified::try_new(container.into())?;
    Ok(Self(verified))
  }

  pub fn cid(&self) -> Cid {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.cid().clone(),
      TixelContainerVersion::V2(v) => v.cid().clone(),
    }
  }

  pub fn strand_cid(&self) -> Cid {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.strand_cid().clone(),
      TixelContainerVersion::V2(v) => v.strand_cid().clone(),
    }
  }

  pub fn index(&self) -> u64 {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.index(),
      TixelContainerVersion::V2(v) => v.index(),
    }
  }

  pub fn spec_str(&self) -> &str {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.spec_str(),
      TixelContainerVersion::V2(v) => v.spec_str(),
    }
  }

  pub fn version(&self) -> Version {
    match &*self.0 {
      TixelContainerVersion::V1(_) => Version::parse("1.0.0").unwrap(),
      TixelContainerVersion::V2(v) => v.version(),
    }
  }

  pub fn subspec(&self) -> Option<Subspec> {
    match &*self.0 {
      TixelContainerVersion::V1(_) => None,
      TixelContainerVersion::V2(v) => v.subspec(),
    }
  }

  pub fn payload(&self) -> &Ipld {
    match &*self.0 {
      TixelContainerVersion::V1(v) => &v.payload(),
      TixelContainerVersion::V2(v) => &v.payload(),
    }
  }

  pub fn extract_payload<T: DeserializeOwned>(&self) -> Result<T, SerdeError> {
    let payload = self.payload();
    from_ipld(payload.clone())
  }

  pub fn back_stitches(&self) -> BackStitches {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.back_stitches(),
      TixelContainerVersion::V2(v) => v.back_stitches(),
    }
  }

  pub fn cross_stitches(&self) -> CrossStitches {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.cross_stitches(),
      TixelContainerVersion::V2(v) => v.cross_stitches(),
    }
  }

  pub fn bytes(&self) -> Arc<[u8]> {
    DagCborCodec::encode_to_vec(self).unwrap().into()
  }

  pub fn verify_with(&self, strand: &Strand) -> Result<(), VerificationError> {
    strand.verify_tixel(self)
  }

  pub fn previous(&self) -> Option<Stitch> {
    self.back_stitches().get(0).cloned()
  }

  pub(crate) fn signature(&self) -> Signature {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.signature().as_bytes().to_vec().into(),
      TixelContainerVersion::V2(v) => v.signature(),
    }
  }

  pub(super) fn v2_container(&self) -> &v2::TixelContainerV2 {
    match &*self.0 {
      TixelContainerVersion::V2(v) => v,
      _ => panic!("Tixel is not a V2 container"),
    }
  }
}

impl From<Tixel> for Cid {
  fn from(t: Tixel) -> Self {
    t.cid()
  }
}

impl AsCid for Tixel {
  fn as_cid(&self) -> &Cid {
    match &*self.0 {
      TixelContainerVersion::V1(v) => v.cid(),
      TixelContainerVersion::V2(v) => v.cid(),
    }
  }
}

impl TwineBlock for Tixel {
  fn cid(&self) -> &Cid {
    self.as_cid()
  }
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let j: TwineContainerJson<TixelContainerVersion> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    let cid = j.cid;
    let container = match j.data {
      // v1 requires recomputing the CID
      mut container@TixelContainerVersion::V1(_) => {
        let hasher = get_hasher(&cid)?;
        container.compute_cid(hasher);
        container
      },
      container@TixelContainerVersion::V2(_) => container,
    };
    let twine = Self(Verified::try_new(container)?);
    twine.verify_cid(&cid)?;
    Ok(twine)
  }

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let mut twine: TixelContainerVersion = DagCborCodec::decode_from_slice(bytes.as_slice())?;
    // if v1... recompute cid
    if let TixelContainerVersion::V1(_) = twine {
      twine.compute_cid(hasher);
    }
    let twine = Self(Verified::try_new(twine)?);
    Ok(twine)
  }

  /// Decode from a Block
  ///
  /// A block is a cid and DAG-CBOR bytes. CID is verified.
  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError> {
    let hasher = get_hasher(&cid)?;
    let twine = Self::from_bytes_unchecked(hasher, bytes.as_ref().to_vec())?;
    twine.verify_cid(&cid)?;
    Ok(twine)
  }

  /// Encode to DAG-JSON
  fn dag_json(&self) -> String {
    format!(
      "{{\"cid\":{},\"data\":{}}}",
      String::from_utf8(DagJsonCodec::encode_to_vec(&self.cid()).unwrap()).unwrap(),
      String::from_utf8(DagJsonCodec::encode_to_vec(self).unwrap()).unwrap()
    )
  }

  /// Encode to raw bytes
  fn bytes(&self) -> Arc<[u8]> {
    DagCborCodec::encode_to_vec(self).unwrap().as_slice().into()
  }

  fn content_bytes(&self) -> Arc<[u8]> {
    let bytes = match &*self.0 {
      TixelContainerVersion::V1(v) => DagCborCodec::encode_to_vec(v.content()).unwrap(),
      TixelContainerVersion::V2(v) => v.content_bytes().unwrap().into(),
    };
    bytes.as_slice().into()
  }
}

impl Display for Tixel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.dag_json_pretty())
  }
}
