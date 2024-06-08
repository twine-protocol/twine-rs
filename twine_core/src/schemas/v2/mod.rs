use std::hash::{Hash, Hasher};
use std::ops::Deref;
use multihash_codetable::Code;
use semver::Version;
use serde::Deserializer;
use serde::{Serialize, Deserialize};
use crate::crypto::{crypto_serialize, PublicKey, Signature};
use crate::crypto::get_cid;
use crate::errors::VerificationError;
use crate::twine::{BackStitches, CrossStitches};
use crate::{Bytes, Cid};
use crate::Ipld;
use crate::verify::{Verified, Verifiable};

mod content;
mod tixel;
mod strand;

use content::*;
pub use tixel::{TixelContentV2, TixelFields};
pub use strand::{StrandContentV2, StrandFields};

use super::{StrandContainer, TixelContainer, TwineContainer};

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
  pub fn get_cid<S: Serialize>(&self, input: S) -> Result<Cid, serde_ipld_dagcbor::EncodeError<std::collections::TryReserveError>> {
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

impl<C> ContainerV2<C> where C: Clone + Send + Verifiable + Serialize {
  pub fn new_from_parts(content: Verified<ContentV2<C>>, signature: Signature) -> Self {
    let fields = ContainerFields {
      content,
      signature,
    };

    let cid = fields.content.code().get_cid(&fields).unwrap();

    ContainerV2 {
      cid,
      fields,
    }
  }
}

impl<C> TwineContainer for ContainerV2<C> where C: Clone + Send + Verifiable + Serialize {
  fn cid(&self) -> &Cid {
    &self.cid
  }

  fn version(&self) -> Version {
    self.fields.content.specification.semver()
  }

  fn subspec(&self) -> Option<crate::specification::Subspec> {
    self.fields.content.specification.subspec()
  }

  fn signature(&self) -> Signature {
    self.fields.signature.clone()
  }

  fn content_bytes(&self) -> Result<Bytes, VerificationError> {
    crypto_serialize(&self.fields.content)
      .map_err(|e| VerificationError::General(e.to_string()))
  }
}

impl<C> PartialEq for ContainerV2<C> where C: Clone + Send + Verifiable {
  fn eq(&self, other: &Self) -> bool {
    self.cid == other.cid
  }
}

impl<C> Eq for ContainerV2<C> where C: Clone + Send + Verifiable {}

impl<C> Deref for ContainerV2<C> where C: Clone + Send + Verifiable {
  type Target = ContainerFields<C>;

  fn deref(&self) -> &Self::Target {
    &self.fields
  }
}

impl<C> Hash for ContainerV2<C> where C: Clone + Send + Verifiable {
  fn hash<H: Hasher>(&self, state: &mut H) {
    Hash::hash(&self.cid, state);
  }
}

impl<'de, T> Deserialize<'de> for ContainerV2<T> where T: Clone + Send + Verifiable + Serialize + for<'a> Deserialize<'a> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let fields = ContainerFields::<T>::deserialize(deserializer)?;
    // now use the content code to create the cid
    let cid = fields.content.code().get_cid(&fields).map_err(
      |e| serde::de::Error::custom(format!("Failed to create CID: {:?}", e))
    )?;

    Ok(ContainerV2 {
      cid,
      fields,
    })
  }
}

pub type StrandContainerV2 = ContainerV2<StrandFields>;
pub type TixelContainerV2 = ContainerV2<TixelFields>;

impl StrandContainer for StrandContainerV2 {
  fn key(&self) -> &PublicKey {
    &self.fields.content.key
  }

  fn radix(&self) -> u8 {
    self.fields.content.radix
  }

  fn details(&self) -> &Ipld {
    &self.fields.content.details
  }
}

impl Verifiable for StrandContainerV2 {
  fn verify(&self) -> Result<(), VerificationError> {
    self.verify_signature(&self.key())
  }
}

impl TixelContainer for TixelContainerV2 {
  fn index(&self) -> u64 {
    self.fields.content.index
  }

  fn strand_cid(&self) -> &Cid {
    &self.fields.content.strand
  }

  fn cross_stitches(&self) -> CrossStitches {
    (*self.fields.content.cross_stitches).clone()
  }

  fn back_stitches(&self) -> crate::twine::BackStitches {
    // checked in verify method
    BackStitches::try_new_from_condensed(*self.strand_cid(), self.fields.content.back_stitches.clone()).unwrap()
  }

  fn drop(&self) -> u64 {
    self.fields.content.drop
  }

  fn payload(&self) -> &Ipld {
    &self.fields.content.payload
  }
}
