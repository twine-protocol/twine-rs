use crate::Cid;
use semver::Version;
use std::{fmt::Display, ops::Deref};

use crate::{
  as_cid::AsCid,
  errors::VerificationError,
  specification::Subspec,
  twine::{Strand, Tixel},
};

/// Primary data structure for interacting with Twine records
///
/// A Twine struct is a Tixel/Strand pair. Any Twine created
/// will automatically have verified the Tixel and Strand integrity
/// and authenticity.
///
/// Since Strands and Tixels are effectively Arcs of their underlying
/// data, Twines are efficient to clone and pass around.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Twine {
  strand: Strand,
  tixel: Tixel,
}

impl Twine {
  /// Create a new Twine from a Strand and Tixel
  pub fn try_new(strand: Strand, tixel: Tixel) -> Result<Self, VerificationError> {
    strand.verify_tixel(&tixel)?;
    Ok(Self { strand, tixel })
  }

  /// Get a reference to the Strand
  pub fn strand(&self) -> &Strand {
    &self.strand
  }

  /// Get a reference to the Tixel
  pub fn tixel(&self) -> &Tixel {
    &self.tixel
  }

  /// Get the radix value of the Strand
  pub fn radix(&self) -> u8 {
    self.strand().radix()
  }

  /// Get the twine version of this record
  pub fn version(&self) -> Version {
    let strand_ver = self.strand().version();
    match strand_ver.major {
      1 => strand_ver,
      _ => self.tixel().version(),
    }
  }

  /// Get the subspec of this record if it exists
  pub fn subspec(&self) -> Option<Subspec> {
    let strand_ver = self.strand().version();
    match strand_ver.major {
      1 => self.strand().subspec(),
      _ => self.tixel().subspec(),
    }
  }
}

impl Deref for Twine {
  type Target = Tixel;

  fn deref(&self) -> &Self::Target {
    &self.tixel
  }
}

impl From<Twine> for Cid {
  fn from(twine: Twine) -> Self {
    twine.tixel().cid()
  }
}

impl AsRef<Cid> for Twine {
  fn as_ref(&self) -> &Cid {
    self.tixel.as_cid()
  }
}

impl AsCid for Twine {
  fn as_cid(&self) -> &Cid {
    self.tixel.as_cid()
  }
}

impl PartialEq<Tixel> for Twine {
  fn eq(&self, other: &Tixel) -> bool {
    self.tixel.eq(other)
  }
}

impl PartialEq<Twine> for Tixel {
  fn eq(&self, other: &Twine) -> bool {
    self.eq(&other.tixel)
  }
}

impl Display for Twine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.tixel)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::twine::TwineBlock;

  fn valid_twine() -> Twine {
    let strand = Strand::from_tagged_dag_json(crate::test::STRAND_V2_JSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(crate::test::TIXEL_V2_JSON).unwrap();
    Twine::try_new(strand, tixel).unwrap()
  }

  #[test]
  fn try_new_rejects_tixel_from_other_strand() {
    // v1 strand and v2 tixel do not belong together.
    let strand = Strand::from_tagged_dag_json(crate::test::STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(crate::test::TIXEL_V2_JSON).unwrap();
    let err = Twine::try_new(strand, tixel).unwrap_err();
    assert!(matches!(err, VerificationError::TixelNotOnStrand));
  }

  #[test]
  fn accessors_agree_with_components() {
    let twine = valid_twine();
    assert_eq!(twine.strand().cid(), twine.strand.cid());
    assert_eq!(twine.tixel().cid(), twine.tixel.cid());
    assert_eq!(twine.radix(), twine.strand().radix());
    // v2 strand => version comes from the tixel.
    assert_eq!(twine.version(), twine.tixel().version());
    assert_eq!(twine.version().major, 2);
    assert_eq!(twine.subspec(), twine.tixel().subspec());
  }

  #[test]
  fn version_for_v1_comes_from_strand() {
    // For a v1 strand, version() and subspec() defer to the strand.
    let strand = Strand::from_tagged_dag_json(crate::test::STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(crate::test::TIXELJSON).unwrap();
    let twine = Twine::try_new(strand, tixel).unwrap();
    assert_eq!(twine.version().major, 1);
    assert_eq!(twine.version(), twine.strand().version());
    assert_eq!(twine.subspec(), twine.strand().subspec());
  }

  #[test]
  fn deref_and_cid_conversions() {
    let twine = valid_twine();
    let tixel_cid = twine.tixel().cid();

    // Deref to Tixel
    assert_eq!(twine.index(), twine.tixel().index());

    // From<Twine> for Cid + AsRef + AsCid all point at the tixel.
    let cid: Cid = twine.clone().into();
    assert_eq!(cid, tixel_cid);
    assert_eq!(<Twine as AsRef<Cid>>::as_ref(&twine), &tixel_cid);
    assert_eq!(twine.as_cid(), &tixel_cid);
  }

  #[test]
  fn cross_type_equality_and_display() {
    let twine = valid_twine();
    let tixel = twine.tixel().clone();
    assert_eq!(twine, tixel);
    assert_eq!(tixel, twine);

    // Display mirrors the tixel's Display.
    assert_eq!(twine.to_string(), twine.tixel().to_string());
  }
}
