use super::{Tagged, Tixel, TwineBlock};
use crate::errors::VerificationError;
use crate::Ipld;
use crate::{
  as_cid::AsCid,
  crypto::{get_hasher, PublicKey},
  schemas::StrandSchemaVersion,
  specification::Subspec,
  verify::Verified,
};
use ipld_core::{cid::Cid, codec::Codec, serde::from_ipld};
use multihash_codetable::Code;
use semver::Version;
use serde::de::DeserializeOwned;
use serde_ipld_dagcbor::codec::DagCborCodec;
use serde_ipld_dagjson::codec::DagJsonCodec;
use std::{fmt::Display, sync::Arc};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Strand(pub(crate) Arc<Verified<StrandSchemaVersion>>);

impl Strand {
  pub fn try_new<C>(container: C) -> Result<Self, VerificationError>
  where
    C: TryInto<StrandSchemaVersion>,
    VerificationError: From<<C as TryInto<StrandSchemaVersion>>::Error>,
  {
    let container = container.try_into()?;
    Ok(Self(Arc::new(Verified::try_new(container)?)))
  }

  pub fn cid(&self) -> Cid {
    *self.0.cid()
  }

  pub fn key(&self) -> PublicKey {
    self.0.key()
  }

  pub fn radix(&self) -> u8 {
    self.0.radix()
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

  pub fn details(&self) -> &Ipld {
    self.0.details()
  }

  pub fn extract_details<T: DeserializeOwned>(&self) -> Result<T, VerificationError> {
    let details = self.details();
    Ok(from_ipld(details.clone()).map_err(|e| VerificationError::Payload(e.to_string()))?)
  }

  pub fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>> {
    self.0.expiry()
  }

  pub fn verify_tixel(&self, tixel: &Tixel) -> Result<(), VerificationError> {
    self.0.verify_tixel(tixel)
  }

  pub fn hasher(&self) -> Code {
    self.0.hasher()
  }
}

impl From<Strand> for Cid {
  fn from(t: Strand) -> Self {
    t.cid()
  }
}

impl AsCid for Strand {
  fn as_cid(&self) -> &Cid {
    self.0.cid()
  }
}

impl TwineBlock for Strand {
  fn cid(&self) -> &Cid {
    self.as_cid()
  }

  fn from_tagged_dag_json<S: Display>(json: S) -> Result<Self, VerificationError> {
    let t: Tagged<Strand> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    Ok(t.unpack())
  }

  fn from_bytes_unchecked(hasher: Code, bytes: Vec<u8>) -> Result<Self, VerificationError> {
    let mut twine: StrandSchemaVersion = DagCborCodec::decode_from_slice(bytes.as_slice())?;
    // if v1... recompute cid
    if let StrandSchemaVersion::V1(_) = twine {
      twine.compute_cid(hasher);
    }
    Ok(Self(Arc::new(Verified::try_new(twine)?)))
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
    DagCborCodec::encode_to_vec(&self.0)
      .unwrap()
      .as_slice()
      .into()
  }

  fn content_bytes(&self) -> Arc<[u8]> {
    self.0.content_bytes()
  }
}

impl Display for Strand {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.tagged_dag_json_pretty())
  }
}
