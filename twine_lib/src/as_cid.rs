//! Trait for anything that can be represented as a CID
use crate::Cid;
use std::sync::Arc;

/// Trait for anything that can be represented as a CID
pub trait AsCid {
  /// Get the CID as a reference
  fn as_cid(&self) -> &Cid;
}

impl AsCid for Cid {
  fn as_cid(&self) -> &Cid {
    self
  }
}

impl<T> AsCid for &T
where
  T: AsCid,
{
  fn as_cid(&self) -> &Cid {
    (*self).as_cid()
  }
}

impl<T> AsCid for Arc<T>
where
  T: AsCid,
{
  fn as_cid(&self) -> &Cid {
    self.as_ref().as_cid()
  }
}

impl<T> AsCid for Box<T>
where
  T: AsCid,
{
  fn as_cid(&self) -> &Cid {
    self.as_ref().as_cid()
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn sample_cid() -> Cid {
    "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu"
      .parse()
      .unwrap()
  }

  #[test]
  fn as_cid_for_all_wrappers_points_at_same_cid() {
    let cid = sample_cid();

    // Plain Cid
    assert_eq!(cid.as_cid(), &cid);
    // Reference forwarding
    let r = &cid;
    assert_eq!(r.as_cid(), &cid);
    // Arc
    let arc = Arc::new(cid);
    assert_eq!(arc.as_cid(), &*arc);
    // Box
    let boxed = Box::new(sample_cid());
    assert_eq!(boxed.as_cid(), &sample_cid());
  }
}
