//! Structs and traits common to both Chain's and Pulses

use std::{collections::HashMap, fmt::{Display, Write}, io::Read, error::Error};
use josekit::{jwk::Jwk};
use libipld::{Ipld, Cid};
use serde::{Serialize, Deserialize};
use serde_ipld_dagcbor::{DecodeError, EncodeError};
use crate::serde_utils::bytes_base64;
use serde_json::Error as JsonError;

pub const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct Mixin {
    pub chain: Cid,
    pub value: Cid
}
impl Display for Mixin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct ChainContent {
    pub source: String, // TODO: should these be public
    pub specification: String,
    pub radix: u32,
    pub key: Jwk, 
    pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    pub meta: HashMap<String, Ipld>, // TODO: should be a map?
}

type Payload = HashMap<String, Ipld>;

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct PulseContent {
    pub source: String,
    pub chain: Cid,
    pub index: u32, // note: DAG-CBOR supports i64, but we don't
    pub previous: Vec<Cid>,
    pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    pub payload: Payload // TODO: is payload supposed to be a Map? (see specs/twine/data-structures.md)
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize)]
pub(crate) struct ChainHashable<'a> {
    pub content: &'a ChainContent,
    pub signature: &'a Vec<u8>
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize)]
pub(crate) struct PulseHashable<'a> {
    pub content: &'a PulseContent,
    pub signature: &'a Vec<u8>
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Chain {
    pub content: ChainContent,
    #[serde(with="bytes_base64")]
    pub signature: Vec<u8>,
    #[serde(rename = "/")]
    pub cid: Cid
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Pulse {
    pub content: PulseContent,
    #[serde(with="bytes_base64")]
    pub signature: Vec<u8>,
    #[serde(rename = "/")]
    pub cid: Cid
}


pub trait Twine {
    /// Decode from DAG-JSON
    fn from_json(json: String) -> Result<Self, JsonError> where Self: Sized; 

    /// Decode from DAG-JSON file
    fn from_json_reader<R: Read>(rdr: R) -> Result<Self, JsonError> where Self: Sized { todo!() } 

    /// Encode to DAG-JSON
    fn to_json(&self) -> Result<String, JsonError>;

    /// Write DAG-JSON
    fn to_json_writer<W: Write>(&self, wtr: W) -> Result<(), JsonError> { todo!() } 

    /// Decode from DAG-CBOR
    fn from_cbor(bytes: &[u8]) -> Result<Self, DecodeError<&Box<dyn Error>>> where Self: Sized { todo!() } // TODO: change the Error type

    /// Decode from DAG-CBOR file
    fn from_cbor_reader<R: Read>(rdr: R) -> Result<Self, DecodeError<&'static Box<dyn Error>>> where Self: Sized { todo!() }

    /// Encode to DAG-CBOR
    fn to_cbor<W: Write>() -> Result<Vec<u8>, EncodeError<&'static Box<dyn Error>>> { todo!() } 

    /// Write DAG-CBOR
    fn to_cbor_writer<W: Write>(&self, wtr: W) -> Result<(), EncodeError<&'static Box<dyn Error>>> { todo!() } 
}

impl Twine for Pulse {
    fn from_json(json: String) -> Result<Self, JsonError> {
        serde_json::from_str(&json)
    }

    fn to_json(&self) -> Result<String, JsonError> {
        serde_json::to_string(self)
    }
}

impl Twine for Chain {
    fn from_json(json: String) -> Result<Self, JsonError> {
        serde_json::from_str(&json)
    }

    fn to_json(&self) -> Result<String, JsonError> {
        serde_json::to_string(self)
    }
}