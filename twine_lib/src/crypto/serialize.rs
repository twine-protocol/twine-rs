use serde::Serialize;
use serde_ipld_dagcbor::EncodeError;
use std::collections::TryReserveError;

/// Serialize a type to a CBOR-encoded byte vector
pub fn crypto_serialize<S: Serialize>(input: S) -> Result<Vec<u8>, EncodeError<TryReserveError>> {
  let bytes = serde_ipld_dagcbor::to_vec(&input)?;
  Ok(bytes)
}
