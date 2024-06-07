use crate::{errors::VerificationError, verify::is_all_unique};

use super::*;
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TixelFields {
  #[serde(rename = "s")]
  strand: Cid,
  #[serde(rename = "i")]
  index: u32,
  #[serde(rename = "x")]
  #[serde_as(as = "Vec<(_, _)>")]
  cross_stitches: HashMap<Cid, Cid>,
  #[serde(rename = "b")]
  back_stitches: Vec<Option<Cid>>,
  #[serde(rename = "d")]
  drop: u32,
  #[serde(rename = "p")]
  payload: Ipld,
}

pub type TixelContentV2 = ContentV2<TixelFields>;

impl Verifiable for TixelFields {
  fn verify(&self) -> Result<(), VerificationError> {
    // must have at least one back-stitch if not the starting tixel
    if self.back_stitches.len() == 0 && self.index != 0 {
      return Err(VerificationError::InvalidTwineFormat("Non-starting tixel has zero links".into()));
    }

    // cross-stitches can't contain own strand
    if self.cross_stitches.contains_key(&self.strand) {
      return Err(VerificationError::InvalidTwineFormat("Contains cross-stitch on own strand".into()));
    }

    Ok(())
  }
}
