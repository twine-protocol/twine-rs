use crate::errors::VerificationError;
use std::ops::Deref;

use super::*;

/// A return type for Resolver methods that return a Twine
///
/// This ensures that the query matches the returned Twine.
/// If the query is requests an index or CID, this checks
/// that the returned Twine's index or CID matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwineResolution {
  query: SingleQuery,
  twine: Twine,
}

impl TwineResolution {
  /// Create a new TwineResolution
  pub fn try_new(query: SingleQuery, twine: Twine) -> Result<Self, ResolutionError> {
    if !query.matches(&twine) {
      return Err(ResolutionError::QueryMismatch(query));
    }
    Ok(Self { query, twine })
  }

  /// Access the query that was resolved
  pub fn query(&self) -> &SingleQuery {
    &self.query
  }

  /// Access the Twine that was resolved
  pub fn twine(&self) -> &Twine {
    &self.twine
  }

  /// Unpack the Twine from the resolution
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

/// A return type for Resolver methods that return a Strand
///
/// This ensures that the query matches the returned Strand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrandResolution {
  cid: Cid,
  strand: Strand,
}

impl StrandResolution {
  /// Create a new StrandResolution
  pub fn try_new(cid: Cid, strand: Strand) -> Result<Self, ResolutionError> {
    if cid != strand.cid() {
      return Err(
        VerificationError::CidMismatch {
          expected: cid.to_string(),
          actual: strand.cid().to_string(),
        }
        .into(),
      );
    }
    Ok(Self { cid, strand })
  }

  /// Access the CID that was requested
  pub fn requested_cid(&self) -> &Cid {
    &self.cid
  }

  /// Access the Strand that was resolved
  pub fn strand(&self) -> &Strand {
    &self.strand
  }

  /// Unpack the Strand from the resolution
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
