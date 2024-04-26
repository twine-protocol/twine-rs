use crate::verify::Verifiable;
use crate::{errors::VerificationError, schemas::v1};
use libipld::Cid;
use libipld::Ipld;
use serde::{Serialize, Deserialize};
use super::container::TwineContent;
use super::Stitch;
use super::{container::TwineContainer, Strand};

pub type Tixel = TwineContainer<TixelContent>;

impl Tixel {
  pub fn strand(&self) -> Cid {
    self.content().strand()
  }

  pub fn payload(&self) -> Ipld {
    self.content().payload()
  }

  pub fn source(&self) -> String {
    self.content().source()
  }

  pub fn verify_signature(&self, strand: &Strand) -> Result<(), VerificationError> {
    strand.verify_signature(self)
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum TixelContent {
  V1(v1::PulseContentV1),
}

impl Verifiable for TixelContent {
  fn verify(&self) -> Result<(), VerificationError> {
    match self {
      TixelContent::V1(v) => v.verify(),
    }
  }
}

impl TwineContent for TixelContent {
  fn back_stitches(&self) -> Vec<Stitch> {
    let links: &Vec<Cid> = match self {
      TixelContent::V1(v) => &v.links,
    };

    let strand = self.strand();
    links.iter().map(|&tixel| Stitch{ strand, tixel }).collect()
  }

  fn cross_stitches(&self) -> Vec<Stitch> {
    match self {
      TixelContent::V1(v) => v.mixins.iter().cloned().collect(),
    }
  }
}

impl TixelContent {
  pub fn strand(&self) -> Cid {
    match self {
      TixelContent::V1(v) => v.chain,
    }
  }

  pub fn payload(&self) -> Ipld {
    match self {
      TixelContent::V1(v) => v.payload.clone(),
    }
  }

  pub fn source(&self) -> String {
    match self {
      TixelContent::V1(v) => v.source.clone(),
    }
  }
}
