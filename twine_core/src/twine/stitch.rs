use std::collections::HashMap;

use libipld::Cid;

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

impl From<Twine> for Stitch {
  fn from(twine: Twine) -> Self {
    Stitch {
      strand: twine.strand_cid(),
      tixel: twine.cid(),
    }
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
