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

#[cfg(test)]
mod test {
  use super::*;
  use multihash_codetable::Code;

  // -----------------------------------------------------------------------
  // get_hasher: supported codes
  // -----------------------------------------------------------------------

  #[test]
  fn get_hasher_sha2_256_roundtrip() {
    let cid = get_cid(Code::Sha2_256, b"test data");
    let code = get_hasher(&cid).unwrap();
    assert_eq!(code, Code::Sha2_256);
  }

  #[test]
  fn get_hasher_sha2_512_roundtrip() {
    let cid = get_cid(Code::Sha2_512, b"test data");
    let code = get_hasher(&cid).unwrap();
    assert_eq!(code, Code::Sha2_512);
  }

  // -----------------------------------------------------------------------
  // get_hasher: unsupported code → UnsupportedHashAlgorithm
  // -----------------------------------------------------------------------

  #[test]
  fn get_hasher_unsupported_code() {
    // Build a CID whose multihash uses a code that is not in the Code enum.
    // 0x0000_dead is not a standard multihash code.
    let fake_digest = [0u8; 32];
    let mh = multihash::Multihash::<64>::wrap(0x0000_dead, &fake_digest)
      .expect("wrap should succeed for any code");
    let codec = <serde_ipld_dagcbor::codec::DagCborCodec as Codec<bool>>::CODE;
    let cid = Cid::new_v1(codec, mh);

    let err = get_hasher(&cid).unwrap_err();
    assert!(
      matches!(err, VerificationError::UnsupportedHashAlgorithm),
      "expected UnsupportedHashAlgorithm, got {:?}",
      err
    );
  }

  // -----------------------------------------------------------------------
  // get_cid: determinism
  // -----------------------------------------------------------------------

  #[test]
  fn get_cid_is_deterministic() {
    let a = get_cid(Code::Sha2_256, b"hello");
    let b = get_cid(Code::Sha2_256, b"hello");
    assert_eq!(a, b);
  }

  #[test]
  fn get_cid_differs_for_different_data() {
    let a = get_cid(Code::Sha2_256, b"hello");
    let b = get_cid(Code::Sha2_256, b"world");
    assert_ne!(a, b);
  }

  #[test]
  fn get_cid_differs_for_different_hashers() {
    let a = get_cid(Code::Sha2_256, b"hello");
    let b = get_cid(Code::Sha2_512, b"hello");
    assert_ne!(a, b);
  }

  // -----------------------------------------------------------------------
  // assert_cid: match and mismatch
  // -----------------------------------------------------------------------

  #[test]
  fn assert_cid_same_cids_ok() {
    let cid = get_cid(Code::Sha2_256, b"some data");
    assert_cid(&cid, &cid).unwrap();
  }

  #[test]
  fn assert_cid_different_cids_returns_error() {
    let a = get_cid(Code::Sha2_256, b"data_a");
    let b = get_cid(Code::Sha2_256, b"data_b");
    let err = assert_cid(&a, &b).unwrap_err();
    assert!(matches!(
      err,
      VerificationError::CidMismatch { .. }
    ));
  }

  #[test]
  fn assert_cid_mismatch_includes_cid_strings() {
    let a = get_cid(Code::Sha2_256, b"data_a");
    let b = get_cid(Code::Sha2_256, b"data_b");
    let err = assert_cid(&a, &b).unwrap_err();
    let msg = err.to_string();
    // The error message should contain both CID representations.
    assert!(msg.contains(&a.to_string()), "expected CID not in message: {}", msg);
    assert!(msg.contains(&b.to_string()), "actual CID not in message: {}", msg);
  }

  // -----------------------------------------------------------------------
  // get_cid + get_hasher full roundtrip
  // -----------------------------------------------------------------------

  #[test]
  fn cid_hasher_roundtrip_sha2_512() {
    let data = b"twine content";
    let cid = get_cid(Code::Sha2_512, data);
    let recovered_code = get_hasher(&cid).unwrap();
    assert_eq!(recovered_code, Code::Sha2_512);
    // Re-computing the CID with the recovered hasher should reproduce the same CID.
    let recomputed = get_cid(recovered_code, data);
    assert_cid(&cid, &recomputed).unwrap();
  }
}
