use libipld::Cid;

use super::Tixel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
