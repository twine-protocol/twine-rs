use std::{collections::HashMap, error::Error};

use josekit::{jws::{alg::eddsa::EddsaJwsAlgorithm, JwsVerifier}, jwk::alg::ed::EdCurve::Ed25519};

use twine_builder::ChainBuilder;
use libipld::cid::multihash;

fn main() -> Result<(), Box<dyn Error>> {
    let keys = EddsaJwsAlgorithm::Eddsa.generate_key_pair(Ed25519)?;
    let signer = EddsaJwsAlgorithm::Eddsa.signer_from_jwk(&keys.to_jwk_private_key())?;
    let verifier = EddsaJwsAlgorithm::Eddsa.verifier_from_jwk(&keys.to_jwk_public_key())?;
    let hasher = multihash::Code::Sha3_512; 
    let builder = ChainBuilder::new(
        "gold".into(),
        keys.to_jwk_public_key(), 
        HashMap::new()
    );
    let chain = builder.finalize(&signer, hasher)?;
    verifier.verify(&hasher.digest(&serde_ipld_dagcbor::to_vec(&chain.content)?).to_bytes(), chain.
    
    // builder is consumed, so we can't use it again here even if we wanted to
    println!("Chain Built!");
    println!("{:#?}", chain);

    Ok(())
}