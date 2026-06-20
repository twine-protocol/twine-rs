//! A signer backed entirely by [RustCrypto](https://github.com/RustCrypto)
//! primitives (no `ring`).
//!
//! [`RustCryptoSigner`] is the recommended signer for Twine v2 data. It supports
//! Ed25519, ECDSA P-256 / P-384, and (with the `rsa` feature) RSA, and is
//! designed to make safe key handling the path of least resistance:
//!
//! - private key material is **zeroized on drop**,
//! - the [`std::fmt::Debug`] impl **never** prints secret bytes,
//! - the type is intentionally **not [`Clone`]** to discourage copying secrets,
//! - keys can be imported/exported as **encrypted PKCS#8** (with the
//!   `encryption` feature) so they need never sit on disk in the clear,
//! - ECDSA signatures are always emitted in **canonical low-S form**, which is
//!   what allows v2 verification to be strict (see the crate-level docs).
use crate::{Signer, SigningError};
use pkcs8::{
  spki::EncodePublicKey, DecodePrivateKey, EncodePrivateKey, LineEnding, SecretDocument,
};
use rand_core::OsRng;
use thiserror::Error;
use twine_lib::crypto::{PublicKey, Signature, SignatureAlgorithm};
#[cfg(feature = "rsa")]
use twine_lib::crypto::MIN_RSA_KEY_BITS;
use zeroize::Zeroizing;

/// Errors produced when constructing or exporting a [`RustCryptoSigner`].
#[derive(Debug, Error)]
pub enum RustCryptoSignerError {
  /// The private key's algorithm is not supported by Twine.
  #[error("unsupported algorithm")]
  UnsupportedAlgorithm,
  /// The RSA modulus is below [`twine_lib::crypto::MIN_RSA_KEY_BITS`].
  #[error("RSA key size {0} bits is below the {1}-bit minimum")]
  WeakKey(usize, usize),
  /// A key could not be parsed or constructed.
  #[error("key error: {0}")]
  Key(String),
  /// A PKCS#8 (de)serialization error.
  #[error("pkcs8 error: {0}")]
  Pkcs8(String),
  /// An SPKI public-key encoding error.
  #[error("spki error: {0}")]
  Spki(String),
}

impl From<pkcs8::Error> for RustCryptoSignerError {
  fn from(e: pkcs8::Error) -> Self {
    RustCryptoSignerError::Pkcs8(e.to_string())
  }
}

impl From<pkcs8::der::Error> for RustCryptoSignerError {
  fn from(e: pkcs8::der::Error) -> Self {
    RustCryptoSignerError::Pkcs8(e.to_string())
  }
}

impl From<pkcs8::spki::Error> for RustCryptoSignerError {
  fn from(e: pkcs8::spki::Error) -> Self {
    RustCryptoSignerError::Spki(e.to_string())
  }
}

/// The private key material, held in exactly one form per algorithm.
///
/// The contained RustCrypto key types zeroize their secret scalars on drop, so
/// dropping a [`RustCryptoSigner`] wipes the key from memory.
enum SecretKey {
  Ed25519(ed25519_dalek::SigningKey),
  P256(p256::ecdsa::SigningKey),
  P384(p384::ecdsa::SigningKey),
  #[cfg(feature = "rsa")]
  Rsa {
    key: rsa::RsaPrivateKey,
    alg: SignatureAlgorithm,
  },
}

/// A RustCrypto-backed [`Signer`] for Twine data — the recommended signer for
/// Twine v2.
///
/// Supports Ed25519, ECDSA P-256 / P-384, and (with the `rsa` feature) RSA, and
/// is designed to make safe key handling the path of least resistance:
///
/// Begin by creating a signer. This can be done by importing a private key as a
/// PEM file. For example:
///
/// ```rust
/// use twine_builder::{RustCryptoSigner, Signer};
/// const PRIVATE_KEY_ED25519_PEM: &'static str = r#"
/// -----BEGIN PRIVATE KEY-----
/// MFECAQEwBQYDK2VwBCIEIJHCvDsbaia6M9aMlRXjdIMVbMyeGLwj/2crnzzoJnmH
/// gSEALX8wMpAh1EA0zraJTfEUx8F2uQBCvBmFkYpmvpX+jDc=
/// -----END PRIVATE KEY-----
/// "#;
///
/// let signer = RustCryptoSigner::from_pkcs8_pem(PRIVATE_KEY_ED25519_PEM).unwrap();
/// // print the public key
/// println!("{:?}", signer.public_key());
pub struct RustCryptoSigner(SecretKey);

