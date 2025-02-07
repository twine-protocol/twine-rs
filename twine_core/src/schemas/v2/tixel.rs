use crate::{errors::VerificationError, twine::BackStitches};

use super::*;

/// It's vital that these are ordered to ensure that
/// the entries can't be used as a kind of nonce.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(try_from = "Vec<(Cid, Cid)>", into = "Vec<(Cid, Cid)>")]
pub struct EncodedCrossStitches(CrossStitches);

impl TryFrom<Vec<(Cid, Cid)>> for EncodedCrossStitches {
  type Error = VerificationError;

  fn try_from(v: Vec<(Cid, Cid)>) -> Result<Self, Self::Error> {
    for i in 0..v.len() - 1 {
      if v[i].0 >= v[i + 1].0 {
        return Err(VerificationError::InvalidTwineFormat("Cross-stitches are not ordered correctly".into()));
      }
    }

    Ok(Self(v.into()))
  }
}

impl From<EncodedCrossStitches> for Vec<(Cid, Cid)> {
  fn from(v: EncodedCrossStitches) -> Self {
    let mut vec : Vec<_> = v.0.into();
    vec.sort_by(|a: &(Cid, Cid), b: &(Cid, Cid)| a.0.cmp(&b.0));
    vec
  }
}

impl From<CrossStitches> for EncodedCrossStitches {
  fn from(v: CrossStitches) -> Self {
    Self(v)
  }
}

impl Deref for EncodedCrossStitches {
  type Target = CrossStitches;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TixelFields {
  #[serde(rename = "s")]
  pub strand: Cid,
  #[serde(rename = "i")]
  pub index: u64,
  #[serde(rename = "x")]
  pub cross_stitches: EncodedCrossStitches,
  #[serde(rename = "b")]
  pub back_stitches: Vec<Option<Cid>>,
  #[serde(rename = "d")]
  pub drop: u64,
  #[serde(rename = "p")]
  pub payload: Ipld,
}

pub type TixelContentV2 = ContentV2<TixelFields>;

impl Verifiable for TixelFields {
  fn verify(&self) -> Result<(), VerificationError> {
    // must have at least one back-stitch if not the starting tixel
    if self.back_stitches.len() == 0 && self.index != 0 {
      return Err(VerificationError::InvalidTwineFormat("Non-starting tixel has zero links".into()));
    }

    // ensure back-stitches are valid condensed form
    BackStitches::try_new_from_condensed(self.strand, self.back_stitches.clone())?;

    // cross-stitches can't contain own strand
    if self.cross_stitches.get(&self.strand).is_some() {
      return Err(VerificationError::InvalidTwineFormat("Contains cross-stitch on own strand".into()));
    }

    Ok(())
  }
}
