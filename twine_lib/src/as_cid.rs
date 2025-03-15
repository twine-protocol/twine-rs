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
