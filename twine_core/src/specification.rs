use std::fmt::Display;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::Error;
use semver::{Version, VersionReq};

const PREFIX: &str = "twine";

#[derive(Debug, Clone, PartialEq)]
pub struct VersionError(pub String);

impl std::fmt::Display for VersionError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "VersionError: {}", self.0)
  }
}

impl VersionError {
  pub fn new<S: Display>(message: S) -> VersionError {
    VersionError(message.to_string())
  }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Specification<const V: u8>(pub String);

impl<const V: u8> Specification<V> {
  pub fn parts(&self) -> (String, String, Option<Subspec>) {
    // has the form twine/1.0.x or twine/1.0.x/subspec/1.0.x
    let mut parts = self.0.splitn(3, '/');
    let prefix = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    // in version 1.0 it has an x. Remove it.
    let version = if V == 1 {
      let s = version.replace(".x", ".0");
      s
    } else { version.to_string() };
    let subspec = parts.next().map(|s| {
      Subspec(
        if V == 1 {
          // in version 1.0 we allow x. Remove it.
          s.replace(".x", ".0")
        } else { s.to_string() }
      )
    });
    (prefix.to_string(), version, subspec)
  }

  pub fn validate(&self) -> Result<(), VersionError> {
    // ensure either 1 or three /
    let count = self.0.chars().filter(|&c| c == '/').count();
    if count != 1 && count != 3 {
      return Err(VersionError::new("Specification string does not have the correct number of /"));
    }
    let (prefix, ver, subspec) = self.parts();
    if prefix != PREFIX {
      return Err(VersionError::new(format!("Specification string does not start with '{}'", PREFIX)));
    }
    let version = Version::parse(&ver).map_err(VersionError::new)?;
    if version.major != V as u64 {
      return Err(VersionError::new(format!("Expected different twine version. Expected: {}, Found: {}", V, version.major)));
    }
    subspec.map_or(Ok(()), |s| s.validate())?;
    Ok(())
  }

  pub fn semver(&self) -> Result<Version, semver::Error> {
    let (_, ver, _) = self.parts();
    Version::parse(&ver)
  }

  pub fn subspec(&self) -> Option<Subspec> {
    let (_, _, subspec) = self.parts();
    subspec
  }

  pub fn satisfies<S: Display>(&self, req: S) -> bool {
    if let Ok(version) = self.semver() {
      let req = VersionReq::parse(&req.to_string()).unwrap();
      req.matches(&version)
    } else { false }
  }
}

impl<'de, const V: u8> Deserialize<'de> for Specification<V> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de>
  {
    let s = String::deserialize(deserializer)?;
    // ensure the version is correct
    let spec = Specification(s);
    spec.validate().map_err(D::Error::custom)?;
    Ok(spec)
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subspec(pub String);

impl Subspec {
  pub fn parts(&self) -> (String, String) {
    // has the form subspec/1.0.0
    let mut parts = self.0.splitn(2, '/');
    let prefix = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    (prefix.to_string(), version.to_string())
  }

  pub fn validate(&self) -> Result<(), VersionError> {
    let (prefix, ver) = self.parts();
    if prefix.len() == 0 {
      return Err(VersionError::new("Subspec string does not have a prefix"));
    }
    Version::parse(&ver).map_err(VersionError::new)?;
    Ok(())
  }

  pub fn semver(&self) -> Result<Version, semver::Error> {
    let (_, ver) = self.parts();
    Version::parse(&ver)
  }

  pub fn satisfies<S: Display>(&self, req: S) -> bool {
    if let Ok(version) = self.semver() {
      let req = VersionReq::parse(&req.to_string()).unwrap();
      req.matches(&version)
    } else { false }
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
    let spec = Specification::<1>("twine/1.0.x".to_string());
    assert!(spec.validate().is_ok(), "{}", spec.validate().unwrap_err());
    assert_eq!(spec.semver().unwrap(), Version::parse("1.0.0").unwrap());
    assert_eq!(spec.subspec(), None);

    let spec = Specification::<1>("twine/1.0.x/subspec/1.0.x".to_string());
    assert!(spec.validate().is_ok(), "{}", spec.validate().unwrap_err());
    assert_eq!(spec.semver().unwrap(), Version::parse("1.0.0").unwrap());
    assert_eq!(spec.subspec(), Some(Subspec("subspec/1.0.0".to_string())));

    let spec = Specification::<2>("twine/2.0.1".to_string());
    assert!(spec.validate().is_ok(), "{}", spec.validate().unwrap_err());
    assert_eq!(spec.semver().unwrap(), Version::parse("2.0.1").unwrap());
    assert_eq!(spec.subspec(), None);

    let spec = Specification::<2>("twine/2.0.1/subspec/2.0.1".to_string());
    assert!(spec.validate().is_ok(), "{}", spec.validate().unwrap_err());
    assert_eq!(spec.semver().unwrap(), Version::parse("2.0.1").unwrap());
    assert_eq!(spec.subspec(), Some(Subspec("subspec/2.0.1".to_string())));

    // bad
    let spec = Specification::<2>("twine/1.0.x".to_string());
    assert!(spec.validate().is_err());

    let spec = Specification::<2>("twine/2.0.1/subspec/1.0.0/garbage".to_string());
    assert!(spec.validate().is_err());

    let spec = Specification::<1>("twine/1.0.1//1.0.0".to_string());
    assert!(spec.validate().is_err());
  }
}
