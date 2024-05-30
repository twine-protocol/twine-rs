use crate::specification::Subspec;
use crate::verify::Verifiable;
use crate::{errors::VerificationError, schemas::v1};
use crate::Cid;
use crate::Ipld;
use ipld_core::serde::{from_ipld, SerdeError};
use semver::Version;
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use ipld_core::codec::Codec;
use serde_ipld_dagcbor::codec::DagCborCodec;
use super::container::TwineContent;
use super::{CrossStitches, Stitch};
use super::{container::TwineContainer, Strand};

pub type Tixel = TwineContainer<TixelContent>;

impl PartialOrd for Tixel {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    if self.strand_cid() != other.strand_cid() {
      return None;
    }
    Some(self.index().cmp(&other.index()))
  }
}

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

  pub fn payload(&self) -> &Ipld {
    self.content().payload()
  }

  pub fn extract_payload<T: DeserializeOwned>(&self) -> Result<T, SerdeError> {
    let payload = self.payload();
    from_ipld(payload.clone())
  }

  pub fn source(&self) -> &str {
    self.content().source()
  }

  pub fn verify_with(&self, strand: &Strand) -> Result<(), VerificationError> {
    strand.verify_tixel(self)
  }

  pub fn previous(&self) -> Option<Stitch> {
    self.back_stitches().first().cloned()
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

  fn cross_stitches(&self) -> CrossStitches {
    match self {
      TixelContent::V1(v) => CrossStitches::new(
        v.mixins.iter().cloned().collect::<Vec<Stitch>>()
      ),
    }
  }

  fn bytes(&self) -> Vec<u8> {
    DagCborCodec::encode_to_vec(self).unwrap()
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

  pub fn payload(&self) -> &Ipld {
    match self {
      TixelContent::V1(v) => &v.payload,
    }
  }

  pub fn source(&self) -> &str {
    match self {
      TixelContent::V1(v) => v.source.as_str(),
    }
  }
}

impl From<v1::PulseContentV1> for TixelContent {
  fn from(content: v1::PulseContentV1) -> Self {
    TixelContent::V1(content)
  }
}
