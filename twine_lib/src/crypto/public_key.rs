use super::Signature;
use crate::{errors::VerificationError, Bytes};
use biscuit::jwk::JWK;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

/// The minimum RSA modulus size (in bits) accepted for signing and verification.
///
/// Smaller moduli are considered cryptographically weak. Keeping this floor in
/// one place means a non-expert can neither generate nor accept a weak RSA key
/// without deliberately bypassing the library.
pub const MIN_RSA_KEY_BITS: usize = 2048;

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
      // NB: the input is upper-cased above, so these arms must be upper-case too.
      "ED25519 SHA512" => Ok(SignatureAlgorithm::Ed25519),
      // shorthand
      "RS256" => Ok(SignatureAlgorithm::Sha256Rsa(2048)),
      "RS384" => Ok(SignatureAlgorithm::Sha384Rsa(2048)),
      "RS512" => Ok(SignatureAlgorithm::Sha512Rsa(2048)),
      "ES256" => Ok(SignatureAlgorithm::EcdsaP256),
      "ES384" => Ok(SignatureAlgorithm::EcdsaP384),
      "ED25519" => Ok(SignatureAlgorithm::Ed25519),
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
    use rsa::traits::PublicKeyParts;
    use rsa::Pkcs1v15Sign;
    use sha2::{Digest, Sha256, Sha384, Sha512};

    // The stored key is a PKCS#1 RSAPublicKey. Any modulus size verifies, so
    // there is no per-bitsize table to keep in sync — 2048/3072/4096/… all work.
    let public_key = rsa::RsaPublicKey::from_pkcs1_der(&self.key)
      .map_err(|e| VerificationError::BadSignature(e.to_string()))?;

    // Reject undersized moduli regardless of what the strand declares, so a
    // weak RSA strand can never produce a passing verification.
    let bits = public_key.n().bits();
    if bits < MIN_RSA_KEY_BITS {
      return Err(VerificationError::WeakKey(format!(
        "RSA modulus is {bits} bits; minimum is {MIN_RSA_KEY_BITS}"
      )));
    }

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
    //
    // We additionally require the signature to be in canonical **low-S** form.
    // ECDSA is malleable: for any valid `(r, s)`, `(r, n - s)` also verifies,
    // and because a Twine CID is hashed over content *and* signature, a high-S
    // twin would have the same content but a different CID. Rejecting high-S
    // here makes each (key, content) pair map to exactly one valid CID.
    //
    // This is reached only by the v2 verification path; v1 signatures are
    // checked via biscuit JWS (see `crypto::jws`) and remain permissive for
    // backward compatibility with already-deployed v1 chains.
    use p256::ecdsa::signature::Verifier;
    match self.alg {
      SignatureAlgorithm::EcdsaP256 => {
        use p256::ecdsa::{Signature as EcSignature, VerifyingKey};
        let key = VerifyingKey::from_sec1_bytes(&self.key)
          .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
        let sig = EcSignature::from_der(signature.as_ref())
          .map_err(|e| VerificationError::BadSignature(e.to_string()))?;
        if sig.normalize_s().is_some() {
          return Err(VerificationError::BadSignature(
            "non-canonical (high-S) ECDSA signature".into(),
          ));
        }
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
        if sig.normalize_s().is_some() {
          return Err(VerificationError::BadSignature(
            "non-canonical (high-S) ECDSA signature".into(),
          ));
        }
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

  // -----------------------------------------------------------------------
  // Helpers
  // -----------------------------------------------------------------------

  /// Build an RSA-2048 PublicKey + sign a message using the bundled test key.
  /// Returns (PublicKey, signature_bytes).
  ///
  /// The file `rsa_test_private_key_2048.p8` is in PKCS#8 DER format (the
  /// extension `.p8` is conventional for PKCS#8).  Use `from_pkcs8_der`.
  fn rsa2048_fixture(msg: &[u8]) -> (PublicKey, Vec<u8>) {
    use rsa::pkcs1::EncodeRsaPublicKey;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{SignatureEncoding, Signer};
    use sha2::Sha256;

    let der = include_bytes!("../test/rsa_test_private_key_2048.p8");
    let priv_key = rsa::RsaPrivateKey::from_pkcs8_der(der).unwrap();
    let pub_key = rsa::RsaPublicKey::from(&priv_key);
    let pub_der = pub_key.to_pkcs1_der().unwrap();

    let signing_key = SigningKey::<Sha256>::new(priv_key);
    let sig: rsa::pkcs1v15::Signature = signing_key.sign(msg);

    let pk = PublicKey::new(
      SignatureAlgorithm::Sha256Rsa(2048),
      Bytes::from(pub_der.as_bytes()),
    );
    (pk, sig.to_vec())
  }

  // -----------------------------------------------------------------------
  // SignatureAlgorithm::from_str — standard strings
  // -----------------------------------------------------------------------

  #[test]
  fn from_str_standard_rsa_2048_sha256() {
    let alg: SignatureAlgorithm = "RSA 2048 SHA256".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha256Rsa(2048)));
  }

  #[test]
  fn from_str_standard_rsa_3072_sha384() {
    let alg: SignatureAlgorithm = "RSA 3072 SHA384".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha384Rsa(3072)));
  }

  #[test]
  fn from_str_standard_rsa_4096_sha512() {
    let alg: SignatureAlgorithm = "RSA 4096 SHA512".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha512Rsa(4096)));
  }

  #[test]
  fn from_str_standard_ecdsa_p256() {
    let alg: SignatureAlgorithm = "ECDSA P-256 SHA256".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::EcdsaP256));
  }

  #[test]
  fn from_str_standard_ecdsa_p384() {
    let alg: SignatureAlgorithm = "ECDSA P-384 SHA384".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::EcdsaP384));
  }

  #[test]
  fn from_str_standard_ed25519() {
    // Case-insensitive: from_str upper-cases first, and the match arm is
    // upper-case to match. All of these must parse to Ed25519.
    assert!(matches!(
      "Ed25519 SHA512".parse::<SignatureAlgorithm>(),
      Ok(SignatureAlgorithm::Ed25519)
    ));
    assert!(matches!(
      "ED25519 SHA512".parse::<SignatureAlgorithm>(),
      Ok(SignatureAlgorithm::Ed25519)
    ));
  }

  // -----------------------------------------------------------------------
  // SignatureAlgorithm::from_str — shorthand strings
  // -----------------------------------------------------------------------

  #[test]
  fn from_str_shorthand_rs256() {
    let alg: SignatureAlgorithm = "RS256".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha256Rsa(2048)));
  }

  #[test]
  fn from_str_shorthand_rs384() {
    let alg: SignatureAlgorithm = "RS384".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha384Rsa(2048)));
  }

  #[test]
  fn from_str_shorthand_rs512() {
    let alg: SignatureAlgorithm = "RS512".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha512Rsa(2048)));
  }

  #[test]
  fn from_str_shorthand_es256() {
    let alg: SignatureAlgorithm = "ES256".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::EcdsaP256));
  }

  #[test]
  fn from_str_shorthand_es384() {
    let alg: SignatureAlgorithm = "ES384".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::EcdsaP384));
  }

  #[test]
  fn from_str_shorthand_ed25519() {
    assert!(matches!(
      "Ed25519".parse::<SignatureAlgorithm>(),
      Ok(SignatureAlgorithm::Ed25519)
    ));
  }

  // -----------------------------------------------------------------------
  // SignatureAlgorithm::from_str — case-insensitivity & whitespace
  // -----------------------------------------------------------------------

  #[test]
  fn from_str_case_insensitive_rs256() {
    // "rs256" should parse the same as "RS256"
    let alg: SignatureAlgorithm = "rs256".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha256Rsa(2048)));
  }

  #[test]
  fn from_str_leading_trailing_whitespace() {
    let alg: SignatureAlgorithm = "  RS256  ".parse().unwrap();
    assert!(matches!(alg, SignatureAlgorithm::Sha256Rsa(2048)));
  }

  #[test]
  fn from_str_mixed_case_ed25519_standard() {
    // Parsing is case-insensitive: "ed25519 sha512" upper-cases to the
    // "ED25519 SHA512" arm.
    assert!(matches!(
      "ed25519 sha512".parse::<SignatureAlgorithm>(),
      Ok(SignatureAlgorithm::Ed25519)
    ));
  }

  // -----------------------------------------------------------------------
  // SignatureAlgorithm::from_str — invalid strings
  // -----------------------------------------------------------------------

  #[test]
  fn from_str_invalid_empty() {
    assert!("".parse::<SignatureAlgorithm>().is_err());
  }

  #[test]
  fn from_str_invalid_garbage() {
    assert!("not_an_alg".parse::<SignatureAlgorithm>().is_err());
  }

  #[test]
  fn from_str_invalid_unknown_rsa_size() {
    // 1024 is not in the table
    assert!("RSA 1024 SHA256".parse::<SignatureAlgorithm>().is_err());
  }

  #[test]
  fn from_str_invalid_partial_match() {
    assert!("RSA 2048".parse::<SignatureAlgorithm>().is_err());
  }

  #[test]
  fn from_str_invalid_whitespace_only() {
    assert!("   ".parse::<SignatureAlgorithm>().is_err());
  }

  // -----------------------------------------------------------------------
  // SignatureAlgorithm::Display
  // -----------------------------------------------------------------------

  #[test]
  fn display_sha256_rsa_2048() {
    assert_eq!(
      SignatureAlgorithm::Sha256Rsa(2048).to_string(),
      "RSA 2048 SHA256"
    );
  }

  #[test]
  fn display_sha384_rsa_3072() {
    assert_eq!(
      SignatureAlgorithm::Sha384Rsa(3072).to_string(),
      "RSA 3072 SHA384"
    );
  }

  #[test]
  fn display_sha512_rsa_4096() {
    assert_eq!(
      SignatureAlgorithm::Sha512Rsa(4096).to_string(),
      "RSA 4096 SHA512"
    );
  }

  #[test]
  fn display_ecdsa_p256() {
    assert_eq!(
      SignatureAlgorithm::EcdsaP256.to_string(),
      "ECDSA P-256 SHA256"
    );
  }

  #[test]
  fn display_ecdsa_p384() {
    assert_eq!(
      SignatureAlgorithm::EcdsaP384.to_string(),
      "ECDSA P-384 SHA384"
    );
  }

  #[test]
  fn display_ed25519() {
    assert_eq!(SignatureAlgorithm::Ed25519.to_string(), "Ed25519 SHA512");
  }

  // -----------------------------------------------------------------------
  // Display round-trip: Display → from_str
  // -----------------------------------------------------------------------

  #[test]
  fn display_roundtrip_all_variants() {
    let variants: &[SignatureAlgorithm] = &[
      SignatureAlgorithm::Sha256Rsa(2048),
      SignatureAlgorithm::Sha384Rsa(3072),
      SignatureAlgorithm::Sha512Rsa(4096),
      SignatureAlgorithm::EcdsaP256,
      SignatureAlgorithm::EcdsaP384,
      SignatureAlgorithm::Ed25519,
    ];
    for v in variants {
      let s = v.to_string();
      let parsed: SignatureAlgorithm = s.parse().expect(&s);
      assert_eq!(parsed.to_string(), s);
    }
  }

  // -----------------------------------------------------------------------
  // Ed25519 verify
  // -----------------------------------------------------------------------

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
  fn ed25519_rejects_tampered_message() {
    use ed25519_dalek::{Signer, SigningKey};

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let sig = signing_key.sign(b"hello, world");
    let pk = PublicKey::new(
      SignatureAlgorithm::Ed25519,
      Bytes::from(signing_key.verifying_key().to_bytes().as_slice()),
    );
    let err = pk
      .verify(Bytes::from(sig.to_bytes().as_slice()), b"goodbye, world")
      .unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn ed25519_rejects_garbage_signature() {
    use ed25519_dalek::SigningKey;

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let pk = PublicKey::new(
      SignatureAlgorithm::Ed25519,
      Bytes::from(signing_key.verifying_key().to_bytes().as_slice()),
    );
    // A 64-byte garbage buffer is a valid-length Ed25519 signature byte sequence
    // but will fail mathematical verification.
    let garbage = Bytes::from(vec![0xffu8; 64]);
    let err = pk.verify(garbage, b"hello").unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn ed25519_rejects_truncated_signature() {
    use ed25519_dalek::SigningKey;

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let pk = PublicKey::new(
      SignatureAlgorithm::Ed25519,
      Bytes::from(signing_key.verifying_key().to_bytes().as_slice()),
    );
    // Only 10 bytes — must not panic, must return an error.
    let short = Bytes::from(vec![0u8; 10]);
    assert!(pk.verify(short, b"hello").is_err());
  }

  #[test]
  fn ed25519_rejects_short_public_key() {
    // A key shorter than 32 bytes must not panic.
    let pk = PublicKey::new(
      SignatureAlgorithm::Ed25519,
      Bytes::from(vec![0u8; 10]), // too short
    );
    let garbage_sig = Bytes::from(vec![0u8; 64]);
    assert!(pk.verify(garbage_sig, b"hello").is_err());
  }

  // -----------------------------------------------------------------------
  // ECDSA P-256 verify
  // -----------------------------------------------------------------------

  #[test]
  fn ecdsa_p256_valid_signature() {
    use p256::ecdsa::{signature::Signer, SigningKey};

    let signing_key = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
    let message = b"test message p256";
    // ECDSA's raw `sign` may emit high-S; v2 requires canonical low-S.
    let sig: p256::ecdsa::Signature = signing_key.sign(message);
    let sig = sig.normalize_s().unwrap_or(sig);
    let der_sig = sig.to_der();

    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP256,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    pk.verify(Bytes::from(der_sig.as_bytes()), message).unwrap();
  }

  #[test]
  fn ecdsa_p256_rejects_high_s_signature() {
    use p256::ecdsa::{signature::Signer, SigningKey};

    let signing_key = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
    let message = b"test message p256";
    // Start from the canonical low-S signature, then build the malleable
    // high-S twin (r, n - s), which is mathematically valid but must be rejected.
    let sig: p256::ecdsa::Signature = signing_key.sign(message);
    let sig = sig.normalize_s().unwrap_or(sig);
    let neg_s = -*sig.s();
    let high = p256::ecdsa::Signature::from_scalars(sig.r().to_bytes(), neg_s.to_bytes()).unwrap();
    assert!(
      high.normalize_s().is_some(),
      "twin must actually be high-S"
    );

    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP256,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    // The canonical signature verifies...
    pk.verify(Bytes::from(sig.to_der().as_bytes()), message)
      .unwrap();
    // ...but its high-S twin is rejected.
    let err = pk
      .verify(Bytes::from(high.to_der().as_bytes()), message)
      .unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn ecdsa_p256_rejects_tampered_message() {
    use p256::ecdsa::{signature::Signer, SigningKey};

    let signing_key = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
    let sig: p256::ecdsa::Signature = signing_key.sign(b"original message");
    let der_sig = sig.to_der();

    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP256,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    let err = pk
      .verify(Bytes::from(der_sig.as_bytes()), b"tampered message")
      .unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn ecdsa_p256_rejects_garbage_signature() {
    use p256::ecdsa::{SigningKey};

    let signing_key = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP256,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    let garbage = Bytes::from(vec![0u8; 72]);
    assert!(pk.verify(garbage, b"hello").is_err());
  }

  #[test]
  fn ecdsa_p256_rejects_truncated_signature() {
    use p256::ecdsa::{SigningKey};

    let signing_key = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP256,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    // Must not panic on a tiny buffer.
    assert!(pk
      .verify(Bytes::from(vec![0u8; 4]), b"hello")
      .is_err());
  }

  #[test]
  fn ecdsa_p256_rejects_corrupted_public_key() {
    // All-zeros is not a valid EC point.
    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP256,
      Bytes::from(vec![0u8; 65]), // uncompressed point length but invalid
    );
    let garbage_sig = Bytes::from(vec![0u8; 64]);
    assert!(pk.verify(garbage_sig, b"hello").is_err());
  }

  // -----------------------------------------------------------------------
  // ECDSA P-384 verify
  // -----------------------------------------------------------------------

  #[test]
  fn ecdsa_p384_valid_signature() {
    use p384::ecdsa::{signature::Signer, SigningKey};

    let key_bytes = [5u8; 48];
    let signing_key = SigningKey::from_bytes((&key_bytes).into()).unwrap();
    let message = b"test message p384";
    let sig: p384::ecdsa::Signature = signing_key.sign(message);
    let der_sig = sig.to_der();

    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP384,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    pk.verify(Bytes::from(der_sig.as_bytes()), message).unwrap();
  }

  #[test]
  fn ecdsa_p384_rejects_tampered_message() {
    use p384::ecdsa::{signature::Signer, SigningKey};

    let key_bytes = [5u8; 48];
    let signing_key = SigningKey::from_bytes((&key_bytes).into()).unwrap();
    let sig: p384::ecdsa::Signature = signing_key.sign(b"original message");
    let der_sig = sig.to_der();

    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP384,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    let err = pk
      .verify(Bytes::from(der_sig.as_bytes()), b"tampered message")
      .unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn ecdsa_p384_rejects_garbage_signature() {
    use p384::ecdsa::{SigningKey};

    let key_bytes = [5u8; 48];
    let signing_key = SigningKey::from_bytes((&key_bytes).into()).unwrap();
    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP384,
      Bytes::from(signing_key.verifying_key().to_sec1_bytes().as_ref()),
    );
    let garbage = Bytes::from(vec![0xabu8; 100]);
    assert!(pk.verify(garbage, b"hello").is_err());
  }

  #[test]
  fn ecdsa_p384_rejects_corrupted_public_key() {
    let pk = PublicKey::new(
      SignatureAlgorithm::EcdsaP384,
      Bytes::from(vec![0u8; 97]), // uncompressed P-384 length but invalid
    );
    let garbage_sig = Bytes::from(vec![0u8; 96]);
    assert!(pk.verify(garbage_sig, b"hello").is_err());
  }

  // -----------------------------------------------------------------------
  // RSA 2048 SHA-256 verify (using bundled fixture key)
  // -----------------------------------------------------------------------

  #[test]
  fn rsa_2048_sha256_valid_signature() {
    let message = b"test rsa 2048 message";
    let (pk, sig) = rsa2048_fixture(message);
    pk.verify(Bytes::from(sig), message).unwrap();
  }

  #[test]
  fn rsa_2048_sha256_rejects_tampered_message() {
    let (pk, sig) = rsa2048_fixture(b"original");
    let err = pk
      .verify(Bytes::from(sig), b"tampered")
      .unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn rsa_2048_sha256_rejects_garbage_signature() {
    let message = b"test rsa 2048 message";
    let (pk, _) = rsa2048_fixture(message);
    let garbage = Bytes::from(vec![0u8; 256]); // right length, wrong bits
    assert!(pk.verify(garbage, message).is_err());
  }

  #[test]
  fn rsa_2048_sha256_rejects_truncated_signature() {
    let message = b"test rsa 2048 message";
    let (pk, _) = rsa2048_fixture(message);
    let short = Bytes::from(vec![0u8; 10]);
    assert!(pk.verify(short, message).is_err());
  }

  #[test]
  fn rsa_rejects_corrupted_public_key() {
    // A key that isn't a valid PKCS#1 DER blob must return an error, not panic.
    let pk = PublicKey::new(
      SignatureAlgorithm::Sha256Rsa(2048),
      Bytes::from(vec![0xffu8; 32]),
    );
    let garbage_sig = Bytes::from(vec![0u8; 256]);
    assert!(pk.verify(garbage_sig, b"hello").is_err());
  }

  // -----------------------------------------------------------------------
  // Weak-RSA guard: sub-2048-bit keys must be rejected with WeakKey
  // -----------------------------------------------------------------------

  #[test]
  fn rsa_weak_key_rejected() {
    // Build a minimal 512-bit RSA public key in PKCS#1 DER format.
    // We use fixed, pre-computed values so the test runs fast without keygen.
    // These are the n and e for a 512-bit RSA key.
    use rsa::pkcs1::EncodeRsaPublicKey;

    // Use the private key bytes to derive a small key.
    // For speed, just create a small RsaPublicKey directly from known small n/e.
    // n = a small 512-bit composite (not actually secure, just for testing).
    // We use 64 bytes of 0xff for n as a placeholder big integer.
    let n = rsa::BigUint::from_bytes_be(&[0xffu8; 64]); // 512 bits
    let e = rsa::BigUint::from(65537u32);
    let pub_key = rsa::RsaPublicKey::new(n, e).unwrap();
    let pub_der = pub_key.to_pkcs1_der().unwrap();

    let pk = PublicKey::new(
      SignatureAlgorithm::Sha256Rsa(512),
      Bytes::from(pub_der.as_bytes()),
    );
    // Any signature bytes are fine — the weak-key check fires before sig check.
    let garbage_sig = Bytes::from(vec![0u8; 64]);
    let err = pk.verify(garbage_sig, b"hello").unwrap_err();
    assert!(
      matches!(err, VerificationError::WeakKey(_)),
      "expected WeakKey, got {:?}",
      err
    );
  }

  #[test]
  fn from_jwk_ec_p256_has_correct_alg() {
    use biscuit::jwk::JWK;
    use p256::ecdsa::{SigningKey};

    // Derive the same key to sign, then verify via the JWK conversion path.
    let signing_key = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
    let verifying_key = signing_key.verifying_key();

    // Serialize the verifying key to a JWK JSON string dynamically.
    let encoded = verifying_key.to_encoded_point(false);
    let x_b64 = base64::engine::Engine::encode(
      &base64::engine::general_purpose::URL_SAFE_NO_PAD,
      encoded.x().unwrap(),
    );
    let y_b64 = base64::engine::Engine::encode(
      &base64::engine::general_purpose::URL_SAFE_NO_PAD,
      encoded.y().unwrap(),
    );
    let jwk_json = format!(
      r#"{{"kty":"EC","crv":"P-256","alg":"ES256","x":"{}","y":"{}"}}"#,
      x_b64, y_b64
    );

    let jwk: JWK<()> = serde_json::from_str(&jwk_json).unwrap();
    let pk: PublicKey = jwk.into();
    assert!(matches!(pk.alg, SignatureAlgorithm::EcdsaP256));
  }

  #[test]
  fn from_jwk_ec_p384_has_correct_alg() {
    use biscuit::jwk::JWK;
    use p384::ecdsa::{SigningKey};

    let key_bytes = [5u8; 48];
    let signing_key = SigningKey::from_bytes((&key_bytes).into()).unwrap();
    let verifying_key = signing_key.verifying_key();

    let encoded = verifying_key.to_encoded_point(false);
    let x_b64 = base64::engine::Engine::encode(
      &base64::engine::general_purpose::URL_SAFE_NO_PAD,
      encoded.x().unwrap(),
    );
    let y_b64 = base64::engine::Engine::encode(
      &base64::engine::general_purpose::URL_SAFE_NO_PAD,
      encoded.y().unwrap(),
    );
    let jwk_json = format!(
      r#"{{"kty":"EC","crv":"P-384","alg":"ES384","x":"{}","y":"{}"}}"#,
      x_b64, y_b64
    );

    let jwk: JWK<()> = serde_json::from_str(&jwk_json).unwrap();
    let pk: PublicKey = jwk.into();
    assert!(matches!(pk.alg, SignatureAlgorithm::EcdsaP384));

    // Same caveat as the P-256 test: SPKI DER is stored, not raw SEC1 bytes.
    let _key_bytes = pk.key.as_ref();
  }

  #[test]
  fn from_jwk_rsa_has_correct_alg() {
    use biscuit::jwk::JWK;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{SignatureEncoding, Signer};
    use rsa::traits::PublicKeyParts;
    use sha2::Sha256;

    // Load the bundled 2048-bit private key and derive a JWK JSON from it.
    let der = include_bytes!("../test/rsa_test_private_key_2048.p8");
    let priv_key = rsa::RsaPrivateKey::from_pkcs8_der(der).unwrap();
    let pub_key = rsa::RsaPublicKey::from(&priv_key);

    // Encode n and e as base64url to build a JWK JSON string.
    let n_b64 = base64::engine::Engine::encode(
      &base64::engine::general_purpose::URL_SAFE_NO_PAD,
      pub_key.n().to_bytes_be(),
    );
    let e_b64 = base64::engine::Engine::encode(
      &base64::engine::general_purpose::URL_SAFE_NO_PAD,
      pub_key.e().to_bytes_be(),
    );
    let jwk_json = format!(
      r#"{{"kty":"RSA","alg":"RS256","n":"{}","e":"{}"}}"#,
      n_b64, e_b64
    );

    let jwk: JWK<()> = serde_json::from_str(&jwk_json).unwrap();
    let pk: PublicKey = jwk.into();
    // This RSA key is 2048 bits, so alg should be Sha256Rsa(2048).
    assert!(matches!(pk.alg, SignatureAlgorithm::Sha256Rsa(2048)));

    // Verify a real signature round-trips through the JWK conversion path.
    let signing_key = SigningKey::<Sha256>::new(priv_key);
    let message = b"rsa from jwk";
    let sig: rsa::pkcs1v15::Signature = signing_key.sign(message);
    pk.verify(Bytes::from(sig.to_vec()), message).unwrap();
  }
}
