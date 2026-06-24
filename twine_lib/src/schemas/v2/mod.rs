//! The `v2` module contains the data structures for
//! describing version 2 schemas
use crate::crypto::get_cid;
use crate::crypto::{crypto_serialize, PublicKey, Signature};
use crate::errors::VerificationError;
use crate::twine::{BackStitches, CrossStitches};
use crate::verify::{Verifiable, Verified};
use crate::Ipld;
use crate::{Bytes, Cid};
use chrono::{DateTime, Utc};
use multihash_codetable::Code;
use semver::Version;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

mod content;
mod strand;
mod tixel;

use content::*;
pub use strand::{StrandContentV2, StrandFields};
pub use tixel::{TixelContentV2, TixelFields};

/// The version 2 [`Specification`](crate::specification::Specification)
pub type V2 = crate::specification::Specification<2>;

impl Default for V2 {
  fn default() -> Self {
    Self("twine/2.0.0".into())
  }
}

/// Helper to serialize [`Code`] as a u64
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(try_from = "u64", into = "u64")]
pub struct HashCode(pub Code);

impl Deref for HashCode {
  type Target = Code;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl HashCode {
  /// Get the [`Cid`] for the given input
  pub fn get_cid<S: Serialize>(
    &self,
    input: S,
  ) -> Result<Cid, serde_ipld_dagcbor::EncodeError<std::collections::TryReserveError>> {
    let dat = crypto_serialize(input)?;
    Ok(get_cid(**self, dat))
  }
}

impl From<Code> for HashCode {
  fn from(value: Code) -> Self {
    HashCode(value)
  }
}

impl TryFrom<u64> for HashCode {
  type Error = multihash_derive::UnsupportedCode;

  fn try_from(value: u64) -> Result<Self, Self::Error> {
    Code::try_from(value).map(HashCode)
  }
}

impl From<HashCode> for u64 {
  fn from(value: HashCode) -> Self {
    value.0.into()
  }
}

/// General container for a version 2 schema
///
/// Stores the content and signature, and computes and stores the CID
/// when deserialized. The `cid` is derived, not transmitted: serializing
/// a container yields exactly the wire form `{ "c": content, "s": signature }`,
/// which is what the CID commits to.
#[derive(Debug, Serialize, Clone)]
pub struct ContainerV2<C: Clone + Send + Verifiable> {
  #[serde(skip)]
  cid: Cid,

  #[serde(rename = "c")]
  content: Verified<ContentV2<C>>,
  #[serde(rename = "s")]
  signature: Bytes,
}

impl<C> ContainerV2<C>
where
  C: Clone + Send + Verifiable + Serialize,
{
  /// Create a new container from its parts
  pub fn new_from_parts(content: Verified<ContentV2<C>>, signature: Signature) -> Self {
    let mut container = ContainerV2 {
      cid: Cid::default(),
      content,
      signature,
    };
    // cid commits to the serialized container (cid field is skipped)
    container.cid = container.content.code().get_cid(&container).unwrap();
    container
  }
}

impl<C> ContainerV2<C>
where
  C: Clone + Send + Verifiable + Serialize,
{
  /// Get the CID of the container
  pub fn cid(&self) -> &Cid {
    &self.cid
  }

  /// Get the version
  pub fn version(&self) -> Version {
    self.content.specification.semver()
  }

  /// Get the spec string
  pub fn spec_str(&self) -> &str {
    self.content.specification.0.as_str()
  }

  /// Get the subspec if it exists
  pub fn subspec(&self) -> Option<crate::specification::Subspec> {
    self.content.specification.subspec()
  }

  /// Get the signature
  pub fn signature(&self) -> Signature {
    self.signature.clone()
  }

  /// Get the serialized content as bytes
  pub fn content_bytes(&self) -> Result<Bytes, VerificationError> {
    crypto_serialize(&self.content)
      .map_err(|e| VerificationError::General(e.to_string()))
      .map(Bytes)
  }
}

impl<C> PartialEq for ContainerV2<C>
where
  C: Clone + Send + Verifiable,
{
  fn eq(&self, other: &Self) -> bool {
    self.cid == other.cid
  }
}

impl<C> Eq for ContainerV2<C> where C: Clone + Send + Verifiable {}

impl<C> Hash for ContainerV2<C>
where
  C: Clone + Send + Verifiable,
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    Hash::hash(&self.cid, state);
  }
}

impl<'de, T> Deserialize<'de> for ContainerV2<T>
where
  T: Clone + Send + Verifiable + Serialize + for<'a> Deserialize<'a>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    // The wire shape; `cid` is derived below, not transmitted.
    #[derive(Serialize, Deserialize)]
    #[serde(bound(deserialize = "T: Serialize + for<'a> Deserialize<'a>"))]
    #[serde(deny_unknown_fields)]
    struct Wire<T: Clone + Send + Verifiable> {
      #[serde(rename = "c")]
      content: Verified<ContentV2<T>>,
      #[serde(rename = "s")]
      signature: Bytes,
    }
    let wire = Wire::<T>::deserialize(deserializer)?;
    let cid = wire.content
      .code()
      .get_cid(&wire)
      .map_err(|e| serde::de::Error::custom(format!("Failed to create CID: {:?}", e)))?;

    let Wire { content, signature } = wire;
    let container = ContainerV2 {
      cid,
      content,
      signature,
    };
    Ok(container)
  }
}

