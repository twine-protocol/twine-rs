use crate::{crypto::verify_signature, schemas::v1, specification::Subspec};
use josekit::jwk::Jwk;
use semver::Version;
use serde::{Serialize, Deserialize};
use super::{container::{TwineContainer, TwineContent}, Stitch, Tixel};
use crate::errors::VerificationError;

pub type Strand = TwineContainer<StrandContent>;

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

  pub fn verify(&self) -> Result<(), VerificationError> {
    self.content().verify()?;
    self.verify_own_signature()?;
    Ok(())
  }

  pub fn verify_signature(&self, tixel: &Tixel) -> Result<(), VerificationError> {
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

impl TwineContent for StrandContent {
  fn loop_stitches(&self) -> Vec<Stitch> {
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

  pub fn verify(&self) -> Result<(), VerificationError> {
    match self {
      StrandContent::V1(v) => v.verify(),
    }
  }

  pub fn verify_signature<C: TwineContent + Serialize + for<'de> Deserialize<'de>>(&self, twine: &TwineContainer<C>) -> Result<(), VerificationError> {
    verify_signature(self.key(), twine.signature(), twine.content_hash())
  }
}

