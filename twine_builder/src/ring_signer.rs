use std::vec;
use twine_core::crypto::{PublicKey, Signature, SignatureAlgorithm};

use crate::{Signer, SigningError};

enum Keys {
  Ed25519(ring::signature::Ed25519KeyPair),
  Ecdsa(ring::signature::EcdsaKeyPair),
  Rsa(ring::signature::RsaKeyPair),
}

pub struct RingSigner {
  alg: SignatureAlgorithm,
  keypair: Keys,
  rng: ring::rand::SystemRandom,
  pkcs8: Vec<u8>,
}

impl RingSigner {
  pub fn new<T: AsRef<[u8]>>(alg: SignatureAlgorithm, pkcs8: T) -> Result<Self, ring::error::KeyRejected> {
    let signer = match alg {
      SignatureAlgorithm::ED25519 => {
        let keypair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())?;
        Self {
          alg,
          keypair: Keys::Ed25519(keypair),
          rng: ring::rand::SystemRandom::new(),
          pkcs8: pkcs8.as_ref().to_vec(),
        }
      },
      SignatureAlgorithm::EcdsaP256 => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
          &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
          pkcs8.as_ref(),
          &rng
        )?;
        Self {
          alg,
          keypair: Keys::Ecdsa(keypair),
          rng,
          pkcs8: pkcs8.as_ref().to_vec(),
        }
      },
      SignatureAlgorithm::EcdsaP384 => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
          &ring::signature::ECDSA_P384_SHA384_ASN1_SIGNING,
          pkcs8.as_ref(),
          &rng
        )?;
        Self {
          alg,
          keypair: Keys::Ecdsa(keypair),
          rng,
          pkcs8: pkcs8.as_ref().to_vec(),
        }
      },
      SignatureAlgorithm::Sha256Rsa(bitsize) => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::RsaKeyPair::from_pkcs8(
          pkcs8.as_ref(),
        )?;
        assert!(bitsize == keypair.public().modulus_len());
        Self {
          alg,
          keypair: Keys::Rsa(keypair),
          rng,
          pkcs8: pkcs8.as_ref().to_vec(),
        }
      },
      SignatureAlgorithm::Sha384Rsa(bitsize) => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::RsaKeyPair::from_pkcs8(
          pkcs8.as_ref(),
        )?;
        assert!(bitsize == keypair.public().modulus_len());
        Self {
          alg,
          keypair: Keys::Rsa(keypair),
          rng,
          pkcs8: pkcs8.as_ref().to_vec(),
        }
      },
      SignatureAlgorithm::Sha512Rsa(bitsize) => {
        let rng = ring::rand::SystemRandom::new();
        let keypair = ring::signature::RsaKeyPair::from_pkcs8(
          pkcs8.as_ref(),
        )?;
        assert!(bitsize == keypair.public().modulus_len());
        Self {
          alg,
          keypair: Keys::Rsa(keypair),
          rng,
          pkcs8: pkcs8.as_ref().to_vec(),
        }
      },
      _ => panic!("Unsupported algorithm")
    };

    Ok(signer)
  }

  pub fn pkcs8(&self) -> &[u8] {
    &self.pkcs8
  }

  #[cfg(feature = "rsa")]
  pub fn generate_rs256(bitsize: usize) -> rsa::Result<Self> {
    let keypair = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), bitsize)?;
    use rsa::pkcs8::EncodePrivateKey;
    let pkcs8 = keypair.to_pkcs8_der()?;
    Ok(Self::new(SignatureAlgorithm::Sha256Rsa(bitsize), pkcs8.as_bytes()).unwrap())
  }

  #[cfg(feature = "rsa")]
  pub fn generate_rs384(bitsize: usize) -> rsa::Result<Self> {
    let keypair = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), bitsize)?;
    use rsa::pkcs8::EncodePrivateKey;
    let pkcs8 = keypair.to_pkcs8_der()?;
    Ok(Self::new(SignatureAlgorithm::Sha384Rsa(bitsize), pkcs8.as_bytes()).unwrap())
  }

  pub fn generate_p256() -> Result<Self, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let keypair = ring::signature::EcdsaKeyPair::generate_pkcs8(
      &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
      &rng
    )?;
    Ok(Self::new(SignatureAlgorithm::EcdsaP256, keypair.as_ref()).unwrap())
  }

  pub fn generate_p384() -> Result<Self, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let keypair = ring::signature::EcdsaKeyPair::generate_pkcs8(
      &ring::signature::ECDSA_P384_SHA384_ASN1_SIGNING,
      &rng
    )?;
    Ok(Self::new(SignatureAlgorithm::EcdsaP384, keypair.as_ref()).unwrap())
  }

  pub fn generate_ed25519() -> Result<Self, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let keypair = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
    Ok(Self::new(SignatureAlgorithm::ED25519, keypair.as_ref()).unwrap())
  }
}

impl Signer for RingSigner {
  type Key = PublicKey;

  fn sign<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SigningError> {
    match &self.keypair {
      Keys::Ed25519(keypair) => {
        Ok(keypair.sign(message.as_ref()).as_ref().into())
      },
      Keys::Ecdsa(keypair) => {
        Ok(keypair.sign(&self.rng, message.as_ref())
          .map_err(|e| SigningError(e.to_string()))?
          .as_ref().into())
      },
      Keys::Rsa(keypair) => {
        let mut signature = vec![0; keypair.public().modulus_len()];
        let alg = match self.alg {
          SignatureAlgorithm::Sha256Rsa(_) => &ring::signature::RSA_PKCS1_SHA256,
          SignatureAlgorithm::Sha384Rsa(_) => &ring::signature::RSA_PKCS1_SHA384,
          SignatureAlgorithm::Sha512Rsa(_) => &ring::signature::RSA_PKCS1_SHA512,
          _ => unreachable!(),
        };
        keypair.sign(alg, &self.rng, message.as_ref(), &mut signature)
          .map_err(|e| SigningError(e.to_string()))?;
        Ok(signature.into())
      },
    }
  }

  fn public_key(&self) -> Self::Key {
    match &self.keypair {
      Keys::Ed25519(keypair) => {
        PublicKey {
          alg: SignatureAlgorithm::ED25519,
          key: ring::signature::KeyPair::public_key(keypair).as_ref().into(),
        }
      },
      Keys::Ecdsa(keypair) => {
        let alg = match self.alg {
          SignatureAlgorithm::EcdsaP256 => SignatureAlgorithm::EcdsaP256,
          SignatureAlgorithm::EcdsaP384 => SignatureAlgorithm::EcdsaP384,
          _ => unreachable!(),
        };
        PublicKey {
          alg,
          key: ring::signature::KeyPair::public_key(keypair).as_ref().into(),
        }
      },
      Keys::Rsa(keypair) => {
        let alg = match self.alg {
          SignatureAlgorithm::Sha256Rsa(_) => SignatureAlgorithm::Sha256Rsa(keypair.public().modulus_len()),
          SignatureAlgorithm::Sha384Rsa(_) => SignatureAlgorithm::Sha384Rsa(keypair.public().modulus_len()),
          SignatureAlgorithm::Sha512Rsa(_) => SignatureAlgorithm::Sha512Rsa(keypair.public().modulus_len()),
          _ => unreachable!(),
        };
        PublicKey {
          alg,
          key: keypair.public().as_ref().into(),
        }
      },
    }
  }
}
