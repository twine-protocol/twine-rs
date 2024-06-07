use std::collections::HashMap;
use std::ops::Deref;
use multihash_codetable::Code;
use serde::Deserializer;
use serde::{Serialize, Deserialize};
use crate::crypto::crypto_serialize;
use crate::crypto::get_cid;
use crate::Cid;
use crate::Ipld;
use crate::verify::{Verified, Verifiable};

mod content;
mod tixel;
mod strand;

use content::*;
pub use tixel::TixelContentV2;
pub use strand::StrandContentV2;

pub type Bytes = Vec<u8>;
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
pub enum SignatureAlgorithm {
  RSA,
  ECDSA,
  ED25519,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicKey {
  alg: SignatureAlgorithm,
  key: Bytes,
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

impl<C> Deref for ContainerV2<C> where C: Clone + Send + Verifiable {
  type Target = ContainerFields<C>;

  fn deref(&self) -> &Self::Target {
    &self.fields
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