impl RustCryptoSigner {
  // --- Generation -----------------------------------------------------------

  /// Generate a new Ed25519 signer using the operating system CSPRNG.
  ///
  /// Ed25519 is the recommended default: fast, small keys, and signatures are
  /// deterministic and canonical (no malleability to worry about).
  pub fn generate_ed25519() -> Self {
    Self(SecretKey::Ed25519(ed25519_dalek::SigningKey::generate(
      &mut OsRng,
    )))
  }

  /// Generate a new ECDSA P-256 signer using the operating system CSPRNG.
  pub fn generate_p256() -> Self {
    Self(SecretKey::P256(p256::ecdsa::SigningKey::random(&mut OsRng)))
  }

  /// Generate a new ECDSA P-384 signer using the operating system CSPRNG.
  pub fn generate_p384() -> Self {
    Self(SecretKey::P384(p384::ecdsa::SigningKey::random(&mut OsRng)))
  }

  /// Generate a new RSA (RS256) signer of the given modulus size.
  ///
  /// Returns [`RustCryptoSignerError::WeakKey`] if `bits` is below
  /// [`twine_lib::crypto::MIN_RSA_KEY_BITS`].
  #[cfg(feature = "rsa")]
  pub fn generate_rsa(bits: usize) -> Result<Self, RustCryptoSignerError> {
    if bits < MIN_RSA_KEY_BITS {
      return Err(RustCryptoSignerError::WeakKey(bits, MIN_RSA_KEY_BITS));
    }
    let key = rsa::RsaPrivateKey::new(&mut OsRng, bits)
      .map_err(|e| RustCryptoSignerError::Key(e.to_string()))?;
    Ok(Self(SecretKey::Rsa {
      key,
      alg: SignatureAlgorithm::Sha256Rsa(bits),
    }))
  }

  /// Generate a signer for the given algorithm.
  ///
  /// For RSA variants the modulus size encoded in the algorithm is used.
  pub fn generate(alg: SignatureAlgorithm) -> Result<Self, RustCryptoSignerError> {
    match alg {
      SignatureAlgorithm::Ed25519 => Ok(Self::generate_ed25519()),
      SignatureAlgorithm::EcdsaP256 => Ok(Self::generate_p256()),
      SignatureAlgorithm::EcdsaP384 => Ok(Self::generate_p384()),
      #[cfg(feature = "rsa")]
      SignatureAlgorithm::Sha256Rsa(bits)
      | SignatureAlgorithm::Sha384Rsa(bits)
      | SignatureAlgorithm::Sha512Rsa(bits) => {
        let mut s = Self::generate_rsa(bits)?;
        if let SecretKey::Rsa { alg: a, .. } = &mut s.0 {
          *a = alg;
        }
        Ok(s)
      }
      _ => Err(RustCryptoSignerError::UnsupportedAlgorithm),
    }
  }

  // --- Import ----------------------------------------------------------------

  /// Import a private key from PKCS#8 DER. The algorithm is detected from the
  /// key's OID.
  pub fn from_pkcs8_der(der: &[u8]) -> Result<Self, RustCryptoSignerError> {
    use pkcs8::der::Decode;
    let info = pkcs8::PrivateKeyInfo::from_der(der)?;
    let secret = match info.algorithm.oid {
      const_oid::db::rfc8410::ID_ED_25519 => SecretKey::Ed25519(
        ed25519_dalek::SigningKey::from_pkcs8_der(der)
          .map_err(|e| RustCryptoSignerError::Key(e.to_string()))?,
      ),
      const_oid::db::rfc5912::ID_EC_PUBLIC_KEY => {
        let curve = info
          .algorithm
          .parameters_oid()
          .map_err(|_| RustCryptoSignerError::UnsupportedAlgorithm)?;
        match curve {
          const_oid::db::rfc5912::SECP_256_R_1 => SecretKey::P256(
            p256::ecdsa::SigningKey::from_pkcs8_der(der)
              .map_err(|e| RustCryptoSignerError::Key(e.to_string()))?,
          ),
          const_oid::db::rfc5912::SECP_384_R_1 => SecretKey::P384(
            p384::ecdsa::SigningKey::from_pkcs8_der(der)
              .map_err(|e| RustCryptoSignerError::Key(e.to_string()))?,
          ),
          _ => return Err(RustCryptoSignerError::UnsupportedAlgorithm),
        }
      }
      #[cfg(feature = "rsa")]
      const_oid::db::rfc5912::RSA_ENCRYPTION => {
        use rsa::traits::PublicKeyParts;
        let key = rsa::RsaPrivateKey::from_pkcs8_der(der)
          .map_err(|e| RustCryptoSignerError::Key(e.to_string()))?;
        let bits = key.n().bits();
        if bits < MIN_RSA_KEY_BITS {
          return Err(RustCryptoSignerError::WeakKey(bits, MIN_RSA_KEY_BITS));
        }
        SecretKey::Rsa {
          key,
          alg: SignatureAlgorithm::Sha256Rsa(bits),
        }
      }
      _ => return Err(RustCryptoSignerError::UnsupportedAlgorithm),
    };
    Ok(Self(secret))
  }

