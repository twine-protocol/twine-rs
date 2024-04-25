use crate::schemas::v1;
use josekit::{jws::deserialize_compact, jwk::Jwk, jws::JwsVerifier, JoseError};
use serde::{Serialize, Deserialize};
use super::{container::TwineContainer, Tixel};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum StrandContent {
  V1(v1::ChainContentV1),
}

pub type Strand = TwineContainer<StrandContent>;

impl Strand {
  pub fn verify_signature(&self, tixel: &Tixel) -> Result<(), JoseError> {
    self.content().verify_signature(tixel)
  }

  pub fn verify_own_signature(&self) -> Result<(), JoseError> {
    self.content().verify_signature(self)
  }
}

impl StrandContent {
  pub fn key(&self) -> Jwk {
    match self {
      StrandContent::V1(v) => v.key.clone(),
    }
  }

  pub fn verifier(&self) -> Result<Box<dyn JwsVerifier + 'static>, JoseError> {
    let jwk = self.key();

    use josekit::jws::*;

    // Try every algorithm ...
    // ... this is crazy
    let rsa = vec![
      RS256,
      RS384,
      RS512,
    ];

    for alg in rsa {
      let verifier = alg.verifier_from_jwk(&jwk);
      if verifier.is_ok() {
        return verifier.map(|v| Box::new(v) as Box<dyn JwsVerifier + 'static>);
      }
    }

    let ecdsa = vec![
      ES256,
      ES256K,
      ES384,
      ES512
    ];

    for alg in ecdsa {
      let verifier = alg.verifier_from_jwk(&jwk);
      if verifier.is_ok() {
        return verifier.map(|v| Box::new(v) as Box<dyn JwsVerifier + 'static>);
      }
    }

    let pss = vec![
      PS256,
      PS384,
      PS512
    ];

    for alg in pss {
      let verifier = alg.verifier_from_jwk(&jwk);
      if verifier.is_ok() {
        return verifier.map(|v| Box::new(v) as Box<dyn JwsVerifier + 'static>);
      }
    }

    let verifier = EdDSA.verifier_from_jwk(&jwk);
    if verifier.is_ok() {
      return verifier.map(|v| Box::new(v) as Box<dyn JwsVerifier + 'static>);
    }

    Err(JoseError::UnsupportedSignatureAlgorithm(anyhow::anyhow!("Unsupported signature algorithm")))
  }

  pub fn verify_signature<C: Clone + Serialize + for<'de> Deserialize<'de>>(&self, twine: &TwineContainer<C>) -> Result<(), JoseError> {
    let verifier = self.verifier()?;
    // this checks sig
    let (payload, _) = deserialize_compact(twine.signature(), verifier.as_ref())?;
    // check the content hash
    let content_hash = twine.content_hash();
    if content_hash != payload {
      return Err(JoseError::InvalidSignature(anyhow::anyhow!("JWS payload field does not match content digest")));
    }
    Ok(())
  }
}

