use crate::schemas::v1;
use libipld::Ipld;
use serde::{Serialize, Deserialize};
use super::container::TwineContainer;

pub type Tixel = TwineContainer<TixelContent>;

impl Tixel {
  pub fn payload(&self) -> Ipld {
    self.content().payload()
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
}
