use std::{collections::HashMap, sync::Arc};
use libipld::Cid;
use crate::{errors::ResolutionError, resolver::{BaseResolver, Resolver}};
use super::{Tixel, Twine};

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub struct Stitch {
  pub strand: Cid,
  pub tixel: Cid,
}

impl From<Tixel> for Stitch {
  fn from(tixel: Tixel) -> Self {
    Stitch {
      strand: tixel.strand_cid(),
      tixel: tixel.cid(),
    }
  }
}

impl From<Arc<Tixel>> for Stitch {
  fn from(tixel: Arc<Tixel>) -> Self {
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CrossStitches(HashMap<Cid, Stitch>);

impl CrossStitches {
  pub fn new<S: AsRef<[Stitch]>>(stitches: S) -> Self {
    Self(stitches.as_ref().iter().map(|s| (s.strand, *s)).collect())
  }

  pub fn get(&self, strand: &Cid) -> Option<&Stitch> {
    self.0.get(strand)
  }

  pub fn stitches(&self) -> Vec<Stitch> {
    self.0.values().cloned().collect()
  }

  pub fn into_inner(self) -> HashMap<Cid, Stitch> {
    self.0
  }

  pub async fn refresh<R: Resolver>(self, resolver: &R) -> Result<Self, ResolutionError> {
    let mut new_stitches = HashMap::new();
    for (strand, stitch) in self {
      use futures::join;
      let (old, new) = match join!(resolver.resolve(stitch), resolver.resolve(strand)) {
        (Ok(old), Ok(new)) => (old, new),
        (Err(e), _) | (_, Err(e)) => return Err(e),
      };
      if old.index() > new.index() {
        return Err(ResolutionError::BadData("Latest tixel in resolver is behind recorded stitch".into()));
      }
      new_stitches.insert(strand, new.into());
    }
    Ok(Self(new_stitches))
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

