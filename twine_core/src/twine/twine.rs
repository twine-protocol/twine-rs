use std::{ops::Deref, fmt::Display};
use libipld::Cid;
use semver::Version;
use std::sync::Arc;

use crate::{as_cid::AsCid, errors::VerificationError, specification::Subspec, twine::{Strand, Tixel}, verify::Verifiable};

#[derive(Debug, PartialEq, Clone)]
pub struct Twine {
  // so we have the option of not duplicating immutable data
  strand: Arc<Strand>,
  tixel: Arc<Tixel>,
}

impl Twine {
  pub fn try_new(strand: Strand, tixel: Tixel) -> Result<Self, VerificationError> {
    strand.verify()?;
    strand.verify_tixel(&tixel)?;
    let strand = Arc::new(strand);
    let tixel = Arc::new(tixel);
    Ok(Self { strand, tixel })
  }

  pub fn try_new_from_shared(strand: Arc<Strand>, tixel: Arc<Tixel>) -> Result<Self, VerificationError> {
    strand.verify()?;
    strand.verify_tixel(&tixel)?;
    Ok(Self { strand, tixel })
  }

  pub fn strand(&self) -> Arc<Strand> {
    self.strand.clone()
  }

  pub fn tixel(&self) -> Arc<Tixel> {
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
    (*self.tixel).eq(other)
  }
}

impl PartialEq<Twine> for Tixel {
  fn eq(&self, other: &Twine) -> bool {
    self.eq(&*other.tixel)
  }
}

impl Display for Twine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.tixel)
  }
}
