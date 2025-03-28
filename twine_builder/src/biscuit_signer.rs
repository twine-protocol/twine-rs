//! A signer that uses the `biscuit` crate to sign data.
//!
//! Requires the `v1` feature to be enabled.
use crate::{Signer, SigningError};
use biscuit::{
  jwk::{AlgorithmParameters, JWK},
  jws::{Header, Secret},
};
use ring::signature::{EcdsaKeyPair, RsaKeyPair};
use serde_json::json;
use twine_lib::crypto::Signature;

/// A signer that uses the `biscuit` crate to sign data.
///
/// Requires the `v1` feature to be enabled.
///
/// # Deprecated
///
/// This signer is intended to be used with v1 data, which is
/// being phased out. Use `RingSigner` with twine/2.0.0 instead.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use twine_lib::{ipld_core::ipld, multihash_codetable::Code};
/// use twine_builder::{TwineBuilder, BiscuitSigner};
/// use biscuit::jwk::JWK;
/// use ring::signature::*;
/// use biscuit::jws::Secret;
/// let rng = ring::rand::SystemRandom::new();
/// let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
/// let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
/// let secret = Secret::EcdsaKeyPair(Arc::new(key));
/// let signer = BiscuitSigner::new(secret, "ES256".to_string());
/// ```
pub struct BiscuitSigner(Secret, String);

impl BiscuitSigner {
  /// Create a new `BiscuitSigner` with the given secret and algorithm.
  #[deprecated(note = "Use `RingSigner` with twine/2.0.0 instead")]
  pub fn new(secret: Secret, alg: String) -> Self {
    Self(secret, alg)
  }
}

impl From<RsaKeyPair> for BiscuitSigner {
  fn from(rsa: RsaKeyPair) -> Self {
    Self(Secret::RsaKeyPair(rsa.into()), "RS256".into())
  }
}

impl From<EcdsaKeyPair> for BiscuitSigner {
  fn from(ec: EcdsaKeyPair) -> Self {
    Self(Secret::EcdsaKeyPair(ec.into()), "PS256".into())
  }
}

impl Signer for BiscuitSigner {
  type Key = JWK<()>;

  fn sign<T: AsRef<[u8]>>(&self, data: T) -> Result<Signature, SigningError> {
    let mut header = Header::default();
    header.registered.algorithm = serde_json::from_value(json!(&self.1)).unwrap();
    header.registered.media_type = None;
    let jws = biscuit::jws::Compact::<_, ()>::new_decoded(header, data.as_ref().to_vec());
    let signature = jws
      .encode(&self.0)
      .map_err(|e| SigningError(format!("Failed to sign: {}", e)))?;
    Ok(signature.encoded().unwrap().encode().as_bytes().into())
  }

  fn public_key(&self) -> JWK<()> {
    use ring::signature::KeyPair;
    match &self.0 {
      Secret::RsaKeyPair(rsa) => {
        let pk = rsa.public_key();
        let components: ring::rsa::PublicKeyComponents<Vec<u8>> = pk.into();
        use num_bigint::BigUint;
        let params: biscuit::jwk::RSAKeyParameters = biscuit::jwk::RSAKeyParameters {
          key_type: biscuit::jwk::RSAKeyType::RSA,
          n: BigUint::from_bytes_be(&components.n),
          e: BigUint::from_bytes_be(&components.e),
          d: None,
          p: None,
          q: None,
          dp: None,
          dq: None,
          qi: None,
          other_primes_info: None,
        };
        let algorithm = AlgorithmParameters::RSA(params);
        let alg = &self.1;
        JWK {
          common: serde_json::from_value(json!({ "alg": alg })).unwrap(),
          algorithm,
          additional: (),
        }
      }
      Secret::EcdsaKeyPair(ec) => {
        let pk = ec.public_key();
        let point = pk.as_ref();
        let alg = &self.1;
        let (x, y) = point[1..].split_at((point.len() + 1) / 2);
        let params: biscuit::jwk::EllipticCurveKeyParameters =
          biscuit::jwk::EllipticCurveKeyParameters {
            key_type: biscuit::jwk::EllipticCurveKeyType::EC,
            curve: serde_json::from_value(json!(alg.replace("ES", "P-"))).unwrap(),
            x: x.to_vec(),
            y: y.to_vec(),
            d: None,
          };
        let algorithm = AlgorithmParameters::EllipticCurve(params);
        JWK {
          common: serde_json::from_value(json!({ "alg": alg })).unwrap(),
          algorithm,
          additional: (),
        }
      }
      _ => panic!("Unsupported key type"),
    }
  }
}
