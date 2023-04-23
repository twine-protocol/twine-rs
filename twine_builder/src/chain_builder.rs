use std::{fmt::Display, collections::HashMap};

use twine_core::twine::{Chain, Mixin, ChainContent, DEFAULT_SPECIFICATION, ChainHashable};
use josekit::{jws::JwsSigner, jwk::Jwk};
use libipld::{cid::multihash, Ipld};
use serde::{de, ser};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ChainBuilderError {
    Serde(String),
}

impl fmt::Display for ChainBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChainBuilderError::Serde(reason) => write!(f, "Serde: {}", reason)
        }
    }
}

impl std::error::Error for ChainBuilderError {}

type Result<T, E = ChainBuilderError> = std::result::Result<T, E>;

impl ser::Error for ChainBuilderError {
    fn custom<T: Display>(msg: T) -> Self {
        ChainBuilderError::Serde(msg.to_string())
    }
}

impl de::Error for ChainBuilderError {
    fn custom<T: Display>(msg: T) -> Self {
        ChainBuilderError::Serde(msg.to_string())
    }
}


pub struct ChainBuilder {
    content: ChainContent
}

// todo: should this be self-consuming
/// A self consuming builder for a chain
impl ChainBuilder {
    pub fn new(source: String, key: Jwk, meta: HashMap<String, Ipld>) -> Self {
        Self { 
            content: ChainContent {
                source,
                specification: DEFAULT_SPECIFICATION.to_string(), // Do not allow specification to be set
                radix: 32,
                mixins: Vec::new(),
                meta,
                key,
            }
        }
    }

    pub fn source(mut self, source: String) -> Self {
        self.content.source = source;
        self
    }

    pub fn radix(mut self, radix: u32) -> Self {
        self.content.radix = radix;
        self
    }

    pub fn mixin(mut self, mixin: Mixin) -> Self {
        self.content.mixins.push(mixin);
        self
    }

    pub fn mixins(mut self, mixins: Vec<Mixin>) -> Self {
        self.content.mixins.extend(mixins);
        self
    }

    pub fn extend_meta(mut self, key: String, value: Ipld) -> Self {
        self.content.meta.insert(key, value);
        self
    }

    // TODO: should it totally replace the existing meta?
    pub fn meta(mut self, meta: HashMap<String, Ipld>) -> Self {
        self.content.meta = meta;
        self
    }

    pub fn key(mut self, key: Jwk) -> Self {
        self.content.key = key;
        self
    }

    pub fn finalize(self, signer: dyn JwsSigner, hasher: multihash::Code) -> Result<Chain> {
        // Note: we do not check that chain spec matches current spec
        let signature = signer.sign(hasher.digest(serde_ipld_dagcbor::to_vec(&self.content)?))?;
        let cid = hasher.digest(serde_ipld_dagcbor::to_vec(
            &ChainHashable {
                content: self.content,
                signature
            }
        )?);

        Ok(Chain {
            content: self.content,
            signature,
            cid
        });

    }
}

