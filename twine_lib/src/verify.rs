//! Verification utilities for ensuring that data structures are valid.
use std::hash::Hash;
use serde::{Deserialize, Serialize};

/// Verifies that a collection of items are all unique.
pub fn is_all_unique<T: Eq + std::hash::Hash, I: IntoIterator<Item = T>>(iter: I) -> bool {
  let mut seen = std::collections::HashSet::new();
  for item in iter {
    if !seen.insert(item) {
      return false;
    }
  }
  true
}

/// A trait for types that can verify their own validity.
pub trait Verifiable {
  /// The error type that is returned when verification fails.
  type Error: std::fmt::Debug + std::fmt::Display;
  /// Verify the integrity of the data structure.
  fn verify(&self) -> Result<(), Self::Error>;
}

/// An opaque trait that can be implemented to verify the integrity of a data structure.
///
/// This trait implements deref so that the inner type can be accessed directly.
/// It is intended to be used on types that are deserialized from external sources.
/// When the type is deserialized, the `verify` method is called and if
/// it returns an error, the deserialization fails.
///
/// # Example
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use twine::verify::{Verifiable, Verified};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct GreaterThanZero(u32);
///
/// impl Verifiable for GreaterThanZero {
///   type Error = &'static str;
///   fn verify(&self) -> Result<(), Self::Error> {
///     if self.0 > 0 {
///       Ok(())
///     } else {
///       Err("Value must be greater than zero")
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Verified<T: Verifiable>(T);

impl<T> PartialEq for Verified<T>
where
  T: Verifiable + PartialEq,
{
  fn eq(&self, other: &Self) -> bool {
    self.as_inner() == other.as_inner()
  }
}

impl<T> Eq for Verified<T> where T: Verifiable + Eq {}

impl<T> Hash for Verified<T>
where
  T: Verifiable + Hash,
{
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.as_inner().hash(state);
  }
}

impl<T: Verifiable> Verified<T> {
  /// Create a new verified container.
  pub fn try_new(inner: T) -> Result<Self, T::Error> {
    inner.verify()?;
    Ok(Self(inner))
  }

  /// Consume the container and return the inner value.
  pub fn into_inner(self) -> T {
    self.0
  }

  /// Get a reference to the inner value.
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

impl<T: Verifiable> std::ops::DerefMut for Verified<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
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
  use crate::errors::VerificationError;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  struct TestStruct {
    value: u32,
  }

  impl Verifiable for TestStruct {
    type Error = VerificationError;
    fn verify(&self) -> Result<(), Self::Error> {
      if self.value == 42 {
        Ok(())
      } else {
        Err(VerificationError::InvalidTwineFormat(
          "Value is not 42".to_string(),
        ))
      }
    }
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  struct WithNested {
    value: u32,
    nested: Verified<TestStruct>,
  }

  impl Verifiable for WithNested {
    type Error = VerificationError;
    fn verify(&self) -> Result<(), Self::Error> {
      if self.value == 42 {
        Ok(())
      } else {
        Err(VerificationError::InvalidTwineFormat(
          "Value is not 42".to_string(),
        ))
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

  #[test]
  fn test_nested_deserialize() {
    let data = r#"{"value": 42, "nested": {"value": 42}}"#;
    let res: Result<WithNested, _> = serde_json::from_str(data);
    assert!(res.is_ok());

    let data = r#"{"value": 42, "nested": {"value": 9}}"#;
    let res: Result<WithNested, _> = serde_json::from_str(data);
    assert!(res.is_err());
  }
}
