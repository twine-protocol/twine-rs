use std::fmt::Display;
use josekit::jwk::Jwk;
use crate::crypto::sign;

#[derive(Debug, thiserror::Error)]
pub struct SigningError(pub String);

impl Display for SigningError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SigningError: {}", self.0)
  }
}

pub trait Signer {
  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<String, SigningError>;
  fn public_key(&self) -> Jwk;
}

impl Signer for Jwk {
  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<String, SigningError> {
    sign(self, data)
  }

  fn public_key(&self) -> Jwk {
    self.to_public_key().expect("Could not get public key")
  }
}
