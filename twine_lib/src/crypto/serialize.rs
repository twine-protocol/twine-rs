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

  #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
  struct MyStruct {
    x: u32,
    name: String,
  }

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
