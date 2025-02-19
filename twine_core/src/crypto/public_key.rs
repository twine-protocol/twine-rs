use std::{fmt::Display, str::FromStr};
use biscuit::jwk::JWK;
use serde::{Deserialize, Serialize};
use crate::{errors::VerificationError, Bytes};
use super::Signature;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicKey {
  #[serde(rename = "a")]
  pub alg: SignatureAlgorithm,
  /// ASN.1 DER encoded public key
  #[serde(rename = "k")]
  pub key: Bytes,
}

impl PublicKey {
  pub fn new(alg: SignatureAlgorithm, key: Bytes) -> Self {
    Self { alg, key }
  }

  pub fn verify<D: AsRef<[u8]>>(&self, signature: Signature, message: D) -> Result<(), VerificationError> {
    // Verify the signature
    match self.alg {
      SignatureAlgorithm::Sha256Rsa(_) | SignatureAlgorithm::Sha384Rsa(_) | SignatureAlgorithm::Sha512Rsa(_) => {
        self.verify_rsa(&signature, message.as_ref())
      },
      SignatureAlgorithm::EcdsaP256 | SignatureAlgorithm::EcdsaP384 => {
        self.verify_ecdsa(&signature, message.as_ref())
      },
      SignatureAlgorithm::Ed25519 => {
        self.verify_ed25519(&signature, message.as_ref())
      },
    }
  }

  fn verify_rsa(&self, signature: &Signature, message: &[u8]) -> Result<(), VerificationError> {
    // Verify the RSA signature
    let alg = match self.alg {
      SignatureAlgorithm::Sha256Rsa(bitsize) => match bitsize {
        2048 => &ring::signature::RSA_PKCS1_2048_8192_SHA256,
        _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
      },
      SignatureAlgorithm::Sha384Rsa(bitsize) => match bitsize {
        2048 => &ring::signature::RSA_PKCS1_2048_8192_SHA384,
        3072 => &ring::signature::RSA_PKCS1_3072_8192_SHA384,
        _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
      },
      SignatureAlgorithm::Sha512Rsa(bitsize) => match bitsize {
        2048 => &ring::signature::RSA_PKCS1_2048_8192_SHA512,
        _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
      },
      _ => unreachable!(),
    };

    let public_key = ring::signature::UnparsedPublicKey::new(alg, &self.key);
    public_key.verify(message, signature).map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    Ok(())
  }

  fn verify_ecdsa(&self, signature: &Signature, message: &[u8]) -> Result<(), VerificationError> {
    let alg = match self.alg {
      SignatureAlgorithm::EcdsaP256 => &ring::signature::ECDSA_P256_SHA256_ASN1,
      SignatureAlgorithm::EcdsaP384 => &ring::signature::ECDSA_P384_SHA384_ASN1,
      _ => unreachable!(),
    };

    let public_key = ring::signature::UnparsedPublicKey::new(alg, &self.key);
    public_key.verify(message, signature).map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    Ok(())
  }

  fn verify_ed25519(&self, signature: &Signature, message: &[u8]) -> Result<(), VerificationError> {
    let public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, &self.key);
    public_key.verify(message, signature).map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    Ok(())
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
        let pk = rsa::RsaPublicKey::new(rsa::BigUint::from_bytes_be(&rsa.n.to_bytes_be()), rsa::BigUint::from_bytes_be(&rsa.e.to_bytes_be())).unwrap();
        pk.to_pkcs1_der().unwrap()
      },
      biscuit::jwk::AlgorithmParameters::EllipticCurve(ec) => {
        use elliptic_curve::pkcs8::EncodePublicKey;
        let sec1 = match ec.jws_public_key_secret() {
          biscuit::jws::Secret::PublicKey(b) => b,
          _ => unimplemented!(),
        };
        match alg {
          SignatureAlgorithm::EcdsaP256 => {
            p256::PublicKey::from_sec1_bytes(&sec1).unwrap().to_public_key_der().unwrap()
          },
          SignatureAlgorithm::EcdsaP384 => {
            p384::PublicKey::from_sec1_bytes(&sec1).unwrap().to_public_key_der().unwrap()
          },
          _ => unimplemented!(),
        }
      },
      _ => unimplemented!(),
    };

    Self { alg, key: key.as_bytes().into() }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use ring::signature::KeyPair;

  #[test]
  fn test_signature_ed25519_roundtrip() {
    // Generate a key pair in PKCS#8 (v2) format.
    let rng = ring::rand::SystemRandom::new();
    let pkcs8_bytes = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key_pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).unwrap();

    // Sign the message "hello, world".
    const MESSAGE: &[u8] = b"hello, world";
    let sig = key_pair.sign(MESSAGE);
    let sig_bytes = sig.as_ref().into();

    let pk = PublicKey::new(SignatureAlgorithm::Ed25519, Bytes::from(key_pair.public_key().as_ref()));
    pk.verify(sig_bytes, MESSAGE).unwrap();
  }
}
