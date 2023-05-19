use std::{collections::{HashMap, TryReserveError}};

use serde_ipld_dagcbor::EncodeError;
use twine_core::{twine::{Chain, Mixin, ChainContent, DEFAULT_SPECIFICATION}, verify::{verify_chain, ChainVerificationError}, utils::{chain_cid, CIDGenerationError}};
use josekit::{jws::{JwsSigner, JwsVerifier}, jwk::Jwk, JoseError};
use libipld::{cid::multihash, Ipld};
use libipld::cid::multihash::MultihashDigest;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChainBuilderError {
    #[error("Could not allocate space to serialize to DAG CBOR: {0}")]
    SerializationError(#[from] EncodeError<TryReserveError>),
    #[error("Could not generate a CID from this chain: {0}")]
    CIDGenerationError(#[from] CIDGenerationError),
    #[error("Cannot generate signature: {0}")]
    JoseError(#[from] JoseError),
    #[error("Could not verify chain: {0}")]
    ChainVerificationError(#[from] ChainVerificationError)
}

type Result<T, E = ChainBuilderError> = std::result::Result<T, E>;

pub struct ChainBuilder {
    content: ChainContent
}

// todo: should this be self-consuming
/// A self consuming builder for a chain
impl ChainBuilder {
    pub fn new(source: String, meta: HashMap<String, Ipld>, key: Jwk) -> Self {
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

    pub fn finalize(
        self,
        signer: &(dyn JwsSigner),
        verifier: &(dyn JwsVerifier), 
        hasher: multihash::Code // TODO: should hasher be a reference?
    ) -> Result<Chain, > {
        let signature = signer.sign(&hasher.digest(&serde_ipld_dagcbor::to_vec(&self.content)?).to_bytes())?;
        Ok(verify_chain(Chain {
            cid: chain_cid(&self.content, &signature, hasher)?,
            content: self.content,
            signature
        }, verifier)?)
    }
}