  /// Import a private key from a PKCS#8 PEM document (`-----BEGIN PRIVATE KEY-----`).
  pub fn from_pkcs8_pem(pem: &str) -> Result<Self, RustCryptoSignerError> {
    let (_, doc) = SecretDocument::from_pem(pem)?;
    Self::from_pkcs8_der(doc.as_bytes())
  }

  /// Import a password-encrypted PKCS#8 PEM document
  /// (`-----BEGIN ENCRYPTED PRIVATE KEY-----`).
  ///
  /// Lets keys live on disk encrypted at rest rather than in the clear.
  #[cfg(feature = "encryption")]
  pub fn from_encrypted_pkcs8_pem(
    pem: &str,
    password: impl AsRef<[u8]>,
  ) -> Result<Self, RustCryptoSignerError> {
    let (_, doc) = SecretDocument::from_pem(pem)?;
    let enc = pkcs8::EncryptedPrivateKeyInfo::try_from(doc.as_bytes())?;
    let plain = enc.decrypt(password)?;
    Self::from_pkcs8_der(plain.as_bytes())
  }

  // --- Export (sensitive) ----------------------------------------------------

  /// Export the private key as PKCS#8 DER. The returned document zeroizes on drop.
  pub fn to_pkcs8_der(&self) -> Result<SecretDocument, RustCryptoSignerError> {
    Ok(match &self.0 {
      SecretKey::Ed25519(k) => k.to_pkcs8_der()?,
      SecretKey::P256(k) => k.to_pkcs8_der()?,
      SecretKey::P384(k) => k.to_pkcs8_der()?,
      #[cfg(feature = "rsa")]
      SecretKey::Rsa { key, .. } => key.to_pkcs8_der()?,
    })
  }

  /// Export the private key as PKCS#8 PEM. The returned string zeroizes on drop.
  pub fn to_pkcs8_pem(&self) -> Result<Zeroizing<String>, RustCryptoSignerError> {
    Ok(self.to_pkcs8_der()?.to_pem("PRIVATE KEY", LineEnding::LF)?)
  }

  /// Export the private key as a password-encrypted PKCS#8 PEM document.
  ///
  /// This is the recommended way to persist a key.
  #[cfg(feature = "encryption")]
  pub fn to_encrypted_pkcs8_pem(
    &self,
    password: impl AsRef<[u8]>,
  ) -> Result<String, RustCryptoSignerError> {
    use pkcs8::der::Decode;
    let plain = self.to_pkcs8_der()?;
    let info = pkcs8::PrivateKeyInfo::from_der(plain.as_bytes())?;
    let enc = info.encrypt(OsRng, password.as_ref())?;
    Ok(enc.to_pem("ENCRYPTED PRIVATE KEY", LineEnding::LF)?.to_string())
  }

  // --- Public material -------------------------------------------------------

  /// The signature algorithm this signer uses.
  pub fn algorithm(&self) -> SignatureAlgorithm {
    match &self.0 {
      SecretKey::Ed25519(_) => SignatureAlgorithm::Ed25519,
      SecretKey::P256(_) => SignatureAlgorithm::EcdsaP256,
      SecretKey::P384(_) => SignatureAlgorithm::EcdsaP384,
      #[cfg(feature = "rsa")]
      SecretKey::Rsa { alg, .. } => alg.clone(),
    }
  }

