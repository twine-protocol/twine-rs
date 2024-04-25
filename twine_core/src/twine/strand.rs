use crate::{crypto::verify_signature, schemas::v1, specification::{Specification, Subspec}};
use josekit::jwk::Jwk;
use semver::Version;
use serde::{Serialize, Deserialize};
use super::{container::TwineContainer, Tixel};
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

  pub fn verify_signature<C: Clone + Serialize + for<'de> Deserialize<'de>>(&self, twine: &TwineContainer<C>) -> Result<(), VerificationError> {
    verify_signature(self.key(), twine.signature(), twine.content_hash())
  }
}

