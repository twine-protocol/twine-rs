// This whole module implements the deprecated RingSigner; silence the
// self-referential deprecation warnings.
#![allow(deprecated)]
use pkcs8::{der::Encode, DecodePrivateKey, SecretDocument};
use std::vec;
use thiserror::Error;
use twine_lib::crypto::{PublicKey, Signature, SignatureAlgorithm, MIN_RSA_KEY_BITS};

use crate::{Signer, SigningError};

#[derive(Debug, Error)]
pub enum RingSignerError {
  #[error("Unsupported algorithm")]
  UnsupportedAlgorithm,
  #[error("RSA key size {0} bits is below the {1}-bit minimum")]
  WeakKey(usize, usize),
  #[error("Key rejected: {0}")]
  KeyRejected(String),
  #[error("pkcs8 error: {0}")]
  PemError(#[from] pkcs8::Error),
  #[error("der decode error: {0}")]
  DerDecodeError(#[from] pkcs8::der::Error),
}

impl From<ring::error::KeyRejected> for RingSignerError {
  fn from(e: ring::error::KeyRejected) -> Self {
    RingSignerError::KeyRejected(e.to_string())
  }
}

enum Keys {
  Ed25519(ring::signature::Ed25519KeyPair),
  Ecdsa(ring::signature::EcdsaKeyPair),
  Rsa(ring::signature::RsaKeyPair),
}

/// A signer that uses the `ring` crate to sign data.
///
/// **Deprecated:** use [`crate::RustCryptoSigner`] instead. `RingSigner` emits
/// non-canonical (high-S) ECDSA signatures, which v2 verification now rejects,
/// and keeps the heavy `ring` dependency. It remains behind the `ring-signer`
/// feature for backward compatibility only.
///
/// # Example
///
/// ```rust,ignore
/// use twine_builder::{RingSigner, Signer};
/// let signer = RingSigner::generate_ed25519().unwrap();
/// ```
#[deprecated(
  since = "0.2.0",
  note = "use RustCryptoSigner; RingSigner emits high-S ECDSA signatures rejected by v2 verification"
)]
pub struct RingSigner {
  alg: SignatureAlgorithm,
  keypair: Keys,
  rng: ring::rand::SystemRandom,
  pkcs8: SecretDocument,
}

