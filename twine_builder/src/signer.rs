//! Defines the `Signer` trait for creating digital signatures
use ring::signature::Ed25519KeyPair;
use std::fmt::Display;
use twine_lib::crypto::{PublicKey, Signature, SignatureAlgorithm};

/// An error that occurs when signing data.
#[derive(Debug, thiserror::Error)]
pub struct SigningError(pub String);

impl Display for SigningError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SigningError: {}", self.0)
  }
}

/// Constructing Twine data requires the ability to create digital signatures. Types
/// that provide this functionality should implement the [`Signer`] trait. There are two
/// signers included in this library:
///
/// - [`crate::RingSigner`] for v2 data, that uses [`ring`](https://docs.rs/ring/latest/ring/).
/// - [`crate::BiscuitSigner`] for v1 data, that uses [`biscuit`](https://docs.rs/biscuit/0.7.0/biscuit/).
///
/// Begin by creating a signer. For RingSigner, this can be done by importing a
/// private key as a PEM file. For example:
///
/// ```rust
/// use twine_builder::{RingSigner, Signer};
/// const PRIVATE_KEY_ED25519_PEM: &'static str = r#"
/// -----BEGIN PRIVATE KEY-----
/// MFECAQEwBQYDK2VwBCIEIJHCvDsbaia6M9aMlRXjdIMVbMyeGLwj/2crnzzoJnmH
/// gSEALX8wMpAh1EA0zraJTfEUx8F2uQBCvBmFkYpmvpX+jDc=
/// -----END PRIVATE KEY-----
/// "#;
///
/// let signer = RingSigner::from_pem(PRIVATE_KEY_ED25519_PEM).unwrap();
/// // print the public key
/// println!("{:?}", signer.public_key());
/// ```
pub trait Signer {
  /// The type of public key that this signer produces.
  type Key;
  /// Sign the given data and return the signature.
  ///
  /// The data is the message to sign.
  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<Signature, SigningError>;
  /// Get the public key for this signer.
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
