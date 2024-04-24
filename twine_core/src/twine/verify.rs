use libipld::multihash::{Code, MultihashDigest};
use libipld::Cid;
use super::ParseError;

pub fn get_hasher(cid: &Cid) -> Result<Code, libipld::multihash::Error> {
  cid.hash().code().try_into()
}

pub fn get_cid<D: AsRef<[u8]>>(hasher: Code, dat: D) -> Cid {
  let mh = hasher.digest(dat.as_ref());
  Cid::new_v1(libipld::cbor::DagCborCodec.into(), mh)
}

pub fn assert_cid(expected: Cid, actual: Cid) -> Result<(), ParseError> {
  if expected != actual {
    return Err(ParseError(format!("Cid mismatch: expected {}, got {}", expected, actual)));
  }
  Ok(())
}
