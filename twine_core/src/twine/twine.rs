use std::ops::Deref;
use crate::{prelude::VerificationError, twine::{Strand, Tixel}, verify::Verifiable};

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
}

impl Deref for Twine {
  type Target = Tixel;

  fn deref(&self) -> &Self::Target {
    self.tixel()
  }
}