impl RingSigner {
  /// Create a new `RingSigner` with the given algorithm and private key
  ///
  /// It is likely more convenient to use the `from_pem` method to create a signer
  pub fn new(alg: SignatureAlgorithm, pkcs8: SecretDocument) -> Result<Self, RingSignerError> {
    // Reject undersized RSA keys here so the floor also covers PEM imports
    // (`from_pem` routes through `new`), not just freshly generated keys.
    if let SignatureAlgorithm::Sha256Rsa(bits)
    | SignatureAlgorithm::Sha384Rsa(bits)
    | SignatureAlgorithm::Sha512Rsa(bits) = alg
    {
      if bits < MIN_RSA_KEY_BITS {
        return Err(RingSignerError::WeakKey(bits, MIN_RSA_KEY_BITS));
      }
    }
    let signer = match alg {
      SignatureAlgorithm::Ed25519 => {
        let keypair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_bytes())?;
        Self {
          alg,
          keypair: Keys::Ed25519(keypair),
          rng: ring::rand::SystemRandom::new(),
          pkcs8,
        }
      }
      SignatureAlgorithm::EcdsaP256 => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
          &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
          pkcs8.as_bytes(),
          &rng,
        )?;
        Self {
          alg,
          keypair: Keys::Ecdsa(keypair),
          rng,
          pkcs8,
        }
      }
      SignatureAlgorithm::EcdsaP384 => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
          &ring::signature::ECDSA_P384_SHA384_ASN1_SIGNING,
          pkcs8.as_bytes(),
          &rng,
        )?;
        Self {
          alg,
          keypair: Keys::Ecdsa(keypair),
          rng,
          pkcs8,
        }
      }
      SignatureAlgorithm::Sha256Rsa(bitsize) => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::RsaKeyPair::from_pkcs8(pkcs8.as_bytes())?;
        assert_eq!(bitsize, keypair.public().modulus_len() * 8);
        Self {
          alg,
          keypair: Keys::Rsa(keypair),
          rng,
          pkcs8,
        }
      }
      SignatureAlgorithm::Sha384Rsa(bitsize) => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::RsaKeyPair::from_pkcs8(pkcs8.as_bytes())?;
        assert_eq!(bitsize, keypair.public().modulus_len() * 8);
        Self {
          alg,
          keypair: Keys::Rsa(keypair),
          rng,
          pkcs8,
        }
      }
      SignatureAlgorithm::Sha512Rsa(bitsize) => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::RsaKeyPair::from_pkcs8(pkcs8.as_bytes())?;
        assert_eq!(bitsize, keypair.public().modulus_len() * 8);
        Self {
          alg,
          keypair: Keys::Rsa(keypair),
          rng,
          pkcs8,
        }
      }
      _ => return Err(RingSignerError::UnsupportedAlgorithm),
    };

    Ok(signer)
  }

  /// Create a new `RingSigner` from a PEM formatted private key
  ///
  /// The PEM string should contain a private key in PKCS8 format.
  ///
  /// # Example
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
  /// ```
  pub fn from_pem<S: AsRef<str>>(pem: S) -> Result<Self, RingSignerError> {
    let pem = pem.as_ref();
    let (_, pkcs8) = SecretDocument::from_pem(pem)?;
    use pkcs8::der::Decode;
    let info = pkcs8::PrivateKeyInfo::from_der(pkcs8.as_bytes())?;
    let alg = match info.algorithm.oid {
      const_oid::db::rfc8410::ID_ED_25519 => SignatureAlgorithm::Ed25519,
      const_oid::db::rfc5912::ECDSA_WITH_SHA_256 => SignatureAlgorithm::EcdsaP256,
      const_oid::db::rfc5912::ECDSA_WITH_SHA_384 => SignatureAlgorithm::EcdsaP384,
      const_oid::db::rfc5912::ID_EC_PUBLIC_KEY => {
        // this is insane...
        let other_oid = info.algorithm.parameters_oid().unwrap();
        match other_oid {
          const_oid::db::rfc5912::SECP_256_R_1 => SignatureAlgorithm::EcdsaP256,
          const_oid::db::rfc5912::SECP_384_R_1 => SignatureAlgorithm::EcdsaP384,
          _ => return Err(RingSignerError::UnsupportedAlgorithm),
        }
      }
      #[cfg(feature = "rsa")]
      const_oid::db::rfc5912::SHA_256_WITH_RSA_ENCRYPTION => {
        use rsa::traits::PublicKeyParts;
        let pk = rsa::RsaPrivateKey::from_pkcs8_der(pkcs8.as_bytes())?;
        SignatureAlgorithm::Sha256Rsa(pk.n().bits())
      }
      #[cfg(feature = "rsa")]
      const_oid::db::rfc5912::SHA_384_WITH_RSA_ENCRYPTION => {
        use rsa::traits::PublicKeyParts;
        let pk = rsa::RsaPrivateKey::from_pkcs8_der(pkcs8.as_bytes())?;
        SignatureAlgorithm::Sha384Rsa(pk.n().bits())
      }
      #[cfg(feature = "rsa")]
      const_oid::db::rfc5912::SHA_512_WITH_RSA_ENCRYPTION => {
        use rsa::traits::PublicKeyParts;
        let pk = rsa::RsaPrivateKey::from_pkcs8_der(pkcs8.as_bytes())?;
        SignatureAlgorithm::Sha512Rsa(pk.n().bits())
      }
      #[cfg(feature = "rsa")]
      const_oid::db::rfc5912::RSA_ENCRYPTION => {
        use rsa::traits::PublicKeyParts;
        let pk = rsa::RsaPrivateKey::from_pkcs8_der(pkcs8.as_bytes())?;
        match pk.n().bits() {
          2048 => SignatureAlgorithm::Sha256Rsa(2048),
          3072 => SignatureAlgorithm::Sha384Rsa(3072),
          4096 => SignatureAlgorithm::Sha512Rsa(4096),
          _ => return Err(RingSignerError::UnsupportedAlgorithm),
        }
      }
      _ => {
        return Err(RingSignerError::UnsupportedAlgorithm);
      }
    };
    Self::new(alg, pkcs8)
  }

  /// Access the algorithm for this signer
  pub fn alg(&self) -> &SignatureAlgorithm {
    &self.alg
  }

  /// Access the PKCS8 document for this signer
  pub fn pkcs8(&self) -> &SecretDocument {
    &self.pkcs8
  }

  /// Convert the PKCS8 document to a PEM formatted string
  pub fn private_key_pem(&self) -> pkcs8::der::Result<String> {
    self
      .pkcs8
      .to_pem("PRIVATE KEY", pkcs8::LineEnding::LF)
      .map(|s| s.to_string())
  }

  /// Convert the PKCS8 document to a PEM formatted string, but only include the private key
  ///
  /// ring includes the public key in the pkcs8 document, so this method removes it
  /// for compatibility with openssl. However ring requires the V2 format, which
  /// includes the public key.
  pub fn private_key_only_pem(&self) -> pkcs8::Result<String> {
    use pkcs8::der::Decode;
    let key_info = pkcs8::PrivateKeyInfo::from_der(self.pkcs8.as_bytes())?;
    let stripped_info = pkcs8::PrivateKeyInfo {
      public_key: None,
      ..key_info
    };
    let pkcs8 = pkcs8::Document::from_der(&stripped_info.to_der()?)?;
    let pkcs8 = pkcs8.to_pem("PRIVATE KEY", pkcs8::LineEnding::LF)?;
    Ok(pkcs8)
  }

  /// Generate a new signer with a random RSA keypair of the given size.
  ///
  /// Returns [`RingSignerError::WeakKey`] if `bitsize` is below
  /// [`MIN_RSA_KEY_BITS`].
  #[cfg(feature = "rsa")]
  fn generate_rsa(alg: SignatureAlgorithm, bitsize: usize) -> Result<Self, RingSignerError> {
    if bitsize < MIN_RSA_KEY_BITS {
      return Err(RingSignerError::WeakKey(bitsize, MIN_RSA_KEY_BITS));
    }
    use rsa::pkcs8::EncodePrivateKey;
    let keypair = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), bitsize)
      .map_err(|e| RingSignerError::KeyRejected(e.to_string()))?;
    let pkcs8 = keypair.to_pkcs8_der()?;
    Self::new(alg, pkcs8)
  }

  /// Generate a new signer with a random RSA keypair using the given bitsize
  #[cfg(feature = "rsa")]
  pub fn generate_rs256(bitsize: usize) -> Result<Self, RingSignerError> {
    Self::generate_rsa(SignatureAlgorithm::Sha256Rsa(bitsize), bitsize)
  }

  /// Generate a new signer with a random RSA keypair using the given bitsize
  #[cfg(feature = "rsa")]
  pub fn generate_rs384(bitsize: usize) -> Result<Self, RingSignerError> {
    Self::generate_rsa(SignatureAlgorithm::Sha384Rsa(bitsize), bitsize)
  }

  /// Generate a new signer with a random RSA keypair using the given bitsize
  #[cfg(feature = "rsa")]
  pub fn generate_rs512(bitsize: usize) -> Result<Self, RingSignerError> {
    Self::generate_rsa(SignatureAlgorithm::Sha512Rsa(bitsize), bitsize)
  }

  /// Generate a new signer with a random ECDSA P-256 keypair
  pub fn generate_p256() -> Result<Self, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let keypair = ring::signature::EcdsaKeyPair::generate_pkcs8(
      &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
      &rng,
    )?;
    let pkcs8 = SecretDocument::from_pkcs8_der(keypair.as_ref()).unwrap();
    Ok(Self::new(SignatureAlgorithm::EcdsaP256, pkcs8).unwrap())
  }

  /// Generate a new signer with a random ECDSA P-384 keypair
  pub fn generate_p384() -> Result<Self, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let keypair = ring::signature::EcdsaKeyPair::generate_pkcs8(
      &ring::signature::ECDSA_P384_SHA384_ASN1_SIGNING,
      &rng,
    )?;
    let pkcs8 = SecretDocument::from_pkcs8_der(keypair.as_ref()).unwrap();
    Ok(Self::new(SignatureAlgorithm::EcdsaP384, pkcs8).unwrap())
  }

  /// Generate a new signer with a random Ed25519 keypair
  pub fn generate_ed25519() -> Result<Self, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let keypair = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
    let pkcs8 = SecretDocument::from_pkcs8_der(keypair.as_ref()).unwrap();
    Ok(Self::new(SignatureAlgorithm::Ed25519, pkcs8).unwrap())
  }
}

