//! Provides the interface to build Twine data.
use crate::{signer::SigningError, Signer};
use twine_lib::{
  crypto::PublicKey,
  errors::{SpecificationError, VerificationError},
  twine::{Strand, Twine},
};

#[cfg(feature = "v1")]
use biscuit::jwk::JWK;
#[cfg(feature = "v1")]
pub mod builder_v1;
pub mod builder_v2;

/// Errors that can occur when building Twine data.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
  /// Twine data verification failed.
  #[error("Bad twine data: {0}")]
  BadData(#[from] VerificationError),
  /// Invalid specification string
  #[error("Bad specification: {0}")]
  BadSpecification(#[from] SpecificationError),
  /// Problem signing the data
  #[error("Problem signing: {0}")]
  ProblemSigning(#[from] SigningError),
  /// Reached the highest index possible to represent
  #[error("Tixel index maximum reached")]
  IndexMaximum,
  /// Problem occurred when attempting to construct the payload
  #[error("Payload construction failed: {0}")]
  PayloadConstruction(String),
}

/// Provides the interface to build Strands and Tixels.
///
/// It uses the [`Signer`] it is provided for signatures, and
/// this must match the key used to construct any pre-existing data.
///
/// # Example
///
/// ```no_run
/// use twine_builder::{TwineBuilder, RingSigner};
/// let signer = RingSigner::generate_ed25519().unwrap();
/// let builder = TwineBuilder::new(signer);
///
/// // build a simple test strand
/// let strand = builder.build_strand().done().unwrap();
/// println!("{}", strand);
/// // build the first tixel
/// let first = builder.build_first(strand.clone()).done().unwrap();
/// println!("{}", first);
/// // build the next tixel
/// let next = builder.build_next(&first).done().unwrap();
/// println!("{}", next);
/// ```
pub struct TwineBuilder<const V: u8, S: Signer> {
  signer: S,
}

impl<const V: u8, S: Signer> TwineBuilder<V, S> {
  /// Create a new TwineBuilder with the provided [`Signer`].
  pub fn new(signer: S) -> Self {
    Self { signer }
  }
}

#[cfg(feature = "v1")]
impl<S: Signer<Key = JWK<()>>> TwineBuilder<1, S> {
  /// Begin building a new [`Strand`].
  ///
  /// This method is intended to be chained with the
  /// [`builder_v1::StrandBuilder`] methods to set the strand's details.
  ///
  /// Requires the `v1` feature to be enabled.
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use std::sync::Arc;
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code};
  /// use twine_builder::{TwineBuilder, BiscuitSigner};
  /// # use biscuit::jwk::JWK;
  /// # use ring::signature::*;
  /// # use biscuit::jws::Secret;
  /// # let rng = ring::rand::SystemRandom::new();
  /// # let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
  /// # let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
  /// # let secret = Secret::EcdsaKeyPair(Arc::new(key));
  /// # let biscuit_signer = BiscuitSigner::new(secret, "ES256".to_string());
  /// // ...
  /// let builder = TwineBuilder::new(biscuit_signer);
  /// let strand = builder.build_strand()
  ///   .radix(2)
  ///   .subspec("my_spec/1.0.1".to_string())
  ///   .hasher(Code::Sha3_256)
  ///   .details(ipld!({
  ///     "arbitrary": "data",
  ///   }))
  ///   .done()
  ///   .unwrap();
  /// ```
  pub fn build_strand<'a>(&'a self) -> builder_v1::StrandBuilder<'a, S> {
    builder_v1::StrandBuilder::new(&self.signer)
  }
  /// Begin building the first tixel (as a [`Twine`])
  ///
  /// This method is intended to be chained with the
  /// [`builder_v1::TixelBuilder`] methods to set the tixel's details.
  ///
  /// Requires the `v1` feature to be enabled.
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use std::sync::Arc;
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code, twine::CrossStitches};
  /// use twine_builder::{TwineBuilder, BiscuitSigner};
  /// # use biscuit::jwk::JWK;
  /// # use ring::signature::*;
  /// # use biscuit::jws::Secret;
  /// # let rng = ring::rand::SystemRandom::new();
  /// # let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
  /// # let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
  /// # let secret = Secret::EcdsaKeyPair(Arc::new(key));
  /// # let biscuit_signer = BiscuitSigner::new(secret, "ES256".to_string());
  /// // ...
  /// let builder = TwineBuilder::new(biscuit_signer);
  /// # let strand = builder.build_strand().done().unwrap();
  /// // ...
  /// let first = builder.build_first(strand)
  ///   .cross_stitches(CrossStitches::new(vec![]))
  ///   .payload(ipld!({
  ///     "arbitrary": "data",
  ///   }))
  ///   .done()
  ///   .unwrap();
  /// ```
  pub fn build_first<'a>(&'a self, strand: Strand) -> builder_v1::TixelBuilder<'a, 'a, S> {
    builder_v1::TixelBuilder::new_first(&self.signer, strand)
  }

  /// Begin building subsequent tixel (as a [`Twine`])
  ///
  /// This method is intended to be chained with the
  /// [`builder_v1::TixelBuilder`] methods to set the tixel's details.
  ///
  /// Requires the `v1` feature to be enabled.
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use std::sync::Arc;
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code, twine::CrossStitches};
  /// use twine_builder::{TwineBuilder, BiscuitSigner};
  /// # use biscuit::jwk::JWK;
  /// # use biscuit::jws::Secret;
  /// # use ring::signature::*;
  /// # let rng = ring::rand::SystemRandom::new();
  /// # let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
  /// # let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
  /// # let secret = Secret::EcdsaKeyPair(Arc::new(key));
  /// # let biscuit_signer = BiscuitSigner::new(secret, "ES256".to_string());
  /// // ...
  /// let builder = TwineBuilder::new(biscuit_signer);
  /// # let strand = builder.build_strand().done().unwrap();
  /// # let prev = builder.build_first(strand).done().unwrap();
  /// // ...
  /// let first = builder.build_next(&prev)
  ///   .cross_stitches(CrossStitches::new(vec![]))
  ///   .payload(ipld!({
  ///     "arbitrary": "data",
  ///   }))
  ///   .done()
  ///   .unwrap();
  /// ```
  pub fn build_next<'a, 'b>(&'a self, prev: &'b Twine) -> builder_v1::TixelBuilder<'a, 'b, S> {
    builder_v1::TixelBuilder::new_next(&self.signer, prev)
  }
}

