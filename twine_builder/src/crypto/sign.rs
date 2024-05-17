use anyhow::anyhow;
use josekit::{jwk::Jwk, jws::{JwsSigner, *}, JoseError};

use crate::signer::SigningError;

fn get_jws_signer(jwk: &Jwk, header: &JwsHeader) -> Result<Box<dyn JwsSigner>, JoseError> {
  match header.algorithm() {
    Some("RS256") => Ok(Box::new(RS256.signer_from_jwk(jwk)?)),
    Some("RS384") => Ok(Box::new(RS384.signer_from_jwk(jwk)?)),
    Some("RS512") => Ok(Box::new(RS512.signer_from_jwk(jwk)?)),
    Some("ES256") => Ok(Box::new(ES256.signer_from_jwk(jwk)?)),
    Some("ES256K") => Ok(Box::new(ES256K.signer_from_jwk(jwk)?)),
    Some("ES384") => Ok(Box::new(ES384.signer_from_jwk(jwk)?)),
    Some("ES512") => Ok(Box::new(ES512.signer_from_jwk(jwk)?)),
    Some("EdDSA") => Ok(Box::new(EdDSA.signer_from_jwk(jwk)?)),
    _ => return Err(JoseError::UnsupportedSignatureAlgorithm(anyhow!("Unsupported algorithm {}", header.algorithm().unwrap_or("none")))),
  }
}

fn get_header(jwk: &Jwk) -> JwsHeader {
  let alg = match jwk.key_type() {
    "RSA" => {
      match jwk.curve() {
        Some("P-256") => "RS256",
        Some("P-384") => "RS384",
        Some("P-521") => "RS512",
        _ => "RS256",
      }
    },
    "EC" => {
      match jwk.curve() {
        Some("P-256") => "ES256",
        Some("P-384") => "ES384",
        Some("P-521") => "ES512",
        _ => "ES256",
      }
    },
    "OKP" => {
      match jwk.curve() {
        Some("Ed25519") => "EdDSA",
        _ => "EdDSA",
      }
    },
    _ => panic!("Unsupported key type")
  };
  let mut h = JwsHeader::new();
  h.set_algorithm(alg);
  h
}

struct SignerSelector<'a> {
  jwk: &'a Jwk,
  signer: Option<Box<dyn JwsSigner>>,
}

impl<'a> SignerSelector<'a> {
  pub fn new(jwk: &'a Jwk) -> Self {
    Self {
      jwk,
      signer: None,
    }
  }

  pub fn select(&mut self, header: &JwsHeader) -> Result<&dyn JwsSigner, JoseError> {
    self.signer = Some(get_jws_signer(self.jwk, header)?);
    Ok(self.signer.as_deref().map(|v| v).unwrap())
  }
}

pub fn sign<P: AsRef<[u8]>>(jwk: &Jwk, payload: P) -> Result<String, SigningError> {
  let mut selector = SignerSelector::new(jwk);
  let header = get_header(jwk);
  let signer = selector.select(&header)
    .map_err(|e| SigningError(e.to_string()))?;
  serialize_compact(payload.as_ref(), &header, signer)
    .map_err(|e| SigningError(format!("Could not sign: {}", e)))
}
