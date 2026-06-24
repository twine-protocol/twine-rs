//! The `v1` module contains the data structures for
//! describing version 1 schemas
//!
//! In version 1, Strands were called Chains,
//! Tixels were called Pulses, and Stitches were called Mixins.
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
use multihash_codetable::{Code, Multihash};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor::codec::DagCborCodec;

mod chain;
mod mixin;
mod pulse;

/// The [`crate::specification::Specification`] type for version 1 schemas
pub type V1 = crate::specification::Specification<1>;

impl Default for V1 {
  fn default() -> Self {
    Self("twine/1.0.x".into())
  }
}

pub use chain::*;
pub use mixin::*;
pub use pulse::*;

/// A container for a chain or pulse
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
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    let hasher = get_hasher(&self.cid)?;
    let computed = get_cid(hasher, DagCborCodec::encode_to_vec(self).unwrap());
    assert_cid(&self.cid, &computed)?;
    use multihash_codetable::MultihashDigest;
    let content_hash = hasher.digest(&DagCborCodec::encode_to_vec(&self.content).unwrap());
    self.verify_signature(&self.signature, content_hash)
  }
}

impl<C> ContainerV1<C>
where
  C: Clone + Verifiable + Send + Serialize + for<'de> Deserialize<'de>,
{
  /// Compute the CID using the given hasher
  pub fn compute_cid(&mut self, hasher: Code) {
    let dat = DagCborCodec::encode_to_vec(self).unwrap();
    self.cid = get_cid(hasher, dat.as_slice());
  }

  /// Get the CID
  pub fn cid(&self) -> &Cid {
    &self.cid
  }

  /// Get the content
  pub fn content(&self) -> &C {
    &self.content
  }

  /// Get the signature
  pub fn signature(&self) -> &str {
    &self.signature
  }
}

impl ContainerV1<ChainContentV1> {
  /// Create a new Chain Container
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

  /// Get the public key JWK
  pub fn key(&self) -> JWK<()> {
    self.content.key.clone()
  }

  /// Get the specification string
  pub fn spec_str(&self) -> &str {
    self.content.specification.0.as_str()
  }

  /// Get the radix value
  pub fn radix(&self) -> u8 {
    self.content.links_radix as u8
  }

  /// Get the version
  pub fn version(&self) -> Version {
    self.content.specification.semver()
  }

  /// Get the subspec if it exists
  pub fn subspec(&self) -> Option<Subspec> {
    self.content.specification.subspec()
  }

  /// Get the details
  pub fn details(&self) -> &Ipld {
    &self.content.meta
  }

  /// Check a given signature using this Chain's public key
  pub fn verify_signature<T: Display>(
    &self,
    sig: T,
    content_hash: Multihash,
  ) -> Result<(), VerificationError> {
    verify_signature(&self.key(), sig.to_string(), content_hash.to_bytes())
  }
}

impl Verifiable for ContainerV1<PulseContentV1> {
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    let hasher = get_hasher(&self.cid)?;
    let computed = get_cid(hasher, DagCborCodec::encode_to_vec(self).unwrap());
    assert_cid(&self.cid, &computed)?;
    Ok(())
  }
}

impl ContainerV1<PulseContentV1> {
  /// Create a new Pulse Container
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

  /// Get the strand CID
  pub fn strand_cid(&self) -> &Cid {
    &self.content.chain
  }

  /// Get the specification string
  pub fn spec_str(&self) -> &str {
    "twine/1.0.x"
  }

  /// Get the index
  pub fn index(&self) -> u64 {
    self.content.index as u64
  }

  /// Get the source field value
  pub fn source(&self) -> &str {
    &self.content.source
  }

  /// Get the details
  pub fn payload(&self) -> &Ipld {
    &self.content.payload
  }

  /// Get the back stitches
  pub fn back_stitches(&self) -> BackStitches {
    let strand = self.strand_cid().clone();
    BackStitches::new(strand, self.content.links.clone())
  }

  /// Get the cross stitches
  pub fn cross_stitches(&self) -> CrossStitches {
    CrossStitches::new(self.content.mixins.iter().cloned().collect::<Vec<Stitch>>())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    schemas::TixelSchemaVersion,
    test::{STRANDJSON, TIXELJSON},
    twine::{Strand, Tixel, TwineBlock},
  };

  fn strand_v1() -> Strand {
    Strand::from_tagged_dag_json(STRANDJSON).unwrap()
  }
  fn tixel_v1() -> Tixel {
    Tixel::from_tagged_dag_json(TIXELJSON).unwrap()
  }

  // --- V1::default (lines 30-32) -----------------------------------------

  #[test]
  fn v1_default_is_twine_1_0_x() {
    // V1::default() must return the canonical v1 spec string.
    let spec = V1::default();
    assert_eq!(spec.0.as_str(), "twine/1.0.x");
  }

  #[test]
  fn v1_default_semver_is_1_0_0() {
    let spec = V1::default();
    assert_eq!(spec.semver(), semver::Version::new(1, 0, 0));
  }

  // --- ContainerV1::Hash impl (lines 65-67) ------------------------------

  #[test]
  fn container_v1_chain_hash_equal_for_same_cid() {
    // Two clones of the same container must hash identically.
    use crate::schemas::StrandSchemaVersion;
    let strand = strand_v1();
    let a = match &**strand.0 {
      StrandSchemaVersion::V1(c) => c.clone(),
      _ => panic!("expected V1"),
    };
    let b = a.clone();
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut ha = DefaultHasher::new();
    let mut hb = DefaultHasher::new();
    a.hash(&mut ha);
    b.hash(&mut hb);
    assert_eq!(ha.finish(), hb.finish(), "identical V1 containers must hash equal");
  }

  #[test]
  fn container_v1_pulse_hash_equal_for_same_cid() {
    // Same check for the PulseContentV1 container.
    let tixel = tixel_v1();
    let a = match &**tixel.0 {
      TixelSchemaVersion::V1(c) => c.clone(),
      _ => panic!("expected V1"),
    };
    let b = a.clone();
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut ha = DefaultHasher::new();
    let mut hb = DefaultHasher::new();
    a.hash(&mut ha);
    b.hash(&mut hb);
    assert_eq!(ha.finish(), hb.finish(), "identical V1 pulse containers must hash equal");
  }

  // --- ContainerV1<PulseContentV1>::source (lines 206-208) ---------------

  #[test]
  fn pulse_container_source_matches_fixture() {
    // The v1 tixel fixture has "source": "random.colorado.edu".
    let tixel = tixel_v1();
    let pulse = match &**tixel.0 {
      TixelSchemaVersion::V1(c) => c.clone(),
      _ => panic!("expected V1"),
    };
    assert_eq!(
      pulse.source(),
      "random.colorado.edu",
      "source must match the fixture value"
    );
  }

  // --- ContainerV1 PartialEq impl (lines 54-56) --------------------------

  #[test]
  fn container_v1_chain_eq_reflexive() {
    use crate::schemas::StrandSchemaVersion;
    let strand = strand_v1();
    let a = match &**strand.0 {
      StrandSchemaVersion::V1(c) => c.clone(),
      _ => panic!("expected V1"),
    };
    let b = a.clone();
    assert_eq!(a, b, "cloned V1 chain container must equal itself");
  }

  #[test]
  fn container_v1_pulse_eq_reflexive() {
    let tixel = tixel_v1();
    let a = match &**tixel.0 {
      TixelSchemaVersion::V1(c) => c.clone(),
      _ => panic!("expected V1"),
    };
    let b = a.clone();
    assert_eq!(a, b, "cloned V1 pulse container must equal itself");
  }
}
