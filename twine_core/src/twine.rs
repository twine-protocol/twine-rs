//! Structs and traits common to both Chain's and Pulses

use std::collections::HashMap;
use josekit::{jwk::Jwk, jws::JwsSigner};
use libipld::{Ipld, Cid, cid::multihash};
use serde::{Serialize, Deserialize};

pub const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?

#[derive(Debug)]
pub enum TwineError {
    MixinError(String),
    KeyError(String),
    SpecificationError(String),
    ChainError,
    ResolutionError,
}

#[derive(Deserialize, Serialize)]
pub struct Mixin {
    chain: Cid,
    value: Cid
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
    content: ChainContent,
    signature: Vec<u8>
}

pub enum Twine {
    Chain(Chain),
    Pulse(Pulse)
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize, Deserialize)]
pub struct PulseHashable {
    content: PulseContent,
    signature: Vec<u8>
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

impl Chain {
    pub fn builder(source: String, key: Jwk) -> ChainContent {
        ChainContent::new(source, key)
    }

    /// sugar for creating the first pulse on a chain
    pub fn first(
        &self, 
        mixins: Vec<Mixin>,
        payload: Payload,
        signer: dyn JwsSigner,
        hasher: multihash::Code
    ) -> Result<Pulse, TwineError> {
        self.create_pulse(Vec::new(), mixins, payload, signer, hasher)
    }
}

impl Pulse {
    pub fn chain(&self) -> Option<Chain> {
        // do some resolution to get the chain or return None if it does not exist
    }
}