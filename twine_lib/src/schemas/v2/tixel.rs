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

#[cfg(test)]
mod test {
  use super::*;
  use crate::errors::VerificationError;
  use ipld_core::cid::Cid;

  // Helper: build two distinct CIDs to use as strand / tixel references
  fn dummy_cid_1() -> Cid {
    use ipld_core::cid::Version;
    use multihash_codetable::{Code, MultihashDigest};
    let hash = Code::Sha2_256.digest(b"dummy-cid-1");
    Cid::new(Version::V1, 0x71u64, hash).unwrap()
  }

  fn dummy_cid_2() -> Cid {
    use ipld_core::cid::Version;
    use multihash_codetable::{Code, MultihashDigest};
    let hash = Code::Sha2_256.digest(b"dummy-cid-2");
    Cid::new(Version::V1, 0x71u64, hash).unwrap()
  }

  fn dummy_cid_3() -> Cid {
    use ipld_core::cid::Version;
    use multihash_codetable::{Code, MultihashDigest};
    let hash = Code::Sha2_256.digest(b"dummy-cid-3");
    Cid::new(Version::V1, 0x71u64, hash).unwrap()
  }

  // --- EncodedCrossStitches ordering -------------------------------------

  #[test]
  fn encoded_cross_stitches_empty_ok() {
    // Empty cross-stitches list is always valid
    let result: Result<EncodedCrossStitches, VerificationError> =
      EncodedCrossStitches::try_from(vec![]);
    assert!(result.is_ok(), "empty cross-stitches should be Ok");
  }

  #[test]
  fn encoded_cross_stitches_single_ok() {
    let result = EncodedCrossStitches::try_from(vec![(dummy_cid_1(), dummy_cid_2())]);
    assert!(result.is_ok(), "single cross-stitch should be Ok");
  }

  #[test]
  fn encoded_cross_stitches_ordered_ok() {
    // Two entries where first strand CID < second strand CID
    let a = dummy_cid_1();
    let b = dummy_cid_2();
    // Ensure ordering by using sorted cids
    let mut cids = vec![a, b];
    cids.sort();
    let input = vec![(cids[0], dummy_cid_3()), (cids[1], dummy_cid_3())];
    let result = EncodedCrossStitches::try_from(input);
    assert!(result.is_ok(), "strictly ordered cross-stitches should be Ok");
  }

  #[test]
  fn encoded_cross_stitches_duplicate_strand_rejected() {
    // Two entries with the same strand CID (w[0].0 >= w[1].0 because equal)
    let same = dummy_cid_1();
    let input = vec![(same, dummy_cid_2()), (same, dummy_cid_3())];
    let result = EncodedCrossStitches::try_from(input);
    assert!(
      result.is_err(),
      "duplicate strand CIDs in cross-stitches should be rejected"
    );
    assert!(matches!(
      result,
      Err(VerificationError::InvalidTwineFormat(_))
    ));
  }

  #[test]
  fn encoded_cross_stitches_reverse_order_rejected() {
    // Two entries in reverse order (w[0].0 > w[1].0)
    let a = dummy_cid_1();
    let b = dummy_cid_2();
    let mut cids = vec![a, b];
    cids.sort();
    // Reverse so that first > second
    let input = vec![(cids[1], dummy_cid_3()), (cids[0], dummy_cid_3())];
    let result = EncodedCrossStitches::try_from(input);
    assert!(
      result.is_err(),
      "reverse-ordered cross-stitches must be rejected"
    );
    assert!(matches!(
      result,
      Err(VerificationError::InvalidTwineFormat(_))
    ));
  }

  // --- TixelFields::verify -----------------------------------------------

  #[test]
  fn tixel_fields_genesis_tixel_ok() {
    let fields = TixelFields {
      strand: dummy_cid_1(),
      index: 0,
      cross_stitches: EncodedCrossStitches::try_from(vec![]).unwrap(),
      back_stitches: vec![],
      drop: 0,
      payload: Ipld::Null,
    };
    assert!(fields.verify().is_ok(), "genesis tixel (index 0) must pass");
  }

  #[test]
  fn tixel_fields_non_genesis_with_back_stitch_ok() {
    let strand = dummy_cid_1();
    let prev = dummy_cid_2();
    let fields = TixelFields {
      strand,
      index: 1,
      cross_stitches: EncodedCrossStitches::try_from(vec![]).unwrap(),
      back_stitches: vec![Some(prev)],
      drop: 0,
      payload: Ipld::Null,
    };
    assert!(fields.verify().is_ok());
  }

  #[test]
  fn tixel_fields_non_genesis_no_back_stitch_rejected() {
    let fields = TixelFields {
      strand: dummy_cid_1(),
      index: 1,
      cross_stitches: EncodedCrossStitches::try_from(vec![]).unwrap(),
      back_stitches: vec![], // required but missing
      drop: 0,
      payload: Ipld::Null,
    };
    let result = fields.verify();
    assert!(result.is_err(), "non-genesis tixel with no back-stitch must fail");
    assert!(matches!(result, Err(VerificationError::InvalidTwineFormat(_))));
  }

  #[test]
  fn tixel_fields_cross_stitch_on_own_strand_rejected() {
    let strand = dummy_cid_1();
    let other = dummy_cid_2();
    // Cross-stitch pointing to own strand must be rejected
    let fields = TixelFields {
      strand,
      index: 0,
      cross_stitches: EncodedCrossStitches::try_from(vec![(strand, other)]).unwrap(),
      back_stitches: vec![],
      drop: 0,
      payload: Ipld::Null,
    };
    let result = fields.verify();
    assert!(result.is_err(), "cross-stitch on own strand must be rejected");
    assert!(matches!(result, Err(VerificationError::InvalidTwineFormat(_))));
  }

  #[test]
  fn tixel_fields_cross_stitch_on_different_strand_ok() {
    let strand = dummy_cid_1();
    let other_strand = dummy_cid_2();
    let tixel_ref = dummy_cid_3();
    let fields = TixelFields {
      strand,
      index: 0,
      cross_stitches: EncodedCrossStitches::try_from(vec![(other_strand, tixel_ref)]).unwrap(),
      back_stitches: vec![],
      drop: 0,
      payload: Ipld::Null,
    };
    assert!(fields.verify().is_ok(), "cross-stitch on different strand must be Ok");
  }

  #[test]
  fn tixel_fields_invalid_condensed_back_stitches_rejected() {
    // A back_stitches list where a non-last entry is None AND the last is also
    // None (invalid condensed form).
    let strand = dummy_cid_1();
    let fields = TixelFields {
      strand,
      index: 1,
      cross_stitches: EncodedCrossStitches::try_from(vec![]).unwrap(),
      back_stitches: vec![None], // None at last position is invalid
      drop: 0,
      payload: Ipld::Null,
    };
    let result = fields.verify();
    assert!(result.is_err(), "invalid condensed back-stitches must fail");
  }
}
