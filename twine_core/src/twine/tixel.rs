use crate::{schemas::v1, specification::Subspec};
use libipld::Ipld;
use serde::{Serialize, Deserialize};
use super::{container::TwineContainer, Payload, Strand};

pub type Tixel = TwineContainer<TixelContent>;

impl Tixel {
  pub fn payload(&self) -> Ipld {
    self.content().payload()
  }

  pub fn unpack_payload<P: Payload>(&self, strand: &Strand) -> Result<P, libipld::error::SerdeError> {
    self.content().unpack_payload(strand.subspec())
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum TixelContent {
  V1(v1::PulseContentV1),
}

impl TixelContent {
  pub fn payload(&self) -> Ipld {
    match self {
      TixelContent::V1(v) => v.payload.clone(),
    }
  }

  pub fn unpack_payload<P: Payload>(&self, subspec: Option<Subspec>) -> Result<P, libipld::error::SerdeError> {
    match self {
      TixelContent::V1(v) => P::from_ipld(subspec, v.payload.clone()),
    }
  }
}
