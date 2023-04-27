use std::{collections::HashMap, fmt::{Display, self}};

use libipld::{Ipld, multihash::Code, Cid};
use libipld::cid::multihash::MultihashDigest;
use twine_core::twine::{PulseContent, Chain, Pulse, DEFAULT_SPECIFICATION, Mixin, PulseHashable};
use serde::{ser, de};
use crate::util::hasher_of;
use josekit::{jws::JwsSigner, jwk::Jwk};

#[derive(Debug)]
pub enum PulseBuilderError {
    Serde(String), // serde errors
    InvalidLink(String),
    InvalidMixin(String),
    MismatchedKeys,
    UninferableHasher,
    MismatchedVersion,
    KeyError,
}

impl fmt::Display for PulseBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PulseBuilderError::Serde(reason) => write!(f, "Serde: {}", reason),
            v => write!(f, "{:?}", v)

        }
    }
}

impl std::error::Error for PulseBuilderError {}


type Err = PulseBuilderError;
type Result<T, E = PulseBuilderError> = std::result::Result<T, E>;


impl ser::Error for PulseBuilderError {
    fn custom<T: Display>(msg: T) -> Self {
        PulseBuilderError::Serde(msg.to_string())
    }
}

impl de::Error for PulseBuilderError {
    fn custom<T: Display>(msg: T) -> Self {
        PulseBuilderError::Serde(msg.to_string())
    }
}

pub struct PulseBuilder {
    content: PulseContent,
    hasher: Code,
    key: Jwk // this is a private field, so users can't change the key
}


// TODO: should this be self-consuming?
/// A self-consuming builder for pulses
impl PulseBuilder {
    pub fn new(chain: Chain, previous: Pulse) -> Result<Self> {
        if previous.content.chain != chain.cid { 
            return Err(Err::InvalidLink(String::from("The chain and previous pulse's chain do not match"))) 
        }
        if chain.content.specification != DEFAULT_SPECIFICATION { return Err(Err::MismatchedVersion) }
        Ok(Self { 
            content: PulseContent {
                source: chain.content.source,
                previous: vec![previous.cid],
                chain: chain.cid,
                index: previous.content.index + 1,
                mixins: previous.content.mixins.clone(),
                payload: HashMap::new(),
            },
            hasher: match hasher_of(chain.cid) {
                Err(_) => return Err(Err::UninferableHasher),
                Ok(v) => v
            },
            key: chain.content.key
        })
    }

    pub fn first(chain: Chain) -> Result<Self, Err> {
        Ok(Self {
            content: PulseContent { 
                source: chain.content.source,
                chain: chain.cid, 
                index: 1, // TODO: 0 or 1
                previous: Vec::new(),
                mixins: Vec::new(),
                payload: HashMap::new()
            },
            hasher: match hasher_of(chain.cid) {
                Err(_) => return Err(Err::UninferableHasher),
                Ok(v) => v
            }, // TODO: duplicate code
            key: chain.content.key
        })
    }

    pub fn source(mut self, source: String) -> Result<Self> {
        self.content.source = source;
        Ok(self)
    }

    pub fn mixin(mut self, mixin: Mixin) -> Result<Self> {
        if mixin.chain == self.content.chain {
            return Err(Err::InvalidMixin(String::from("Mixin points back to the chain of this pulse")))
        }
        self.content.mixins.push(mixin);
        Ok(self)
    }


    pub fn mixins(self, mixins: Vec<Mixin>) -> Result<Self> {
        mixins.into_iter().fold(
            Ok(self), 
            |builder, mixin| Ok(builder?.mixin(mixin)?) // TODO: manual deref
        )
    }

    pub fn link(mut self, prev: Pulse) -> Result<Self, Err> {
        if prev.content.chain != self.content.chain {
            return Err(Err::InvalidLink(String::from("Chain of link doesn't match")))
        }
        self.content.previous.push(prev.cid);
        Ok(self)
    }

    pub fn links(self, prevs: Vec<Pulse>) -> Result<Self, Err> {
        // return builder after links have been added one by one
        prevs.into_iter().fold(
            Ok(self), 
            |builder, prev| Ok(builder?.link(prev)?)
        )
    }

    pub fn payload(mut self, payload: HashMap<String, Ipld>) -> Result<Self> {
        self.content.payload.extend(payload);
        Ok(self)
    }

    pub fn finalize(self, signer: &(dyn JwsSigner)) -> Result<Pulse, Box<dyn std::error::Error>> {
        // TODO: add in check for signer matching key:
        /*  
        if signer.public_key() != self.key {
            return Err(Box::new(Err::KeyError))
        } */

        let signature = signer.sign(
            &self.hasher.digest(&serde_ipld_dagcbor::to_vec(&self.content)?).to_bytes()
        )?;
        let hashable = PulseHashable {
            content: self.content,
            signature
        };
        let cid = self.hasher.digest(&serde_ipld_dagcbor::to_vec(
            &hashable
        )?);

        Ok(Pulse {
            content: hashable.content,
            signature: hashable.signature,
            cid: Cid::new_v0(cid)?
        })
    }
}