/// The container for a version 2 strand
pub type StrandContainerV2 = ContainerV2<StrandFields>;
/// The container for a version 2 tixel
pub type TixelContainerV2 = ContainerV2<TixelFields>;

impl StrandContainerV2 {
  /// Get the public key of the strand
  pub fn key(&self) -> &PublicKey {
    &self.content.key
  }

  /// Get the radix of the strand
  pub fn radix(&self) -> u8 {
    self.content.radix
  }

  /// Get the details of the strand
  pub fn details(&self) -> &Ipld {
    &self.content.details
  }

  /// Get the expiry date of the strand if it is set
  pub fn expiry(&self) -> Option<DateTime<Utc>> {
    self.content.expiry
  }
}

impl Verifiable for StrandContainerV2 {
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    self
      .key()
      .verify(self.signature(), &self.content_bytes()?)?;
    Ok(())
  }
}

impl TixelContainerV2 {
  /// Get the index of the tixel
  pub fn index(&self) -> u64 {
    self.content.index
  }

  /// Get the strand CID of the tixel
  pub fn strand_cid(&self) -> &Cid {
    &self.content.strand
  }

  /// Get the cross stitches of the tixel
  pub fn cross_stitches(&self) -> CrossStitches {
    (*self.content.cross_stitches).clone()
  }

  /// Get the back stitches of the tixel
  pub fn back_stitches(&self) -> crate::twine::BackStitches {
    // checked in verify method
    BackStitches::try_new_from_condensed(
      *self.strand_cid(),
      self.content.back_stitches.clone(),
    )
    .unwrap()
  }

  /// Get the drop index of the tixel
  pub fn drop_index(&self) -> u64 {
    self.content.drop
  }

  /// Get the payload of the tixel
  pub fn payload(&self) -> &Ipld {
    &self.content.payload
  }
}

