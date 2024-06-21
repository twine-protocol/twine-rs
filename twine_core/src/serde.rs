use serde::{Deserialize, Serialize, Serializer};

/// For use with serde_with to serialize and deserialize IPLD DAG-JSON
pub mod dag_json {
  use super::*;

  pub fn serialize<S: Serializer, T: Serialize>(value: &T, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    let ser = ::serde_ipld_dagjson::Serializer::new(serializer);
    value.serialize(ser)
  }

  pub fn deserialize<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> std::result::Result<T, D::Error> {
    let de = ::serde_ipld_dagjson::Deserializer::new(deserializer);
    Deserialize::deserialize(de)
  }
}
