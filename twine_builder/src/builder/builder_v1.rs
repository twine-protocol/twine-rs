//! Underlying builder for Twine V1
use super::*;
use biscuit::jwk::JWK;
use twine_lib::schemas::v1::{ChainContentV1, ContainerV1, PulseContentV1};
use twine_lib::{
  errors::{SpecificationError, VerificationError},
  ipld_core::{codec::Codec, serde::to_ipld},
  multihash_codetable::{Code, MultihashDigest},
  semver::Version,
  skiplist::get_layer_pos,
  specification::Subspec,
  twine::{CrossStitches, Stitch, Strand, Tixel, Twine},
  verify::Verified,
  Ipld,
};

/// Builder for constructing a Twine V1 data
///
/// Don't create this directly, instead use [`crate::TwineBuilder`]
pub struct TixelBuilder<'a, 'b, S: Signer<Key = JWK<()>>> {
  signer: &'a S,
  strand: Strand,
  prev: Option<&'b Twine>,
  stitches: CrossStitches,
  payload: Ipld,
  source: String,
}

impl<'a, 'b, S: Signer<Key = JWK<()>>> TixelBuilder<'a, 'b, S> {
  pub(crate) fn new_first(signer: &'a S, strand: Strand) -> Self {
    Self {
      signer,
      strand,
      prev: None,
      stitches: CrossStitches::default(),
      payload: Ipld::Map(Default::default()),
      source: String::new(),
    }
  }

  pub(crate) fn new_next(signer: &'a S, prev: &'b Twine) -> Self {
    Self {
      signer,
      strand: prev.strand().clone(),
      prev: Some(prev),
      stitches: prev.cross_stitches(),
      payload: Ipld::Map(Default::default()),
      source: String::new(),
    }
  }

  /// Set the cross stitches for this tixel
  ///
  /// The cross stitches must contain all cross stitches from the previous tixel
  /// otherwise an error will be returned when building the tixel.
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

  /// Set the source property for this tixel
  #[deprecated(note = "Use payload() or strand.details() instead")]
  pub fn source(mut self, source: String) -> Self {
    self.source = source;
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
  /// # let signer = BiscuitSigner::new(secret, "ES256".to_string());
  /// // ...
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

    // validate the cross stitches
    let cross_stitches = self.stitches.clone();
    if let Some(prev) = &self.prev {
      let prev_stitches = prev.cross_stitches();
      // ensure all previous cross stitches are present
      let all_present = prev_stitches
        .into_iter()
        .all(|s| cross_stitches.strand_is_stitched(s.1.strand));

      if !all_present {
        return Err(BuildError::BadData(VerificationError::InvalidTwineFormat(
          "Cross stitches must contain all cross stitches from previous tixel".into(),
        )));
      }
    }

    let content: PulseContentV1 = match self.strand.version().major {
      1 => v1::PulseContentV1 {
        index: self
          .prev
          .as_ref()
          .map(|p| {
            (p.index() as u32)
              .checked_add(1)
              .ok_or(BuildError::IndexMaximum)
          })
          .unwrap_or(Ok(0))?,
        links: self
          .next_back_stitches()?
          .into_iter()
          .map(|s| s.tixel)
          .collect(),
        payload: self.payload,
        mixins: self.stitches.stitches().into_iter().collect(),
        chain: self.strand.cid(),
        source: self.source,
      }
      .into(),
      _ => {
        return Err(BuildError::BadSpecification(SpecificationError::new(
          format!("Unsupported version: {}", self.strand.version()),
        )))
      }
    };

    let hasher = self.strand.hasher();
    let bytes =
      twine_lib::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let dat = hasher.digest(&bytes).to_bytes();
    let signature = String::from_utf8(self.signer.sign(&dat)?.into()).unwrap();

    let container =
      ContainerV1::<PulseContentV1>::new_from_parts(hasher, Verified::try_new(content)?, signature);
    let tixel = Tixel::try_new(container)?;
    Ok(Twine::try_new(self.strand, tixel)?)
  }
}

/// Builder for constructing a Strand V1 data
///
/// Don't create this directly, instead use [`crate::TwineBuilder`]
pub struct StrandBuilder<'a, S: Signer<Key = JWK<()>>> {
  signer: &'a S,
  hasher: Code,
  version: Version,
  details: Ipld,
  subspec: Option<Subspec>,
  radix: u32,
  stitches: CrossStitches,
  source: String,
}

impl<'a, S: Signer<Key = JWK<()>>> StrandBuilder<'a, S> {
  pub(crate) fn new(signer: &'a S) -> Self {
    Self {
      signer,
      hasher: Code::Sha3_512,
      version: Version::new(1, 0, 0),
      details: Ipld::Map(Default::default()),
      subspec: None,
      radix: 32,
      stitches: CrossStitches::default(),
      source: String::new(),
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

  /// Set the subspec string for this strand
  ///
  /// See [`twine_lib::specification::Subspec`] for more information
  pub fn subspec(mut self, subspec: String) -> Self {
    self.subspec = Some(Subspec::from_string(subspec).expect("Invalid subspec"));
    self
  }

  /// Set the radix for this strand
  pub fn radix(mut self, radix: u32) -> Self {
    self.radix = radix;
    self
  }

  /// Set the cross stitches for this strand
  ///
  /// This is only a feature of v1 strands.
  pub fn cross_stitches<C: Into<CrossStitches>>(mut self, stitches: C) -> Self {
    self.stitches = stitches.into();
    self
  }

  /// Set the source property for this strand
  #[deprecated(note = "Use details() instead")]
  pub fn source(mut self, source: String) -> Self {
    self.source = source;
    self
  }

  /// Finalize the strand and return the constructed strand
  pub fn done(self) -> Result<Strand, BuildError> {
    use twine_lib::schemas::*;
    let key = self.signer.public_key();
    let content: ChainContentV1 = match self.version.major {
      1 => v1::ChainContentV1 {
        key,
        links_radix: self.radix,
        mixins: self.stitches.stitches().into_iter().collect(),
        meta: self.details,
        specification: match self.subspec {
          Some(subspec) => format!("twine/{}/{}", self.version, subspec).try_into()?,
          None => format!("twine/{}", self.version).try_into()?,
        },
        source: self.source,
      }
      .into(),
      _ => {
        return Err(BuildError::BadSpecification(SpecificationError::new(
          format!("Unsupported version: {}", self.version),
        )))
      }
    };

    let bytes =
      twine_lib::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let dat = self.hasher.digest(&bytes).to_bytes();
    let signature = String::from_utf8(self.signer.sign(&dat)?.into()).unwrap();
    let container = ContainerV1::<ChainContentV1>::new_from_parts(
      self.hasher,
      Verified::try_new(content)?,
      signature,
    );
    Ok(Strand::try_new(container)?)
  }
}
