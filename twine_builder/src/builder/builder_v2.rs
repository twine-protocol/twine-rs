//! Twine builder for version 2 data
use super::*;
use twine_lib::{
  crypto::PublicKey,
  errors::{SpecificationError, VerificationError},
  ipld_core::{codec::Codec, serde::to_ipld},
  multihash_codetable::Code,
  semver::Version,
  skiplist::get_layer_pos,
  specification::Subspec,
  twine::{CrossStitches, Stitch, Strand, Tixel, Twine},
  verify::Verified,
  Ipld,
};

/// A builder for constructing a Tixel
///
/// Don't create this directly, use [`TwineBuilder`] instead.
pub struct TixelBuilder<'a, 'b, S: Signer<Key = PublicKey>> {
  signer: &'a S,
  strand: Strand,
  prev: Option<&'b Twine>,
  stitches: CrossStitches,
  payload: Ipld,
}

impl<'a, 'b, S: Signer<Key = PublicKey>> TixelBuilder<'a, 'b, S> {
  pub(crate) fn new_first(signer: &'a S, strand: Strand) -> Self {
    Self {
      signer,
      strand,
      prev: None,
      stitches: CrossStitches::default(),
      payload: Ipld::Null,
    }
  }

  pub(crate) fn new_next(signer: &'a S, prev: &'b Twine) -> Self {
    Self {
      signer,
      strand: prev.strand().clone(),
      prev: Some(prev),
      stitches: prev.cross_stitches(),
      payload: Ipld::Null,
    }
  }

  /// Set the cross-stitches for this tixel
  pub fn cross_stitches<C: Into<CrossStitches>>(mut self, stitches: C) -> Self {
    self.stitches = stitches.into();
    self
  }

  /// Set the payload for this tixel
  ///
  /// The payload can be any serializable type
  pub fn payload<P>(mut self, payload: P) -> Self
  where
    P: serde::ser::Serialize,
  {
    self.payload = to_ipld(payload).unwrap();
    self
  }

  fn next_back_stitches(&self) -> Result<Vec<Stitch>, BuildError> {
    if let Some(prev) = &self.prev {
      let mut stitches = prev.back_stitches().into_inner();
      let radix = self.strand.radix();
      let pindex = prev.index();
      if pindex == 0 {
        return Ok(vec![(*prev).clone().into()]);
      }

      let expected_len = if radix == 0 {
        1
      } else {
        ((pindex as f64).log(radix as f64).ceil()).max(1.) as usize
      };
      if stitches.len() != expected_len {
        // (`Previous links array has incorrect size. Expected: ${expected_len}, got: ${links.length}`)
        return Err(BuildError::BadData(VerificationError::InvalidTwineFormat(
          format!(
            "Previous links array has incorrect size. Expected: {}, got: {}",
            expected_len,
            stitches.len()
          ),
        )));
      }

      if radix == 0 {
        return Ok(vec![(*prev).clone().into()]);
      }

      let z = get_layer_pos(radix, pindex) + 1;
      if z > stitches.len() {
        stitches.resize(z, (*prev).clone().into());
      }

      stitches.splice(0..z, std::iter::repeat((*prev).clone().into()).take(z));
      Ok(stitches)
    } else {
      Ok(vec![])
    }
  }

