//! Structs and traits common to both Chain's and Pulses

use std::{collections::{HashMap, TryReserveError}, fmt::Display, io::{Read, Write, BufRead}, error::Error, convert::Infallible};
use josekit::jwk::Jwk;
use ipld_core::{cid::multihash::Multihash, codec::Codec};
use libipld::{ipld, multihash::MultihashDigest, Block, Cid};
use serde_ipld_dagjson::{codec::DagJsonCodec, error::CodecError as JsonCodecError};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_ipld_dagcbor::{codec::DagCborCodec, error::CodecError as CborCodecError, DecodeError, EncodeError};
use serde_json::Error as JsonError;
use super::Strand;
use super::Tixel;

pub const DEFAULT_SPECIFICATION: &str = env!("CARGO_PKG_VERSION"); // TODO / NOTE: this implies that we need cargo to build this

fn get_cid(dat: &[u8]) -> Cid {
  use libipld::multihash::{Code, MultihashDigest};
  let mh = Code::Sha3_512.digest(dat);
  Cid::new_v1(libipld::cbor::DagCborCodec.into(), mh)
}

#[derive(Debug, PartialEq)]
pub struct ParseError(pub String);

impl Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "ParseError: {}", self.0)
  }
}

impl From<JsonCodecError> for ParseError {
  fn from(e: JsonCodecError) -> Self {
    ParseError(format!("JsonCodecError: {}", e))
  }
}

