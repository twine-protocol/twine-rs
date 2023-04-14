use std::{collections::HashMap, hash::Hash};

use libipld::{Ipld, multihash::{Hasher, Code}, Cid, cid::CidGeneric};
use twine_core::twine::{PulseContent, Chain, Pulse, DEFAULT_SPECIFICATION};
use serde::{ser, de};
use crate::util::hasher_of;
use josekit::{jws::JwsSigner, jwk::Jwk};

#[derive(Debug)]
pub enum PulseBuilderError {
    Serde(String), // serde errors
    InvalidLink,
    InvalidMixin(String),
    MismatchedKeys,
    MismatchedVersion,
}
pub type Result<T> = Result<T, PulseBuilderError>;

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
    hasher: Hasher,
    key: Jwk // this is a private field, so users can't change the key
}


// TODO: should this be self-consuming?
/// A self-consuming builder for pulses
impl PulseBuilder {
    pub fn new(chain: Chain, previous: Pulse) -> Result<Self, Err> {
        if previous.content.chain != chain.cid { return Err(Err::InvalidLink) }
        if chain.content.specification != DEFAULT_SPECIFICATION { return Err(Err::MismatchedVersion) }
        Ok(Self { 
            content: PulseContent {
                source: chain.content.source,
                previous: vec![previous.cid],
                chain,
                index: previous.content.index + 1,
                mixins: Vec::new(),
                payload: HashMap::new(),
            },
            hasher: hasher_of(chain),
            key: chain.content.key
        })
    }

    pub fn first(chain: Chain) -> Result<Self, Err> {
        Ok(Self {
            content: PulseContent { 
                source: Chain.content.source,
                chain: chain.cid, 
                index: 1, // TODO: 0 or 1
                previous: Vec::new(),
                mixins: Vec::new(),
                payload: HashMap::new()
            },
            hasher: hasher_of(chain)?,
            key: chain.content.key
        })
    }

    pub fn source(mut self, source: String) -> Result<Self, Err> {
        self.content.source = source;
        Ok(self)
    }

    pub fn mixin(mut self, mixin: Mixin) -> Result<Self, Err> {
        if mixin.chain == self.content.chain.cid {
            return Err(Err::InvalidMixin(String::from("Mixin points back to the chain of this pulse")))
        }
        self.content.mixins.push(mixin);
        Ok(self)
    }

    pub fn mixins(mut self, mixins: Mixin) -> Result<Self, Err> {
        Ok(mixins.iter().fold(self, |builder, mixin| builder.link(prev)?))
    }

    pub fn link(mut self, prev: Pulse) -> Result<Self, Err> {
        if (prev.content.chain != self.content.chain) {
            return Err(Err::InvalidLink(String::from("Chain of link doesn't match ")))
        }
        self.content.previous.push(pulse.cid);
        Ok(self)
    }

    pub fn links(mut self, prevs: Vec<Pulse>) -> Result<Self, Err> {
        // return builder after links have been added one by one
        Ok(prevs.iter().fold(self, |builder, prev| builder.link(prev)?))
    }

    pub fn payload(mut self, payload: HashMap<String, Ipld>) -> Result<Self, Err> {
        self.content.payload.extend(payload.iter());
        Ok(self)
    }

    pub fn finalize(self, signer: dyn JwsSigner) -> Result<Pulse, TwineError> {
        if signer.public_key() == self.key {
            return Err(Err::KeyError)
        }

        let signature = signer.sign(hasher.digest(serde_ipld_dagcbor::to_vec(&self.content)?))?;
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
