//! Types that handle specification strings
use crate::errors::SpecificationError;
use semver::{Version, VersionReq};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;
use std::str::FromStr;

// TODO: consider using Verifiable trait and combining similar functionality

const PREFIX: &str = "twine";

/// Type for a specification string
///
/// Used internally to represent a specification string
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Specification<const V: u8>(pub(crate) String);

impl<const V: u8> FromStr for Specification<V> {
  type Err = SpecificationError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Specification::from_string(s)
  }
}

impl<const V: u8> Specification<V> {
  /// Create a new specification from a string
  pub fn from_string<S: Display>(s: S) -> Result<Self, SpecificationError> {
    let spec = Specification(s.to_string());
    spec.verify()?;
    Ok(spec)
  }

  /// Get the parts of the specification
  pub fn parts(&self) -> (String, String, Option<Subspec>) {
    // has the form twine/1.0.x or twine/1.0.x/subspec/1.0.x
    let mut parts = self.0.splitn(3, '/');
    let prefix = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    // in version 1.0 it has an x. Remove it.
    let version = if V == 1 {
      let s = version.replace(".x", ".0");
      s
    } else {
      version.to_string()
    };
    let subspec = parts.next().map(|s| {
      Subspec(if V == 1 {
        // in version 1.0 we allow x. Remove it.
        s.replace(".x", ".0")
      } else {
        s.to_string()
      })
    });
    (prefix.to_string(), version, subspec)
  }

  /// Verify the specification string
  pub fn verify(&self) -> Result<(), SpecificationError> {
    // ensure either 1 or three /
    let count = self.0.chars().filter(|&c| c == '/').count();
    if count != 1 && count != 3 {
      return Err(SpecificationError::new(
        "Specification string does not have the correct number of /",
      ));
    }
    let (prefix, ver, subspec) = self.parts();
    if prefix != PREFIX {
      return Err(SpecificationError::new(format!(
        "Specification string does not start with '{}'",
        PREFIX
      )));
    }
    let version = Version::parse(&ver).map_err(SpecificationError::new)?;
    if version.major != V as u64 {
      return Err(SpecificationError::new(format!(
        "Expected different twine version. Expected: {}, Found: {}",
        V, version.major
      )));
    }
    subspec.map_or(Ok(()), |s| s.verify())?;
    Ok(())
  }

  /// Get the semver of the specification
  pub fn semver(&self) -> Version {
    // at this point we know it's ok
    let (_, ver, _) = self.parts();
    Version::parse(&ver).unwrap()
  }

  /// Get the subspec
  pub fn subspec(&self) -> Option<Subspec> {
    let (_, _, subspec) = self.parts();
    subspec
  }

  /// Check if the specification satisfies a version requirement
  pub fn satisfies(&self, req: VersionReq) -> bool {
    let version = self.semver();
    req.matches(&version)
  }
}

impl<'de, const V: u8> Deserialize<'de> for Specification<V> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    // ensures the version is correct
    Ok(Specification::from_string(s).map_err(D::Error::custom)?)
  }
}

impl<const V: u8> TryFrom<String> for Specification<V> {
  type Error = SpecificationError;

  fn try_from(s: String) -> Result<Self, Self::Error> {
    Specification::from_string(s)
  }
}

/// A twine subspec
///
/// A subspec is part of the overall Twine specification string
/// and has the form {prefix}/{version}
#[derive(Debug, Clone, PartialEq)]
pub struct Subspec(pub(crate) String);

impl Subspec {
  /// Create a new subspec from a string
  pub fn from_string<S: Display>(s: S) -> Result<Self, SpecificationError> {
    let spec = Subspec(s.to_string());
    spec.verify()?;
    Ok(spec)
  }

  /// Get the parts of the subspec
  pub fn parts(&self) -> (String, String) {
    // has the form subspec/1.0.0
    let mut parts = self.0.splitn(2, '/');
    let prefix = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    (prefix.to_string(), version.to_string())
  }

  /// Verify the subspec string
  pub fn verify(&self) -> Result<(), SpecificationError> {
    let (prefix, ver) = self.parts();
    if prefix.len() == 0 {
      return Err(SpecificationError::new(
        "Subspec string does not have a prefix",
      ));
    }
    Version::parse(&ver).map_err(SpecificationError::new)?;
    Ok(())
  }

  /// Get the prefix of the subspec
  pub fn prefix(&self) -> String {
    let (prefix, _) = self.parts();
    prefix
  }

  /// Get the semver of the subspec
  pub fn semver(&self) -> Version {
    let (_, ver) = self.parts();
    Version::parse(&ver).unwrap()
  }

  /// Check if the subspec satisfies a version requirement
  pub fn satisfies(&self, req: VersionReq) -> bool {
    let version = self.semver();
    req.matches(&version)
  }
}

impl Display for Subspec {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_versions() {
    // good
    let spec = Specification::<1>::from_string("twine/1.0.x");
    assert!(spec.is_ok(), "{}", spec.unwrap_err());
    let spec = spec.unwrap();
    assert_eq!(spec.semver(), Version::parse("1.0.0").unwrap());
    assert_eq!(spec.subspec(), None);

    let spec = Specification::<1>::from_string("twine/1.0.x/subspec/1.0.x");
    assert!(spec.is_ok(), "{}", spec.unwrap_err());
    let spec = spec.unwrap();
    assert_eq!(spec.semver(), Version::parse("1.0.0").unwrap());
    assert_eq!(spec.subspec(), Some(Subspec("subspec/1.0.0".into())));

    let spec = Specification::<2>::from_string("twine/2.0.1");
    assert!(spec.is_ok(), "{}", spec.unwrap_err());
    let spec = spec.unwrap();
    assert_eq!(spec.semver(), Version::parse("2.0.1").unwrap());
    assert_eq!(spec.subspec(), None);

    let spec = Specification::<2>::from_string("twine/2.0.1/subspec/2.0.1");
    assert!(spec.is_ok(), "{}", spec.unwrap_err());
    let spec = spec.unwrap();
    assert_eq!(spec.semver(), Version::parse("2.0.1").unwrap());
    assert_eq!(spec.subspec(), Some(Subspec("subspec/2.0.1".into())));

    // bad
    let spec = Specification::<2>::from_string("twine/1.0.x");
    assert!(spec.is_err());

    let spec = Specification::<2>::from_string("twine/2.0.1/subspec/1.0.0/garbage");
    assert!(spec.is_err());

    let spec = Specification::<1>::from_string("twine/1.0.1//1.0.0");
    assert!(spec.is_err());
  }
}
