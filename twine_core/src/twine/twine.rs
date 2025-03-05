use std::{ops::Deref, fmt::Display};
use crate::Cid;
use semver::Version;

use crate::{as_cid::AsCid, errors::VerificationError, specification::Subspec, twine::{Strand, Tixel}};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Twine {
  strand: Strand,
  tixel: Tixel,
}

impl Twine {
  pub fn try_new(strand: Strand, tixel: Tixel) -> Result<Self, VerificationError> {
    strand.verify_tixel(&tixel)?;
    Ok(Self { strand, tixel })
  }

  pub fn strand(&self) -> Strand {
    self.strand.clone()
  }

  pub fn tixel(&self) -> Tixel {
    self.tixel.clone()
  }

  pub fn radix(&self) -> u8 {
    self.strand().radix()
  }

  pub fn version(&self) -> Version {
    let strand_ver = self.strand().version();
    match strand_ver.major {
      1 => strand_ver,
      _ => self.tixel().version(),
    }
  }

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
