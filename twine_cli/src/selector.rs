use std::convert::TryFrom;
use anyhow::Result;
use twine_core::Cid;
use twine_core::resolver::{Query, RangeQuery};

#[derive(Debug, Clone, Copy)]
pub enum Selector {
  Strand(Cid),
  Query(Query),
  RangeQuery(RangeQuery),
}

// expects format <cid>[:<index>?[:<lower_index>?]]
// ... could be <cid>:: (whole range),
// <cid>::<lower_index> (range from latest to lower_index)
// <cid>:<upper_index>: (range from upper_index to 0)
pub fn parse_selector(selector: &str) -> Result<Selector> {
  match selector.split(':').count() {
    1 => {
      let cid = Cid::try_from(selector)?;
      Ok(Selector::Strand(cid))
    },
    2 => {
      Ok(Selector::Query(selector.parse()?))
    },
    3 => {
      Ok(Selector::RangeQuery(selector.parse()?))
    },
    _ => Err(anyhow::anyhow!("Invalid selector format")),
  }
}
