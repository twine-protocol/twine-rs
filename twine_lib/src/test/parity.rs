//! Characterization / parity tests.
//!
//! These pin the *current* observable behavior of the schema layer (every
//! accessor value for v1 + v2 strands and tixels, plus the security-relevant
//! verification paths) through the public API. They are deliberately
//! refactor-agnostic: they go through `Strand`/`Tixel`/`Twine`/`AnyTwine` and
//! never touch the internal enum, so they must hold identically before and
//! after the schema-trait refactor.

use super::*;
use crate::twine::*;
use crate::Ipld;
use semver::Version;
use serde::Deserialize;

// CIDs as they appear in the fixtures (`cid` field).
const STRAND_V1_CID: &str =
  "bafyriqe3zxf5g4ifgqhea5zozxpdcfi5qcpkfpogtxzbizmmuxdjuzuq44a2cifbr7xplo4kcfsdz2c5pxfxektavrqxxb3nvbmclxz7qiz6e";
const TIXEL_V1_CID: &str =
  "bafyriqgafbhhudahpnzrdvuzjjczro43i4mnv7637vq4oh6m6lfdccpazmmurfu4vluy7iddrhwbbfvjs62uo2wrzx4axaxx5lv7pfmqveqt2";
const STRAND_V2_CID: &str = "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu";
const TIXEL_V2_CID: &str = "bafyrmibrw2iojkmsnyaffhaqwujqriumkk6whnd3bc6rdob7le7zquslp4";

fn strand_v1() -> Strand { Strand::from_tagged_dag_json(STRANDJSON).unwrap() }
fn tixel_v1() -> Tixel { Tixel::from_tagged_dag_json(TIXELJSON).unwrap() }
fn strand_v2() -> Strand { Strand::from_tagged_dag_json(STRAND_V2_JSON).unwrap() }
fn tixel_v2() -> Tixel { Tixel::from_tagged_dag_json(TIXEL_V2_JSON).unwrap() }

// ---------- Strand accessors ----------

#[test]
fn parity_strand_v1_accessors() {
  let s = strand_v1();
  assert_eq!(s.cid().to_string(), STRAND_V1_CID);
  assert_eq!(s.version(), Version::new(1, 0, 0));
  assert_eq!(s.spec_str(), "twine/1.0.x/bell/1.0.x");
  assert_eq!(s.subspec().map(|x| x.to_string()), Some("bell/1.0.0".to_string()));
  assert_eq!(s.radix(), 32);
  assert_eq!(s.expiry(), None);
  assert!(matches!(s.details(), Ipld::Map(_)), "v1 details should be a map");
  // key is exercised indirectly by verify_tixel below
}

#[test]
fn parity_strand_v2_accessors() {
  let s = strand_v2();
  assert_eq!(s.cid().to_string(), STRAND_V2_CID);
  assert_eq!(s.version(), Version::new(2, 0, 0));
  assert_eq!(s.spec_str(), "twine/2.0.0/time/1.0.0");
  assert_eq!(s.subspec().map(|x| x.to_string()), Some("time/1.0.0".to_string()));
  assert_eq!(s.radix(), 32);
  assert_eq!(s.expiry(), None); // "e": null
  assert!(matches!(s.details(), Ipld::Map(m) if m.is_empty()), "v2 details should be empty map");
}

// ---------- Tixel accessors ----------

#[derive(Deserialize)]
struct PayloadV1 { timestamp: String }
#[derive(Deserialize)]
struct PayloadV2 { timestamp: u64 }

#[test]
fn parity_tixel_v1_accessors() {
  let t = tixel_v1();
  assert_eq!(t.cid().to_string(), TIXEL_V1_CID);
  assert_eq!(t.index(), 100);
  assert_eq!(t.strand_cid().to_string(), STRAND_V1_CID);
  assert_eq!(t.spec_str(), "twine/1.0.x");
  assert_eq!(t.version(), Version::new(1, 0, 0));
  assert_eq!(t.subspec(), None);
  assert_eq!(t.drop_index(), 0);
  assert_eq!(t.back_stitches().len(), 2);
  assert_eq!(t.cross_stitches().len(), 1);
  assert!(t.previous().is_some());
  let p: PayloadV1 = t.extract_payload().unwrap();
  assert_eq!(p.timestamp, "2023-10-26T21:25:56.936Z");
}

