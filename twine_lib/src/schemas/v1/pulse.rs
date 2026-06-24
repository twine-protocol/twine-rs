use super::Mixin;
use crate::verify::is_all_unique;
use crate::{errors::VerificationError, verify::Verifiable};
use crate::{Cid, Ipld};
use serde::{Deserialize, Serialize};

/// The content field of a Pulse
#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PulseContentV1<P = Ipld> {
  /// The chain CID
  pub chain: Cid,
  /// The index of the pulse
  pub index: u32, // note: DAG-CBOR supports i64, but we don't
  /// The source of the pulse
  pub source: String,
  /// The back stitches
  pub links: Vec<Cid>,
  /// The cross stitches
  pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  /// The payload
  pub payload: P,
}

impl Verifiable for PulseContentV1 {
  type Error = VerificationError;
  /// Self-verification of the PulseContentV1
  ///
  /// The implemented verification checks are:
  /// - Check that all mixins are unique
  /// - Check that there are no mixins on the same chain as the pulse
  /// - Check that non-starting pulses have at least one link
  fn verify(&self) -> Result<(), VerificationError> {
    if !is_all_unique(&self.mixins) {
      return Err(VerificationError::InvalidTwineFormat(
        "Contains mixins with duplicate chains".into(),
      ));
    }

    // can't have a mixin on own chain
    if self.mixins.iter().any(|mixin| mixin.chain == self.chain) {
      return Err(VerificationError::InvalidTwineFormat(
        "Contains mixin on own chain".into(),
      ));
    }

    if self.links.len() == 0 && self.index != 0 {
      return Err(VerificationError::InvalidTwineFormat(
        "Non-starting pulse has zero links".into(),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use ipld_core::ipld;

  #[test]
  fn test_pulse_content_v1_verify() {
    let pulse = PulseContentV1 {
      chain: Cid::default(),
      index: 0,
      source: "test".into(),
      links: vec![],
      mixins: vec![],
      payload: ipld!({
        "test": null
      }),
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

  #[test]
  fn test_pulse_content_v1_verify_mixin_on_own_chain_rejected() {
    // A single mixin whose chain CID == self.chain triggers lines 42-44.
    // We use a non-default CID to avoid the duplicate check which fires first.
    use ipld_core::cid::Version;
    use multihash_codetable::{Code, MultihashDigest};
    let hash = Code::Sha2_256.digest(b"own-chain-cid");
    let own_chain = Cid::new(Version::V1, 0x71u64, hash).unwrap();

    let pulse = PulseContentV1 {
      chain: own_chain,
      index: 0,
      source: "test".into(),
      links: vec![],
      mixins: vec![Mixin {
        chain: own_chain, // same as self.chain → must fail
        value: Cid::default(),
      }],
      payload: Ipld::Null,
    };

    let result = pulse.verify();
    assert!(
      result.is_err(),
      "mixin pointing to own chain must be rejected"
    );
    assert!(
      matches!(result, Err(crate::errors::VerificationError::InvalidTwineFormat(_))),
      "expected InvalidTwineFormat, got {:?}",
      result
    );
  }
}
