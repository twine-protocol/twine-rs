use crate::Ipld;
use serde::{Serialize, Deserialize};
use biscuit::jwk::{AlgorithmParameters, JWK};
use crate::{errors::VerificationError, verify::{is_all_unique, Verifiable}};

use super::{V1, Mixin};

#[derive(Debug, Serialize, Deserialize, Clone,  PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChainContentV1 {
  pub specification: V1,
  pub key: JWK<()>,
  pub meta: Ipld,
  pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  pub source: String,
  pub links_radix: u32,
}

impl Verifiable for ChainContentV1 {
  fn verify(&self) -> Result<(), VerificationError> {
    if !is_all_unique(&self.mixins) {
      return Err(VerificationError::InvalidTwineFormat("Contains mixins with duplicate chains".into()));
    }

    if self.links_radix == 1 {
      return Err(VerificationError::InvalidTwineFormat("Chain radix must not equal 1".into()));
    }

    match self.key.algorithm {
      AlgorithmParameters::EllipticCurve(ref ec) => {
        if ec.d.is_some() {
          return Err(VerificationError::InvalidTwineFormat("Can not use a private key".into()));
        }
      },
      AlgorithmParameters::RSA(ref rsa) => {
        if rsa.d.is_some() {
          return Err(VerificationError::InvalidTwineFormat("Can not use a private key".into()));
        }
      },
      AlgorithmParameters::OctetKey(_) => {},
      _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use serde_json::json;
  use crate::Cid;

  fn pub_key() -> JWK<()> {
    serde_json::from_value(json!{
      {
        "kty": "EC",
        "crv": "P-256",
        "x": "Nyf5aq1BaIfddcwuMzw9jgbc35aLYCRXlEmiuALvyJE",
        "y": "9jjHUc9ofm_5ooDhG3A2WF5gyjK7Rpw-V5mKKJ4IYKY"
      }
    }).unwrap()
  }

  #[test]
  fn test_chain_content_v1_verify() {
    let chain = ChainContentV1 {
      specification: V1::from_string("twine/1.0.0").unwrap(),
      key: pub_key(),
      meta: Ipld::Null,
      mixins: vec![],
      source: "test".into(),
      links_radix: 0,
    };

    assert!(chain.verify().is_ok());
  }

  #[test]
  fn test_chain_content_v1_verify_duplicate_mixins() {
    let chain = ChainContentV1 {
      specification: V1::from_string("twine/1.0.0").unwrap(),
      key: pub_key(),
      meta: Ipld::Null,
      mixins: vec![
        Mixin {
          chain: Cid::default(),
          value: Cid::default(),
        },
        Mixin {
          chain: Cid::default(),
          value: Cid::default(),
        },
      ],
      source: "test".into(),
      links_radix: 0,
    };

    assert!(chain.verify().is_err());
  }

  #[test]
  fn test_chain_content_v1_verify_radix_1() {
    let chain = ChainContentV1 {
      specification: V1::from_string("twine/1.0.0").unwrap(),
      key: pub_key(),
      meta: Ipld::Null,
      mixins: vec![],
      source: "test".into(),
      links_radix: 1,
    };

    assert!(chain.verify().is_err());
  }

  #[test]
  fn test_chain_content_v1_treat_signing_key_as_invalid() {
    let chain = ChainContentV1 {
      specification: V1::from_string("twine/1.0.0").unwrap(),
      key: pub_key(),
      meta: Ipld::Null,
      mixins: vec![],
      source: "test".into(),
      links_radix: 0,
    };

    assert!(chain.verify().is_err());
  }
}
