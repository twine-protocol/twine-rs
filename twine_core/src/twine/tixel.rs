use std::fmt::Display;
use std::sync::Arc;

use crate::as_cid::AsCid;
use crate::crypto::get_hasher;
use crate::crypto::Signature;
use crate::schemas::TixelSchemaVersion;
use crate::specification::Subspec;
use crate::errors::VerificationError;
use crate::verify::Verified;
use crate::Cid;
use crate::Ipld;
use ipld_core::serde::from_ipld;
use multihash_codetable::Code;
use semver::Version;
use serde::de::DeserializeOwned;
use ipld_core::codec::Codec;
use serde_ipld_dagcbor::codec::DagCborCodec;
use serde_ipld_dagjson::codec::DagJsonCodec;
use super::{BackStitches, CrossStitches, Stitch, Tagged, TwineBlock};
use super::Strand;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Tixel(pub(crate) Verified<TixelSchemaVersion>);

impl PartialOrd for Tixel {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    if self.strand_cid() != other.strand_cid() {
      return None;
    }
    Some(self.index().cmp(&other.index()))
  }
}

impl Tixel {
  pub fn try_new<C>(container: C) -> Result<Self, VerificationError>
  where
    C: TryInto<TixelSchemaVersion>,
    VerificationError:From<<C as TryInto<TixelSchemaVersion>>::Error>
  {
    let container = container.try_into()?;
    Ok(Self(Verified::try_new(container)?))
  }

  pub fn cid(&self) -> Cid {
    *self.0.cid()
  }

  pub fn strand_cid(&self) -> Cid {
    *self.0.strand_cid()
  }

  pub fn index(&self) -> u64 {
    self.0.index()
  }

  pub fn spec_str(&self) -> &str {
    self.0.spec_str()
  }

  pub fn version(&self) -> Version {
    self.0.version()
  }

  pub fn subspec(&self) -> Option<Subspec> {
    self.0.subspec()
  }

  pub fn payload(&self) -> &Ipld {
    self.0.payload()
  }

  pub fn extract_payload<T: DeserializeOwned>(&self) -> Result<T, VerificationError> {
    let payload = self.payload();
    from_ipld(payload.clone()).map_err(|e| VerificationError::Payload(e.to_string()))
  }

  pub fn drop_index(&self) -> u64 {
    self.0.drop_index()
  }

  pub fn back_stitches(&self) -> BackStitches {
    self.0.back_stitches()
  }

  pub fn cross_stitches(&self) -> CrossStitches {
    self.0.cross_stitches()
  }

  pub fn bytes(&self) -> Arc<[u8]> {
    DagCborCodec::encode_to_vec(&self.0).unwrap().into()
  }

  pub fn verify_with(&self, strand: &Strand) -> Result<(), VerificationError> {
    strand.verify_tixel(self)
  }

  pub fn previous(&self) -> Option<Stitch> {
    self.back_stitches().get(0).cloned()
  }

  pub fn includes<C: AsCid>(&self, other: C) -> bool {
    self.back_stitches().includes(other.as_cid())
    || self.cross_stitches().includes(other.as_cid())
  }

  pub(crate) fn signature(&self) -> Signature {
    self.0.signature()
  }
}

impl TryFrom<TixelSchemaVersion> for Tixel {
  type Error = VerificationError;

  fn try_from(t: TixelSchemaVersion) -> Result<Self, Self::Error> {
    Ok(Self(Verified::try_new(t)?))
  }
}

impl From<Tixel> for Cid {
  fn from(t: Tixel) -> Self {
    t.cid()
  }
}

impl AsCid for Tixel {
  fn as_cid(&self) -> &Cid {
    self.0.cid()
  }
}

impl TwineBlock for Tixel {
  fn cid(&self) -> &Cid {
    self.as_cid()
  }

  fn from_tagged_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let t : Tagged<Tixel> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    Ok(t.unpack())
  }

  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let mut twine: TixelSchemaVersion = DagCborCodec::decode_from_slice(bytes.as_slice())?;
    // if v1... recompute cid
    if let TixelSchemaVersion::V1(_) = twine {
      twine.compute_cid(hasher);
    }
    let twine = Self::try_new(twine)?;
    Ok(twine)
  }

  fn from_block<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, VerificationError> {
    let hasher = get_hasher(&cid)?;
    let twine = Self::from_bytes_unchecked(hasher, bytes.as_ref().to_vec())?;
    twine.verify_cid(&cid)?;
    Ok(twine)
  }

  fn tagged_dag_json(&self) -> String {
    format!(
      "{{\"cid\":{},\"data\":{}}}",
      String::from_utf8(DagJsonCodec::encode_to_vec(&self.cid()).unwrap()).unwrap(),
      String::from_utf8(DagJsonCodec::encode_to_vec(&self.0).unwrap()).unwrap()
    )
  }

  fn bytes(&self) -> Arc<[u8]> {
    DagCborCodec::encode_to_vec(&self.0).unwrap().as_slice().into()
  }

  fn content_bytes(&self) -> Arc<[u8]> {
    self.0.content_bytes()
  }
}

impl Display for Tixel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.tagged_dag_json_pretty())
  }
}
