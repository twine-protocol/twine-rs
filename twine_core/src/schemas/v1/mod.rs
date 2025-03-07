use std::{fmt::Display, hash::Hash};

use crate::{
  crypto::{assert_cid, get_cid, get_hasher, verify_signature},
  errors::VerificationError,
  specification::Subspec,
  twine::{BackStitches, CrossStitches, Stitch},
  verify::{Verifiable, Verified},
};
use biscuit::jwk::JWK;
use ipld_core::{cid::Cid, codec::Codec, ipld::Ipld};
use multihash_codetable::Code;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor::codec::DagCborCodec;

mod chain;
mod mixin;
mod pulse;

pub type V1 = crate::specification::Specification<1>;

impl Default for V1 {
  fn default() -> Self {
    Self("twine/1.0.x".into())
  }
}

pub use chain::*;
pub use mixin::*;
pub use pulse::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ContainerV1<C: Clone + Verifiable + Send> {
  #[serde(skip)]
  cid: Cid,

  content: Verified<C>,
  signature: String,
}

impl<C> PartialEq for ContainerV1<C>
where
  C: Clone + Verifiable + Send,
{
  fn eq(&self, other: &Self) -> bool {
    self.cid == other.cid
  }
}

impl<C> Eq for ContainerV1<C> where C: Clone + Verifiable + Send {}

impl<C> Hash for ContainerV1<C>
where
  C: Clone + Verifiable + Send,
{
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    Hash::hash(&self.cid, state);
  }
}

impl Verifiable for ContainerV1<ChainContentV1> {
  fn verify(&self) -> Result<(), VerificationError> {
    let hasher = get_hasher(&self.cid)?;
    let computed = get_cid(hasher, DagCborCodec::encode_to_vec(self).unwrap());
    assert_cid(&self.cid, &computed)?;
    use multihash_codetable::MultihashDigest;
    let content_hash = hasher.digest(&DagCborCodec::encode_to_vec(&self.content).unwrap());
    self.verify_signature(&self.signature, content_hash.to_bytes())
  }
}

impl<C> ContainerV1<C>
where
  C: Clone + Verifiable + Send + Serialize + for<'de> Deserialize<'de>,
{
  pub fn compute_cid(&mut self, hasher: Code) {
    let dat = DagCborCodec::encode_to_vec(self).unwrap();
    self.cid = get_cid(hasher, dat.as_slice());
  }

  pub fn cid(&self) -> &Cid {
    &self.cid
  }

  pub fn content(&self) -> &C {
    &self.content
  }

  pub fn signature(&self) -> &str {
    &self.signature
  }
}

impl ContainerV1<ChainContentV1> {
  pub fn new_from_parts(
    hasher: Code,
    content: Verified<ChainContentV1>,
    signature: String,
  ) -> Self {
    let mut chain = Self {
      cid: Cid::default(),
      content,
      signature,
    };
    chain.compute_cid(hasher);
    chain
  }

  pub fn key(&self) -> JWK<()> {
    self.content.key.clone()
  }

  pub fn spec_str(&self) -> &str {
    self.content.specification.0.as_str()
  }

  pub fn radix(&self) -> u8 {
    self.content.links_radix as u8
  }

  pub fn version(&self) -> Version {
    self.content.specification.semver()
  }

  pub fn subspec(&self) -> Option<Subspec> {
    self.content.specification.subspec()
  }

  pub fn details(&self) -> &Ipld {
    &self.content.meta
  }

  pub fn verify_signature<T: Display>(
    &self,
    sig: T,
    content_hash: Vec<u8>,
  ) -> Result<(), VerificationError> {
    verify_signature(&self.key(), sig.to_string(), content_hash)
  }
}

impl Verifiable for ContainerV1<PulseContentV1> {
  fn verify(&self) -> Result<(), VerificationError> {
    let hasher = get_hasher(&self.cid)?;
    let computed = get_cid(hasher, DagCborCodec::encode_to_vec(self).unwrap());
    assert_cid(&self.cid, &computed)?;
    Ok(())
  }
}

impl ContainerV1<PulseContentV1> {
  pub fn new_from_parts(
    hasher: Code,
    content: Verified<PulseContentV1>,
    signature: String,
  ) -> Self {
    let mut pulse = Self {
      cid: Cid::default(),
      content,
      signature,
    };
    pulse.compute_cid(hasher);
    pulse
  }

  pub fn strand_cid(&self) -> &Cid {
    &self.content.chain
  }

  pub fn spec_str(&self) -> &str {
    "twine/1.0.x"
  }

  pub fn index(&self) -> u64 {
    self.content.index as u64
  }

  pub fn source(&self) -> &str {
    &self.content.source
  }

  pub fn payload(&self) -> &Ipld {
    &self.content.payload
  }

  pub fn back_stitches(&self) -> BackStitches {
    let strand = self.strand_cid().clone();
    BackStitches::new(strand, self.content.links.clone())
  }

  pub fn cross_stitches(&self) -> CrossStitches {
    CrossStitches::new(self.content.mixins.iter().cloned().collect::<Vec<Stitch>>())
  }
}
