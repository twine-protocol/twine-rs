use super::*;
use std::sync::Arc;
use twine_core::{
  crypto::PublicKey, errors::{SpecificationError, VerificationError}, ipld_core::{codec::Codec, serde::to_ipld}, multihash_codetable::Code, semver::Version, skiplist::get_layer_pos, specification::Subspec, twine::{
    CrossStitches,
    Stitch,
    Strand,
    Tixel,
    Twine
  }, verify::Verified, Ipld
};

pub struct TixelBuilder<'a, 'b, S: Signer<Key = PublicKey>> {
  signer: &'a S,
  strand: Arc<Strand>,
  prev: Option<&'b Twine>,
  stitches: CrossStitches,
  payload: Ipld,
}

impl <'a, 'b, S: Signer<Key = PublicKey>> TixelBuilder<'a, 'b, S> {
  pub fn new_first(signer: &'a S, strand: Arc<Strand>) -> Self {
    Self {
      signer,
      strand,
      prev: None,
      stitches: CrossStitches::default(),
      payload: Ipld::Null,
    }
  }

  pub fn new_next(signer: &'a S, prev: &'b Twine) -> Self {
    Self {
      signer,
      strand: prev.strand(),
      prev: Some(prev),
      stitches: CrossStitches::default(),
      payload: Ipld::Map(Default::default()),
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
        return Err(BuildError::BadData(VerificationError::InvalidTwineFormat(format!(
          "Previous links array has incorrect size. Expected: {}, got: {}",
          expected_len, stitches.len()
        ))));
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

  pub fn done(self) -> Result<Twine, BuildError> {
    use twine_core::schemas::*;

    // TODO: Implement drop
    let drop = 0;

    let content: v2::TixelContentV2 = match self.strand.version().major {
      2 => v2::TixelContentV2 {
        code: self.strand.hasher().into(),
        specification: self.strand.spec_str().parse()?,
        fields: Verified::try_new(v2::TixelFields {
          index: self.prev.as_ref().map(|p|
            (p.index()).checked_add(1)
              .ok_or(BuildError::IndexMaximum)
          ).unwrap_or(Ok(0))?,
          back_stitches: self.next_back_stitches()?.into_iter().map(|s| Some(s.tixel)).collect(),
          payload: self.payload,
          cross_stitches: self.stitches.into(),
          strand: self.strand.cid(),
          drop,
        })?,
      },
      _ => return Err(BuildError::BadSpecification(
        SpecificationError::new(format!("Unsupported version: {}", self.strand.version()))
      )),
    };

    let bytes = twine_core::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let signature = self.signer.sign(&bytes)?;

    let container = v2::ContainerV2::new_from_parts(Verified::try_new(content)?, signature);
    let tixel = Tixel::try_new(container)?;
    Ok(Twine::try_new_from_shared(self.strand, Arc::new(tixel))?)
  }
}

pub struct StrandBuilder<'a, S: Signer<Key = PublicKey>> {
  signer: &'a S,
  hasher: Code,
  version: Version,
  details: Ipld,
  genesis: Option<chrono::DateTime<chrono::Utc>>,
  subspec: Option<Subspec>,
  radix: u8,
  stitches: CrossStitches,
}

impl <'a, S: Signer<Key = PublicKey>> StrandBuilder<'a, S> {
  pub fn new(signer: &'a S) -> Self {
    Self {
      signer,
      hasher: Code::Sha3_512,
      version: Version::new(2, 0, 0),
      details: Ipld::Map(Default::default()),
      genesis: None,
      subspec: None,
      radix: 32,
      stitches: CrossStitches::default(),
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

  pub fn genesis(mut self, genesis: chrono::DateTime<chrono::Utc>) -> Self {
    self.genesis = Some(genesis);
    self
  }

  pub fn subspec(mut self, subspec: String) -> Self {
    self.subspec = Some(Subspec::from_string(subspec).expect("Invalid subspec"));
    self
  }

  pub fn radix(mut self, radix: u8) -> Self {
    self.radix = radix;
    self
  }

  pub fn cross_stitches<C: Into<CrossStitches>>(mut self, stitches: C) -> Self {
    self.stitches = stitches.into();
    self
  }

  pub fn done(self) -> Result<Strand, BuildError> {
    use twine_core::schemas::*;
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
          key: key,
          genesis: self.genesis.unwrap_or_else(|| chrono::Utc::now()),
          expiry: None,
        })?,
      },
      _ => return Err(BuildError::BadSpecification(
        SpecificationError::new(format!("Unsupported version: {}", self.version))
      )),
    };

    let bytes = twine_core::serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&content).unwrap();
    let signature = self.signer.sign(&bytes)?;
    let container = v2::ContainerV2::new_from_parts(Verified::try_new(content)?, signature);
    Ok(Strand::try_new(container)?)
  }
}


#[cfg(test)]
mod test {
  use crate::RingSigner;
  use super::*;

  const TEST_KEY: &str = include_str!("../../test_data/test_rsa_key.pem");

  #[test]
  fn test_rsa(){
    let signer = RingSigner::from_pem(TEST_KEY).unwrap();
    let strand = StrandBuilder::new(&signer)
      .hasher(Code::Sha3_512)
      .details("test")
      .radix(32)
      .done().unwrap();

    let tixel = TixelBuilder::new_first(&signer, Arc::new(strand))
      .payload("test")
      .done().unwrap();

    dbg!(tixel);
  }
}
