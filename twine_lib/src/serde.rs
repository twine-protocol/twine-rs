//! Serialization helpers
use serde::{Deserialize, Serialize, Serializer};

/// For use with serde_with to serialize and deserialize IPLD DAG-JSON
///
/// Mainly useful when using some framework that deserializes using
/// serde_json, but you want it to use dag_json.
///
/// # Example
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use twine_lib::Ipld;
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyStruct {
///   #[serde(with = "twine_lib::serde::dag_json")]
///   some_obj: Ipld,
/// }
/// ```
pub mod dag_json {
  use super::*;

  #[allow(missing_docs)]
  pub fn serialize<S: Serializer, T: Serialize>(
    value: &T,
    serializer: S,
  ) -> std::result::Result<S::Ok, S::Error> {
    let ser = ::serde_ipld_dagjson::Serializer::new(serializer);
    value.serialize(ser)
  }

  #[allow(missing_docs)]
  pub fn deserialize<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
  ) -> std::result::Result<T, D::Error> {
    let de = ::serde_ipld_dagjson::Deserializer::new(deserializer);
    Deserialize::deserialize(de)
  }
}
