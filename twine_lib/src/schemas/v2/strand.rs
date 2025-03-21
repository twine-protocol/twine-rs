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
