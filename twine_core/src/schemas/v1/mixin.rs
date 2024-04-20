use libipld::Cid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct Mixin {
  pub chain: Cid,
  pub value: Cid
}
