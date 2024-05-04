use crate::{crypto::verify_signature, schemas::v1, specification::Subspec, verify::Verifiable};
use josekit::jwk::Jwk;
use semver::Version;
use libipld::ipld::Ipld;
use serde::{Serialize, Deserialize};
use super::{container::{TwineContainer, TwineContent}, Stitch, Tixel};
use crate::errors::VerificationError;

pub type Strand = TwineContainer<StrandContent>;

impl Verifiable for Strand {
  fn verify(&self) -> Result<(), VerificationError> {
    self.verify_own_signature()?;
    Ok(())
  }
}

impl Strand {
  pub fn key(&self) -> Jwk {
    self.content().key()
  }

  pub fn version(&self) -> Version {
    self.content().version()
  }

  pub fn subspec(&self) -> Option<Subspec> {
    self.content().subspec()
  }

  pub fn radix(&self) -> u64 {
    self.content().radix()
  }

  pub fn details(&self) -> Ipld {
    self.content().details()
  }

  pub fn verify_tixel(&self, tixel: &Tixel) -> Result<(), VerificationError> {
    // also verify that this tixel belongs to the strand
    if tixel.strand_cid() != self.cid() {
      return Err(VerificationError::TixelNotOnStrand);
    }
    self.content().verify_signature(tixel)
  }

  pub fn verify_own_signature(&self) -> Result<(), VerificationError> {
    self.content().verify_signature(self)
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum StrandContent {
  V1(v1::ChainContentV1),
}

impl Verifiable for StrandContent {
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      StrandContent::V1(v) => v.verify(),
    }
  }
}

impl TwineContent for StrandContent {
  fn back_stitches(&self) -> Vec<Stitch> {
    vec![]
  }

  fn cross_stitches(&self) -> Vec<Stitch> {
    match self {
      StrandContent::V1(v) => v.mixins.iter().cloned().collect(),
    }
  }
}

impl StrandContent {
  pub fn key(&self) -> Jwk {
    match self {
      StrandContent::V1(v) => v.key.clone(),
    }
  }

  pub fn radix(&self) -> u64 {
    match self {
      StrandContent::V1(v) => v.links_radix as u64,
    }
  }

  pub fn version(&self) -> Version {
    match self {
      StrandContent::V1(v) => v.specification.semver(),
    }
  }

  pub fn subspec(&self) -> Option<Subspec> {
    match self {
      StrandContent::V1(v) => v.specification.subspec(),
    }
  }

  pub fn details(&self) -> Ipld {
    match self {
      StrandContent::V1(v) => v.meta.clone(),
    }
  }

  pub fn verify_signature<C: TwineContent + Serialize + for<'de> Deserialize<'de>>(&self, twine: &TwineContainer<C>) -> Result<(), VerificationError> {
    verify_signature(self.key(), twine.signature(), twine.content_hash())
  }
}

