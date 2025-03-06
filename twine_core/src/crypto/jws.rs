use crate::errors::VerificationError;

use biscuit::{
  jwk::{JWKSet, JWK},
  jws,
};

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
