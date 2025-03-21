//! Schema definitions for the Twine protocol data structures.
//!
//! These are internal to the library and should not be used directly.
use std::sync::Arc;

use crate::{
  crypto::{get_hasher, PublicKey, Signature},
  errors::VerificationError,
  specification::Subspec,
  twine::{BackStitches, CrossStitches, Tixel, TwineBlock},
  verify::Verifiable,
};
use ipld_core::{cid::Cid, codec::Codec, ipld::Ipld};
use multihash_codetable::Code;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor::codec::DagCborCodec;

pub mod v1;
pub mod v2;

/// The different Strand schema versions
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
pub enum StrandSchemaVersion {
  /// version 1
  V1(v1::ContainerV1<v1::ChainContentV1>),
  /// version 2
  V2(v2::StrandContainerV2),
}

impl Verifiable for StrandSchemaVersion {
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      StrandSchemaVersion::V1(v) => v.verify(),
      StrandSchemaVersion::V2(v) => v.verify(),
    }
  }
}

impl StrandSchemaVersion {
  /// Compute the CID of the data structure
  ///
  /// This was necessary for v1 schemas since the hash algorithm
  /// for the CID was not stored in the schema.
  ///
  /// Is not implemented for v2
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      StrandSchemaVersion::V1(v) => {
        v.compute_cid(hasher);
      }
      StrandSchemaVersion::V2(_) => unimplemented!(),
    }
  }

  /// Get the CID of the data structure
  pub fn cid(&self) -> &Cid {
    match self {
      StrandSchemaVersion::V1(v) => v.cid(),
      StrandSchemaVersion::V2(v) => v.cid(),
    }
  }

  /// Get the version of the data structure
  pub fn version(&self) -> Version {
    match self {
      StrandSchemaVersion::V1(v) => v.version(),
      StrandSchemaVersion::V2(v) => v.version(),
    }
  }

  /// Get the spec string of the data structure
  pub fn spec_str(&self) -> &str {
    match self {
      StrandSchemaVersion::V1(v) => v.spec_str(),
      StrandSchemaVersion::V2(v) => v.spec_str(),
    }
  }

  /// Get the subspec of the data structure if it exists
  pub fn subspec(&self) -> Option<Subspec> {
    match self {
      StrandSchemaVersion::V1(v) => v.subspec(),
      StrandSchemaVersion::V2(v) => v.subspec(),
    }
  }

  /// Get the public key of the data structure
  pub fn key(&self) -> PublicKey {
    match self {
      StrandSchemaVersion::V1(v) => v.key().into(),
      StrandSchemaVersion::V2(v) => v.key().clone(),
    }
  }

  /// Get the radix value of the skiplist
  pub fn radix(&self) -> u8 {
    match self {
      StrandSchemaVersion::V1(v) => v.radix(),
      StrandSchemaVersion::V2(v) => v.radix(),
    }
  }

  /// Get the details of the data structure
  pub fn details(&self) -> &Ipld {
    match self {
      StrandSchemaVersion::V1(v) => v.details(),
      StrandSchemaVersion::V2(v) => v.details(),
    }
  }

  /// Get the expiry date of the data structure if it exists
  pub fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>> {
    match self {
      StrandSchemaVersion::V1(_) => None,
      StrandSchemaVersion::V2(v) => v.expiry(),
    }
  }

  /// Verify a Tixel using this Strand's public key
  pub fn verify_tixel(&self, tixel: &Tixel) -> Result<(), VerificationError> {
    // also verify that this tixel belongs to the strand
    if &tixel.strand_cid() != self.cid() {
      return Err(VerificationError::TixelNotOnStrand);
    }
    // tixel must have same major version as strand
    if tixel.version().major != self.version().major {
      return Err(VerificationError::InvalidTwineFormat(
        "Tixel version does not match Strand version".into(),
      ));
    }
    match self {
      Self::V1(v) => {
        v.verify_signature(
          String::from_utf8(tixel.signature().into()).unwrap(),
          tixel.content_hash(),
        )?;
      }
      Self::V2(_) => {
        self
          .key()
          .verify(tixel.signature(), tixel.content_bytes())?;
      }
    };
    Ok(())
  }

  /// Get the serialized content of the data structure as bytes
  pub fn content_bytes(&self) -> Arc<[u8]> {
    let bytes = match self {
      Self::V1(v) => DagCborCodec::encode_to_vec(v.content()).unwrap(),
      Self::V2(v) => v.content_bytes().unwrap().into(),
    };
    bytes.as_slice().into()
  }

  /// Get the hasher ([`Code`]) used to compute the CID
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

