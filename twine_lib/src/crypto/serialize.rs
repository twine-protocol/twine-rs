use serde::Serialize;
use serde_ipld_dagcbor::EncodeError;
use std::collections::TryReserveError;

/// Serialize a type to a CBOR-encoded byte vector
pub fn crypto_serialize<S: Serialize>(input: S) -> Result<Vec<u8>, EncodeError<TryReserveError>> {
  let bytes = serde_ipld_dagcbor::to_vec(&input)?;
  Ok(bytes)
}

#[cfg(test)]
mod test {
  use super::*;
  use serde::Serialize;

  // -----------------------------------------------------------------------
  // Basic round-trip: serialize a simple value
  // -----------------------------------------------------------------------

  #[test]
  fn serialize_integer() {
    let result = crypto_serialize(42u64).unwrap();
    // CBOR for integer 42 is 0x18 0x2a (one-byte uint).
    assert!(!result.is_empty());
    let decoded: u64 = serde_ipld_dagcbor::from_slice(&result).unwrap();
    assert_eq!(decoded, 42);
  }

  #[test]
  fn serialize_string() {
    let result = crypto_serialize("hello").unwrap();
    assert!(!result.is_empty());
    let decoded: String = serde_ipld_dagcbor::from_slice(&result).unwrap();
    assert_eq!(decoded, "hello");
  }

  // -----------------------------------------------------------------------
  // Struct serialization
  // -----------------------------------------------------------------------

  #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
  struct MyStruct {
    x: u32,
    name: String,
  }

  #[test]
  fn serialize_struct_roundtrip() {
    let input = MyStruct {
      x: 7,
      name: "twine".to_string(),
    };
    let bytes = crypto_serialize(&input).unwrap();
    let decoded: MyStruct = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
    assert_eq!(decoded, input);
  }

  // -----------------------------------------------------------------------
  // Determinism: same input always produces same output
  // -----------------------------------------------------------------------

  #[test]
  fn serialize_is_deterministic() {
    let input = MyStruct {
      x: 42,
      name: "deterministic".to_string(),
    };
    let a = crypto_serialize(&input).unwrap();
    let b = crypto_serialize(&input).unwrap();
    assert_eq!(a, b, "crypto_serialize is not deterministic");
  }

  // -----------------------------------------------------------------------
  // Different values produce different outputs
  // -----------------------------------------------------------------------

  #[test]
  fn different_inputs_differ() {
    let a = crypto_serialize("foo").unwrap();
    let b = crypto_serialize("bar").unwrap();
    assert_ne!(a, b);
  }

  // -----------------------------------------------------------------------
  // Empty collections / None produce non-empty, different bytes
  // -----------------------------------------------------------------------

  #[test]
  fn serialize_empty_vec() {
    let v: Vec<u8> = vec![];
    let bytes = crypto_serialize(&v).unwrap();
    assert!(!bytes.is_empty());
  }

  #[test]
  fn serialize_empty_differs_from_nonempty() {
    let empty = crypto_serialize::<Vec<u8>>(vec![]).unwrap();
    let nonempty = crypto_serialize::<Vec<u8>>(vec![1, 2, 3]).unwrap();
    assert_ne!(empty, nonempty);
  }

  // -----------------------------------------------------------------------
  // Nested structure
  // -----------------------------------------------------------------------

  #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
  struct Nested {
    inner: MyStruct,
    flag: bool,
  }

  #[test]
  fn serialize_nested_roundtrip() {
    let input = Nested {
      inner: MyStruct {
        x: 99,
        name: "inner".to_string(),
      },
      flag: true,
    };
    let bytes = crypto_serialize(&input).unwrap();
    let decoded: Nested = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
    assert_eq!(decoded, input);
  }
}
