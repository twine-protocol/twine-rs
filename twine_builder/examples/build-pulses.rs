use std::{collections::HashMap, error::Error};

use twine_core::josekit::{jws::alg::eddsa::EddsaJwsAlgorithm, jwk::alg::ed::EdCurve::Ed25519};
use twine_core::libipld::multihash;
use twine_builder::{ChainBuilder, PulseBuilder};
use twine_core::libipld::ipld;

fn main() -> Result<(), Box<dyn Error>>{
    // create a chain
    let alg = EddsaJwsAlgorithm::Eddsa;
    let keys = alg.generate_key_pair(Ed25519)?;
    let signer = alg.signer_from_jwk(&keys.to_jwk_private_key())?;
    let verifier = alg.verifier_from_jwk(&keys.to_jwk_public_key())?;
    let hasher = multihash::Code::Sha3_512;
    let chain = ChainBuilder::new(
        "gold".into(), 
        HashMap::new(),
        keys.to_jwk_public_key()
    ).finalize(&signer, &verifier, hasher)?;
    
    // the first pulse uses the `first` method
    let first = PulseBuilder::first(&chain).payload(
        HashMap::from([(String::from("count"), ipld!{1})])
    ).finalize(&signer, &verifier)?;

    // subsequent pulses use the `new` method
    let next = PulseBuilder::new(&chain, &first)
        .payload(HashMap::from([(String::from("count"), ipld!{2})]))
        .finalize(&signer, &verifier)?;

    assert_eq!(next.content.index, first.content.index + 1);

    // builder is consumed, so we can't use it again here even if we wanted to
    println!("Pulse Built!");
    println!("{:#?}", next);

    Ok(())
}