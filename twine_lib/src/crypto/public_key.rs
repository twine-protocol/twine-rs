use super::Signature;
use crate::{errors::VerificationError, Bytes};
use biscuit::jwk::JWK;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

/// Digital signature algorithms used by Twine
#[derive(Debug, Serialize, Deserialize, Clone)]
#[non_exhaustive]
#[serde(rename_all = "UPPERCASE")]
pub enum SignatureAlgorithm {
  /// RSA(bitsize) PKCS1.5 sha256
  Sha256Rsa(usize),
  /// RSA(bitsize) PKCS1.5 sha384
  Sha384Rsa(usize),
  /// RSA(bitsize) PKCS1.5 sha512
  Sha512Rsa(usize),
  /// ECDSA P-256 sha256
  EcdsaP256,
  /// ECDSA P-384 sha384
  EcdsaP384,
  /// Ed25519 sha512
  Ed25519,
}

impl Display for SignatureAlgorithm {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SignatureAlgorithm::Sha256Rsa(bitsize) => write!(f, "RSA {} SHA256", bitsize),
      SignatureAlgorithm::Sha384Rsa(bitsize) => write!(f, "RSA {} SHA384", bitsize),
      SignatureAlgorithm::Sha512Rsa(bitsize) => write!(f, "RSA {} SHA512", bitsize),
      SignatureAlgorithm::EcdsaP256 => write!(f, "ECDSA P-256 SHA256"),
      SignatureAlgorithm::EcdsaP384 => write!(f, "ECDSA P-384 SHA384"),
      SignatureAlgorithm::Ed25519 => write!(f, "Ed25519 SHA512"),
    }
  }
}

impl FromStr for SignatureAlgorithm {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.trim().to_uppercase().as_str() {
      // standard
      "RSA 2048 SHA256" => Ok(SignatureAlgorithm::Sha256Rsa(2048)),
      "RSA 3072 SHA256" => Ok(SignatureAlgorithm::Sha256Rsa(3072)),
      "RSA 4096 SHA256" => Ok(SignatureAlgorithm::Sha256Rsa(4096)),
      "RSA 2048 SHA384" => Ok(SignatureAlgorithm::Sha384Rsa(2048)),
      "RSA 3072 SHA384" => Ok(SignatureAlgorithm::Sha384Rsa(3072)),
      "RSA 4096 SHA384" => Ok(SignatureAlgorithm::Sha384Rsa(4096)),
      "RSA 2048 SHA512" => Ok(SignatureAlgorithm::Sha512Rsa(2048)),
      "RSA 3072 SHA512" => Ok(SignatureAlgorithm::Sha512Rsa(3072)),
      "RSA 4096 SHA512" => Ok(SignatureAlgorithm::Sha512Rsa(4096)),
      "ECDSA P-256 SHA256" => Ok(SignatureAlgorithm::EcdsaP256),
      "ECDSA P-384 SHA384" => Ok(SignatureAlgorithm::EcdsaP384),
      "Ed25519 SHA512" => Ok(SignatureAlgorithm::Ed25519),
      // shorthand
      "RS256" => Ok(SignatureAlgorithm::Sha256Rsa(2048)),
      "RS384" => Ok(SignatureAlgorithm::Sha384Rsa(2048)),
      "RS512" => Ok(SignatureAlgorithm::Sha512Rsa(2048)),
      "ES256" => Ok(SignatureAlgorithm::EcdsaP256),
      "ES384" => Ok(SignatureAlgorithm::EcdsaP384),
      "Ed25519" => Ok(SignatureAlgorithm::Ed25519),
      _ => Err(()),
    }
  }
}

/// A public key used for verifying digital signatures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicKey {
  /// The signature algorithm used by the key
  #[serde(rename = "a")]
  pub alg: SignatureAlgorithm,
  /// ASN.1 DER encoded public key
  #[serde(rename = "k")]
  pub key: Bytes,
}

