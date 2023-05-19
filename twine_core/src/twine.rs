//! Structs and traits common to both Chain's and Pulses

use std::{collections::HashMap, fmt::Display};
use josekit::{jwk::Jwk};
use libipld::{Ipld, Cid};
use serde::{Serialize, Deserialize};


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
    pub signature: Vec<u8>,
    #[serde(rename = "/")]
    pub cid: Cid
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Pulse {
    pub content: PulseContent,
    pub signature: Vec<u8>,
    #[serde(rename = "/")]
    pub cid: Cid
}

trait Twine {
    /// Recompute the cid
    fn from_json() -> Self; 
}
