
use base64::Engine;
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

struct VerifierSelector<'a> {
  jwk: &'a Jwk,
  verifier: Option<Box<dyn JwsVerifier>>,
}

impl<'a> VerifierSelector<'a> {
  pub fn new(jwk: &'a Jwk) -> Self {
    Self {
      jwk,
      verifier: None,
    }
  }
}

impl<'a> VerifierSelector<'a> {
  pub fn select(&mut self, header: &JwsHeader) -> Result<&dyn JwsVerifier, JoseError> {
    self.verifier = get_jws_verifier(self.jwk, header)?;
    Ok(self.verifier.as_deref().map(|v| v).unwrap())
  }
}

fn get_header(input: impl AsRef<[u8]>) -> Result<JwsHeader, JoseError> {
  let input = input.as_ref();
  let indexies: Vec<usize> = input
      .iter()
      .enumerate()
      .filter(|(_, b)| **b == b'.' as u8)
      .map(|(pos, _)| pos)
      .collect();
  if indexies.len() != 2 {
    JoseError::InvalidJwsFormat(
      anyhow::anyhow!("The compact serialization form of JWS must be three parts separated by colon.")
    );
  }

  let header = &input[0..indexies[0]];
  // let payload = &input[(indexies[0] + 1)..(indexies[1])];
  // let signature = &input[(indexies[1] + 1)..];

  use josekit::{Map, Value};
  let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(header)
    .map_err(|e| JoseError::InvalidJwsFormat(anyhow::anyhow!(e.to_string())))?;
  let header: Map<String, Value> = serde_json::from_slice(&header)
    .map_err(|e| JoseError::InvalidJwsFormat(anyhow::anyhow!(e.to_string())))?;
  let header = JwsHeader::from_map(header)?;
  Ok(header)
}

pub fn verify_signature<S: AsRef<str>, P: AsRef<[u8]>>(jwk: &Jwk, signature: S, expected_payload: P) -> Result<(), VerificationError> {
  let mut selector = VerifierSelector::new(jwk);
  let header = get_header(signature.as_ref())
    .map_err(|e| VerificationError::InvalidTwineFormat(e.to_string()))?;
  let verifier = selector.select(&header)
    .map_err(|e| VerificationError::InvalidTwineFormat(e.to_string()))?;
  // this checks sig
  let (payload, _) = deserialize_compact(signature.as_ref(), verifier).map_err(|e| {
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
