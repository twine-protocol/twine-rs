use std::{fmt::Display, ops::Deref, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use twine_lib::{as_cid::AsCid, Cid};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct CidStr(#[serde_as(as = "serde_with::DisplayFromStr")] Cid);

impl Display for CidStr {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl FromStr for CidStr {
  type Err = twine_lib::cid::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(Self(Cid::from_str(s)?))
  }
}

impl Deref for CidStr {
  type Target = Cid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl From<Cid> for CidStr {
  fn from(cid: Cid) -> Self {
    Self(cid)
  }
}

impl From<CidStr> for Cid {
  fn from(cid_str: CidStr) -> Self {
    cid_str.0
  }
}

impl PartialEq<Cid> for CidStr {
  fn eq(&self, other: &Cid) -> bool {
    &self.0 == other
  }
}

impl PartialEq<CidStr> for Cid {
  fn eq(&self, other: &CidStr) -> bool {
    self == &other.0
  }
}

impl AsCid for CidStr {
  fn as_cid(&self) -> &Cid {
    &self.0
  }
}
