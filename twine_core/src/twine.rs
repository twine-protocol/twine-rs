//! Structs and traits common to both Chain's and Pulses

use std::collections::HashMap;
use josekit::{jwk::Jwk, jws::JwsCondensed};
use libipld::{Ipld, Cid, cid::multihash};
use serde::{Serialize, Deserialize};
use crate::sign::Signer;

pub const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?


#[derive(Debug)]
pub enum TwineError {}

pub struct Mixin {
    chain: Cid,
    value: Cid
}

pub struct ChainContent {
    pub source: String, // TODO: should these be public
    pub specification: String,
    pub radix: u32,
    pub key: Jwk, 
    pub mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    pub meta: HashMap<String, Ipld>, // TODO: should be a map?
}

type Payload = HashMap<String, dyn Into<Ipld>>;

pub struct PulseContent {
    source: String,
    chain: Cid,
    index: u32, // note: DAG-CBOR supports i64, but we don't
    previous: Vec<Cid>,
    mixins: Vec<Cid>, // we check that these links are not on the same chain at runtime
    payload: Payload // TODO: is payload supposed to be a Map? (see specs/twine/data-structures.md)
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize, Deserialize)]
pub(crate) struct ChainHashable {
    content: ChainContent,
    signature: JwsCondensed
}

#[derive(Serialize, Deserialize)]
pub struct Chain {
    pub content: ChainContent,
    pub signature: JwsCondensed,
    pub cid: Cid
}

#[derive(Serialize, Deserialize)]
pub struct Pulse {
    content: PulseContent,
    signature: JwsCondensed,
    cid: Cid
}

impl Chain {
    pub fn builder(source: String) -> ChainContent {
        ChainContent::new(source)
    }

    pub fn create_pulse(
        &self, 
        previous: Option(Pulse),
        mixins: Vec<Mixin>,
        payload: Payload,
        signer: dyn Signer,
        hasher: multihash::Code
    ) -> Result<Pulse, TwineError> {
        // validate payload
        

        // validate mixins (are from different chains)
        // validate previous (are on the same chain)
        // validate signer matches chain key
        // check that chain spec matches current spec
        // generate PulseContent using source etc from the chain
        // generate signature from hash of dag-cbor of content
        // generate cid from hash of dag-cbor of content and signature
    }
}