//! Various error types used throughout the Twine library
use crate::resolver::SingleQuery;
use serde_ipld_dagcbor::error::CodecError as CborCodecError;
use serde_ipld_dagjson::error::CodecError as JsonCodecError;
use std::{convert::Infallible, fmt::Display};
use thiserror::Error;

/// Errors that can occur during verification of Twine data structures
#[derive(Debug, Error)]
pub enum VerificationError {
  /// Indicates that a Tixel's declared strand is different from the expected strand
  #[error("The tixel does not belong to the supplied strand")]
  TixelNotOnStrand,
  /// Indicates that the data structure does not conform to any known Twine format
  #[error("The data structure does not conform to any known Twine format {0}")]
  InvalidTwineFormat(String),
  /// Indicates a problem decoding CBOR data
  #[error("Problem parsing CBOR because: {0}")]
  BadCbor(#[from] CborCodecError),
  /// Indicates a problem decoding DAG-JSON data
  #[error("Problem parsing JSON because: {0}")]
  BadJson(#[from] JsonCodecError),
  /// Indicates that a signature is invalid
  #[error("Signature is invalid: {0}")]
  BadSignature(String),
  /// Indicates that a strand's key algorithm is unsupported for verification
  #[error("Unsupported key algorithm")]
  UnsupportedKeyAlgorithm,
  /// Indicates that a key is too weak to be trusted (e.g. an undersized RSA modulus)
  #[error("Key does not meet minimum strength requirements: {0}")]
  WeakKey(String),
  /// Indicates incorrectly formatted JWK in Twine v1 specification
  #[error("Malformed JWK")]
  MalformedJwk(#[from] anyhow::Error),
  /// Indicates that a hash algorithm is unsupported for verification
  ///
  /// This could be due to feature flags for those algorithms not being enabled
  #[error("Unsupported hash algorithm")]
  UnsupportedHashAlgorithm,
  /// Indicates that the expected CID does not match the actual computed CID
  #[allow(missing_docs)]
  #[error("Cid mismatch: expected {expected}, got {actual}")]
  CidMismatch { expected: String, actual: String },
  /// Indicates that strand/tixel data structure was expected, but the other was provided
  #[allow(missing_docs)]
  #[error("Twine has wrong type: expected {expected}, found {found}")]
  WrongType { expected: String, found: String },
  /// Indicates that the specification string is invalid
  #[error("Bad Specification: {0}")]
  BadSpecification(#[from] SpecificationError),
  /// Catch-all for general errors
  #[error("General error: {0}")]
  General(String),
  /// Indicates that a payload is invalid.
  ///
  /// This is intended for use by third-party libraries providing
  /// sub-specifications for Twine.
  #[error("Payload invalid: {0}")]
  Payload(String),
}

impl From<Infallible> for VerificationError {
  fn from(_: Infallible) -> Self {
    unreachable!()
  }
}

// TODO: add impl for .is_not_found() to ResolutionError

/// Errors that can occur in Resolver operations
#[derive(Error, Debug)]
pub enum ResolutionError {
  /// Indicates that a tixel or strand was not found
  #[error("Twine not found")]
  NotFound,
  /// Indicates invalid Twine data
  #[error("Twine is invalid: {0}")]
  Invalid(#[from] VerificationError),
  /// Indicates unreadable Twine data
  #[error("Bad data: {0}")]
  BadData(String),
  /// Indicates that the retrieved data does not match the query
  /// used to resolve it.
  #[error("Data does not match query: {0}")]
  QueryMismatch(SingleQuery),
  /// Indicates a general problem when fetching data
  ///
  /// For example, a network error or a problem with the underlying storage
  #[error("Problem fetching data: {0}")]
  Fetch(String),
}

/// Errors that can occur in Store operations
#[derive(Error, Debug)]
pub enum StoreError {
  /// Indicates invalid Twine data
  #[error("Twine is invalid: {0}")]
  Invalid(#[from] VerificationError),
  /// Indicates a problem saving the data
  #[error("Problem saving data: {0}")]
  Saving(String),
  /// Indicates a problem fetching the data
  #[error("Problem fetching data: {0}")]
  Fetching(#[from] ResolutionError),
}

/// Errors that can occur when parsing a Twine specification string
#[derive(Debug, Error)]
pub struct SpecificationError(pub String);

impl std::fmt::Display for SpecificationError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SpecificationError: {}", self.0)
  }
}

impl SpecificationError {
  /// Create a new SpecificationError
  pub fn new<S: Display>(message: S) -> Self {
    Self(message.to_string())
  }
}

/// Errors that can occur when converting between types
///
/// This mainly happens in the `twine_lib::query` module
#[derive(Debug, Error)]
pub enum ConversionError {
  /// Indicates that the data is not in the expected format
  #[error("Invalid format: {0}")]
  InvalidFormat(String),
  /// Indicates an invalid CID
  #[error("Invalid CID: {0}")]
  InvalidCid(#[from] ipld_core::cid::Error),
  /// Indicates an invalid index value
  #[error("Invalid index value: {0}")]
  InvalidIndex(#[from] std::num::ParseIntError),
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn verification_error_display_covers_all_variants() {
    let cases: Vec<(VerificationError, &str)> = vec![
      (VerificationError::TixelNotOnStrand, "does not belong"),
      (
        VerificationError::InvalidTwineFormat("nope".into()),
        "does not conform",
      ),
      (VerificationError::BadSignature("oops".into()), "invalid"),
      (
        VerificationError::UnsupportedKeyAlgorithm,
        "Unsupported key algorithm",
      ),
      (VerificationError::WeakKey("1024 bit".into()), "strength"),
      (
        VerificationError::UnsupportedHashAlgorithm,
        "Unsupported hash algorithm",
      ),
      (
        VerificationError::CidMismatch {
          expected: "a".into(),
          actual: "b".into(),
        },
        "mismatch",
      ),
      (
        VerificationError::WrongType {
          expected: "strand".into(),
          found: "tixel".into(),
        },
        "wrong type",
      ),
      (VerificationError::General("boom".into()), "General"),
      (VerificationError::Payload("bad".into()), "Payload invalid"),
    ];
    for (err, needle) in cases {
      let msg = err.to_string();
      assert!(
        msg.contains(needle),
        "expected {needle:?} in {msg:?}"
      );
    }
  }

  #[test]
  fn verification_error_from_bad_cbor_and_json() {
    // Decoding garbage yields a DecodeError, which converts into the
    // codec-level CodecError and then into our VerificationError.
    let cbor: VerificationError =
      CborCodecError::from(serde_ipld_dagcbor::from_slice::<u32>(&[0xff, 0xff]).unwrap_err())
        .into();
    assert!(matches!(cbor, VerificationError::BadCbor(_)));
    assert!(cbor.to_string().contains("CBOR"));

    let json: VerificationError =
      JsonCodecError::from(serde_ipld_dagjson::from_slice::<u32>(b"not json").unwrap_err())
        .into();
    assert!(matches!(json, VerificationError::BadJson(_)));
    assert!(json.to_string().contains("JSON"));
  }

  #[test]
  fn verification_error_from_anyhow_and_specification() {
    let jwk: VerificationError = anyhow::anyhow!("malformed").into();
    assert!(matches!(jwk, VerificationError::MalformedJwk(_)));
    assert!(jwk.to_string().contains("Malformed JWK"));

    let spec: VerificationError = SpecificationError::new("bad spec").into();
    assert!(matches!(spec, VerificationError::BadSpecification(_)));
    assert!(spec.to_string().contains("Bad Specification"));
  }

  #[test]
  fn resolution_error_display() {
    assert_eq!(ResolutionError::NotFound.to_string(), "Twine not found");
    assert!(ResolutionError::BadData("x".into())
      .to_string()
      .contains("Bad data"));
    assert!(ResolutionError::Fetch("net".into())
      .to_string()
      .contains("fetching"));
    // From<VerificationError>
    let invalid: ResolutionError = VerificationError::TixelNotOnStrand.into();
    assert!(matches!(invalid, ResolutionError::Invalid(_)));
    assert!(invalid.to_string().contains("invalid"));
  }

  #[test]
  fn store_error_display_and_from() {
    let invalid: StoreError = VerificationError::UnsupportedKeyAlgorithm.into();
    assert!(matches!(invalid, StoreError::Invalid(_)));
    assert!(StoreError::Saving("disk full".into())
      .to_string()
      .contains("saving"));
    let fetching: StoreError = ResolutionError::NotFound.into();
    assert!(matches!(fetching, StoreError::Fetching(_)));
    assert!(fetching.to_string().contains("fetching"));
  }

  #[test]
  fn specification_error_display_and_new() {
    let err = SpecificationError::new("oops");
    assert_eq!(err.0, "oops");
    assert!(err.to_string().contains("SpecificationError: oops"));
  }

  #[test]
  fn conversion_error_display_and_from() {
    assert!(ConversionError::InvalidFormat("x".into())
      .to_string()
      .contains("Invalid format"));

    let cid_err: ConversionError = "not-a-cid"
      .parse::<crate::Cid>()
      .unwrap_err()
      .into();
    assert!(matches!(cid_err, ConversionError::InvalidCid(_)));
    assert!(cid_err.to_string().contains("Invalid CID"));

    let idx_err: ConversionError = "abc".parse::<u64>().unwrap_err().into();
    assert!(matches!(idx_err, ConversionError::InvalidIndex(_)));
    assert!(idx_err.to_string().contains("Invalid index"));
  }
}
