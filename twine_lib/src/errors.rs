use crate::resolver::SingleQuery;
use serde_ipld_dagcbor::error::CodecError as CborCodecError;
use serde_ipld_dagjson::error::CodecError as JsonCodecError;
use std::{convert::Infallible, fmt::Display};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerificationError {
  #[error("The tixel does not belong to the supplied strand")]
  TixelNotOnStrand,
  #[error("The data structure does not conform to any known Twine format {0}")]
  InvalidTwineFormat(String),
  #[error("Problem parsing CBOR because: {0}")]
  BadCbor(#[from] CborCodecError),
  #[error("Problem parsing JSON because: {0}")]
  BadJson(#[from] JsonCodecError),
  #[error("Signature is invalid: {0}")]
  BadSignature(String),
  #[error("Unsupported key algorithm")]
  UnsupportedKeyAlgorithm,
  #[error("Malformed JWK")]
  MalformedJwk(#[from] anyhow::Error),
  #[error("Unsupported hash algorithm")]
  UnsupportedHashAlgorithm,
  #[error("Cid mismatch: expected {expected}, got {actual}")]
  CidMismatch { expected: String, actual: String },
  #[error("Twine has wrong type: expected {expected}, found {found}")]
  WrongType { expected: String, found: String },
  #[error("Bad Specification: {0}")]
  BadSpecification(#[from] SpecificationError),
  #[error("General error: {0}")]
  General(String),
  #[error("Payload invalid: {0}")]
  Payload(String),
}

impl From<Infallible> for VerificationError {
  fn from(_: Infallible) -> Self {
    unreachable!()
  }
}

#[derive(Error, Debug)]
pub enum ResolutionError {
  #[error("Twine not found")]
  NotFound,
  #[error("Twine is invalid: {0}")]
  Invalid(#[from] VerificationError),
  #[error("Bad data: {0}")]
  BadData(String),
  #[error("Data does not match query: {0}")]
  QueryMismatch(SingleQuery),
  #[error("Problem fetching data: {0}")]
  Fetch(String),
}

#[derive(Error, Debug)]
pub enum StoreError {
  #[error("Twine is invalid: {0}")]
  Invalid(#[from] VerificationError),
  #[error("Problem saving data: {0}")]
  Saving(String),
  #[error("Problem fetching data: {0}")]
  Fetching(#[from] ResolutionError),
}

#[derive(Debug, Error)]
pub struct SpecificationError(pub String);

impl std::fmt::Display for SpecificationError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SpecificationError: {}", self.0)
  }
}

impl SpecificationError {
  pub fn new<S: Display>(message: S) -> Self {
    Self(message.to_string())
  }
}

#[derive(Debug, Error)]
pub enum ConversionError {
  #[error("Invalid format: {0}")]
  InvalidFormat(String),
  #[error("Invalid CID: {0}")]
  InvalidCid(#[from] ipld_core::cid::Error),
  #[error("Invalid index value: {0}")]
  InvalidIndex(#[from] std::num::ParseIntError),
}
