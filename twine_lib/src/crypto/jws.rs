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

  // -----------------------------------------------------------------------
  // Algorithm-confusion / downgrade attacks against a real RSA strand key
  //
  // These probe the classic JWS pitfalls: an attacker who knows the public
  // key (it's published in the strand) tries to forge a signature by
  // changing the header `alg`. If verification trusts the token's `alg`
  // instead of the JWK's, the log's authenticity guarantee collapses.
  // -----------------------------------------------------------------------

  /// The real RSA public-key JWK published in the v1 strand fixture.
  fn strand_rsa_jwk() -> JWK<()> {
    let v: serde_json::Value = serde_json::from_str(crate::test::STRANDJSON).unwrap();
    serde_json::from_value(v["data"]["content"]["key"].clone()).unwrap()
  }

  #[test]
  fn rejects_alg_none_token_against_rsa_key() {
    let jwk = strand_rsa_jwk();
    let payload = b"content hash bytes";

    // Forge an unsecured (alg:"none") token carrying the target payload.
    let header = Header::<Empty>::from(RegisteredHeader {
      algorithm: JwaAlg::None,
      ..Default::default()
    });
    let token: Compact<Vec<u8>, Empty> = Compact::new_decoded(header, payload.to_vec());
    let forged = match token.encode(&Secret::None).unwrap() {
      Compact::Encoded(c) => c.encode(),
      _ => panic!("expected encoded token"),
    };

    // Must NOT verify: an RSA strand key must never accept an unsigned token.
    let res = verify_signature(&jwk, &forged, payload);
    assert!(
      res.is_err(),
      "alg:none token was accepted against an RSA key -- auth bypass!"
    );
  }

  #[test]
  fn rejects_hs256_confusion_using_public_modulus_as_secret() {
    let jwk = strand_rsa_jwk();
    let payload = b"content hash bytes";

    // The attacker uses the public RSA modulus (known to everyone) as an
    // HMAC secret and signs an HS256 token over the target payload.
    let modulus = match &jwk.algorithm {
      AlgorithmParameters::RSA(params) => params.n.to_bytes_be(),
      _ => panic!("expected RSA key params"),
    };
    let forged = sign_hs256(payload, &modulus);

    // Must NOT verify: HS256-with-public-key confusion must be rejected.
    let res = verify_signature(&jwk, &forged, payload);
    assert!(
      res.is_err(),
      "HS256 confusion token verified against RSA key -- auth bypass!"
    );
  }

  #[test]
  fn rejects_confusion_even_when_jwk_omits_alg() {
    // A strand could publish an RSA key without the optional `alg` member.
    // If verification then trusts the token header's `alg`, alg:none and
    // HS256-confusion forgeries become possible. Pin behaviour with a test.
    let mut jwk = strand_rsa_jwk();
    jwk.common.algorithm = None;
    let payload = b"content hash bytes";

    // alg:none
    let header = Header::<Empty>::from(RegisteredHeader {
      algorithm: JwaAlg::None,
      ..Default::default()
    });
    let token: Compact<Vec<u8>, Empty> = Compact::new_decoded(header, payload.to_vec());
    let none_token = match token.encode(&Secret::None).unwrap() {
      Compact::Encoded(c) => c.encode(),
      _ => panic!("expected encoded token"),
    };
    assert!(
      verify_signature(&jwk, &none_token, payload).is_err(),
      "alg:none accepted for an RSA key with no `alg` -- auth bypass!"
    );

    // HS256 confusion with the public modulus as secret
    let modulus = match &jwk.algorithm {
      AlgorithmParameters::RSA(params) => params.n.to_bytes_be(),
      _ => panic!("expected RSA key params"),
    };
    let hs_token = sign_hs256(payload, &modulus);
    assert!(
      verify_signature(&jwk, &hs_token, payload).is_err(),
      "HS256 confusion accepted for an RSA key with no `alg` -- auth bypass!"
    );
  }
}
