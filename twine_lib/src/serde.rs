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

#[cfg(test)]
mod test {
  use crate::Ipld;
  use serde::{Deserialize, Serialize};

  #[derive(Debug, PartialEq, Serialize, Deserialize)]
  struct Wrapper {
    #[serde(with = "crate::serde::dag_json")]
    some_obj: Ipld,
  }

  #[test]
  fn round_trips_ipld_through_serde_json() {
    let original = Wrapper {
      some_obj: Ipld::List(vec![
        Ipld::Integer(42),
        Ipld::String("hello".into()),
        Ipld::Bool(true),
      ]),
    };

    // serde_json drives the wrapper, but the inner field is encoded as DAG-JSON.
    let json = serde_json::to_string(&original).unwrap();
    let decoded: Wrapper = serde_json::from_str(&json).unwrap();
    assert_eq!(original, decoded);
  }

  #[test]
  fn encodes_bytes_as_dag_json_link_form() {
    // DAG-JSON encodes byte strings specially (`{"/": {"bytes": ...}}`),
    // which is the whole point of routing through serde_ipld_dagjson.
    let original = Wrapper {
      some_obj: Ipld::Bytes(vec![1, 2, 3, 4]),
    };
    let json = serde_json::to_string(&original).unwrap();
    assert!(
      json.contains("bytes"),
      "expected DAG-JSON bytes encoding, got: {json}"
    );
    let decoded: Wrapper = serde_json::from_str(&json).unwrap();
    assert_eq!(original, decoded);
  }
}
