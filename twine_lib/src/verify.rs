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
/// use twine_lib::verify::{Verifiable, Verified};
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

  // --- is_all_unique ----------------------------------------------------

  #[test]
  fn is_all_unique_empty_is_true() {
    assert!(is_all_unique(std::iter::empty::<u32>()));
  }

  #[test]
  fn is_all_unique_single_element_is_true() {
    assert!(is_all_unique(vec![1u32]));
  }

  #[test]
  fn is_all_unique_all_distinct_is_true() {
    assert!(is_all_unique(vec![1u32, 2, 3, 4, 5]));
  }

  #[test]
  fn is_all_unique_with_duplicate_is_false() {
    assert!(!is_all_unique(vec![1u32, 2, 3, 2, 4]));
  }

  #[test]
  fn is_all_unique_all_same_is_false() {
    assert!(!is_all_unique(vec![7u32, 7, 7]));
  }

  #[test]
  fn is_all_unique_strings() {
    assert!(is_all_unique(vec!["a", "b", "c"]));
    assert!(!is_all_unique(vec!["a", "b", "a"]));
  }

  // --- Verified<T> Deref and accessors ----------------------------------

  #[test]
  fn verified_deref_gives_inner_value() {
    let v = Verified::try_new(TestStruct { value: 42 }).unwrap();
    // Deref should give access to inner fields
    assert_eq!(v.value, 42);
  }

  #[test]
  fn verified_as_inner_matches_deref() {
    let v = Verified::try_new(TestStruct { value: 42 }).unwrap();
    assert_eq!(v.as_inner().value, 42);
    assert_eq!((*v).value, 42);
  }

  #[test]
  fn verified_into_inner_consumes() {
    let v = Verified::try_new(TestStruct { value: 42 }).unwrap();
    let inner = v.into_inner();
    assert_eq!(inner.value, 42);
  }

  #[test]
  fn verified_try_new_propagates_error() {
    let res = Verified::try_new(TestStruct { value: 0 });
    assert!(
      res.is_err(),
      "try_new must propagate verification error"
    );
    // Error type should be VerificationError::InvalidTwineFormat
    let err = res.unwrap_err();
    assert!(
      matches!(err, VerificationError::InvalidTwineFormat(_)),
      "expected InvalidTwineFormat, got {:?}",
      err
    );
  }

  #[test]
  fn verified_deserialize_triggers_verify() {
    // Deserialization of Verified<T> must call verify() and fail if invalid.
    let invalid_json = r#"{"value": 0}"#;
    let res: Result<Verified<TestStruct>, _> = serde_json::from_str(invalid_json);
    assert!(res.is_err(), "deserialization of invalid T must fail");
  }

  #[test]
  fn verified_deserialize_succeeds_for_valid() {
    let valid_json = r#"{"value": 42}"#;
    let res: Result<Verified<TestStruct>, _> = serde_json::from_str(valid_json);
    assert!(res.is_ok(), "deserialization of valid T must succeed");
    assert_eq!(res.unwrap().value, 42);
  }

  #[test]
  fn verified_equality() {
    let a = Verified::try_new(TestStruct { value: 42 }).unwrap();
    let b = Verified::try_new(TestStruct { value: 42 }).unwrap();
    assert_eq!(a, b);
  }
}
