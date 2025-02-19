use std::sync::Arc;

use ipld_core::{cid::Cid, codec::Codec, ipld::Ipld};
use multihash_codetable::Code;
use semver::Version;
use serde_ipld_dagcbor::codec::DagCborCodec;
use crate::{crypto::{get_hasher, PublicKey, Signature}, errors::VerificationError, specification::Subspec, twine::{BackStitches, CrossStitches, Tixel, TwineBlock}, verify::Verifiable};
use serde::{Deserialize, Serialize};

pub mod v1;
pub mod v2;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
pub enum StrandSchemaVersion {
  V1(v1::ContainerV1<v1::ChainContentV1>),
  V2(v2::StrandContainerV2),
}

impl Verifiable for StrandSchemaVersion {
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      StrandSchemaVersion::V1(v) => v.verify(),
      StrandSchemaVersion::V2(v) => v.verify(),
    }
  }
}

impl StrandSchemaVersion {
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      StrandSchemaVersion::V1(v) => {
        v.compute_cid(hasher);
      },
      StrandSchemaVersion::V2(_) => unimplemented!(),
    }
  }

  pub fn cid(&self) -> &Cid {
    match self {
      StrandSchemaVersion::V1(v) => v.cid(),
      StrandSchemaVersion::V2(v) => v.cid(),
    }
  }

  pub fn version(&self) -> Version {
    match self {
      StrandSchemaVersion::V1(v) => v.version(),
      StrandSchemaVersion::V2(v) => v.version(),
    }
  }

  pub fn spec_str(&self) -> &str {
    match self {
      StrandSchemaVersion::V1(v) => v.spec_str(),
      StrandSchemaVersion::V2(v) => v.spec_str(),
    }
  }

  pub fn subspec(&self) -> Option<Subspec> {
    match self {
      StrandSchemaVersion::V1(v) => v.subspec(),
      StrandSchemaVersion::V2(v) => v.subspec(),
    }
  }

  pub fn key(&self) -> PublicKey {
    match self {
      StrandSchemaVersion::V1(v) => v.key().into(),
      StrandSchemaVersion::V2(v) => v.key().clone(),
    }
  }

  pub fn radix(&self) -> u8 {
    match self {
      StrandSchemaVersion::V1(v) => v.radix(),
      StrandSchemaVersion::V2(v) => v.radix(),
    }
  }

  pub fn details(&self) -> &Ipld {
    match self {
      StrandSchemaVersion::V1(v) => v.details(),
      StrandSchemaVersion::V2(v) => v.details(),
    }
  }

  pub fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>> {
    match self {
      StrandSchemaVersion::V1(_) => None,
      StrandSchemaVersion::V2(v) => v.expiry(),
    }
  }

  pub fn verify_tixel(&self, tixel: &Tixel) -> Result<(), VerificationError> {
    // also verify that this tixel belongs to the strand
    if &tixel.strand_cid() != self.cid() {
      return Err(VerificationError::TixelNotOnStrand);
    }
    // tixel must have same major version as strand
    if tixel.version().major != self.version().major {
      return Err(VerificationError::InvalidTwineFormat("Tixel version does not match Strand version".into()));
    }
    match self {
      Self::V1(v) => {
        v.verify_signature(String::from_utf8(tixel.signature().into()).unwrap(), tixel.content_hash())?;
      },
      Self::V2(_) => {
        self.key().verify(tixel.signature(), tixel.content_bytes())?;
      }
    };
    Ok(())
  }

  pub fn content_bytes(&self) -> Arc<[u8]> {
    let bytes = match self {
      Self::V1(v) => DagCborCodec::encode_to_vec(v.content()).unwrap(),
      Self::V2(v) => v.content_bytes().unwrap().into(),
    };
    bytes.as_slice().into()
  }

  pub fn hasher(&self) -> Code {
    get_hasher(&self.cid()).unwrap()
  }
}

impl TryFrom<v1::ContainerV1<v1::ChainContentV1>> for StrandSchemaVersion {
  type Error = VerificationError;

  fn try_from(v: v1::ContainerV1<v1::ChainContentV1>) -> Result<Self, Self::Error> {
    Ok(StrandSchemaVersion::V1(v))
  }
}