/// The different Tixel schema versions
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
pub enum TixelSchemaVersion {
  /// version 1
  V1(v1::ContainerV1<v1::PulseContentV1>),
  /// version 2
  V2(v2::TixelContainerV2),
}

impl Verifiable for TixelSchemaVersion {
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      TixelSchemaVersion::V1(v) => v.verify(),
      TixelSchemaVersion::V2(v) => v.verify(),
    }
  }
}

impl TixelSchemaVersion {
  /// Compute the CID
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      TixelSchemaVersion::V1(v) => {
        v.compute_cid(hasher);
      }
      TixelSchemaVersion::V2(_) => unimplemented!(),
    }
  }
  /// Get the CID
  pub fn cid(&self) -> &Cid {
    match self {
      TixelSchemaVersion::V1(v) => v.cid(),
      TixelSchemaVersion::V2(v) => v.cid(),
    }
  }

  /// Get the index
  pub fn index(&self) -> u64 {
    match self {
      TixelSchemaVersion::V1(v) => v.index(),
      TixelSchemaVersion::V2(v) => v.index(),
    }
  }

  /// Get the strand CID
  pub fn strand_cid(&self) -> &Cid {
    match self {
      TixelSchemaVersion::V1(v) => v.strand_cid(),
      TixelSchemaVersion::V2(v) => v.strand_cid(),
    }
  }

  /// Get the spec string
  pub fn spec_str(&self) -> &str {
    match self {
      TixelSchemaVersion::V1(v) => v.spec_str(),
      TixelSchemaVersion::V2(v) => v.spec_str(),
    }
  }

  /// Get the version
  pub fn version(&self) -> Version {
    match self {
      TixelSchemaVersion::V1(_) => Version::new(1, 0, 0),
      TixelSchemaVersion::V2(v) => v.version(),
    }
  }

  /// Get the subspec if it exists
  pub fn subspec(&self) -> Option<Subspec> {
    match self {
      TixelSchemaVersion::V1(_) => None,
      TixelSchemaVersion::V2(v) => v.subspec(),
    }
  }

  /// Get the cross stitches
  pub fn cross_stitches(&self) -> CrossStitches {
    match self {
      TixelSchemaVersion::V1(v) => v.cross_stitches(),
      TixelSchemaVersion::V2(v) => v.cross_stitches(),
    }
  }

  /// Get the back stitches
  pub fn back_stitches(&self) -> BackStitches {
    match self {
      TixelSchemaVersion::V1(v) => v.back_stitches(),
      TixelSchemaVersion::V2(v) => v.back_stitches(),
    }
  }

  /// Get the drop index
  pub fn drop_index(&self) -> u64 {
    match self {
      TixelSchemaVersion::V1(_) => 0,
      TixelSchemaVersion::V2(v) => v.drop_index(),
    }
  }

  /// Access the payload as an IPLD object
  pub fn payload(&self) -> &Ipld {
    match self {
      TixelSchemaVersion::V1(v) => v.payload(),
      TixelSchemaVersion::V2(v) => v.payload(),
    }
  }

  /// Get the signature
  pub fn signature(&self) -> Signature {
    match self {
      TixelSchemaVersion::V1(v) => v.signature().as_bytes().to_vec().into(),
      TixelSchemaVersion::V2(v) => v.signature(),
    }
  }

  /// Get the serialized content of the data structure as bytes
  pub fn content_bytes(&self) -> Arc<[u8]> {
    let bytes = match self {
      TixelSchemaVersion::V1(v) => DagCborCodec::encode_to_vec(v.content()).unwrap(),
      TixelSchemaVersion::V2(v) => v.content_bytes().unwrap().into(),
    };
    bytes.as_slice().into()
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