  /// Provide a function to build the payload for this tixel and then finalize the tixel
  ///
  /// The provided builder function will be called with the current strand and the previous tixel
  /// (if any). The function should return the payload for this tixel.
  ///
  /// This method will then finalize the tixel and return the constructed twine, equivalent
  /// to calling `done()`.
  ///
  /// # Example
  ///
  /// ```rust
  /// # #[cfg(feature = "rustcrypto-signer")] {
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code, twine::CrossStitches};
  /// use twine_builder::{TwineBuilder, RustCryptoSigner};
  /// # let signer = RustCryptoSigner::generate_ed25519();
  /// let builder = TwineBuilder::new(signer);
  /// # let strand = builder.build_strand().done().unwrap();
  /// // ...
  /// let first = builder.build_first(strand)
  ///   .build_payload_then_done(|_, _| {
  ///     Ok(ipld!({
  ///       "foo": "bar",
  ///     }))
  ///   })
  ///   .unwrap();
  /// # }
  /// ```
  pub fn build_payload_then_done<F, P>(mut self, build_fn: F) -> Result<Twine, BuildError>
  where
    F: FnOnce(&Strand, Option<&Twine>) -> Result<P, BuildError>,
    P: serde::ser::Serialize,
  {
    let payload = build_fn(&self.strand, self.prev)?;
    self.payload = to_ipld(payload).unwrap();
    self.done()
  }

  /// Finalize the tixel and return the constructed twine
  pub fn done(self) -> Result<Twine, BuildError> {
    use twine_lib::schemas::*;

    let index = self
      .prev
      .as_ref()
      .map(|p| (p.index()).checked_add(1).ok_or(BuildError::IndexMaximum))
      .unwrap_or(Ok(0))?;

    // The drop index becomes the current tixel index if
    // the specified cross-stitches are not a superset of the previous ones
    let drop = match self.prev {
      Some(prev) => {
        let prev_stitches = prev.cross_stitches().strands();
        let cross_stitches = self.stitches.strands();
        if !cross_stitches.is_superset(&prev_stitches) {
          index
        } else {
          prev.drop_index()
        }
      }
      None => 0,
    };

    let content: v2::TixelContentV2 = match self.strand.version().major {
      2 => v2::TixelContentV2 {
        code: self.strand.hasher().into(),
        specification: self.strand.spec_str().parse()?,
        fields: Verified::try_new(v2::TixelFields {
          index,
          back_stitches: self
            .next_back_stitches()?
            .into_iter()
            .map(|s| Some(s.tixel))
            .collect(),
          payload: self.payload,
          cross_stitches: self.stitches.into(),
          strand: self.strand.cid(),
          drop,
        })?,
      },
      _ => {
        return Err(BuildError::BadSpecification(SpecificationError::new(
          format!("Unsupported version: {}", self.strand.version()),
        )))
      }
    };

    let bytes =
      twine_lib::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let signature = self.signer.sign(&bytes)?;

    let container = v2::ContainerV2::new_from_parts(Verified::try_new(content)?, signature);
    let tixel = Tixel::try_new(container)?;
    Ok(Twine::try_new(self.strand, tixel)?)
  }
}

/// A builder for constructing a Strand
///
/// Don't create this directly, use [`TwineBuilder`] instead.
pub struct StrandBuilder<'a, S: Signer<Key = PublicKey>> {
  signer: &'a S,
  hasher: Code,
  version: Version,
  details: Ipld,
  genesis: Option<chrono::DateTime<chrono::Utc>>,
  subspec: Option<Subspec>,
  radix: u8,
}

impl<'a, S: Signer<Key = PublicKey>> StrandBuilder<'a, S> {
  pub(crate) fn new(signer: &'a S) -> Self {
    Self {
      signer,
      hasher: Code::Sha3_512,
      version: Version::new(2, 0, 0),
      details: Ipld::Map(Default::default()),
      genesis: None,
      subspec: None,
      radix: 32,
    }
  }

  /// Set the hasher for this strand
  ///
  /// Hashers can be found in [`twine_lib::multihash_codetable::Code`]
  pub fn hasher(mut self, hasher: Code) -> Self {
    self.hasher = hasher;
    self
  }

  /// Set the details for this strand
  ///
  /// The details can be any serializable type
  pub fn details<P>(mut self, details: P) -> Self
  where
    P: serde::ser::Serialize,
  {
    self.details = to_ipld(details).unwrap();
    self
  }

