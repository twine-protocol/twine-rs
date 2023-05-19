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
    let verifier = alg.verifier_from_jwk(&keys.to_jwk_public_key())?;
    let hasher = multihash::Code::Sha3_512;
    let chain = ChainBuilder::new(
        "gold".into(), 
        HashMap::new()
    ).finalize(&keys.to_jwk_public_key(), &signer, &verifier, hasher)?;
    
    let jsonified = serde_json::to_string(&chain)?;
    print!("{}", jsonified);

    let roundtrip: Chain = serde_json::from_str(&jsonified)?;
    
    assert_eq!(chain, roundtrip);

    Ok(())
}