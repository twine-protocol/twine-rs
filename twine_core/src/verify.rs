use std::collections::TryReserveError;
use josekit::{jws::JwsVerifier, JoseError};
use libipld::{Cid, multihash::MultihashDigest};
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
    #[error("pulse's chain (chain cid: {0}) does not match the previous pulse's chain ({1})")]
    ChainMismatch(Cid, Cid),
    #[error("A mixin ({0}) has the same CID as the chain of the pulse ({1})")]
    SameChainMixin(Mixin, Cid),
    #[error("Mixin(s) of the previous pulse was excluded from this pulse (first, {0})")]
    PreviousMixinExclusion(Mixin),
    #[error("Previous pulse not in links (Pulse.content.previous)")]
    PreviousPulseExclusion,
    #[error("Bad Signature: {0}")]
    BadSignature(#[from] SignatureVerificationError),
    #[error("Could not generate a CID from this pulse: {0}")]
    CIDGenerationError(#[from] CIDGenerationError),
    #[error("pulse's CID ({0}) does not match the expected CID to be generated from this pulse ({1})")]
    CidMismatch(Cid, Cid),
}
type PulseErr = PulseVerificationError;

#[derive(Debug, Error)]
pub enum ChainVerificationError {
    #[error("")]
    BadSignature,
    #[error("")]
    CidMismatch,
    #[error("")]
    UninferableHasher,

}
type ChainErr = ChainVerificationError;

pub fn verify_pulse(pulse: Pulse, previous: Option<&Pulse>, verifier: &(dyn JwsVerifier)) -> Result<Pulse, PulseVerificationError> {
    if let Some(prev_pulse) = previous {
        if prev_pulse.content.chain != pulse.content.chain { 
            return Err(PulseErr::ChainMismatch(prev_pulse.content.chain, pulse.content.chain));
        }

        if let Some(mixin) = prev_pulse.content.mixins.iter().filter(
            |mixin| !pulse.content.mixins.contains(mixin)
        ).next() { 
            return Err(PulseErr::PreviousMixinExclusion(mixin));
        }
        
        if !pulse.content.previous.contains(&prev_pulse.cid) {
            return Err(PulseErr::PreviousPulseExclusion);
        }
    }

    for mixin in pulse.content.mixins.iter() {
        if mixin.chain == pulse.content.chain {
            return Err(PulseErr::SameChainMixin(mixin, pulse.content.chain));
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
        return Err(PulseErr::CidMismatch(pulse.cid, cid))
    }

    Ok(pulse)
}

pub fn verify_chain(chain: Chain, verifier: &(dyn JwsVerifier)) -> Result<Chain, ChainVerificationError> {
    // signature
    let hasher = hasher_of(&chain.cid).or(Err(ChainVerificationError::UninferableHasher))?; // TODO: safe to assume the same hasher is used for cid and signature?
    verifier.verify(
        &hasher.digest(&serde_ipld_dagcbor::to_vec(&chain.content)?).to_bytes(), 
        &chain.signature
    ).or(Err(ChainErr::BadSignature))?;

    // TODO: add in check for signer matching key:
    /*  
    if signer.public_key() != self.key {
        return Err(Box::new(Err::KeyError))
    } */

    // cid
    let cid = chain_cid(&chain.content, &chain.signature, hasher)?;

    //  TODO: specification field (in libp2p format)

    Ok(chain)
}