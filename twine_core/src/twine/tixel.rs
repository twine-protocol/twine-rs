use crate::schemas::v1;
use serde::{Serialize, Deserialize};
use super::container::TwineContainer;

pub type Tixel = TwineContainer<TixelContent>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum TixelContent {
  V1(v1::PulseContentV1),
}
