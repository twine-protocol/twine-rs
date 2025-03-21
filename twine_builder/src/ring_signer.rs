use pkcs8::{der::Encode, DecodePrivateKey, SecretDocument};
use std::vec;
use thiserror::Error;
use twine_lib::crypto::{PublicKey, Signature, SignatureAlgorithm};

use crate::{Signer, SigningError};

#[derive(Debug, Error)]
pub enum RingSignerError {
  #[error("Unsupported algorithm")]
  UnsupportedAlgorithm,
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

/// A signer that uses the `ring` crate to sign data
///
/// This is a v2 signer, and is intended to be used with twine/2.0.0.
///
/// # Example
///
/// ```rust
/// use twine_builder::{RingSigner, Signer};
/// let signer = RingSigner::generate_ed25519().unwrap();
/// let pem = signer
///   .pkcs8()
///   .to_pem("PRIVATE_KEY", pkcs8::LineEnding::LF)
///   .unwrap();
/// let signer2 = RingSigner::from_pem(&pem).unwrap();
/// assert_eq!(signer.pkcs8().as_bytes(), signer2.pkcs8().as_bytes());
/// ```
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

  /// Generate a new signer with a random RSA keypair using the given bitsize
  #[cfg(feature = "rsa")]
  pub fn generate_rs256(bitsize: usize) -> rsa::Result<Self> {
    let keypair = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), bitsize)?;
    use rsa::pkcs8::EncodePrivateKey;
    let pkcs8 = keypair.to_pkcs8_der()?;
    Ok(Self::new(SignatureAlgorithm::Sha256Rsa(bitsize), pkcs8).unwrap())
  }

  /// Generate a new signer with a random RSA keypair using the given bitsize
  #[cfg(feature = "rsa")]
  pub fn generate_rs384(bitsize: usize) -> rsa::Result<Self> {
    let keypair = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), bitsize)?;
    use rsa::pkcs8::EncodePrivateKey;
    let pkcs8 = keypair.to_pkcs8_der()?;
    Ok(Self::new(SignatureAlgorithm::Sha384Rsa(bitsize), pkcs8).unwrap())
  }

  /// Generate a new signer with a random RSA keypair using the given bitsize
  #[cfg(feature = "rsa")]
  pub fn generate_rs512(bitsize: usize) -> rsa::Result<Self> {
    let keypair = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), bitsize)?;
    use rsa::pkcs8::EncodePrivateKey;
    let pkcs8 = keypair.to_pkcs8_der()?;
    Ok(Self::new(SignatureAlgorithm::Sha512Rsa(bitsize), pkcs8).unwrap())
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
}
