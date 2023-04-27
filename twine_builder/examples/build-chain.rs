use std::{collections::HashMap, error::Error};

use josekit::{jws::{alg::eddsa::EddsaJwsAlgorithm}, jwk::alg::ed::EdCurve::Ed25519};

use twine_builder::ChainBuilder;
use libipld::cid::multihash;

fn main() -> Result<(), Box<dyn Error>> {
    let keys = EddsaJwsAlgorithm::Eddsa.generate_key_pair(Ed25519)?;
    let signer = EddsaJwsAlgorithm::Eddsa.signer_from_jwk(&keys.to_jwk_private_key())?;
    let hasher = multihash::Code::Sha3_512; 
    let builder = ChainBuilder::new(
        "gold".into(),
        keys.to_jwk_public_key(), 
        HashMap::new()
    );
    let _chain = builder.finalize(&signer, hasher)?;
    
    // builder is consumed, so we can't use it again here
    println!("Chain Built!");

    Ok(())
}