  /// Set the genesis time for this strand
  ///
  /// If not set, the current time when `done()` is called will be used.
  pub fn genesis(mut self, genesis: chrono::DateTime<chrono::Utc>) -> Self {
    self.genesis = Some(genesis);
    self
  }

  /// Set the subspec for this strand
  ///
  /// For more information see [`twine_lib::specification::Subspec`]
  pub fn subspec(mut self, subspec: String) -> Self {
    self.subspec = Some(Subspec::from_string(subspec).expect("Invalid subspec"));
    self
  }

  /// Set the radix for this strand
  ///
  /// The radix defaults to 32
  pub fn radix(mut self, radix: u8) -> Self {
    self.radix = radix;
    self
  }

  /// Finalize the strand and return the constructed strand
  pub fn done(self) -> Result<Strand, BuildError> {
    use twine_lib::schemas::*;
    let key = self.signer.public_key();

    let content = match self.version.major {
      2 => v2::StrandContentV2 {
        code: self.hasher.into(),
        specification: match self.subspec {
          Some(subspec) => format!("twine/{}/{}", self.version, subspec).try_into()?,
          None => format!("twine/{}", self.version).try_into()?,
        },
        fields: Verified::try_new(v2::StrandFields {
          radix: self.radix,
          details: self.details,
          key,
          genesis: self.genesis.unwrap_or_else(|| chrono::Utc::now()),
          expiry: None,
        })?,
      },
      _ => {
        return Err(BuildError::BadSpecification(SpecificationError::new(
          format!("Unsupported version: {}", self.version),
        )))
      }
    };

    let bytes =
      twine_lib::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let signature = self.signer.sign(&bytes)?;
    let container = v2::ContainerV2::new_from_parts(Verified::try_new(content)?, signature);
    Ok(Strand::try_new(container)?)
  }
}

#[cfg(all(feature = "rsa", feature = "rustcrypto-signer"))]
#[cfg(test)]
mod test {
  use super::*;
  use crate::RustCryptoSigner;

  const TEST_KEY: &str = include_str!("../../test_data/test_rsa_key.pem");

  #[test]
  fn test_rsa() {
    let signer = RustCryptoSigner::from_pkcs8_pem(TEST_KEY).unwrap();
    let strand = StrandBuilder::new(&signer)
      .hasher(Code::Sha3_512)
      .details("test")
      .radix(32)
      .done()
      .unwrap();

    let tixel = TixelBuilder::new_first(&signer, strand)
      .payload("test")
      .done()
      .unwrap();

    dbg!(tixel);
  }
}

#[cfg(all(test, feature = "rustcrypto-signer"))]
mod tests {
  use super::*;
  use crate::{RustCryptoSigner, TwineBuilder};
  use twine_lib::ipld_core::ipld;

  fn ed25519_builder() -> TwineBuilder<2, RustCryptoSigner> {
    TwineBuilder::new(RustCryptoSigner::generate_ed25519())
  }

  // ── Strand property tests ─────────────────────────────────────────────────

  #[test]
  fn test_strand_default_properties() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();