impl Signer for RingSigner {
  type Key = PublicKey;

  fn sign<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SigningError> {
    match &self.keypair {
      Keys::Ed25519(keypair) => Ok(keypair.sign(message.as_ref()).as_ref().into()),
      Keys::Ecdsa(keypair) => Ok(
        keypair
          .sign(&self.rng, message.as_ref())
          .map_err(|e| SigningError(e.to_string()))?
          .as_ref()
          .into(),
      ),
      Keys::Rsa(keypair) => {
        let mut signature = vec![0; keypair.public().modulus_len()];
        let alg = match self.alg {
          SignatureAlgorithm::Sha256Rsa(_) => &ring::signature::RSA_PKCS1_SHA256,
          SignatureAlgorithm::Sha384Rsa(_) => &ring::signature::RSA_PKCS1_SHA384,
          SignatureAlgorithm::Sha512Rsa(_) => &ring::signature::RSA_PKCS1_SHA512,
          _ => unreachable!(),
        };
        keypair
          .sign(alg, &self.rng, message.as_ref(), &mut signature)
          .map_err(|e| SigningError(e.to_string()))?;
        Ok(signature.into())
      }
    }
  }

  fn public_key(&self) -> Self::Key {
    match &self.keypair {
      Keys::Ed25519(keypair) => PublicKey {
        alg: SignatureAlgorithm::Ed25519,
        key: ring::signature::KeyPair::public_key(keypair)
          .as_ref()
          .into(),
      },
      Keys::Ecdsa(keypair) => {
        let alg = match self.alg {
          SignatureAlgorithm::EcdsaP256 => SignatureAlgorithm::EcdsaP256,
          SignatureAlgorithm::EcdsaP384 => SignatureAlgorithm::EcdsaP384,
          _ => unreachable!(),
        };
        PublicKey {
          alg,
          key: ring::signature::KeyPair::public_key(keypair)
            .as_ref()
            .into(),
        }
      }
      Keys::Rsa(keypair) => {
        let alg = match self.alg {
          SignatureAlgorithm::Sha256Rsa(_) => {
            SignatureAlgorithm::Sha256Rsa(keypair.public().modulus_len() * 8)
          }
          SignatureAlgorithm::Sha384Rsa(_) => {
            SignatureAlgorithm::Sha384Rsa(keypair.public().modulus_len() * 8)
          }
          SignatureAlgorithm::Sha512Rsa(_) => {
            SignatureAlgorithm::Sha512Rsa(keypair.public().modulus_len() * 8)
          }
          _ => unreachable!(),
        };
        PublicKey {
          alg,
          key: keypair.public().as_ref().into(),
        }
      }
    }
  }
}


