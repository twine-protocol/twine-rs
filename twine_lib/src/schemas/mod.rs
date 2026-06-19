//! Schema definitions for the Twine protocol data structures.
//!
//! These are internal to the library and should not be used directly.
// Each schema version is a concrete container type (in v1/v2) that
// implements the version-agnostic read trait (StrandSchema / TixelSchema).
// The StrandSchemaVersion / TixelSchemaVersion enums hold one of the known
// versions and forward every accessor to the active version via static dispatch
// (see forward_schema!). Version-absent fields are expressed as trait default
// methods, so older versions inherit the right answer without per-method boilerplate.
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

/// The version-agnostic read surface of a Strand.
// Implemented once per schema version. Fields that do not exist in every
// version are provided as default methods so older versions inherit the
// historically-correct value (e.g. v1 strands have no expiry).
pub trait StrandSchema {
  /// Get the CID of the data structure
  fn cid(&self) -> &Cid;
  /// Get the version of the data structure
  fn version(&self) -> Version;
  /// Get the spec string of the data structure
  fn spec_str(&self) -> &str;
  /// Get the subspec of the data structure if it exists
  fn subspec(&self) -> Option<Subspec>;
  /// Get the public key of the data structure
  fn key(&self) -> PublicKey;
  /// Get the radix value of the skiplist
  fn radix(&self) -> u8;
  /// Get the details of the data structure
  fn details(&self) -> &Ipld;
  /// Get the expiry date of the data structure if it exists
  fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>> {
    None
  }
}

/// The version-agnostic read surface of a Tixel.
// Implemented once per schema version. Version-absent fields are default
// methods (e.g. v1 tixels have no subspec or drop index).
pub trait TixelSchema {
  /// Get the CID
  fn cid(&self) -> &Cid;
  /// Get the index
  fn index(&self) -> u64;
  /// Get the strand CID
  fn strand_cid(&self) -> &Cid;
  /// Get the spec string
  fn spec_str(&self) -> &str;
  /// Get the version
  fn version(&self) -> Version;
  /// Get the subspec if it exists
  fn subspec(&self) -> Option<Subspec> {
    None
  }
  /// Get the cross stitches
  fn cross_stitches(&self) -> CrossStitches;
  /// Get the back stitches
  fn back_stitches(&self) -> BackStitches;
  /// Get the drop index
  fn drop_index(&self) -> u64 {
    0
  }
  /// Access the payload as an IPLD object
  fn payload(&self) -> &Ipld;
  /// Get the signature
  fn signature(&self) -> Signature;
}

// --- per-version implementations ---------------------------------------------

impl StrandSchema for v1::ContainerV1<v1::ChainContentV1> {
  fn cid(&self) -> &Cid {
    self.cid()
  }
  fn version(&self) -> Version {
    self.version()
  }
  fn spec_str(&self) -> &str {
    self.spec_str()
  }
  fn subspec(&self) -> Option<Subspec> {
    self.subspec()
  }
  fn key(&self) -> PublicKey {
    self.key().into()
  }
  fn radix(&self) -> u8 {
    self.radix()
  }
  fn details(&self) -> &Ipld {
    self.details()
  }
  // expiry: v1 strands have none -> trait default (None)
}

impl StrandSchema for v2::StrandContainerV2 {
  fn cid(&self) -> &Cid {
    self.cid()
  }
  fn version(&self) -> Version {
    self.version()
  }
  fn spec_str(&self) -> &str {
    self.spec_str()
  }
  fn subspec(&self) -> Option<Subspec> {
    self.subspec()
  }
  fn key(&self) -> PublicKey {
    self.key().clone()
  }
  fn radix(&self) -> u8 {
    self.radix()
  }
  fn details(&self) -> &Ipld {
    self.details()
  }
  fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>> {
    self.expiry()
  }
}

