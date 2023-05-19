use std::collections::TryReserveError;
use josekit::{jws::JwsVerifier, JoseError};
use libipld::multihash::MultihashDigest;
use crate::{twine::{Pulse, Chain, Mixin}, utils::{hasher_of, pulse_cid, chain_cid, CIDGenerationError}};
use thiserror::Error;


// Error types
#[derive(Error, Debug)]
pub enum SignatureVerificationError {
    #[error("Cannot infer hasher from CID of chain. Source: {0}")]
    UninferableHasher(#[from] libipld::multihash::Error),
    #[error("Cannot serialize into bytes before signing. Source: {0}")]
    Serializationerror(#[from] serde_ipld_dagcbor::EncodeError<TryReserveError>),
    #[error("signature verifier failed: {0}")]
    JoseError(#[from] JoseError)
}

#[derive(Debug, Error)]
pub enum PulseVerificationError {
    #[error("pulse's chain does not match the previous pulse's chain")]
    ChainMismatch,
    #[error("A mixin has the same CID as the chain of the pulse")]
    SameChainMixin,
    #[error("Mixin(s) of the previous pulse was excluded from this pulse (first, {0})")]
    PreviousMixinExclusion(Mixin),
    #[error("Previous pulse not in links (Pulse.content.previous)")]
    PreviousPulseExclusion,
    #[error("Bad Signature: {0}")]
    BadSignature(#[from] SignatureVerificationError),
    #[error("Could not generate a CID from this pulse: {0}")]
    CIDGenerationError(#[from] CIDGenerationError),
    #[error("pulse's CID does not match the CID we would expect to be generated from this pulse")]
    CidMismatch,
}

#[derive(Debug, Error)]
pub enum ChainVerificationError {
    #[error("Bad Signature: {0}")]
    BadSignature(#[from] SignatureVerificationError),
    #[error("Could not generate a CID from this pulse: {0}")]
    CIDGenerationError(#[from] CIDGenerationError),
    #[error("pulse's CID does not match the CID we would expect to be generated from this pulse")]
    CidMismatch,
    #[error("")]
    UninferableHasher,

}

pub fn verify_pulse(pulse: Pulse, previous: Option<&Pulse>, verifier: &(dyn JwsVerifier)) -> Result<Pulse, PulseVerificationError> {
    if let Some(prev_pulse) = previous {
        if prev_pulse.content.chain != pulse.content.chain { 
            return Err(PulseVerificationError::ChainMismatch);
        }

        for mixin in prev_pulse.content.mixins.iter() {
            if !pulse.content.mixins.contains(mixin) {
                return Err(PulseVerificationError::PreviousMixinExclusion(mixin.clone()));
            }
        }
        
        if !pulse.content.previous.contains(&prev_pulse.cid) {
            return Err(PulseVerificationError::PreviousPulseExclusion);
        }
    }

    for mixin in pulse.content.mixins.iter() {
        if mixin.chain == pulse.content.chain {
            return Err(PulseVerificationError::SameChainMixin);
        }
    }
    
    let hasher = match hasher_of(&pulse.cid) {
        Ok(v) => v,
        Err(e) => Err(SignatureVerificationError::UninferableHasher(e))?
    };
    let serialized = match serde_ipld_dagcbor::to_vec(&pulse.content) {
        Ok(v) => v,
        Err(e) => Err(SignatureVerificationError::Serializationerror(e))?
    };
    match verifier.verify(&hasher.digest(&serialized).to_bytes(), &pulse.signature) {
        Ok(_) => (),
        Err(e) => Err(SignatureVerificationError::JoseError(e))?
    };

    // TODO: add in check for signer matching key:
    /*  
    if signer.public_key() != self.key {
        return Err(Box::new(Err::KeyError))
    } */

    let cid = pulse_cid(&pulse.content, &pulse.signature)?;
    if cid != pulse.cid {
        return Err(PulseVerificationError::CidMismatch)
    }

    Ok(pulse)
}

pub fn verify_chain(chain: Chain, verifier: &(dyn JwsVerifier)) -> Result<Chain, ChainVerificationError> {
    let hasher = match hasher_of(&chain.cid) {
        Ok(v) => v,
        Err(e) => Err(SignatureVerificationError::UninferableHasher(e))?
    };
    let serialized = match serde_ipld_dagcbor::to_vec(&chain.content) {
        Ok(v) => v,
        Err(e) => Err(SignatureVerificationError::Serializationerror(e))?
    };
    match verifier.verify(&hasher.digest(&serialized).to_bytes(), &chain.signature) {
        Ok(_) => (),
        Err(e) => Err(SignatureVerificationError::JoseError(e))?
    };

    // TODO: add in check for signer matching key:
    /*  
    if signer.public_key() != self.key {
        return Err(Box::new(Err::KeyError))
    } */

    // cid
    let cid = chain_cid(&chain.content, &chain.signature, hasher)?;
    if cid != chain.cid {
        return Err(ChainVerificationError::CidMismatch);
    }
    
    Ok(chain)
}