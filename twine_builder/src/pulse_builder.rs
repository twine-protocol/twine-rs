use std::{collections::HashMap, fmt::{Display, self}};

use libipld::{Ipld, multihash::Code, Cid};
use libipld::cid::multihash::MultihashDigest;
use twine_core::{twine::{PulseContent, Chain, Pulse, DEFAULT_SPECIFICATION, Mixin, PulseHashable}, verify::{verify_pulse, hasher_of}};
use serde::{ser, de};
use josekit::{jws::{JwsSigner, JwsVerifier}, jwk::Jwk};

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
    key: Jwk, // this is a private field, so users can't change the key
    previous: &Pulse
}


/// A self-consuming builder for pulses
impl PulseBuilder {

    /// A helper function to create a PulseBuilder using new() or first()
    fn start_pulse(
        chain: &Chain, 
        &prev_chain: &Cid, 
        previous: Vec<Cid>, 
        index: u32, 
        mixins: Vec<Mixin>
    ) -> Result<Self> {
        Ok(Self { 
            content: PulseContent {
                source: chain.content.source.clone(),
                previous,
                chain: chain.cid,
                index,
                mixins,
                payload: HashMap::new(),
            },
            hasher: match hasher_of(&chain.cid) {
                Err(_) => return Err(Err::UninferableHasher),
                Ok(v) => v
            },
            key: chain.content.key.clone(),
            previous
        })
    }

    pub fn new(chain: &Chain, previous: &Pulse) -> Result<Self> {
        Self::start_pulse(
            chain, 
            &previous.content.chain,
            vec![previous.cid], 
            previous.content.index + 1, 
            previous.content.mixins.clone()
        )
    }

    pub fn first(chain: &Chain) -> Result<Self> {
        Self::start_pulse(
            chain, 
            &chain.cid, 
            Vec::new(), // TODO: 0 or 1
            1, 
            Vec::new()
        )
    }

    pub fn source(mut self, source: String) -> Result<Self> {
        self.content.source = source;
        Ok(self)
    }

    pub fn mixin(mut self, mixin: Mixin) -> Result<Self> {
        self.content.mixins.push(mixin);
        Ok(self)
    }


    pub fn mixins(self, mixins: Vec<Mixin>) -> Result<Self> {
        self.content.mixins.extend(mixins);
        Ok(self)
    }

    pub fn link(mut self, prev: Pulse) -> Result<Self, Err> {
        // TODO: this check is impossible to do given just a Pulse struct
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

    pub fn finalize(self, signer: &(dyn JwsSigner), verifier: &(dyn JwsVerifier)) -> Result<Pulse, Box<dyn std::error::Error>> {
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

        Ok(verify_pulse(Pulse {
            content: hashable.content,
            signature: hashable.signature,
            cid: Cid::new_v1(0, cid)
        }, self.previous, verifier)?)
    }
}