impl TryFrom<v2::StrandContainerV2> for StrandSchemaVersion {
  type Error = VerificationError;

  fn try_from(v: v2::StrandContainerV2) -> Result<Self, Self::Error> {
    Ok(StrandSchemaVersion::V2(v))
  }
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
pub enum TixelSchemaVersion {
  V1(v1::ContainerV1<v1::PulseContentV1>),
  V2(v2::TixelContainerV2),
}

impl Verifiable for TixelSchemaVersion {
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      TixelSchemaVersion::V1(v) => v.verify(),
      TixelSchemaVersion::V2(v) => v.verify(),
    }
  }
}

impl TixelSchemaVersion {
  pub fn cid(&self) -> &Cid {
    match self {
      TixelSchemaVersion::V1(v) => v.cid(),
      TixelSchemaVersion::V2(v) => v.cid(),
    }
  }

  pub fn index(&self) -> u64 {
    match self {
      TixelSchemaVersion::V1(v) => v.index(),
      TixelSchemaVersion::V2(v) => v.index(),
    }
  }

  pub fn strand_cid(&self) -> &Cid {
    match self {
      TixelSchemaVersion::V1(v) => v.strand_cid(),
      TixelSchemaVersion::V2(v) => v.strand_cid(),
    }
  }

  pub fn spec_str(&self) -> &str {
    match self {
      TixelSchemaVersion::V1(v) => v.spec_str(),
      TixelSchemaVersion::V2(v) => v.spec_str(),
    }
  }

  pub fn version(&self) -> Version {
    match self {
      TixelSchemaVersion::V1(_) => Version::new(1, 0, 0),
      TixelSchemaVersion::V2(v) => v.version(),
    }
  }

  pub fn subspec(&self) -> Option<Subspec> {
    match self {
      TixelSchemaVersion::V1(_) => None,
      TixelSchemaVersion::V2(v) => v.subspec(),
    }
  }

  pub fn cross_stitches(&self) -> CrossStitches {
    match self {
      TixelSchemaVersion::V1(v) => v.cross_stitches(),
      TixelSchemaVersion::V2(v) => v.cross_stitches(),
    }
  }

  pub fn back_stitches(&self) -> BackStitches {
    match self {
      TixelSchemaVersion::V1(v) => v.back_stitches(),
      TixelSchemaVersion::V2(v) => v.back_stitches(),
    }
  }

  pub fn drop_index(&self) -> u64 {
    match self {
      TixelSchemaVersion::V1(_) => 0,
      TixelSchemaVersion::V2(v) => v.drop_index(),
    }
  }

  pub fn payload(&self) -> &Ipld {
    match self {
      TixelSchemaVersion::V1(v) => v.payload(),
      TixelSchemaVersion::V2(v) => v.payload(),
    }
  }

  pub fn signature(&self) -> Signature {
    match self {
      TixelSchemaVersion::V1(v) => v.signature().as_bytes().to_vec().into(),
      TixelSchemaVersion::V2(v) => v.signature(),
    }
  }

  pub fn content_bytes(&self) -> Arc<[u8]> {
    let bytes = match self {
      TixelSchemaVersion::V1(v) => DagCborCodec::encode_to_vec(v.content()).unwrap(),
      TixelSchemaVersion::V2(v) => v.content_bytes().unwrap().into(),
    };
    bytes.as_slice().into()
  }
}

impl TixelSchemaVersion {
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      TixelSchemaVersion::V1(v) => {
        v.compute_cid(hasher);
      },
      TixelSchemaVersion::V2(_) => unimplemented!(),
    }
  }
}

impl TryFrom<v1::ContainerV1<v1::PulseContentV1>> for TixelSchemaVersion {
  type Error = VerificationError;

  fn try_from(v: v1::ContainerV1<v1::PulseContentV1>) -> Result<Self, Self::Error> {
    Ok(TixelSchemaVersion::V1(v))
  }
}

impl TryFrom<v2::TixelContainerV2> for TixelSchemaVersion {
  type Error = VerificationError;

  fn try_from(v: v2::TixelContainerV2) -> Result<Self, Self::Error> {
    Ok(TixelSchemaVersion::V2(v))
  }
}
