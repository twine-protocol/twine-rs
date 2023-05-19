use std::collections::{HashMap, TryReserveError};

use josekit::JoseError;
use libipld::{Ipld, multihash::Code};
use libipld::cid::multihash::MultihashDigest;
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
    previous: Option<&'a Pulse>
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
                mixins,
                payload: HashMap::new(),
            },
            previous
        }
    }

    pub fn new(chain: &Chain, previous: &'a Pulse) -> Self {
        Self::start_pulse(
            chain, 
            previous.content.mixins.clone(),
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

    pub fn source(mut self, source: String) -> Self {
        self.content.source = source;
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

    pub fn link(mut self, prev: Pulse) -> Self {
        self.content.previous.push(prev.cid);
        self
    }

    pub fn links(mut self, prevs: Vec<Pulse>) -> Self {
        // return builder after links have been added one by one
        self.content.previous.extend(prevs.iter().map(|prev| prev.cid));
        self
    }

    pub fn payload(mut self, payload: HashMap<String, Ipld>) -> Self {
        self.content.payload.extend(payload);
        self
    }

    pub fn finalize(self, signer: &(dyn JwsSigner), verifier: &(dyn JwsVerifier)) -> Result<Pulse, Box<dyn std::error::Error>> {
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