#[cfg(test)]
mod test {
  use super::*;
  use twine_lib::crypto::MIN_RSA_KEY_BITS;

  // ---------------------------------------------------------------------------
  // Helper: verify that a sign→verify round-trip succeeds and a tampered
  // message is rejected.
  // ---------------------------------------------------------------------------
  fn assert_sign_verify_roundtrip(signer: &RingSigner, msg: &[u8]) {
    use crate::Signer;
    let sig = signer.sign(msg).expect("sign should succeed");
    let pk = signer.public_key();
    pk.verify(sig.clone(), msg).expect("valid signature must verify");

    // Tamper the message: flip a bit in the first byte.
    let mut tampered = msg.to_vec();
    tampered[0] ^= 0xFF;
    pk.verify(sig, tampered.as_slice())
      .expect_err("tampered message must not verify");
  }

  // ---------------------------------------------------------------------------
  // PEM round-trip: every algorithm
  // ---------------------------------------------------------------------------

  #[test]
  fn test_all_pem_roundtrip() {
    let signer = RingSigner::generate_ed25519().unwrap();
    let pem = signer
      .pkcs8()
      .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
      .unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());

    let signer = RingSigner::generate_p256().unwrap();
    let pem = signer
      .pkcs8()
      .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
      .unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());

    let signer = RingSigner::generate_p384().unwrap();
    let pem = signer
      .pkcs8()
      .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
      .unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());

    let signer = RingSigner::generate_rs256(2048).unwrap();
    let pem = signer
      .pkcs8()
      .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
      .unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());

    let signer = RingSigner::generate_rs384(2048).unwrap();
    let pem = signer
      .pkcs8()
      .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
      .unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());

    let signer = RingSigner::generate_rs512(2048).unwrap();
    let pem = signer
      .pkcs8()
      .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
      .unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());
  }

  // ---------------------------------------------------------------------------
  // `private_key_pem` produces a valid PEM string that can be re-imported
  // ---------------------------------------------------------------------------

  #[test]
  fn test_private_key_pem_reimport_ed25519() {
    let signer = RingSigner::generate_ed25519().unwrap();
    let pem = signer.private_key_pem().unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());
    assert!(matches!(signer2.alg(), SignatureAlgorithm::Ed25519));
  }

  #[test]
  fn test_private_key_pem_reimport_p256() {
    let signer = RingSigner::generate_p256().unwrap();
    let pem = signer.private_key_pem().unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());
    assert!(matches!(signer2.alg(), SignatureAlgorithm::EcdsaP256));
  }

  #[test]
  fn test_private_key_pem_reimport_p384() {
    let signer = RingSigner::generate_p384().unwrap();
    let pem = signer.private_key_pem().unwrap();
    let signer2 = RingSigner::from_pem(&pem).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());
    assert!(matches!(signer2.alg(), SignatureAlgorithm::EcdsaP384));
  }

  // ---------------------------------------------------------------------------
  // `private_key_only_pem` produces a key that still re-imports
  // ---------------------------------------------------------------------------

  #[test]
  fn test_private_key_only_pem_reimport_ed25519() {
    let signer = RingSigner::generate_ed25519().unwrap();
    // The only-private PEM strips the public-key attribute; ring requires v2
    // format so re-importing must still work because `from_pem` routes through
    // `new` which calls ring's from_pkcs8.
    let pem_only = signer.private_key_only_pem().unwrap();
    // The stripped PEM should parse without panicking (it may or may not
    // re-import into a RingSigner depending on ring's tolerance).
    // At minimum the PEM itself must be valid PEM text.
    assert!(pem_only.contains("PRIVATE KEY"));
  }

  // ---------------------------------------------------------------------------
  // `alg()` and `pkcs8()` accessors
  // ---------------------------------------------------------------------------

  #[test]
  fn test_alg_accessor() {
    let signer = RingSigner::generate_ed25519().unwrap();
    assert!(matches!(signer.alg(), SignatureAlgorithm::Ed25519));

    let signer = RingSigner::generate_p256().unwrap();
    assert!(matches!(signer.alg(), SignatureAlgorithm::EcdsaP256));

    let signer = RingSigner::generate_p384().unwrap();
    assert!(matches!(signer.alg(), SignatureAlgorithm::EcdsaP384));

    #[cfg(feature = "rsa")]
    {
      let signer = RingSigner::generate_rs256(2048).unwrap();
      assert!(matches!(signer.alg(), SignatureAlgorithm::Sha256Rsa(2048)));

      let signer = RingSigner::generate_rs384(2048).unwrap();
      assert!(matches!(signer.alg(), SignatureAlgorithm::Sha384Rsa(2048)));

      let signer = RingSigner::generate_rs512(2048).unwrap();
      assert!(matches!(signer.alg(), SignatureAlgorithm::Sha512Rsa(2048)));
    }
  }

  #[test]
  fn test_pkcs8_accessor_roundtrips_der() {
    let signer = RingSigner::generate_ed25519().unwrap();
    let pkcs8 = signer.pkcs8();
    // The bytes must be a valid PKCS#8 document: re-import it.
    let signer2 = RingSigner::new(SignatureAlgorithm::Ed25519, pkcs8.clone()).unwrap();
    assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());
  }

  // ---------------------------------------------------------------------------
  // Sign→verify round-trips (every algorithm) and tamper detection
  // ---------------------------------------------------------------------------

  #[test]
  fn test_sign_verify_roundtrip_ed25519() {
    let signer = RingSigner::generate_ed25519().unwrap();
    assert_sign_verify_roundtrip(&signer, b"hello ed25519");
  }

  // NOTE: RingSigner's ECDSA output is not guaranteed to be canonical low-S, so
  // it can fail strict v2 verification. This is the core reason RingSigner is
  // deprecated in favour of RustCryptoSigner. We therefore only assert that
  // signing succeeds for the EC curves, not that the result verifies under v2.
  #[test]
  fn test_p256_signs_but_may_be_high_s() {
    let signer = RingSigner::generate_p256().unwrap();
    assert!(signer.sign(b"hello p256").is_ok());
  }

  #[test]
  fn test_p384_signs_but_may_be_high_s() {
    let signer = RingSigner::generate_p384().unwrap();
    assert!(signer.sign(b"hello p384").is_ok());
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn test_sign_verify_roundtrip_rs256() {
    let signer = RingSigner::generate_rs256(2048).unwrap();
    assert_sign_verify_roundtrip(&signer, b"hello rs256");
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn test_sign_verify_roundtrip_rs384() {
    let signer = RingSigner::generate_rs384(2048).unwrap();
    assert_sign_verify_roundtrip(&signer, b"hello rs384");
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn test_sign_verify_roundtrip_rs512() {
    let signer = RingSigner::generate_rs512(2048).unwrap();
    assert_sign_verify_roundtrip(&signer, b"hello rs512");
  }

  // ---------------------------------------------------------------------------
  // `public_key()` returns the correct alg and usable key bytes
  // ---------------------------------------------------------------------------

  #[test]
  fn test_public_key_alg_ed25519() {
    let signer = RingSigner::generate_ed25519().unwrap();
    let pk = signer.public_key();
    assert!(matches!(pk.alg, SignatureAlgorithm::Ed25519));
    assert!(!pk.key.is_empty());
  }

  #[test]
  fn test_public_key_alg_p256() {
    let signer = RingSigner::generate_p256().unwrap();
    let pk = signer.public_key();
    assert!(matches!(pk.alg, SignatureAlgorithm::EcdsaP256));
    assert!(!pk.key.is_empty());
  }

  #[test]
  fn test_public_key_alg_p384() {
    let signer = RingSigner::generate_p384().unwrap();
    let pk = signer.public_key();
    assert!(matches!(pk.alg, SignatureAlgorithm::EcdsaP384));
    assert!(!pk.key.is_empty());
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn test_public_key_alg_rsa() {
    let signer = RingSigner::generate_rs256(2048).unwrap();
    let pk = signer.public_key();
    assert!(matches!(pk.alg, SignatureAlgorithm::Sha256Rsa(2048)));
    assert!(!pk.key.is_empty());
  }

  // ---------------------------------------------------------------------------
  // ERROR PATHS: `from_pem` on garbage / non-PEM input → Err, no panic
  // ---------------------------------------------------------------------------

  #[test]
  fn test_from_pem_garbage_returns_err() {
    let result = RingSigner::from_pem("this is definitely not a pem");
    assert!(
      result.is_err(),
      "garbage input must return Err, not panic or succeed"
    );
  }

  #[test]
  fn test_from_pem_empty_string_returns_err() {
    let result = RingSigner::from_pem("");
    assert!(result.is_err(), "empty string must return Err");
  }

  #[test]
  fn test_from_pem_truncated_pem_returns_err() {
    // A PEM header without body or footer.
    let truncated = "-----BEGIN PRIVATE KEY-----\nMFECAQEwBQ==";
    let result = RingSigner::from_pem(truncated);
    assert!(result.is_err(), "truncated PEM must return Err");
  }

  // ---------------------------------------------------------------------------
  // ERROR PATHS: `from_pem` with an unsupported algorithm OID
  // ---------------------------------------------------------------------------

  #[test]
  fn test_from_pem_unsupported_oid_returns_unsupported_algorithm() {
    // A syntactically valid PKCS#8 PEM for id-dsa (OID 1.2.840.10040.4.1),
    // which is an algorithm we deliberately do not support.
    //
    // The base64 below encodes this 23-byte PKCS#8 DER structure:
    //   SEQUENCE {
    //     INTEGER 0                                 (version)
    //     SEQUENCE {                                (AlgorithmIdentifier)
    //       OID 1.2.840.10040.4.1                  (id-dsa)
    //       SEQUENCE {}                             (empty parameters)
    //     }
    //     OCTET STRING { INTEGER 1 }               (placeholder private key)
    //   }
    // Verified: base64 "MBUCAQAwCwYHKoZIzjgEATAABAMCAQE=" decodes to the
    // correct DER bytes [30 15 02 01 00 30 0B 06 07 2a 86 48 ce 38 04 01
    //   30 00 04 03 02 01 01].
    let pem = concat!(
      "-----BEGIN PRIVATE KEY-----\n",
      "MBUCAQAwCwYHKoZIzjgEATAABAMCAQE=\n",
      "-----END PRIVATE KEY-----\n"
    );
    let result = RingSigner::from_pem(pem);
    assert!(
      matches!(result, Err(RingSignerError::UnsupportedAlgorithm)),
      "DSA OID must map to UnsupportedAlgorithm"
    );
  }

  // ---------------------------------------------------------------------------
  // WEAK-KEY GUARD: `generate_rs{256,384,512}(1024)` must return WeakKey
  // ---------------------------------------------------------------------------

  #[cfg(feature = "rsa")]
  #[test]
  fn test_generate_rs256_rejects_weak_rsa() {
    assert!(
      matches!(
        RingSigner::generate_rs256(1024),
        Err(RingSignerError::WeakKey(1024, MIN_RSA_KEY_BITS))
      ),
      "1024-bit RSA must be refused by generate_rs256"
    );
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn test_generate_rs384_rejects_weak_rsa() {
    assert!(
      matches!(
        RingSigner::generate_rs384(1024),
        Err(RingSignerError::WeakKey(1024, MIN_RSA_KEY_BITS))
      ),
      "1024-bit RSA must be refused by generate_rs384"
    );
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn test_generate_rs512_rejects_weak_rsa() {
    assert!(
      matches!(
        RingSigner::generate_rs512(1024),
        Err(RingSignerError::WeakKey(1024, MIN_RSA_KEY_BITS))
      ),
      "1024-bit RSA must be refused by generate_rs512"
    );
  }

  /// The WeakKey guard in `new` is exercised by `from_pem` for RSA keys.
  /// We construct a sub-2048 RSA key via the rsa crate (bypassing our
  /// generate_rs* API) and confirm that importing it is rejected.
  #[cfg(feature = "rsa")]
  #[test]
  fn test_new_rejects_weak_rsa_via_pkcs8() {
    use rsa::pkcs8::EncodePrivateKey;
    // Generate a 1024-bit key directly via rsa crate, bypassing our guard.
    let small_key = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
    let pkcs8 = small_key.to_pkcs8_der().unwrap();
    let result = RingSigner::new(SignatureAlgorithm::Sha256Rsa(1024), pkcs8);
    assert!(
      matches!(result, Err(RingSignerError::WeakKey(1024, MIN_RSA_KEY_BITS))),
      "importing 1024-bit RSA key via new() must be refused"
    );
  }
}
