use std::collections::{HashMap, TryReserveError};

use josekit::JoseError;
use libipld::{Cid, Link}; // TODO: remove separate dependence on libipld
use libipld::{Ipld, multihash::Code};
use libipld::cid::multihash::MultihashDigest;
use linked_hash_map::LinkedHashMap;
use serde_ipld_dagcbor::EncodeError;
use twine_core::utils::{CIDGenerationError, pulse_cid};
use twine_core::{twine::{PulseContent, Chain, Pulse, Mixin}, verify::verify_pulse, utils::hasher_of};
use josekit::jws::{JwsSigner, JwsVerifier};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChainBuilderError {
    #[error("Could not allocate space to serialize to DAG CBOR: {0}")]
    SerializationError(#[from] EncodeError<TryReserveError>),
    #[error("Could not generate a CID from this chain: {0}")]
    CIDGenerationError(#[from] CIDGenerationError),
    #[error("Signature verifier failed: {0}")]
    JoseError(#[from] JoseError)
}

type Result<T, E = ChainBuilderError> = std::result::Result<T, E>;

pub struct PulseBuilder<'a> {
    content: PulseContent,
    previous: Option<&'a Pulse>,
    mixin_map: LinkedHashMap<Cid, Cid>
}

/// A self-consuming builder for pulses
impl<'a> PulseBuilder<'a> {
    /// A helper function to create a PulseBuilder using new() or first()
    fn start_pulse(
        chain: &Chain, 
        mixins: Vec<Mixin>,
        previous: Option<&'a Pulse>
    ) -> Self {
        Self { 
            content: PulseContent {
                source: chain.content.source.clone(),
                previous: previous.map_or(Vec::new(), |pulse| vec![pulse.cid]),
                chain: chain.cid,
                index: previous.map_or(1, |pulse| pulse.content.index + 1),
                mixins: Vec::new(),
                payload: HashMap::new(),
            },
            mixin_map: LinkedHashMap::from_iter(mixins.into_iter().map(|mixin| (mixin.chain, mixin.value))),
            previous,
        }
    }

    pub fn new(chain: &Chain, previous: &'a Pulse) -> Self {
        Self::start_pulse(
            chain, 
            previous.content.mixins.clone(), // TODO: should this be a clone?
            Some(previous)
        )
    }

    pub fn first(chain: &Chain) -> Self {
        Self::start_pulse(
            chain, 
            Vec::new(),
            None
        )
    }

    /// Upsert a mixin.
    /// This method inserts a mixin to the end of the sequence of mixins
    /// if a mixin of the same chain does not already exist.
    /// If a mixin with the same chain already exists (for example,
    /// because the previous pulse mixed in the chain) then the old mixin
    /// is updated, and no new mixin is added.
    pub fn mixin(mut self, mixin: Mixin) -> Self {
        self.mixin_map.insert(mixin.chain, mixin.value);
        self
    }

    /// Upsert a sequence of mixins.
    /// This method has the same behavior as repeatedly applying mixin
    /// for each element of the vector in sequence.
    pub fn mixins(mut self, mixins: Vec<Mixin>) -> Self {
        self.mixin_map.extend(mixins.into_iter().map(|mixin| (mixin.chain, mixin.value)));
        self
    }

    pub fn link(mut self, prev: Pulse) -> Self {
        self.content.previous.push(prev.cid);
        self
    }

    pub fn links(mut self, prevs: Vec<Pulse>) -> Self {
        self.content.previous.extend(prevs.iter().map(|prev| prev.cid));
        self
    }

    pub fn payload(mut self, payload: HashMap<String, Ipld>) -> Self {
        self.content.payload.extend(payload);
        self
    }

    pub fn finalize(
        mut self, 
        signer: &(dyn JwsSigner), 
        verifier: &(dyn JwsVerifier)
    ) -> Result<Pulse, Box<dyn std::error::Error>> {
        self.content.mixins.extend(self.mixin_map.into_iter().map(|(chain, value)| Mixin { chain, value } ));
        let hasher: Code = hasher_of(&self.content.chain)?;
        let signature = signer.sign(
            &hasher.digest(&serde_ipld_dagcbor::to_vec(&self.content)?).to_bytes()
        )?;

        Ok(verify_pulse(Pulse {
            cid: pulse_cid(&self.content, &signature)?,
            content: self.content,
            signature
        }, self.previous, verifier)?)
    }
}
