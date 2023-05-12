use std::{collections::HashMap, error::Error};
use josekit::{jws::alg::eddsa::EddsaJwsAlgorithm, jwk::alg::ed::EdCurve::Ed25519};
use libipld::multihash;
use twine_builder::{ChainBuilder};
use twine_core::twine::Chain;

fn main() -> Result<(), Box<dyn Error>> {
    // create a chain
    let alg = EddsaJwsAlgorithm::Eddsa;
    let keys = alg.generate_key_pair(Ed25519)?;
    let signer = alg.signer_from_jwk(&keys.to_jwk_private_key())?;
    let hasher = multihash::Code::Sha3_512;
    let chain = ChainBuilder::new(
        "gold".into(), 
        keys.to_jwk_public_key(), 
        HashMap::new()
    ).finalize(&signer, hasher)?;
    
    let roundtrip: Chain = serde_ipld_dagcbor::from_slice(&serde_ipld_dagcbor::to_vec(&chain)?)?;
    
    assert_eq!(chain, roundtrip);

    Ok(())
}