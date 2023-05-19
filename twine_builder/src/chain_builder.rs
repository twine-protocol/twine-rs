use std::{fmt::Display, collections::HashMap};

use serde_ipld_dagcbor::EncodeError;
use twine_core::{twine::{Chain, Mixin, ChainContent, DEFAULT_SPECIFICATION}, verify::verify_chain, utils::{chain_cid, CIDGenerationError}};
use josekit::{jws::{JwsSigner, JwsVerifier}, jwk::Jwk};
use libipld::{cid::{multihash, CidGeneric}, Ipld};
use libipld::cid::multihash::MultihashDigest;
use serde::{de, ser};
use std::fmt;
use thiserror::Error;

const DUMMY_KEY: Jwk = Jwk::generate_rsa_key(256); // TODO: this feels like a bad solution

#[derive(Debug, Error)]
pub enum ChainBuilderError {
    #[error("Could not allocate space to serialize to DAG CBOR: {0}")]
    SerializationError(#[from] EncodeError<TryReserveError>),
    #[error("Could not generate a CID from this pulse: {0}")]
    CIDGenerationError(#[from] CIDGenerationError),
    #[error("Signature verifier failed: {0}")]
    JoseError(#[from] JoseError)
}

type Result<T, E = ChainBuilderError> = std::result::Result<T, E>;

pub(crate) struct ChainBuilder {
    content: ChainContent
}

// todo: should this be self-consuming
/// A self consuming builder for a chain
impl ChainBuilder {
    pub fn new(source: String, meta: HashMap<String, Ipld>) -> Self {
        Self { 
            content: ChainContent {
                source,
                specification: DEFAULT_SPECIFICATION.to_string(), // Do not allow specification to be set
                radix: 32,
                mixins: Vec::new(),
                meta,
                key: DUMMY_KEY, 
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

    pub fn finalize(
        self,
        key: Jwk,
        signer: &(dyn JwsSigner),
        verifier: &(dyn JwsVerifier), 
        hasher: multihash::Code // TODO: should hasher be a reference?
    ) -> Result<Chain, > {
        self.content.key = key;
        let signature = signer.sign(&hasher.digest(&serde_ipld_dagcbor::to_vec(&self.content)?).to_bytes())?;
        Ok(verify_chain(Chain {
            content: self.content,
            signature, 
            cid: chain_cid(&content, &signature, self.hasher)?
        }, verifier))
    }
}

