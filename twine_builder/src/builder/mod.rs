use std::sync::Arc;
use twine_core::{
  crypto::PublicKey, errors::{SpecificationError, VerificationError}, twine::{
    Strand,
    Twine
  }
};
use crate::{signer::SigningError, Signer};
use biscuit::jwk::JWK;

mod builder_v1;
mod builder_v2;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
  #[error("Bad twine data: {0}")]
  BadData(#[from] VerificationError),
  #[error("Bad specification: {0}")]
  BadSpecification(#[from] SpecificationError),
  #[error("Problem signing: {0}")]
  ProblemSigning(#[from] SigningError),
  #[error("Tixel index maximum reached")]
  IndexMaximum,
}

pub struct TwineBuilder<T, S: Signer<Key = T>> {
  signer: S,
  _phantom: std::marker::PhantomData<T>,
}

impl<T, S: Signer<Key = T>> TwineBuilder<T, S> {
  pub fn new(signer: S) -> Self {
    Self {
      signer,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<S: Signer<Key = JWK<()>>> TwineBuilder<JWK<()>, S> {
  pub fn build_strand<'a>(&'a self) -> builder_v1::StrandBuilder<'a, S> {
    builder_v1::StrandBuilder::new(&self.signer)
  }

  pub fn build_first<'a>(&'a self, strand: Strand) -> builder_v1::TixelBuilder<'a, S> {
    builder_v1::TixelBuilder::new_first(&self.signer, Arc::new(strand))
  }

  pub fn build_next<'a>(&'a self, prev: Twine) -> builder_v1::TixelBuilder<'a, S> {
    builder_v1::TixelBuilder::new_next(&self.signer, prev)
  }
}

impl<S: Signer<Key = PublicKey>> TwineBuilder<PublicKey, S> {
  pub fn build_strand<'a>(&'a self) -> builder_v2::StrandBuilder<'a, S> {
    builder_v2::StrandBuilder::new(&self.signer)
  }

  pub fn build_first<'a>(&'a self, strand: Strand) -> builder_v2::TixelBuilder<'a, S> {
    builder_v2::TixelBuilder::new_first(&self.signer, Arc::new(strand))
  }

  pub fn build_next<'a>(&'a self, prev: Twine) -> builder_v2::TixelBuilder<'a, S> {
    builder_v2::TixelBuilder::new_next(&self.signer, prev)
  }
}


#[cfg(test)]
mod test {
  use biscuit::jws::Secret;
  use twine_core::{ipld_core::ipld, twine::TwineBlock};
  use crate::BiscuitSigner;

  use super::*;
  use ring::signature::*;

  fn ec_key(alg: &'static EcdsaSigningAlgorithm) -> EcdsaKeyPair {
    let rng = ring::rand::SystemRandom::new();
    let pkcs = EcdsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
    EcdsaKeyPair::from_pkcs8(alg, pkcs.as_ref(), &rng).unwrap()
  }

  #[test]
  fn test_build_es256() {
    let key = ec_key(&ECDSA_P256_SHA256_FIXED_SIGNING);
    let secret = Secret::EcdsaKeyPair(Arc::new(key));
    let signer = BiscuitSigner::new(secret, "ES256".to_string());
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
  }

  #[test]
  fn test_build_es384() {
    let key = ec_key(&ECDSA_P384_SHA384_FIXED_SIGNING);
    let secret = Secret::EcdsaKeyPair(Arc::new(key));
    let signer = BiscuitSigner::new(secret, "ES384".to_string());
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
  }

  // #[test]
  // fn test_build_ed25519() {
  //   let signer = jwk::Jwk::generate_ed_key(jwk::alg::ed::EdCurve::Ed25519).unwrap();
  //   let builder = TwineBuilder::new(signer);
  //   let strand = builder.build_strand()
  //     .version("1.0.0".to_string())
  //     .details(ipld!({
  //       "foo": "bar",
  //     }))
  //     .done();

  //   assert!(strand.is_ok(), "{}", strand.unwrap_err());
  //   assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  // }

  // #[test]
  // fn test_build_ed448() {
  //   let signer = jwk::Jwk::generate_ed_key(jwk::alg::ed::EdCurve::Ed448).unwrap();
  //   let builder = TwineBuilder::new(signer);
  //   let strand = builder.build_strand()
  //     .version("1.0.0".to_string())
  //     .details(ipld!({
  //       "foo": "bar",
  //     }))
  //     .done();

  //   assert!(strand.is_ok(), "{}", strand.unwrap_err());
  //   assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  // }

  // #[test]
  // fn test_build_rsa() {
  //   let rng = ring::rand::SystemRandom::new();
  //   let pkcs = RsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
  //   let key = RsaKeyPair::from_pkcs8(alg, pkcs.as_ref(), &rng).unwrap()

  //   let builder = TwineBuilder::new(signer);
  //   let strand = builder.build_strand()
  //     .version("1.0.0".to_string())
  //     .details(ipld!({
  //       "foo": "bar",
  //     }))
  //     .done();

  //   assert!(strand.is_ok(), "{}", strand.unwrap_err());
  // }

  #[test]
  fn text_build_tixels() {
    let key = ec_key(&ECDSA_P256_SHA256_FIXED_SIGNING);
    let secret = Secret::EcdsaKeyPair(Arc::new(key));
    let signer = BiscuitSigner::new(secret, "ES256".to_string());
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .radix(2)
      .done()
      .unwrap();

    let mut prev = builder.build_first(strand.clone())
      .payload(ipld!({
        "baz": "qux",
      }))
      .done()
      .unwrap();

    for i in 1..10 {
      prev = builder.build_next(prev)
        .payload(ipld!({
          "baz": "qux",
          "index": i,
        }))
        .done()
        .unwrap();
    }
  }

  #[test]
  fn test_struct_payload() {
    use serde::{Serialize, Deserialize};
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct Timestamped {
      timestamp: String,
    }

    let key = ec_key(&ECDSA_P256_SHA256_FIXED_SIGNING);
    let secret = Secret::EcdsaKeyPair(Arc::new(key));
    let signer = BiscuitSigner::new(secret, "ES256".to_string());
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done()
      .unwrap();

    let my_struct = Timestamped {
      timestamp: "2023-10-26T21:25:56.936Z".to_string(),
    };

    let tixel = builder.build_first(strand)
      .payload(my_struct)
      .done()
      .unwrap();

    let t: Timestamped = tixel.extract_payload().unwrap();
    assert_eq!(t.timestamp, "2023-10-26T21:25:56.936Z");
  }

  #[test]
  fn test_v2() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let builder = TwineBuilder::new(key);
    let strand = builder.build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done()
      .unwrap();

    println!("{}", &strand.dag_json());

    let mut prev = builder.build_first(strand.clone())
      .payload(ipld!({
        "baz": "qux",
      }))
      .done()
      .unwrap();

    println!("{}", &prev);

    for i in 1..10 {
      prev = builder.build_next(prev)
        .payload(ipld!({
          "baz": "qux",
          "index": i,
        }))
        .done()
        .unwrap();

      println!("{}", &prev);
    }
  }
}
