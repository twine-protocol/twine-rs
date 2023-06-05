use std::{collections::HashMap, error::Error};
use josekit::{jws::alg::eddsa::EddsaJwsAlgorithm, jwk::alg::ed::EdCurve::Ed25519};
use libipld::multihash;
use twine_builder::{ChainBuilder};
use twine_core::twine::Chain;

fn main() -> Result<(), Box<dyn Error>> {
    // create a chain
    let keys = EddsaJwsAlgorithm::Eddsa.generate_key_pair(Ed25519)?;
    let signer = EddsaJwsAlgorithm::Eddsa.signer_from_jwk(&keys.to_jwk_private_key())?;
    let verifier = EddsaJwsAlgorithm::Eddsa.verifier_from_jwk(&keys.to_jwk_public_key())?;
    let hasher = multihash::Code::Sha3_512; 
    let builder = ChainBuilder::new(
        "gold".into(),
        HashMap::new(),
        keys.to_jwk_public_key()
    );
    let chain = builder.finalize( &signer, &verifier, hasher)?;
    
    let jsonified = serde_json::to_string(&chain)?;
    print!("{}", jsonified);

    let roundtrip: Chain = serde_json::from_str(&jsonified)?;
    
    assert_eq!(chain, roundtrip);

    Ok(())
}