#[test]
fn parity_tixel_v2_accessors() {
  let t = tixel_v2();
  assert_eq!(t.cid().to_string(), TIXEL_V2_CID);
  assert_eq!(t.index(), 0);
  assert_eq!(t.strand_cid().to_string(), STRAND_V2_CID);
  assert_eq!(t.spec_str(), "twine/2.0.0/time/1.0.0");
  assert_eq!(t.version(), Version::new(2, 0, 0));
  assert_eq!(t.subspec().map(|x| x.to_string()), Some("time/1.0.0".to_string()));
  assert_eq!(t.drop_index(), 0);
  assert_eq!(t.back_stitches().len(), 0);
  assert_eq!(t.cross_stitches().len(), 0);
  assert!(t.previous().is_none());
  let p: PayloadV2 = t.extract_payload().unwrap();
  assert_eq!(p.timestamp, 1734743590);
}

// ---------- Security / integrity behaviors ----------

#[test]
fn parity_verify_tixel_v1_ok() {
  assert!(strand_v1().verify_tixel(&tixel_v1()).is_ok());
}

#[test]
fn parity_verify_tixel_v2_ok() {
  assert!(strand_v2().verify_tixel(&tixel_v2()).is_ok());
}

#[test]
fn parity_verify_tixel_bad_signature_errors() {
  let tixel = Tixel::from_tagged_dag_json(INVALID_SIGNATURE_TIXELJSON).unwrap();
  assert!(strand_v1().verify_tixel(&tixel).is_err());
}

#[test]
fn parity_verify_tixel_wrong_strand_errors() {
  // a v2 tixel against a v1 strand: strand_cid mismatch must be rejected
  assert!(strand_v1().verify_tixel(&tixel_v2()).is_err());
  assert!(strand_v2().verify_tixel(&tixel_v1()).is_err());
}

#[test]
fn parity_radix_one_strand_rejected() {
  // BADSTRANDJSON has links_radix = 1, which must fail verification on decode
  assert!(Strand::from_tagged_dag_json(BADSTRANDJSON).is_err());
  assert!(AnyTwine::from_tagged_dag_json(BADSTRANDJSON).is_err());
}

#[test]
fn parity_invalid_index_tixel_rejected() {
  assert!(Tixel::from_tagged_dag_json(INVALID_TIXELJSON).is_err());
}

#[test]
fn parity_from_block_roundtrip_all() {
  // exercises CID recompute (v1) + verify_cid for every kind
  for (cid, bytes) in [
    { let s = strand_v1(); (s.cid(), s.bytes()) },
    { let s = strand_v2(); (s.cid(), s.bytes()) },
  ] {
    assert!(Strand::from_block(cid, bytes).is_ok());
  }
  for (cid, bytes) in [
    { let t = tixel_v1(); (t.cid(), t.bytes()) },
    { let t = tixel_v2(); (t.cid(), t.bytes()) },
  ] {
    assert!(Tixel::from_block(cid, bytes).is_ok());
  }
}

#[test]
fn parity_from_block_wrong_cid_rejected() {
  // claiming the strand's bytes have the tixel's CID must fail
  let t = tixel_v1();
  let s = strand_v1();
  assert!(Strand::from_block(t.cid(), s.bytes()).is_err());
}

#[test]
fn parity_json_roundtrip_all() {
  for json in [STRANDJSON, TIXELJSON, STRAND_V2_JSON, TIXEL_V2_JSON] {
    let twine = AnyTwine::from_tagged_dag_json(json).unwrap();
    let again = AnyTwine::from_tagged_dag_json(&twine.tagged_dag_json()).unwrap();
    assert_eq!(twine, again);
  }
}

#[test]
fn parity_twine_pairing() {
  let twine = Twine::try_new(strand_v1(), tixel_v1()).unwrap();
  assert_eq!(twine.previous(), twine.back_stitches().first().copied());
  // mismatched pair must fail
  assert!(Twine::try_new(strand_v1(), tixel_v2()).is_err());
}
