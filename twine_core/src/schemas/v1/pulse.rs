use libipld::{Ipld, Cid};
use serde::{Serialize, Deserialize};
use super::Mixin;

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct PulseContentV1 {
  chain: Cid,
  index: u32, // note: DAG-CBOR supports i64, but we don't
  source: String,
  links: Vec<Cid>,
  mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  payload: Ipld,
}
