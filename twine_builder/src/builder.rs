use twine_core::{
  errors::{SpecificationError, VerificationError}, libipld::multihash::{Code, MultihashDigest}, semver::Version, specification::Subspec, twine::{container::TwineContent, CrossStitches, Strand, StrandContent}, verify::Verified, Ipld
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
}

pub struct Builder<S: Signer> {
  signer: S,
}

impl <S: Signer> Builder<S> {
  pub fn new(signer: S) -> Self {
    Self {
      signer,
    }
  }

  pub fn build_strand<'a>(&'a self) -> StrandBuilder<'a, S> {
    StrandBuilder::new(&self.signer)
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
  use twine_core::libipld::ipld;
  use super::*;

  #[test]
  fn test_build_p256() {
    let signer = jwk::Jwk::generate_ec_key(jwk::alg::ec::EcCurve::P256).unwrap();
    let builder = Builder::new(signer);
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
    let builder = Builder::new(signer);
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
    let builder = Builder::new(signer);
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
    let builder = Builder::new(signer);
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
    let builder = Builder::new(signer);
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
    let builder = Builder::new(signer);
    let strand = builder.build_strand()
      .version("1.0.0".to_string())
      .details(ipld!({
        "foo": "bar",
      }))
      .done();

    assert!(strand.is_ok(), "{}", strand.unwrap_err());
    assert!(strand.unwrap().verify_own_signature().is_ok(), "Failed to verify signature");
  }
}
