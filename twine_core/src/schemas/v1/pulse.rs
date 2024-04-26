use libipld::{Ipld, Cid};
use serde::{Serialize, Deserialize};
use crate::errors::VerificationError;
use super::Mixin;
use crate::verify::is_all_unique;

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct PulseContentV1 {
  pub chain: Cid,
  pub index: u32, // note: DAG-CBOR supports i64, but we don't
  pub source: String,
  pub links: Vec<Cid>,
  pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  pub payload: Ipld,
}

impl PulseContentV1 {
  pub fn verify(&self) -> Result<(), VerificationError> {
    if !is_all_unique(&self.mixins) {
      return Err(VerificationError::InvalidTwineFormat("Contains mixins with duplicate chains".into()));
    }

    if self.links.len() == 0 && self.index != 0 {
      return Err(VerificationError::InvalidTwineFormat("Non-starting pulse has zero links".into()));
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_pulse_content_v1_verify() {
    let pulse = PulseContentV1 {
      chain: Cid::default(),
      index: 0,
      source: "test".into(),
      links: vec![],
      mixins: vec![],
      payload: Ipld::Null,
    };

    assert!(pulse.verify().is_ok());
  }

  #[test]
  fn test_pulse_content_v1_verify_duplicate_mixins() {
    let pulse = PulseContentV1 {
      chain: Cid::default(),
      index: 0,
      source: "test".into(),
      links: vec![],
      mixins: vec![
        Mixin {
          chain: Cid::default(),
          value: Cid::default(),
        },
        Mixin {
          chain: Cid::default(),
          value: Cid::default(),
        },
      ],
      payload: Ipld::Null,
    };

    assert!(pulse.verify().is_err());
  }

  #[test]
  fn test_pulse_content_v1_verify_non_starting_pulse_no_links() {
    let pulse = PulseContentV1 {
      chain: Cid::default(),
      index: 1,
      source: "test".into(),
      links: vec![],
      mixins: vec![],
      payload: Ipld::Null,
    };

    assert!(pulse.verify().is_err());
  }
}
