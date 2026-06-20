use crate::Ipld;
use crate::{
  errors::VerificationError,
  verify::{is_all_unique, Verifiable},
};
use biscuit::jwk::{AlgorithmParameters, JWK};
use serde::{Deserialize, Serialize};

use super::{Mixin, V1};

/// The content field of a Chain
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChainContentV1 {
  /// The specification of the chain
  pub specification: V1,
  /// The public key of the chain
  pub key: JWK<()>,
  /// The meta data of the chain (the details)
  pub meta: Ipld,
  /// The mixins of the chain (cross-stitches)
  pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
  /// The source of the chain
  pub source: String,
  /// The radix of the chain
  pub links_radix: u32,
}

impl Verifiable for ChainContentV1 {
  type Error = VerificationError;
  fn verify(&self) -> Result<(), VerificationError> {
    if !is_all_unique(&self.mixins) {
      return Err(VerificationError::InvalidTwineFormat(
        "Contains mixins with duplicate chains".into(),
      ));
    }

    if self.links_radix == 1 {
      return Err(VerificationError::InvalidTwineFormat(
        "Chain radix must not equal 1".into(),
      ));
    }

    match self.key.algorithm {
      AlgorithmParameters::EllipticCurve(ref ec) => {
        if ec.d.is_some() {
          return Err(VerificationError::InvalidTwineFormat(
            "Can not use a private key".into(),
          ));
        }
      }
      AlgorithmParameters::RSA(ref rsa) => {
        if rsa.d.is_some() {
          return Err(VerificationError::InvalidTwineFormat(
            "Can not use a private key".into(),
          ));
        }
      }
      AlgorithmParameters::OctetKey(_) => {}
      _ => return Err(VerificationError::UnsupportedKeyAlgorithm),
    }

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::Cid;
  use serde_json::json;

  fn pub_key() -> JWK<()> {
    serde_json::from_value(json! {
      {
        "kty": "EC",
        "crv": "P-256",
        "x": "Nyf5aq1BaIfddcwuMzw9jgbc35aLYCRXlEmiuALvyJE",
        "y": "9jjHUc9ofm_5ooDhG3A2WF5gyjK7Rpw-V5mKKJ4IYKY"
      }
    })
    .unwrap()
  }

  fn private_key() -> JWK<()> {
    serde_json::from_value(json! {
      {
        "crv": "P-256",
        "d": "2LeOeNTRS9XiMGOOG7iCzV9tMRK46H9TswZuThIhy78",
        "ext": true,
        "key_ops": [
          "sign"
        ],
        "kty": "EC",
        "x": "9xMGxDMhQCSyVOQKttgkeUThPpS6HrtP6FVt5295UOA",
        "y": "J9xTVYrw8eXwBHej41mbpZeZl3eyYD5lpjP_WSGyArE"
      }
    })
    .unwrap()
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
      key: private_key(),
      meta: Ipld::Null,
      mixins: vec![],
      source: "test".into(),
      links_radix: 0,
    };

    assert!(chain.verify().is_err());
  }

  #[test]
  fn chain_content_v1_rsa_private_key_rejected() {
    // An RSA key with `d` present must be rejected with InvalidTwineFormat
    // (lines 54-56 in chain.rs).
    let rsa_private: JWK<()> = serde_json::from_value(json! {
      {
        "kty": "RSA",
        "alg": "RS256",
        "n": "zI7ywpS55pGdNZ3NwaWmFNVnYMeaxwNdAtfc8nTewwvkKJ4LE1wzYcWXebZjt_D9NtoB2BS9Lo_HYSIwfsIdTLymCdEn9iJvBANRU6ZjO_OeOIFTeCzBb-nZ7_XFXLUl8Xv2GGYFl1yZoKwWVwypcfWVKKDsUz9OxXKWZ4sq9ACwrLjY-w9U_EgqTbRSZvfZQOk1c6CbORjXNRaoVCgEU6_jzgHzWMMiDZIgTf_lRWy5vIiJJV-fd0c0XAJpAZjO1ZqzwaBMUe64KLjcLNxIV2VdeOrJbiis9s8QGVGZAYw40sk74B-OMssrftXD-_cRORR8FP4FMAaybuyQvDB8w0pqw5lHOZ3_2WkmS8tDm6X_CKFxBI6ZzO3Z4m8yEaSTK2-YrWchWlmQ4ADiGdpGCymoowEnv366zi86_Plqqla8e8vcCLkq9KGMOICVZsL4juvptOD_wEdLYBiHrSL8kLCyK7fJj2dT7eJ1S5H2UJ_SaI1jb5Y0zTY0fgHfatzmc2ZG8T0tobaC_1RtM4Y5bzm7eMqXt3S0vFlXdZhySw1_2bxW-rA1WcM2PUiIYqvaXtrHbAXDJCvZ_pLUdi98JA1TCzUuemKwu3kbROuwNiakev8vq7NDWBipo_cIOYs4GaXb3FhElzC7W4F22jHiNI_uT_wERSlQhzXnSxqIRYc",
        "e": "AQAB",
        "d": "AQAB"
      }
    }).unwrap();
    let chain = ChainContentV1 {
      specification: V1::from_string("twine/1.0.0").unwrap(),
      key: rsa_private,
      meta: Ipld::Null,
      mixins: vec![],
      source: "test".into(),
      links_radix: 0,
    };
    let result = chain.verify();
    assert!(
      result.is_err(),
      "RSA key with private component must be rejected, got Ok"
    );
    assert!(
      matches!(result, Err(VerificationError::InvalidTwineFormat(_))),
      "expected InvalidTwineFormat, got {:?}",
      result
    );
  }
}
