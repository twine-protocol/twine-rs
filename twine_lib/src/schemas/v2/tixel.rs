use crate::{errors::VerificationError, twine::BackStitches};

use super::*;

/// Structure handling serialization of cross-stitches
///
/// It's vital that these are ordered to ensure that
/// the entries can't be used as a kind of nonce
/// so this structure manages that.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(try_from = "Vec<(Cid, Cid)>", into = "Vec<(Cid, Cid)>")]
pub struct EncodedCrossStitches(CrossStitches);

impl TryFrom<Vec<(Cid, Cid)>> for EncodedCrossStitches {
  type Error = VerificationError;

  fn try_from(v: Vec<(Cid, Cid)>) -> Result<Self, Self::Error> {
    if v.windows(2).any(|w| w[0].0 >= w[1].0) {
      return Err(VerificationError::InvalidTwineFormat(
        "Cross-stitches are not ordered correctly".into(),
      ));
    }

    Ok(Self(v.into()))
  }
}

impl From<EncodedCrossStitches> for Vec<(Cid, Cid)> {
  fn from(v: EncodedCrossStitches) -> Self {
    let mut vec: Vec<_> = v.0.into();
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

/// Tixel fields in the content field
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TixelFields {
  /// strand cid
  #[serde(rename = "s")]
  pub strand: Cid,
  /// index
  #[serde(rename = "i")]
  pub index: u64,
  /// cross stitches
  #[serde(rename = "x")]
  pub cross_stitches: EncodedCrossStitches,
  /// back stitches
  #[serde(rename = "b")]
  pub back_stitches: Vec<Option<Cid>>,
  /// drop index
  #[serde(rename = "d")]
  pub drop: u64,
  /// payload
  #[serde(rename = "p")]
  pub payload: Ipld,
}

/// Content field of tixels
pub type TixelContentV2 = ContentV2<TixelFields>;

impl Verifiable for TixelFields {
  type Error = VerificationError;
  /// Self verification
  ///
  /// Verifications performed:
  /// - Check that non-starting tixels have at least one back-stitch
  /// - Ensures back-stitches are of a valid form
  /// - Ensures cross-stitches don't contain the current strand
  fn verify(&self) -> Result<(), VerificationError> {
    // must have at least one back-stitch if not the starting tixel
    if self.back_stitches.len() == 0 && self.index != 0 {
      return Err(VerificationError::InvalidTwineFormat(
        "Non-starting tixel has zero links".into(),
      ));
    }

    // ensure back-stitches are valid condensed form
    BackStitches::try_new_from_condensed(self.strand, self.back_stitches.clone())?;

    // cross-stitches can't contain own strand
    if self.cross_stitches.get(&self.strand).is_some() {
      return Err(VerificationError::InvalidTwineFormat(
        "Contains cross-stitch on own strand".into(),
      ));
    }

    Ok(())
  }
}
