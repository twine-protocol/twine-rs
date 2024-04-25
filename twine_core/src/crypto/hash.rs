use libipld::multihash::{Code, MultihashDigest};
use libipld::Cid;
use crate::errors::VerificationError;

pub fn get_hasher(cid: &Cid) -> Result<Code, VerificationError> {
  cid.hash().code().try_into().map_err(|_| VerificationError::UnsupportedHashAlgorithm)
}

pub fn get_cid<D: AsRef<[u8]>>(hasher: Code, dat: D) -> Cid {
  let mh = hasher.digest(dat.as_ref());
  Cid::new_v1(libipld::cbor::DagCborCodec.into(), mh)
}

pub fn assert_cid(expected: Cid, actual: Cid) -> Result<(), VerificationError> {
  if expected != actual {
    return Err(VerificationError::CidMismatch {
      expected: expected.to_string(),
      actual: actual.to_string(),
    });
  }
  Ok(())
}
