use std::sync::Arc;

use twine_core::{
  errors::{SpecificationError, VerificationError},
  semver::Version,
  multihash_codetable::{Code, MultihashDigest},
  skiplist::get_layer_pos,
  specification::Subspec,
  twine::{
    container::TwineContent,
    CrossStitches,
    Stitch,
    Strand,
    StrandContent,
    Tixel,
    TixelContent,
    Twine
  },
  verify::Verified,
  Ipld
};
use crate::{signer::SigningError, Signer};

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

pub struct TwineBuilder<S: Signer> {
  signer: S,
}

impl <S: Signer> TwineBuilder<S> {
  pub fn new(signer: S) -> Self {
    Self {
      signer,
    }
  }

  pub fn build_strand<'a>(&'a self) -> StrandBuilder<'a, S> {
    StrandBuilder::new(&self.signer)
  }

  pub fn build_first<'a>(&'a self, strand: Strand) -> TixelBuilder<'a, S> {
    TixelBuilder::new_first(&self.signer, Arc::new(strand))
  }

  pub fn build_next<'a>(&'a self, prev: Twine) -> TixelBuilder<'a, S> {
    TixelBuilder::new_next(&self.signer, prev)
  }
}

pub struct TixelBuilder<'a, S: Signer> {
  signer: &'a S,
  strand: Arc<Strand>,
  prev: Option<Twine>,
  stitches: CrossStitches,
  payload: Ipld,
  source: String,
}

impl <'a, S: Signer> TixelBuilder<'a, S> {
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

  pub fn payload(mut self, payload: Ipld) -> Self {
    self.payload = payload;
    self
  }

  #[deprecated(note = "Use payload() or strand.details() instead")]
  pub fn source(mut self, source: String) -> Self {
    self.source = source;
    self
  }

  fn next_back_stitches(&self) -> Result<Vec<Stitch>, BuildError> {
    if let Some(prev) = &self.prev {
      let mut stitches = prev.back_stitches();
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
    let content: TixelContent = match self.strand.version().major {
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
    let bytes = content.bytes();
    let dat = hasher.digest(&bytes).to_bytes();
    let signature = self.signer.sign(&dat)?;

    let tixel = Tixel::new_from_parts(hasher, Verified::try_new(content)?, signature);
    Ok(Twine::try_new_from_shared(self.strand, Arc::new(tixel))?)
  }
}

pub struct StrandBuilder<'a, S: Signer> {
  signer: &'a S,
  hasher: Code,
  version: Version,
  details: Ipld,
  subspec: Option<Subspec>,
  radix: u32,
  stitches: CrossStitches,
  source: String,
}

impl <'a, S: Signer> StrandBuilder<'a, S> {
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

  pub fn version(mut self, version: String) -> Self {
    self.version = Version::parse(&version).expect("Invalid version");
    self
  }

  pub fn details(mut self, details: Ipld) -> Self {
    self.details = details;
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
    let content: StrandContent = match self.version.major {
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

    let bytes = content.bytes();
    let dat = self.hasher.digest(&bytes).to_bytes();
    let signature = self.signer.sign(&dat)?;

    Ok(Strand::new_from_parts(self.hasher, Verified::try_new(content)?, signature))
  }
}

#[cfg(test)]
mod test {
  use josekit::jwk;
  use twine_core::ipld_core::ipld;
  use super::*;

  #[test]
  fn test_build_p256() {
    let signer = jwk::Jwk::generate_ec_key(jwk::alg::ec::EcCurve::P256).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }

  #[test]
  fn test_build_p384() {
    let signer = jwk::Jwk::generate_ec_key(jwk::alg::ec::EcCurve::P384).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }

  #[test]
  fn test_build_p521() {
    let signer = jwk::Jwk::generate_ec_key(jwk::alg::ec::EcCurve::P521).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }

  #[test]
  fn test_build_ed25519() {
    let signer = jwk::Jwk::generate_ed_key(jwk::alg::ed::EdCurve::Ed25519).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }

  #[test]
  fn test_build_ed448() {
    let signer = jwk::Jwk::generate_ed_key(jwk::alg::ed::EdCurve::Ed448).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }

  #[test]
  fn test_build_rsa() {
    let signer = jwk::Jwk::generate_rsa_key(2048).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }

  #[test]
  fn text_build_tixels() {
    let signer = jwk::Jwk::generate_ed_key(jwk::alg::ed::EdCurve::Ed25519).unwrap();
    let builder = TwineBuilder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .radix(2)
      .done()
      .unwrap();

    let mut signatures = vec![];
    let mut prev = builder.build_first(strand.clone())
      .payload(ipld!({
        "baz": "qux",
      }))
      .done()
      .unwrap();

    signatures.push(prev.signature().to_string());

    for i in 1..10 {
      prev = builder.build_next(prev)
        .payload(ipld!({
          "baz": "qux",
          "index": i,
        }))
        .done()
        .unwrap();

      signatures.push(prev.signature().to_string());
    }

    dbg!(signatures);
  }
}