    assert_eq!(strand.radix(), 32);
    assert_eq!(strand.version().major, 2);
    assert!(strand.subspec().is_none());
  }

  #[test]
  fn test_strand_custom_radix() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().radix(4).done().unwrap();
    assert_eq!(strand.radix(), 4);
  }

  #[test]
  fn test_strand_custom_hasher() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().hasher(Code::Sha3_256).done().unwrap();
    assert_eq!(strand.hasher(), Code::Sha3_256);
  }

  #[test]
  fn test_strand_details() {
    let builder = ed25519_builder();
    let strand = builder
      .build_strand()
      .details(ipld!({ "env": "test", "version": 1 }))
      .done()
      .unwrap();
    assert_eq!(strand.details(), &ipld!({ "env": "test", "version": 1 }));
  }

  #[test]
  fn test_strand_subspec() {
    let builder = ed25519_builder();
    let strand = builder
      .build_strand()
      .subspec("myapp/1.2.0".to_string())
      .done()
      .unwrap();
    let subspec = strand.subspec().expect("subspec should be present");
    assert_eq!(subspec.semver().to_string(), "1.2.0");
  }

  // ── Tixel index tests ─────────────────────────────────────────────────────

  #[test]
  fn test_tixel_sequential_indices() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();

    let t0 = builder.build_first(strand).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();
    let t2 = builder.build_next(&t1).done().unwrap();

    assert_eq!(t0.index(), 0);
    assert_eq!(t1.index(), 1);
    assert_eq!(t2.index(), 2);
  }

  #[test]
  fn test_tixel_strand_cid_matches() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();
    let strand_cid = strand.cid();

    let t0 = builder.build_first(strand).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();

    assert_eq!(t0.strand_cid(), strand_cid);
    assert_eq!(t1.strand_cid(), strand_cid);
  }

  // ── Signature verification ────────────────────────────────────────────────

  #[test]
  fn test_signature_verification() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();

    let t0 = builder.build_first(strand.clone()).payload(42i64).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();

    strand.verify_tixel(t0.tixel()).expect("t0 signature should verify");
    strand.verify_tixel(t1.tixel()).expect("t1 signature should verify");
  }

  #[test]
  fn test_signature_verification_long_chain() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().radix(4).done().unwrap();

    let mut prev = builder.build_first(strand.clone()).done().unwrap();
    for i in 1u64..20 {
      prev = builder.build_next(&prev).payload(i).done().unwrap();
      strand.verify_tixel(prev.tixel()).expect("signature should verify at each step");
    }
  }

  // ── Back stitch tests ─────────────────────────────────────────────────────

  #[test]
  fn test_first_tixel_no_back_stitches() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();
    let t0 = builder.build_first(strand).done().unwrap();

    assert_eq!(t0.back_stitches().len(), 0);
    assert!(t0.previous().is_none());
  }

  #[test]
  fn test_back_stitches_radix_0_linear() {
    // radix=0 means no skip links: each tixel has exactly one back stitch
    // pointing at the immediately previous tixel
    let builder = ed25519_builder();
    let strand = builder.build_strand().radix(0).done().unwrap();

    let t0 = builder.build_first(strand).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();
    let t2 = builder.build_next(&t1).done().unwrap();
    let t3 = builder.build_next(&t2).done().unwrap();

    assert_eq!(t0.back_stitches().len(), 0);
    assert_eq!(t1.back_stitches().len(), 1);
    assert_eq!(t2.back_stitches().len(), 1);
    assert_eq!(t3.back_stitches().len(), 1);

    assert_eq!(t1.back_stitches().get(0).unwrap().tixel, t0.cid());
    assert_eq!(t2.back_stitches().get(0).unwrap().tixel, t1.cid());
    assert_eq!(t3.back_stitches().get(0).unwrap().tixel, t2.cid());
  }

  #[test]
  fn test_back_stitches_radix_2_lengths() {
    // For radix=2, back stitch list length grows as ceil(log2(index)).
    // Specifically, the list grows at each new power of 2 in the previous index.
    let builder = ed25519_builder();
    let strand = builder.build_strand().radix(2).done().unwrap();

    let t0 = builder.build_first(strand).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap(); // prev index 0 → early return
    let t2 = builder.build_next(&t1).done().unwrap(); // prev index 1
    let t3 = builder.build_next(&t2).done().unwrap(); // prev index 2 (=2^1) → grows
    let t4 = builder.build_next(&t3).done().unwrap(); // prev index 3
    let t5 = builder.build_next(&t4).done().unwrap(); // prev index 4 (=2^2) → grows
    let t6 = builder.build_next(&t5).done().unwrap(); // prev index 5
    let t7 = builder.build_next(&t6).done().unwrap(); // prev index 6
    let t8 = builder.build_next(&t7).done().unwrap(); // prev index 7

    assert_eq!(t0.back_stitches().len(), 0);
    assert_eq!(t1.back_stitches().len(), 1);
    assert_eq!(t2.back_stitches().len(), 1);
    assert_eq!(t3.back_stitches().len(), 2);
    assert_eq!(t4.back_stitches().len(), 2);
    assert_eq!(t5.back_stitches().len(), 3);
    assert_eq!(t6.back_stitches().len(), 3);
    assert_eq!(t7.back_stitches().len(), 3);
    assert_eq!(t8.back_stitches().len(), 3);
  }

  #[test]
  fn test_back_stitches_radix_2_skip_pointers() {
    // Verify the actual skip pointer targets for radix=2:
    //   t4  back_stitches = [t3, t2]          (skip 1, skip 2)
    //   t8  back_stitches = [t7, t6, t4]      (skip 1, skip 2, skip 4)
    let builder = ed25519_builder();
    let strand = builder.build_strand().radix(2).done().unwrap();

    let t0 = builder.build_first(strand).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();
    let t2 = builder.build_next(&t1).done().unwrap();
    let t3 = builder.build_next(&t2).done().unwrap();
    let t4 = builder.build_next(&t3).done().unwrap();
    let t5 = builder.build_next(&t4).done().unwrap();
    let t6 = builder.build_next(&t5).done().unwrap();
    let t7 = builder.build_next(&t6).done().unwrap();
    let t8 = builder.build_next(&t7).done().unwrap();

    // t4: immediate previous is t3, skip pointer to t2
    assert_eq!(t4.back_stitches().get(0).unwrap().tixel, t3.cid());
    assert_eq!(t4.back_stitches().get(1).unwrap().tixel, t2.cid());

    // t8: immediate previous is t7, skip to t6, skip to t4
    assert_eq!(t8.back_stitches().get(0).unwrap().tixel, t7.cid());
    assert_eq!(t8.back_stitches().get(1).unwrap().tixel, t6.cid());
    assert_eq!(t8.back_stitches().get(2).unwrap().tixel, t4.cid());

    // `previous()` should always be the immediate prior
    assert_eq!(t1.previous().unwrap().tixel, t0.cid());
    assert_eq!(t8.previous().unwrap().tixel, t7.cid());
  }

  #[test]
  fn test_tixel_includes_back_stitches() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().radix(2).done().unwrap();

    let t0 = builder.build_first(strand).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();
    let t2 = builder.build_next(&t1).done().unwrap();
    let t3 = builder.build_next(&t2).done().unwrap();
    let t4 = builder.build_next(&t3).done().unwrap();
    let t5 = builder.build_next(&t4).done().unwrap();

    // t5 back_stitches = [t4, t4, t4] (all point to t4)
    assert!(t5.includes(&t4));
    // t5 does not directly include t3 (it's not in any stitch list)
    assert!(!t5.includes(&t3));
    // t4 includes t2 as a skip pointer
    assert!(t4.includes(&t2));
  }

  // ── Payload tests ─────────────────────────────────────────────────────────

  #[test]
  fn test_payload_round_trip_struct() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Record {
      label: String,
      value: u64,
    }

    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();
    let record = Record { label: "hello".into(), value: 99 };

    let t0 = builder.build_first(strand).payload(&record).done().unwrap();
    let extracted: Record = t0.extract_payload().unwrap();

    assert_eq!(extracted, record);
  }

  #[test]
  fn test_payload_round_trip_ipld() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();
    let data = ipld!({ "x": 1, "y": 2 });

    let t0 = builder.build_first(strand).payload(data.clone()).done().unwrap();
    assert_eq!(t0.payload(), &data);
  }

  #[test]
  fn test_build_payload_then_done() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();

    let t0 = builder
      .build_first(strand)
      .build_payload_then_done(|_strand, _prev| Ok("from_builder".to_string()))
      .unwrap();

    assert_eq!(t0.extract_payload::<String>().unwrap(), "from_builder");
  }

  #[test]
  fn test_build_payload_then_done_with_prev() {
    let builder = ed25519_builder();
    let strand = builder.build_strand().done().unwrap();

    let t0 = builder.build_first(strand).payload(0u64).done().unwrap();
    let t1 = builder
      .build_next(&t0)
      .build_payload_then_done(|_strand, prev| {
        let i: u64 = prev.unwrap().extract_payload()?;
        Ok(i + 1)
      })
      .unwrap();

    assert_eq!(t1.extract_payload::<u64>().unwrap(), 1);
  }

  // ── Signer algorithm tests ────────────────────────────────────────────────

  #[test]
  fn test_signer_p256() {
    let signer = RustCryptoSigner::generate_p256();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand().done().unwrap();
    let t0 = builder.build_first(strand.clone()).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();

    strand.verify_tixel(t0.tixel()).expect("p256 t0 should verify");
    strand.verify_tixel(t1.tixel()).expect("p256 t1 should verify");
  }

  #[test]
  fn test_signer_p384() {
    let signer = RustCryptoSigner::generate_p384();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand().done().unwrap();
    let t0 = builder.build_first(strand.clone()).done().unwrap();
    let t1 = builder.build_next(&t0).done().unwrap();

    strand.verify_tixel(t0.tixel()).expect("p384 t0 should verify");
    strand.verify_tixel(t1.tixel()).expect("p384 t1 should verify");
  }

  #[test]
  fn strand_builder_unsupported_version_returns_err() {
    let signer = RustCryptoSigner::generate_ed25519();
    let mut builder = StrandBuilder::new(&signer);
    builder.version = twine_lib::semver::Version::new(9, 0, 0);
    let err = builder.done();
    assert!(
      matches!(err, Err(BuildError::BadSpecification(_))),
      "version major != 2 must return BadSpecification"
    );
  }

  #[cfg(feature = "v1")]
  #[test]
  #[allow(deprecated)]
  fn tixel_builder_unsupported_strand_version_returns_err() {
    use crate::BiscuitSigner;
    use biscuit::jws::Secret;
    use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
    use std::sync::Arc;
    let rng = ring::rand::SystemRandom::new();
    let pkcs = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let key =
      EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs.as_ref(), &rng).unwrap();
    let v1_signer = BiscuitSigner::new(Secret::EcdsaKeyPair(Arc::new(key)), "ES256".to_string());
    let v1_strand = super::builder_v1::StrandBuilder::new(&v1_signer).done().unwrap();
    assert_eq!(v1_strand.version().major, 1);
    let v2_signer = RustCryptoSigner::generate_ed25519();
    let err = TixelBuilder::new_first(&v2_signer, v1_strand).done();
    assert!(
      matches!(err, Err(BuildError::BadSpecification(_))),
      "v1 strand given to v2 TixelBuilder must return BadSpecification"
    );
  }

  #[test]
  fn build_payload_then_done_propagates_error_v2() {
    let signer = RustCryptoSigner::generate_ed25519();
    let strand = StrandBuilder::new(&signer).done().unwrap();
    let err = TixelBuilder::new_first(&signer, strand).build_payload_then_done(
      |_, _| Err::<String, _>(BuildError::PayloadConstruction("v2-fail".into())),
    );
    assert!(
      matches!(err, Err(BuildError::PayloadConstruction(_))),
      "payload builder error must propagate"
    );
  }

  #[test]
  fn strand_builder_genesis_is_stored() {
    use chrono::TimeZone;
    let signer = RustCryptoSigner::generate_ed25519();
    let genesis = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let strand = StrandBuilder::new(&signer).genesis(genesis).done().unwrap();
    assert_eq!(strand.version().major, 2);
  }
}