impl TixelSchema for v1::ContainerV1<v1::PulseContentV1> {
  fn cid(&self) -> &Cid {
    self.cid()
  }
  fn index(&self) -> u64 {
    self.index()
  }
  fn strand_cid(&self) -> &Cid {
    self.strand_cid()
  }
  fn spec_str(&self) -> &str {
    self.spec_str()
  }
  fn version(&self) -> Version {
    // v1 pulses do not carry a version field
    Version::new(1, 0, 0)
  }
  // subspec: v1 tixels have none -> trait default (None)
  fn cross_stitches(&self) -> CrossStitches {
    self.cross_stitches()
  }
  fn back_stitches(&self) -> BackStitches {
    self.back_stitches()
  }
  // drop_index: v1 tixels have none -> trait default (0)
  fn payload(&self) -> &Ipld {
    self.payload()
  }
  fn signature(&self) -> Signature {
    self.signature().as_bytes().to_vec().into()
  }
}

impl TixelSchema for v2::TixelContainerV2 {
  fn cid(&self) -> &Cid {
    self.cid()
  }
  fn index(&self) -> u64 {
    self.index()
  }
  fn strand_cid(&self) -> &Cid {
    self.strand_cid()
  }
  fn spec_str(&self) -> &str {
    self.spec_str()
  }
  fn version(&self) -> Version {
    self.version()
  }
  fn subspec(&self) -> Option<Subspec> {
    self.subspec()
  }
  fn cross_stitches(&self) -> CrossStitches {
    self.cross_stitches()
  }
  fn back_stitches(&self) -> BackStitches {
    self.back_stitches()
  }
  fn drop_index(&self) -> u64 {
    self.drop_index()
  }
  fn payload(&self) -> &Ipld {
    self.payload()
  }
  fn signature(&self) -> Signature {
    self.signature()
  }
}

// Generate the enum's accessor surface by forwarding each method to the active
// version's trait implementation. The variants are fixed, so a single `match`
// per method is enough — no nested repetition, and dispatch stays static
// (inlinable, no vtable). `$trait::$m(v)` is unambiguous UFCS, so it always
// selects the trait method even when the container also has an inherent one.
macro_rules! forward_schema {
  ($enum:ident : $trait:ident { $( fn $m:ident(&self) -> $r:ty; )* }) => {
    impl $enum {
      $(
        #[doc = concat!(
          "Forwards [`", stringify!($trait), "::", stringify!($m),
          "`] to the active schema version."
        )]
        pub fn $m(&self) -> $r {
          match self {
            $enum::V1(v) => $trait::$m(v),
            $enum::V2(v) => $trait::$m(v),
          }
        }
      )*
    }
  };
}

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

forward_schema!(StrandSchemaVersion: StrandSchema {
  fn cid(&self) -> &Cid;
  fn version(&self) -> Version;
  fn spec_str(&self) -> &str;
  fn subspec(&self) -> Option<Subspec>;
  fn key(&self) -> PublicKey;
  fn radix(&self) -> u8;
  fn details(&self) -> &Ipld;
  fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>>;
});

impl StrandSchemaVersion {
  /// Compute and assign the CID using the given hasher.
  // v1 schemas don't store the hash algorithm in the schema, so the CID must
  // be computed externally. Not implemented for v2.
  pub fn compute_cid(&mut self, hasher: Code) {
    match self {
      StrandSchemaVersion::V1(v) => {
        v.compute_cid(hasher);
      }
      StrandSchemaVersion::V2(_) => unimplemented!(),
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

forward_schema!(TixelSchemaVersion: TixelSchema {
  fn cid(&self) -> &Cid;
  fn index(&self) -> u64;
  fn strand_cid(&self) -> &Cid;
  fn spec_str(&self) -> &str;
  fn version(&self) -> Version;
  fn subspec(&self) -> Option<Subspec>;
  fn cross_stitches(&self) -> CrossStitches;
  fn back_stitches(&self) -> BackStitches;
  fn drop_index(&self) -> u64;
  fn payload(&self) -> &Ipld;
  fn signature(&self) -> Signature;
});

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