impl PublicKey {
  /// Create a new public key struct
  pub fn new(alg: SignatureAlgorithm, key: Bytes) -> Self {
    Self { alg, key }
  }

  /// Verify the signature of a message using this public key
  pub fn verify<D: AsRef<[u8]>>(
    &self,
    signature: Signature,
    message: D,
  ) -> Result<(), VerificationError> {
    // Verify the signature
    match self.alg {
      SignatureAlgorithm::Sha256Rsa(_)
      | SignatureAlgorithm::Sha384Rsa(_)
      | SignatureAlgorithm::Sha512Rsa(_) => self.verify_rsa(&signature, message.as_ref()),
      SignatureAlgorithm::EcdsaP256 | SignatureAlgorithm::EcdsaP384 => {
        self.verify_ecdsa(&signature, message.as_ref())
      }
      SignatureAlgorithm::Ed25519 => self.verify_ed25519(&signature, message.as_ref()),
    }
  }

  fn verify_rsa(&self, signature: &Signature, message: &[u8]) -> Result<(), VerificationError> {
    use rsa::pkcs1::DecodeRsaPublicKey;
    use rsa::Pkcs1v15Sign;
    use sha2::{Digest, Sha256, Sha384, Sha512};

    // The stored key is a PKCS#1 RSAPublicKey. Any modulus size verifies, so
    // there is no per-bitsize table to keep in sync — 2048/3072/4096/… all work.
    let public_key = rsa::RsaPublicKey::from_pkcs1_der(&self.key)
      .map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    let (scheme, hashed) = match self.alg {
      SignatureAlgorithm::Sha256Rsa(_) => {
        (Pkcs1v15Sign::new::<Sha256>(), Sha256::digest(message).to_vec())
      }
      SignatureAlgorithm::Sha384Rsa(_) => {
        (Pkcs1v15Sign::new::<Sha384>(), Sha384::digest(message).to_vec())
      }
      SignatureAlgorithm::Sha512Rsa(_) => {
        (Pkcs1v15Sign::new::<Sha512>(), Sha512::digest(message).to_vec())
      }
      _ => unreachable!(),
    };

    public_key
      .verify(scheme, &hashed, signature.as_ref())
      .map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    Ok(())
  }

  fn verify_ecdsa(&self, signature: &Signature, message: &[u8]) -> Result<(), VerificationError> {
    // `VerifyingKey::verify` hashes the message with the curve's associated
    // digest (SHA-256 / SHA-384). The stored key is an uncompressed SEC1 point;
    // the signature is ASN.1 DER.
    use p256::ecdsa::signature::Verifier;
    match self.alg {
      SignatureAlgorithm::EcdsaP256 => {
        use p256::ecdsa::{Signature as EcSignature, VerifyingKey};
        let key = VerifyingKey::from_sec1_bytes(&self.key)
          .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
        let sig = EcSignature::from_der(signature.as_ref())
          .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
        key
          .verify(message, &sig)
          .map_err(|e| VerificationError::BadSignature(e.to_string()))
      }
      SignatureAlgorithm::EcdsaP384 => {
        use p384::ecdsa::{Signature as EcSignature, VerifyingKey};
        let key = VerifyingKey::from_sec1_bytes(&self.key)
          .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
        let sig = EcSignature::from_der(signature.as_ref())
          .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
        key
          .verify(message, &sig)
          .map_err(|e| VerificationError::BadSignature(e.to_string()))
      }
      _ => unreachable!(),
    }
  }

  fn verify_ed25519(&self, signature: &Signature, message: &[u8]) -> Result<(), VerificationError> {
    use ed25519_dalek::{Signature as EdSignature, Verifier, VerifyingKey};

    // The stored key is the raw 32-byte Ed25519 public key.
    let key_bytes: [u8; 32] = self
      .key
      .as_ref()
      .try_into()
      .map_err(|_| VerificationError::BadSignature("invalid ed25519 public key length".into()))?;
    let public_key = VerifyingKey::from_bytes(&key_bytes)
      .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
    let sig = EdSignature::from_slice(signature.as_ref())
      .map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    public_key
      .verify(message, &sig)
      .map_err(|e| VerificationError::BadSignature(e.to_string()))
  }
}

