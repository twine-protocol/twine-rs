use std::fmt::Display;
use std::sync::Arc;

use super::Strand;
use super::{BackStitches, CrossStitches, Stitch, Tagged, TwineBlock};
use crate::as_cid::AsCid;
use crate::crypto::get_hasher;
use crate::crypto::Signature;
use crate::errors::VerificationError;
use crate::schemas::TixelSchemaVersion;
use crate::specification::Subspec;
use crate::verify::Verified;
use crate::Cid;
use crate::Ipld;
use ipld_core::codec::Codec;
use ipld_core::serde::from_ipld;
use multihash_codetable::Code;
use semver::Version;
use serde::de::DeserializeOwned;
use serde_ipld_dagcbor::codec::DagCborCodec;
use serde_ipld_dagjson::codec::DagJsonCodec;

/// A Tixel is the chained data block of the Twine protocol
///
/// A tixel alone can be checked for integrity, but not authenticity.
/// To verify authenticity, a Tixel must be checked against its [`Strand`].
///
/// The most common way to obtain a Tixel will be from a [`crate::twine::Twine`],
/// a [`crate::resolver::Resolver`], or using [`Tixel::from_tagged_dag_json`]
/// or [`Tixel::from_block`].
///
/// Tixels implement PartialOrd, ordering by index IF they are part of
/// the same strand.
///
/// A Tixel contains an Arc to its underlying data, so it is
/// efficient to clone and pass around.
///
/// # See also
///
/// - [`Strand`]
/// - [`crate::twine::Twine`]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Tixel(pub(crate) Arc<Verified<TixelSchemaVersion>>);

impl PartialOrd for Tixel {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    if self.strand_cid() != other.strand_cid() {
      return None;
    }
    Some(self.index().cmp(&other.index()))
  }
}

impl Tixel {
  /// Create a new Tixel from its underlying data
  ///
  /// It is uncommon to use this directly.
  pub fn try_new<C>(container: C) -> Result<Self, VerificationError>
  where
    C: TryInto<TixelSchemaVersion>,
    VerificationError: From<<C as TryInto<TixelSchemaVersion>>::Error>,
  {
    let container = container.try_into()?;
    Ok(Self(Arc::new(Verified::try_new(container)?)))
  }

  /// Get the CID of this Tixel
  pub fn cid(&self) -> Cid {
    // TODO: this method is confusing in the context of TwineBlock
    *self.0.cid()
  }

  /// Get the CID of the Strand this Tixel belongs to
  pub fn strand_cid(&self) -> Cid {
    // TODO: this method is not consistent with &Cid convention
    *self.0.strand_cid()
  }

  /// Get the index
  pub fn index(&self) -> u64 {
    self.0.index()
  }

  /// Get the spec string
  pub fn spec_str(&self) -> &str {
    self.0.spec_str()
  }

  /// Get the version
  pub fn version(&self) -> Version {
    self.0.version()
  }

  /// Get the subspec
  pub fn subspec(&self) -> Option<Subspec> {
    self.0.subspec()
  }

  /// Get the payload
  pub fn payload(&self) -> &Ipld {
    self.0.payload()
  }

  /// Extract the payload as specified type
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # let r = twine_lib::store::MemoryStore::default();
  /// # let tixel = tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// #   use twine_lib::resolver::unchecked_base::BaseResolver;
  /// #   r.fetch_latest(&twine_lib::Cid::default()).await.unwrap()
  /// # });
  /// use twine_lib::twine::Tixel;
  ///
  /// #[derive(serde::Deserialize)]
  /// struct MyPayload {
  ///   foo: String,
  /// }
  ///
  /// let payload: MyPayload = tixel.extract_payload().unwrap();
  /// ```
  pub fn extract_payload<T: DeserializeOwned>(&self) -> Result<T, VerificationError> {
    let payload = self.payload();
    from_ipld(payload.clone()).map_err(|e| VerificationError::Payload(e.to_string()))
  }

  /// Get the drop index
  pub fn drop_index(&self) -> u64 {
    self.0.drop_index()
  }

  /// Get the back stitches
  pub fn back_stitches(&self) -> BackStitches {
    self.0.back_stitches()
  }

  /// Get the cross stitches
  pub fn cross_stitches(&self) -> CrossStitches {
    self.0.cross_stitches()
  }

  /// Get the tixel as DAG-CBOR bytes
  pub fn bytes(&self) -> Arc<[u8]> {
    DagCborCodec::encode_to_vec(&self.0).unwrap().into()
  }

  /// Verify the Tixel against a Strand
  pub fn verify_with(&self, strand: &Strand) -> Result<(), VerificationError> {
    strand.verify_tixel(self)
  }

  /// Get the stitch of the previous Tixel
  pub fn previous(&self) -> Option<Stitch> {
    self.back_stitches().get(0).cloned()
  }

  /// Check if this tixel includes a stitch to another via its CID
  pub fn includes<C: AsCid>(&self, other: C) -> bool {
    self.back_stitches().includes(other.as_cid()) || self.cross_stitches().includes(other.as_cid())
  }

  /// Get the signature
  pub(crate) fn signature(&self) -> Signature {
    self.0.signature()
  }
}

impl TryFrom<TixelSchemaVersion> for Tixel {
  type Error = VerificationError;

  fn try_from(t: TixelSchemaVersion) -> Result<Self, Self::Error> {
    Ok(Self(Arc::new(Verified::try_new(t)?)))
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
    let t: Tagged<Tixel> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
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
    DagCborCodec::encode_to_vec(&self.0)
      .unwrap()
      .as_slice()
      .into()
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
