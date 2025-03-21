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
  /// use twine_lib::{ipld_core::ipld, multihash_codetable::Code, twine::CrossStitches};
  /// use twine_builder::{TwineBuilder, RingSigner};
  /// # let signer = RingSigner::generate_ed25519().unwrap();
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

#[cfg(feature = "rsa")]
#[cfg(test)]
mod test {
  use super::*;
  use crate::RingSigner;

  const TEST_KEY: &str = include_str!("../../test_data/test_rsa_key.pem");

  #[test]
  fn test_rsa() {
    let signer = RingSigner::from_pem(TEST_KEY).unwrap();
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