impl<S: Signer<Key = PublicKey>> TwineBuilder<2, S> {
  /// Begin building a new [`Strand`].
  ///
  /// This method is intended to be chained with the
  /// [`builder_v2::StrandBuilder`] methods to set the strand's details.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code};
  /// use twine_builder::{TwineBuilder, RingSigner};
  /// let signer = RingSigner::generate_ed25519().unwrap();
  /// let builder = TwineBuilder::new(signer);
  /// let strand = builder.build_strand()
  ///   .details(ipld!({
  ///     "arbitrary": "data",
  ///   }))
  ///   .done()
  ///   .unwrap();
  /// ```
  pub fn build_strand<'a>(&'a self) -> builder_v2::StrandBuilder<'a, S> {
    builder_v2::StrandBuilder::new(&self.signer)
  }

  /// Begin building the first tixel (as a [`Twine`])
  ///
  /// This method is intended to be chained with the
  /// [`builder_v2::TixelBuilder`] methods to set the tixel's details.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code, twine::CrossStitches};
  /// use twine_builder::{TwineBuilder, RingSigner};
  /// let signer = RingSigner::generate_ed25519().unwrap();
  /// let builder = TwineBuilder::new(signer);
  /// let strand = builder.build_strand().done().unwrap();
  /// let first = builder.build_first(strand)
  ///   .cross_stitches(CrossStitches::new(vec![]))
  ///   .payload(ipld!({
  ///     "arbitrary": "data",
  ///    }))
  ///    .done()
  ///    .unwrap();
  /// ```
  pub fn build_first<'a>(&'a self, strand: Strand) -> builder_v2::TixelBuilder<'a, 'a, S> {
    builder_v2::TixelBuilder::new_first(&self.signer, strand)
  }

  /// Begin building subsequent tixel (as a [`Twine`])
  ///
  /// This method is intended to be chained with the
  /// [`builder_v2::TixelBuilder`] methods to set the tixel's details.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code, twine::CrossStitches};
  /// use twine_builder::{TwineBuilder, RingSigner};
  /// let signer = RingSigner::generate_ed25519().unwrap();
  /// let builder = TwineBuilder::new(signer);
  /// let strand = builder.build_strand().done().unwrap();
  /// let prev = builder.build_first(strand).done().unwrap();
  /// let next = builder.build_next(&prev)
  ///   .cross_stitches(CrossStitches::new(vec![]))
  ///   .payload(ipld!({
  ///     "arbitrary": "data",
  ///    }))
  ///    .done()
  ///    .unwrap();
  /// ```
  pub fn build_next<'a, 'b>(&'a self, prev: &'b Twine) -> builder_v2::TixelBuilder<'a, 'b, S> {
    builder_v2::TixelBuilder::new_next(&self.signer, prev)
  }
}

#[cfg(feature = "v1")]
#[allow(deprecated)]
#[cfg(test)]
mod testv1 {
  use crate::BiscuitSigner;
  use biscuit::jws::Secret;
  use std::sync::Arc;
  use twine_lib::ipld_core::ipld;

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
    let strand = builder
      .build_strand()
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
    let strand = builder
      .build_strand()
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
    let strand = builder
      .build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .radix(2)
      .done()
      .unwrap();

    let mut prev = builder
      .build_first(strand.clone())
      .payload(ipld!({
        "baz": "qux",
      }))
      .done()
      .unwrap();

