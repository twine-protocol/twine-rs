use crate::errors::VerificationError;
use serde::{Deserialize, Serialize};

pub fn is_all_unique<T: Eq + std::hash::Hash, I: IntoIterator<Item = T>>(iter: I) -> bool {
  let mut seen = std::collections::HashSet::new();
  for item in iter {
    if !seen.insert(item) {
      return false;
    }
  }
  true
}

/// Identifies data structures that can be verified.
pub trait Verifiable {
  fn verify(&self) -> Result<(), VerificationError>;
}

/// Container that identifies an inner structure that has been verified.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Verified<T: Verifiable>(T);

impl<T: Verifiable> Verified<T> {
  pub fn try_new(inner: T) -> Result<Self, VerificationError> {
    inner.verify()?;
    Ok(Self(inner))
  }

  pub fn into_inner(self) -> T {
    self.0
  }

  pub fn as_inner(&self) -> &T {
    &self.0
  }
}

impl<T: Verifiable> std::ops::Deref for Verified<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.as_inner()
  }
}

impl<'de, T: Verifiable + Deserialize<'de>> Deserialize<'de> for Verified<T> {
  fn deserialize<D>(deserializer: D) -> Result<Verified<T>, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let inner = T::deserialize(deserializer)?;
    Self::try_new(inner).map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  struct TestStruct {
    value: u32,
  }

  impl Verifiable for TestStruct {
    fn verify(&self) -> Result<(), VerificationError> {
      if self.value == 42 {
        Ok(())
      } else {
        Err(VerificationError::InvalidTwineFormat("Value is not 42".to_string()))
      }
    }
  }

  #[test]
  fn test_verified_struct() {
    let res = Verified::try_new(TestStruct { value: 42 });
    assert!(res.is_ok());

    let res = Verified::try_new(TestStruct { value: 9 });
    assert!(res.is_err());
  }
}
