use libipld::Ipld;
use serde::{Serialize, Deserialize};
use josekit::jwk::Jwk;
use super::{V1, Mixin};

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct ChainContentV1 {
  pub specification: V1,
  pub key: Jwk,
  pub meta: Ipld,
  pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  pub source: String,
  pub links_radix: u32,
}
