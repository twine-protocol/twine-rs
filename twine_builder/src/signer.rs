//! Defines the `Signer` trait for creating digital signatures
use std::fmt::Display;
use twine_lib::crypto::Signature;
#[cfg(feature = "ring-signer")]
use twine_lib::crypto::{PublicKey, SignatureAlgorithm};

/// An error that occurs when signing data.
#[derive(Debug, thiserror::Error)]
pub struct SigningError(pub String);

impl Display for SigningError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "SigningError: {}", self.0)
  }
}

/// Constructing Twine data requires the ability to create digital signatures. Types
/// that provide this functionality should implement the [`Signer`] trait. The
/// signers included in this library are:
///
/// - [`crate::RustCryptoSigner`] for v2 data, the recommended pure-Rust signer
///   (enable the `rustcrypto-signer` feature).
/// - [`crate::BiscuitSigner`] for v1 data, that uses [`biscuit`](https://docs.rs/biscuit/0.7.0/biscuit/).
///
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

#[cfg(feature = "ring-signer")]
impl Signer for ring::signature::Ed25519KeyPair {
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

#[cfg(all(test, feature = "ring-signer"))]
mod test {
  use super::*;

  /// Construct an Ed25519KeyPair via ring's PKCS#8 keygen, call `sign` and
  /// `public_key` through the `Signer` trait, and verify the round-trip.
  #[test]
  fn test_ed25519_keypair_sign_verify_roundtrip() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)
      .expect("keygen must succeed");
    let keypair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
      .expect("key load must succeed");

    const MSG: &[u8] = b"twine signer trait test";
    let sig = Signer::sign(&keypair, MSG).expect("sign must succeed");
    let pk = Signer::public_key(&keypair);

    // The public key must carry the Ed25519 algorithm.
    assert!(
      matches!(pk.alg, SignatureAlgorithm::Ed25519),
      "public_key() must report Ed25519"
    );
    assert!(!pk.key.is_empty(), "public key bytes must not be empty");

    // Valid signature must verify.
    pk.verify(sig.clone(), MSG)
      .expect("valid signature must verify");

    // Tampered message must fail.
    let mut tampered = MSG.to_vec();
    tampered[0] ^= 0xFF;
    pk.verify(sig, tampered.as_slice())
      .expect_err("tampered message must not verify");
  }

  /// `sign` must not panic when given an empty message.
  #[test]
  fn test_ed25519_keypair_sign_empty_message() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let keypair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();

    let sig = Signer::sign(&keypair, b"" as &[u8]).expect("sign of empty msg must succeed");
    let pk = Signer::public_key(&keypair);
    pk.verify(sig, b"" as &[u8])
      .expect("empty-message signature must verify");
  }
}
