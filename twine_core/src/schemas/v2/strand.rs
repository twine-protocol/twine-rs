use chrono::{Utc, DateTime};
use crate::errors::VerificationError;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrandFields {
  #[serde(rename = "k")]
  pub key: PublicKey,
  #[serde(rename = "r")]
  pub radix: u8,
  #[serde(rename = "d")]
  pub details: Ipld,
  #[serde(rename = "g")]
  pub genesis: DateTime<Utc>,
  #[serde(rename = "e")]
  pub expiry: Option<DateTime<Utc>>,
}

pub type StrandContentV2 = ContentV2<StrandFields>;

impl Verifiable for StrandFields {
  fn verify(&self) -> Result<(), VerificationError> {
    if self.radix == 1 {
      return Err(VerificationError::InvalidTwineFormat("Chain radix must not equal 1".into()));
    }

    Ok(())
  }
}
