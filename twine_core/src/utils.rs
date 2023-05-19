use std::collections::TryReserveError;

use josekit::jws::{JwsSigner, JwsVerifier};
use libipld::{Cid, multihash::{Code, MultihashDigest}};
use serde_ipld_dagcbor::EncodeError;

use crate::twine::{ChainContent, PulseContent, PulseHashable, ChainHashable};

use thiserror::Error;


/// A helper function that gets the hasher that produced a given CID.
pub fn hasher_of(&cid: &Cid) -> Result<Code, libipld::multihash::Error> {
    Code::try_from(cid.hash().code())
}


#[derive(Error, Debug)]
pub enum CIDGenerationError {
    #[error("hasher (hash function) cannot be inferred from pulse's chain")]
    UninferableHasher(#[from] libipld::multihash::Error), // Pulse only error
    #[error("could not allocate space to serialize to DAG CBOR ")]
    SerializationError(#[from] EncodeError<TryReserveError>)
}

pub fn pulse_cid(content: &PulseContent, signature: &Vec<u8>) -> Result<Cid, CIDGenerationError> {
    let hasher = hasher_of(&content.chain)?;
    Ok(Cid::new_v1(0, hasher.digest(&serde_ipld_dagcbor::to_vec(
        &(PulseHashable {
            content,
            signature
        })
    )?)))
}

pub fn chain_cid(content: &ChainContent, signature: &Vec<u8>, hasher: Code) -> Result<Cid, CIDGenerationError> {
    Ok(Cid::new_v1(0, hasher.digest(&serde_ipld_dagcbor::to_vec(
        &(ChainHashable {
            content,
            signature
        })
    )?)))
}