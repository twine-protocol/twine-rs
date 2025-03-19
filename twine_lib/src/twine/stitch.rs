use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

use super::{Tixel, Twine};
use crate::as_cid::AsCid;
use crate::errors::VerificationError;
use crate::Cid;
use crate::{errors::ResolutionError, resolver::Resolver};

/// A Stitch is a reference to a Tixel via its CID and Strand CID
///
/// These get included in Tixels to chain them together.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub struct Stitch {
  /// The strand CID
  pub strand: Cid,
  /// The tixel CID
  pub tixel: Cid,
}

impl Stitch {
  /// Refresh changes this stitch to the latest version of the tixel.
  pub async fn refresh(self, resolver: &impl Resolver) -> Result<Self, ResolutionError> {
    use futures::join;
    let (old, new) = join!(resolver.resolve(self), resolver.resolve_latest(self.strand));
    let (old, new) = (old?.unpack(), new?.unpack());
    if old.index() > new.index() {
      return Err(ResolutionError::BadData(
        "Latest tixel in resolver is behind recorded stitch".into(),
      ));
    }
    Ok(new.into())
  }
}

impl PartialEq<Twine> for Stitch {
  fn eq(&self, other: &Twine) -> bool {
    self.strand == other.strand_cid() && self.tixel == other.cid()
  }
}

impl PartialEq<Stitch> for Twine {
  fn eq(&self, other: &Stitch) -> bool {
    self.strand_cid() == other.strand && self.cid() == other.tixel
  }
}

impl PartialEq<Stitch> for Tixel {
  fn eq(&self, other: &Stitch) -> bool {
    self.strand_cid() == other.strand && self.cid() == other.tixel
  }
}

impl PartialEq<Tixel> for Stitch {
  fn eq(&self, other: &Tixel) -> bool {
    self.strand == other.strand_cid() && self.tixel == other.cid()
  }
}

impl From<Tixel> for Stitch {
  fn from(tixel: Tixel) -> Self {
    Stitch {
      strand: tixel.strand_cid(),
      tixel: tixel.cid(),
    }
  }
}

impl From<Twine> for Stitch {
  fn from(twine: Twine) -> Self {
    Stitch {
      strand: twine.strand_cid(),
      tixel: twine.cid(),
    }
  }
}

impl From<(Cid, Cid)> for Stitch {
  fn from((strand, tixel): (Cid, Cid)) -> Self {
    Stitch { strand, tixel }
  }
}

/// BackStitches are links within the same strand
///
/// A [`Tixel`] will have a list stitches to previous tixels in the same strand.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BackStitches(Vec<Stitch>);

impl BackStitches {
  /// Creates back stitches from a list of Tixel CIDs.
  pub fn new(strand: Cid, cids: Vec<Cid>) -> Self {
    Self(
      cids
        .into_iter()
        .map(|tixel| Stitch { strand, tixel })
        .collect(),
    )
  }

  /// Creates back stitches from condensed form.
  ///
  /// Condensed back stitches are a list of CIDs, where the last is mandatory.
  /// Any missing CIDs are implicitly the same as the later one.
  pub fn try_new_from_condensed(
    strand: Cid,
    cids: Vec<Option<Cid>>,
  ) -> Result<Self, VerificationError> {
    let rev_list = cids
      .into_iter()
      .rev()
      .scan(None, |prev, tixel| {
        let tixel = tixel.or(*prev);
        *prev = tixel;
        Some(
          tixel
            .ok_or(VerificationError::InvalidTwineFormat(
              "Invalid back-stitches condensed format".into(),
            ))
            .map(|tixel| Stitch { strand, tixel }),
        )
      })
      .collect::<Result<Vec<Stitch>, _>>()?;

    Ok(Self(rev_list.into_iter().rev().collect()))
  }

  /// Returns the number of stitches in the list
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Returns the CID of the strand these stitches are in
  pub fn strand_cid(&self) -> Cid {
    self.0.first().map(|s| s.strand).unwrap()
  }

  /// Converts it into the condensed form
  pub fn into_condensed(self) -> Vec<Option<Cid>> {
    let rev_list = self
      .0
      .into_iter()
      .rev()
      .scan(None, |prev, stitch| {
        let tixel = stitch.tixel;
        let curr = if prev.is_none() || &tixel != prev.as_ref().unwrap() {
          *prev = Some(tixel);
          Some(tixel)
        } else {
          None
        };
        Some(curr)
      })
      .collect::<Vec<_>>();

    rev_list.into_iter().rev().collect()
  }

  /// Get the first stitch in the list
  pub fn first(&self) -> Option<&Stitch> {
    self.0.first()
  }

  /// Get the stitch at the given index
  pub fn get(&self, index: usize) -> Option<&Stitch> {
    self.0.get(index)
  }

  /// Get all the stitches as a list
  pub fn stitches(&self) -> Vec<Stitch> {
    self.0.clone()
  }

  /// Unwrap the underlying list without cloning
  pub fn into_inner(self) -> Vec<Stitch> {
    self.0
  }

