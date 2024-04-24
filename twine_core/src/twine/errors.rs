use serde_ipld_dagjson::error::CodecError as JsonCodecError;
use serde_ipld_dagcbor::error::CodecError as CborCodecError;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct ParseError(pub String);

impl Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "ParseError: {}", self.0)
  }
}

impl From<JsonCodecError> for ParseError {
  fn from(e: JsonCodecError) -> Self {
    ParseError(format!("JsonCodecError: {}", e))
  }
}

impl From<CborCodecError> for ParseError {
  fn from(e: CborCodecError) -> Self {
    ParseError(format!("CborCodecError: {}", e))
  }
}

impl From<serde_json::Error> for ParseError {
  fn from(e: serde_json::Error) -> Self {
    ParseError(format!("JsonError: {}", e))
  }
}

impl From<libipld::multihash::Error> for ParseError {
  fn from(e: libipld::multihash::Error) -> Self {
    ParseError(format!("MultihashError: {}", e))
  }
}
