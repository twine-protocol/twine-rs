use std::hash::Hash;

use crate::Cid;
use serde::{Deserialize, Serialize};

use crate::twine::Stitch;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Mixin {
  pub chain: Cid,
  pub value: Cid
}

impl Hash for Mixin {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.chain.hash().hash(state);
  }
}

impl From<Stitch> for Mixin {
  fn from(stitch: Stitch) -> Self {
    Mixin {
      chain: stitch.strand,
      value: stitch.tixel
    }
  }
}

impl From<Mixin> for Stitch {
  fn from(mixin: Mixin) -> Self {
    Stitch {
      strand: mixin.chain,
      tixel: mixin.value
    }
  }
}

impl FromIterator<Mixin> for Vec<Stitch> {
  fn from_iter<I: IntoIterator<Item=Mixin>>(iter: I) -> Self {
    iter.into_iter().map(Stitch::from).collect()
  }
}

impl FromIterator<Stitch> for Vec<Mixin> {
  fn from_iter<I: IntoIterator<Item=Stitch>>(iter: I) -> Self {
    iter.into_iter().map(Mixin::from).collect()
  }
}