impl From<CborCodecError> for ParseError {
  fn from(e: CborCodecError) -> Self {
    ParseError(format!("CborCodecError: {}", e))
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TwineContainer<C> {
  #[serde(skip)]
  pub cid: Cid,

  pub content: C,
  pub signature: String,
}

impl<C> TwineContainer<C> where C: Serialize + for<'de> Deserialize<'de> {

  pub fn from_parts(content: C, signature: String) -> Self {
    let mut twine = Self { cid: Cid::default(), content, signature };
    let dat = DagCborCodec::encode_to_vec(&twine).unwrap();
    twine.cid = get_cid(dat.as_slice());
    twine
  }

  pub fn from_dag_json<S: Display>(json: S) -> Result<Self, ParseError> {
    let j: TwineContainerJson<C> = DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?;
    let twine = Self::from_parts(j.data.content, j.data.signature);
    // if j.cid != twine.cid {
    //   return Err(ParseError(format!("Cid mismatch: expected {}, got {}", j.cid, twine.cid).into()));
    // }
    Ok(twine)
  }

  pub fn from_bytes<T: AsRef<[u8]>>(cid: Cid, bytes: T) -> Result<Self, ParseError> {
    let twine: Self = DagCborCodec::decode_from_slice(bytes.as_ref())?;
    let actual_cid = get_cid(DagCborCodec::encode_to_vec(&twine).unwrap().as_slice());
    if cid != actual_cid {
      return Err(ParseError(format!("Cid mismatch: expected {}, got {}", cid, actual_cid).into()));
    }
    Ok(twine)
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TwineContainerJson<C> {
  pub cid: Cid,
  pub data: TwineContainer<C>,
}

pub enum Twine {
  Strand(Strand),
  Tixel(Tixel),
}

mod old {
  use super::*;

// #[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
// pub struct Mixin {
//     pub chain: Cid,
//     pub value: Cid
// }
// impl Display for Mixin {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self)
//     }
// }
// impl From<Pulse> for Mixin {
//     fn from(value: Pulse) -> Self {
//         Self {
//             chain: value.content.chain,
//             value: value.cid
//         }
//     }
// }

// #[derive(Deserialize, Serialize, PartialEq, Debug)]
// pub struct ChainContent {
//     pub source: String,
//     pub specification: String,
//     pub radix: u32,
//     pub key: Jwk,
//     pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
//     pub meta: HashMap<String, Ipld>,
// }

// type Payload = HashMap<String, Ipld>;

// #[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
// pub struct PulseContent {
//     pub source: String,
//     pub chain: Cid,
//     pub index: u32, // note: DAG-CBOR supports i64, but we don't
//     pub previous: Vec<Cid>,
//     pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
//     pub payload: Payload
// }

// /// A thin wrapper around content and signature used to create CIDs
// #[derive(Serialize)]
// pub(crate) struct ChainHashable<'a> {
//     pub content: &'a ChainContent,
//     pub signature: &'a Vec<u8>
// }

// /// A thin wrapper around content and signature used to create CIDs
// #[derive(Serialize)]
// pub(crate) struct PulseHashable<'a> {
//     pub content: &'a PulseContent,
//     pub signature: &'a Vec<u8>
// }

// #[derive(Serialize, Deserialize, PartialEq, Debug)]
// pub struct Chain {
//     pub content: ChainContent,
//     #[serde(with="bytes_base64")]
//     pub signature: Vec<u8>, // TODO: signatures should be JWS, not bytes
//     #[serde(rename = "/")]
//     pub cid: Cid
// }

// #[derive(Serialize, Deserialize, PartialEq, Debug)]
// pub struct Pulse {
//     pub content: PulseContent,
//     #[serde(with="bytes_base64")]
//     pub signature: Vec<u8>,
//     #[serde(rename = "/")]
//     pub cid: Cid
// }

// pub trait Twine: Serialize + DeserializeOwned {}
// impl Twine for Pulse {}
// impl Twine for Chain {}

// /// Provide a subset of serde functionality in easy to use methods
// pub trait TwineSerialize {
//     /// Decode from DAG-JSON
//     fn from_json(json: String) -> Result<Self, JsonError> where Self: Sized;

//     /// Decode from DAG-JSON file
//     fn from_json_reader<R: Read>(rdr: R) -> Result<Self, JsonError> where Self: Sized;

//     /// Encode to DAG-JSON
//     fn to_json(&self) -> Result<String, JsonError>;

//     /// Write DAG-JSON
//     fn to_json_writer<W: Write>(&self, wtr: W) -> Result<(), JsonError>;

//     /// Decode from DAG-CBOR
//     fn from_cbor(bytes: &[u8]) -> Result<Self, DecodeError<Infallible>> where Self: Sized; // TODO: change the Error type

//     /// Decode from DAG-CBOR file
//     fn from_cbor_reader<R: BufRead>(rdr: R) -> Result<Self, DecodeError<std::io::Error>> where Self: Sized;

//     /// Encode to DAG-CBOR
//     fn to_cbor(&self) -> Result<Vec<u8>, EncodeError<TryReserveError>>;

//     /// Write DAG-CBOR
//     fn to_cbor_writer<W: Write>(&self, wtr: W) -> Result<(), EncodeError<std::io::Error>>;
// }

// impl<T> TwineSerialize for T where T: Twine {
//     fn from_json(json: String) -> Result<Self, JsonError> {
//         serde_json::from_str(&json)
//     }

//     fn to_json(&self) -> Result<String, JsonError> {
//         serde_json::to_string(self)
//     }

//     fn from_json_reader<R: Read>(rdr: R) -> Result<Self, JsonError> where Self: Sized {
//         serde_json::from_reader(rdr)
//     }

//     fn to_json_writer<W: Write>(&self, wtr: W) -> Result<(), JsonError> {
//         serde_json::to_writer(wtr, self)
//     }

//     fn from_cbor(bytes: &[u8]) -> Result<Self, DecodeError<Infallible>> where Self: Sized {
//         serde_ipld_dagcbor::from_slice(bytes)
//     }

//     fn from_cbor_reader<R: BufRead>(rdr: R) -> Result<T, DecodeError<std::io::Error>> where Self: Sized {
//         serde_ipld_dagcbor::from_reader(rdr)
//     }

//     fn to_cbor(&self) -> Result<Vec<u8>, EncodeError<TryReserveError>> {
//         serde_ipld_dagcbor::to_vec(self)
//     }

//     fn to_cbor_writer<W: Write>(&self, wtr: W) -> Result<(), EncodeError<std::io::Error>> {
//         serde_ipld_dagcbor::to_writer(wtr, self)
//     }
// }

}
