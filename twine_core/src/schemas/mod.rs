use ipld_core::{cid::Cid, ipld::Ipld};
use semver::Version;
use crate::{crypto::{PublicKey, Signature}, errors::VerificationError, specification::Subspec, twine::{BackStitches, CrossStitches}, Bytes};

pub mod v1;
pub mod v2;

pub trait TwineContainer {
  fn cid(&self) -> &Cid;
  fn version(&self) -> Version;
  fn spec_str(&self) -> &str;
  fn subspec(&self) -> Option<Subspec>;
  fn signature(&self) -> Signature;
  fn content_bytes(&self) -> Result<Bytes, VerificationError>;
  fn verify_signature(&self, pk: &PublicKey) -> Result<(), VerificationError> {
    pk.verify(self.signature(), &self.content_bytes()?)
  }
}

pub trait StrandContainer: TwineContainer {
  fn key(&self) -> &PublicKey;
  fn radix(&self) -> u8;
  fn details(&self) -> &Ipld;

  fn verify_tixel<T: TixelContainer>(&self, tixel: &T) -> Result<(), VerificationError> {
    // also verify that this tixel belongs to the strand
    if tixel.strand_cid() != self.cid() {
      return Err(VerificationError::TixelNotOnStrand);
    }
    // tixel must have same major version as strand
    if tixel.version().major != self.version().major {
      return Err(VerificationError::InvalidTwineFormat("Tixel version does not match Strand version".into()));
    }
    tixel.verify_signature(self.key())
  }
}

pub trait TixelContainer: TwineContainer {
  fn index(&self) -> u64;
  fn strand_cid(&self) -> &Cid;
  fn cross_stitches(&self) -> CrossStitches;
  fn back_stitches(&self) -> BackStitches;
  fn drop(&self) -> u64;
  fn payload(&self) -> &Ipld;
}
