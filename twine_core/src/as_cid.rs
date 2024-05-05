use libipld::Cid;
use std::sync::Arc;

pub trait AsCid {
  fn as_cid(&self) -> &Cid;
}

impl AsCid for Cid {
  fn as_cid(&self) -> &Cid {
    self
  }
}

impl<T> AsCid for &T where T: AsCid {
  fn as_cid(&self) -> &Cid {
    (*self).as_cid()
  }
}

impl<T> AsCid for Arc<T> where T: AsCid {
  fn as_cid(&self) -> &Cid {
    self.as_ref().as_cid()
  }
}

impl<T> AsCid for Box<T> where T: AsCid {
  fn as_cid(&self) -> &Cid {
    self.as_ref().as_cid()
  }
}
