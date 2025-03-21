use super::*;
use serde::{Deserialize, Serialize};

/// Common fields for the content field
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ContentV2<T: Clone + Send + Verifiable> {
  /// The hash [`Code`]
  #[serde(rename = "h")]
  pub code: HashCode,
  /// The specification
  #[serde(rename = "v")]
  pub specification: V2,

  #[serde(flatten)]
  pub fields: Verified<T>,
}

impl<T> ContentV2<T>
where
  T: Clone + Send + Verifiable,
{
  pub fn code(&self) -> &HashCode {
    &self.code
  }
}

impl<T> Deref for ContentV2<T>
where
  T: Clone + Send + Verifiable,
{
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.fields
  }
}

impl<T> Verifiable for ContentV2<T>
where
  T: Clone + Send + Verifiable,
{
  type Error = crate::errors::VerificationError;
  fn verify(&self) -> Result<(), crate::errors::VerificationError> {
    // no need to verify
    Ok(())
  }
}