  /// Export the public key as SPKI PEM (`-----BEGIN PUBLIC KEY-----`), suitable
  /// for sharing or pinning a trusted signer.
  pub fn public_key_pem(&self) -> Result<String, RustCryptoSignerError> {
    let pem = match &self.0 {
      SecretKey::Ed25519(k) => k.verifying_key().to_public_key_pem(LineEnding::LF)?,
      SecretKey::P256(k) => k.verifying_key().to_public_key_pem(LineEnding::LF)?,
      SecretKey::P384(k) => k.verifying_key().to_public_key_pem(LineEnding::LF)?,
      #[cfg(feature = "rsa")]
      SecretKey::Rsa { key, .. } => key.to_public_key().to_public_key_pem(LineEnding::LF)?,
    };
    Ok(pem)
  }
}

impl Signer for RustCryptoSigner {
  type Key = PublicKey;

  fn sign<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SigningError> {
    let msg = message.as_ref();
    let bytes: Vec<u8> = match &self.0 {
      SecretKey::Ed25519(k) => {
        use ed25519_dalek::Signer as _;
        k.sign(msg).to_bytes().to_vec()
      }
      SecretKey::P256(k) => {
        use p256::ecdsa::signature::Signer as _;
        let sig: p256::ecdsa::Signature = k.sign(msg);
        // Always emit canonical low-S so v2 verification can be strict.
        let sig = sig.normalize_s().unwrap_or(sig);
        sig.to_der().as_bytes().to_vec()
      }
      SecretKey::P384(k) => {
        use p384::ecdsa::signature::Signer as _;
        let sig: p384::ecdsa::Signature = k.sign(msg);
        let sig = sig.normalize_s().unwrap_or(sig);
        sig.to_der().as_bytes().to_vec()
      }
      #[cfg(feature = "rsa")]
      SecretKey::Rsa { key, alg } => {
        use rsa::signature::{SignatureEncoding, Signer as _};
        use sha2::{Sha256, Sha384, Sha512};
        match alg {
          SignatureAlgorithm::Sha256Rsa(_) => rsa::pkcs1v15::SigningKey::<Sha256>::new(key.clone())
            .try_sign(msg)
            .map_err(|e| SigningError(e.to_string()))?
            .to_vec(),
          SignatureAlgorithm::Sha384Rsa(_) => rsa::pkcs1v15::SigningKey::<Sha384>::new(key.clone())
            .try_sign(msg)
            .map_err(|e| SigningError(e.to_string()))?
            .to_vec(),
          SignatureAlgorithm::Sha512Rsa(_) => rsa::pkcs1v15::SigningKey::<Sha512>::new(key.clone())
            .try_sign(msg)
            .map_err(|e| SigningError(e.to_string()))?
            .to_vec(),
          _ => return Err(SigningError("invalid RSA algorithm".into())),
        }
      }
    };
    Ok(bytes.into())
  }

  fn public_key(&self) -> Self::Key {
    match &self.0 {
      SecretKey::Ed25519(k) => PublicKey {
        alg: SignatureAlgorithm::Ed25519,
        key: k.verifying_key().to_bytes().as_slice().into(),
      },
      SecretKey::P256(k) => PublicKey {
        alg: SignatureAlgorithm::EcdsaP256,
        key: k.verifying_key().to_encoded_point(false).as_bytes().into(),
      },
      SecretKey::P384(k) => PublicKey {
        alg: SignatureAlgorithm::EcdsaP384,
        key: k.verifying_key().to_encoded_point(false).as_bytes().into(),
      },
      #[cfg(feature = "rsa")]
      SecretKey::Rsa { key, alg } => {
        use rsa::pkcs1::EncodeRsaPublicKey;
        PublicKey {
          alg: alg.clone(),
          key: key
            .to_public_key()
            .to_pkcs1_der()
            .expect("RSA public key encodes")
            .as_bytes()
            .into(),
        }
      }
    }
  }
}

/// Redacted: never prints secret key material.
impl std::fmt::Debug for RustCryptoSigner {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RustCryptoSigner")
      .field("algorithm", &self.algorithm())
      .field("private_key", &"<redacted>")
      .finish()
  }
}

#[cfg(test)]
mod test {
  use super::*;

  const MSG: &[u8] = b"twine rustcrypto signer test message";

  /// Sign MSG, verify it through the real twine_lib verifier, and confirm a
  /// tampered message is rejected.
  fn assert_roundtrip(signer: &RustCryptoSigner) {
    let sig = signer.sign(MSG).expect("sign");
    let pk = signer.public_key();
    assert_eq!(pk.alg.to_string(), signer.algorithm().to_string());
    pk.verify(sig.clone(), MSG).expect("valid signature verifies");

    let mut tampered = MSG.to_vec();
    tampered[0] ^= 0xff;
    pk.verify(sig, tampered.as_slice())
      .expect_err("tampered message must not verify");
  }

