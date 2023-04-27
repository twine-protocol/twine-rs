//! Structs and traits common to both Chain's and Pulses

use std::collections::HashMap;
use josekit::{jwk::Jwk};
use libipld::{Ipld, Cid};
use serde::{Serialize, Deserialize};

pub const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?

#[derive(Debug)]
pub enum TwineError {
    ChainError,
    ResolutionError,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Mixin {
    pub chain: Cid,
    pub value: Cid
}

#[derive(Deserialize, Serialize)]
pub struct ChainContent {
    pub source: String, // TODO: should these be public
    pub specification: String,
    pub radix: u32,
    pub key: Jwk, 
    pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    pub meta: HashMap<String, Ipld>, // TODO: should be a map?
}

type Payload = HashMap<String, Ipld>;

#[derive(Deserialize, Serialize, Clone)]
pub struct PulseContent {
    pub source: String,
    pub chain: Cid,
    pub index: u32, // note: DAG-CBOR supports i64, but we don't
    pub previous: Vec<Cid>,
    pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    pub payload: Payload // TODO: is payload supposed to be a Map? (see specs/twine/data-structures.md)
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize, Deserialize)]
pub struct ChainHashable {
    pub content: ChainContent,
    pub signature: Vec<u8>
}

pub enum Twine {
    Chain(Chain),
    Pulse(Pulse)
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize, Deserialize)]
pub struct PulseHashable {
    pub content: PulseContent,
    pub signature: Vec<u8>
}

#[derive(Serialize, Deserialize)]
pub struct Chain {
    pub content: ChainContent,
    pub signature: Vec<u8>,
    pub cid: Cid
}

#[derive(Serialize, Deserialize)]
pub struct Pulse {
    pub content: PulseContent,
    pub signature: Vec<u8>,
    pub cid: Cid
}

impl Pulse {
    pub fn chain(&self) -> Option<Chain> {
        // do some resolution to get the chain or return None if it does not exist
        todo!()
    }
}