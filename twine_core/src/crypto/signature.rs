
use josekit::{jwk::Jwk, jws::{JwsVerifier, *}, JoseError};
use crate::errors::VerificationError;

fn get_jws_verifier(jwk: &Jwk, header: &JwsHeader) -> Result<Option<Box<dyn JwsVerifier>>, JoseError> {
  match header.algorithm() {
    Some("RS256") => Ok(Some(Box::new(RS256.verifier_from_jwk(jwk)?))),
    Some("RS384") => Ok(Some(Box::new(RS384.verifier_from_jwk(jwk)?))),
    Some("RS512") => Ok(Some(Box::new(RS512.verifier_from_jwk(jwk)?))),
    Some("ES256") => Ok(Some(Box::new(ES256.verifier_from_jwk(jwk)?))),
    Some("ES256K") => Ok(Some(Box::new(ES256K.verifier_from_jwk(jwk)?))),
    Some("ES384") => Ok(Some(Box::new(ES384.verifier_from_jwk(jwk)?))),
    Some("ES512") => Ok(Some(Box::new(ES512.verifier_from_jwk(jwk)?))),
    Some("EdDSA") => Ok(Some(Box::new(EdDSA.verifier_from_jwk(jwk)?))),
    _ => Ok(None),
  }
}

pub fn verify_signature<S: AsRef<str>, P: AsRef<[u8]>>(jwk: &Jwk, signature: S, expected_payload: P) -> Result<(), VerificationError> {
  let selector = |header: &JwsHeader| -> Result<Option<&dyn JwsVerifier>, JoseError> {
    get_jws_verifier(jwk, header).map(|v| v.map(|v| {
      let leaked: &dyn JwsVerifier = Box::leak(v);
      leaked
    }))
  };
  // this checks sig
  let (payload, _) = deserialize_compact_with_selector(signature.as_ref(), selector).map_err(|e| {
    match e {
      JoseError::InvalidSignature(e) => VerificationError::BadSignature(e.to_string()),
      _ => VerificationError::InvalidTwineFormat("Signature was not formatted as compact JWS".to_string()),
    }
  })?;
  // check the content hash
  if expected_payload.as_ref() != payload {
    return Err(VerificationError::BadSignature("Payload does not match signature".to_string()));
  }
  Ok(())
}
