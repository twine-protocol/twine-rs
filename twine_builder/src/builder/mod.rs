use std::sync::Arc;
use twine_core::{
  crypto::PublicKey, errors::{SpecificationError, VerificationError}, twine::{
    Strand,
    Twine
  }
};
use crate::{signer::SigningError, Signer};

#[cfg(feature = "v1")]
use biscuit::jwk::JWK;
#[cfg(feature = "v1")]
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
  #[error("Payload construction failed: {0}")]
  PayloadConstruction(String),
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

#[cfg(feature = "v1")]
impl<S: Signer<Key = JWK<()>>> TwineBuilder<JWK<()>, S> {
  pub fn build_strand<'a>(&'a self) -> builder_v1::StrandBuilder<'a, S> {
    builder_v1::StrandBuilder::new(&self.signer)
  }

  pub fn build_first<'a>(&'a self, strand: Strand) -> builder_v1::TixelBuilder<'a, '_, S> {
    builder_v1::TixelBuilder::new_first(&self.signer, Arc::new(strand))
  }

  pub fn build_next<'a, 'b>(&'a self, prev: &'b Twine) -> builder_v1::TixelBuilder<'a, 'b, S> {
    builder_v1::TixelBuilder::new_next(&self.signer, prev)
  }
}

impl<S: Signer<Key = PublicKey>> TwineBuilder<PublicKey, S> {
  pub fn build_strand<'a>(&'a self) -> builder_v2::StrandBuilder<'a, S> {
    builder_v2::StrandBuilder::new(&self.signer)
  }

  pub fn build_first<'a>(&'a self, strand: Strand) -> builder_v2::TixelBuilder<'a, '_, S> {
    builder_v2::TixelBuilder::new_first(&self.signer, Arc::new(strand))
  }

  pub fn build_next<'a, 'b>(&'a self, prev: &'b Twine) -> builder_v2::TixelBuilder<'a, 'b, S> {
    builder_v2::TixelBuilder::new_next(&self.signer, prev)
  }
}

#[cfg(feature = "v1")]
#[allow(deprecated)]
#[cfg(test)]
mod testv1 {
  use biscuit::jws::Secret;
  use crate::BiscuitSigner;
  use twine_core::ipld_core::ipld;

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
      prev = builder.build_next(&prev)
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
  fn test_payload_builder() {
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

    let tixel = builder.build_first(strand)
      .build_payload_then_done(|_strand, _| {
        Ok("payload".to_string())
      })
      .unwrap();

    assert_eq!(tixel.extract_payload::<String>().unwrap(), "payload".to_string());
  }
}

#[allow(deprecated)]
#[cfg(test)]
mod testv2 {
  use ring::signature::Ed25519KeyPair;
  use twine_core::{ipld_core::ipld, store::MemoryStore, twine::{Twine, TwineBlock}};
  use super::*;

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

    println!("{}", &strand.tagged_dag_json());

    let mut prev = builder.build_first(strand.clone())
      .payload(ipld!({
        "baz": "qux",
      }))
      .done()
      .unwrap();

    println!("{}", &prev);

    for i in 1..10 {
      prev = builder.build_next(&prev)
        .payload(ipld!({
          "baz": "qux",
          "index": i,
        }))
        .done()
        .unwrap();

      println!("{}", &prev);
    }
  }

  #[tokio::test]
  async fn test_entwining(){
    fn make_strand(builder: &TwineBuilder<PublicKey, Ed25519KeyPair>, store: MemoryStore) -> (Strand, Twine) {
      let strand = builder.build_strand()
        .done()
        .unwrap();

      store.save_sync(strand.clone().into()).unwrap();

      let mut prev = builder.build_first(strand.clone())
        .payload(ipld!({
          "index": 0,
        }))
        .done()
        .unwrap();

      for i in 1..10 {
        let tixel = builder.build_next(&prev)
          .payload(ipld!({
            "index": i,
          }))
          .done()
          .unwrap();
        store.save_sync(tixel.clone().into()).unwrap();
        prev = tixel;
      }

      store.save_sync(strand.clone().into()).unwrap();
      (strand, prev)
    }

    let store = MemoryStore::new();
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let builder = TwineBuilder::new(key);

    let first = make_strand(&builder, store.clone());
    let second = make_strand(&builder, store.clone());
    let third = make_strand(&builder, store.clone());

    let cross_stitches = third.1.cross_stitches()
      .add_or_refresh(&first.0, &store).await.unwrap()
      .add_or_refresh(&second.0, &store).await.unwrap();

    let tixel = builder.build_next(&third.1)
      .cross_stitches(cross_stitches)
      .payload(ipld!({
        "index": 10,
      }))
      .done()
      .unwrap();

    assert!(tixel.cross_stitches().strand_is_stitched(first.0));
    assert!(tixel.cross_stitches().strand_is_stitched(second.0));
  }

  #[test]
  fn test_payload_builder_v2() {
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

    let build_payload = |message: String| {
      move |_strand: &Strand, _prev: Option<&Twine>| {
        Ok(ipld!(message))
      }
    };

    let tixel = builder.build_first(strand)
      .build_payload_then_done(build_payload("payload".to_string()))
      .unwrap();

    assert_eq!(tixel.extract_payload::<String>().unwrap(), "payload".to_string());
  }
}
