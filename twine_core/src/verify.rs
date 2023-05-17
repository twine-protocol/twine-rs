use std::fmt;

use josekit::jws::JwsVerifier;
use libipld::{Cid, multihash::{Code, MultihashDigest}};

use crate::twine::{Pulse, Chain, ChainHashable};


// Error types
#[derive(Debug)]
pub enum PulseVerificationError {
    ChainMismatch(Cid, Cid),
    TwineVersionMismatch(String, String),
    SameChainMixin,
    PreviousMixinExclusion,
    PreviousPulseExclusion,
    BadSignature,
    CidMismatch,
    UninferableHasher
}
type PulseErr = PulseVerificationError;
impl fmt::Display for PulseVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for PulseVerificationError {}

#[derive(Debug)]
pub enum ChainVerificationError {
    BadSignature,
    CidMismatch,
    UninferableHasher
}
type ChainErr = ChainVerificationError;
impl fmt::Display for ChainVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", v)
    }
}
impl std::error::Error for ChainVerificationError {}

/// A helper function that gets the hasher that produced a given CID.
pub fn hasher_of(&cid: &Cid) -> Result<Code, libipld::multihash::Error> {
    Code::try_from(cid.hash().code())
}

pub fn verify_pulse(pulse: Pulse, &previous: Pulse, verifier: &(dyn JwsVerifier)) -> Result<Pulse, PulseVerificationError> {
    if previous.content.chain != pulse.content.chain { 
        return Err(PulseErr::ChainMismatch(previous.content.chain, pulse.content.chain));
    }

    if previous.content.version != pulse.content.version {
        return Err(PulseErr::TwineVersionMismatch(pulse.content.version, pulse.content.version));
    }

    // mixins
    for mixin in pulse.content.mixins.iter() {
        if mixin.chain != pulse.content.chain {
            return Err(PulseErr::SameChainMixin);
        }
    }

    if previous.content.mixins.any(|mixin| !pulse.content.mixins.contains(mixin)) { 
        return Err(PulseErr::PreviousMixinExclusion);
    }

    // links
    if !pulse.content.previous.contains(&previous.cid) {
        return Err(PulseErr::PreviousPulseExclusion);
    }

    // todo: should we check that source is the same as a chain
    
    // signature
    let hasher = hasher_of(&pulse.cid).or(Err(PulseErr::UninferableHasher))?; // TODO: safe to assume the same hasher is used for cid and signature?
    verifier.verify(
        &hasher.digest(&serde_ipld_dagcbor::to_vec(&pulse.content)?).to_bytes(), 
        &pulse.signature
    ).or(Err(PulseErr::BadSignature))?;

    // TODO: add in check for signer matching key:
    /*  
    if signer.public_key() != self.key {
        return Err(Box::new(Err::KeyError))
    } */

    // cid
    let cid = self.hasher.digest(&serde_ipld_dagcbor::to_vec(
        &(PulseHashable {
            content: self.content,
            signature
        })
    )?);
    if cid != pulse.cid {
        return Err(PulseErr::CidMismatch)
    }

    Ok(pulse)
}

pub fn verify_chain(chain: Chain, verifier: &(dyn JwsVerifier)) -> Result<Chain, ChainVerificationError> {
    // signature
    let hasher = hasher_of(&chain.cid).or(Err(PulseErr::UninferableHasher))?; // TODO: safe to assume the same hasher is used for cid and signature?
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
    let cid = hasher.digest(&serde_ipld_dagcbor::to_vec(
        &(ChainHashable {
            content: chain.content,
            signature: chain.signature
        })
    )?);
    if Cid::new_v1(0, cid) != chain.cid {
        return Err(ChainErr::CidMismatch);
    }

    //  TODO: specification field (in libp2p format)

    Ok(chain)
}