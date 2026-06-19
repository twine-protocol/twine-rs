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

#[cfg(test)]
mod test {
  use crate::{
    schemas::StrandSchemaVersion,
    test::STRAND_V2_JSON,
    twine::{Strand, TwineBlock},
  };

  fn strand_container() -> super::super::StrandContainerV2 {
    let strand = Strand::from_tagged_dag_json(STRAND_V2_JSON).unwrap();
    match &**strand.0 {
      StrandSchemaVersion::V2(c) => c.clone(),
      _ => panic!("expected V2"),
    }
  }

  // ContentV2 is accessed only through the ContainerV2; we reach its
  // fields (code, specification) through the container's methods.

  #[test]
  fn content_v2_verify_always_ok() {
    // ContentV2::verify is a no-op wrapper; it must always return Ok.
    // We exercise it indirectly by loading the container (deserialization
    // triggers Verified::try_new which calls verify).
    let _c = strand_container();
    // If we reach here, ContentV2::verify returned Ok.
  }

  #[test]
  fn content_v2_code_is_accessible() {
    // The hash code embedded in the content (h field) determines how the CID
    // is computed. It must be accessible without panic.
    let strand = Strand::from_tagged_dag_json(STRAND_V2_JSON).unwrap();
    match &**strand.0 {
      StrandSchemaVersion::V2(c) => {
        // content_bytes() uses the hash code internally
        assert!(c.content_bytes().is_ok());
      }
      _ => panic!("expected V2"),
    }
  }
}
