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

pub type V2 = crate::specification::Specification<2>;

impl Default for V2 {
  fn default() -> Self {
    Self("twine/2.0.0".into())
  }
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerFields<C: Clone + Send + Verifiable> {
  #[serde(rename = "c")]
  content: Verified<ContentV2<C>>,
  #[serde(rename = "s")]
  signature: Bytes,
}

#[derive(Debug, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ContainerV2<C: Clone + Send + Verifiable> {
  #[serde(skip)]
  cid: Cid,

  #[serde(flatten)]
  fields: ContainerFields<C>,
}

impl<C> ContainerV2<C>
where
  C: Clone + Send + Verifiable + Serialize,
{
  pub fn new_from_parts(content: Verified<ContentV2<C>>, signature: Signature) -> Self {
    let fields = ContainerFields { content, signature };

    let cid = fields.content.code().get_cid(&fields).unwrap();

    ContainerV2 { cid, fields }
  }
}

impl<C> ContainerV2<C>
where
  C: Clone + Send + Verifiable + Serialize,
{
  pub fn cid(&self) -> &Cid {
    &self.cid
  }

  pub fn version(&self) -> Version {
    self.fields.content.specification.semver()
  }

  pub fn spec_str(&self) -> &str {
    self.fields.content.specification.0.as_str()
  }

  pub fn subspec(&self) -> Option<crate::specification::Subspec> {
    self.fields.content.specification.subspec()
  }

  pub fn signature(&self) -> Signature {
    self.fields.signature.clone()
  }

  pub fn content_bytes(&self) -> Result<Bytes, VerificationError> {
    crypto_serialize(&self.fields.content)
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

impl<C> Deref for ContainerV2<C>
where
  C: Clone + Send + Verifiable,
{
  type Target = ContainerFields<C>;

  fn deref(&self) -> &Self::Target {
    &self.fields
  }
}

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
    let fields = ContainerFields::<T>::deserialize(deserializer)?;
    // now use the content code to create the cid
    let cid = fields
      .content
      .code()
      .get_cid(&fields)
      .map_err(|e| serde::de::Error::custom(format!("Failed to create CID: {:?}", e)))?;

    Ok(ContainerV2 { cid, fields })
  }
}

pub type StrandContainerV2 = ContainerV2<StrandFields>;
pub type TixelContainerV2 = ContainerV2<TixelFields>;

impl StrandContainerV2 {
  pub fn key(&self) -> &PublicKey {
    &self.fields.content.key
  }

  pub fn radix(&self) -> u8 {
    self.fields.content.radix
  }

  pub fn details(&self) -> &Ipld {
    &self.fields.content.details
  }

  pub fn expiry(&self) -> Option<DateTime<Utc>> {
    self.fields.content.expiry
  }
}

impl Verifiable for StrandContainerV2 {
  fn verify(&self) -> Result<(), VerificationError> {
    self
      .key()
      .verify(self.signature(), &self.content_bytes()?)?;
    Ok(())
  }
}

impl TixelContainerV2 {
  pub fn index(&self) -> u64 {
    self.fields.content.index
  }

  pub fn strand_cid(&self) -> &Cid {
    &self.fields.content.strand
  }

  pub fn cross_stitches(&self) -> CrossStitches {
    (*self.fields.content.cross_stitches).clone()
  }

  pub fn back_stitches(&self) -> crate::twine::BackStitches {
    // checked in verify method
    BackStitches::try_new_from_condensed(
      *self.strand_cid(),
      self.fields.content.back_stitches.clone(),
    )
    .unwrap()
  }

  pub fn drop_index(&self) -> u64 {
    self.fields.content.drop
  }

  pub fn payload(&self) -> &Ipld {
    &self.fields.content.payload
  }
}

impl Verifiable for TixelContainerV2 {
  fn verify(&self) -> Result<(), VerificationError> {
    // currently there are no further verifications to do for the tixel alone
    Ok(())
  }
}