  /// Check if a tixel CID is included in the list
  pub fn includes<C: AsCid>(&self, cid: C) -> bool {
    self.0.iter().any(|s| &s.tixel == cid.as_cid())
  }
}

/// CrossStitches are links to other strands
///
/// A [`Tixel`] will have a list of stitches to tixels in other strands.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CrossStitches(HashMap<Cid, Stitch>);

impl CrossStitches {
  /// Create a new CrossStitches from a list of Stitch references
  pub fn new<S: AsRef<[Stitch]>>(stitches: S) -> Self {
    Self(stitches.as_ref().iter().map(|s| (s.strand, *s)).collect())
  }

  /// Get the stitch for a given strand CID
  pub fn get(&self, strand: &Cid) -> Option<&Stitch> {
    self.0.get(strand)
  }

  /// Returns the number of stitches in the list
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Get a set of all the strands that are stitched
  pub fn strands(&self) -> HashSet<Cid> {
    self.0.keys().cloned().collect()
  }

  /// Get all the stitches as a list
  pub fn stitches(&self) -> Vec<Stitch> {
    self.0.values().cloned().collect()
  }

  /// Unwrap the underlying map without cloning
  pub fn into_inner(self) -> HashMap<Cid, Stitch> {
    self.0
  }

  /// Check if a strand CID is included in the list
  pub fn strand_is_stitched<C: AsCid>(&self, strand: C) -> bool {
    self.0.contains_key(strand.as_cid())
  }

  /// Add a new stitch or refresh an existing one
  pub async fn add_or_refresh<R: Resolver, C: AsCid>(
    mut self,
    strand: C,
    resolver: &R,
  ) -> Result<Self, ResolutionError> {
    let latest = resolver.resolve_latest(strand.as_cid()).await?;
    let stitch: Stitch = latest.unpack().into();
    self.0.insert(stitch.strand, stitch);
    Ok(self)
  }

  /// Refreshes as many stitches as possible, returning the new stitches and any errors.
  pub async fn refresh_any<R: Resolver>(
    self,
    resolver: &R,
  ) -> (Self, Vec<(Stitch, ResolutionError)>) {
    let mut new_stitches = HashMap::new();
    let mut errors = Vec::new();
    for (strand, stitch) in self {
      match stitch.refresh(resolver).await {
        Ok(new) => {
          new_stitches.insert(strand, new);
        }
        Err(err) => {
          errors.push((stitch, err));
        }
      }
    }
    (Self(new_stitches), errors)
  }

  /// Refresh all stitches, returning the new stitches or an error
  pub async fn refresh_all<R: Resolver>(self, resolver: &R) -> Result<Self, ResolutionError> {
    let mut new_stitches = HashMap::new();
    for (strand, stitch) in self {
      let new = stitch.refresh(resolver).await?;
      new_stitches.insert(strand, new);
    }
    Ok(Self(new_stitches))
  }

  /// Check if a tixel CID is included in the list
  pub fn includes<C: AsCid>(&self, cid: C) -> bool {
    self.0.values().any(|s| &s.tixel == cid.as_cid())
  }
}

impl IntoIterator for CrossStitches {
  type Item = (Cid, Stitch);
  type IntoIter = std::collections::hash_map::IntoIter<Cid, Stitch>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl From<Vec<Stitch>> for CrossStitches {
  fn from(stitches: Vec<Stitch>) -> Self {
    Self::new(stitches)
  }
}

impl From<CrossStitches> for Vec<Stitch> {
  fn from(cross_stitches: CrossStitches) -> Self {
    cross_stitches.stitches()
  }
}

impl From<HashMap<Cid, Cid>> for CrossStitches {
  fn from(cross_stitches: HashMap<Cid, Cid>) -> Self {
    Self(
      cross_stitches
        .into_iter()
        .map(|(strand, tixel)| (strand, Stitch { strand, tixel }))
        .collect(),
    )
  }
}

impl From<CrossStitches> for HashMap<Cid, Cid> {
  fn from(cross_stitches: CrossStitches) -> Self {
    cross_stitches
      .0
      .into_iter()
      .map(|(strand, stitch)| (strand, stitch.tixel))
      .collect()
  }
}

impl From<HashMap<Cid, Stitch>> for CrossStitches {
  fn from(cross_stitches: HashMap<Cid, Stitch>) -> Self {
    Self(cross_stitches)
  }
}

impl From<CrossStitches> for HashMap<Cid, Stitch> {
  fn from(cross_stitches: CrossStitches) -> Self {
    cross_stitches.0
  }
}

impl From<Vec<(Cid, Cid)>> for CrossStitches {
  fn from(cross_stitches: Vec<(Cid, Cid)>) -> Self {
    Self(
      cross_stitches
        .into_iter()
        .map(|(strand, tixel)| (strand, Stitch { strand, tixel }))
        .collect(),
    )
  }
}

impl From<CrossStitches> for Vec<(Cid, Cid)> {
  fn from(cross_stitches: CrossStitches) -> Self {
    cross_stitches
      .0
      .into_iter()
      .map(|(strand, stitch)| (strand, stitch.tixel))
      .collect()
  }
}
