use crate::specification::Subspec;
use crate::verify::Verifiable;
use crate::{errors::VerificationError, schemas::v1};
use libipld::Cid;
use libipld::Ipld;
use semver::Version;
use serde::{Serialize, Deserialize};
use super::container::TwineContent;
use super::Stitch;
use super::{container::TwineContainer, Strand};

pub type Tixel = TwineContainer<TixelContent>;

impl Tixel {
  pub fn strand_cid(&self) -> Cid {
    self.content().strand_cid()
  }

  pub fn index(&self) -> u64 {
    self.content().index()
  }

  pub fn version(&self) -> Version {
    self.content().version()
  }

  pub fn subspec(&self) -> Option<Subspec> {
    self.content().subspec()
  }

  pub fn payload(&self) -> Ipld {
    self.content().payload()
  }

  pub fn source(&self) -> String {
    self.content().source()
  }

  pub fn verify_with(&self, strand: &Strand) -> Result<(), VerificationError> {
    strand.verify_tixel(self)
  }

  pub fn previous(&self) -> Stitch {
    self.back_stitches().first().unwrap().to_owned()
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

    let strand = self.strand_cid();
    links.iter().map(|&tixel| Stitch{ strand, tixel }).collect()
  }

  fn cross_stitches(&self) -> Vec<Stitch> {
    match self {
      TixelContent::V1(v) => v.mixins.iter().cloned().collect(),
    }
  }
}

impl TixelContent {
  pub fn strand_cid(&self) -> Cid {
    match self {
      TixelContent::V1(v) => v.chain,
    }
  }

  pub fn index(&self) -> u64 {
    match self {
      TixelContent::V1(v) => v.index as u64,
    }
  }

  pub fn version(&self) -> Version {
    match self {
      TixelContent::V1(_) => Version::parse("1.0.0").unwrap(),
    }
  }

  pub fn subspec(&self) -> Option<Subspec> {
    match self {
      TixelContent::V1(_) => None,
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
