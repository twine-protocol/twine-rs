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

#[cfg(test)]
mod test {
  use super::*;
  use biscuit::jwk::AlgorithmParameters;
  use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
  use std::sync::Arc;

  fn make_ecdsa_signer() -> BiscuitSigner {
    let rng = ring::rand::SystemRandom::new();
    let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let key =
      EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
    #[allow(deprecated)]
    BiscuitSigner::new(Secret::EcdsaKeyPair(Arc::new(key)), "ES256".to_string())
  }

  /// Construct a BiscuitSigner from EcdsaKeyPair (ECDSA P-256) and call
  /// `sign` + `public_key()` without panicking.  Check that:
  ///   - `sign` returns `Ok`
  ///   - `public_key()` returns a JWK with the EC algorithm parameters
  ///   - the `From<EcdsaKeyPair>` conversion also produces a usable signer
  #[test]
  fn test_ecdsa_signer_sign_and_public_key() {
    let signer = make_ecdsa_signer();

    let msg = b"biscuit ecdsa test";
    let sig = Signer::sign(&signer, msg as &[u8]).expect("ECDSA sign must succeed");
    assert!(!sig.is_empty(), "signature bytes must not be empty");

    let jwk = Signer::public_key(&signer);
    // The JWK algorithm parameters must be EllipticCurve.
    assert!(
      matches!(jwk.algorithm, AlgorithmParameters::EllipticCurve(_)),
      "public_key() must return EC JWK"
    );
  }

  /// Document the behavior of `From<EcdsaKeyPair>` for `BiscuitSigner`.
  ///
  /// `From<EcdsaKeyPair>` sets the algorithm string to "PS256" (RSA-PSS),
  /// which is inconsistent with an ECDSA key pair.  As a result:
  ///   - `sign` returns an error (biscuit requires an RsaKeyPair for PS256)
  ///   - `public_key` would panic (replace("ES", "P-") doesn't match "PS256")
  ///
  /// This test exists to document the broken invariant and to ensure the
  /// failure is an Err result from `sign`, not an unhandled panic from the
  /// wrong key type.
  #[test]
  fn test_from_ecdsa_keypair_sign_returns_err_due_to_wrong_alg() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let key =
      EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
    let signer = BiscuitSigner::from(key);

    // The alg string "PS256" requires an RsaKeyPair; biscuit returns an error
    // rather than panicking, so sign must return Err, not panic.
    let result = Signer::sign(&signer, b"hello" as &[u8]);
    assert!(
      result.is_err(),
      "From<EcdsaKeyPair> with PS256 alg must fail to sign (wrong key type)"
    );
  }

  /// Construct a BiscuitSigner from an RsaKeyPair (via rsa crate + ring
  /// loader) and call `sign` + `public_key()` without panicking.
  ///
  /// Note: We use the `rsa` dev-dependency to generate the key material
  /// (only available under `#[cfg(test)]`) so that the test binary can
  /// produce a 2048-bit RSA key quickly.
  #[cfg(feature = "rsa")]
  #[test]
  fn test_rsa_signer_sign_and_public_key() {
    use rsa::pkcs8::EncodePrivateKey;
    // Generate a 2048-bit RSA key via the rsa crate.
    let rsa_key =
      rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("RSA keygen must succeed");
    let pkcs8_der = rsa_key
      .to_pkcs8_der()
      .expect("PKCS#8 encode must succeed");

    // Load it into a ring RsaKeyPair.
    let ring_key =
      ring::signature::RsaKeyPair::from_pkcs8(pkcs8_der.as_bytes()).expect("ring must accept 2048-bit RSA");
    let signer = BiscuitSigner::from(ring_key);

    let msg = b"biscuit rsa test";
    let sig = Signer::sign(&signer, msg as &[u8]).expect("RSA sign must succeed");
    assert!(!sig.is_empty(), "RSA signature bytes must not be empty");

    let jwk = Signer::public_key(&signer);
    assert!(
      matches!(jwk.algorithm, AlgorithmParameters::RSA(_)),
      "public_key() must return RSA JWK"
    );
  }

  /// `From<RsaKeyPair>` gives an RS256 signer; verify the JWK carries RSA
  /// parameters and that `sign` does not panic.
  #[cfg(feature = "rsa")]
  #[test]
  fn test_from_rsa_keypair_conversion() {
    use rsa::pkcs8::EncodePrivateKey;
    let rsa_key =
      rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("RSA keygen must succeed");
    let pkcs8_der = rsa_key.to_pkcs8_der().unwrap();
    let ring_key = ring::signature::RsaKeyPair::from_pkcs8(pkcs8_der.as_bytes()).unwrap();
    let signer = BiscuitSigner::from(ring_key);

    let jwk = Signer::public_key(&signer);
    assert!(matches!(jwk.algorithm, AlgorithmParameters::RSA(_)));

    let sig = Signer::sign(&signer, b"test" as &[u8]).expect("sign must succeed");
    assert!(!sig.is_empty());
  }

  /// Exercise the JWK → PublicKey conversion (From<JWK<()>> for PublicKey)
  /// for the ECDSA branch.  We cannot fully verify the biscuit JWS signature
  /// through `PublicKey::verify` because the v1 biscuit encoding is a JWS
  /// compact string, not a raw byte signature.  What we CAN verify is that:
  ///   - the conversion does not panic
  ///   - the resulting PublicKey reports EcdsaP256
  #[test]
  fn test_ecdsa_jwk_to_public_key_conversion() {
    use twine_lib::crypto::PublicKey;
    let signer = make_ecdsa_signer();
    let jwk = Signer::public_key(&signer);

    // From<JWK<()>> must not panic.
    let pk: PublicKey = jwk.into();
    assert!(
      matches!(pk.alg, twine_lib::crypto::SignatureAlgorithm::EcdsaP256),
      "JWK→PublicKey conversion must give EcdsaP256"
    );
    assert!(!pk.key.is_empty());
  }
}
