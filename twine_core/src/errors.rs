use serde_ipld_dagjson::error::CodecError as JsonCodecError;
use serde_ipld_dagcbor::error::CodecError as CborCodecError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerificationError {
  #[error("The data structure does not conform to any known Twine format")]
  InvalidTwineFormat,
  #[error("Problem parsing CBOR")]
  BadCbor(#[from] CborCodecError),
  #[error("Problem parsing JSON")]
  BadJson(#[from] JsonCodecError),
  #[error("Signature is invalid")]
  BadSignature,
  #[error("Bad signature format")]
  BadSignatureFormat,
  #[error("Unsupported key algorithm")]
  UnsupportedKeyAlgorithm,
  #[error("Malformed JWK")]
  MalformedJwk(#[from] anyhow::Error),
  #[error("Unsupported hash algorithm")]
  UnsupportedHashAlgorithm,
  #[error("Cid mismatch: expected {expected}, got {actual}")]
  CidMismatch {
    expected: String,
    actual: String,
  },
}
