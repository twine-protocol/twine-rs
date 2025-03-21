//! Hashing utilities
use crate::errors::VerificationError;
use crate::Cid;
use ipld_core::codec::Codec;
use multihash_codetable::{Code, MultihashDigest};

/// Get the hash function ([`Code`]) used by a CID
pub fn get_hasher(cid: &Cid) -> Result<Code, VerificationError> {
  cid
    .hash()
    .code()
    .try_into()
    .map_err(|_| VerificationError::UnsupportedHashAlgorithm)
}

/// Compute the CID of some data using a given hash function
pub fn get_cid<D: AsRef<[u8]>>(hasher: Code, dat: D) -> Cid {
  let mh = hasher.digest(dat.as_ref());
  let code = <serde_ipld_dagcbor::codec::DagCborCodec as Codec<bool>>::CODE;
  Cid::new_v1(code, mh)
}

/// Assert that two CIDs are equal, if they are not, return a [`VerificationError`]
pub fn assert_cid(expected: &Cid, actual: &Cid) -> Result<(), VerificationError> {
  if expected != actual {
    return Err(VerificationError::CidMismatch {
      expected: expected.to_string(),
      actual: actual.to_string(),
    });
  }
  Ok(())
}
