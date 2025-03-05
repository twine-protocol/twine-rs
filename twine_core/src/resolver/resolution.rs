use std::ops::Deref;
use crate::errors::VerificationError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwineResolution {
  query: SingleQuery,
  twine: Twine,
}

impl TwineResolution {
  pub fn try_new(query: SingleQuery, twine: Twine) -> Result<Self, ResolutionError> {
    if !query.matches(&twine) {
      return Err(ResolutionError::QueryMismatch(query));
    }
    Ok(Self { query, twine })
  }

  pub fn query(&self) -> &SingleQuery {
    &self.query
  }

  pub fn twine(&self) -> &Twine {
    &self.twine
  }

  pub fn unpack(self) -> Twine {
    self.twine
  }
}

impl Deref for TwineResolution {
  type Target = Twine;

  fn deref(&self) -> &Self::Target {
    &self.twine
  }
}

impl From<TwineResolution> for Twine {
  fn from(resolution: TwineResolution) -> Self {
    resolution.twine
  }
}

impl PartialEq<Twine> for TwineResolution {
  fn eq(&self, other: &Twine) -> bool {
    self.twine == *other
  }
}

impl PartialEq<TwineResolution> for Twine {
  fn eq(&self, other: &TwineResolution) -> bool {
    *self == other.twine
  }
}


impl PartialEq<Tixel> for TwineResolution {
  fn eq(&self, other: &Tixel) -> bool {
    self.twine == *other
  }
}

impl PartialEq<TwineResolution> for Tixel {
  fn eq(&self, other: &TwineResolution) -> bool {
    *self == other.twine
  }
}

impl AsCid for TwineResolution {
  fn as_cid(&self) -> &Cid {
    self.twine.as_cid()
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrandResolution {
  cid: Cid,
  strand: Strand,
}

impl StrandResolution {
  pub fn try_new(cid: Cid, strand: Strand) -> Result<Self, ResolutionError> {
    if cid != strand.cid() {
      return Err(VerificationError::CidMismatch{
        expected: cid.to_string(),
        actual: strand.cid().to_string()
      }.into());
    }
    Ok(Self { cid, strand })
  }

  pub fn requested_cid(&self) -> &Cid {
    &self.cid
  }

  pub fn strand(&self) -> &Strand {
    &self.strand
  }

  pub fn unpack(self) -> Strand {
    self.strand
  }
}

impl Deref for StrandResolution {
  type Target = Strand;

  fn deref(&self) -> &Self::Target {
    &self.strand
  }
}

impl From<StrandResolution> for Strand {
  fn from(resolution: StrandResolution) -> Self {
    resolution.strand
  }
}

impl PartialEq<Strand> for StrandResolution {
  fn eq(&self, other: &Strand) -> bool {
    self.strand == *other
  }
}

impl PartialEq<StrandResolution> for Strand {
  fn eq(&self, other: &StrandResolution) -> bool {
    *self == other.strand
  }
}

impl AsCid for StrandResolution {
  fn as_cid(&self) -> &Cid {
    self.strand.as_cid()
  }
}