impl Verifiable for TixelContainerV2 {
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    // currently there are no further verifications to do for the tixel alone
    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    test::{STRAND_V2_JSON, TIXEL_V2_JSON},
    twine::{Strand, Tixel, TwineBlock},
  };
  use semver::Version;

  // --- helpers -----------------------------------------------------------

  fn strand_v2() -> Strand {
    Strand::from_tagged_dag_json(STRAND_V2_JSON).unwrap()
  }
  fn tixel_v2() -> Tixel {
    Tixel::from_tagged_dag_json(TIXEL_V2_JSON).unwrap()
  }

  // Get the raw StrandContainerV2 from a loaded strand
  fn strand_container_v2() -> StrandContainerV2 {
    use crate::schemas::StrandSchemaVersion;
    let strand = strand_v2();
    match &**strand.0 {
      StrandSchemaVersion::V2(c) => c.clone(),
      _ => panic!("expected V2 strand"),
    }
  }

  // Get the raw TixelContainerV2 from a loaded tixel
  fn tixel_container_v2() -> TixelContainerV2 {
    use crate::schemas::TixelSchemaVersion;
    let tixel = tixel_v2();
    match &**tixel.0 {
      TixelSchemaVersion::V2(c) => c.clone(),
      _ => panic!("expected V2 tixel"),
    }
  }

  // --- ContainerV2 CID derivation ----------------------------------------

  #[test]
  fn strand_container_v2_cid_is_derived_from_content() {
    // CID is computed at deserialize time from the content; must be non-default.
    let c = strand_container_v2();
    assert_ne!(c.cid(), &Cid::default(), "CID must be derived, not default");
    // Re-serializing and re-loading gives the same CID (round-trip)
    let strand = strand_v2();
    let json = strand.tagged_dag_json();
    let strand2 = Strand::from_tagged_dag_json(&json).unwrap();
    assert_eq!(strand.cid(), strand2.cid());
  }

  #[test]
  fn tixel_container_v2_cid_is_derived_from_content() {
    let c = tixel_container_v2();
    assert_ne!(c.cid(), &Cid::default(), "CID must be derived, not default");
    let tixel = tixel_v2();
    let json = tixel.tagged_dag_json();
    let tixel2 = Tixel::from_tagged_dag_json(&json).unwrap();
    assert_eq!(tixel.cid(), tixel2.cid());
  }

  // --- StrandContainerV2::verify (self-signature) ------------------------

  #[test]
  fn strand_container_v2_verify_passes_for_valid_fixture() {
    let c = strand_container_v2();
    assert!(
      c.verify().is_ok(),
      "valid StrandContainerV2 must pass self-verification"
    );
  }

  #[test]
  fn strand_container_v2_verify_fails_for_tampered_signature() {
    // Tamper the signature by replacing it with zeros in the JSON and
    // then verifying the container. Strand deserialization calls verify(),
    // so a bad signature is caught either during deserialization or on an
    // explicit verify() call.
    //
    // The ED25519 signature in STRAND_V2_JSON is base64-encoded in the "s"
    // field. We replace it with 64 zero bytes (encoded as base64) to simulate
    // a forged / corrupted signature.
    let zero_sig_b64 =
      "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let tampered = STRAND_V2_JSON.replace(
      "hN5hlT+3+zwJzgmrej8LvtPrAnRsf0c2Qo8xZE0Bj0uY0Tudhi9CbBx/5AjPmceyYGifWb0uw5SZRLMDS15YBA",
      zero_sig_b64,
    );
    // Either deserialization fails (Verifiable catches it) or if the JSON
    // format happens to be accepted, the container's verify() must fail.
    let strand_result = Strand::from_tagged_dag_json(&tampered);
    match strand_result {
      Err(_) => {
        // Deserialization itself rejected the tampered fixture — correct.
      }
      Ok(strand) => {
        use crate::schemas::StrandSchemaVersion;
        match &**strand.0 {
          StrandSchemaVersion::V2(c) => {
            let result = c.verify();
            assert!(result.is_err(), "tampered v2 strand must fail verification");
          }
          _ => panic!("expected V2 strand variant"),
        }
      }
    }
  }

  // --- TixelContainerV2::verify ------------------------------------------

  #[test]
  fn tixel_container_v2_verify_returns_ok() {
    // TixelContainerV2::verify has no checks of its own; must return Ok.
    let c = tixel_container_v2();
    assert!(c.verify().is_ok());
  }

  // --- StrandContainerV2 accessors ---------------------------------------

  #[test]
  fn strand_v2_key_accessible() {
    let c = strand_container_v2();
    let _ = c.key();
  }

  #[test]
  fn strand_v2_radix_32() {
    let c = strand_container_v2();
    assert_eq!(c.radix(), 32);
  }

  #[test]
  fn strand_v2_details_accessible() {
    let c = strand_container_v2();
    let _ = c.details();
  }

  #[test]
  fn strand_v2_expiry_none_for_null_fixture() {
    let c = strand_container_v2();
    assert_eq!(c.expiry(), None);
  }

  #[test]
  fn strand_v2_version_is_2_0_0() {
    let c = strand_container_v2();
    assert_eq!(c.version(), Version::new(2, 0, 0));
  }

  #[test]
  fn strand_v2_spec_str_contains_twine() {
    let c = strand_container_v2();
    assert!(c.spec_str().starts_with("twine/"));
  }

  #[test]
  fn strand_v2_subspec_some() {
    let c = strand_container_v2();
    assert!(c.subspec().is_some());
  }

  #[test]
  fn strand_v2_signature_nonempty() {
    let c = strand_container_v2();
    assert!(!c.signature().is_empty());
  }

  #[test]
  fn strand_v2_content_bytes_nonempty() {
    let c = strand_container_v2();
    assert!(c.content_bytes().is_ok());
    assert!(!c.content_bytes().unwrap().is_empty());
  }

  // --- TixelContainerV2 accessors ----------------------------------------

  #[test]
  fn tixel_v2_index_0() {
    let c = tixel_container_v2();
    assert_eq!(c.index(), 0);
  }

  #[test]
  fn tixel_v2_strand_cid_non_default() {
    let c = tixel_container_v2();
    assert_ne!(c.strand_cid(), &Cid::default());
  }

  #[test]
  fn tixel_v2_cross_stitches_empty() {
    let c = tixel_container_v2();
    // fixture has no cross-stitches
    assert_eq!(c.cross_stitches().len(), 0);
  }

  #[test]
  fn tixel_v2_back_stitches_empty() {
    let c = tixel_container_v2();
    // first tixel (index 0) has no back-stitches
    assert_eq!(c.back_stitches().len(), 0);
  }

  #[test]
  fn tixel_v2_drop_index_0() {
    let c = tixel_container_v2();
    assert_eq!(c.drop_index(), 0);
  }

  #[test]
  fn tixel_v2_payload_accessible() {
    let c = tixel_container_v2();
    let _ = c.payload();
  }

  #[test]
  fn tixel_v2_version_is_2_0_0() {
    let c = tixel_container_v2();
    assert_eq!(c.version(), Version::new(2, 0, 0));
  }

  #[test]
  fn tixel_v2_spec_str_nonempty() {
    let c = tixel_container_v2();
    assert!(!c.spec_str().is_empty());
  }

  #[test]
  fn tixel_v2_subspec_some() {
    let c = tixel_container_v2();
    assert!(c.subspec().is_some());
  }

  #[test]
  fn tixel_v2_signature_nonempty() {
    let c = tixel_container_v2();
    assert!(!c.signature().is_empty());
  }
}
