use chrono::{Utc, DateTime};
use crate::errors::VerificationError;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrandFields {
  #[serde(rename = "k")]
  pub(super) key: PublicKey,
  #[serde(rename = "r")]
  pub(super) radix: u8,
  #[serde(rename = "d")]
  pub(super) details: Ipld,
  #[serde(rename = "g")]
  pub(super) genesis: DateTime<Utc>,
  #[serde(rename = "e")]
  pub(super) expiry: Option<DateTime<Utc>>,
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
