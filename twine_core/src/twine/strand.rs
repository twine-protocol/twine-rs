use std::{fmt::Display, sync::Arc};
use crate::{as_cid::AsCid, crypto::{get_hasher, PublicKey}, dag_json::TwineContainerJson, specification::Subspec, verify::{Verifiable, Verified}};
use crate::schemas::v1::{self, ChainContentV1};
use crate::schemas::v2;
use multihash_codetable::Code;
use semver::Version;
use serde_ipld_dagcbor::codec::DagCborCodec;
use serde_ipld_dagjson::codec::DagJsonCodec;
use crate::Ipld;
use serde::{Serialize, Deserialize};
use ipld_core::{cid::Cid, codec::Codec};
use super::{Tixel, TwineBlock};
use crate::errors::VerificationError;
use crate::schemas::{StrandContainer, TwineContainer};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
pub enum StrandContainerVersion {
  V1(v1::ContainerV1<ChainContentV1>),
  V2(v2::StrandContainerV2),
}

impl StrandContainerVersion {
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      StrandContainerVersion::V1(v) => {
        v.compute_cid(hasher);
      },
      StrandContainerVersion::V2(_) => unimplemented!(),
    }
  }
}

impl Verifiable for StrandContainerVersion {
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      StrandContainerVersion::V1(v) => v.verify(),
      StrandContainerVersion::V2(v) => v.verify(),
    }
  }
}

impl From<v1::ContainerV1<ChainContentV1>> for StrandContainerVersion {
  fn from(v: v1::ContainerV1<ChainContentV1>) -> Self {
    StrandContainerVersion::V1(v)
  }
}

impl From<v2::StrandContainerV2> for StrandContainerVersion {
  fn from(v: v2::StrandContainerV2) -> Self {
    StrandContainerVersion::V2(v)
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Strand(Verified<StrandContainerVersion>);

impl Strand {
  pub fn try_new<C: Into<StrandContainerVersion>>(container: C) -> Result<Self, VerificationError> {
    let verified = Verified::try_new(container.into())?;
    Ok(Self(verified))
  }

  pub fn cid(&self) -> Cid {
    match &*self.0 {
      StrandContainerVersion::V1(v) => v.cid().clone(),
      StrandContainerVersion::V2(v) => v.cid().clone(),
    }
  }

  pub fn key(&self) -> PublicKey {
    match &*self.0 {
      StrandContainerVersion::V1(v) => v.key().into(),
      StrandContainerVersion::V2(v) => v.key().clone(),
    }
  }

  pub fn radix(&self) -> u8 {
    match &*self.0 {
      StrandContainerVersion::V1(v) => v.radix(),
      StrandContainerVersion::V2(v) => v.radix(),
    }
  }

  pub fn version(&self) -> Version {
    match &*self.0 {
      StrandContainerVersion::V1(v) => v.version(),
      StrandContainerVersion::V2(v) => v.version(),
    }
  }

  pub fn subspec(&self) -> Option<Subspec> {
    match &*self.0 {
      StrandContainerVersion::V1(v) => v.subspec(),
      StrandContainerVersion::V2(v) => v.subspec(),
    }
  }

  pub fn details(&self) -> &Ipld {
    match &*self.0 {
      StrandContainerVersion::V1(v) => &v.details(),
      StrandContainerVersion::V2(v) => &v.details(),
    }
  }

  pub fn verify_tixel(&self, tixel: &Tixel) -> Result<(), VerificationError> {
    match &*self.0 {
      StrandContainerVersion::V1(v) => {
        // also verify that this tixel belongs to the strand
        if tixel.strand_cid() != self.cid() {
          return Err(VerificationError::TixelNotOnStrand);
        }
        v.verify_signature(String::from_utf8(tixel.signature().into()).unwrap(), tixel.content_hash())
      },
      StrandContainerVersion::V2(v) => v.verify_tixel(tixel.v2_container()),
    }
  }

  pub fn hasher(&self) -> Code {
    get_hasher(&self.cid()).unwrap()
  }
}

impl From<Strand> for Cid {
  fn from(t: Strand) -> Self {
    t.cid()
  }
}

impl AsCid for Strand {
  fn as_cid(&self) -> &Cid {
    match &*self.0 {
      StrandContainerVersion::V1(v) => v.cid(),
      StrandContainerVersion::V2(v) => v.cid(),
    }
  }
}

impl TwineBlock for Strand {
  fn cid(&self) -> &Cid {
    self.as_cid()
  }
  /// Decode from DAG-JSON
  ///
  /// DAG-JSON is a JSON object with a CID and a data object. CID is verified.
  fn from_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let j: TwineContainerJson<StrandContainerVersion> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    let cid = j.cid;
    let container = match j.data {
      // v1 requires recomputing the CID
      mut container@StrandContainerVersion::V1(_) => {
        let hasher = get_hasher(&cid)?;
        container.compute_cid(hasher);
        container
      },
      container@StrandContainerVersion::V2(_) => container,
    };

    let twine = Self(Verified::try_new(container)?);
    twine.verify_cid(&cid)?;
    Ok(twine)
  }

  /// Decode from raw bytes without checking CID
  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let mut twine: StrandContainerVersion = DagCborCodec::decode_from_slice(bytes.as_slice())?;
    // if v1... recompute cid
    if let StrandContainerVersion::V1(_) = twine {
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
      StrandContainerVersion::V1(v) => DagCborCodec::encode_to_vec(v.content()).unwrap(),
      StrandContainerVersion::V2(v) => v.content_bytes().unwrap().into(),
    };
    bytes.as_slice().into()
  }
}

impl Display for Strand {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_dag_json_pretty())
  }
}