  #[test]
  fn ed25519_roundtrip() {
    assert_roundtrip(&RustCryptoSigner::generate_ed25519());
  }

  #[test]
  fn p256_roundtrip_and_low_s() {
    let signer = RustCryptoSigner::generate_p256();
    assert_roundtrip(&signer);
    // Run several signatures; every one must be canonical low-S.
    for _ in 0..16 {
      let sig = signer.sign(MSG).unwrap();
      let parsed = p256::ecdsa::Signature::from_der(sig.as_ref()).unwrap();
      assert!(
        parsed.normalize_s().is_none(),
        "emitted ECDSA signature must be low-S"
      );
    }
  }

  #[test]
  fn p384_roundtrip_and_low_s() {
    let signer = RustCryptoSigner::generate_p384();
    assert_roundtrip(&signer);
    let sig = signer.sign(MSG).unwrap();
    let parsed = p384::ecdsa::Signature::from_der(sig.as_ref()).unwrap();
    assert!(parsed.normalize_s().is_none(), "p384 must be low-S");
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn rsa_roundtrip() {
    let signer = RustCryptoSigner::generate_rsa(2048).unwrap();
    assert_roundtrip(&signer);
  }

  #[cfg(feature = "rsa")]
  #[test]
  fn rsa_rejects_weak_key() {
    let err = RustCryptoSigner::generate_rsa(1024).unwrap_err();
    assert!(matches!(err, RustCryptoSignerError::WeakKey(1024, _)));
  }

  #[test]
  fn pkcs8_der_and_pem_round_trip() {
    let signer = RustCryptoSigner::generate_p256();
    let der = signer.to_pkcs8_der().unwrap();
    let reimported = RustCryptoSigner::from_pkcs8_der(der.as_bytes()).unwrap();
    // Same key => same public key.
    assert_eq!(
      signer.public_key().key.as_ref(),
      reimported.public_key().key.as_ref()
    );

    let pem = signer.to_pkcs8_pem().unwrap();
    assert!(pem.contains("BEGIN PRIVATE KEY"));
    let from_pem = RustCryptoSigner::from_pkcs8_pem(&pem).unwrap();
    let sig = from_pem.sign(MSG).unwrap();
    signer.public_key().verify(sig, MSG).unwrap();
  }

  #[test]
  fn detects_algorithm_from_pkcs8_for_each_curve() {
    for s in [
      RustCryptoSigner::generate_ed25519(),
      RustCryptoSigner::generate_p256(),
      RustCryptoSigner::generate_p384(),
    ] {
      let der = s.to_pkcs8_der().unwrap();
      let back = RustCryptoSigner::from_pkcs8_der(der.as_bytes()).unwrap();
      assert_eq!(back.algorithm().to_string(), s.algorithm().to_string());
    }
  }

  #[cfg(feature = "encryption")]
  #[test]
  fn encrypted_pkcs8_round_trip() {
    let signer = RustCryptoSigner::generate_ed25519();
    let pem = signer.to_encrypted_pkcs8_pem("hunter2").unwrap();
    assert!(pem.contains("BEGIN ENCRYPTED PRIVATE KEY"));

    // Wrong password fails.
    assert!(RustCryptoSigner::from_encrypted_pkcs8_pem(&pem, "wrong").is_err());

    // Right password recovers the same key.
    let back = RustCryptoSigner::from_encrypted_pkcs8_pem(&pem, "hunter2").unwrap();
    assert_eq!(
      signer.public_key().key.as_ref(),
      back.public_key().key.as_ref()
    );
  }

  #[test]
  fn public_key_pem_is_spki() {
    let pem = RustCryptoSigner::generate_ed25519().public_key_pem().unwrap();
    assert!(pem.contains("BEGIN PUBLIC KEY"));
  }

  #[test]
  fn debug_does_not_leak_secret() {
    let signer = RustCryptoSigner::generate_ed25519();
    let dbg = format!("{signer:?}");
    assert!(dbg.contains("redacted"));
    // The PKCS#8 secret bytes must not appear in the debug output.
    let der = signer.to_pkcs8_der().unwrap();
    let hex: String = der.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    assert!(!dbg.contains(&hex));
  }
}
