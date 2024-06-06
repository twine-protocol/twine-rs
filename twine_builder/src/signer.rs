use std::fmt::Display;
use biscuit::jwk::JWK;

#[derive(Debug, thiserror::Error)]
pub struct SigningError(pub String);

impl Display for SigningError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SigningError: {}", self.0)
  }
}

pub trait Signer {
  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<String, SigningError>;
  fn public_key(&self) -> JWK<()>;
}