impl From<JWK<()>> for PublicKey {
  fn from(jwk: JWK<()>) -> Self {
    let modulus = match &jwk.algorithm {
      biscuit::jwk::AlgorithmParameters::RSA(rsa) => rsa.n.bits() as usize,
      _ => 0,
    };
    let alg = match &jwk.common.algorithm {
      Some(alg) => match alg {
        biscuit::jwa::Algorithm::Signature(sigalg) => match sigalg {
          biscuit::jwa::SignatureAlgorithm::RS256 => SignatureAlgorithm::Sha256Rsa(modulus),
          biscuit::jwa::SignatureAlgorithm::RS384 => SignatureAlgorithm::Sha384Rsa(modulus),
          biscuit::jwa::SignatureAlgorithm::RS512 => SignatureAlgorithm::Sha512Rsa(modulus),
          biscuit::jwa::SignatureAlgorithm::ES256 => SignatureAlgorithm::EcdsaP256,
          biscuit::jwa::SignatureAlgorithm::ES384 => SignatureAlgorithm::EcdsaP384,
          _ => unimplemented!(),
        },
        _ => unimplemented!(),
      },
      None => unimplemented!(),
    };

    let key = match &jwk.algorithm {
      biscuit::jwk::AlgorithmParameters::RSA(rsa) => {
        use rsa::pkcs1::EncodeRsaPublicKey;
        let pk = rsa::RsaPublicKey::new(
          rsa::BigUint::from_bytes_be(&rsa.n.to_bytes_be()),
          rsa::BigUint::from_bytes_be(&rsa.e.to_bytes_be()),
        )
        .unwrap();
        pk.to_pkcs1_der().unwrap()
      }
      biscuit::jwk::AlgorithmParameters::EllipticCurve(ec) => {
        use elliptic_curve::pkcs8::EncodePublicKey;
        let sec1 = match ec.jws_public_key_secret() {
          biscuit::jws::Secret::PublicKey(b) => b,
          _ => unimplemented!(),
        };
        match alg {
          SignatureAlgorithm::EcdsaP256 => p256::PublicKey::from_sec1_bytes(&sec1)
            .unwrap()
            .to_public_key_der()
            .unwrap(),
          SignatureAlgorithm::EcdsaP384 => p384::PublicKey::from_sec1_bytes(&sec1)
            .unwrap()
            .to_public_key_der()
            .unwrap(),
          _ => unimplemented!(),
        }
      }
      _ => unimplemented!(),
    };

    Self {
      alg,
      key: key.as_bytes().into(),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_signature_ed25519_roundtrip() {
    use ed25519_dalek::{Signer, SigningKey};

    // Deterministic key from a fixed seed so the test needs no RNG dependency.
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);

    const MESSAGE: &[u8] = b"hello, world";
    let sig = signing_key.sign(MESSAGE);

    let pk = PublicKey::new(
      SignatureAlgorithm::Ed25519,
      Bytes::from(signing_key.verifying_key().to_bytes().as_slice()),
    );
    pk.verify(Bytes::from(sig.to_bytes().as_slice()), MESSAGE)
      .unwrap();
  }

  #[test]
  fn test_ed25519_rejects_bad_signature() {
    use ed25519_dalek::{Signer, SigningKey};

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let sig = signing_key.sign(b"hello, world");
    let pk = PublicKey::new(
      SignatureAlgorithm::Ed25519,
      Bytes::from(signing_key.verifying_key().to_bytes().as_slice()),
    );
    // wrong message must fail
    assert!(pk
      .verify(Bytes::from(sig.to_bytes().as_slice()), b"goodbye, world")
      .is_err());
  }
}
