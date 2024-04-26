use std::ops::Deref;
use semver::Version;

use crate::{prelude::VerificationError, specification::Subspec, twine::{Strand, Tixel}, verify::Verifiable};

#[derive(Debug, PartialEq, Clone)]
pub struct Twine {
  strand: Strand,
  tixel: Tixel,
}

impl Twine {
  pub fn try_new(strand: Strand, tixel: Tixel) -> Result<Self, VerificationError> {
    strand.verify()?;
    strand.verify_tixel(&tixel)?;
    Ok(Self { strand, tixel })
  }

  pub fn strand(&self) -> &Strand {
    &self.strand
  }

  pub fn tixel(&self) -> &Tixel {
    &self.tixel
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
    self.tixel()
  }
}
