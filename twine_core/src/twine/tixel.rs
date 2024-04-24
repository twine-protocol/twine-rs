use crate::schemas::v1;
use serde::{Serialize, Deserialize};
use super::container::TwineContainer;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum TixelContent {
  V1(v1::PulseContentV1),
}

pub type Tixel = TwineContainer<TixelContent>;

