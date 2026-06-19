use crate::errors::VerificationError;
use chrono::{DateTime, Utc};

use super::*;

/// Content fields for Strands
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct StrandFields {
  /// public key
  #[serde(rename = "k")]
  pub key: PublicKey,
  /// radix
  #[serde(rename = "r")]
  pub radix: u8,
  /// details
  #[serde(rename = "d")]
  pub details: Ipld,
  /// genesis datetime
  #[serde(rename = "g")]
  pub genesis: DateTime<Utc>,
  /// expiry datetime
  #[serde(rename = "e")]
  pub expiry: Option<DateTime<Utc>>,
}

/// Strand content
pub type StrandContentV2 = ContentV2<StrandFields>;

impl Verifiable for StrandFields {
  type Error = VerificationError;
  /// Self-verification
  ///
  /// Verifications:
  /// - That the radix value is not 1
  fn verify(&self) -> Result<(), VerificationError> {
    if self.radix == 1 {
      return Err(VerificationError::InvalidTwineFormat(
        "Chain radix must not equal 1".into(),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::verify::Verifiable;
  use chrono::Utc;
  use ipld_core::ipld::Ipld;

  fn dummy_key() -> PublicKey {
    use crate::test::STRAND_V2_JSON;
    use crate::schemas::StrandSchemaVersion;
    use crate::twine::{Strand, TwineBlock};
    let strand = Strand::from_tagged_dag_json(STRAND_V2_JSON).unwrap();
    match &**strand.0 {
      StrandSchemaVersion::V2(c) => c.key().clone(),
      _ => panic!("expected V2 strand"),
    }
  }

  fn make_strand_fields(radix: u8) -> StrandFields {
    StrandFields {
      key: dummy_key(),
      radix,
      details: Ipld::Map(Default::default()),
      genesis: Utc::now(),
      expiry: None,
    }
  }

  // --- StrandFields::verify ----------------------------------------------

  #[test]
  fn strand_fields_zero_radix_ok() {
    // radix 0 means "no skiplist" and is valid
    let fields = make_strand_fields(0);
    assert!(fields.verify().is_ok(), "radix 0 must pass verification");
  }

  #[test]
  fn strand_fields_radix_2_ok() {
    let fields = make_strand_fields(2);
    assert!(fields.verify().is_ok(), "radix 2 must pass verification");
  }

  #[test]
  fn strand_fields_radix_32_ok() {
    let fields = make_strand_fields(32);
    assert!(fields.verify().is_ok(), "radix 32 must pass verification");
  }

  #[test]
  fn strand_fields_radix_1_rejected() {
    let fields = make_strand_fields(1);
    let result = fields.verify();
    assert!(result.is_err(), "radix 1 must fail verification");
    assert!(
      matches!(result, Err(VerificationError::InvalidTwineFormat(_))),
      "expected InvalidTwineFormat, got {:?}",
      result
    );
  }

  #[test]
  fn strand_fields_radix_255_ok() {
    let fields = make_strand_fields(255);
    assert!(fields.verify().is_ok(), "radix 255 must pass verification");
  }

  #[test]
  fn strand_fields_expiry_some_accessible() {
    let mut fields = make_strand_fields(4);
    fields.expiry = Some(Utc::now());
    assert!(fields.expiry.is_some());
    assert!(fields.verify().is_ok());
  }
}
