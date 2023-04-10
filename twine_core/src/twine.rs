//! Structs and traits common to both Chain's and Pulses

use std::collections::HashMap;
use josekit::{jwk::Jwk, jws::JwsCondensed};
use libipld::{Ipld, Cid, cid::multihash};
use serde::{Serialize, Deserialize};
use crate::sign::Signer;

pub const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?

#[derive(Debug)]
pub enum TwineError {
    MixinError(String),
    KeyError(String),
    SpecificationError(String),
    ChainError,
    ResolutionError,
}

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

#[derive(Deserialize, Serialize)]
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
pub(crate) struct ChainHashable {
    content: ChainContent,
    signature: JwsCondensed
}

pub enum Twine {
    Chain(Chain),
    Pulse(Pulse)
}

/// A thin wrapper around content and signature used to create CIDs
#[derive(Serialize, Deserialize)]
pub(crate) struct PulseHashable {
    content: PulseContent,
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
    pub content: PulseContent,
    pub signature: JwsCondensed,
    pub cid: Cid
}

impl Chain {
    pub fn builder(source: String) -> ChainContent {
        ChainContent::new(source)
    }

    /// sugar for creating the first pulse on a chain
    pub fn first(
        &self, 
        mixins: Vec<Mixin>,
        payload: Payload,
        signer: dyn Signer,
        hasher: multihash::Code
    ) -> Result<Pulse, TwineError> {
        self.create_pulse(Vec::new(), mixins, payload, signer, hasher)
    }

    pub fn pulse(
        &self, 
        previous: Vec<Pulse>,
        mixins: Vec<Mixin>,
        payload: Payload,
        signer: dyn Signer,
        hasher: multihash::Code
    ) -> Result<Pulse, TwineError> {
        // validate mixins (are from different chains)
        if mixins.iter().any(|m| m.chain == self.cid) {
            return Err(TwineError::MixinError(String::from("mixins are from the same chain as the pulse you are trying to create")))
        }

        // validate previous (are on the same chain)
        if previous.iter().any(|p| p.chain != self) {
            return Err(TwineError::MixinError(String::from("previous should be from the same chain as the pulse you are trying to create")))
        }

        // validate signer matches chain key
        if signer.public_key() == self.content.key {
            return Err(TwineError::KeyError(String::from("signer should match key of chain")))
        }

        // check that chain spec matches current spec
        if DEFAULT_SPECIFICATION != self.content.specification {
            return Err(TwineError::SpecificationError(String::from("chain specification should match twine lib specification")))
        }

        // generate PulseContent using source etc from the chain
        let prev_index = previous.iter().map(|p| p.content.index).max().unwrap_or(0);
        let content = PulseContent {
            source: self.content.source,
            chain: self.cid,
            index: prev_index + 1,
            previous: previous.map(|p| p.cid),
            mixins,
            payload
        };
        // generate signature from hash of dag-cbor of content
        let signature = signer.sign(hasher.digest(serde_ipld_dagcbor::to_vec(&content)?))?;
        
        // generate cid from hash of dag-cbor of content and signature
        let cid = hasher.digest(serde_ipld_dagcbor::to_vec(
            &PulseHashable {
                content,
                signature
            }
        ));

        Ok(Pulse {
            content,
            signature,
            cid
        })
    }
}

impl Pulse {
    pub fn chain(&self) -> Option<Chain> {
        // do some resolution to get the chain or return None if it does not exist
    }
}