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
        // v1 signatures are JWS compact strings; reject (don't panic on)
        // non-UTF-8 bytes from crafted input.
        let sig = String::from_utf8(tixel.signature().into()).map_err(|_| {
          VerificationError::BadSignature("v1 signature is not valid UTF-8".into())
        })?;
        v.verify_signature(sig, tixel.content_hash())?;
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

#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    errors::VerificationError,
    test::{INVALID_SIGNATURE_TIXELJSON, STRAND_V2_JSON, STRANDJSON, TIXEL_V2_JSON, TIXELJSON},
    twine::{Strand, Tixel, TwineBlock},
  };
  use semver::Version;

  // --- helpers -----------------------------------------------------------

  fn strand_v1() -> Strand {
    Strand::from_tagged_dag_json(STRANDJSON).unwrap()
  }
  fn tixel_v1() -> Tixel {
    Tixel::from_tagged_dag_json(TIXELJSON).unwrap()
  }
  fn strand_v2() -> Strand {
    Strand::from_tagged_dag_json(STRAND_V2_JSON).unwrap()
  }
  fn tixel_v2() -> Tixel {
    Tixel::from_tagged_dag_json(TIXEL_V2_JSON).unwrap()
  }

  // --- StrandSchemaVersion accessor coverage (v1) ------------------------

  #[test]
  fn ssv_v1_cid_non_default() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert_ne!(inner.cid(), &Cid::default());
  }

  #[test]
  fn ssv_v1_version_is_1_0_0() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert_eq!(inner.version(), Version::new(1, 0, 0));
  }

  #[test]
  fn ssv_v1_spec_str_nonempty() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(!inner.spec_str().is_empty());
  }

  #[test]
  fn ssv_v1_subspec_some() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    let sub = inner.subspec();
    // the fixture spec is "twine/1.0.x/bell/1.0.x" so there is a subspec
    assert!(sub.is_some(), "v1 fixture should have a subspec");
  }

  #[test]
  fn ssv_v1_radix_32() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert_eq!(inner.radix(), 32);
  }

  #[test]
  fn ssv_v1_expiry_none() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert_eq!(inner.expiry(), None);
  }

  #[test]
  fn ssv_v1_details_is_ipld() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    // v1 details / meta is stored as Ipld; must be accessible
    let _ = inner.details();
  }

  #[test]
  fn ssv_v1_key_accessible() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    let _ = inner.key();
  }

  #[test]
  fn ssv_v1_content_bytes_nonempty() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(!inner.content_bytes().is_empty());
  }

  #[test]
  fn ssv_v1_hasher_accessible() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    let _ = inner.hasher();
  }

  // --- StrandSchemaVersion accessor coverage (v2) ------------------------

  #[test]
  fn ssv_v2_version_is_2_0_0() {
    let strand = strand_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    assert_eq!(inner.version(), Version::new(2, 0, 0));
  }

  #[test]
  fn ssv_v2_spec_str_nonempty() {
    let strand = strand_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(!inner.spec_str().is_empty());
  }

  #[test]
  fn ssv_v2_subspec_some() {
    let strand = strand_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    // fixture spec is "twine/2.0.0/time/1.0.0"
    assert!(inner.subspec().is_some());
  }

  #[test]
  fn ssv_v2_radix_32() {
    let strand = strand_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    assert_eq!(inner.radix(), 32);
  }

  #[test]
  fn ssv_v2_expiry_none_for_fixture() {
    let strand = strand_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    // fixture has "e": null
    assert_eq!(inner.expiry(), None);
  }

  // --- TixelSchemaVersion accessor coverage (v1) -------------------------

  #[test]
  fn tsv_v1_index_100() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.index(), 100);
  }

  #[test]
  fn tsv_v1_strand_cid_non_default() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_ne!(inner.strand_cid(), &Cid::default());
  }

  #[test]
  fn tsv_v1_spec_str_nonempty() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert!(!inner.spec_str().is_empty());
  }

  #[test]
  fn tsv_v1_version_is_1_0_0() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.version(), Version::new(1, 0, 0));
  }

  #[test]
  fn tsv_v1_subspec_none() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.subspec(), None);
  }

  #[test]
  fn tsv_v1_cross_stitches_len_1() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.cross_stitches().len(), 1);
  }

  #[test]
  fn tsv_v1_back_stitches_len_2() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.back_stitches().len(), 2);
  }

  #[test]
  fn tsv_v1_drop_index_is_0() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.drop_index(), 0);
  }

  #[test]
  fn tsv_v1_payload_accessible() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    let _ = inner.payload();
  }

  #[test]
  fn tsv_v1_signature_nonempty() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert!(!inner.signature().is_empty());
  }

  // --- TixelSchemaVersion accessor coverage (v2) -------------------------

  #[test]
  fn tsv_v2_index_0() {
    let tixel = tixel_v2();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.index(), 0);
  }

  #[test]
  fn tsv_v2_subspec_some() {
    let tixel = tixel_v2();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert!(inner.subspec().is_some());
  }

  #[test]
  fn tsv_v2_drop_index_0() {
    let tixel = tixel_v2();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert_eq!(inner.drop_index(), 0);
  }

  // --- verify_tixel branches ---------------------------------------------

  #[test]
  fn verify_tixel_v2_valid_ok() {
    let strand = strand_v2();
    let tixel = tixel_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(
      inner.verify_tixel(&tixel).is_ok(),
      "valid v2 tixel should verify against its strand"
    );
  }

  #[test]
  fn verify_tixel_v1_valid_ok() {
    let strand = strand_v1();
    let tixel = tixel_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(
      inner.verify_tixel(&tixel).is_ok(),
      "valid v1 tixel should verify against its strand"
    );
  }

  #[test]
  fn verify_tixel_wrong_strand_cid_v1_tixel_against_v2_strand() {
    let strand = strand_v2();
    let tixel = tixel_v1(); // belongs to a different strand
    let inner: &StrandSchemaVersion = &strand.0;
    let result = inner.verify_tixel(&tixel);
    assert!(
      matches!(result, Err(VerificationError::TixelNotOnStrand)),
      "tixel on wrong strand must return TixelNotOnStrand, got {:?}",
      result
    );
  }

  #[test]
  fn verify_tixel_wrong_strand_cid_v2_tixel_against_v1_strand() {
    let strand = strand_v1();
    let tixel = tixel_v2(); // belongs to a different strand
    let inner: &StrandSchemaVersion = &strand.0;
    let result = inner.verify_tixel(&tixel);
    // First check is strand_cid; TixelNotOnStrand fires
    assert!(
      matches!(result, Err(VerificationError::TixelNotOnStrand)),
      "tixel on wrong strand must return TixelNotOnStrand, got {:?}",
      result
    );
  }

  #[test]
  fn verify_tixel_v1_bad_signature_errors() {
    let strand = strand_v1();
    // INVALID_SIGNATURE_TIXELJSON has the same strand_cid as STRANDJSON but
    // the signature is bad (wrong algorithm in JWS header).
    let bad_tixel = Tixel::from_tagged_dag_json(INVALID_SIGNATURE_TIXELJSON).unwrap();
    let inner: &StrandSchemaVersion = &strand.0;
    let result = inner.verify_tixel(&bad_tixel);
    assert!(
      result.is_err(),
      "v1 tixel with bad signature must be rejected"
    );
    // Must not be TixelNotOnStrand – the strand CID matches, so the signature
    // check is what fires.
    assert!(
      !matches!(result, Err(VerificationError::TixelNotOnStrand)),
      "error must not be TixelNotOnStrand, got {:?}",
      result
    );
  }

  // --- Verifiable impls --------------------------------------------------

  #[test]
  fn ssv_verifiable_v1() {
    let strand = strand_v1();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(inner.verify().is_ok());
  }

  #[test]
  fn ssv_verifiable_v2() {
    let strand = strand_v2();
    let inner: &StrandSchemaVersion = &strand.0;
    assert!(inner.verify().is_ok());
  }

  #[test]
  fn tsv_verifiable_v1() {
    let tixel = tixel_v1();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert!(inner.verify().is_ok());
  }

  #[test]
  fn tsv_verifiable_v2() {
    let tixel = tixel_v2();
    let inner: &TixelSchemaVersion = &tixel.0;
    assert!(inner.verify().is_ok());
  }

  #[test]
  fn tsv_v1_compute_cid_works() {
    use multihash_codetable::Code;
    let tixel = tixel_v1();
    let mut inner = (**tixel.0).clone();
    // Only V1 supports compute_cid; confirm it executes without panic.
    inner.compute_cid(Code::Sha2_256);
    assert_ne!(inner.cid(), &Cid::default());
  }
}