    for i in 1..10 {
      prev = builder
        .build_next(&prev)
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
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct Timestamped {
      timestamp: String,
    }

    let key = ec_key(&ECDSA_P256_SHA256_FIXED_SIGNING);
    let secret = Secret::EcdsaKeyPair(Arc::new(key));
    let signer = BiscuitSigner::new(secret, "ES256".to_string());
    let builder = TwineBuilder::new(signer);
    let strand = builder
      .build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done()
      .unwrap();

    let my_struct = Timestamped {
      timestamp: "2023-10-26T21:25:56.936Z".to_string(),
    };

    let tixel = builder
      .build_first(strand)
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
    let strand = builder
      .build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done()
      .unwrap();

    let tixel = builder
      .build_first(strand)
      .build_payload_then_done(|_strand, _| Ok("payload".to_string()))
      .unwrap();

    assert_eq!(
      tixel.extract_payload::<String>().unwrap(),
      "payload".to_string()
    );
  }
}

#[allow(deprecated)]
#[cfg(test)]
mod testv2 {
  use super::*;
  use ring::signature::Ed25519KeyPair;
  use twine_lib::{
    ipld_core::ipld,
    store::MemoryStore,
    twine::{CrossStitches, Twine, TwineBlock},
  };

  #[test]
  fn test_v2() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let builder = TwineBuilder::new(key);
    let strand = builder
      .build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done()
      .unwrap();

    println!("{}", &strand.tagged_dag_json());

    let mut prev = builder
      .build_first(strand.clone())
      .payload(ipld!({
        "baz": "qux",
      }))
      .done()
      .unwrap();

    println!("{}", &prev);

    for i in 1..10 {
      prev = builder
        .build_next(&prev)
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
  async fn test_entwining() {
    fn make_strand(
      builder: &TwineBuilder<2, Ed25519KeyPair>,
      store: MemoryStore,
    ) -> (Strand, Twine) {
      let strand = builder.build_strand().done().unwrap();

      store.save_sync(strand.clone().into()).unwrap();

      let mut prev = builder
        .build_first(strand.clone())
        .payload(ipld!({
          "index": 0,
        }))
        .done()
        .unwrap();

      for i in 1..10 {
        let tixel = builder
          .build_next(&prev)
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

    let cross_stitches = third
      .1
      .cross_stitches()
      .add_or_refresh(&first.0, &store)
      .await
      .unwrap()
      .add_or_refresh(&second.0, &store)
      .await
      .unwrap();

    let tixel = builder
      .build_next(&third.1)
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
    let strand = builder
      .build_strand()
      .details(ipld!({
        "foo": "bar",
      }))
      .done()
      .unwrap();

    let build_payload =
      |message: String| move |_strand: &Strand, _prev: Option<&Twine>| Ok(ipld!(message));

    let tixel = builder
      .build_first(strand)
      .build_payload_then_done(build_payload("payload".to_string()))
      .unwrap();

    assert_eq!(
      tixel.extract_payload::<String>().unwrap(),
      "payload".to_string()
    );
  }

  #[test]
  fn test_deny_stitches_to_self() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let builder = TwineBuilder::new(key);
    let strand = builder
      .build_strand()
      .details("a".to_string())
      .done()
      .unwrap();

    let t_a1 = builder
      .build_first(strand.clone())
      .payload("a1".to_string())
      .done()
      .unwrap();

    let res = builder
      .build_next(&t_a1)
      .payload("a2".to_string())
      .cross_stitches(CrossStitches::new(vec![t_a1.clone().into()]))
      .done();

    assert!(res.is_err());
  }

  #[test]
  fn test_dropped_stitch() {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let builder = TwineBuilder::new(key);
    let strand_a = builder
      .build_strand()
      .details("a".to_string())
      .done()
      .unwrap();

    let t_a1 = builder
      .build_first(strand_a.clone())
      .payload("a1".to_string())
      .done()
      .unwrap();

    let strand_b = builder
      .build_strand()
      .details("b".to_string())
      .done()
      .unwrap();

    let t_b1 = builder
      .build_first(strand_b.clone())
      .payload("b1".to_string())
      .done()
      .unwrap();

    let strand_c = builder
      .build_strand()
      .details("c".to_string())
      .done()
      .unwrap();

    let t_c1 = builder
      .build_first(strand_c.clone())
      .payload("c1".to_string())
      .cross_stitches(CrossStitches::new(vec![
        t_a1.clone().into(),
        t_b1.clone().into(),
      ]))
      .done()
      .unwrap();

    let t_c2 = builder
      .build_next(&t_c1)
      .payload("c2".to_string())
      .cross_stitches(CrossStitches::new(vec![t_a1.clone().into()]))
      .done()
      .unwrap();

    assert!(t_c1.includes(&t_a1));
    assert!(t_c1.includes(&t_b1));
    assert!(t_c2.includes(&t_a1));
    assert!(!t_c2.includes(&t_b1));
    assert!(t_c2.drop_index() == 1);
    assert!(t_c2.cross_stitches().len() == 1);
  }
}
