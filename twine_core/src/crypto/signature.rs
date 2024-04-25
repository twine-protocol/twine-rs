
use josekit::{jwk::Jwk, jws::{JwsVerifier, *}, JoseError};
use crate::errors::VerificationError;

fn map_key_error(e: JoseError) -> VerificationError {
  VerificationError::MalformedJwk(e.into())
}

fn verifier_from_rsa_type(jwk: &Jwk) -> Result<Box<dyn JwsVerifier + 'static>, VerificationError> {
  // by the time it gets here we know it's an RSA key
  assert_eq!(jwk.key_type(), "RSA");

  let verifier = match jwk.algorithm() {
    Some("RS256") => RS256.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    Some("RS384") => RS384.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    Some("RS512") => RS512.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
  };

  Ok(Box::new(verifier) as Box<dyn JwsVerifier + 'static>)
}

fn verifier_from_ecdsa_type(jwk: &Jwk) -> Result<Box<dyn JwsVerifier + 'static>, VerificationError> {
  // by the time it gets here we know it's an ECDSA key
  assert_eq!(jwk.key_type(), "EC");

  let verifier = match jwk.algorithm() {
    Some("ES256") => ES256.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    Some("ES256K") => ES256K.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    Some("ES384") => ES384.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    Some("ES512") => ES512.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
  };

  Ok(Box::new(verifier) as Box<dyn JwsVerifier + 'static>)
}

fn verifier_from_eddsa_type(jwk: &Jwk) -> Result<Box<dyn JwsVerifier + 'static>, VerificationError> {
  // by the time it gets here we know it's an EdDSA key
  assert_eq!(jwk.key_type(), "OKP");

  let verifier = match jwk.algorithm() {
    Some("EdDSA") => EdDSA.verifier_from_jwk(&jwk).map_err(map_key_error)?,
    _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
  };

  Ok(Box::new(verifier) as Box<dyn JwsVerifier + 'static>)
}

pub fn get_jws_verifier(jwk: Jwk) -> Result<Box<dyn JwsVerifier + 'static>, VerificationError> {
  match jwk.key_type() {
    "RSA" => verifier_from_rsa_type(&jwk),
    "EC" => verifier_from_ecdsa_type(&jwk),
    "OKP" => verifier_from_eddsa_type(&jwk),
    _ => Err(VerificationError::UnsupportedKeyAlgorithm),
  }
}

pub fn verify_signature<S: AsRef<str>, P: AsRef<[u8]>>(jwk: Jwk, signature: S, expected_payload: P) -> Result<(), VerificationError> {
  let verifier = get_jws_verifier(jwk)?;
  // this checks sig
  let (payload, _) = deserialize_compact(signature.as_ref(), verifier.as_ref()).map_err(|e| {
    match e {
      JoseError::InvalidSignature(_) => VerificationError::BadSignature,
      _ => VerificationError::BadSignatureFormat,
    }
  })?;
  // check the content hash
  if expected_payload.as_ref() != payload {
    return Err(VerificationError::BadSignature);
  }
  Ok(())
}
