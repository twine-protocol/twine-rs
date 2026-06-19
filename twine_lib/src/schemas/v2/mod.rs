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
