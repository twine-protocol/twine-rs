use std::fmt::Display;
use biscuit::{jwk::JWK, jws::Secret};
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
  fn public_key(&self) -> JWK<()>;
}

impl Signer for Secret {
  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<String, SigningError> {
    sign(self, data)
  }

  fn public_key(&self) -> JWK<()> {
    use ring::signature::KeyPair;
    unimplemented!()
    // match self {
    //   Secret::RsaKeyPair(rsa) => Secret::PublicKey(rsa.public_key().as_ref()),
    //   Secret::EcKeyPair(ec) => ec.public_key(),
    //   _ => panic!("Unsupported key type"),
    // }
  }
}
