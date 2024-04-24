use crate::schemas::v1;
use serde::{Serialize, Deserialize};
use super::container::TwineContainer;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum StrandContent {
  V1(v1::ChainContentV1),
}

pub type Strand = TwineContainer<StrandContent>;
