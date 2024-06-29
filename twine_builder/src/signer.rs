use std::fmt::Display;
use ring::signature::Ed25519KeyPair;
use twine_core::crypto::{PublicKey, SignatureAlgorithm, Signature};

#[derive(Debug, thiserror::Error)]
pub struct SigningError(pub String);

impl Display for SigningError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SigningError: {}", self.0)
  }
}

pub trait Signer {
  type Key;
  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<Signature, SigningError>;
  fn public_key(&self) -> Self::Key;
}

impl Signer for Ed25519KeyPair {
  type Key = PublicKey;

  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<Signature, SigningError> {
    Ok(self.sign(data.as_ref()).as_ref().into())
  }

  fn public_key(&self) -> Self::Key {
    PublicKey {
      alg: SignatureAlgorithm::Ed25519,
      key: ring::signature::KeyPair::public_key(self).as_ref().into(),
    }
  }
}
