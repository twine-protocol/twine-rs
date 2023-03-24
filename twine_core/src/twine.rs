//! Structs and traits common to both Chain's and Pulses

use std::collections::HashMap;
use josekit::{jwk::Jwk, jws::JwsCondensed};
use libipld::{Ipld, Link, Cid, cid::multihash};
use serde::{Serialize, Deserialize};
use crate::sign::Signer;

const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?

type Payload = HashMap<String, dyn Into<Ipld>>;

pub struct Mixin {
    chain: Cid,
    value: Cid
}

pub struct ChainContent {
    source: String,
    specification: String,
    radix: i64, // TODO: sizing; TODO: links_radix instead?
    key: Jwk, 
    mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    meta: Ipld, // TODO: should be a map?
}

pub struct PulseContent {
    source: String,
    chain: Cid,
    index: u32, // note: DAG-CBOR supports i64, but we don't
    previous: Vec<Cid>,
    mixins: Vec<Cid>, // we check that these links are not on the same chain at runtime
    payload: Payload // TODO: is payload supposed to be a Map? (see specs/twine/data-structures.md)
}

#[derive(Serialize, Deserialize)]
pub struct Chain {
    content: ChainContent,
    signature: JwsCondensed,
    cid: Cid // hash of content and signature
}

#[derive(Serialize, Deserialize)]
pub struct Pulse {
    content: PulseContent,
    signature: JwsCondensed,
    cid: Cid
}

#[derive(Debug)]
pub enum TwineError {}

enum TwineContent { // TODO: should I remove this enum?
    Chain(ChainContent),
    Pulse(PulseContent)
}

impl TwineContent {
    fn is_valid(&self) -> bool {}
}

trait Twine {
    fn sign(&self) -> &JwsCondensed;
    fn create_cid(&self) -> Cid;
}

impl Twine for Chain {
    fn signature(&self) -> &JwsCondensed { &self.signature }
    fn create_cid(&self) -> Cid {}
}

impl Twine for Pulse {
    fn signature(&self) -> &JwsCondensed {  }
    fn create_cid(&self) -> Cid {}
}

impl Chain {
    fn build_chain(&self) -> Result<Chain, TwineError> {}
    fn create_pulse(
        &self, 
        previous: Option(Pulse),
        mixins: Vec<Mixin>,
        payload: Payload,
        signer: Signer,
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