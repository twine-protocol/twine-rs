use crate::schemas::v1;
use serde::{Serialize, Deserialize};
use super::twine::TwineContainer;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TixelContent {
  V1(v1::PulseContentV1),
}

pub type Tixel = TwineContainer<TixelContent>;

