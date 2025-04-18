use ipld_core::{codec::Codec, ipld};
use serde::Serialize;
use serde_ipld_dagjson::codec::DagJsonCodec;

use super::*;
use crate::twine::*;

#[test]
fn test_deserialize_tixel_json() {
  let res = Tixel::from_tagged_dag_json(TIXELJSON);
  dbg!(&res);
  assert!(res.is_ok(), "Failed to deserialize Tixel: {:?}", res.err());
}

#[test]
fn test_invalid_signature_tixel_json() {
  let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
  let tixel = Tixel::from_tagged_dag_json(INVALID_SIGNATURE_TIXELJSON).unwrap();
  let res = strand.verify_tixel(&tixel);
  dbg!(&res);
  assert!(res.is_err(), "Signature verification should have failed");
}

#[test]
fn test_deserialize_strand_json() {
  let res = Strand::from_tagged_dag_json(STRANDJSON);
  dbg!(&res);
  assert!(res.is_ok(), "Failed to deserialize Strand: {:?}", res.err());
}

#[test]
fn test_deserialize_tixel_bytes() {
  let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
  let bytes = tixel.bytes();
  let res = Tixel::from_block(tixel.cid(), bytes);
  dbg!(&res);
  assert!(
    res.is_ok(),
    "Failed to deserialize Tixel from bytes: {:?}",
    res.err()
  );
}

#[test]
fn test_deserialize_strand_bytes() {
  let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
  let res = Strand::from_block(strand.cid(), strand.bytes());
  // dbg!(&res);
  assert!(
    res.is_ok(),
    "Failed to deserialize Strand from bytes: {:?}",
    res.err()
  );
}

#[test]
fn test_deserialize_generic() {
  let twine = AnyTwine::from_tagged_dag_json(STRANDJSON);
  assert!(
    twine.is_ok(),
    "Failed to deserialize Strand: {:?}",
    twine.err()
  );
  assert!(twine.unwrap().is_strand(), "Twine is not a Strand");
}

#[test]
fn test_deserialize_generic_invalid() {
  let twine = AnyTwine::from_tagged_dag_json(BADSTRANDJSON);
  assert!(twine.is_err(), "Deserialization should have failed");
}

#[test]
fn test_in_out_json() {
  let twine = AnyTwine::from_tagged_dag_json(TIXELJSON).unwrap();
  let json = twine.tagged_dag_json();
  let twine2 = AnyTwine::from_tagged_dag_json(&json).unwrap();
  assert_eq!(twine, twine2, "Twine JSON roundtrip failed. Json: {}", json);
  assert!(twine2.is_tixel(), "Twine is not a Tixel");
}

#[test]
fn test_signature_verification() {
  let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();

  let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
  let res = strand.verify_tixel(&tixel);
  assert!(res.is_ok(), "Failed to verify signature: {:?}", res.err());
}

#[test]
fn test_decoding_fail() {
  let res = Tixel::from_tagged_dag_json(INVALID_TIXELJSON);
  assert!(res.is_err(), "Decoding should have failed");
}

#[test]
fn test_simple_payload_unpack() {
  use serde::{Deserialize, Serialize};
  #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
  struct Timestamped {
    timestamp: String,
  }

  // let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
  let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
  let t: Timestamped = tixel.extract_payload().unwrap();
  assert_eq!(t.timestamp, "2023-10-26T21:25:56.936Z");
}

#[test]
fn test_twine() {
  let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
  let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
  let twine = Twine::try_new(strand, tixel).unwrap();
  assert_eq!(twine.previous(), twine.back_stitches().first().copied());
}

#[test]
fn test_null_payload() {
  let ipld = ipld!({
    "payload": {
      "baz": null
    }
  });
  let encoded = DagJsonCodec::encode_to_vec(&ipld).unwrap();
  let decoded = DagJsonCodec::decode_from_slice(&encoded).unwrap();
  assert_eq!(ipld, decoded);
}

#[test]
fn test_roundtrip_null() {
  let test = ipld!({
    "test": null
  });
  let s = DagJsonCodec::encode_to_vec(&test).unwrap();
  let decoded: ipld_core::ipld::Ipld = DagJsonCodec::decode_from_slice(&s).unwrap();
  assert_eq!(test, decoded);
}

#[test]
fn test_bytes() {
  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  struct MyStruct {
    foo: crate::Bytes,
  }

  let test = MyStruct {
    foo: vec![1, 2, 3, 4, 5].into(),
  };

  let s = DagJsonCodec::encode_to_vec(&test).unwrap();
  dbg!(String::from_utf8(s.clone()).unwrap());
  let decoded: MyStruct = DagJsonCodec::decode_from_slice(&s).unwrap();
  assert_eq!(test, decoded);
}

#[test]
fn test_deserialize_strand_v2() {
  let res = Strand::from_tagged_dag_json(STRAND_V2_JSON);
  assert!(res.is_ok(), "Failed to deserialize Strand: {:?}", res.err());
  println!("{}", res.unwrap().tagged_dag_json_pretty());
}
