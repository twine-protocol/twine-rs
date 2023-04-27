use std::{collections::HashMap, error::Error};

use josekit::{jws::alg::eddsa::EddsaJwsAlgorithm, jwk::alg::ed::EdCurve::Ed25519};
use libipld::{multihash, Ipld};
use twine_builder::{ChainBuilder, PulseBuilder};

fn main() -> Result<(), Box<dyn Error>>{
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
    
    // the first pulse uses the `first` method
    let first = PulseBuilder::first(&chain)?.payload(
        HashMap::from([(String::from("count"), Ipld::Integer(1))])
    )?.finalize(&signer)?;

    // subsequent pulses use the `new` method
    let next = PulseBuilder::new(&chain, &first)
        ?.payload(HashMap::from([(String::from("count"), Ipld::Integer(2))]))
        ?.finalize(&signer)?;

    assert_eq!(next.content.index, first.content.index + 1);

    Ok(())
}