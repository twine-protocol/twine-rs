use super::*;
use std::sync::Arc;
use twine_core::{
  errors::{SpecificationError, VerificationError}, ipld_core::{codec::Codec, serde::to_ipld}, multihash_codetable::{Code, MultihashDigest}, semver::Version, skiplist::get_layer_pos, specification::Subspec, twine::{
    CrossStitches,
    Stitch,
    Strand,
    Tixel,
    Twine
  }, verify::Verified, Ipld
};
use twine_core::schemas::v1::{ContainerV1, ChainContentV1, PulseContentV1};
use biscuit::jwk::JWK;

pub struct TixelBuilder<'a, S: Signer<Key = JWK<()>>> {
  signer: &'a S,
  strand: Arc<Strand>,
  prev: Option<Twine>,
  stitches: CrossStitches,
  payload: Ipld,
  source: String,
}

impl <'a, S: Signer<Key = JWK<()>>> TixelBuilder<'a, S> {
  pub fn new_first(signer: &'a S, strand: Arc<Strand>) -> Self {
    Self {
      signer,
      strand,
      prev: None,
      stitches: CrossStitches::default(),
      payload: Ipld::Null,
      source: String::new(),
    }
  }

  pub fn new_next(signer: &'a S, prev: Twine) -> Self {
    Self {
      signer,
      strand: prev.strand(),
      prev: Some(prev),
      stitches: CrossStitches::default(),
      payload: Ipld::Map(Default::default()),
      source: String::new(),
    }
  }

  pub fn cross_stitches<C: Into<CrossStitches>>(mut self, stitches: C) -> Self {
    self.stitches = stitches.into();
    self
  }

  pub fn payload<P>(mut self, payload: P) -> Self where P: serde::ser::Serialize {
    self.payload = to_ipld(payload).unwrap();
    self
  }

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
        return Ok(vec![prev.clone().into()]);
      }

      let expected_len = if radix == 0 {
        1
      } else {
        ((pindex as f64).log(radix as f64).ceil()).max(1.) as usize
      };
      if stitches.len() != expected_len {
        // (`Previous links array has incorrect size. Expected: ${expected_len}, got: ${links.length}`)
        return Err(BuildError::BadData(VerificationError::InvalidTwineFormat(format!(
          "Previous links array has incorrect size. Expected: {}, got: {}",
          expected_len, stitches.len()
        ))));
      }

      if radix == 0 {
        return Ok(vec![prev.clone().into()]);
      }

      let z = get_layer_pos(radix, pindex) + 1;
      if z > stitches.len() {
        stitches.resize(z, prev.clone().into());
      }

      stitches.splice(0..z, std::iter::repeat(prev.clone().into()).take(z));
      Ok(stitches)
    } else {
      Ok(vec![])
    }
  }

  pub fn done(self) -> Result<Twine, BuildError> {
    use twine_core::schemas::*;
    let content: PulseContentV1 = match self.strand.version().major {
      1 => v1::PulseContentV1 {
        index: self.prev.as_ref().map(|p|
          (p.index() as u32).checked_add(1)
            .ok_or(BuildError::IndexMaximum)
        ).unwrap_or(Ok(0))?,
        links: self.next_back_stitches()?.into_iter().map(|s| s.tixel).collect(),
        payload: self.payload,
        mixins: self.stitches.stitches().into_iter().collect(),
        chain: self.strand.cid(),
        source: self.source,
      }.into(),
      _ => return Err(BuildError::BadSpecification(
        SpecificationError::new(format!("Unsupported version: {}", self.strand.version()))
      )),
    };

    let hasher = self.strand.hasher();
    let bytes = twine_core::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let dat = hasher.digest(&bytes).to_bytes();
    let signature = String::from_utf8(self.signer.sign(&dat)?.into()).unwrap();

    let container = ContainerV1::<PulseContentV1>::new_from_parts(hasher, Verified::try_new(content)?, signature);
    let tixel = Tixel::try_new(container)?;
    Ok(Twine::try_new_from_shared(self.strand, Arc::new(tixel))?)
  }
}

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

impl <'a, S: Signer<Key = JWK<()>>> StrandBuilder<'a, S> {
  pub fn new(signer: &'a S) -> Self {
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

  pub fn hasher(mut self, hasher: Code) -> Self {
    self.hasher = hasher;
    self
  }

  pub fn details<P>(mut self, details: P) -> Self where P: serde::ser::Serialize {
    self.details = to_ipld(details).unwrap();
    self
  }

  pub fn subspec(mut self, subspec: String) -> Self {
    self.subspec = Some(Subspec::from_string(subspec).expect("Invalid subspec"));
    self
  }

  pub fn radix(mut self, radix: u32) -> Self {
    self.radix = radix;
    self
  }

  pub fn cross_stitches<C: Into<CrossStitches>>(mut self, stitches: C) -> Self {
    self.stitches = stitches.into();
    self
  }

  #[deprecated(note = "Use details() instead")]
  pub fn source(mut self, source: String) -> Self {
    self.source = source;
    self
  }

  pub fn done(self) -> Result<Strand, BuildError> {
    use twine_core::schemas::*;
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
      }.into(),
      _ => return Err(BuildError::BadSpecification(
        SpecificationError::new(format!("Unsupported version: {}", self.version))
      )),
    };

    let bytes = twine_core::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let dat = self.hasher.digest(&bytes).to_bytes();
    let signature = String::from_utf8(self.signer.sign(&dat)?.into()).unwrap();
    let container = ContainerV1::<ChainContentV1>::new_from_parts(self.hasher, Verified::try_new(content)?, signature);
    Ok(Strand::try_new(container)?)
  }
}
