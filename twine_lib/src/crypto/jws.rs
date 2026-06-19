//! JSON Web Signature (JWS) utilities
use crate::errors::VerificationError;

use biscuit::{
  jwk::{JWKSet, JWK},
  jws,
};

/// Verify a JWS signature with a JWK
pub fn verify_signature<T: Clone, S: AsRef<str>, P: AsRef<[u8]>>(
  jwk: &JWK<T>,
  signature: S,
  expected_payload: P,
) -> Result<(), VerificationError> {
  let keys = JWKSet {
    keys: vec![jwk.clone()],
  };
  jws::Compact::<Vec<u8>, biscuit::Empty>::new_encoded(signature.as_ref())
    .decode_with_jwks_ignore_kid(&keys)
    .map_err(|e| VerificationError::BadSignature(e.to_string()))?
    .payload()
    .map_err(|e| VerificationError::BadSignature(e.to_string()))?
    .eq(expected_payload.as_ref())
    .then(|| ())
    .ok_or(VerificationError::BadSignature("Payload mismatch".into()))?;
  Ok(())
}

#[cfg(test)]
mod test {
  use super::*;
  use biscuit::{
    jwa::SignatureAlgorithm as JwaAlg,
    jwk::{AlgorithmParameters, CommonParameters, OctetKeyParameters, OctetKeyType, JWK},
    jws::{Compact, Header, RegisteredHeader, Secret},
    Empty,
  };

  // -----------------------------------------------------------------------
  // Helper: create an HMAC-HS256 JWK and a signed JWS token for a payload
  // -----------------------------------------------------------------------

  /// Builds a minimal `oct` (HMAC) JWK from a raw secret.
  fn hmac_jwk(secret: &[u8]) -> JWK<()> {
    JWK {
      common: CommonParameters {
        algorithm: Some(biscuit::jwa::Algorithm::Signature(JwaAlg::HS256)),
        ..Default::default()
      },
      algorithm: AlgorithmParameters::OctetKey(OctetKeyParameters {
        key_type: OctetKeyType::Octet,
        value: secret.to_vec(),
      }),
      additional: (),
    }
  }

  /// Sign `payload` with HMAC-HS256 using `secret` and return the compact JWS.
  fn sign_hs256(payload: &[u8], secret: &[u8]) -> String {
    let header = Header::<Empty>::from(RegisteredHeader {
      algorithm: JwaAlg::HS256,
      ..Default::default()
    });
    let token: Compact<Vec<u8>, Empty> = Compact::new_decoded(header, payload.to_vec());
    let encoded = token
      .encode(&Secret::Bytes(secret.to_vec()))
      .unwrap();
    match encoded {
      Compact::Encoded(compact) => compact.encode(),
      _ => panic!("expected encoded token"),
    }
  }

  // -----------------------------------------------------------------------
  // Valid round-trip
  // -----------------------------------------------------------------------

  #[test]
  fn verify_signature_valid_hs256() {
    let secret = b"super-secret-test-key";
    let payload = b"hello twine";
    let jwk = hmac_jwk(secret);
    let token = sign_hs256(payload, secret);
    verify_signature(&jwk, &token, payload).unwrap();
  }

  // -----------------------------------------------------------------------
  // Bad signature: token tampered after signing
  // -----------------------------------------------------------------------

  #[test]
  fn verify_signature_bad_signature() {
    let secret = b"super-secret-test-key";
    let payload = b"hello twine";
    let jwk = hmac_jwk(secret);
    let token = sign_hs256(payload, secret);

    // Swap one char in the signature part (the third dot-separated segment).
    let mut parts: Vec<String> = token.splitn(3, '.').map(String::from).collect();
    let sig = parts[2].as_bytes();
    // Flip the first byte of the base64url signature.
    let first = if sig[0] == b'A' { b'B' } else { b'A' };
    parts[2] = String::from_utf8(
      std::iter::once(first)
        .chain(sig[1..].iter().copied())
        .collect(),
    )
    .unwrap();
    let tampered = parts.join(".");

    let err = verify_signature(&jwk, &tampered, payload).unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  // -----------------------------------------------------------------------
  // Payload mismatch: signature valid but payload doesn't match expected
  // -----------------------------------------------------------------------

  #[test]
  fn verify_signature_payload_mismatch() {
    let secret = b"super-secret-test-key";
    let payload = b"hello twine";
    let jwk = hmac_jwk(secret);
    let token = sign_hs256(payload, secret);

    // Signature is valid but we check against a different expected payload.
    let err = verify_signature(&jwk, &token, b"wrong expected payload").unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  // -----------------------------------------------------------------------
  // Garbage input: not a valid JWS compact token
  // -----------------------------------------------------------------------

  #[test]
  fn verify_signature_garbage_token() {
    let secret = b"super-secret-test-key";
    let jwk = hmac_jwk(secret);
    let err = verify_signature(&jwk, "notavalidtoken", b"payload").unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  #[test]
  fn verify_signature_empty_token() {
    let secret = b"super-secret-test-key";
    let jwk = hmac_jwk(secret);
    let err = verify_signature(&jwk, "", b"payload").unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }

  // -----------------------------------------------------------------------
  // Wrong key: valid JWS but verified against the wrong HMAC key
  // -----------------------------------------------------------------------

  #[test]
  fn verify_signature_wrong_key() {
    let payload = b"sensitive data";
    let token = sign_hs256(payload, b"correct-key");
    // Build a JWK for a *different* key.
    let wrong_jwk = hmac_jwk(b"wrong-key");
    let err = verify_signature(&wrong_jwk, &token, payload).unwrap_err();
    assert!(matches!(err, VerificationError::BadSignature(_)));
  }
}
