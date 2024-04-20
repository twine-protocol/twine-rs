use libipld::Ipld;
use serde::{Serialize, Deserialize};
use josekit::jwk::Jwk;
use super::{V1, Mixin};

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct ChainContentV1 {
  specification: V1,
  key: Jwk,
  meta: Ipld,
  mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  source: String,
  links_radix: u32,
}
